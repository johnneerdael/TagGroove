# Provider-document read policy

R19 includes a confirmed mismatch: provider-document import, reload, recovery
and post-publication descriptions used default tag-core readers, so selected
tag-kind read switches and custom field mappings did not apply. The Android
store now receives the existing `settings::tag_options` supplier and uses the
shared full/summary readers with those options and mappings. Options are read
on demand, rather than captured once at store initialization. A working-copy
read and its permitted verified-baseline fallback use one captured policy.

## Read-path audit

The integration audit uses approved main `54ed97e2bba6bb957cc5ecc068de6bb86feaa455`
as its baseline. The following metadata paths use the selected policy:

| Path | Description source |
| --- | --- |
| New import and reselecting an existing handle | `Store::entry` full reader |
| Reload result | `Store::entry`, after validating and replacing the provider snapshot |
| Successful source save | `Store::publish` calls `Store::entry`; the mutation closure's description is discarded |
| Both Original backup and Edited copy results | `Store::save_copy` publishes through the same `Store::publish` path |
| Recovery restoration | `Store::entry` reads retained edits, with a verified-baseline fallback only if a non-ready working copy cannot be read |
| Retained edited-copy summary | `Store::read_working` summary reader; no baseline substitution or provider access |

Reload and edited-copy preflights also use the options-aware full reader.
`Store::project` only replaces filename/path/time presentation with provider
identity; it does not read metadata. The default reader in `documents::reload`
is the non-Android, unmanaged-file fallback. Byte snapshots, hashes, manifest
reads and artwork input snapshots are not metadata descriptions.

The audit found no additional production read-policy defect after the main
integration. PR #53's copy-command tests still expected native field names in
returned descriptions; those expectations now require mapped editor fields,
while their independent native-item, audio and artwork checks remain intact.
The integration adds regression coverage rather than a new startup state
mechanism. Existing frontend startup reconciliation re-reads recovered edited
bytes; this change preserves that behavior. `Store::open` checks recorded state
and byte fingerprints and does not introduce a new parse/readability check.

## Reproduction and regression scope

Six focused cases exercise the real filesystem-backed persistence engine and
settings-backed document adapter:

- Conflicting MPEG APE/ID3v2 titles, mapped import, reload and post-save result.
- Changed policy on an existing handle, summary, duplicate import and recovery
  through a verified baseline when the working copy is unreadable.
- A specification-derived WavPack block with competing APE/ID3v1 titles,
  Ape, Mpc read switches and an APEv2 custom mapping.
- Three competing MPEG tag kinds, independently disabling APE, ID3v2 and ID3v1,
  including a view with all three disabled.
- Original backup and Edited copy result projection, pending mapped fields and
  readable retained edits after publication failure and restart.
- The `settings::tag_options` adapter, changed settings after import, working
  summary, reload, source-save result and both Save a copy results.

The controls retain provider URI projection and verify read-only operations do
not change provider or working bytes. The fixtures use hand-built MPEG frames
and WavPack block headers with serialized tags. They are not original-Mp3tag
captures or real-encoder qualification. The settings-backed adapter runs on the
calling test thread, without launching Tauri or exercising Android IPC.

A fresh default-reader control temporarily replaced only full/summary metadata
readers with default readers while keeping the PR API and tests. All six cases
failed with the expected wrong or missing selected/mapped fields. The source
was restored exactly afterwards. This is a controlled reproduction of the
original policy defect, not a newly discovered defect in the integrated branch.
The log is `/tmp/taggroove-pr59-provider-policy-default-reader-red.log`.

The unmodified PR #53 copy-command group also reproduced its obsolete result
expectation: it reported three passes and four failures, with one primary
native-name assertion failure and three clipboard mutex-poison cascades.
An isolated run of the artwork-plus-fields case confirmed the same primary
failure independently. Logs are `/tmp/taggroove-pr59-copy-command-integration-red.log`
and `/tmp/taggroove-pr59-copy-fields-integration-red.log`. After correcting the
result expectations, all seven copy-command cases passed, including original
backups and failure/retry controls.

## Local validation

Fresh integration checks use:

```sh
env AR=/usr/bin/ar RANLIB=/usr/bin/ranlib CARGO_TARGET_DIR=/tmp/taggroove-target OPENTAG_AUDIO_SOURCE_DIR=/tmp/taggroove-parity-audio-sources cargo test -p opentag --lib documents:: --locked
env AR=/usr/bin/ar RANLIB=/usr/bin/ranlib CARGO_TARGET_DIR=/tmp/taggroove-target OPENTAG_AUDIO_SOURCE_DIR=/tmp/taggroove-parity-audio-sources cargo test -p opentag --lib document_copy_tests:: --locked
env AR=/usr/bin/ar RANLIB=/usr/bin/ranlib CARGO_TARGET_DIR=/tmp/taggroove-target OPENTAG_AUDIO_SOURCE_DIR=/tmp/taggroove-parity-audio-sources cargo test -p opentag --lib --locked -- tests::managed_commands tests::working_entries tests::bulk_group_import
cargo fmt --all --check
git diff --check
/tmp/opentag-pr45-docs-venv/bin/python -m unittest discover -s docs/tests -v
/tmp/opentag-pr45-docs-venv/bin/python scripts/docs-status.py
/tmp/opentag-pr45-docs-venv/bin/python -m mkdocs build --strict --site-dir /tmp/taggroove-pr59-provider-read-guide
```

The focused policy cases passed before the default-reader control in
`/tmp/taggroove-pr59-provider-policy-green.log`. Restored-source validation passed:

| Check | Result | Log |
| --- | --- | --- |
| Document engine and settings-backed adapter | 23 passed | `/tmp/taggroove-pr59-documents-final.log` |
| Copy commands | 7 passed | `/tmp/taggroove-pr59-copy-command-final.log` |
| Managed commands, working entries and provider group imports | 6 passed | `/tmp/taggroove-pr59-managed-command-fixtures-final.log` |
| Documentation tests | 8 passed | `/tmp/taggroove-pr59-docs-tests.log` |
| Writing plan | 176 written / 188 planned; 12 drafts | `/tmp/taggroove-pr59-docs-status.log` |
| Strict guide build | Passed | `/tmp/taggroove-pr59-guide.log` |
| Cargo formatting and diff whitespace | Passed | No output |

The writing plan includes Android and public navigation registers it under
Platforms. These counts describe this branch's source, not a confirmed deployed
site. Final full workspace/JavaScript tests, strict workspace Clippy and build
checks are recorded separately below; the focused receipts do not substitute
for them.

### Final integrated local validation

The final integrated source passed 625 frontend tests and 1,150 Rust workspace
tests, with nine existing Rust tests ignored. After replacing two unnecessary
URI clones in test assertions with borrowed-slice comparisons, the full Rust
workspace suite passed again with the same counts. Production reader files
were unchanged by that test-only correction.

Strict all-target workspace Clippy, the frontend build, Cargo formatting,
catalog extraction and diff checks passed. The documentation publication suite
passed eight tests, the writing plan reported 176/188 pages with 12 drafts,
and the strict guide build passed. Final local logs are
`/tmp/taggroove-pr59-final-npm-test.log`,
`/tmp/taggroove-pr59-final-rust-tests.log`,
`/tmp/taggroove-pr59-final-clippy.log`,
`/tmp/taggroove-pr59-final-build.log` and
`/tmp/taggroove-pr59-final-mkdocs.log`.

These local checks do not replace exact-head GitHub Codex review or protected
current-main CI and do not establish device or original-app qualification.

### Historical pre-integration receipts

Earlier work recorded 1,084 workspace tests and 409 app-library tests, with six
existing ignored tests, plus Clippy, formatting and guide checks. Those are
historical receipts from before the current approved-main integration, not fresh
full-suite evidence for this branch. Their logs remain
`/tmp/taggroove-provider-read-policy-workspace.log`,
`/tmp/taggroove-provider-read-policy-final-lib.log`,
`/tmp/taggroove-provider-read-policy-clippy.log`,
`/tmp/taggroove-provider-read-policy-final-clippy.log` and
`/tmp/taggroove-provider-read-policy-guide.log`. The original reproduction log
was `/tmp/taggroove-provider-read-policy-red.log`.

PR #53 has merged. Its pending-cover and pending-field copy writes, reverse
mappings, physical ordering and single write-policy snapshot are preserved;
there is no remaining default pending-field writer limitation to assign to a
frozen PR. Provider transport, baseline lifetime and publication authorization
are unchanged by this read-policy work.

## Remaining qualification

No Android device, SAF implementation, cloud provider or iOS document route
was exercised by these desktop engine tests. Real-device read/edit/save/reload,
permission loss, interruption and recovery remain open. This change addresses
the read-policy portion of R19; it does not close R19 or establish original-Mp3tag
parity. PR #47's branding hold has ended. Exact-final-head clean Codex code
review and authorized current-main protected CI remain merge requirements.
