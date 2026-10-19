# Undo and history

TagGroove's desktop session history records supported changes to local audio
files. It is separate from pending tag edits, text-box undo and remembered
format strings. Read the operation name in the Edit menu before restoring
anything: history follows the session's recorded operations, not just the
files you have selected now.

## Choose the right recovery route

| State to recover | Use | What to keep in mind |
| --- | --- | --- |
| Text just typed inside an input | That input's normal undo shortcut | This edits the text box; it is not file history |
| Unsaved tag-panel or Extended tags changes | **Edit → Discard pending changes**, or deliberate correction | Discard applies to selected pending edits; it cannot recover an earlier saved file |
| A recorded Save, rename, tag removal or action | **Edit → Undo …** | Read its label, file count and any history notice |
| The last successful Undo | **Edit → Redo …**, when available | A new recorded change normally clears Redo |
| A file sent to the system trash | The OS trash's restore command | This is outside the tag Undo journal |
| An external program's edit, permanent deletion or older session | Your independent backup/recovery process | Do not assume a TagGroove step can restore it |

The saved-format lists called **Manage history…** are reusable text histories.
Clearing one does not undo file edits. A
[configuration backup](../customize/configuration.md) also does not include
session Undo snapshots or your audio files.

## Practice Undo and Redo on one saved title

Use a copied ordinary local audio file, initially saved with `TITLE` =
`Paper Boats`. Keep other file-changing applications closed for this exercise.

1. Select the practice file and change its title to `Paper Boats - Practice`.
   It is pending until **Save tags** writes it.
2. Choose **Save tags** and check that the save succeeded. Open **Edit** and
   read **Undo Save tags (1 file)**.
3. Finish any active text edit and focus the file list. Choose that Undo
   menu item, or use ++cmd+z++ / ++ctrl+z++.
4. Check the operation result and the restored title, `Paper Boats`.
5. Read **Redo Save tags (1 file)** and choose it, or use
   ++cmd+y++ / ++ctrl+y++. The saved title becomes
   `Paper Boats - Practice` again.

The count describes audio-file records in that history step. A step can cover
several selected files, and chapter rows can refer to one underlying file.
Do not infer one independent Undo step for each row or button click in a
multi-file operation.

Saving a different title with the same number of characters still creates an
Undo step when the save keeps the file size and modification date unchanged.
Undo restores the previous tags; Redo reapplies the save, subject to normal
history limits. A request that writes nothing adds no step and keeps an
existing Redo step.

This also applies when a recorded action renames a file or moves its folder
before saving its tags. Undo can restore the previous location and tags;
Redo reapplies the recorded location and saved tags together. This concerns
recorded converter/action operations, rather than the separate **Move files
to…** command.

## Resolve pending edits separately

Suppose the title above was saved successfully, then you typed another title
without saving. Undo can restore the recorded disk state while that newer
pending title remains in the application. The pending display is not proof
that Undo failed. Saving it afterward can overwrite the restored result.

Before Undo, decide what to do with pending changes:

- Save only those you genuinely intend to keep. This can create a newer
  history step, so read the menu again afterward.
- Discard unwanted pending changes deliberately for the intended selection.
- If uncertain, record the intended values separately and inspect the current
  saved file before choosing either route.

Undo does not restore list membership. Files already removed from the list
stay out even if a history operation restores their disk contents. Reopen
those files to inspect them. Conversely, changing the selection does not
restrict an existing history step to only that new selection.

Text inputs retain native typing undo. If a shortcut appears to change the
field text instead of the file, finish/cancel the text edit and use the
explicit Edit-menu item with the file list focused.

## What the journal records

Supported local Save operations, tag removal, embedded-cover changes,
metadata converters, file/folder renames performed by converters/actions,
and the built-in tag utilities use the journal. Action groups can combine
several effects into one labeled step. Its label describes the operation;
it is not a list of every field changed.

For recorded file writes, restoration puts back the captured file state. It
does not recreate removed metadata by writing whatever tag formats are now
checked in Options. The read policy can still affect which restored values
are displayed. This differs from assuming that only the currently displayed
tag block was retained. Check
[tag-block reading and writing](../concepts/tag-formats.md) when files contain
several blocks with different values.

Some supported operations also record explicitly captured output images,
including action **Export Cover** and supported source-import cover writes.
Their existing image can be restored, or their newly created image removed,
when that recorded step can replay. This is a specific side-file capture
path, not a promise that every external output belongs to file history.
Ordinary text/CSV exports, playlists, arbitrary action export output,
configuration files and changes made by external tools need their own
backups. Check the [action's output scope](../actions/export.md) before
assuming an export will disappear on Undo.

File-list removal, desktop Copy/Move/Delete commands and default playback
are not recorded tag-Undo operations. Undo of an older tag save is not a
reversal of a later unrecorded transfer or deletion.

## Handle partial errors and changed files

A recorded batch can restore some files while reporting that others could
not be undone. The journal checks recorded filesystem state before replay
and can refuse a file changed outside TagGroove, a missing file, an occupied
old rename destination or a failed snapshot/restore. Do not bypass an error
by deleting the file at the destination.

Read the complete result, not just **Undid …**. A refused file is not
necessarily available for another attempt through the same history step;
Redo can retain only the successfully restored part. Preserve both versions
and use your backups if the history no longer offers the required state.
These guards do not establish universal detection of every external change,
filesystem alias or concurrent writer. Avoid editing the same files in two
applications during a recovery attempt.

On Windows, history also checks that a file has not been replaced by another
file with the same size and modification date. If it cannot obtain the
information needed for a safe snapshot, the operation can succeed without
being recorded for Undo; read the **cannot be undone** notice.

On the tested macOS filesystem, an external in-place audio edit that restores
the recorded file size and modification date passes the current change checks.
Undo or Redo can then replace that edit with a whole-file snapshot without an
error. These operations restore audio as well as tags; do not use them to merge
an external program's edits. Preserve an independent copy of those edits before
using an older TagGroove history step. Redo is not a guaranteed recovery route
for every external change.

A failed or canceled batch is not automatically rolled back. Completed writes
can remain, while files never processed can retain pending edits. Inspect
per-file errors and the current Undo label. Undo is a subsequent recovery
action, not a substitute for checking partial completion.

## Session and storage limits

The current defaults retain at most 50 recorded operations and 2 GiB of
snapshot data; older steps can be removed as limits are reached. These are
retained-storage limits, not a fixed limit on the sum of the audio files'
lengths. Some staged metadata writes share unchanged file storage, while
other rewrites can require a whole-file snapshot. A large operation can
therefore be unavailable for Undo even when a small edit was available.

On Unix-family platforms, snapshot admission also checks that at least
2 GiB would remain free on the journal volume. The inspected implementation
does not apply the same free-space query on every platform. Do not fill a
volume on the assumption that history will always reserve space for you.

Snapshot/storage failure normally does not stop the requested file operation.
TagGroove can report **cannot be undone** while the operation itself succeeds,
or record only part of a batch. The previous Undo label is not proof that
the latest operation has a recovery copy. Read notices immediately and keep
an independent backup for changes that must survive quitting.

History is scoped to the running application session. Restarting does not
load the old stack; stale session snapshots are cleaned separately. Do not
browse or delete the live undo cache to manage history, and do not treat it
as a durable backup archive. Provider documents use separate access and
recovery mechanisms and are not covered by this desktop local-file journal.

## Redo is different from Repeat

Redo has priority on the shared menu item and shortcut while an undone step
is available. When there is no Redo step, TagGroove can offer **Repeat** for
the available last action/group execution.

- **Redo** replays a recorded result from the history stack.
- **Repeat** runs the remembered action/group again against the current
  applicable selection. Expressions, source values and side outputs can be
  evaluated again; the outcome need not equal the earlier run.

Read the current label every time. Do not press ++cmd+y++ / ++ctrl+y++
expecting Redo after changing other files: it may now mean a new action run.
Selection, source access and pending-edit rules still apply to Repeat.

## Related

- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Copy, move and delete files](file-operations.md)
- [Built-in utilities](utilities.md)
- [Actions and action groups](../actions/index.md)
- [Troubleshooting](../help/troubleshooting.md)

<!-- verified: src/edit-history.ts; src/utils-ui.ts:redoOrRepeat; src/main.ts:replayHistory, run, operationReason; src-tauri/src/history.rs; crates/tag-core/src/history.rs; existing focused history fixtures. Session/local filesystem evidence, not native UI or universal external-change detection. -->
