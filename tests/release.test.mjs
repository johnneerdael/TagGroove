import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { PLATFORMS, GitHubApi, releasePlan, readVersions, assetName, collectAssets, assembleRelease, publishRelease, releaseBody } from '../scripts/release.mjs';

const sha = 'a'.repeat(40);
const versions = { package: '0.1.0', cargo: '0.1.0', tauri: '0.1.0' };
const planInput = { versions, head: sha, expectedHead: sha, tagCommit: sha, tag: 'v0.1.0', prerelease: false };
const plan = () => releasePlan(planInput);
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'opentag-release-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}

test('version/tag/checkout policy rejects mismatches; 0.x is a release, suffixes are prereleases', () => {
  assert.equal(plan().prerelease, false);
  assert.equal(releasePlan({ ...planInput, prerelease: true }).prerelease, true);
  assert.equal(releasePlan({ ...planInput, tag: '', tagCommit: null }).tag, 'v0.1.0');
  const stable = { package: '1.2.3', cargo: '1.2.3', tauri: '1.2.3' };
  assert.equal(releasePlan({ ...planInput, versions: stable, tag: 'v1.2.3' }).prerelease, false);
  const dev = { package: '1.2.3-dev.1', cargo: '1.2.3-dev.1', tauri: '1.2.3-dev.1' };
  assert.equal(releasePlan({ ...planInput, versions: dev, tag: 'v1.2.3-dev.1' }).prerelease, true);
  for (const patch of [
    { versions: { ...versions, cargo: '0.2.0' } },
    { tag: 'v0.2.0' }, { tag: 'v0.1.0\nmalicious' },
    { head: 'b'.repeat(40) }, { tagCommit: 'b'.repeat(40) },
    { versions: { package: '01.1.0', cargo: '01.1.0', tauri: '01.1.0' } },
  ]) assert.throws(() => releasePlan({ ...planInput, ...patch }));
});

test('version reader uses package sections rather than unrelated dependency versions', async (t) => {
  const root = await fixture(t);
  await mkdir(join(root, 'src-tauri'));
  await writeFile(join(root, 'package.json'), JSON.stringify({ version: '0.1.0' }));
  await writeFile(join(root, 'src-tauri/tauri.conf.json'), JSON.stringify({ version: '0.1.0' }));
  await writeFile(join(root, 'src-tauri/Cargo.toml'), '[package]\nname="opentag"\nversion = "0.1.0" # app\n[dependencies]\nversion="9.9.9"\n');
  assert.deepEqual(await readVersions(root), versions);
});

test('asset names state OS architecture signing or simulator restrictions', () => {
  assert.equal(assetName('v0.1.0', 'windows-x64', '/tmp/OpenTag.exe'), 'OpenTag-v0.1.0-windows-x64-setup.exe');
  assert.equal(assetName('v0.1.0', 'android-arm64', '/tmp/app-release.apk'), 'OpenTag-v0.1.0-android-arm64-signed.apk');
  assert.equal(assetName('v0.1.0', 'ios-arm64-simulator-preview', '/tmp/OpenTag.app.zip'), 'OpenTag-v0.1.0-ios-arm64-simulator-preview-debug.app.zip');
  assert.throws(() => assetName('v0.1.0', 'windows-x64', '../readme.txt'));
  assert.throws(() => assetName('v0.1.0', '../escape', 'file.exe'));
});

async function completeAssets(t) {
  const root = await fixture(t);
  const outputDir = join(root, 'assets');
  const suffixes = {
    'windows-x64': ['.exe'], 'macos-arm64': ['.dmg', '.app.tar.gz'], 'macos-x64': ['.dmg', '.app.tar.gz'],
    'linux-x64': ['.deb', '.rpm', '.AppImage'], 'linux-arm64': ['.deb', '.rpm', '.AppImage'],
    'android-arm64': ['.apk'], 'ios-arm64-simulator-preview': ['.app.zip'],
  };
  for (const platform of PLATFORMS) {
    const sourcePaths = [];
    for (const suffix of suffixes[platform]) {
      const path = join(root, `${platform}${suffix}`);
      await writeFile(path, `fixture ${platform} ${suffix}`);
      sourcePaths.push(path);
    }
    await collectAssets({ plan: plan(), platform, sourcePaths, outputDir });
  }
  return { root, outputDir };
}

test('assembly requires all seven matching platform manifests and checks every hash', async (t) => {
  const { outputDir } = await completeAssets(t);
  const release = await assembleRelease({ plan: plan(), directory: outputDir });
  assert.equal(release.assets.length, 15); // 13 packages plus manifest and checksum file
  assert.match(await readFile(join(outputDir, 'SHA256SUMS'), 'utf8'), /[a-f0-9]{64}  OpenTag-v0\.1\.0-windows-x64-setup\.exe/);
  await writeFile(join(outputDir, 'OpenTag-v0.1.0-linux-arm64.deb'), 'tampered');
  await assert.rejects(assembleRelease({ plan: plan(), directory: outputDir }), /hash|size/i);
});

test('missing platforms, duplicate package formats, and mixed commits cannot publish', async (t) => {
  const { outputDir } = await completeAssets(t);
  const manifestPath = join(outputDir, 'manifest-windows-x64.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  await writeFile(manifestPath, JSON.stringify({ ...manifest, commit: 'b'.repeat(40) }));
  await assert.rejects(assembleRelease({ plan: plan(), directory: outputDir }), /mixed/i);
  await writeFile(manifestPath, JSON.stringify(manifest));
  await rm(join(outputDir, 'manifest-ios-arm64-simulator-preview.json'));
  await assert.rejects(assembleRelease({ plan: plan(), directory: outputDir }), /missing/i);
  const root = await fixture(t);
  const paths = [join(root, 'one.exe'), join(root, 'two.exe')];
  await Promise.all(paths.map(path => writeFile(path, 'fixture')));
  await assert.rejects(collectAssets({ plan: plan(), platform: 'windows-x64', sourcePaths: paths, outputDir: root }), /duplicate/i);
});

test('published releases are immutable and failed uploads leave the release in draft', async (t) => {
  const { outputDir } = await completeAssets(t);
  const assembled = await assembleRelease({ plan: plan(), directory: outputDir });
  const calls = [];
  const publicApi = { findRelease: async () => ({ id: 1, draft: false }) };
  await assert.rejects(publishRelease({ api: publicApi, plan: plan(), assembled, repository: 'owner/repo' }), /published/i);
  const api = {
    findRelease: async () => null,
    createDraft: async () => { calls.push('draft'); return { id: 1, draft: true }; },
    listAssets: async () => [],
    uploadAsset: async () => { calls.push('upload'); throw new Error('network failed'); },
    publish: async () => calls.push('publish'),
  };
  await assert.rejects(publishRelease({ api, plan: plan(), assembled, repository: 'owner/repo' }), /network failed/);
  assert.deepEqual(calls, ['draft', 'upload']);
});

test('publication follows every upload and exact server asset verification', async (t) => {
  const { outputDir } = await completeAssets(t);
  const assembled = await assembleRelease({ plan: plan(), directory: outputDir });
  const calls = [];
  const uploaded = [];
  const api = {
    findRelease: async () => ({ id: 1, draft: true, target_commitish: sha }),
    updateDraft: async () => { calls.push('update-draft'); return { id: 1, draft: true }; },
    listAssets: async () => uploaded,
    deleteAsset: async () => assert.fail('No stale assets'),
    uploadAsset: async (_release, asset) => { calls.push(asset.name); uploaded.push({ name: asset.name, size: asset.size, state: 'uploaded', digest: `sha256:${asset.sha256}` }); },
    publish: async () => calls.push('publish'),
  };
  await publishRelease({ api, plan: plan(), assembled, repository: 'owner/repo' });
  assert.equal(calls.at(-1), 'publish');
  assert.equal(uploaded.length, assembled.assets.length);
});

test('server asset digest mismatches prevent publication even when names and sizes agree', async (t) => {
  const { outputDir } = await completeAssets(t);
  const assembled = await assembleRelease({ plan: plan(), directory: outputDir });
  for (const digest of [undefined, `sha256:${'0'.repeat(64)}`]) {
    const uploaded = [];
    const api = {
    findRelease: async () => ({ id: 1, draft: true, target_commitish: sha }),
    updateDraft: async () => ({ id: 1, draft: true }),
    listAssets: async () => uploaded,
    uploadAsset: async (_release, asset) => uploaded.push({ name: asset.name, size: asset.size, state: 'uploaded', digest }),
    publish: async () => assert.fail('Must remain draft'),
  };
    await assert.rejects(publishRelease({ api, plan: plan(), assembled, repository: 'owner/repo' }), /assets/i);
  }
});

test('authenticated release discovery includes drafts across paginated results', async (t) => {
  const requested = [];
  t.mock.method(globalThis, 'fetch', async (url) => {
    requested.push(String(url));
    const items = requested.length === 1
      ? Array.from({ length: 100 }, (_, index) => ({ tag_name: `v9.0.${index}`, draft: false }))
      : [{ id: 123, tag_name: 'v0.1.0', draft: true }];
    return new Response(JSON.stringify(items), { status: 200 });
  });
  const api = new GitHubApi('owner/repo', 'test-token');
  assert.equal((await api.findRelease('v0.1.0')).id, 123);
  assert.match(requested[1], /\/releases\?per_page=100&page=2$/);
});

test('release notes link the tagged banner and parity ledger and state signing limits', () => {
  const body = releaseBody(plan(), 'johnneerdael/opentag');
  assert.match(body, /raw\.githubusercontent\.com\/johnneerdael\/opentag\/v0\.1\.0\/assets\/brand\/OpenTag-Brand-Kit\/03-banners\/opentag-github-light-1280x640\.png/);
  assert.match(body, /\/blob\/v0\.1\.0\/docs\/PARITY\.md/);
  for (const word of ['ad-hoc', 'unsigned', 'application key', 'simulator', 'device installer']) assert.ok(body.includes(word), word);
});

test('desktop app directories are archived when build-only action supplies no archive', async (t) => {
  const root = await fixture(t);
  const app = join(root, 'OpenTag.app');
  await mkdir(join(app, 'Contents'), { recursive: true });
  await writeFile(join(app, 'Contents/Info.plist'), 'fixture');
  const dmg = join(root, 'OpenTag.dmg');
  await writeFile(dmg, 'fixture dmg');
  const outputDir = join(root, 'assets');
  const manifest = await collectAssets({ plan: plan(), platform: 'macos-arm64', sourcePaths: [app, dmg], outputDir });
  assert.equal(manifest.assets.length, 2);
  const archive = manifest.assets.find(asset => asset.name.endsWith('.app.tar.gz'));
  assert.ok(archive.size > 0);
});
