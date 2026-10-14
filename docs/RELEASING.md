# Publishing OpenTag releases

`.github/workflows/release.yml` builds the same validated commit for five desktop targets, an Android arm64 APK, and an iOS arm64 simulator app. Only the final job can publish. It waits for every required build, verifies platform manifests and SHA256 hashes, creates or reuses an unpublished draft, uploads the complete asset set, verifies GitHub's asset digests, and then publishes.

Current `0.1.0` packages are development prereleases. Android and iOS packages are **UI previews**: document editing and save-back integration is still in progress in this release source. Desktop feature parity and remaining runtime qualification are recorded in [PARITY.md](PARITY.md).

## Triggering a release

Keep the version identical in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. The tag must be exactly `v` plus that version, currently `v0.1.0`. Any existing tag must point to the checked-out workflow commit. Every build checks out the resolved immutable commit, rather than a moving branch.

Land the workflow and release source on the repository's default branch before publication. GitHub's current [release API permissions](https://docs.github.com/en/rest/releases/releases#create-a-release) restrict creating releases when the target commit changes workflow files relative to the default branch; the normal workflow token cannot acquire the extra Workflows permission. Publishing a landed default-branch snapshot avoids needing an additional personal access token.

Either push the release tag or run the workflow manually. The manual tag input can be empty: preparation derives `v<package version>` from the selected source. The prerelease input defaults to true. Version `0.x` and versions with a semantic-version prerelease suffix are always published as prereleases, even if the input is false.

```sh
gh workflow run release.yml --ref main -f tag=v0.1.0 -F prerelease=true
```

Alternatively, create and push a tag on the intended landed commit:

```sh
git tag v0.1.0 <release-commit>
git push origin v0.1.0
```

Do not move a published tag or overwrite a public release. Preparation and publication both reject an already published release. A failed draft upload remains unpublished; rerun failed jobs using the same source commit. The final job may replace stale assets only on a draft for that exact commit. Release runs are serialized so a manual run and a tag-triggered run cannot race one another.

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

Android builds use JDK 17, platform/build tools 37, and NDK `28.2.13676358`. Before upload, the workflow verifies the APK signature, rejects the Android debug certificate, and checks the application version and sole `arm64-v8a` ABI. Missing signing secrets fail the required Android job and prevent publication; the pipeline never substitutes an unsigned or debug APK.

iOS uses `--target aarch64-sim --debug --no-sign --ci`. Packaging checks the version and `IOSSIMULATOR` binary platform. There are no Apple distribution certificates or provisioning profiles in this pipeline. Device signing, TestFlight and App Store distribution require a separate qualified signing/release path.

## Verification and failure handling

Before committing workflow changes:

```sh
node --test tests/release.test.mjs
actionlint .github/workflows/release.yml
```

The script tests cover version/tag/commit mismatch, forced development prereleases, exact platform coverage, missing and mixed manifests, tampered local files, duplicate asset formats, already published releases, upload failures, missing/wrong server SHA256 digests, draft pagination, publication order, banner/signing notes, and app archive fallback. They do not prove that the remote workflow ran or that built packages install correctly.

Inspect the actual workflow run and release after publication:

```sh
gh run list --workflow release.yml
gh release view v0.1.0
gh release download v0.1.0 --dir release-verification
```

From the downloaded directory, verify the checksum list with the platform's SHA256 tool (`sha256sum -c SHA256SUMS` on Linux, or `shasum -a 256 -c SHA256SUMS` on macOS). Confirm the release is a prerelease, all required platform assets exist, the source commit matches, and the banner and parity links resolve. Installer launches, mobile document access/save-back, same-device behavior, original-runtime differential tests, and signing/notarization qualification remain separate checks.

A failed build prevents the final job from running. A failed publication leaves a draft; no partial release becomes public. Missing GitHub asset digests fail explicitly rather than assuming that matching names or sizes prove integrity. If a public release needs correction, publish a new version instead of replacing existing downloads.
