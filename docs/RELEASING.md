# Publishing OpenTag releases

**Review-first workflow:** follow [CI_LOCKDOWN.md](../.github/CI_LOCKDOWN.md). Legacy automatic workflows remain disabled. Application builds and publication require a completed clean Codex code review and immutable validation proof; direct bootstrap commits do not release.

`.github/workflows/reviewed-release.yml` builds the same validated commit for five desktop targets, an Android arm64 APK, and an iOS arm64 simulator app. Only the final job can publish. It waits for every required build and the corresponding-source archive, verifies platform and native-build manifests and SHA256 hashes, creates or reuses an unpublished draft, uploads the complete asset set, verifies GitHub's asset digests, and then publishes.

An authorized merge to `main` that changes shipping files publishes a full GitHub release (not a prerelease); only a newer full release becomes latest. Pre-1.0 releases say "Release (pre-1.0)" in their notes. Android and iOS packages are **UI previews**: document editing and save-back integration is still in progress. Desktop feature parity and remaining runtime qualification are recorded in [PARITY.md](PARITY.md).

## How versions are derived

`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.lock` (the `opentag` entry), `package-lock.json` and the iOS project files carry the **base version**, currently `0.2.0`. It must be a plain `<major>.<minor>.0`.

For a push to `main`, the release version is `<major>.<minor>.<n>`, where `n` is the number of first-parent commits on `main` since the commit that set the base version that change shipping files (see [Docs-only merges](#docs-only-merges)). The commit that lands the base version is `v0.2.0`; the next code merge is `v0.2.1`, then `v0.2.2`, and so on. Each merge adds one first-parent commit, however many commits its branch had, so versions follow the order of merges, increase strictly and never depend on the runner, the clock or earlier releases. The same commit always gets the same version, so reruns are deterministic.

`scripts/release.mjs prepare` computes the version and tag once; every build job then runs `scripts/release.mjs stamp`, which writes that version into the files listed above in its checkout. Stamping includes `Cargo.lock` so `--locked` builds still pass. Stamped versions are never committed. The app's update check, the installer metadata and the Android `versionName` report the full release version, including any prerelease suffix. Apple requires `CFBundleShortVersionString` and `CFBundleVersion` to be numeric, so the iOS project gets the numeric `<major>.<minor>.<patch>` (`0.3.0` for `0.3.0-beta.1`), as Tauri also does when it builds the app. The Android `versionCode` ignores the suffix too, so `0.3.0-beta.1` and `0.3.0` share code `3000`.

Tauri computes the Android `versionCode` as `major * 1000000 + minor * 1000 + patch`, so the patch must stay below 1000. Preparation fails once `main` reaches 1000 merges for one base version; bump the minor version before then.

Release runs share one serialized concurrency group with [`queue: max`](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency), so up to 100 runs wait their turn instead of the newest replacing the pending one. Authorized merged-PR release dispatches therefore retain their place in the release queue. GitHub does not guarantee that order, so a later merge can occasionally publish before an earlier one; the earlier one still publishes its own version but does not take "latest" from the newer release. If more than 100 runs are waiting, GitHub cancels the extras; rerun a cancelled merge's workflow to publish its version.

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

### Explicit releases, prereleases and dry runs

The reviewed workflow accepts only the merged PR number, reviewed full head SHA and merged source SHA. The trusted controller supplies those inputs after validating the merged source. Tag pushes do not trigger publication, and the workflow has no arbitrary-tag, branch or dry-run input. Do not re-enable the legacy workflow to restore those paths.

The release script retains its explicit-tag and prerelease validation rules: an explicit tag must equal the base version or carry a prerelease suffix, and a suffix always denotes a prerelease. Those script capabilities are covered by local tests but are not exposed by the reviewed publication workflow. Use `node --test tests/release.test.mjs` to validate version planning and stamping without publishing.

## Publication rules

Land the workflow and release source on the repository's default branch before publication. GitHub's current [release API permissions](https://docs.github.com/en/rest/releases/releases#create-a-release) restrict creating releases when the target commit changes workflow files relative to the default branch; the normal workflow token cannot acquire the extra Workflows permission. Every automatic release targets a commit on `main`, so this never applies to it.

Every build checks out the resolved immutable commit, rather than a moving branch. The publish step creates the tag `v<version>` on that exact commit when it publishes the release. Any existing tag must already point to that commit.

Do not move a published tag or overwrite a public release. Releases are idempotent:

- If a published release for that tag and commit already exists, preparation reports it and the run skips every build, so rerunning a finished run publishes nothing new.
- A published release or a tag for that version on a different commit fails the run.
- A failed draft upload remains unpublished. Rerunning the failed jobs reuses the plan from the original run and updates the same draft. The final job may replace stale assets only on a draft for that exact commit.
- A rerun of an older run never takes "latest" from a newer published release.

Release runs are serialized so authorized dispatches and reruns cannot race one another on a draft, a tag or "latest".

## Release notes

Every PR must contain substantive `## Release notes` describing the final public behavior, or a concrete `Internal:` explanation for documentation, tests or CI. Keep test results in `## Validation`. The trusted review controller validates the live PR body and publishes a separate `Release notes` status and blocks reviewed validation when notes are invalid. See [CONTRIBUTING.md](CONTRIBUTING.md) for review and repository setup.

The publisher collects the public note sections from merged PRs on the first-parent range since the previous published release. It includes intervening docs-only merges, deduplicates PRs and links their source. Draft or unpublished tags do not truncate the range. Later PRs without the required section, invalid placeholders and GitHub API failures stop publication rather than falling back to commit titles. PRs at or before the full SHA in `.github/release-notes-baseline` may use a sanitized historical description; direct commits retain their actual subject and explanation. The collector treats PR text as data and never executes it.

The download table, signing status (Windows and Linux packages are unsigned, macOS builds are ad-hoc signed and not notarized) and mobile preview limitations follow the collected notes. CI validates structure and placeholders; reviewers remain responsible for factual prose and scope. The Linux publish job uses Python 3.12 and read access to PR metadata alongside its existing release permission. Existing versioning, artifact, signature and checksum rules remain enforced.

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

Native build tools are installed explicitly before compilation: Python 3.12, CMake, Make and the target C/C++ toolchain, plus NASM for x86 assembly. Windows exposes the MSVC developer environment, binds `OPENTAG_MSYS2` to the setup action's actual installation after verifying Bash and Make, verifies the NASM executable, and adds its installation directory to `PATH`; Apple and Linux jobs install the matching tools through their existing package managers. Android and iOS retain their existing NDK/SDK and simulator targets. These are build prerequisites and do not add runtime programs to the application.

Trusted PR validation runs the workflow from `main`. Land build-tool prerequisites before a feature that needs them: a PR cannot bootstrap its first validation using changes to its own workflow. A runner inventory that does not list a tool is not proof of a live command failure; explicit installation provides the prerequisite without relying on a transitive package.

Linux packages are built on Ubuntu 24.04; compatibility with older distributions is not established by compilation. The [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) documents native arm64 Linux and separate macOS Intel/arm64 runners. Platform jobs use separate artifact names and preserve their OS/architecture in public download filenames.

Each builder uploads its packages, `manifest-<platform>.json`, and `native-<platform>.json` as workflow artifacts. The native sidecar binds the installed static libraries, headers, configuration, source recipe and patch evidence to the release commit, version and Rust target. A fresh per-run native build directory prevents a restored Rust cache from silently substituting a missing or older native prefix. The source job receives only these seven sidecars, rather than downloading the application packages again.

The final publisher requires all seven platform manifests, their exact native sidecars, the original 13 package filenames/formats, and exactly one `OpenTag-v<version>-sources.tar.xz`. It verifies commit/version/target identities, rejects unexpected files or duplicate formats, and streams every public asset through SHA256. It publishes 16 assets: 13 application packages, the corresponding-source archive, `release-manifest.json`, and `SHA256SUMS`. The checksum list includes the packages, source archive and release manifest; it does not recursively include itself. The public manifest records source identity, platform coverage, native evidence hashes, byte sizes and hashes.

The release body embeds the supplied [OpenTag banner](../assets/brand/OpenTag-Brand-Kit/03-banners/opentag-github-light-1280x640.png) from the released tag and links the matching parity ledger. Signing and mobile limitations appear alongside the download table.

## Native audio builds and corresponding source

The native fingerprint backend is built from the four hash-pinned upstream archives in `scripts/audio-deps/sources.json`. Builds require Python 3.11 or newer (CI uses 3.12), CMake, GNU Make and the target C/C++ toolchain. x86-64 builds also require NASM for the enabled assembly implementations; ARM builds use the compiler assembler. Linux installs `build-essential`, `cmake` and `make`; Apple builds use Xcode SDKs, `xcrun`, CMake and Make. Windows uses MSVC/NMake plus MSYS2 Bash/GNU Make, with MSVC tools first in `PATH`. Android uses the configured NDK and API 24 toolchain; the iOS preview uses the arm64 simulator SDK. These are build dependencies; fingerprinting uses the linked native libraries rather than launching an installed media program.

Every release includes `OpenTag-v<version>-sources.tar.xz` with:

- The tracked application source stamped to the release version, `Cargo.lock`, the built frontend, and vendored Cargo dependencies with a separate offline Cargo configuration. Existing project settings, including target linkers, remain unchanged.
- The original, hash-pinned Chromaprint, FFmpeg, zlib and bzip2 source archives and their original license files, plus the tracked recipes and patches.
- Exact native build manifests and non-library prefix evidence for all seven targets: installed headers, configuration, notices, patch output and FFmpeg upstream/generated source-closure metadata. The original archives supply the complete upstream source, including files outside that compiled closure.
- The original Git commit object, reconstructed tree/blob identities, original pre-stamp files and an actual version-stamp diff. The extracted tree needs no `.git` directory to build.

The source job vendors Cargo dependencies, builds the stamped frontend, and verifies the complete archive before upload. Publication independently checks it again against the native sidecars from the package jobs. A missing archive, wrong commit/version, changed source blob, mismatched upstream archive/license/patch, empty or incorrect compiler source-closure hashes, incomplete target set or mismatched native recipe blocks publication. Verification reads archive data using the checked-out verifier; it never executes code from downloaded artifacts.

To verify a downloaded source asset, use the verifier from a trusted checkout of the release commit:

```sh
python3 scripts/audio-source-bundle.py --source-sha FULL_RELEASE_COMMIT --version RELEASE_VERSION --verify OpenTag-vRELEASE_VERSION-sources.tar.xz
```

After extracting the archive, enter its `app` directory. To build and run the Rust fingerprint example offline on a supported host target:

```sh
export OPENTAG_AUDIO_SOURCE_DIR="$(cd ../native/archives && pwd)"
export OPENTAG_AUDIO_BUILD_ROOT="$PWD/../rebuilt-native"
export CARGO_TARGET_DIR="$PWD/../rebuilt-rust"
cargo --config .cargo/opentag-offline.toml run --offline --locked -p opentag-audio-fingerprint --example fingerprint -- /absolute/path/to/audio.flac
```

`OPENTAG_AUDIO_SOURCE_DIR` requires the exact original archive filenames and hashes. If it is set, a missing archive fails instead of falling back to the network. The explicit `--config` overlay selects the bundled `vendor` directory while Cargo also loads the unchanged project configuration. The output directories above are fresh, so the command rebuilds and relinks the native libraries and Rust example rather than using a packaged executable. PowerShell users set the same environment variables with `$env:NAME = 'absolute-path'`. The host still needs its platform compiler, SDK, Python, CMake, Make and (on x86-64) NASM; these system tools are not part of the source archive. Rebuilding the frontend from source requires Node/npm and its dependencies; the included `dist` avoids that download for the Rust example.

The same extracted source builds the complete desktop application without
rebuilding the included frontend:

```sh
cargo --config .cargo/opentag-offline.toml build --offline --locked -p opentag --bin opentag --features tauri/custom-protocol
```

To change an embedded library, extract its archive, edit the source, and repack
it with the same top-level directory. Keep the original archive, choose a new
filename for the modified one, and explicitly update that component's archive
filename, location and SHA256 in `scripts/audio-deps/sources.json`. Put the new
archive beside the other local archives. The changed source pin selects a fresh
native recipe; an unexpected archive change still fails verification. Record
both the source patch and source-pin change when sharing the modified build.

A macOS arm64 host-only control exercised this full path: a local Chromaprint
version marker was rebuilt from a separately pinned modified archive, the
complete application linked offline from vendored Cargo sources, and the
running app returned the marker through its real backend command with an empty
runtime `PATH`. This is one qualified host control, rather than evidence for
all seven release targets. The complete release source asset still requires
all seven actual native manifests.

For maintainers, collect each finished native prefix and assemble the source asset with:

```sh
python3 scripts/audio-source-bundle.py --collect-native RUST_TARGET --native-evidence BUILD_ROOT_OR_PREFIX --source-sha FULL_RELEASE_COMMIT --version RELEASE_VERSION --output native-evidence/native-PLATFORM.json
python3 scripts/audio-source-bundle.py --source-sha FULL_RELEASE_COMMIT --version RELEASE_VERSION --native-evidence native-evidence --output OpenTag-vRELEASE_VERSION-sources.tar.xz
```

The builder accepts `--cargo-vendor` and `--dist` for already prepared dependency/frontend directories; otherwise it runs `cargo vendor --locked --versioned-dirs` and uses the checkout's `dist`. A source archive must contain evidence from every required target. Local fixture tests use tiny synthetic archives and manifests; they prove validation behavior, not seven-platform compilation or codec parity. Desktop workspace tests execute the native corpus on their host runners. Android and iOS build/package evidence does not establish runtime fingerprint equivalence on devices or simulators; those require separate execution and comparison.

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
python3 -m unittest discover -s scripts/tests -p test_audio_source_bundle.py -v
python3 scripts/audio-deps/test_build.py
python3 -m unittest discover -s .github/scripts/tests -v
actionlint -ignore 'unexpected key "queue" for "concurrency" section' .github/workflows/*.yml
```

actionlint 1.7.12 does not yet recognise the documented `concurrency.queue` key ([rhysd/actionlint#654](https://github.com/rhysd/actionlint/issues/654)); the flag ignores only that error.

The script tests cover automatic version derivation from a real git history (merge commits, multi-commit branches, base bumps), the Android versionCode limit, explicit-tag rules, docs-only merges keeping their number, stamping every manifest and `Cargo.lock`, numeric Apple bundle versions, rerun no-ops, "latest" selection, substantive PR-note collection, version/tag/commit mismatch, forced development prereleases, exact platform coverage, missing and mixed manifests, tampered local files, duplicate asset formats, already published releases, upload failures, missing/wrong server SHA256 digests, draft pagination, publication order, banner/signing notes, app archive fallback, required corresponding source, native target/source binding, original/archive/patch integrity, stamped Git provenance, and source tampering. They do not prove that the remote workflow ran or that built packages install correctly.

Inspect the actual workflow run and release after publication:

```sh
gh run list --workflow reviewed-release.yml
gh release view v0.1.0
gh release download v0.1.0 --dir release-verification
```

From the downloaded directory, verify the checksum list with the platform's SHA256 tool (`sha256sum -c SHA256SUMS` on Linux, or `shasum -a 256 -c SHA256SUMS` on macOS). Confirm the release is a full release (or a prerelease for a suffixed tag), all required platform assets exist, the source commit matches, and the banner and parity links resolve. Installer launches, mobile document access/save-back, same-device behavior, original-runtime differential tests, and signing/notarization qualification remain separate checks.

A failed build prevents the final job from running. A failed publication leaves a draft; no partial release becomes public. Missing GitHub asset digests fail explicitly rather than assuming that matching names or sizes prove integrity. If a public release needs correction, publish a new version instead of replacing existing downloads.
