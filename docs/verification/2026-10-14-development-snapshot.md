# Initial OpenTag development snapshot

This is an incomplete independently authored reconstruction. None of these checks establish exact Mp3tag feature, behavior or performance parity. `../PARITY.md` records the remaining scope.

## Current source verification

- `npm run build`: passes, supplied OpenTag SVG identity and palette bundled locally.
- `npm run test:ui`: 15 tests pass.
- `cargo fmt --all -- --check`: passes.
- `cargo test --workspace --locked`: 159 tests pass, zero failures, zero ignored; all doctest suites pass. The full output is retained in `2026-10-14-workspace-tests.txt`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passes; output retained in `2026-10-14-workspace-clippy.txt`.

The verification commands use the installed rustup stable compiler and rustdoc explicitly. This machine also has Homebrew Rust; mixing those toolchains in a shared target directory previously invalidated a doctest dependency during a concurrent build. The final workspace pass uses one toolchain. A Cargo global-cache cleanup permission warning did not change the successful command results; no global permissions were modified.

## Meaningful regression coverage

Real encoded fixtures cover metadata changes, native unknown fields, pictures and audio-packet preservation across eight encoded formats plus WAV. The mapping sweep tests 173 fixed ID3v2.3/v2.4 cells, 78 MP4 cells and nine RIFF cells. Unsupported formats and mapping ambiguities remain listed in `../CRITICAL-MAPPING.md`.

Native action tests exercise ordered import/format/rename workflows, directory rebasing of selected files and unselected siblings, per-file collision reporting, persisted group configuration, reviewed rename receipts, deferred MTE export, UTF16 configuration loading, output encoding/append and streamed file checksums.

Review found and reproduced an output alias that replaced disposable fixture audio through a missing directory followed by `..`. The output guard now resolves existing symlinks and normalizes missing components before directory creation. Playlist output is also rendered and checked for partition collisions before any destination is written. Regression tests require input bytes to remain unchanged and duplicate partitions to produce no output.

The export engine shares immutable loop membership instead of copying a group's indices for every row. A 40,000-row equal-key regression checks all titles, counters and totals. Microbenchmark and full-render observations are qualified separately in `../CRITICAL-EXPORT.md`; they are not original-app performance comparisons.

## Platform and brand evidence

See `mobile-android-debug.md` and `mobile-ios-simulator.md` for exact commands, source-snapshot comparisons, artifact hashes and platform inspection. Native builds do not establish mobile document-provider editing, save-back, lifecycle behavior or physical-device compatibility. Windows/Linux native qualification and same-corpus original performance comparisons remain pending.

`../../assets/brand/VERIFICATION.json` records the user-provided assets and copied resources. Original kit bytes remain preserved. Live UI screenshot verification remains unavailable because the computer-use observation service failed; no screenshot or device behavior claim is inferred from asset or build checks.

The source repository excludes the proprietary executable, vendor artwork/templates/translations/source scripts, local music, build products and caches. Retained REA evidence consists of identification, static resource metadata and feature inventory; implementation tests remain distinct from original-app differential tests.
