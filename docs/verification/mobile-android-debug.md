# Android aarch64 debug artifact verification

The final frozen-source build **passed** and produced `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk` (219,332,173 bytes).

- SHA-256: `a5ab16aab03572b1bd10edc7e754cb2fef9f11eef8701114e3145dce2ac001c1`
- Package: `org.opentag.editor`; version `0.1.0`; ABI `arm64-v8a`.
- Both `libopentag_lib.so` and `libc++_shared.so` are included. The embedded Rust library's hash matches the freshly linked isolated-target output.
- Cargo target directory: `target/mobile-verification/android`; scoped rustup `RUSTC` and NDK 28.2.13676358.
- No source changes occurred during the final build; all captured manifests, lockfiles, Rust/C++/frontend sources and Boost headers match their starting hashes.

The frontend was built once immediately before both final native builds. Their duplicate frontend hooks were disabled with `--config '{"build":{"beforeBuildCommand":null}}'` to avoid concurrent writes to `dist/`.

[Final artifact metadata](mobile-final-artifacts.json), [final build log](mobile-android-final-build.txt), and [final source hashes](mobile-android-final-source-state.json) retain the evidence. This is a debug build; no device installation, document-picker/save-back workflow, release-size/performance or distribution readiness is established.

The earlier stable retry also built successfully but preceded final export regressions. Its scope is preserved in [mobile-stable-artifacts-before-final-fix.json](mobile-stable-artifacts-before-final-fix.json) and [mobile-android-stable-source-state.json](mobile-android-stable-source-state.json).

## Initial attempt

The initial 2026-10-14 build used:

```sh
RUSTC=/Users/jneerdael/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
NDK_HOME=/Users/jneerdael/Library/Android/sdk/ndk/28.2.13676358 \
npm run tauri -- android build --debug --target aarch64 --apk --ci
```

The rustup stable toolchain directory was prepended to PATH for this command only. No global compiler configuration changed. The initial run used the existing default Cargo target directory; subsequent attempts will isolate mobile artifacts to avoid blocking workspace tests.

Rust initially linked `aarch64-linux-android/debug/libopentag_lib.so` successfully in 58.03 seconds, including the Boost C++ dependency. Tauri detected `libc++_shared.so` and symlinked the NDK's arm64 runtime into the APK JNI resource directory automatically; no C++ compiler or dependency patch was necessary for that link.

The complete APK build **failed** during Gradle's second Cargo invocation. The native source changed concurrently: `src-tauri/src/action_host.rs` imported `sha2`, while that Cargo invocation had read a manifest without the direct dependency. Cargo reported `E0432: unresolved import sha2`; Gradle failed `:app:rustBuildArm64Debug`, and the overall command exited 1. The parent subsequently added the dependency, but a successful retry against a stable snapshot is still required.

The full output is retained in [mobile-android-debug-build.txt](mobile-android-debug-build.txt); timestamps and the log hash are in [mobile-android-debug-initial.json](mobile-android-debug-initial.json). This first link is diagnostic evidence, not proof that the current source or APK is complete. This attempt produced no verified APK and performed no device installation or runtime tests.

All supplied Android launcher/adaptive/themed resource files remained byte-for-byte intact after the build attempt. The combined Android/iOS branding integrity check is retained in [mobile-brand-resource-integrity.json](mobile-brand-resource-integrity.json).
