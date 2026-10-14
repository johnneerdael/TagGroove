import { createHash } from 'node:crypto';
import { createReadStream, openAsBlob } from 'node:fs';
import { appendFile, copyFile, mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const FORMATS = {
  'windows-x64': ['-setup.exe'],
  'macos-arm64': ['.app.tar.gz', '.dmg'],
  'macos-x64': ['.app.tar.gz', '.dmg'],
  'linux-x64': ['.AppImage', '.deb', '.rpm'],
  'linux-arm64': ['.AppImage', '.deb', '.rpm'],
  'android-arm64': ['-signed.apk'],
  'ios-arm64-simulator-preview': ['-debug.app.zip'],
};
export const PLATFORMS = Object.keys(FORMATS);

export function releasePlan({ versions, head, expectedHead, tagCommit, tag, prerelease = true }) {
  const version = versions.package;
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(version);
  if (!match || match[4]?.split('.').some(part => /^0\d+$/.test(part))) throw new Error(`Invalid semantic version: ${version}`);
  if (Object.values(versions).some(value => value !== version)) throw new Error('package.json, Cargo.toml and tauri.conf.json versions must match');
  tag ||= `v${version}`;
  if (tag !== `v${version}`) throw new Error('Release tag must exactly match the application version');
  if (!/^[a-f0-9]{40}$/.test(head) || head !== expectedHead) throw new Error('Checked-out commit does not match the selected workflow commit');
  if (tagCommit && tagCommit !== head) throw new Error('Existing release tag points at a different commit');
  if (typeof prerelease !== 'boolean') throw new Error('Prerelease selection must be boolean');
  return { tag, version, commit: head, prerelease: prerelease || match[1] === '0' || Boolean(match[4]) };
}

export async function readVersions(root) {
  const [npm, tauri, cargo] = await Promise.all([
    readFile(join(root, 'package.json'), 'utf8'), readFile(join(root, 'src-tauri/tauri.conf.json'), 'utf8'), readFile(join(root, 'src-tauri/Cargo.toml'), 'utf8'),
  ]);
  const section = cargo.match(/^\[package\][^]*?(?=^\[|$(?![^]))/m)?.[0];
  const version = section?.match(/^version\s*=\s*["']([^"']+)["']\s*(?:#.*)?$/m)?.[1];
  if (!version) throw new Error('src-tauri/Cargo.toml must contain an explicit [package] version');
  return { package: JSON.parse(npm).version, tauri: JSON.parse(tauri).version, cargo: version };
}

function checkedTag(tag) {
  if (!/^v[0-9A-Za-z.+-]+$/.test(tag)) throw new Error('Unsafe release tag');
}

export function assetName(tag, platform, source) {
  checkedTag(tag);
  if (!PLATFORMS.includes(platform)) throw new Error('Unsupported release platform');
  let suffix;
  if (platform === 'windows-x64' && source.endsWith('.exe')) suffix = '-setup.exe';
  else if (platform === 'android-arm64' && source.endsWith('.apk')) suffix = '-signed.apk';
  else if (platform === 'ios-arm64-simulator-preview' && source.endsWith('.app.zip')) suffix = '-debug.app.zip';
  else suffix = FORMATS[platform].find(format => source.endsWith(format));
  if (!suffix) throw new Error(`Unsupported artifact format for ${platform}: ${basename(source)}`);
  return `OpenTag-${tag}-${platform}${suffix}`;
}

async function describe(path) {
  const info = await stat(path);
  if (!info.isFile() || info.size === 0) throw new Error(`Missing or empty release asset: ${path}`);
  const hash = createHash('sha256');
  for await (const bytes of createReadStream(path)) hash.update(bytes);
  return { name: basename(path), size: info.size, sha256: hash.digest('hex'), path };
}

export async function collectAssets({ plan, platform, sourcePaths, outputDir }) {
  const expected = FORMATS[platform];
  if (!expected || !Array.isArray(sourcePaths)) throw new Error('Invalid platform artifacts');
  await mkdir(outputDir, { recursive: true });
  const names = new Set();
  const assets = [];
  for (let source of sourcePaths) {
    // v1 currently archives these even in build-only mode. Keep a fallback so
    // collection does not rely on that action implementation detail.
    if (source.endsWith('.app')) {
      const archive = `${source}.tar.gz`;
      try { await stat(archive); }
      catch (error) {
        if (error.code !== 'ENOENT') throw error;
        execFileSync('tar', ['czf', archive, '-C', dirname(source), basename(source)]);
      }
      source = archive;
    }
    const name = assetName(plan.tag, platform, source);
    if (names.has(name)) throw new Error(`Duplicate artifact format: ${name}`);
    names.add(name);
    const target = join(outputDir, name);
    if (resolve(source) !== resolve(target)) await copyFile(source, target);
    const { path: _path, ...description } = await describe(target);
    assets.push(description);
  }
  if (assets.length !== expected.length || !expected.every(suffix => names.has(`OpenTag-${plan.tag}-${platform}${suffix}`))) throw new Error(`Missing required artifact formats for ${platform}`);
  const manifest = { schema: 1, tag: plan.tag, version: plan.version, commit: plan.commit, platform, assets };
  await writeFile(join(outputDir, `manifest-${platform}.json`), `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

export async function assembleRelease({ plan, directory }) {
  const files = await readdir(directory);
  const assets = [];
  const names = new Set();
  const manifestNames = PLATFORMS.map(platform => `manifest-${platform}.json`);
  for (const [index, manifestName] of manifestNames.entries()) {
    if (!files.includes(manifestName)) throw new Error(`Missing required platform manifest: ${manifestName}`);
    const manifest = JSON.parse(await readFile(join(directory, manifestName), 'utf8'));
    const platform = PLATFORMS[index];
    if (manifest.schema !== 1 || manifest.platform !== platform || manifest.commit !== plan.commit || manifest.tag !== plan.tag || manifest.version !== plan.version) throw new Error(`Mixed source/version in ${manifestName}`);
    const expectedNames = FORMATS[platform].map(suffix => `OpenTag-${plan.tag}-${platform}${suffix}`);
    if (!Array.isArray(manifest.assets) || manifest.assets.length !== expectedNames.length) throw new Error(`Incomplete package set in ${manifestName}`);
    for (const item of manifest.assets) {
      if (!expectedNames.includes(item.name) || names.has(item.name)) throw new Error(`Unexpected or duplicate release asset: ${item.name}`);
      const actual = await describe(join(directory, item.name));
      if (actual.sha256 !== item.sha256 || actual.size !== item.size) throw new Error(`Asset hash or size mismatch: ${item.name}`);
      names.add(item.name);
      assets.push(actual);
    }
  }
  const allowed = new Set([...manifestNames, ...names, 'SHA256SUMS', 'release-manifest.json']);
  if (files.some(file => !allowed.has(file))) throw new Error('Unexpected files in downloaded release artifacts');
  assets.sort((a, b) => a.name.localeCompare(b.name, 'en'));
  const manifestPath = join(directory, 'release-manifest.json');
  await writeFile(manifestPath, `${JSON.stringify({ schema: 1, ...plan, platforms: PLATFORMS, assets: assets.map(({ path: _path, ...item }) => item) }, null, 2)}\n`);
  assets.push(await describe(manifestPath));
  const sumsPath = join(directory, 'SHA256SUMS');
  await writeFile(sumsPath, assets.map(asset => `${asset.sha256}  ${asset.name}\n`).join(''));
  assets.push(await describe(sumsPath));
  return { assets };
}

export function releaseBody(plan, repository) {
  return `![OpenTag](https://raw.githubusercontent.com/${repository}/${plan.tag}/assets/brand/OpenTag-Brand-Kit/03-banners/opentag-github-light-1280x640.png)

OpenTag — Tag, Organize, Enjoy.

${plan.prerelease ? '**Development prerelease.**' : '**Release.**'} Source: [${plan.commit.slice(0, 12)}](https://github.com/${repository}/commit/${plan.commit}).
Feature parity and runtime qualification remain tracked in the [parity ledger](https://github.com/${repository}/blob/${plan.tag}/docs/PARITY.md).

| Download | Architecture | Signing / distribution |
| --- | --- | --- |
| Windows installer | x64 | unsigned; no Authenticode certificate |
| macOS DMG and app archive | Apple Silicon arm64 / Intel x64 | ad-hoc signed; not Apple notarized |
| Linux AppImage, DEB, RPM | x64 / arm64 | unsigned packages |
| Android UI preview (release APK) | arm64 | signed with the OpenTag application key; document editing and save-back integration in progress |
| iOS simulator UI preview ZIP | arm64 simulator, debug | unsigned simulator app; document editing/save-back in progress; not a device installer or App Store package |

All required build jobs passed before publication. This is build/test evidence, not proof of every device or full Mp3tag parity.
Verify downloads using SHA256SUMS; release-manifest.json records source commit, platform coverage, sizes and hashes.
`;
}

export async function publishRelease({ api, plan, assembled, repository }) {
  const metadata = { tag_name: plan.tag, target_commitish: plan.commit, name: `OpenTag ${plan.tag}`, body: releaseBody(plan, repository), draft: true, prerelease: plan.prerelease, make_latest: 'false' };
  let release = await api.findRelease(plan.tag);
  if (release && !release.draft) throw new Error('Refusing to overwrite an already published release');
  if (release) {
    if (release.target_commitish !== plan.commit) throw new Error('Existing draft belongs to a different source commit');
    release = await api.updateDraft(release.id, metadata);
  } else release = await api.createDraft(metadata);
  if (!release.draft) throw new Error('Release must remain draft until every upload is verified');
  for (const old of await api.listAssets(release.id)) await api.deleteAsset(old.id);
  for (const asset of assembled.assets) await api.uploadAsset(release, asset);
  const uploaded = await api.listAssets(release.id);
  if (uploaded.length !== assembled.assets.length || assembled.assets.some(asset => !uploaded.some(item => item.name === asset.name && item.size === asset.size && item.state === 'uploaded' && item.digest === `sha256:${asset.sha256}`))) throw new Error('GitHub release assets do not match the complete verified package set (names, sizes, states and SHA256 digests required)');
  const current = await api.findRelease(plan.tag);
  if (!current?.draft || current.id !== release.id) throw new Error('Release draft changed during upload; refusing to publish');
  await api.publish(release.id, { draft: false, prerelease: plan.prerelease, make_latest: plan.prerelease ? 'false' : 'true' });
}

export class GitHubApi {
  constructor(repository, token) {
    if (!/^[\w.-]+\/[\w.-]+$/.test(repository) || !token) throw new Error('GitHub repository and token are required');
    this.base = `https://api.github.com/repos/${repository}`;
    this.token = token;
  }
  async request(path, method = 'GET', body) {
    const response = await fetch(`${this.base}${path}`, { method, headers: { Authorization: `Bearer ${this.token}`, Accept: 'application/vnd.github+json', 'X-GitHub-Api-Version': '2022-11-28', ...(body ? { 'Content-Type': 'application/json' } : {}) }, body: body ? JSON.stringify(body) : undefined });
    if (response.status === 404 && method === 'GET') return null;
    if (!response.ok) throw new Error(`GitHub ${method} ${path}: ${response.status} ${await response.text()}`);
    return response.status === 204 ? null : response.json();
  }
  async findRelease(tag) {
    // The tag endpoint only promises published releases. Authenticated listing
    // also returns drafts, including a draft whose tag has not been created yet.
    for (let page = 1; ; page++) {
      const batch = await this.request(`/releases?per_page=100&page=${page}`);
      if (!Array.isArray(batch)) throw new Error('Cannot enumerate repository releases');
      const release = batch.find(item => item.tag_name === tag);
      if (release) return release;
      if (batch.length < 100) return null;
    }
  }
  createDraft(body) { return this.request('/releases', 'POST', body); }
  updateDraft(id, body) { return this.request(`/releases/${id}`, 'PATCH', body); }
  publish(id, body) { return this.request(`/releases/${id}`, 'PATCH', body); }
  deleteAsset(id) { return this.request(`/releases/assets/${id}`, 'DELETE'); }
  async listAssets(id) {
    const items = [];
    for (let page = 1; ; page++) {
      const batch = await this.request(`/releases/${id}/assets?per_page=100&page=${page}`);
      if (!Array.isArray(batch)) throw new Error('Cannot enumerate draft assets');
      items.push(...batch);
      if (batch.length < 100) return items;
    }
  }
  async uploadAsset(release, asset) {
    const url = new URL(release.upload_url.split('{')[0]);
    if (url.origin !== 'https://uploads.github.com') throw new Error('Unexpected GitHub upload host');
    url.searchParams.set('name', asset.name);
    const response = await fetch(url, { method: 'POST', headers: { Authorization: `Bearer ${this.token}`, 'Content-Type': 'application/octet-stream', 'Content-Length': String(asset.size) }, body: await openAsBlob(asset.path) });
    if (!response.ok) throw new Error(`GitHub upload ${asset.name}: ${response.status} ${await response.text()}`);
  }
}

function git(...args) { return execFileSync('git', args, { encoding: 'utf8' }).trim(); }
function tagCommit(tag) {
  checkedTag(tag);
  const result = execFileSync('git', ['for-each-ref', '--format=%(refname)', `refs/tags/${tag}`], { encoding: 'utf8' }).trim();
  return result ? git('rev-parse', `refs/tags/${tag}^{commit}`) : null;
}
async function walk(root, accept) {
  const found = [];
  for (const entry of await readdir(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) found.push(...await walk(path, accept));
    else if (entry.isFile() && accept(path)) found.push(path);
  }
  return found;
}

async function main(command) {
  const root = process.cwd();
  if (command === 'prepare') {
    const versions = await readVersions(root);
    const tag = process.env.RELEASE_TAG || `v${versions.package}`;
    const prerelease = process.env.REQUESTED_PRERELEASE ? JSON.parse(process.env.REQUESTED_PRERELEASE) : true;
    const plan = releasePlan({ versions, tag, prerelease, head: git('rev-parse', 'HEAD'), expectedHead: git('rev-parse', `${process.env.GITHUB_SHA}^{commit}`), tagCommit: tagCommit(tag) });
    const api = new GitHubApi(process.env.GITHUB_REPOSITORY, process.env.GH_TOKEN);
    const existing = await api.findRelease(plan.tag);
    if (existing && !existing.draft) throw new Error('Refusing to overwrite an already published release');
    for (const [key, value] of Object.entries(plan)) await appendFile(process.env.GITHUB_OUTPUT, `${key}=${value}\n`);
    console.log(`Validated ${plan.tag} at ${plan.commit}; prerelease=${plan.prerelease}`);
    return;
  }
  const versions = await readVersions(root);
  const plan = releasePlan({ versions, tag: process.env.RELEASE_TAG, prerelease: JSON.parse(process.env.RELEASE_PRERELEASE || 'true'), head: git('rev-parse', 'HEAD'), expectedHead: process.env.RELEASE_COMMIT, tagCommit: tagCommit(process.env.RELEASE_TAG) });
  const outputDir = resolve(process.env.RELEASE_ASSETS || 'release-assets');
  if (command === 'collect-desktop') {
    const sources = JSON.parse(process.env.TAURI_ARTIFACT_PATHS || '[]');
    await collectAssets({ plan, platform: process.env.RELEASE_PLATFORM, sourcePaths: sources, outputDir });
  } else if (command === 'restore-android-key') {
    for (const key of ['ANDROID_KEYSTORE_BASE64', 'ANDROID_KEYSTORE_PASSWORD', 'ANDROID_KEY_ALIAS', 'ANDROID_KEY_PASSWORD']) if (!process.env[key]) throw new Error(`Required signing secret is missing: ${key}`);
    const bytes = Buffer.from(process.env.ANDROID_KEYSTORE_BASE64, 'base64');
    if (bytes.length < 100) throw new Error('Invalid Android keystore');
    await writeFile(process.env.ANDROID_KEYSTORE_PATH, bytes, { mode: 0o600 });
  } else if (command === 'collect-android') {
    const paths = await walk(join(root, 'src-tauri/gen/android/app/build/outputs/apk'), path => path.endsWith('.apk'));
    if (paths.length !== 1 || /unsigned|debug/.test(paths[0])) throw new Error('Expected exactly one signed release APK');
    const tools = join(process.env.ANDROID_HOME, 'build-tools/37.0.0');
    const signature = execFileSync(join(tools, 'apksigner'), ['verify', '--verbose', '--print-certs', paths[0]], { encoding: 'utf8' });
    if (/CN=Android Debug/i.test(signature)) throw new Error('Debug signing certificates are not release keys');
    const info = execFileSync(join(tools, 'aapt'), ['dump', 'badging', paths[0]], { encoding: 'utf8' });
    if (!info.includes(`versionName='${plan.version}'`) || !/^native-code: 'arm64-v8a'\s*$/m.test(info)) throw new Error('APK version/ABI differs from the release plan');
    console.log(signature);
    await collectAssets({ plan, platform: 'android-arm64', sourcePaths: paths, outputDir });
  } else if (command === 'collect-ios') {
    const app = join(root, 'src-tauri/gen/apple/build/arm64-sim/OpenTag.app');
    const executable = execFileSync('plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', join(app, 'Info.plist')], { encoding: 'utf8' }).trim();
    const version = execFileSync('plutil', ['-extract', 'CFBundleShortVersionString', 'raw', '-o', '-', join(app, 'Info.plist')], { encoding: 'utf8' }).trim();
    const info = execFileSync('xcrun', ['vtool', '-show-build', join(app, executable)], { encoding: 'utf8' });
    if (version !== plan.version || !info.includes('IOSSIMULATOR')) throw new Error('Expected matching-version iOS simulator app');
    await mkdir(outputDir, { recursive: true });
    const zip = join(outputDir, assetName(plan.tag, 'ios-arm64-simulator-preview', 'OpenTag.app.zip'));
    execFileSync('ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', app, zip]);
    await collectAssets({ plan, platform: 'ios-arm64-simulator-preview', sourcePaths: [zip], outputDir });
  } else if (command === 'publish') {
    const assembled = await assembleRelease({ plan, directory: outputDir });
    await publishRelease({ api: new GitHubApi(process.env.GITHUB_REPOSITORY, process.env.GH_TOKEN), plan, assembled, repository: process.env.GITHUB_REPOSITORY });
  } else throw new Error(`Unknown release command: ${command}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv[2]).catch(error => { console.error(error.message); process.exitCode = 1; });
}
