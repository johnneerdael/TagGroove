# Undo and history

TagGroove's desktop history records supported changes to local audio files
and keeps eligible completed operations across restarts. It is separate from pending tag edits, text-box undo and remembered
format strings. Read the operation name in the Edit menu before restoring
anything: history follows the recorded operations, not just the
files you have selected now.

## Choose the right recovery route

| State to recover | Use | What to keep in mind |
| --- | --- | --- |
| Text just typed inside an input | That input's normal undo shortcut | This edits the text box; it is not file history |
| Unsaved tag-panel or Extended tags changes | **Edit → Discard pending changes**, or deliberate correction | Discard applies to selected pending edits; it cannot recover an earlier saved file |
| A recorded Save, rename, tag removal or action | **Edit → Undo …** | Read its label, file count and any history notice |
| The last successful Undo | **Edit → Redo …**, when available | A new recorded change normally clears Redo |
| A file sent to the system trash | The OS trash's restore command | This is outside the tag Undo journal |
| A completed recorded operation from an earlier launch | **Edit → Undo …** or **Redo …**, when retained | Restarting reopens eligible history; current files and folders must still match |
| A save or replay interrupted by application/process exit | **Edit → Export recovery copies…**, when available | Export copies for inspection; reopening does not automatically undo, redo or resume partial work |
| An external program's edit or permanent deletion | Your independent backup/recovery process | Do not assume a TagGroove step can restore it |

The saved-format lists called **Manage history…** are reusable text histories.
Clearing one does not undo file edits. A
[configuration backup](../customize/configuration.md) also does not include
the Undo store, its recovery copies or your audio files.

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
when that recorded step can replay. Undo can remove empty output folders that
the operation created; Redo recreates them. Eligible repeated Undo/Redo remains
available across restarts. Replacing a folder outside TagGroove can still make
the operation unavailable. This is a specific side-file capture
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

History also checks the file's complete contents. An outside edit can therefore
be refused even when it leaves the file size and modification date unchanged.
This applies to Undo and Redo, including a recorded rename. The journal checks
its recovery snapshots too: a damaged snapshot is refused instead of restored.
If a file changes after capture but before a staged Save, that Save can fail;
if it changes after Save but before the history step is recorded, the save can
remain without an Undo step. Read both the save result and any history notice.

Checking complete contents takes time proportional to file size. Large files
can make Save, Undo and Redo slower, even when only a small tag changed. The
check uses a bounded buffer rather than loading another whole audio file into
memory. The normal history size and free-space limits still apply.

These operations restore audio as well as tags; they do not merge another
program's edits. Preserve an independent copy before attempting recovery, and
keep other file-changing applications closed. A final check and file replacement
are separate operations, so these safeguards cannot exclude every concurrent
writer. Redo is not a guaranteed recovery route for an external edit.

A failed or canceled batch is not automatically rolled back. Completed writes
can remain, while files never processed can retain pending edits. Inspect
per-file errors and the current Undo label. Undo is a subsequent recovery
action, not a substitute for checking partial completion.

## Restart and storage limits

Completed eligible Undo and Redo steps survive a normal close and restart.
The current defaults retain at most 50 operations and 2 GiB of snapshot data,
shared by Undo, Redo and retained interruption recovery. Older recovery records
are removed first when new history needs space; older Undo/Redo steps can also
be removed as limits are reached. Read any notice about removed recovery copies. These are
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
the latest operation has a recovery copy. Failure to start the history
journal also leaves the requested operation's result unchanged and reports
that it **cannot be undone**. That operation is not recorded. Merely checking
history status or initializing the journal again does not clear the notice;
a later successfully recorded operation clears it. Read notices immediately
and keep an independent backup for changes that must survive cache loss or history limits.

Only one process can write a particular history store at a time. Another
instance can report unavailable history while its requested file operation
still succeeds. Once the instance using that history closes, a subsequent
session can use it again. Operations that ran without a snapshot do not gain
Undo afterward. Do not browse, move or delete the live undo cache to manage
history. Malformed indexes, replaced store entries and changed snapshots are
refused; an unreadable cache is not silently treated as safe history. Provider
documents use separate access and recovery mechanisms and are not covered by
this desktop local-file journal.

## Recover after an interrupted operation

An interrupted Save or Undo/Redo may have changed only part of a batch. On the
next launch, TagGroove leaves the current files as they are. Ambiguous old
steps are kept as separate recovery records when their verified copies fit
the history limits. They are not offered as automatic Undo/Redo. New file
operations can create fresh history.

The **Recovery copies available** notice remains above the file list while
recovery data is retained, including after a later successful Save clears an
ordinary history notice. Export promptly: continuing to make changes can
evict older recovery records. An interrupted capture's final storage cost is
unknown, so admission on reopening conservatively charges its whole preimage.
A very large interrupted operation can therefore exceed the limit even when
an ordinary completed metadata save would have fitted.

1. Stop other applications that may change the affected audio files. Preserve
   independent copies of the current partial results before deciding which
   version to keep.
2. Choose **Export recovery copies…** in the notice or **Edit** menu. Select an
   existing destination folder with enough free space. No file selection is
   needed, and pending tag drafts remain pending. Cancel leaves history intact.
3. Find the newly created **TagGroove recovery …** subfolder at the location
   reported in the status bar. Repeating export creates another subfolder; it
   does not overwrite earlier exports or consume the retained recovery data.
4. Read `manifest.json` and inspect the numbered copies. Each available copy
   names its operation, captured source path and **before** or **after** side.
   A captured operation normally has its original preimages; an interrupted
   replay can include both sides. Do not assume every partially produced file
   has a captured after-image.
5. Compare copies with the current files, then restore a chosen version using
   your normal file-management process. Preserve the current version first.
   Reopen the audio in TagGroove to inspect the restored tags and artwork.

The manifest stores exact native path units in addition to display text.
Folder moves are listed in their recorded order. A side-output entry's `epoch`
is the number of folder moves already completed when its path was captured:
`0` means before any recorded move, `1` means after the first, and so on. This
keeps outputs in a recreated old folder distinct from outputs carried into a
moved folder. These move lists/epochs describe completed or replayed operations.
For an interrupted capture, paths are the absolute locations at which each
preimage was captured; its capture-relative epoch is `0`, and any unfinished
folder-move sequence is not reconstructed. A missing side-file preimage is recorded as an absent state,
not as an instruction to delete a current file.

Links are exported as literal target data and manifest metadata, not recreated
as links. A relative or dangling target can be meaningful only in its original
folder context. Inspect it before restoring any link manually. The export
command does not write over current audio, rename folders or replay operations.

Export verifies complete snapshot contents. A damaged copy, changed destination
or insufficient free-space reserve can fail the export. Read the error before
using any partial output folder; a complete export includes a valid manifest.
The Unix-family destination check includes the normal 2 GiB reserve and space
for full copies, even when the cache used shared storage. This history is a
bounded recovery aid, not a durable backup archive. Application-cache removal
or storage failure can still lose it.

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

<!-- verified: src/edit-history.ts; src/utils-ui.ts:redoOrRepeat; src/main.ts:replayHistory, run, operationReason; src-tauri/src/history.rs; crates/tag-core/src/history.rs; existing focused history fixtures. Local filesystem and process-exit evidence; native UI, other-OS and hardware power-loss qualification remain separate. -->
