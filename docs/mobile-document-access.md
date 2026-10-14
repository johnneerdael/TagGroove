# Android document access

`crates/document-access` is an independently authored Tauri plugin for Android's Storage Access Framework. It provides read/write document selection and actual grant/capability metadata; it does not request broad storage permissions or turn `content://` identifiers into filesystem paths.

## Rust integration

Register `tauri_plugin_document_access::init()` and import `DocumentsExt`. Manager types, including `AppHandle`, expose:

```rust,ignore
app.documents().pick_documents(multiple) // Result<Vec<DocumentInfo>, String>
app.documents().info(&uri)               // Result<DocumentInfo, String>
app.documents().rename(&uri, &name)      // Result<DocumentInfo, String>
```

These synchronous Rust bridge calls must run on `tauri::async_runtime::spawn_blocking`, rather than the UI/event loop. Picker completion is an asynchronous Android activity callback; provider queries and renames execute on a bounded native worker pool. Desktop and iOS return unsupported errors. iOS keeps the existing security-scoped file-dialog/filesystem route.

`DocumentInfo` serializes `uri`, provider `name`, nullable `mime_type`, `size`, `modified_ms`, and Boolean `writable`/`persisted`. URI identity stays opaque. `writable` requires an actual write grant and `FLAG_SUPPORTS_WRITE`; delete permission alone is insufficient. `persisted` means retained read access and, when writable, retained write access. Read-only or temporary-only documents are reported accurately instead of being promoted to writable/persistent sources.

## Android behavior

The picker uses `ACTION_OPEN_DOCUMENT` with `CATEGORY_OPENABLE`, optional multi-select and requested read/write/persistable grants. Only result-granted access bits are passed to `takePersistableUriPermission`. Cancellation returns an empty vector. Unknown size/time/MIME remain null. Renaming requires a write grant and provider rename capability, returns the provider's new URI, and attempts to retain previously retained access where Android offers it. If rename succeeds but the subsequent metadata query fails, the error identifies the new URI and the completed rename; callers must present that partial state rather than claim no change.

The plugin deliberately leaves byte reads, staged metadata work, write-back validation, source-revision checks and UI persistence to the app's working-file adapter. Directory-tree selection is not implemented in this slice. Provider grants can be revoked or documents moved externally; callers must re-query before saving and handle re-selection.

## Evidence and remaining checks

Rust validation/serialization tests cover opaque URI handling, invalid rename paths, null metadata and read-only state. Native JVM tests cover provider write support without a grant, delete-only capability, temporary write with retained read, retained read/write, and excluded directory/virtual-document capabilities.

The registered plugin compiled in the Android arm64 debug APK on 2026-10-14. Three Rust tests, five native JVM tests and strict Rust Clippy passed. The initial host compile exposed an owning `PhantomData<R>` marker in the unsupported-platform fallback; the corrected nonowning `PhantomData<fn() -> R>` passes the generic Tauri managed-state bounds. No unsafe thread-safety implementation was added.

Commands run from the app root:

```sh
cargo test -p tauri-plugin-document-access --offline
cargo clippy -p tauri-plugin-document-access --all-targets --offline -- -D warnings
```

Run native grant-policy tests from `src-tauri/gen/android` after Tauri generates the plugin project inclusion:

```sh
./gradlew :tauri-plugin-document-access:testDebugUnitTest --console=plain
```

The app build used the existing isolated Android cache, Rustup stable compiler and installed NDK:

```sh
env PATH=/Users/jneerdael/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH RUSTC=/Users/jneerdael/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc CARGO_TARGET_DIR=$PWD/target/mobile-verification/android NDK_HOME=/Users/jneerdael/Library/Android/sdk/ndk/28.2.13676358 npm run tauri -- android build --debug --target aarch64 --apk --ci --config '{"build":{"beforeBuildCommand":null}}'
```

`beforeBuildCommand` was disabled because this check verifies the new native plugin against the existing frontend build. The app's document working-file/save-back adapter is separate integration work. The final build exited 0; 44 native source/manifest hashes remained unchanged between capture and completion. The APK contains the plugin class and its embedded `libopentag_lib.so` matches the isolated Rust target output. `aapt dump permissions` shows only `INTERNET` and the app's receiver permission. Full source hashes, test counts, timestamps and artifact hashes are recorded in [the build receipt](../crates/document-access/verification/android-build.json).

| Artifact | SHA-256 |
| --- | --- |
| `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk` | `f0b675ac2971b5bcc328e4ed53d97449a80fdc5e33ec661705bbe583e6cf1047` |
| APK's `lib/arm64-v8a/libopentag_lib.so` | `3fa8c6764a396207eed04efbc2c9af5ece6b760c3ed83c2db2e7d600fde235b3` |

Compilation and these pure capability tests do not prove picker grants, restart persistence, cloud-provider behavior or save-back. Device tests must cover read/write and read-only providers, cancellation, multi-select, revoked grants, restart, renamed identities and actual metadata-write/audio-payload preservation. No device runtime verification was performed for this plugin.

Primary sources: [Android SAF guide](https://developer.android.com/training/data-storage/shared/documents-files), [DocumentsContract](https://developer.android.com/reference/android/provider/DocumentsContract), and [Tauri mobile plugin development](https://v2.tauri.app/develop/plugins/develop-mobile/). The installed Tauri 2.12.3 and dialog 2.8.2 sources establish the bridge/callback API and the existing `ACTION_GET_CONTENT` behavior being replaced for editable audio sources.
