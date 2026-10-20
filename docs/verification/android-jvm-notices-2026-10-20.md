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
