# iOS arm64 simulator debug artifact verification

The final frozen-source build **passed** and produced `src-tauri/gen/apple/build/arm64-sim/OpenTag.app` with signing disabled.

- Executable SHA-256: `af70b3b5ab9889b58437e661898e5d07cacf3d81cd689b203e8c089ca1352d69` (112,468,040 bytes).
- Bundle file-manifest SHA-256: `2ae0faba9a3933e142899a1ec0b7dac381ac50e536cd5e2831e5adc9d226c4c8`.
- Actual Mach-O arm64 executable: `IOSSIMULATOR`, minimum OS 15.0, SDK 26.2.
- Bundle identifier `org.opentag.editor`, version `0.1.0`; phone/iPad primary icon metadata points to `AppIcon`.
- Isolated target directory `target/mobile-verification/ios`; no source changes occurred during the final build.
- The final log contains no unassigned-icon warnings after 18 unused template PNGs were moved to a hashed backup outside the asset catalog. Supplied artwork remains unchanged; see [icon cleanup](mobile-ios-icon-cleanup.json).

The frontend was built once immediately before both native builds; their duplicate frontend hooks were disabled through a command-local config override. [Final artifact metadata](mobile-final-artifacts.json), [bundle file hashes](mobile-ios-final-bundle-files.json), [build log](mobile-ios-final-build.txt), and [source hashes](mobile-ios-final-source-state.json) retain the evidence.

This is an unsigned simulator-target debug app. No simulator was booted or app launched, and no physical-device, sandbox/document-picker/save-back, signing or App Store behavior was verified. Earlier successful and failed attempts remain below for diagnosis; the final snapshot supersedes their build-state limitations.

## Initial attempt

The initial 2026-10-14 attempt used scoped rustup compiler selection and an isolated Cargo target directory:

```sh
RUSTC=/Users/jneerdael/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
CARGO_TARGET_DIR="$PWD/target/mobile-verification/ios" \
npm run tauri -- ios build --debug --target aarch64-sim --no-sign --ci
```

The rustup stable toolchain directory was prepended to PATH for this command only. Xcode used the iPhoneSimulator26.2 SDK and `CODE_SIGNING_REQUIRED=NO`, `CODE_SIGNING_ALLOWED=NO`, and an empty signing identity. The generated project correctly entered the `aarch64-apple-ios-sim` Rust build.

The archive **failed** before an application artifact was completed. Native source/dependency edits were concurrent with this attempt: Cargo reported `E0432: unresolved import sha2` from `src-tauri/src/action_host.rs`; the Rust build phase failed and Xcode exited 65. This was the same moving-source race observed during Android verification, not an established iOS SDK or C++ compiler incompatibility. A stable-snapshot retry is required.

The initial source hashes and scoped settings are retained in [mobile-ios-simulator-source-state.json](mobile-ios-simulator-source-state.json). The full output is in [mobile-ios-simulator-debug-build.txt](mobile-ios-simulator-debug-build.txt), with the failure/log hash/source-change list in [mobile-ios-simulator-initial.json](mobile-ios-simulator-initial.json).

The Boost bridge object was emitted as Mach-O arm64. `xcrun vtool -show-build` reported `platform IOSSIMULATOR`, minimum OS 15.0, SDK 26.2. This verifies target compilation of that C++ object, not successful application linking. After both initial mobile build attempts, all 19 installed mobile branding resources still matched their supplied source bytes; see [mobile-brand-resource-integrity.json](mobile-brand-resource-integrity.json).

No successful simulator app/archive is claimed from this attempt. No simulator was booted, no app was installed or launched, no physical-device behavior was exercised, and no signing/distribution readiness was established.
