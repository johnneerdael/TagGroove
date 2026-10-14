# Rust core cross-target verification

On 2026-10-14, `tag-core` passed individual `cargo check` invocations for:

| Target | Result | Check completion time |
| --- | --- | --- |
| `aarch64-apple-ios-sim` | Passed, target-specific core metadata emitted | 1.56 s |
| `aarch64-linux-android` | Passed, target-specific core metadata emitted | 3.50 s |
| `x86_64-pc-windows-msvc` | Passed, target-specific core metadata emitted | 3.11 s |
| `x86_64-unknown-linux-gnu` | Passed, target-specific core metadata emitted | 4.36 s |

These checks used Rust 1.95.0 (`59807616e`, host `aarch64-apple-darwin`) and Cargo 1.95.0 (`f2d3ce0bd`). They reused the artifacts from an initial successful four-target check, which took 16.19 seconds. Completion times above are confirmation times, not clean-build performance measurements. The JSON record beside this document retains exact artifact filenames, exit statuses, compiler identity, and checked source SHA-256 hashes.

## Compiler-selection diagnosis

The original invocation failed with `E0463`, reporting missing `core` or `std`. Installed Rust target libraries existed in the rustup stable sysroot. A minimal library compiled directly with that compiler for all four targets.

A temporary `RUSTC_WRAPPER` recorded the executable, version, sysroot, working directory, target argument, and selected environment of Cargo compiler children. Those children resolved `rustc` through `/opt/homebrew/bin` to Homebrew Rust 1.92.0 (`ded5c06cf`) with sysroot `/opt/homebrew/Cellar/rust/1.92.0`. That compiler lacked the requested targets. The top-level shell commands had resolved Rust 1.95.0 through rustup. `rustup run stable cargo` alone retained the mismatch in this environment.

The checks succeeded when `RUSTC` was set for the command to the absolute rustup-selected compiler, with Cargo selected from the same toolchain. No global default changes, toolchain reinstallations, target installations, or core source fixes were needed.

The equivalent host-shell invocation is:

```sh
RUSTC="$(rustup which --toolchain stable rustc)" \
  "$(rustup which --toolchain stable cargo)" check -p tag-core \
  --target aarch64-apple-ios-sim \
  --target aarch64-linux-android \
  --target x86_64-pc-windows-msvc \
  --target x86_64-unknown-linux-gnu
```

## Evidence boundary

`cargo check` validates compilation and dependency type checking for the Rust core. It does not link native applications, build Tauri platform artifacts, run metadata tests on those operating systems, exercise filesystem behavior there, sign packages, or validate Android/iOS devices. Those delivery and runtime checks remain required for cross-platform parity.
