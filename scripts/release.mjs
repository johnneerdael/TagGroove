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

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

export function parseVersion(version) {
  const match = SEMVER.exec(version);
  if (!match || match[4]?.split('.').some(part => /^0\d+$/.test(part))) throw new Error(`Invalid semantic version: ${version}`);
  return { major: Number(match[1]), minor: Number(match[2]), patch: Number(match[3]), prerelease: match[4], build: match[5] };
}

export function compareVersions(a, b) {
  const [x, y] = [parseVersion(a), parseVersion(b)];
  for (const key of ['major', 'minor', 'patch']) if (x[key] !== y[key]) return x[key] < y[key] ? -1 : 1;
  if (!x.prerelease || !y.prerelease) return (x.prerelease ? -1 : 0) + (y.prerelease ? 1 : 0);
  const [p, q] = [x.prerelease.split('.'), y.prerelease.split('.')];
  for (let i = 0; i < Math.min(p.length, q.length); i++) {
    if (p[i] === q[i]) continue;
    const [m, n] = [/^\d+$/.test(p[i]), /^\d+$/.test(q[i])];
    if (m && n) return Number(p[i]) < Number(q[i]) ? -1 : 1;
    if (m !== n) return m ? -1 : 1;
    return p[i] < q[i] ? -1 : 1;
  }
  return Math.sign(p.length - q.length);
}

export function baseVersion(versions) {
  const version = versions.package;
  parseVersion(version);
  if (Object.values(versions).some(value => value !== version)) throw new Error('package.json, Cargo.toml and tauri.conf.json versions must match');
  return version;
}

// Merges to main release <major>.<minor>.<n>, where n counts first-parent
// commits since the base version was set. Tauri derives the Android
// versionCode as major * 1000000 + minor * 1000 + patch, so each part < 1000.
export function automaticVersion(base, distance) {
  const { major, minor, patch, prerelease, build } = parseVersion(base);
  if (prerelease || build || patch !== 0) throw new Error(`Automatic releases need a plain <major>.<minor>.0 base version, not ${base}`);
  if (!Number.isInteger(distance) || distance < 0) throw new Error('Invalid commit distance');
  if (major > 2000 || minor > 999 || distance > 999) throw new Error(`Patch ${distance} exceeds the Android versionCode range; bump the base minor version`);
  return `${major}.${minor}.${distance}`;
}

// Apple bundle versions must be three numeric components; Tauri strips the
// prerelease suffix the same way when it builds the iOS app.
export function appleVersion(version) {
  const { major, minor, patch } = parseVersion(version);
  return `${major}.${minor}.${patch}`;
}

// Tauri's default Android versionCode; versionName keeps the full version.
export function androidVersionCode(version) {
  const { major, minor, patch } = parseVersion(version);
  return major * 1000000 + minor * 1000 + patch;
}

// An explicit tag either releases the base version itself or a suffixed
// prerelease, so it can never take a number that automatic releases will use.
export function explicitVersion(base, tag) {
  checkedTag(tag);
  const version = tag.slice(1);
  if (version !== base && !parseVersion(version).prerelease) throw new Error(`Explicit tag ${tag} must equal v${base} or carry a prerelease suffix such as v${base}-beta.1`);
  return version;
}

export function releasePlan({ versions, head, expectedHead, tagCommit, tag, prerelease = false }) {
  const version = baseVersion(versions);
  tag ||= `v${version}`;
  if (tag !== `v${version}`) throw new Error('Release tag must exactly match the application version');
  if (!/^[a-f0-9]{40}$/.test(head) || head !== expectedHead) throw new Error('Checked-out commit does not match the selected workflow commit');
  if (tagCommit && tagCommit !== head) throw new Error('Existing release tag points at a different commit');
  if (typeof prerelease !== 'boolean') throw new Error('Prerelease selection must be boolean');
  // 0.x versions are full releases so they are easy to find and install;
  // only an explicit request or a semver prerelease suffix (-beta.1) marks one.
  return { tag, version, commit: head, prerelease: prerelease || Boolean(parseVersion(version).prerelease) };
}

export const VERSION_FILES = ['package.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml'];
const CARGO_PACKAGE = /^\[package\][^]*?(?=^\[|$(?![^]))/m;
const CARGO_VERSION = /^(version\s*=\s*["'])([^"']+)(["']\s*(?:#.*)?)$/m;

export function parseVersions([npm, tauri, cargo]) {
  const version = cargo.match(CARGO_PACKAGE)?.[0].match(CARGO_VERSION)?.[2];
  if (!version) throw new Error('src-tauri/Cargo.toml must contain an explicit [package] version');
  return { package: JSON.parse(npm).version, tauri: JSON.parse(tauri).version, cargo: version };
}

export async function readVersions(root) {
  return parseVersions(await Promise.all(VERSION_FILES.map(file => readFile(join(root, file), 'utf8'))));
}

// Paths that never reach a package. Merges that only touch these neither get
// a release nor advance the automatic patch number.
export const NON_SHIPPING_PATHS = ['docs/**', 'mkdocs.yml', '*.md', '.github/workflows/docs*.yml', '.github/workflows/pages-deploy.yml', 'tests/docs-screens/**', 'scripts/docs-*', 'scripts/docs-*/**'];
const SHIPPING_PATHSPEC = ['.', ...NON_SHIPPING_PATHS.map(path => `:(exclude,glob)${path}`)];

// True when shipping files differ between two commits (or a commit's first parent).
export function shipsChanges(root, from, to) {
  try {
    execFileSync('git', ['diff', '--quiet', from, to, '--', ...SHIPPING_PATHSPEC], { cwd: root, stdio: 'ignore' });
    return false;
  } catch (error) {
    if (error.status === 1) return true;
    throw error;
  }
}

// Finds the first-parent commit that set the current base version and counts
// the later first-parent commits that change shipping files. Only file
// contents matter, so a version bump merged from a branch counts from its
// merge commit on main.
export function versionOrigin(root, base) {
  const run = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  const attempt = (...args) => { try { return run(...args); } catch { return null; } };
  const blobs = commit => attempt('rev-parse', ...VERSION_FILES.map(file => `${commit}:${file}`));
  const versionAt = commit => {
    try { return baseVersion(parseVersions(VERSION_FILES.map(file => run('show', `${commit}:${file}`)))); } catch { return null; }
  };
  const commits = run('rev-list', '--first-parent', 'HEAD').split('\n');
  if (versionAt(commits[0]) !== base) throw new Error(`HEAD does not contain base version ${base}`);
  let origin = 0;
  let current = blobs(commits[0]);
  for (; commits[origin + 1]; origin++) {
    const previous = blobs(commits[origin + 1]);
    if (previous !== current && versionAt(commits[origin + 1]) !== base) break;
    current = previous;
  }
  const shipping = commits.slice(0, origin).filter(commit => shipsChanges(root, `${commit}^1`, commit));
  return { commit: commits[origin], distance: shipping.length, shipped: shipping[0] ?? commits[origin] };
}

export async function stampVersion(root, version) {
  parseVersion(version);
  const edit = async (file, change, optional = false) => {
    const path = join(root, file);
    let text;
    try { text = await readFile(path, 'utf8'); }
    catch (error) { if (optional && error.code === 'ENOENT') return; throw error; }
    await writeFile(path, change(text));
  };
  const json = update => text => { const data = JSON.parse(text); update(data); return `${JSON.stringify(data, null, 2)}\n`; };
  const once = (pattern, replacement, file) => text => {
    const matches = text.match(new RegExp(pattern.source, `${pattern.flags.replace('g', '')}g`));
    if (matches?.length !== 1) throw new Error(`Expected one version entry in ${file}`);
    return text.replace(pattern, replacement);
  };
  await edit('package.json', json(data => { data.version = version; }));
  await edit('package-lock.json', json(data => {
    if (!data.packages?.['']) throw new Error('package-lock.json has no root package');
    data.version = version;
    data.packages[''].version = version;
  }));
  await edit('src-tauri/tauri.conf.json', json(data => { data.version = version; }));
  await edit('src-tauri/Cargo.toml', text => {
    const section = text.match(CARGO_PACKAGE)?.[0];
    if (!section || !CARGO_VERSION.test(section)) throw new Error('src-tauri/Cargo.toml must contain an explicit [package] version');
    return text.replace(section, section.replace(CARGO_VERSION, `$1${version}$3`));
  });
  // --locked builds fail unless the lockfile agrees with Cargo.toml.
  await edit('Cargo.lock', once(/(\[\[package\]\]\r?\nname = "opentag"\r?\nversion = ")[^"]+(")/, `$1${version}$2`, 'Cargo.lock'));
  const bundle = appleVersion(version);
  for (const key of ['CFBundleShortVersionString', 'CFBundleVersion']) {
    await edit('src-tauri/gen/apple/project.yml', once(new RegExp(`^(\\s*${key}:\\s*).*$`, 'm'), `$1"${bundle}"`, 'project.yml'), true);
    await edit('src-tauri/gen/apple/opentag_iOS/Info.plist', once(new RegExp(`(<key>${key}</key>\\s*<string>)[^<]*(</string>)`), `$1${bundle}$2`, 'Info.plist'), true);
  }
  const stamped = await readVersions(root);
  if (baseVersion(stamped) !== version) throw new Error('Stamped versions do not match');
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

export function releaseBody(plan, repository, notes = '') {
  return `![OpenTag](https://raw.githubusercontent.com/${repository}/${plan.tag}/assets/brand/OpenTag-Brand-Kit/03-banners/opentag-github-light-1280x640.png)

OpenTag — Tag, Organize, Enjoy.

${plan.prerelease ? '**Development prerelease.**' : `**Release${plan.version.startsWith('0.') ? ' (pre-1.0)' : ''}.**`} Source: [${plan.commit.slice(0, 12)}](https://github.com/${repository}/commit/${plan.commit}).
Feature parity and runtime qualification remain tracked in the [parity ledger](https://github.com/${repository}/blob/${plan.tag}/docs/PARITY.md).
${notes.trim() ? `\n${notes.trim()}\n` : ''}
## Downloads

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

// An older run that is rerun after a newer release must not take "latest".
export function makeLatest(plan, latest) {
  if (plan.prerelease) return 'false';
  const version = latest?.tag_name?.replace(/^v/, '');
  try { return !version || compareVersions(plan.version, version) > 0 ? 'true' : 'false'; }
  catch { return 'true'; }
}

export async function publishRelease({ api, plan, assembled, repository, notes = '' }) {
  const metadata = { tag_name: plan.tag, target_commitish: plan.commit, name: `OpenTag ${plan.tag}`, body: releaseBody(plan, repository, notes), draft: true, prerelease: plan.prerelease, make_latest: 'false' };
  let release = await api.findRelease(plan.tag);
  if (release && !release.draft) {
    // A rerun after a successful publication is a no-op, never a duplicate.
    if (release.target_commitish === plan.commit) return { skipped: true, release };
    throw new Error('Refusing to overwrite an already published release');
  }
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
  await api.publish(release.id, { draft: false, prerelease: plan.prerelease, make_latest: makeLatest(plan, await api.latestRelease()) });
  return { skipped: false, release };
}

// The newest earlier release tag reachable from this commit, preferring full
// releases so a full release lists everything since the previous full one.
export function previousTag(tags, plan) {
  const earlier = tags.filter(tag => {
    if (tag === plan.tag || !/^v/.test(tag)) return false;
    try { return compareVersions(tag.slice(1), plan.version) < 0; } catch { return false; }
  });
  const stable = earlier.filter(tag => !parseVersion(tag.slice(1)).prerelease);
  const pool = !plan.prerelease && stable.length ? stable : earlier;
  return pool.sort((a, b) => compareVersions(b.slice(1), a.slice(1)))[0] ?? null;
}

export async function releaseNotes({ api, plan, previous, log }) {
  try {
    const { body } = await api.generateNotes({ tag_name: plan.tag, target_commitish: plan.commit, ...(previous ? { previous_tag_name: previous } : {}) });
    if (body?.trim()) return body;
  } catch (error) { console.warn(`Generated release notes unavailable: ${error.message}`); }
  const lines = log().split('\n').filter(Boolean);
  if (!lines.length) return '';
  return `## Changes${previous ? ` since ${previous}` : ''}\n\n${lines.map(line => `- ${line}`).join('\n')}\n`;
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
  latestRelease() { return this.request('/releases/latest'); }
  generateNotes(body) { return this.request('/releases/generate-notes', 'POST', body); }
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
    const base = baseVersion(await readVersions(root));
    const requested = process.env.RELEASE_TAG || '';
    let version;
    if (requested) version = explicitVersion(base, requested);
    else {
      // Other branches would claim numbers that later main merges need.
      const branch = process.env.RELEASE_BRANCH || 'main';
      if (process.env.DRY_RUN !== 'true') {
        try { git('merge-base', '--is-ancestor', 'HEAD', `refs/remotes/origin/${branch}`); }
        catch { throw new Error(`Automatic versions are only published for commits on ${branch}; pass an explicit prerelease tag or use dry_run`); }
      }
      const origin = versionOrigin(root, base);
      version = automaticVersion(base, origin.distance);
      console.log(`Base ${base} was set by ${origin.commit}; ${origin.distance} shipping first-parent commit(s) since, last ${origin.shipped}`);
    }
    const tag = `v${version}`;
    const prerelease = process.env.REQUESTED_PRERELEASE ? JSON.parse(process.env.REQUESTED_PRERELEASE) : false;
    const head = git('rev-parse', 'HEAD');
    const released = tagCommit(tag);
    // A docs-only merge keeps the previous number; its notes go out with the next code release.
    if (!requested && released && released !== head && !shipsChanges(root, released, head)) {
      const summary = `${head} changes nothing that ships since ${tag} at ${released}; no release.`;
      for (const [key, value] of Object.entries({ tag, version, commit: head, prerelease, skip: true })) await appendFile(process.env.GITHUB_OUTPUT, `${key}=${value}\n`);
      if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `${summary}\n`);
      console.log(summary);
      return;
    }
    const plan = releasePlan({ versions: { package: version, cargo: version, tauri: version }, tag, prerelease, head, expectedHead: git('rev-parse', `${process.env.RELEASE_SOURCE_SHA || process.env.GITHUB_SHA}^{commit}`), tagCommit: released });
    const api = new GitHubApi(process.env.GITHUB_REPOSITORY, process.env.GH_TOKEN);
    const existing = await api.findRelease(plan.tag);
    let skip = false;
    if (existing && !existing.draft) {
      if (tagCommit(plan.tag) !== plan.commit) throw new Error('Refusing to overwrite an already published release');
      skip = true;
    }
    for (const [key, value] of Object.entries({ ...plan, skip })) await appendFile(process.env.GITHUB_OUTPUT, `${key}=${value}\n`);
    const summary = skip ? `${plan.tag} is already published from ${plan.commit}; nothing to build.` : `Validated ${plan.tag} at ${plan.commit}; prerelease=${plan.prerelease}`;
    if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, `${summary}\n`);
    console.log(summary);
    return;
  }
  if (command === 'stamp') {
    const version = process.argv[3] || process.env.RELEASE_TAG?.replace(/^v/, '');
    if (!version) throw new Error('Usage: release.mjs stamp <version>');
    await stampVersion(root, version);
    console.log(`Stamped version ${version} (not committed)`);
    return;
  }
  const versions = await readVersions(root);
  const plan = releasePlan({ versions, tag: process.env.RELEASE_TAG, prerelease: JSON.parse(process.env.RELEASE_PRERELEASE || 'false'), head: git('rev-parse', 'HEAD'), expectedHead: process.env.RELEASE_COMMIT, tagCommit: tagCommit(process.env.RELEASE_TAG) });
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
    if (!info.includes(`versionName='${plan.version}'`) || !info.includes(`versionCode='${androidVersionCode(plan.version)}'`) || !/^native-code: 'arm64-v8a'\s*$/m.test(info)) throw new Error('APK version/ABI differs from the release plan');
    console.log(signature);
    await collectAssets({ plan, platform: 'android-arm64', sourcePaths: paths, outputDir });
  } else if (command === 'collect-ios') {
    const app = join(root, 'src-tauri/gen/apple/build/arm64-sim/OpenTag.app');
    const executable = execFileSync('plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', join(app, 'Info.plist')], { encoding: 'utf8' }).trim();
    const version = execFileSync('plutil', ['-extract', 'CFBundleShortVersionString', 'raw', '-o', '-', join(app, 'Info.plist')], { encoding: 'utf8' }).trim();
    const info = execFileSync('xcrun', ['vtool', '-show-build', join(app, executable)], { encoding: 'utf8' });
    if (version !== appleVersion(plan.version) || !info.includes('IOSSIMULATOR')) throw new Error('Expected matching-version iOS simulator app');
    await mkdir(outputDir, { recursive: true });
    const zip = join(outputDir, assetName(plan.tag, 'ios-arm64-simulator-preview', 'OpenTag.app.zip'));
    execFileSync('ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', app, zip]);
    await collectAssets({ plan, platform: 'ios-arm64-simulator-preview', sourcePaths: [zip], outputDir });
  } else if (command === 'publish') {
    const assembled = await assembleRelease({ plan, directory: outputDir });
    const api = new GitHubApi(process.env.GITHUB_REPOSITORY, process.env.GH_TOKEN);
    const previous = previousTag(git('tag', '--merged', 'HEAD', '--list', 'v*').split('\n').filter(Boolean), plan);
    const notes = await releaseNotes({ api, plan, previous, log: () => git('log', '--first-parent', '--max-count=100', '--format=%s (%h)', ...(previous ? [`${previous}..HEAD`] : ['HEAD'])) });
    const result = await publishRelease({ api, plan, assembled, repository: process.env.GITHUB_REPOSITORY, notes });
    console.log(result.skipped ? `${plan.tag} was already published from ${plan.commit}; nothing to do` : `Published ${plan.tag}`);
  } else throw new Error(`Unknown release command: ${command}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv[2]).catch(error => { console.error(error.message); process.exitCode = 1; });
}
