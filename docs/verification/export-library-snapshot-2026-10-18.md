# Export library snapshot saves — 2026-10-18

## Confirmed defect

At application baseline `bb1c1ecb77c1fed424f054c88cedd55ed11cd069`,
`exports::save` used `load` to inventory removed configurations. Loading also
discovers readable `.mte` files. A file copied into `export/` after the Export
dialog opened therefore became a removal candidate, although the dialog had
never listed it. Saving the cached library moved the new file to `removed/`.

The real Rust backend reproduced this twice with disposable filesystem
fixtures, before changing production code. Both successful saves were followed
by a failing read of the active `Late.MTE`. Its ANSI bytes were
`42 6a f6 72 6b` (Windows-1252 `Björk`). No filesystem persistence was mocked.

## Implemented behavior

The [backend](../../src-tauri/src/exports.rs) inventories retirement candidates
from saved metadata and the filenames observed when the caller loaded the
library. It does not discover files while deciding removals. The transient
`observed_files` list survives the [dialog's copies and edits](../../src/export-ui.ts)
but is excluded from persisted `exports.json`.

Saved metadata also covers a configuration created and saved earlier in the
same open dialog. An initial implementation using only the loaded baseline
failed that explicit-removal regression; the final union preserves removal
without admitting unseen external files. Legacy/programmatic callers without
an observation list use saved metadata alone.

Removal intent is recorded in the same metadata replacement that removes a
configuration. If moving its file to `removed/` fails, retrying the same dialog changes still knows
which file to retire. This internal pending list is validated on read, omitted
from the library sent to the dialog and pruned by a later save once the files
are absent. A recovered file included in the submitted library is retained.
Reopening after a failure still discovers the unretired physical file. Keeping
that listed file cancels its pending removal; removing it again requests
retirement. This preserves the existing reopen behavior, without hidden
automatic removal after restart.

This preserves both intended operations: newly dropped unseen files stay
active, and explicitly removed listed files move to `export/removed/` with
their original bytes. Reopening discovers retained files; copying a retired
file back into `export/` permits recovery.

## Local regression evidence

The original stale-save regression failed twice before the fix and passed
twice after it. Each passing run saves the cached library twice. A separate
create/save/remove/save regression failed against the initial fix, then passed
after the saved-metadata correction.

Independent review then identified a failed-retirement retry case. With
`export/removed` deliberately occupied by a regular file, both dialog-shaped
and legacy payload tests reproduced the lost removal identity. Both failed
before durable intent was added and passed afterwards, including two retries
after removing the owned obstruction.

The 28 export-backend tests pass, including these controls:

- Preserve a late external ANSI template byte for byte through repeated saves,
  then discover it on reopen.
- Explicitly remove a discovered but not yet metadata-saved template, with and
  without an existing metadata file; verify archived bytes and copy-back recovery.
- Create and save a template, drop an unseen external template, remove the
  created one and save twice; retire only the listed template, once.
- Retain unseen files for a caller without an observation baseline.
- Reject unsafe or duplicate observed filenames before writes.
- Reject unsafe or duplicate pending-removal filenames in saved metadata;
  retain unseen files during failure/retry and cancel pending retirement for
  a recovered listed file.
- Restore empty export metadata through the real configuration-restore
  function, rediscover retained physical templates and explicitly remove one.
- Keep the observation list and template bodies out of saved metadata.

All eight focused [frontend tests](../../tests/export-ui.test.mjs) pass,
including independent clones retaining the baseline through duplicate, rename
and removal. The tests use actual backend/filesystem operations and frontend
functions; they do not establish native dialog input or original-runtime parity.

## Limits

This closes the L44 stale-library removal defect only. Reconciliation of
external edits to already-listed templates and concurrent writers remains
outside this change. Metadata/templates are still individually replaced,
without a multi-file transaction. The restore control retains existing physical
MTE files; it does not show that a portable backup contains them. Original
Mp3tag, installed Windows/Linux/macOS input, providers and physical mobile
devices were not exercised by these regressions. The broader encoding,
ordering and original-runtime qualification remains in the [parity ledger](../PARITY.md).
