# Publishing OpenTag releases

`.github/workflows/release.yml` builds the same validated commit for five desktop targets, an Android arm64 APK, and an iOS arm64 simulator app. Only the final job can publish. It waits for every required build, verifies platform manifests and SHA256 hashes, creates or reuses an unpublished draft, uploads the complete asset set, verifies GitHub's asset digests, and then publishes.

Every merge to `main` publishes a full GitHub release (not a prerelease) marked as the latest release, so testers can always download the newest build. Pre-1.0 releases say "Release (pre-1.0)" in their notes. Android and iOS packages are **UI previews**: document editing and save-back integration is still in progress. Desktop feature parity and remaining runtime qualification are recorded in [PARITY.md](PARITY.md).

## How versions are derived

`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.lock` (the `opentag` entry), `package-lock.json` and the iOS project files carry the **base version**, currently `0.2.0`. It must be a plain `<major>.<minor>.0`.

For a push to `main`, the release version is `<major>.<minor>.<n>`, where `n` is the number of first-parent commits on `main` since the commit that set the base version that change shipping files (see [Docs-only merges](#docs-only-merges)). The commit that lands the base version is `v0.2.0`; the next code merge is `v0.2.1`, then `v0.2.2`, and so on. Each merge adds one first-parent commit, however many commits its branch had, so versions follow the order of merges, increase strictly and never depend on the runner, the clock or earlier releases. The same commit always gets the same version, so reruns are deterministic.

`scripts/release.mjs prepare` computes the version and tag once; every build job then runs `scripts/release.mjs stamp`, which writes that version into the files listed above in its checkout. Stamping includes `Cargo.lock` so `--locked` builds still pass. Stamped versions are never committed. The app's update check, the installer metadata and the Android `versionName` report the full release version, including any prerelease suffix. Apple requires `CFBundleShortVersionString` and `CFBundleVersion` to be numeric, so the iOS project gets the numeric `<major>.<minor>.<patch>` (`0.3.0` for `0.3.0-beta.1`), as Tauri also does when it builds the app. The Android `versionCode` ignores the suffix too, so `0.3.0-beta.1` and `0.3.0` share code `3000`.

Tauri computes the Android `versionCode` as `major * 1000000 + minor * 1000 + patch`, so the patch must stay below 1000. Preparation fails once `main` reaches 1000 merges for one base version; bump the minor version before then.

Release runs share one serialized concurrency group with [`queue: max`](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency), so up to 100 runs wait their turn instead of the newest replacing the pending one. Every code merge, tag push and manual run therefore gets its own release, in roughly first-in-first-out order. GitHub does not guarantee that order, so a later merge can occasionally publish before an earlier one; the earlier one still publishes its own version but does not take "latest" from the newer release. If more than 100 runs are waiting, GitHub cancels the extras; rerun a cancelled merge's workflow to publish its version.

### Docs-only merges

A merge that changes only files that never reach a package gets no release and does not advance `n`. These paths are listed in `NON_SHIPPING_PATHS` in `scripts/release.mjs`:

- `docs/**`, `mkdocs.yml` (including `docs/site-requirements.txt`)
- Markdown files at the repository root, such as `README.md` (Markdown elsewhere ships)
- `.github/workflows/docs*.yml` and `.github/workflows/pages-deploy.yml`
- `tests/docs-screens/**` and `scripts/docs-*`

The workflow still runs for such a merge, but preparation finds that the version it derives is already released from a commit with identical shipping files and skips every build, so the run is green and costs only the preparation job. The next code merge gets the next number, and its generated notes list every pull request since the previous release, including the docs-only ones. A merge that touches both docs and shipping files is released normally. If a docs-only merge's run happens to start before the preceding code merge's run has published, it publishes that version itself; the shipped files are identical, and the code merge's run then skips.

### Bumping the minor or major version

Change the base version in one commit, for example to `0.3.0` or `1.0.0`, using the stamping script so every file and lockfile agrees:

```sh
node scripts/release.mjs stamp 0.3.0
git commit -am "release: start 0.3 series"
```

Merging that commit to `main` publishes `v0.3.0`, and later merges publish `v0.3.1`, `v0.3.2`, and so on. Only increase the base version, and always reset the patch to `0`; preparation rejects a base such as `0.3.4`.

### Explicit releases and prereleases

Explicit tags cannot use a number that automatic releases will take later. A tag must either equal `v` plus the base version, or carry a prerelease suffix:

```sh
gh workflow run release.yml --ref main -f tag=v0.3.0-beta.1
```

A suffix such as `-beta.1` always publishes a prerelease, even if the `prerelease` input is false. Pushing the same tag on the intended landed commit also works:

```sh
git tag v0.3.0-beta.1 <release-commit>
git push origin v0.3.0-beta.1
```

Running the workflow manually with an empty `tag` derives the automatic version for the selected commit, exactly like a push to `main`. That only publishes commits already on `main`, because a commit from another branch would claim a number that a later merge needs; use a suffixed tag for other branches.

### Dry run

The `dry_run` input resolves the version and tag, stamps a checkout and checks that `cargo metadata --locked` and `npm ci` accept the stamped files. It builds and publishes nothing:

```sh
gh workflow run release.yml --ref <branch> -F dry_run=true
```

## Publication rules

Land the workflow and release source on the repository's default branch before publication. GitHub's current [release API permissions](https://docs.github.com/en/rest/releases/releases#create-a-release) restrict creating releases when the target commit changes workflow files relative to the default branch; the normal workflow token cannot acquire the extra Workflows permission. Every automatic release targets a commit on `main`, so this never applies to it.

Every build checks out the resolved immutable commit, rather than a moving branch. The publish step creates the tag `v<version>` on that exact commit when it publishes the release. Any existing tag must already point to that commit.

Do not move a published tag or overwrite a public release. Releases are idempotent:

- If a published release for that tag and commit already exists, preparation reports it and the run skips every build, so rerunning a finished run publishes nothing new.
- A published release or a tag for that version on a different commit fails the run.
- A failed draft upload remains unpublished. Rerunning the failed jobs reuses the plan from the original run and updates the same draft. The final job may replace stale assets only on a draft for that exact commit.
- A rerun of an older run never takes "latest" from a newer published release.

Release runs are serialized so a manual run, a tag push and a merge cannot race one another on a draft, a tag or "latest".

## Release notes

The publish step asks GitHub to [generate release notes](https://docs.github.com/en/rest/releases/releases#generate-release-notes-content-for-a-release) since the newest earlier release tag reachable from the commit, which lists the merged pull request titles. If that request fails, the notes fall back to the first-parent commit subjects from `git log`. The download table, the signing status (Windows and Linux packages are unsigned, macOS builds are ad-hoc signed and not notarized) and the mobile preview limitations follow the notes.

## Required build outputs

| Build | Runner | Assets and status |
| --- | --- | --- |
| Windows x64 | `windows-2022` | NSIS installer; unsigned, no Authenticode certificate |
| macOS Apple Silicon | `macos-15` | DMG and app archive; ad-hoc signed, not notarized |
| macOS Intel | `macos-15-intel` | DMG and app archive; ad-hoc signed, not notarized |
| Linux x64 | `ubuntu-24.04` | AppImage, DEB, RPM; unsigned |
| Linux arm64 | `ubuntu-24.04-arm` | AppImage, DEB, RPM; unsigned |
| Android arm64 UI preview | `ubuntu-24.04` | Release APK signed with the persistent OpenTag application key; document editing/save-back still in progress |
| iOS arm64 simulator UI preview | `macos-15` | Unsigned debug `.app` ZIP for the simulator; document editing/save-back still in progress; not an iPhone/iPad installer or App Store package |

Desktop jobs run the frontend build before native tests, then UI tests, Rust formatting, workspace tests and strict clippy. Tauri's [GitHub pipeline guidance](https://v2.tauri.app/distribute/pipelines/github/) and [tauri-action v1](https://github.com/tauri-apps/tauri-action/tree/v1) provide the build path. The action receives no release ID or tag name: it builds only. Its `artifactPaths` output feeds the collection script. The collector supplies a tested tar fallback for `.app` directories if the action does not create their app archives.

Linux packages are built on Ubuntu 24.04; compatibility with older distributions is not established by compilation. The [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) documents native arm64 Linux and separate macOS Intel/arm64 runners. Platform jobs use separate artifact names and preserve their OS/architecture in public download filenames.

Each builder uploads its packages plus `manifest-<platform>.json` as workflow artifacts. The final publisher requires all seven manifests and the exact expected package formats, verifies their commit/version, rejects unexpected files or duplicate formats, and streams every package through SHA256. It publishes 13 packages plus `release-manifest.json` and `SHA256SUMS`. The checksum list includes all packages and the release manifest; it does not recursively include itself. The public manifest records source identity, platform coverage, byte sizes and hashes.

The release body embeds the supplied [OpenTag banner](../assets/brand/OpenTag-Brand-Kit/03-banners/opentag-github-light-1280x640.png) from the released tag and links the matching parity ledger. Signing and mobile limitations appear alongside the download table.

## Android signing setup

Repository Actions secrets must contain:

- `ANDROID_KEYSTORE_BASE64`: base64-encoded persistent release keystore.
- `ANDROID_KEYSTORE_PASSWORD`: keystore password.
- `ANDROID_KEY_ALIAS`: application signing-key alias.
- `ANDROID_KEY_PASSWORD`: signing-key password.

The workflow restores the keystore with restricted permissions under the runner's temporary directory. Gradle reads `ANDROID_KEYSTORE_PATH`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, and `ANDROID_KEY_PASSWORD`; no credentials file is committed. Secret values are neither printed nor uploaded, and the temporary keystore is removed even on build failure.

The current key has an owner-managed private backup under `~/.config/opentag/signing`. Preserve it securely for future updates: rotating the application key can prevent existing installations from accepting later APKs. Do not add the keystore, passwords, or base64 key to source, logs, release assets, or documentation. See [Tauri Android signing](https://v2.tauri.app/distribute/sign/android/) for the underlying Gradle signing requirements.

Android builds use JDK 17, SDK command-line tools 22 (`15859902`), platform `android-37.0`, build tools `37.0.0`, and NDK `28.2.13676358`. API 37's platform package includes the minor version: `platforms;android-37` does not exist. Before upload, the workflow verifies the APK signature, rejects the Android debug certificate, and checks the application version and sole `arm64-v8a` ABI. Missing signing secrets fail the required Android job and prevent publication; the pipeline never substitutes an unsigned or debug APK.

iOS uses `--target aarch64-sim --debug --no-sign --ci`. Packaging checks the version and `IOSSIMULATOR` binary platform. There are no Apple distribution certificates or provisioning profiles in this pipeline. Device signing, TestFlight and App Store distribution require a separate qualified signing/release path.

## Verification and failure handling

Before committing workflow changes:

```sh
node --test tests/release.test.mjs
actionlint -ignore 'unexpected key "queue" for "concurrency" section' .github/workflows/release.yml
```

actionlint 1.7.12 does not yet recognise the documented `concurrency.queue` key ([rhysd/actionlint#654](https://github.com/rhysd/actionlint/issues/654)); the flag ignores only that error.

The script tests cover automatic version derivation from a real git history (merge commits, multi-commit branches, base bumps), the Android versionCode limit, explicit-tag rules, docs-only merges keeping their number, stamping every manifest and `Cargo.lock`, numeric Apple bundle versions, rerun no-ops, "latest" selection, release-note fallback, version/tag/commit mismatch, forced development prereleases, exact platform coverage, missing and mixed manifests, tampered local files, duplicate asset formats, already published releases, upload failures, missing/wrong server SHA256 digests, draft pagination, publication order, banner/signing notes, and app archive fallback. They do not prove that the remote workflow ran or that built packages install correctly.

Inspect the actual workflow run and release after publication:

```sh
gh run list --workflow release.yml
gh release view v0.1.0
gh release download v0.1.0 --dir release-verification
```

From the downloaded directory, verify the checksum list with the platform's SHA256 tool (`sha256sum -c SHA256SUMS` on Linux, or `shasum -a 256 -c SHA256SUMS` on macOS). Confirm the release is a full release (or a prerelease for a suffixed tag), all required platform assets exist, the source commit matches, and the banner and parity links resolve. Installer launches, mobile document access/save-back, same-device behavior, original-runtime differential tests, and signing/notarization qualification remain separate checks.

A failed build prevents the final job from running. A failed publication leaves a draft; no partial release becomes public. Missing GitHub asset digests fail explicitly rather than assuming that matching names or sizes prove integrity. If a public release needs correction, publish a new version instead of replacing existing downloads.
