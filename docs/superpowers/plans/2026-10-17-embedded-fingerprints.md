# Embedded audio fingerprints implementation plan

> **For agentic workers:** Execute with the executing-plans skill in this isolated worktree; root independently reviews the final branch and coordinates normal merge/release.

**Goal:** Make AcoustID fingerprint selected local files inside OpenTag without requiring external fpcalc/FFmpeg executables, then update user documentation in the same feature PR.

**Architecture:** A portable Rust crate owns local files, cancellation/deadline state, one admitted native worker, and C result allocations. A small C++ C ABI uses pinned upstream Chromaprint 1.6.2 and FFmpeg 9.0.2-compatible audio decoding/SWR through custom local-file AVIO; decoder networking/protocols are disabled. Source-built static dependencies and complete corresponding native/application source/build manifests accompany release artifacts; no host Homebrew binaries are shipped.

**Tech stack:** Rust 2021, C++17/C ABI, FFmpeg/Chromaprint, Python 3.12 build orchestration, CMake/GNU Make, existing Tauri platform builds.

**Spec:** docs/verification/embedded-fingerprints.md (source/evidence scope and product contract; derived from the completed /tmp/opentag-fpcalc-native investigation).

## Global constraints

- Keep exact final-head clean Codex CODE review, current-main trusted CI, release notes and normal merge gates.
- Pin and verify archive SHA-256 before extraction/build; never ship the local GPL Homebrew FFmpeg or absolute host paths.
- Preserve existing decoder format coverage, source lookup/ranking/rate policy/API keys and explicit desktop external override.
- Fingerprint TEST2 / internal algorithm ID 1; 11,025-Hz mono s16; compatible SWR filter16/phase8/linear1/cutoff0.8; first120 seconds; full duration quantized to the CLI two-decimal producer contract before current lookup rounding.
- Local regular-file input only; no decoder networking or writable source access. Cooperative cancellation cannot promise hard worker kill; cap admitted native jobs and retain resources until actual return.
- Preserve current release targets: Windows x64 MSVC; macOS arm64/x64; Linux x64/arm64; Android arm64; iOS arm64 simulator.
- Use only this worker's native prefix and Cargo target, debug/incremental disabled for bounded local builds. Gate every heavy phase on free disk greater than2.3GiB; never touch root's stable target.
- Preserve all187 expanded guide pages when rebasing; dated historical verification remains truthful and is marked superseded where needed.

## Review focus

- Cancelling during open/probe/read/decode must release owned native resources eventually and must never admit unbounded replacement jobs.
- Embedded file references/protocol strings must not make the decoder open external files or URLs.
- Odd channel frames, changing layouts, planar channels and decoder failures must preserve the reference output/error contract.
- Half-second duration boundaries must preserve actual current POST duration, not merely approximate media duration.
- Clean released packages must resolve every linked native dependency without locally installed media tools, and expose matching notices/source/rebuild material.

## Task1: Reproducible native dependency build

Files: scripts/audio-deps/{sources.json,build.py,README.md}; crates/audio-fingerprint/build.rs; release/build workflows.

Interface: build.py --target TRIPLE --output DIR produces prefix/include, prefix/lib, component manifest with actual configure/decoder/license/source hashes, and corresponding source inputs.

- [ ] Verify official source archives and retain pinned notices. Select all relevant native audio decoders/parsers and file-family demuxers, no network/protocols/encoders/video/dependency autodetection/GPL/nonfree components.
- [ ] Build owned macOS native prefix with actual source checksums and portable target recipes; assert no host media dylib imports.
- [ ] Add deterministic recipe/checksum/containment/configuration tests and all-target CI tool prerequisites.

## Task2: Portable fingerprint producer

Files: crates/audio-fingerprint/{Cargo.toml,build.rs,native/fingerprint.{h,cpp},src/lib.rs,tests/fingerprint.rs,examples/fingerprint.rs}; workspace/lock.

Interface: fingerprint(path: &Path, control: Control) -> Result<Fingerprint, Error>; Fingerprint carries full quantized duration, encoded fingerprint and processed frame count; Control has shared cancellation and processing budget.

- [ ] Add real generated-media/golden contract tests, invalid duration/short/malformed/source-hash controls and bounded cancellation/admission tests.
- [ ] Implement Rust File/RAII C callbacks and C++ context ownership with exceptions contained at the C ABI. Read-only custom AVIO, interrupt checks, raw count validation, CLI-compatible normalization/prefix/duration/compression.
- [ ] Run focused crate tests and the existing16-case synthetic corpus plus half-second cases through the actual Rust API with PATH empty and source inputs unchanged.

## Task3: Default app integration

Files: src-tauri/{Cargo.toml,src/sources/acoustid.rs,src/sources/mod.rs}; affected frontend settings/source affordances.

- [ ] Replace default subprocess production with the new crate. Preserve explicit configured desktop override; native default on all targets.
- [ ] Bound outstanding blocking workers; UI timeout/cancellation returns promptly while worker resources/admission remain owned until native return.
- [ ] Preserve AcoustID requests, per-file errors, ranking/pairing and credentials. Enable mobile only through approved staged document paths; no arbitrary provider URI/native path assumption.
- [ ] Verify real caller selection/default/override/cancel tests plus appropriate app checks, UI tests/build and relevant platform builds.

## Task4: Source, notices and package verification

Files: scripts/release.mjs and tests; source archive/build manifest generator; About/notices docs; reviewed workflows.

- [ ] Produce the complete corresponding native sources, application sources and target build recipes/configuration/patches/licenses with releases; preserve artifact/source/signature safeguards.
- [ ] Verify all target builds and native linkage/architecture; clean-package smoke tests without media executables or host media libraries. Keep iOS simulator scope explicit.
- [ ] Run review tooling tests and actionlint because this changes release/build integration.

## Task5: User docs and final review

Files: user guide AcoustID/source index/options/whats-new/platform/install/troubleshooting as affected; docs/PARITY.md; new verification record; dated sources-4 supersession note.

- [ ] Explain built-in default and optional advanced desktop override; remove obsolete installation prerequisites only when default native behavior works.
- [ ] Preserve genuine unsupported/encrypted file/provider/cancellation limitations and advanced teaching content; reconcile onto full187-page guide before final PR.
- [ ] Strict guide/publication checks, substantive release notes, root peer review, exact-head clean CODE review, all trusted CI; iterate findings until normal merge/release.
