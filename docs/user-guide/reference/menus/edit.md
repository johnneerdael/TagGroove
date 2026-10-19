# Edit menu

**Edit** contains tag clipboard, history, selection and file operations.
First identify what has focus. Copying selected text is different from copying
the tags of selected files; deleting a file is different from removing its row.

## History and tag clipboard

| Item | Result |
| --- | --- |
| **Undo** | Restore the last recorded operation; its label shows the operation/file count when available |
| **Redo** / **Repeat** | Redo an undone operation, or repeat the available last action/group; the current label identifies which |
| **Export recovery copies…** | Export retained interruption recovery into a new folder with a manifest; keep current files and history unchanged |
| **Cut tags** | Copy tag/cover data and stage its removal from selected files |
| **Copy tags** | Copy selected tag values and available artwork data |
| **Copy as Text…** | Evaluate a format string for the selected files and copy a textual representation |
| **Paste tags** | Stage clipboard tag/cover data on selected targets |
| **Discard pending changes** | Remove unsaved changes without reading another file or applying panel defaults |

Text-entry fields keep their normal text clipboard and undo behavior. Outside
text entry, the app's tag clipboard can copy one file's tags to several targets
or match multiple source entries to targets in order. Check the selection and
ordering before pasting. Pasted artwork and cut-tag removals remain pending
until **Save tags**; copying alone does not write anything.

For **Copy as Text…**, `%artist% - %title%` makes a readable artist/title line
for each selected track. This is calculated text, not a new audio tag. Functions
and placeholders follow the [format-string reference](../format-strings.md).

History concerns retained recorded operations, including eligible steps reopened
from an earlier desktop launch. See the [Undo and recovery guide](../../guides/undo-history.md)
for restart behavior, storage limits and interruption recovery.
Check its label and any errors; do not assume every external change, file
transfer, export or deletion can be reversed with tag-edit undo. If the menu
says **Repeat**, it means another action execution, not restoration of a saved
snapshot. Pending edits and available operations still constrain it.

## Selection and finding

| Item | Which files it selects |
| --- | --- |
| **Select all** | Every row in the current visible list |
| **Invert selection** | Visible rows currently unselected, while deselecting selected rows |
| **Unselect all** | No rows |
| **Select next file** | The row after the last selected row, clamping at the end |
| **Select similar files** | Rows whose active sort values match the active file |
| **Select next similar files** | The next different group of active sort values |
| **Find…** | Open the file-list search dialog |

A filter limits what is visible. **Select all** after filtering therefore
selects that visible result, not every file elsewhere on disk. Hiding a row
with a filter does not automatically unselect it; check the selected count
before a write. Similar-file
selection uses sort criteria, not audio similarity or a duplicate-audio test.

For example, sort by album, select a Harbour Lights track, then choose
**Select similar files** to select that album's matching sort group. Additional
sort criteria can narrow the match. Choose the sort deliberately before using
that result as the target of an action.

**Select next file** also works while editing a tag-panel field, keeping focus
in the panel. [Tags options](../options/tags.md) can save pending edits when
moving between selections. Changing selection alone is not otherwise a
universal save command.

## Names, transfers and deletion

| Item | Effect |
| --- | --- |
| **Edit in file list** | Start editing the first selected filename column, or another editable column when no filename column is available |
| **Copy files to…** | Choose a destination and copy selected audio files |
| **Move files to…** | Choose a destination and move selected audio files |
| **Remove from list** | Remove rows; keep files on disk |
| **Delete files…** | Move selected files to the system Recycle Bin/trash when supported |
| **Delete files permanently…** | Delete without using the system trash |
| **Manage covers…** | Inspect and edit artwork for selected files |

Read the deletion confirmation: **Remove from list** does not delete, while
permanent deletion does not provide a trash restore path. File errors can
leave only part of a requested copy, move or deletion completed; inspect the
reported outcome before repeating it. These operations require supported
local file access and are unavailable for provider selections where the
operation cannot be performed.

Editing a filename cell changes a path; editing a metadata cell changes a
field. Check the cell and the command's commit/preview behavior. For a batch
rename from tags, use [Tag → Filename](../../converters/tag-to-filename.md)
instead of editing each row manually.

## Keyboard and context

Primary+C/X/V, Z/Y and selection shortcuts follow the focus rules in
[keyboard shortcuts](../keyboard-shortcuts.md). F2 is **Edit in file list**
when the list has focus, but can rename an action group or edit a source-review
field in those dialogs. The cover area's context menu has cover-specific
commands; a tag clipboard operation can include artwork as well as text fields.
