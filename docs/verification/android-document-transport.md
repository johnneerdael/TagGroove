# Android document transport checkpoint

The Android bridge now streams private snapshots, checks source SHA-256 before
provider publication, requests truncating output, and verifies a fresh provider
read-back. It also exposes a create-document picker for later copy export.
This is the native transport layer, not completed app document editing.

The [compact receipt](android-document-transport.json) records source commits
and hashes, 16 passing JVM tests, five passing host Rust bridge tests, strict
plugin clippy and the Android debug library/test-APK builds. JVM tests use real
byte streams and private temporary files with injected source-change, short
write, close, read-back and metadata errors. They do not exercise a real
ContentResolver provider. The temporary Gradle fixture uses the actual plugin
sources and installed Tauri 2.12.3 Android source; no original project was built.

## Native descriptor regression

Review found that Android's public descriptor-based FileInputStream and
FileOutputStream constructors do not own the supplied descriptor. Closing the
wrappers therefore leaked descriptors opened by Os.open. Android documents
this ownership in its [input-stream source](https://android.googlesource.com/platform/prebuilts/fullsdk/sources/+/refs/heads/androidx-constraintlayout-release/android-35/java/io/FileInputStream.java)
and [output-stream source](https://android.googlesource.com/platform/prebuilts/fullsdk/sources/+/refs/heads/androidx-constraintlayout-release/android-35/java/io/FileOutputStream.java).

An instrumented API/resource test reproduced the failure on a newly created,
dedicated Android 14 AOSP arm64 emulator. The unfixed APK ran three tests and
failed both descriptor-closure cases immediately; the path/exclusive-open case
passed. The fixed APK passed all three. [Red output](android-private-streams-red.txt)
and [green output](android-private-streams-green.txt) retain the command output with trailing blank lines removed; the receipt records both raw and checked-in hashes.

The fix transfers the duplicate descriptor into an owning ParcelFileDescriptor
AutoClose stream and closes the original Os descriptor on every path. Native
tests inspect descriptor validity and /proc/self/fd targets after repeated
snapshot, successful publication, conflict and transfer-failure operations.
They also exercise exclusive snapshots and private symlink rejection. Only
generated app-private files on emulator-5554 were touched; no music library,
other emulator or physical TV was mutated.

## Boundaries

SAF supplies no general atomic compare-and-swap: a source change after the
preflight hash can still race publication, and interrupted provider writes may
leave partial contents. This transport never attempts an automatic rollback
over unknown provider contents. The working-copy engine must durably retain
baseline and edited bytes and its pending manifest before calling publish.

The device regression is file/API testing, not a picker or application UI test.
Picker grants/cancellation/lifecycle, actual provider streaming and save-back,
restart recovery, Android Rust application integration, full UI workflows,
cloud providers and iOS remain unverified here. The [mobile editing spec](../superpowers/specs/2026-10-14-mobile-document-editing.md)
and [implementation plan](../superpowers/plans/2026-10-14-mobile-document-editing.md)
retain those obligations. No full parity or mobile release qualification is
claimed.
