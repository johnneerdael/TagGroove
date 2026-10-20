# Android/JVM runtime notice material

Working-branch L76 slice. This inventory supplies source-bound Android notice
material; it does not claim exact shrunk DEX membership, legal compliance, a
new published package or completion of the native/installer-helper audit.

## Resolved set and retained material

The actual offline AGP runtime resolution for `arm64ReleaseRuntimeClasspath`
contains 83 graph nodes, including local projects, BOMs and metadata redirects.
Its 68 artifact entries deduplicate to 65 exact Maven coordinates/archives;
three repeated AAR entries have identical bytes. There are no unresolved edges.
These are conservative runtime candidates rather than a count of classes
retained by R8. Test-only dependencies and build-plugin implementation graphs
are outside this configuration.

The committed [inventory](../../src/licenses/android-inventory.json) retains
each artifact's SHA-256, its selected resolution graph, exact POM-chain hashes,
supplier license declarations and per-component material paths/hashes.
Twenty-five candidate archives supply 32 recognized notice/license members;
all are retained, including nested `classes.jar` members and Jackson core's
FastDoubleParser and bigint notices. There are eight distinct UTF-8 texts,
deduplicated by SHA-256 without losing per-component membership.

All inspected POM declarations resolve to Apache 2.0. Guava ListenableFuture
1.0 inherits that declaration from its exact `guava-parent:26.0-android` POM;
it is a real class artifact, not the similarly named empty conflict placeholder.
Where full terms are absent, the inventory explicitly references the exact
Apache terms supplied in pinned `androidx.annotation:annotation-jvm:1.9.1`,
member `META-INF/androidx/annotation/annotation/LICENSE.txt`, SHA-256
`809fa1ed21450f59827d1e9aec720bbc4b687434fa22283c6cb5dd82a47ab9c0`.
The source contains terms 1–9. It does not replace other supplied notices or
invent a common copyright attribution. Unknown declarations refuse collection.

The published v0.2.56 APK is separate baseline evidence: it contains 46 version
markers and five recognizable notice paths. Its Material version is 1.13.0,
selected by the dialog plugin despite the application's direct 1.12.0 request.
An arch-core marker contains a build-task description rather than a version;
that marker alone cannot identify its source. Missing markers or notice paths
do not establish that a library or embedded About payload is absent.

## Collection and checks

[`android-notices.py`](../../scripts/android-notices.py) captures the actual
Gradle graph through a checked-in, metadata-only
[init script](../../scripts/notices/android-runtime-report.init.gradle).
Capture uses the pinned wrapper version, current tracked app/buildSrc settings,
the exact generated dependency/settings structure, the current explicit plugin
set and module configuration matched to checksum-verified Cargo archives.
This refuses a stale or differently configured project before accepting its
snapshot. Signing credentials are not needed or forwarded.

Source fingerprints normalize checkout newlines and the application's routine
release stamps. Build/cache outputs do not become source inputs. Snapshot
target/configuration and input bindings are checked before regeneration.
Actual selected AAR/JAR bytes, their nested notice material, POM identity and
inherited declarations are independently replayed from the Maven cache.
Cache SHA1 identities and fresh SHA-256 values bind the files; this is not a
supplier-signature claim. Unsafe/duplicate ZIP paths and symlink members refuse.

The [rendered payload](../../THIRD_PARTY_ANDROID.md) appears unchanged in About,
the common notice file and escaped installer RTF. Headings translate into Dutch
and German; upstream text retains its supplied language. Output publication
uses the existing staged/ownership-checked recovery mechanism. A detached
payload or changed text fails the ordinary cache-free check.

The Android release job performs capture and material replay after its normal
APK build and before collection. This is an artifact/dependency consistency
check, not a repeated unit or native UI suite. Unexpected selection or archive
bytes stop that job. Metadata is retained separately from the allowlisted
public release assets. This workflow has not yet run for this unmerged change.

## Focused evidence and remaining work

Eight Python cases cover nested exact material, archive tampering/unsafe paths,
POM identity/inheritance/cycles, unknown declarations, stale or incomplete
graphs, source/module/generated-input refusal, detached outputs and real CLI
generation/check/replay. Headless frontend checks load the actual About notice
array and retain the shaded parser material. Independent read-only review
matched all 65 artifact hashes, 32 supplied members and POM chains against the
actual cache; it found no actionable source/material issues.

Final local checks pass: eight collector cases, thirteen affected headless
notice/catalog cases, two existing Rust/JavaScript notice cases, TypeScript,
catalog extraction, ledger consistency and strict guide build. Release-tooling
checks pass: 127 GitHub tooling cases, 28 release cases, and actionlint with only
the repository's documented `queue` syntax exception. Real capture and material
replay pass for all 65 candidates/eight texts. These are local checks, not a
completed GitHub Codex review, remote CI run or published artifact.

Actual postmerge APK delivery and runtime notice inspection remain required.
The native policy repair still needs rebuilt target evidence, decoder
integration and application closure/symbol checks. Installer helper material
and other native subcomponent audits remain L76 work. No application/native UI
test was run for this slice.

## Release 39493635854 source-recipe repair

Android job `118873127144` built its signed APK and successfully captured the
runtime metadata, then refused the bundled inventory with
`Stale Android notice source inputs`. Comparing all 29 inventory input
fingerprints against main `15ca9ab3e09bba9e886b8b0c72fcde3a67f3a37b` isolated
one difference: `scripts/audio-deps/sources.json`. That commit changed zlib's
preferred download URL and retained its original URLs as mirrors; version
`1.3.2`, archive SHA-256
`bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16`,
and all notice material remained unchanged. All 20 Android resolution graph
fingerprints still matched. The strict source check correctly detected an
inventory that had not been regenerated after the recipe edit.

Regenerated through `android-notices.py --generate` with the committed
resolution snapshot and the actual Maven cache, then ran `--check --metadata`
with `--verify-material`. This revalidated the selected graph, all 65 artifact
identities and bytes, POM chains, and eight notice texts. The only inventory
change is the source recipe fingerprint, from `1deed6f1...` to `46d04767...`;
rendered payloads, archive pins and resolution inputs are identical. No
collector, workflow or normalization rule changed. This reused the existing
source-bound graph and did not claim a fresh local Gradle capture.

The existing cache-free JavaScript check reproduced the failure before
regeneration and passes afterward. A new Python regression also failed with
the original inventory. It copies the current source inputs into an isolated
temporary tree, runs the real release stamper for `0.2.60`, and proves that
the normalized inputs stay identical and the inventory remains valid. It
then changes the source recipe URL, a locked dependency checksum, Android
module code and collector code independently; each change still refuses the
inventory. Nine focused Python cases and both Android notice JavaScript
cases pass. No application build or native UI test was run for this repair;
fresh hosted capture, complete APK/native evidence and runtime inspection
remain release requirements.
