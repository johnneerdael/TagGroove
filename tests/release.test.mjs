import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm, copyFile } from 'node:fs/promises';
import {mkdtempSync, rmSync} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { PLATFORMS, GitHubApi, releasePlan, readVersions, assetName, collectAssets, assembleRelease, publishRelease, releaseBody, parseVersion, compareVersions, automaticVersion, explicitVersion, versionOrigin, stampVersion, makeLatest, previousTag, releaseNotes, appleVersion, androidVersionCode, shipsChanges, NON_SHIPPING_PATHS } from '../scripts/release.mjs';

const sourceFixtureRoot = mkdtempSync(join(tmpdir(), 'taggroove-release-source-'));
after(() => rmSync(sourceFixtureRoot, {recursive: true, force: true}));
const sourceFixture = JSON.parse(execFileSync(process.env.PYTHON || (process.platform === 'win32' ? 'python' : 'python3'),
  ['scripts/tests/test_audio_source_bundle.py', '--release-fixture', sourceFixtureRoot], {encoding: 'utf8'}));
const sha = sourceFixture.commit;
const versions = { package: '0.1.0', cargo: '0.1.0', tauri: '0.1.0' };
const planInput = { versions, head: sha, expectedHead: sha, tagCommit: sha, tag: 'v0.1.0', prerelease: false };
const plan = () => releasePlan(planInput);
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'taggroove-release-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}

test('branded Windows installers retain the existing install identity', async () => {
  const config = JSON.parse(await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url)));
  assert.equal(config.bundle.windows.wix.upgradeCode.toLowerCase(), 'ed7df85b-a59c-563d-84e9-a966cbccc202');
  assert.equal(config.bundle.windows.nsis?.template, 'windows/installer.nsi');
  const upstream = JSON.parse(await readFile(new URL('../src-tauri/windows/UPSTREAM.json', import.meta.url)));
  const lock = JSON.parse(await readFile(new URL('../package-lock.json', import.meta.url)));
  assert.equal(lock.packages['node_modules/@tauri-apps/cli'].version, upstream.cliVersion,
    'Resync the installer template when upgrading the Tauri CLI');
  const template = await readFile(new URL('../src-tauri/windows/installer.nsi', import.meta.url), 'utf8');
  const definitions = Object.fromEntries([...template.matchAll(/^!define (\w+) "([^"\r\n]*)"$/gm)]
    .map(([, name, value]) => [name, value]));
  definitions.PRODUCTNAME = config.productName;
  definitions.MANUFACTURER = config.bundle.publisher;
  const expand = (name, parents = []) => {
    assert.ok(Object.hasOwn(definitions, name), `Missing NSIS definition ${name}`);
    assert.ok(!parents.includes(name), `Circular NSIS definition ${name}`);
    return definitions[name].replace(/\$\{(\w+)\}/g, (_, next) => expand(next, [...parents, name]));
  };
  assert.equal(expand('PRODUCTNAME'), 'TagGroove');
  assert.equal(expand('MANUFACTURER'), 'TagGroove');
  assert.equal(expand('UNINSTKEY'), 'Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\OpenTag');
  assert.equal(expand('MANUKEY'), 'Software\\OpenTag');
  assert.equal(expand('MANUPRODUCTKEY'), 'Software\\OpenTag\\OpenTag');
});

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
  assert.equal(assetName('v0.1.0', 'windows-x64', '/tmp/TagGroove.exe'), 'TagGroove-v0.1.0-windows-x64-setup.exe');
  assert.equal(assetName('v0.1.0', 'android-arm64', '/tmp/app-release.apk'), 'TagGroove-v0.1.0-android-arm64-signed.apk');
  assert.equal(assetName('v0.1.0', 'ios-arm64-simulator-preview', '/tmp/TagGroove.app.zip'), 'TagGroove-v0.1.0-ios-arm64-simulator-preview-debug.app.zip');
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
    await collectAssets({ plan: plan(), platform, sourcePaths, outputDir,
      nativeManifest: join(sourceFixture.evidence, `native-${platform}.json`) });
  }
  await copyFile(sourceFixture.bundle, join(outputDir, 'TagGroove-v0.1.0-sources.tar.xz'));
  return { root, outputDir };
}

test('Linux package collection binds source evidence to the actual image and release identity', async (t) => {
  const root = await fixture(t);const platform = 'linux-x64';
  const sourcePaths = await Promise.all(['.deb', '.rpm', '.AppImage'].map(async suffix => {
    const path = join(root, 'fixture'+suffix);await writeFile(path, 'packaged '+suffix);return path;
  }));
  const image = await readFile(sourcePaths[2]);
  const capture = {schema: 1, platform, target: 'x86_64-unknown-linux-gnu', source_commit: sha, version: '0.1.0',
    appimage_sha256: createHash('sha256').update(image).digest('hex'), appimage_size: image.length};
  const linuxManifest = join(root, 'linux-source.json');
  await writeFile(linuxManifest, JSON.stringify(capture));
  const args = {plan: plan(), platform, sourcePaths, outputDir: join(root, 'assets'), linuxManifest,
    nativeManifest: join(sourceFixture.evidence, 'native-linux-x64.json')};
  const manifest = await collectAssets(args);
  assert.equal(manifest.linux_source.appimage_sha256, capture.appimage_sha256);
  assert.equal(manifest.linux_source.name, 'linux-source-linux-x64.json');
  assert.equal(manifest.linux_source.target, capture.target);
  for (const changed of [{source_commit: 'b'.repeat(40)}, {version: '0.2.0'}, {platform: 'linux-arm64'},
    {target: 'aarch64-unknown-linux-gnu'}, {appimage_sha256: '0'.repeat(64)}, {appimage_size: image.length+1}]) {
    await writeFile(linuxManifest, JSON.stringify({...capture, ...changed}));
    await assert.rejects(collectAssets(args), /Linux.*(source|image|target|binding)/i);
  }
});

test('modern release assembly requires the two source-bound Linux sidecars', {skip: process.platform === 'win32'}, async (t) => {
  const root = await fixture(t);
  const python = process.env.PYTHON || 'python3';
  const captured = JSON.parse(execFileSync(python,
    ['scripts/tests/test_audio_source_bundle.py', '--release-linux-fixture', join(root, 'source')], {encoding: 'utf8'}));
  const previousPython = process.env.PYTHON;process.env.PYTHON = captured.python;
  t.after(() => {if (previousPython === undefined) delete process.env.PYTHON;else process.env.PYTHON = previousPython;});
  const releasePlan = {...plan(), commit: captured.commit};const directory = join(root, 'assets');
  const suffixes = { 'windows-x64': ['.exe'], 'macos-arm64': ['.dmg', '.app.tar.gz'], 'macos-x64': ['.dmg', '.app.tar.gz'],
    'linux-x64': ['.deb', '.rpm', '.AppImage'], 'linux-arm64': ['.deb', '.rpm', '.AppImage'],
    'android-arm64': ['.apk'], 'ios-arm64-simulator-preview': ['.app.zip']};
  for (const platform of PLATFORMS) {
    const sourcePaths = [];
    for (const suffix of suffixes[platform]) {
      const path = join(root, platform+suffix);await writeFile(path, `fixture ${platform} ${suffix}`);sourcePaths.push(path);
    }
    await collectAssets({plan: releasePlan, platform, sourcePaths, outputDir: directory,
      nativeManifest: join(captured.evidence, `native-${platform}.json`),
      ...(platform.startsWith('linux-') ? {linuxManifest: join(captured.linux_evidence, `linux-source-${platform}.json`)} : {})});
  }
  await copyFile(captured.bundle, join(directory, 'TagGroove-v0.1.0-sources.tar.xz'));
  const assembled = await assembleRelease({plan: releasePlan, directory});
  assert.equal(assembled.assets.length, 16);
  const published = JSON.parse(await readFile(join(directory, 'release-manifest.json'), 'utf8'));
  assert.deepEqual(Object.keys(published.linux_distribution).sort(), ['linux-arm64', 'linux-x64']);
  const sidecar = join(directory, 'linux-source-linux-x64.json');const original = await readFile(sidecar);
  await writeFile(sidecar, Buffer.concat([original, Buffer.from('\n')]));
  await assert.rejects(assembleRelease({plan: releasePlan, directory}), /differs from built packages/);
  await writeFile(sidecar, original);
  const manifestPath = join(directory, 'manifest-linux-x64.json');const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  await writeFile(manifestPath, JSON.stringify({...manifest, linux_source: {...manifest.linux_source, sha256: '0'.repeat(64)}}));
  await assert.rejects(assembleRelease({plan: releasePlan, directory}), /Linux source evidence hash/);
  await writeFile(manifestPath, JSON.stringify(manifest));
  await writeFile(join(directory, 'linux-source-extra.json'), original);
  await assert.rejects(assembleRelease({plan: releasePlan, directory}), /Unexpected files/);
});

test('assembly requires all seven matching platform manifests and checks every hash', async (t) => {
  const { outputDir } = await completeAssets(t);
  const release = await assembleRelease({ plan: plan(), directory: outputDir });
  assert.equal(release.assets.length, 16); // Exact original 13 packages, source archive, manifest and checksums.
  assert.equal(release.assets.filter(asset => asset.name.endsWith('-sources.tar.xz')).length, 1);
  assert.match(await readFile(join(outputDir, 'SHA256SUMS'), 'utf8'), /[a-f0-9]{64}  TagGroove-v0\.1\.0-windows-x64-setup\.exe/);
  await writeFile(join(outputDir, 'TagGroove-v0.1.0-linux-arm64.deb'), 'tampered');
  await assert.rejects(assembleRelease({ plan: plan(), directory: outputDir }), /hash|size/i);
});

test('assembly requires the exact corresponding-source asset before publication', async (t) => {
  const { outputDir } = await completeAssets(t);
  await rm(join(outputDir, 'TagGroove-v0.1.0-sources.tar.xz'), { force: true });
  await assert.rejects(assembleRelease({ plan: plan(), directory: outputDir }), /source.*(missing|required)|missing.*source/i);
});

test('native provenance and the exact source filename cannot be replaced by extra assets', async (t) => {
  const { outputDir } = await completeAssets(t);
  const nativePath = join(outputDir, 'native-windows-x64.json');
  const original = await readFile(nativePath, 'utf8');
  await writeFile(nativePath, JSON.stringify({...JSON.parse(original), target: 'aarch64-apple-darwin'}));
  await assert.rejects(assembleRelease({plan: plan(), directory: outputDir}), /native.*source.*target/i);
  await writeFile(nativePath, original);
  const manifestPath = join(outputDir, 'manifest-windows-x64.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  await writeFile(manifestPath, JSON.stringify({...manifest, native: {...manifest.native, sha256: '0'.repeat(64)}}));
  await assert.rejects(assembleRelease({plan: plan(), directory: outputDir}), /native.*hash/i);
  await writeFile(manifestPath, JSON.stringify(manifest));
  await copyFile(sourceFixture.bundle, join(outputDir, 'TagGroove-v0.1.0-sources-extra.tar.xz'));
  await assert.rejects(assembleRelease({plan: plan(), directory: outputDir}), /unexpected files/i);
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
  const body = releaseBody(plan(), 'johnneerdael/TagGroove');
  assert.match(body, /raw\.githubusercontent\.com\/johnneerdael\/TagGroove\/v0\.1\.0\/assets\/brand\/TagGroove-Brand-Kit\/03-banners\/taggroove-github-light-1280x640\.png/);
  assert.match(body, /\/blob\/v0\.1\.0\/docs\/PARITY\.md/);
  for (const word of ['ad-hoc', 'unsigned', 'application key', 'simulator', 'device installer']) assert.ok(body.includes(word), word);
});

test('desktop app directories are archived when build-only action supplies no archive', async (t) => {
  const root = await fixture(t);
  const app = join(root, 'TagGroove.app');
  await mkdir(join(app, 'Contents'), { recursive: true });
  await writeFile(join(app, 'Contents/Info.plist'), 'fixture');
  const dmg = join(root, 'TagGroove.dmg');
  await writeFile(dmg, 'fixture dmg');
  const outputDir = join(root, 'assets');
  const manifest = await collectAssets({ plan: plan(), platform: 'macos-arm64', sourcePaths: [app, dmg], outputDir,
    nativeManifest: join(sourceFixture.evidence, 'native-macos-arm64.json') });
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
    await writeFile(join(root, 'package.json'), `${JSON.stringify({ name: 'taggroove', version }, null, 2)}\n`);
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
  assert.deepEqual(versionOrigin(root, '0.2.0'), { commit: git('rev-parse', 'HEAD'), distance: 0, shipped: git('rev-parse', 'HEAD') });
  const origin = git('rev-parse', 'HEAD');
  // Three branch commits merged at once still advance the version by one.
  git('checkout', '-q', '-b', 'feature');
  for (let i = 0; i < 3; i++) await commit();
  git('checkout', '-q', 'main');
  git('merge', '-q', '--no-ff', '-m', 'Merge feature', 'feature');
  assert.deepEqual(versionOrigin(root, '0.2.0'), { commit: origin, distance: 1, shipped: git('rev-parse', 'HEAD') });
  // A direct commit and a version-file edit that keeps the version both count.
  await commit();
  await setVersion('0.2.0', '[dependencies]\nserde = "1"\n');
  await commit('add dependency');
  const result = versionOrigin(root, '0.2.0');
  assert.deepEqual(result, { commit: origin, distance: 3, shipped: git('rev-parse', 'HEAD') });
  assert.equal(automaticVersion('0.2.0', result.distance), '0.2.3');
  assert.throws(() => versionOrigin(root, '0.3.0'), /base version/);
  // Bumping the minor version restarts numbering at that commit.
  await setVersion('0.3.0');
  await commit('bump minor');
  assert.deepEqual(versionOrigin(root, '0.3.0'), { commit: git('rev-parse', 'HEAD'), distance: 0, shipped: git('rev-parse', 'HEAD') });
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
  await writeFile(join(root, 'package.json'), `${JSON.stringify({ name: 'taggroove', version: '0.2.0', private: true }, null, 2)}\n`);
  await writeFile(join(root, 'package-lock.json'), `${JSON.stringify({ name: 'taggroove', version: '0.2.0', lockfileVersion: 3, packages: { '': { name: 'taggroove', version: '0.2.0' }, 'node_modules/vite': { version: '7.3.0' } } }, null, 2)}\n`);
  await writeFile(join(root, 'src-tauri/tauri.conf.json'), `${JSON.stringify({ productName: 'TagGroove', version: '0.2.0' }, null, 2)}\n`);
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

test('prereleases keep their suffix in app metadata but Apple bundle versions stay numeric', async (t) => {
  const root = await stampFixture(t);
  await stampVersion(root, '0.3.0-beta.1');
  assert.deepEqual(await readVersions(root), { package: '0.3.0-beta.1', tauri: '0.3.0-beta.1', cargo: '0.3.0-beta.1' });
  assert.match(await readFile(join(root, 'Cargo.lock'), 'utf8'), /name = "opentag"\nversion = "0\.3\.0-beta\.1"/);
  const yml = await readFile(join(root, 'src-tauri/gen/apple/project.yml'), 'utf8');
  assert.match(yml, /CFBundleShortVersionString: "0\.3\.0"\n {4}CFBundleVersion: "0\.3\.0"\n/);
  const plist = await readFile(join(root, 'src-tauri/gen/apple/opentag_iOS/Info.plist'), 'utf8');
  assert.equal(plist.match(/<string>0\.3\.0<\/string>/g).length, 2);
  assert.doesNotMatch(plist + yml, /beta/);
  assert.equal(appleVersion('0.3.0-beta.1'), '0.3.0');
  assert.equal(appleVersion('1.2.3+build.5'), '1.2.3');
  assert.equal(appleVersion('0.2.17'), '0.2.17');
  // Android keeps the suffix in versionName; versionCode uses Tauri's formula.
  assert.equal(androidVersionCode('0.2.17'), 2017);
  assert.equal(androidVersionCode('0.3.0-beta.1'), 3000);
  assert.equal(androidVersionCode('1.4.999'), 1004999);
  assert.ok(androidVersionCode('0.2.1') > androidVersionCode('0.1.0'));
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

test('release ranges use the previous earlier version', () => {
  const auto = { ...plan(), tag: 'v0.2.3', version: '0.2.3' };
  assert.equal(previousTag(['v0.1.0', 'v0.2.0', 'v0.2.2', 'v0.2.3', 'v0.2.4', 'v0.3.0-beta.1', 'v0.2.3-rc.1', 'nightly'], auto), 'v0.2.2');
  assert.equal(previousTag(['v0.2.0', 'v0.2.3-rc.1'], auto), 'v0.2.0');
  assert.equal(previousTag(['v0.2.0', 'v0.2.3-rc.1'], { ...auto, prerelease: true }), 'v0.2.3-rc.1');
  assert.equal(previousTag(['v0.2.3'], auto), null);
  const body = releaseBody(auto, 'johnneerdael/TagGroove', '## Changes\n\n- Fix list');
  assert.ok(body.indexOf('Fix list') < body.indexOf('| Download |'));
  assert.match(body, /Release \(pre-1\.0\)/);
});

test('published release enumeration excludes drafts and paginates', async () => {
  const api = new GitHubApi('owner/repo', 'test-token');
  const requests = [];
  api.request = async path => {
    requests.push(path);
    return path.endsWith('page=1')
      ? Array.from({ length: 100 }, (_, index) => ({ tag_name: `v0.2.${index}`, draft: index === 99 }))
      : [{ tag_name: 'v0.2.100', draft: false }];
  };
  const tags = await api.publishedReleaseTags();
  assert.equal(tags.size, 100);
  assert.ok(!tags.has('v0.2.99'));
  assert.ok(tags.has('v0.2.100'));
  assert.equal(requests.length, 2);
});

test('notes invoke the strict collector and fail rather than silently downgrade', async t => {
  const root = await fixture(t);
  const { git, setVersion, commit } = repo(root);
  git('init', '-q', '-b', 'main');
  await setVersion('0.2.0');
  await commit('Published release');
  const head = git('rev-parse', 'HEAD');
  git('tag', 'v0.2.0');
  await mkdir(join(root, '.github'));
  await writeFile(join(root, '.github/release-notes-baseline'), `${head}\n`);
  const notes = await releaseNotes({ plan: { ...plan(), commit: head }, previous: 'v0.2.0', root, repository: 'owner/repo' });
  assert.match(notes, /No commits in this release range/);
  await writeFile(join(root, '.github/release-notes-baseline'), 'invalid\n');
  await assert.rejects(releaseNotes({ plan: { ...plan(), commit: head }, previous: 'v0.2.0', root, repository: 'owner/repo' }), /baseline/);
});

test('merges that change nothing that ships keep the previous number', async (t) => {
  const root = await fixture(t);
  const { git, setVersion, commit } = repo(root);
  const touch = async (path, message) => {
    await mkdir(join(root, path, '..'), { recursive: true });
    await writeFile(join(root, path), `${message}\n`);
    git('add', '-A');
    git('commit', '-q', '-m', message);
    return git('rev-parse', 'HEAD');
  };
  git('init', '-q', '-b', 'main');
  await setVersion('0.1.0');
  await commit('initial');
  await setVersion('0.2.0');
  await commit('bump');
  const base = git('rev-parse', 'HEAD');
  const docsOnly = [
    'docs/guide/index.md', 'mkdocs.yml', 'docs/site-requirements.txt', 'README.md', 'CHANGELOG.md',
    '.github/workflows/docs-site.yml', '.github/workflows/pages-deploy.yml', 'tests/docs-screens/shot.mjs', 'scripts/docs-build.mjs', 'scripts/docs-tools/run.sh',
  ];
  for (const path of docsOnly) {
    const head = await touch(path, `docs ${path}`);
    assert.equal(shipsChanges(root, `${head}^1`, head), false, path);
  }
  assert.deepEqual(versionOrigin(root, '0.2.0'), { commit: base, distance: 0, shipped: base });
  // Nested Markdown, other workflows and other scripts ship (or affect what ships).
  const code = await touch('src/notes.md', 'nested markdown ships');
  assert.deepEqual(versionOrigin(root, '0.2.0'), { commit: base, distance: 1, shipped: code });
  await touch('docs/after.md', 'more docs');
  assert.equal(versionOrigin(root, '0.2.0').distance, 1);
  assert.equal(shipsChanges(root, code, 'HEAD'), false);
  for (const path of ['.github/workflows/release.yml', 'scripts/release.mjs', 'tests/release.test.mjs']) {
    const head = await touch(path, `ships ${path}`);
    assert.equal(shipsChanges(root, `${head}^1`, head), true, path);
  }
  assert.equal(versionOrigin(root, '0.2.0').distance, 4);
  // A merge mixing docs and code ships once.
  git('checkout', '-q', '-b', 'mixed');
  await touch('docs/mixed.md', 'mixed docs');
  await touch('src/app.ts', 'mixed code');
  git('checkout', '-q', 'main');
  git('merge', '-q', '--no-ff', '-m', 'Merge mixed', 'mixed');
  assert.equal(versionOrigin(root, '0.2.0').distance, 5);
  assert.ok(NON_SHIPPING_PATHS.includes('docs/**'));
});
