# TagGroove rename verification

Scope: user-visible application branding, product/executable/download names, supplied artwork and the complete user guide. No application behavior change is intended. The gnudb client remains exactly `OpenTagAudioTagEditor`.

## Image audit

`taggroove-image-audit.json` inventories every one of the 412 tracked images at baseline `adbd850`. Each entry records its original hash, visual classification and disposition. The audit covered nine numbered contact sheets of raster images, SVG renders, and the largest representations of ICO/ICNS containers. All wordmarks and app-name artwork are replaced from the supplied TagGroove kit. The supplied guide’s 32 embedded previews were decoded and matched against kit-manifest hashes. The guide stylesheet’s embedded SVG is an unbranded edit pencil. Icon-only artwork keeps the selected identity. Fifty-six unused earlier-generation templates are removed. A second visual pass inspected all 76 distinct replacement image contents (48 SVG and 28 PNG); identical copies are matched by hash, and unchanged files retain the original visual inspection. All replacement wordmarks read TagGroove.

Historical screenshots and structured receipts under `docs/verification/`, `docs/evidence/`, guide `_meta/coverage/`, and native-provider/fingerprint verification retain original names, bytes and hashes. They describe earlier builds and are not proof of TagGroove runtime behavior. All ten public-guide screenshots are replaced with unmodified, individually inspected TagGroove CI captures. The completed inventory accounts for all 356 current tracked images with matching hashes and no pending replacements.

## Compatibility

Application IDs, storage keys, configuration/tag-panel file discriminators, native/Cargo interfaces, build environment variables, provider registrations, both the shared HTTP and browser-style web-source user agents, and signing settings remain stable. The visible executable is `taggroove`, declared separately from the internal Cargo package. Existing version numbers are unchanged. Product-derived installer/package names change with the brand; installation upgrades on other platforms have not been device-tested. Stable data identifiers preserve configuration locations; this is not a claim of identical installer behavior.

## Validation

- Supplied manifest: 249 files hash-verified.
- Integrated resources: 78 copies checked by `python3 scripts/verify-brand.py`.
- Frontend build and strict MkDocs build passed during implementation.
- Rust workspace: 1,082 passed, 6 ignored; UI tests: 559 passed.
- Current integrated tooling: 107 Python controller/tooling tests, 27 release tests and 26 source-package tests passed. Publication boundary: 8 tests passed before adding the prerequisite capture route; rechecked with that route in the prerequisite PR.
- macOS arm64 debug bundle built at `/tmp/taggroove-target/debug/bundle/macos/TagGroove.app`, using the `taggroove` executable. The existing ad-hoc signing policy is retained; no notarization or installation test is claimed.
- Browser preview visibly renders the new header and palette; the guide preview renders the new wordmark. These checks are separate from native CI capture.
- Rendered-guide text scan: 175 HTML files checked. Remaining old-name text appears only in configuration compatibility identifiers, the unchanged gnudb client name, and explicitly historical Linux package instructions.
- Ten genuine guide captures from [reviewed run 39071322009](https://github.com/johnneerdael/TagGroove/actions/runs/39071322009) were individually inspected in both themes: main window, multi-selection keep markers, Convert menu, Tag to Filename preview, and extended tags. Each visibly uses TagGroove artwork and the intended fixture state. The committed PNGs match the downloaded artifact byte for byte.
- Capture source: reviewed head `ba7e849cfb59e4a6d532004cd6f3b0d73851260e`, trusted main `4a1a95ff4eddb9da7492aaace68197c4f81347d0`, immutable test merge `cac0ea56396b457fd8a4563b70fdb44a05c8aacc`; Linux job `117509476998`, artifact `12115172565`. Per-image hashes and the GitHub artifact digest are recorded in `taggroove-image-audit.json`.
- Capture and upload passed under the existing exact-head authorization. This source run's desktop jobs failed a rustfmt line-wrap check on the releases URL. That formatting is corrected; formatting the captured and current `updates.rs` produces identical bytes, as recorded in the audit. This capture receipt does not assert that the source run passed overall validation.
- After capture, review also found a separate browser-style web-source user agent whose token had been renamed. Its original `OpenTag` token is restored, matching canonical main. That wire-only correction does not change the captured artwork, layout or fixture states; the audit records both post-capture source changes.
- Final-head GitHub Codex review and full trusted validation remain required before merge.
