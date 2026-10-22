# Contributing to TagGroove

Open changes through a feature branch and pull request. The PR template separates public release notes, validation and documentation impact. Keep all three aligned with the final implementation.

## Review and merge

Request `@codex review` on the PR's final commit. Address findings, resolve their review threads and request a new code review after the last change. Merge only after that exact head has a clean GitHub Codex code review and the required Linux unit tests, lint, tooling and documentation checks have passed. A local review or a security-only review does not satisfy the code-review requirement.

The trusted `Review authorization controller` workflow publishes these commit statuses:

| Required status | Meaning |
| --- | --- |
| `Codex review` | The authenticated Codex connector completed a code review of the current full head SHA, supplied clean-review approval, and no outstanding findings, review requests or requested changes remain. |
| `Release notes` | The current PR body contains a substantive public release-note section. |
| `Reviewed PR builds` | Linux unit tests, lint, tooling and documentation passed for the reviewed head, current main and immutable test merge. The historical status name is retained for branch protection. |

The gate reads GitHub's full review/comment/thread history. A short displayed SHA must resolve to the exact full commit; a thumbs-up without revision metadata is insufficient. Resolving an earlier finding does not substitute for a newer clean review. New pushes invalidate the review, and a newly edited review request requires a later completion. API failures keep the gate pending.

Approval normally requires the connector's issue `+1` reaction at or after the latest qualifying code-review completion. GitHub [returns the existing reaction when the same account repeats it](https://docs.github.com/en/rest/reactions/reactions#create-reaction-for-an-issue), so its creation time can remain older than a repeat clean review. A retained connector `+1` also qualifies when an authenticated connector Bot comment contains `<!-- codex-review-receipt:v1 -->`, recognized clean `Codex Review` text, a valid creation timestamp and a reviewed commit resolving to the exact current full head SHA. Security-review text cannot supply this receipt approval. The receipt must be at least as recent as every qualifying completion, and the usual request, findings, thread and pending-review blockers still apply. An older receipt cannot authorize a later ambiguous completion; unstructured summaries and generic completions still require a fresh reaction.

A separate controller checks out only trusted `main`, reads live PR state and posts statuses. It has no schedule and does not run on PR pushes or comments; it runs when a review is posted or dismissed (through an unprivileged signal), when a PR description is edited, when a validation finishes and on a main push. It dispatches application validation only after review approval and valid release notes. The dispatched workflow and reusable build repeat authorization against the exact head/main/test-merge triple before PR code runs. PR jobs receive no signing secrets, checkout credentials or shared caches. A separate metadata-only workflow watches recognized code-review commands (including trailing instructions), reviewer/team requests, draft conversion and connector comments and withdraws stale review/build approvals without dispatching application validation. It checks out only trusted `main` and has no Actions write permission; ordinary discussion skips the job. Opening or reopening a PR, changing its head, marking it ready, or changing its base resets all three commit statuses to pending to withdraw approval and validation inherited from another PR. Base changes run only this invalidation path; dispatch the controller for a fresh review, notes and build check. Reactions have no Actions event, so dispatch the controller for a PR when it has a clean Codex review and again right before you merge:

```sh
gh workflow run review-controller.yml --ref main -f pr_number=42
```

Commit statuses are shared by SHA. Withhold review, release-note and build success while another open PR anywhere in the repository has the exact same head, including drafts and PRs targeting another base. Check the paginated live open-PR list before reconciliation success, validation authorization/completion and release authorization; API failures deny approval. Close one PR or advance one head before rechecking. Lifecycle invalidation watches all base branches so opening a sibling PR also withdraws inherited statuses. These live checks are not atomic PR identity isolation; repeat the live merge-readiness check immediately before merging.

### Repository protection and containment

After merge, release authorization can recover the preceding successful build
proof when a trusted Actions status says exactly `PR changed while reading
review state`: a metadata reader may have overlapped the merge. Live clean
review, release notes, unique head, trusted successful run, exact merge parents
and tested tree still have to pass. Other pending reasons, failures and
untrusted statuses remain blocking; this exception never authorizes a PR merge.

Normal changes require PRs, a current base, resolved threads, `Codex review` and `Reviewed PR builds`; use merge commits. The owner-authorized direct-main workflow and failed-build repair exceptions are described below. The controller validates release notes before dispatch, during validation preflights and before publication. `Release notes` also exposes a separate diagnostic status. A green optional status does not enforce a merge policy.

Keep the legacy build/release, Docs, Docs screenshots and CI tooling workflows disabled. Do not restore automatic application jobs. Releases use only the reviewed controller and `reviewed-release.yml`, including successful trusted build proof and identical validated source checks. See [the containment contract](../.github/CI_LOCKDOWN.md). No Codex API key is needed: the installed GitHub connector performs reviews; the controller uses the workflow token for metadata and dispatch authorization. Failed validation needs an explicit `retry_build=true` dispatch after investigation; an API error or elapsed time never authorizes execution.

## Public notes and documentation

Write `## Release notes` for people using the app: what changes, the practical effect and relevant limits. For example, “Filename → Tag skips non-matching files while importing matching files in the selection” is more useful than “Fix converter.” For internal work, name the actual change: “Internal: Validate documentation links before publication.” Empty sections, `TODO`, `N/A`, vague maintenance text and a bare “No user-visible changes” fail validation.

Put test commands, measured results, coverage gaps and review evidence in `## Validation`. Update affected user-guide pages and explain the documentation impact in the PR. A strict MkDocs build catches broken links, anchors and navigation; reviewers still check whether instructions match implemented behavior, screenshots remain useful and platform limitations are accurate. Publication requires a successful Pages deployment and a working public site.

The release publisher collects each merged PR's public notes since the previous published release, including intervening docs-only merges. It excludes draft/unpublished tags and fails on unavailable metadata or missing newly required notes. Historical PRs at or before `.github/release-notes-baseline` retain their sanitized descriptions; later merges require the dedicated section. Tests, badges and agent transcripts belong outside that section. See [RELEASING.md](RELEASING.md) for artifact and publication safeguards.

## Lean validation and main-build repairs

After the exact-head Codex gate, one Linux worker runs frontend and Rust unit tests, formatting, Clippy, TypeScript checking, tooling tests and strict documentation validation. Tests compile their test binaries, but this stage does not build a desktop app or Android package, run native UI suites or generate screenshots. PR execution receives no signing secrets, checkout credentials or shared caches. The existing `Reviewed PR builds` status name remains for branch-protection compatibility.

After merge, the release workflow builds all required application targets. Native UI suites on Linux, macOS and Windows, guide screenshots and Android document integration run in the same post-merge workflow and must pass before application publication. Desktop matrices stop remaining matrix work on failure, and native suites stop at their first failing suite. Native UI uses an instrumented debug app with a separate test identity; that harness is distinct from the shipped release package. Post-merge jobs do not repeat frontend or Rust unit tests, formatting, Clippy or documentation test suites. They retain package metadata, signature, version, native provenance, corresponding-source and checksum checks; Pages renders the validated guide for deployment.

Owner-authorized workflow changes and repairs for failed builds on main go directly to main. Retain the failing run and diagnosis, run focused checks, and push with a lease on the inspected main head. Preserve unrelated history. A release-producing repair uses a single-parent commit directly after the failed release source and a commit-message trailer `CI-Repair-Run: <failed-run-id>`. The trusted gate verifies the referenced completed failed main release run with successful authorization and preparation, both revisions' main ancestry and the absence of an associated merged PR before authorizing the repair. This is a separate owner-authorized repair path, not a fabricated PR review. If main has advanced beyond the failed source, validate that newer main and diagnose its build rather than rewriting intervening commits. If the failed run was dispatched from a later controller commit, a repair may preserve that exact controller as its sole parent only when a complete, bounded GitHub comparison proves the intervening changes are confined to `.github/` and `docs/`. Application changes, renames across that boundary and incomplete or truncated comparisons refuse this exception. Workflow-only/bootstrap pushes without this evidence do not authorize an application release. All native integration and publication safeguards still apply.

### Retain completed packages after a native test repair

When only desktop native UI validation fails, a trusted-main test-harness repair
can resume the original release without rebuilding its completed packages. Keep
`source_sha` and `head_sha` on the original reviewed application and set
`resume_run_id` to the failed release run. The recovery gate requires every
original package, corresponding-source archive and Android integration check to
have passed, with unexpired source-bound artifacts. It refuses application
changes, other failures and ambiguous evidence. Main tooling/test changes do not
authorize a new application source.

Recovery reruns the failed/cancelled desktop UI validation with the pinned repaired
assertion against the original application, then independently verifies the
retained package bytes before publication. Notes identify both runs and the
harness commit. The guide and successful Android validation are retained.

## Validation

For application work, run `npm test` and `npm run build`, plus the relevant native UI/platform suites. For CI and release-tooling changes, also run:

```sh
python3 -m unittest discover -s .github/scripts/tests -v
node --test tests/release.test.mjs
actionlint -ignore 'unexpected key "queue" for "concurrency" section' .github/workflows/*.yml
git diff --check
```

Python 3.10 or newer is required for release-note tooling; CI pins Python 3.12. The actionlint exception covers the documented concurrency queue key, unsupported by actionlint 1.7.12. Do not suppress other workflow errors.

## Local debug build with application keys

Release builds compile TagGroove's own keys for Discogs, AcoustID, Last.fm and fanart.tv in from `OPENTAG_*` variables (`OPENTAG_DISCOGS_CONSUMER_KEY`, `OPENTAG_DISCOGS_CONSUMER_SECRET`, `OPENTAG_ACOUSTID_API_KEY`, `OPENTAG_LASTFM_API_KEY`, `OPENTAG_LASTFM_SHARED_SECRET`, `OPENTAG_FANARTTV_API_KEY`). The release workflow takes them from the repository secrets of the same names without the prefix (`DISCOGS_CONSUMER_KEY`, `ACOUSTID_API_KEY`, `LASTFM_SHARED_SECRET` and so on) in the steps that build shipped binaries; the Android and iOS steps use `TAURI_OPENTAG_*` aliases because Tauri forwards only `TAURI`-prefixed variables to those builds (see [RELEASING.md](RELEASING.md#application-keys-in-release-builds)). A build without a key still compiles: Discogs then asks for a personal token, fanart.tv for the user's own key, and AcoustID and Last.fm (no one enters keys for those) are unavailable.

For a local debug build, keep the credential files `.acoustid`, `.fanart`, `.discogs` and `.lastfm` in the repository root (git ignores them; `scripts/local-app-keys.py` documents their layout) and run the build through the helper. It adds the variables to that one command and never prints a value:

```sh
python3 scripts/local-app-keys.py --check
python3 scripts/local-app-keys.py -- npx tauri build --debug --bundles app
```

## Embedded audio build tools

AcoustID fingerprinting uses source-built static Chromaprint, FFmpeg, zlib and
bzip2 libraries. Normal application use requires no media command-line tools.
The first Cargo build downloads only the four checksum-pinned source archives
in `scripts/audio-deps/sources.json`; it requires Python 3.10+, CMake, GNU Make
and a C/C++ toolchain. x86_64 targets also need NASM at build time; ARM
targets use the selected compiler for assembly. macOS/iOS use the selected Xcode SDK and Apple archive
tools. Windows uses Visual Studio's MSVC toolchain plus MSYS2 Bash/Make
(`OPENTAG_MSYS2`, default `C:/msys64`). Android uses the existing NDK through
`NDK_HOME` or `ANDROID_NDK_HOME`. Source builds disable external-library
autodetection and do not link an installed Homebrew or GPL FFmpeg.

Fresh source downloads share a maximum of three attempts; the pin is unchanged
on every attempt. A declared mirror can recover from wrong downloaded bytes
or an HTTP failure, and requests bypass HTTP caches. Without a mirror, HTTP
errors still fail immediately. Persistent checksum failures name the archive
and show expected/actual hashes. Tampered existing caches and explicit offline
failures never fall back to the network. See the
[download repair evidence](verification/native-source-download-repair-2026-10-20.md).

Choose an absolute owned build/cache directory without spaces with
`OPENTAG_AUDIO_BUILD_ROOT`; its completed prefixes verify installed libraries,
headers, configuration and notices against the retained manifest. Set
`OPENTAG_AUDIO_SOURCE_DIR` to a flat directory containing the original pinned
archives for offline builds. An absent or changed offline archive fails
without falling back to a network download. The default cache is inside
Cargo's actual build-script output directory, including when its target directory
is relative to a caller outside the workspace. Native build phases require more than 2.3 GiB free.

Run the actual producer tests with:

```sh
cargo test -p opentag-audio-fingerprint --locked
python3 -m unittest discover -s scripts/audio-deps -p test_build.py -v
```

The frozen synthetic corpus includes the independent reference fingerprints
and hashed audio carriers. Tests need no installed `fpcalc` or `ffmpeg`.
Regeneration tools under `tests/fixtures` document developer-only generation;
changing expected fingerprints requires new independent reference evidence.
See [the compatibility ledger](verification/embedded-fingerprints.md) and
[release/source packaging](RELEASING.md).
