# Native and installer notice material

Working-branch L76 slice. This adds retained supplier material and package
consistency checks; it is not a legal determination, a reproducible-build
claim or evidence that a new release has shipped.

## Retained material

The [descriptor](../../scripts/notices/component-material.json) maps four
grouped native/installer entries to 39 source-material members. These numbers
are not counts of linked libraries or runtime functions.

- **FFmpeg 9.0.2:** 30 complete leading-comment blocks, two inner Boost-derived
  grants (`libavutil/mathematics.c` and `libavutil/libm.h`), and the original
  DFPWM public-domain reference comment. Every selection retains its original
  source-file hash, exact byte range and notice hash from the pinned official
  archive. The Independent JPEG Group acknowledgement is explicit. Existing
  whole-library LGPL and component-license notices remain.
- **Android libc++/libc++abi:** exact supplied LLVM/legacy notice sections from
  NDK r28c, revision `28.2.13676358`. The complete source NOTICE is retained as
  provenance context; the displayed supplement contains the two selected
  runtime sections. It does not label all compiler/toolchain components as
  bundled runtimes.
- **NSIS 3.11:** complete upstream `COPYING` and NSISdl license text from the
  checksum-pinned toolchain ZIP, including the supplied compression-specific
  terms and exception. Keeping a multi-license COPYING is not a claim that
  every listed compression module is used.
- **Tauri NSIS utilities 0.5.3:** full MIT and Apache texts from immutable commit
  `13d9edd27b69310e108d6fbd49f90992f8a05390`, verified against their Git blob and
  SHA-256 identities. Root licenses alone do not establish the complete
  dependency graph of the upstream prebuilt helper DLL.

The [payload](../../THIRD_PARTY_NATIVE.md) is imported by About and appended to
the canonical common notice and escaped RTF. Raw source files preserve their
original bytes across checkouts. Displayed text normalizes line endings, and
upstream text is excluded from translation while headings still translate.

## FFmpeg compiler-source coverage

The prior seven-target release source union has 1,051 files whose bytes match
the official FFmpeg archive. Excluding the three explicit GPL implementation
files described in the [policy repair](ffmpeg-native-policy-audit-2026-10-20.md)
leaves a 1,048-file historical set for focused coverage checking. That pass is
not a new build or proof of the new compiler closure's membership.

The checked-in [checker](../../scripts/component-notices.py) handles complete
leading comments, C comment blocks, and line/assembly comments containing
explicit supplier grants or public-domain references. It verifies exact retained
ranges and requires each recognized block to be covered. An LGPL first header
cannot hide an inner supplier notice. The two small `file_open.c` aliases are
accepted only with their exact include and included source present.

Corresponding-source publication applies this check to the **actual** union of
the new build's compiler inputs, using the pinned archive and notice bytes in
the Git-bound source package. Missing, changed or unmapped blocks refuse
publication. This mechanical safeguard supplements source review; it does not
interpret every possible licensing statement or establish dead-stripped
membership. Independent review found the two initially omitted inner Boost
blocks; their exact bytes and a focused regression now cover that gap.

## Actual baseline package checks

The v0.2.56 APK's `lib/arm64-v8a/libc++_shared.so` is 9,236,352 bytes, SHA-256
`ab4e6c71b96b851de45a8a9bd86369e7dbc2130a44b3b4520564be94847910f2`.
It exactly matches the selected r28c SDK's AArch64 runtime. The SDK NOTICE
has SHA-256 `f96f763beb66a7ba7a667647fc64c0226ace875e590c831fdd9579ec1c1d91e1`.
The corresponding Linux NOTICE was fetched from the official SDK ZIP using
HTTP byte ranges and ZIP CRC verification and exactly matches the installed
macOS SDK's NOTICE. That partial read does not verify the complete SDK archive.

The actual v0.2.56 NSIS installer contains five helper DLLs: System, nsDialogs,
StartMenu, NSISdl and nsis_tauri_utils. Read-only 7-Zip extraction matches every
helper's complete bytes to the pinned NSIS ZIP or Tauri utility download.
The toolchain ZIP's SHA-256 is
`c7d27f780ddb6cffb4730138cd1591e841f4b7edb155856901cdf5f214394fa1`;
the utility DLL's SHA-256 is
`5ba143b5db4a87d32d6e7802e033330aae56cbceabe0d1e3ba41948385ad4709`.
The downloaded files also match the source's SHA-1 download pins for locked
Tauri CLI 2.12.2; the SHA-256 values above are independently recorded identities.
Its NPM metadata identifies commit `5b23c82ea5ba6b9aec420dd71f60493edc486aa9`.
The manifest binds this CLI identity and the existing tracked NSIS toolset pin.
Archive-listing size estimates are not substituted for extracted helper bytes.

The release checks reject a changed or additional helper DLL, a different or
additional Android native runtime, a mismatched NDK revision, or absent selected
runtime notice material. Windows path separators are normalized and aliases
refuse. Exactly one built package must be selected for each target. Inspecting
the archives does not execute Windows, Android, the installer or its uninstaller.

## Qualification and remaining L76 work

The helper's source declares Windows API bindings, internal NSIS API/macro code
and semver. Existing Rust candidate notices contain Windows bindings and semver
material, but no upstream publisher Cargo lock or complete build provenance was
supplied with the DLL. This receipt does not certify its exact transitive
resolution, compiler-support closure or a rebuilt matching binary. Compiler/OS
support and target-specific package qualification remain explicit audit topics.

The [source-provenance control](installer-helper-provenance-2026-10-20.json)
binds all seven inspected source manifests to their upstream Git blobs. The
complete recursive tree has no Cargo lock. There were 28 stable semver 1.x
versions available by the helper's publication date; the latest was 1.0.27.
Its checksum-verified crate contains the same complete MIT and Apache license
bytes already retained for the app's 1.0.28 candidate. This rules out a text
difference for that control, but does not identify the helper producer's version
or establish material coverage for every earlier version.

Static inspection of the hash-verified Linux x64 AppImage found 115 copyright
members, including GTK/WebKit metadata, but no `usr/share/common-licenses` files
despite references to that location. The [working-branch packaging repair](appimage-license-material-2026-10-20.md)
now stages these texts and verifies reference closure after packaging. Actual
new-image delivery and remaining Linux qualifications stay separate L76 work. Its SHA-256 is
`81fe77912a172d0c0695820dc91545ac5bd5ba1c8e4382ed68cc47ef79dad880`.

New-release native integration, actual rebuilt application closure/symbol
checks, new notice delivery and remaining package/source inspection still need
their own evidence. No native application UI replay was run for this slice.

Focused local checks pass: nine component cases, 23 source-bundle cases,
16 affected headless notice/catalog cases, TypeScript, extraction, ledger
consistency and strict guide build. Required release-tooling checks pass:
128 GitHub tooling cases, 28 release cases, and actionlint with only the
documented `queue` syntax exception. Actual source-material and baseline
APK/installer replay pass. Independent review found the inner-notice gap,
verified its repair, and found no further actionable source issue. These
results do not replace final GitHub Codex review or release evidence.
