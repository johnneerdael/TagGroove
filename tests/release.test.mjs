import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { PLATFORMS, GitHubApi, releasePlan, readVersions, assetName, collectAssets, assembleRelease, publishRelease, releaseBody, parseVersion, compareVersions, automaticVersion, explicitVersion, versionOrigin, stampVersion, makeLatest, previousTag, releaseNotes } from '../scripts/release.mjs';

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
    latestRelease: async () => ({ tag_name: 'v0.0.9' }),
    publish: async (_id, body) => calls.push(['publish', body]),
  };
  await publishRelease({ api, plan: plan(), assembled, repository: 'owner/repo' });
  assert.deepEqual(calls.at(-1), ['publish', { draft: false, prerelease: false, make_latest: 'true' }]);
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

test('semantic versions parse and order by precedence', () => {
  assert.deepEqual(parseVersion('1.2.3-beta.1'), { major: 1, minor: 2, patch: 3, prerelease: 'beta.1', build: undefined });
  assert.throws(() => parseVersion('1.2'));
  const ordered = ['0.1.0', '0.2.0-alpha', '0.2.0-alpha.1', '0.2.0-beta', '0.2.0-beta.2', '0.2.0-beta.11', '0.2.0', '0.2.1', '0.2.10', '0.10.0', '1.0.0'];
  for (let i = 1; i < ordered.length; i++) {
    assert.equal(compareVersions(ordered[i - 1], ordered[i]), -1, `${ordered[i - 1]} < ${ordered[i]}`);
    assert.equal(compareVersions(ordered[i], ordered[i - 1]), 1);
  }
  assert.equal(compareVersions('0.2.3', '0.2.3'), 0);
});

test('automatic versions count from a plain base and stay inside the Android versionCode range', () => {
  assert.equal(automaticVersion('0.2.0', 0), '0.2.0');
  assert.equal(automaticVersion('0.2.0', 17), '0.2.17');
  assert.equal(automaticVersion('1.4.0', 999), '1.4.999');
  for (const [base, distance] of [['0.2.0', 1000], ['0.2.1', 0], ['0.2.0-beta.1', 0], ['0.2.0+build', 0], ['0.1000.0', 0], ['0.2.0', -1], ['0.2.0', 1.5]]) {
    assert.throws(() => automaticVersion(base, distance), `${base} + ${distance}`);
  }
});

test('explicit tags release the base version or a suffixed prerelease, never an automatic number', () => {
  assert.equal(explicitVersion('0.2.0', 'v0.2.0'), '0.2.0');
  assert.equal(explicitVersion('0.2.0', 'v0.3.0-beta.1'), '0.3.0-beta.1');
  for (const tag of ['v0.2.7', 'v0.3.0', '0.2.0', 'v0.2.0\nx']) assert.throws(() => explicitVersion('0.2.0', tag), tag);
});

function repo(root) {
  const git = (...args) => execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', '-c', 'commit.gpgsign=false', ...args], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  const setVersion = async (version, cargoExtra = '') => {
    await mkdir(join(root, 'src-tauri'), { recursive: true });
    await writeFile(join(root, 'package.json'), `${JSON.stringify({ name: 'opentag', version }, null, 2)}\n`);
    await writeFile(join(root, 'src-tauri/tauri.conf.json'), `${JSON.stringify({ version }, null, 2)}\n`);
    await writeFile(join(root, 'src-tauri/Cargo.toml'), `[package]\nname = "opentag"\nversion = "${version}"\n${cargoExtra}`);
  };
  let counter = 0;
  const commit = async (message = `change ${++counter}`) => {
    await writeFile(join(root, 'notes.txt'), `${message}\n`);
    git('add', '-A');
    git('commit', '-q', '-m', message);
  };
  return { git, setVersion, commit };
}

test('automatic numbering counts first-parent merges since the base version changed', async (t) => {
  const root = await fixture(t);
  const { git, setVersion, commit } = repo(root);
  git('init', '-q', '-b', 'main');
  await setVersion('0.1.0');
  await commit('initial');
  await commit();
  // The bump lands from a branch; the merge commit on main is version 0.2.0.
  git('checkout', '-q', '-b', 'bump');
  await setVersion('0.2.0');
  await commit('bump');
  await commit('follow-up on branch');
  git('checkout', '-q', 'main');
  git('merge', '-q', '--no-ff', '-m', 'Merge bump', 'bump');
  assert.deepEqual(versionOrigin(root, '0.2.0'), { commit: git('rev-parse', 'HEAD'), distance: 0 });
  const origin = git('rev-parse', 'HEAD');
  // Three branch commits merged at once still advance the version by one.
  git('checkout', '-q', '-b', 'feature');
  for (let i = 0; i < 3; i++) await commit();
  git('checkout', '-q', 'main');
  git('merge', '-q', '--no-ff', '-m', 'Merge feature', 'feature');
  assert.deepEqual(versionOrigin(root, '0.2.0'), { commit: origin, distance: 1 });
  // A direct commit and a version-file edit that keeps the version both count.
  await commit();
  await setVersion('0.2.0', '[dependencies]\nserde = "1"\n');
  await commit('add dependency');
  const result = versionOrigin(root, '0.2.0');
  assert.deepEqual(result, { commit: origin, distance: 3 });
  assert.equal(automaticVersion('0.2.0', result.distance), '0.2.3');
  assert.throws(() => versionOrigin(root, '0.3.0'), /base version/);
  // Bumping the minor version restarts numbering at that commit.
  await setVersion('0.3.0');
  await commit('bump minor');
  assert.deepEqual(versionOrigin(root, '0.3.0'), { commit: git('rev-parse', 'HEAD'), distance: 0 });
  await commit();
  assert.equal(versionOrigin(root, '0.3.0').distance, 1);
});

test('automatic numbering starts at the root when the base version never changed', async (t) => {
  const root = await fixture(t);
  const { git, setVersion, commit } = repo(root);
  git('init', '-q', '-b', 'main');
  await setVersion('0.2.0');
  await commit('initial');
  await commit();
  await commit();
  assert.equal(versionOrigin(root, '0.2.0').distance, 2);
});

const LOCK = `version = 4

[[package]]
name = "openssl"
version = "0.10.0"

[[package]]
name = "opentag"
version = "0.2.0"
dependencies = [
 "tag-core",
]

[[package]]
name = "tag-core"
version = "0.2.0"
`;

async function stampFixture(t, { apple = true } = {}) {
  const root = await fixture(t);
  await mkdir(join(root, 'src-tauri/gen/apple/opentag_iOS'), { recursive: true });
  await writeFile(join(root, 'package.json'), `${JSON.stringify({ name: 'opentag', version: '0.2.0', private: true }, null, 2)}\n`);
  await writeFile(join(root, 'package-lock.json'), `${JSON.stringify({ name: 'opentag', version: '0.2.0', lockfileVersion: 3, packages: { '': { name: 'opentag', version: '0.2.0' }, 'node_modules/vite': { version: '7.3.0' } } }, null, 2)}\n`);
  await writeFile(join(root, 'src-tauri/tauri.conf.json'), `${JSON.stringify({ productName: 'OpenTag', version: '0.2.0' }, null, 2)}\n`);
  await writeFile(join(root, 'src-tauri/Cargo.toml'), '[package]\nname = "opentag"\nversion = "0.2.0"\n\n[dependencies]\ntag-core = { path = "../crates/tag-core", version = "0.2.0" }\n');
  await writeFile(join(root, 'Cargo.lock'), LOCK);
  if (apple) {
    await writeFile(join(root, 'src-tauri/gen/apple/project.yml'), 'info:\n  properties:\n    CFBundleShortVersionString: 0.2.0\n    CFBundleVersion: "0.2.0"\n');
    await writeFile(join(root, 'src-tauri/gen/apple/opentag_iOS/Info.plist'), '<dict>\n\t<key>CFBundleShortVersionString</key>\n\t<string>0.2.0</string>\n\t<key>CFBundleVersion</key>\n\t<string>0.2.0</string>\n</dict>\n');
  }
  return root;
}

test('stamping sets the release version in every manifest and lockfile without touching dependencies', async (t) => {
  const root = await stampFixture(t);
  await stampVersion(root, '0.2.42');
  assert.deepEqual(await readVersions(root), { package: '0.2.42', tauri: '0.2.42', cargo: '0.2.42' });
  const lock = JSON.parse(await readFile(join(root, 'package-lock.json'), 'utf8'));
  assert.equal(lock.version, '0.2.42');
  assert.equal(lock.packages[''].version, '0.2.42');
  assert.equal(lock.packages['node_modules/vite'].version, '7.3.0');
  assert.match(await readFile(join(root, 'src-tauri/Cargo.toml'), 'utf8'), /tag-core = \{ path = "\.\.\/crates\/tag-core", version = "0\.2\.0" \}/);
  assert.equal(await readFile(join(root, 'Cargo.lock'), 'utf8'), LOCK.replace('name = "opentag"\nversion = "0.2.0"', 'name = "opentag"\nversion = "0.2.42"'));
  const yml = await readFile(join(root, 'src-tauri/gen/apple/project.yml'), 'utf8');
  assert.match(yml, /CFBundleShortVersionString: "0\.2\.42"\n {4}CFBundleVersion: "0\.2\.42"/);
  const plist = await readFile(join(root, 'src-tauri/gen/apple/opentag_iOS/Info.plist'), 'utf8');
  assert.equal(plist.match(/<string>0\.2\.42<\/string>/g).length, 2);
  // Stamping is repeatable, so a rerun job may stamp an already stamped tree.
  await stampVersion(root, '0.2.42');
  assert.deepEqual(await readVersions(root), { package: '0.2.42', tauri: '0.2.42', cargo: '0.2.42' });
});

test('stamping fails rather than leave a lockfile that breaks --locked builds', async (t) => {
  const root = await stampFixture(t, { apple: false });
  await stampVersion(root, '0.3.0-beta.1');
  assert.deepEqual(await readVersions(root), { package: '0.3.0-beta.1', tauri: '0.3.0-beta.1', cargo: '0.3.0-beta.1' });
  await assert.rejects(stampVersion(root, '0.3'), /semantic version/);
  await writeFile(join(root, 'Cargo.lock'), LOCK.replace('name = "opentag"', 'name = "other"'));
  await assert.rejects(stampVersion(root, '0.2.5'), /Cargo\.lock/);
});

test('only a newer full release becomes latest', () => {
  assert.equal(makeLatest(plan(), null), 'true');
  assert.equal(makeLatest(plan(), { tag_name: 'v0.0.9' }), 'true');
  assert.equal(makeLatest(plan(), { tag_name: 'v0.1.0' }), 'false');
  assert.equal(makeLatest(plan(), { tag_name: 'v0.2.3' }), 'false');
  assert.equal(makeLatest({ ...plan(), prerelease: true }, null), 'false');
});

test('a rerun after publication is a no-op and never a duplicate release', async (t) => {
  const { outputDir } = await completeAssets(t);
  const assembled = await assembleRelease({ plan: plan(), directory: outputDir });
  const api = {
    findRelease: async () => ({ id: 1, draft: false, target_commitish: sha }),
    createDraft: async () => assert.fail('No new draft'),
    updateDraft: async () => assert.fail('No draft update'),
    uploadAsset: async () => assert.fail('No upload'),
    publish: async () => assert.fail('No publish'),
  };
  assert.equal((await publishRelease({ api, plan: plan(), assembled, repository: 'owner/repo' })).skipped, true);
  api.findRelease = async () => ({ id: 1, draft: false, target_commitish: 'b'.repeat(40) });
  await assert.rejects(publishRelease({ api, plan: plan(), assembled, repository: 'owner/repo' }), /published/);
});

test('release notes cover changes since the previous earlier release', async () => {
  const auto = { ...plan(), tag: 'v0.2.3', version: '0.2.3' };
  assert.equal(previousTag(['v0.1.0', 'v0.2.0', 'v0.2.2', 'v0.2.3', 'v0.2.4', 'v0.3.0-beta.1', 'v0.2.3-rc.1', 'nightly'], auto), 'v0.2.2');
  assert.equal(previousTag(['v0.2.0', 'v0.2.3-rc.1'], auto), 'v0.2.0');
  assert.equal(previousTag(['v0.2.0', 'v0.2.3-rc.1'], { ...auto, prerelease: true }), 'v0.2.3-rc.1');
  assert.equal(previousTag(['v0.2.3'], auto), null);
  const requests = [];
  const generated = await releaseNotes({ api: { generateNotes: async (body) => { requests.push(body); return { body: "## What's Changed\n* Add chapters by @dev in #17\n" }; } }, plan: auto, previous: 'v0.2.2', log: () => assert.fail('No fallback') });
  assert.match(generated, /Add chapters/);
  assert.deepEqual(requests, [{ tag_name: 'v0.2.3', target_commitish: sha, previous_tag_name: 'v0.2.2' }]);
  const fallback = await releaseNotes({ api: { generateNotes: async () => { throw new Error('offline'); } }, plan: auto, previous: 'v0.2.2', log: () => 'Merge pull request #17 (abc1234)\nFix list (def5678)\n' });
  assert.equal(fallback, '## Changes since v0.2.2\n\n- Merge pull request #17 (abc1234)\n- Fix list (def5678)\n');
  const body = releaseBody(auto, 'johnneerdael/opentag', fallback);
  assert.ok(body.indexOf('Fix list') < body.indexOf('| Download |'));
  assert.match(body, /Release \(pre-1\.0\)/);
});
