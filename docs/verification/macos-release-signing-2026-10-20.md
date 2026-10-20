# Mac release signing and notarization

The working branch implements L71's approved desktop Mac scope. This is source
and focused-test evidence, not a claim that an older release was notarized or
that the new packages have passed a native build. Windows signing and physical
iOS distribution remain separately deferred/qualified.

## Release-only configuration

`src-tauri/macos/signing-policy.json` pins the Developer ID Application identity
and team from the owner's existing certificate. The release workflow uses the
existing Actions certificate/password and the three `APPLE_NOTARY_*` credentials.
The owner authorized their transfer from personal Codespaces secrets; all three
Actions names were verified and the temporary Codespace was deleted. No private
values belong in this receipt, the repository, command arguments or logs.

`scripts/macos-release.py prepare` requires all credentials, writes the notary
key with mode0600 inside a mode0700 directory outside the checkout, and creates
a Tauri override for the pinned identity, hardened runtime and mandatory stapling.
Missing/invalid credential structure refuses before building. A failed partial
write removes newly created private staging. Guarded cleanup runs regardless of
later job success and refuses unknown or changed staging material.

The normal source configuration retains its ad-hoc signing identity for local
development/native tests. Signing secrets are scoped to Mac release steps;
non-Mac builders and premerge validation receive none of these credentials.

Certificate import is performed by the pinned CLI itself. Its
[bundler keychain entry point](https://github.com/tauri-apps/tauri/blob/5b23c82ea5ba6b9aec420dd71f60493edc486aa9/crates/tauri-bundler/src/bundle/macos/sign.rs#L18-L35)
reads `APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD` and selects the
certificate-backed keychain path. The
[signing library implementation](https://github.com/tauri-apps/tauri/blob/5b23c82ea5ba6b9aec420dd71f60493edc486aa9/crates/tauri-macos-sign/src/keychain.rs#L52-L177)
creates and unlocks a temporary keychain, imports the certificate/private
identity, configures tool access and selects its signing identity. Its drop
handler removes that keychain. The direct Mac CLI build step supplies both
certificate variables; the preparation helper stages the separate notary key
and configuration. An action-specific import step is not a prerequisite for
this locked CLI path. Registry metadata for CLI2.12.2 identifies the same Git
commit and matches the integrity value in `package-lock.json`.

## Verification and package binding

The locked Tauri CLI2.12.2 source at commit
`5b23c82ea5ba6b9aec420dd71f60493edc486aa9` signs/notarizes/staples the app before
returning its Mac bundle. Its DMG path signs the image but does not notarize or
staple it. The release helper therefore submits the signed DMG explicitly using
`notarytool --wait`, requires Accepted, then staples and validates its ticket.
There is no second application build.

Before collecting packages, the helper checks the pinned signer/team, secure
timestamp, application hardened runtime, expected bundle identity/version,
stapled tickets and Gatekeeper assessments. It mounts the DMG read-only,
verifies the contained app's full seal and metadata, and compares its CodeDirectory
identity with the verified loose app. A stale same-signer image cannot pass by
presenting only a valid outer signature. The image is detached on failure too.
Only then does the helper create the collected app archive.

Each Mac receipt binds commit/version/target, Git-bound signing policy and both
final package hashes. Both receipts enter the corresponding-source archive.
Offline validation checks their structure, policy, exact source binding and
external-sidecar equality; the final publisher additionally matches their hashes
to the actual package files. These are trusted Mac-builder observations, not a
claim that a Linux publisher independently ran Apple's native validation tools.

## Focused evidence and remaining acceptance

Six focused helper tests use temporary files and injected Apple-command results.
Red-green regressions cover partial-key cleanup, stderr corrupting notary JSON,
a mismatched DMG-contained app and refusal before notarization/archive creation.
Other cases cover missing credentials, private modes, changed/unknown cleanup
material, wrong signer/team, missing timestamp/runtime and receipt identity
mismatches. Corresponding-source tests accept both synthetic target receipts and
refuse changed policy/sidecars, including tampering after outer rehashing.

No app was built, mounted, submitted to Apple or launched locally for these tests.
Actual arm64/x64 signatures, accepted submissions, stapled packages, postmerge
native integration and published release assets must be observed before L71 is
closed. Existing release metadata and checksums alone do not establish those
results. Apple's [Developer ID guidance](https://developer.apple.com/developer-id/)
describes signing and notarization as separate distribution checks.
