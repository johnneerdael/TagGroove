# Distribution remainders: source audit — 2026-10-19

This read-only audit inspected current source and existing bounded receipts.
No build, native application, signing tool, authentication change, installation,
publishing action or new artifact download was performed. L58/L70/L71/L76
remain active. L69 is separately represented by existing C47 because its usable
macOS argument route is already implemented and documented; real running-app
qualification remains local.

Line numbers below identify the inspected source snapshot and can shift after
integration. Source configuration, test existence, a package, installation and
runtime behavior are separate evidence layers.

## L58: updates

[The update command](../../src-tauri/src/updates.rs) at lines 162–190 requests
GitHub release metadata and selects the newest applicable release. Lines
193–207 validate a release-page URL and open it in the browser. The
[frontend](../../src/main.ts) at the inspected lines 1998–2007 explicitly says
it does not download/install updates and dispatches `open_release_page`.
[Release packaging](../../.github/workflows/reviewed-release.yml) line 192 sets
`uploadUpdaterJson: false`.

The check, beta policy and browser handoff are implemented. No downloaded-update
verification, installation or integrated restart workflow was found. The
remaining implementation is real and can be designed/implemented locally;
platform installer/upgrade acceptance follows on the relevant OS. Do not mark
this as a completed updater or assign portable implementation away merely
because Windows installation eventually needs a Windows host.

## L70: associations and installed identity

[The macOS-only configuration](../../src-tauri/tauri.macos.conf.json) lines
4–76 declares document associations. The shared
[configuration](../../src-tauri/tauri.conf.json) contains no association list;
[the association test](../../tests/launch-forwarding.test.mjs) lines 39–40
explicitly enforces that macOS-only policy. The
[historical launch receipt](launch-forwarding.md) lines 29 and 73 records the
Windows/Linux omission, including the Linux desktop argument limitation.

Windows/Linux association declarations therefore remain actual missing
packaging configuration. The shared CLI and single-instance route is already
implemented in [file operations](../../src-tauri/src/file_ops.rs), lines
604–651 and 724–741. Local configuration/static validation can precede real
Windows/Linux default-handler, file-manager and installed opening checks.

Stable Windows upgrade identity is already present:

- [Bundle configuration](../../src-tauri/tauri.conf.json), lines 47–51:
  preserved MSI upgrade code and a custom NSIS template.
- [Installer](../../src-tauri/windows/installer.nsi), lines 69–74:
  stable OpenTag uninstall/publisher/install-location keys and legacy executable
  identity; association macros at lines 737/879 are templated machinery, not
  evidence that missing association declarations were supplied.
- [Identity rules](../../src-tauri/windows/README.md), lines 13–25:
  legacy MSI/executable/shortcut continuity and explicit native-upgrade limits.
- [Release test](../../tests/release.test.mjs), lines 24 onward, checks actual
  configuration/template identity; [NSIS shortcut tests](../../tests/nsis-shortcuts.test.mjs)
  exercise the production shortcut macros. Their presence is not a new test
  execution claim in this audit.

Keep the existing identity implementation out of the missing-code description.
Actual installation, file opening, upgrade and uninstall remain native
Windows/Linux acceptance. This audit records the completed subset without
inventing a new C61 or treating configured continuity as a runtime pass.

## L71: signing and physical iOS distribution

[macOS bundle configuration](../../src-tauri/tauri.conf.json), lines 43–45,
sets `signingIdentity` to `-`. The
[reviewed release workflow](../../.github/workflows/reviewed-release.yml),
lines 358–365, builds iOS with `aarch64-sim`, `--debug` and `--no-sign`.
[Release collection](../../scripts/release.mjs), lines 540–549, verifies an
`IOSSIMULATOR` executable and packages its app archive. Those are implemented
ad-hoc/simulator production policies, not a configured Developer ID/notarized,
Authenticode or physical-iOS distribution path.

Physical iOS native dependency support is already implemented in
[the native build recipe](../../scripts/audio-deps/build.py), lines 215–236:
`aarch64-apple-ios` selects `iphoneos`, the device target triple and iOS CMake
settings separately from the simulator. That does not produce a signed IPA,
provisioned install, TestFlight/App Store distribution or device runtime proof.

Production signing/release integration is still work; Tauri's available signing
features and Android release-key validation do not establish these missing
platform paths. Keep macOS setup/artifact inspection on the available Mac;
credential-dependent signing/notarization requires the actual Apple account and
certificate. Windows signing needs its Windows toolchain/certificate, and
physical iOS installation needs a device plus valid provisioning/distribution
credentials. No credential availability or signed publication was inspected.

## L76: license, artifact and reproducibility audits

The [published v0.2.46 Rust-source receipt](released-source-license-inventory-2026-10-19.md),
lines 20–29, records 33,491 hash-verified source files and 601 registry package
inventories, with bounded independent review. Lines 31–35 expressly retain
binary-specific, JavaScript, nested native and reproducibility scope. This is
an older published source artifact, not PR64 build or delivery proof. C36
separately represents the Boost notice repair.

Current source contains meaningful implemented infrastructure:
[offline component notices](../../src/audio-licenses.ts), lines 1–18;
[tracked notices](../../THIRD_PARTY_NOTICES.md); and
[corresponding-source validation](../../scripts/audio-source-bundle.py), lines
295–322 and 374–400, which checks vendored package identity/checksums and native
source/license bytes. These are not a complete mapping from each shipped
binary's linked content to its required notices/relinking material, or a full
JavaScript dependency audit.

Remaining notice/linked-content/relinking audits, available artifact inspection
and macOS reproducibility checks can be performed locally. Static unpacking and
inspection of Windows/Linux archives also does not inherently require those
operating systems. Target-native installation/runtime and reproducible-build
execution can require the appropriate host/runner; only those genuine
environment dependencies belong in the external handover. No new license
finding or completed all-platform audit is asserted here.
