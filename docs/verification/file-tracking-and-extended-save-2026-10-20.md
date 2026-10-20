# Individually loaded file tracking and Extended Tags saves

Working-branch evidence dated 2026-10-20, based on main `7544625`.
This records local implementation and focused verification for L47a and L73.
It is not a merged-PR, published-release or original-Windows execution receipt.
Both intake IDs remain active until the P1 batch has a covering PR.

## Refresh follows the captured file

[Native tracking](../../src-tauri/src/source_tracking.rs) captures an owned
file handle before scanning an individually opened audio source. A random
session token identifies that capture. Refresh verifies the file's native
identity and current handle-derived location; it never enumerates a missing
file's parent to choose a replacement. macOS uses `F_GETPATH`, Linux uses
`/proc/self/fd`, and Windows uses full `FILE_ID_INFO` with the final handle
path. Local runtime evidence here is macOS only.

Existing valid aliases retain the chosen logical path. An occupied old name,
retargeted alias or ambiguous hard-link move refuses. A deleted old object
can be replaced at its existing explicit path, supporting atomic external
saves without treating a different still-linked object as the source.
Non-regular inputs refuse without waiting for a FIFO writer.

[Frontend ownership](../../src/source-tracking.ts) retains per-file tokens
through accepted loads, additions and known renames. A canceled or stale
preparation releases only its newly acquired tokens. Cleanup failures are
retryable and do not recapture accepted sources by filename. Workspace
archives contain paths, never native handles or session tokens.

The [Refresh host](../../src/main.ts) checks workspace generation, pending
edits and cancellation before publishing results. Verified renames rebase
explicit roots, aliases, selection and manual ordering without admitting
their parent folders. Save and [journal replay](../../src-tauri/src/history.rs)
rebind the same token to the resulting file. Write epochs are captured after
history snapshots are created; successful-write rebinding happens before a
grouped commit removes redundant snapshots. Replay additionally requires a
verified pre-operation identity and a successfully restored path, since the
old object can remain linked inside the history store.

Focused regressions reproduced the previously stale operational address,
the Save/Undo/Redo snapshot handoff and the grouped-save hard-link case.
The final backend run executed **14 tests successfully**, with one ignored
subprocess-worker entry and 603 unrelated cases filtered out. It includes
actual WAV Save, Undo and Redo commands, exact saved/original bytes, grouped
saves with retained hard links, and a second external rename. The harness
took 0.55 seconds after compilation. **75 headless host/ownership tests**
passed, including cancellation, queued cleanup, aliases and workspace loads.
Scoped production Clippy passed. No native UI/app replay was run locally.

Backend run: `taggroove-l47a-group-snapshot-green-45fgendi`.
Host run: `taggroove-l47a-final-host-wx673stx`.
Clippy run: `taggroove-p1-backend-clippy-t0djwmjc`.
The local runner retains command, source bindings, stdout/stderr and exit status.

## Extended Tags confirms or stages deliberately

The original [Extended Tags documentation](https://docs.mp3tag.de/extended-tags/)
states that confirming the dialog saves its selected files and that navigation
uses the configured save policy. [Tags options](https://docs.mp3tag.de/customization/options/tags/)
describes the separate main-list automatic-save setting. These are documented
contracts; no original application was executed for this change.

[Extended Tags](../../src/extended-tags.ts) now offers primary **Save tags**
and separate **Keep changes**. Save stages the displayed selection, uses the
existing batched save/journal path, and closes only after all targeted drafts
are saved. Failure, cancellation or partial success leaves the dialog open
with unsaved drafts available for retry. A current save-back refusal leaves
Keep changes available. Previously visited files kept pending are not silently
included in a later save of another displayed file.

The main host no longer invokes general save-on-move when Extended Tags
changes its selection. Its own pending, ask, ignore and save modes decide
what to do before navigation. A failed navigation save stays on the same file.

Each displayed file has an immutable opening field snapshot. Retrying a
name-only rename therefore preserves individual values in mixed selections.
The dialog also reconciles keys owned by earlier staging when a failed rename
is corrected or reverted, including files already saved in a partial result.
During saves, existing controls are disabled, fresh field menus are refused,
and cover redraws cannot create newly enabled editing controls. Cover actions
retain their separately documented immediate-write behavior.

**67 headless tests passed**: 12 focused save/navigation/retry cases and 55
existing value-suggestion cases. The rename retry and corrected-name failures
were reproduced before their fixes. Tests cover deferred saves, duplicate
submission, failed/partial mixed-value renames, corrected/reverted names,
Ask discard/cancel, pending navigation, save-back refusal and menu/cover locking.
Run: `taggroove-l73-corrected-rename-green-h9qc200r`.

The [native keyboard scenario](../../tests/e2e/keyboard-ui.js) additionally
checks pending navigation with general auto-save enabled, explicit Save of
the displayed file, unchanged earlier-file title and Undo. It is syntax-checked
only; execution belongs to postmerge native CI. Original Windows timing,
physical keyboard acceptance and released builds remain separate qualifications.

The user guide now explains [Refresh](../user-guide/getting-started/opening-files.md#refresh-after-changing-files-elsewhere),
[Extended Tags saving](../user-guide/guides/extended-tags.md) and
[independent navigation modes](../user-guide/reference/options/tags.md).
