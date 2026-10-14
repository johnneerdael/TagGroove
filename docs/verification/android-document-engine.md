# Durable document engine checkpoint

Source commits `2ffbcc9` and `950d594` add the provider working-copy engine.
This checkpoint precedes command and UI integration; it does not qualify an
Android editing workflow or a mobile release.

The engine persists a pending manifest before local mutation, retains immutable
baseline versions and edited bytes, and requires verified provider publication
before committing ready state. Failed publication, a pending restart or changed
private working bytes expose recovery-required state. Startup recovery does not
need provider access. Edited-copy export includes pending fields; original-copy
export uses the verified baseline and leaves the original recovery record intact.

Portable tests use real generated WAV files and an injected provider boundary.
The implementation report records 63 native tests, 167 core tests, six bridge
tests and 16 JVM tests passing, with strict Clippy. Cases include a same-size,
same-time outside change, partial writes, read-back failure, invalid manifests,
restart recovery, original/edited exports, cancellation and audio preservation.
These are local engine contracts; they do not substitute for real provider UAT.

On the dedicated API 34 emulator, the native instrumentation APK with SHA-256
`49b35b7af68d0f51455e78ec900e26248996c22e2a713d7ec3d91be9ad3686aa`
passed four tests, including the actual storage-root bridge command and repeated
private stream descriptor/path checks. The engine uses that command's canonical
`Context.filesDir`; Tauri's Android data directory resolves to `Context.dataDir`
and is not an allowed transport root by itself.

The full Android arm64 debug app built successfully from `950d594` before
integration. APK SHA-256:
`cf4b06f7d65354dba8e9ad795a462e79a12a00c53914ee57d1c04e011cf6a01e`.
Its embedded native library matched the isolated Rust output. The build reported
31 unused-code warnings because the new engine had no command consumers yet.
It was not installed or used to claim application save-back behavior.

The separate [disposable provider fixture](../../crates/document-access/verification/provider-fixture/README.md)
built and passed [ten actual ContentResolver checks](android-provider-fixture-api.json) on emulator-5554. These prove
the fixture's read-only rejection, metadata-only conflict with preserved size,
time and PCM, partial-write injection, create-document identity and exact-byte
copy. Its API checks consume only generated fixtures; only that test package was
cleared afterward for fresh UI tests. Android lint completed with nine warnings
about synchronous fixture preferences, API targets, backup metadata, icon and
test-screen strings; no lint errors. This fixture is a separate APK and is never
included in OpenTag.

Actual app picker/cancel, editing, verified save/reload, read-only copy export,
conflict/partial-write recovery, restart without source access, both appearances,
IME/focus and orientation remain the next verification gate. SAF still supplies
no general atomic replace or compare-and-swap; no automatic rollback is claimed.
