# TagGroove rename verification

Scope: user-visible application branding, product/executable/download names, supplied artwork and the complete user guide. No application behavior change is intended. The gnudb client remains exactly `OpenTagAudioTagEditor`.

## Image audit

`taggroove-image-audit.json` inventories every one of the 412 tracked images at baseline `adbd850`. Each entry records its original hash, visual classification and disposition. The audit covered nine numbered contact sheets of raster images, SVG renders, and the largest representations of ICO/ICNS containers. All wordmarks and app-name artwork are replaced from the supplied TagGroove kit. The supplied guide’s 32 embedded previews were decoded and matched against kit-manifest hashes. The guide stylesheet’s embedded SVG is an unbranded edit pencil. Icon-only artwork keeps the selected identity. Fifty-six unused earlier-generation templates are removed.

Historical screenshots and structured receipts under `docs/verification/`, `docs/evidence/`, guide `_meta/coverage/`, and native-provider/fingerprint verification retain original names, bytes and hashes. They describe earlier builds and are not proof of TagGroove runtime behavior. Public-guide screenshots require new captures; they are not retouched to manufacture verification evidence.

## Compatibility

Application IDs, storage keys, configuration/tag-panel file discriminators, native/Cargo interfaces, build environment variables, provider registrations, the shared HTTP user agent, and signing settings remain stable. The visible executable is `taggroove`, declared separately from the internal Cargo package. Existing version numbers are unchanged. Product-derived installer/package names change with the brand; installation upgrades on other platforms have not been device-tested. Stable data identifiers preserve configuration locations; this is not a claim of identical installer behavior.

## Validation

- Supplied manifest: 249 files hash-verified.
- Integrated resources: 78 copies checked by `python3 scripts/verify-brand.py`.
- Frontend build and strict MkDocs build passed during implementation.
- Rust workspace: 1,082 passed, 6 ignored; UI tests: 559 passed.
- Review/release tooling: 84 tests passed. Publication boundary: 8 tests passed before adding the prerequisite capture route; rechecked with that route in the prerequisite PR.
- macOS arm64 debug bundle built at `/tmp/taggroove-target/debug/bundle/macos/TagGroove.app`, using the `taggroove` executable. The existing ad-hoc signing policy is retained; no notarization or installation test is claimed.
- Browser preview visibly renders the new header and palette; the guide preview renders the new wordmark. These checks are separate from native CI capture.
- New guide captures and final-head GitHub validation remain pending.
- Guide captures run only in the existing review-authorized Linux CI workload. After downloading that artifact, inspect all ten images, record the source run/head and update the public guide before the final review and merge.
