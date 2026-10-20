# The file list

The file list is the workspace's ordered set of loaded audio rows. It shows
field values and technical information, lets you select the scope of an
operation, and supplies the processing order for tasks such as numbering
and positional text import. The order on screen is separate from filenames
and track tags on disk.

## Reopen the same list

In the local desktop app, the loaded list and its manual order are remembered
across restarts. Column sorting and the original playlist/source order remain
separate choices. Restart reads the saved files again; it does not restore
unsaved tag or artwork edits. Clearing the list also clears its remembered
membership. Selection and filter details are captured with list changes and
again when closing normally.

Explicit command-line/file-open requests take precedence. Requests arriving
while a remembered list is being restored are applied afterward, in arrival
order. The startup-directory choice is the fallback when no remembered list
exists. Provider-document grants and recovery copies use their existing
separate workflow.

If a saved row is unavailable, restoration reports the failure without
substituting its containing audio file or publishing a partial list. Open
files or a folder to start a new list. Corrupt state, storage refusal and an
oversized restart snapshot are reported without changing audio. Save very
large lists as playlists; browser storage quotas can be smaller than the
16 MiB restart snapshot limit.

## Read the list and selection information

Each row represents a loaded file or a supported chapter/subsong row.
Columns can show editable fields, filenames or measured properties. A
column's label and displayed expression are configurable, so a column named
**Artist** is not proof of its actual configured field.

The information line summarizes selected-file duration/size, listed versus
loaded totals, and the active file's available codec, bitrate, sample rate
and channels. For example, **3 of 8 files** means the current view shows
three rows from eight loaded files; it does not mean five files were deleted.
An unavailable technical property is not supplied by guessing from the suffix.

A filter changes what is visible. It does not remove files from disk or
rewrite tags. After changing a filter, explicitly choose the intended
selection and inspect the selected count before running a batch command.
Use [filtering](filter.md) to learn queries and their application timing.

## Select the intended files

Click a row to select it alone. Use Command-click on macOS or Ctrl-click
on Windows/Linux to toggle individual rows. Shift-click selects the range
from the selection anchor to the clicked row. The row checkbox also toggles
its selection. With a row focused, arrow keys, Home/End and Page Up/Down
move through the visible list; modifiers can extend the selection.

| Task | Shortcut | Effect |
| --- | --- | --- |
| Select all | ++cmd+a++ / ++ctrl+a++ | Select the visible file-list rows when focus is outside a text editor. |
| Invert selection | ++cmd+shift+a++ / ++ctrl+shift+a++ | Select visible rows that were not selected. |
| Unselect all | ++cmd+u++ / ++ctrl+u++ | Clear the selection. |
| Select next file | ++cmd+n++ / ++ctrl+n++ | Select the row after the last selected row, or the first row when nothing is selected. At the end it stays on the last row. |
| Select similar files | ++cmd+g++ / ++ctrl+g++ | Select visible rows with the same current sort-value key as the active file. |
| Select next similar files | ++cmd+shift+g++ / ++ctrl+shift+g++ | Find the next different key after the last selected row, then select visible rows with that key. |

Similarity uses all active sort criteria, ignoring letter case. For an
album-wide selection, sort only by Album first. If Track is a secondary
criterion, the key includes Track too, so it does not group every track
merely because ALBUM is equal. With no active sort, filename supplies the
comparison key.

Right-clicking an unselected row first selects that row alone. Right-clicking
a selected row preserves the multi-file selection. The
[row context menu](../reference/menus/context-menus.md) can therefore act on
several files even though the pointer is over only one.

## Sort with one or several columns

Click a column header to sort ascending by that column. Click the same
primary header again to reverse it. Shift-click another header to add a
secondary criterion; shift-clicking an existing criterion reverses that
criterion without replacing the rest. The header arrows and priority
numbers show which rules apply.

For an artist/album collection:

1. Click **Artist**.
2. Shift-click **Album**.
3. Shift-click **Track**.
4. Inspect a few boundaries between artists, albums and discs.

A configured numeric column compares the leading number, so tracks 2 and
10 can sort numerically. A custom column can instead have a separate sort
format. Equal keys retain their prior order. Inspect mixed/empty values and
numbered disc groups; sorting cannot infer a release's intended track order.

Choose **View → Unsorted (load and playlist order)** to remove active sort
criteria for this view. This returns to the underlying workspace order, which
may already have been manually rearranged. It does not reconstruct an earlier
order after shuffling or manual moves. For loaded playlist sources, choose
**View → Sort by → Playlist order** to recover their remembered source rank.
These session choices do not erase your saved column-sort preferences.

## Move rows into a manual order

Select rows, focus the file list, then use:

| Key | Manual move |
| --- | --- |
| ++alt+up++ | One row earlier |
| ++alt+down++ | One row later |
| ++alt+page-up++ | Gather at the beginning |
| ++alt+page-down++ | Gather at the end |

Alt is ⌥ Option on macOS. Alt-dragging selected rows shows an insertion line
and moves the rows to that position. Dragging an unselected row first
selects it alone. Selected rows retain their relative order; an individual
row already at a boundary cannot move beyond it.

For this example, start again with `01.mp3, 02.mp3, 03.mp3, 04.mp3` before
each move and select `02.mp3` plus `04.mp3`:

<!-- manual-file-order-examples:start -->
| Move | Resulting file-list order |
| --- | --- |
| `up` | `02.mp3, 01.mp3, 04.mp3, 03.mp3` |
| `down` | `01.mp3, 03.mp3, 02.mp3, 04.mp3` |
| `top` | `02.mp3, 04.mp3, 01.mp3, 03.mp3` |
| `bottom` | `01.mp3, 03.mp3, 02.mp3, 04.mp3` |
<!-- manual-file-order-examples:end -->

Making a real manual move in a sorted view clears its active sort and
uses the displayed order as the starting point. With a filter, visible
rows are rearranged within the positions held by those rows; hidden rows
keep their places. No audio file is moved to another disk folder.

Click the file-icon column header to shuffle the shown rows. Shuffling
also makes a manual order, and subsequent numbering/playlists use that
order. Keep a playlist or other deliberate order record before experimenting
when the sequence must be restored later. Manual order lasts while the
files stay listed; it does not write track tags automatically.

Refresh keeps the relative order of surviving exact rows and appends newly
found rows. Removed rows disappear; a changed chapter identity counts as a new
row. To see an externally reordered playlist's new source sequence, choose
**Playlist order** explicitly. Refresh does not erase your saved column sort
or switch the chosen order mode.

## Edit a cell

Choose **Edit → Edit in file list** or ++f2++. This starts the first selected
file's editable filename column when one exists, otherwise its first
editable column. It does not bulk-fill every selected row.

To edit a particular tag cell, select one file, then make a separate delayed
second click on that editable cell. A fast double-click runs the configured
default external tool; **No action** leaves it without an action. Open
**Edit → Extended tags…** for the additional-field editor. Technical/read-only
columns cannot be made writable merely by clicking them.

| Key in the cell editor | Effect |
| --- | --- |
| Enter | Choose an explicitly highlighted matching suggestion first. A later Enter accepts the cell value; the General option can advance to the next file in the same column. |
| Escape | Close matching suggestions first; a later Escape cancels the current cell edit. |
| Tab | Accept and move to the next editable cell, wrapping to the next row. |
| Shift+Tab | Accept and move backward through editable cells/rows. |

With **Auto-completion at selection boxes** enabled in General options,
metadata cells show matching custom/loaded values while retaining the first
inline completion. Up/Down highlights a choice and Enter inserts it without
committing the cell or saving tags. Tab commits the current editor text using
its normal navigation policy, without accepting a highlighted alternative.
Escape closes matching suggestions before a later Escape cancels the cell.
Whole multiline suggestions keep their line breaks in the cell editor and
remain pending until the cell is committed. Explicitly choosing a suggestion
keeps its whole stored value, including leading/trailing spaces, line breaks
and literal two-backslash text. Typing a value still uses the usual trimming
and two-backslash separator policy. Enter and Tab still finish a File
List cell; they do not insert newlines here. For ordinary Enter-to-newline
editing, use a multiline tag-panel field or Extended Tags. Filename and
technical controls do not offer metadata suggestions.

Ordinary field edits become pending changes. Save tags explicitly, or use
the configured save-on-selection-change behavior, when they must reach
disk. For multiple separate values or multiline text,
[Extended tags](extended-tags.md) has dedicated controls.

A filename cell is different: accepting it requests an immediate filesystem
rename. The text is literal: `%title%`, `$upper(x)` and apostrophes are kept
as entered. Editing a filename does not automatically change TITLE.

To move one file into a folder, enter `Album/Track.mp3`; on Windows,
`Album\Track.mp3` works too. A relative destination starts in that file's
current folder. Use `../Track.mp3` to move one folder up. Paths are resolved
before the required destination folders are created. On macOS and Linux,
use `/` for folders: a backslash remains part of the filename. An absolute
destination uses the path entered. Include a filename after the last folder
separator; `Album/`, `Album/.` and `Album/..` are refused.

The original audio extension is retained. If the entered suffix already
matches it, including a different letter case, it is used once. Otherwise the
audio extension is appended: `Track.flac` entered for an MP3 file becomes
`Track.flac.mp3`. The audio format stays unchanged. Both filename columns
use this rule; the stem-only column simply starts without the extension.

An existing destination is refused. Inspect the operation result and use
**Edit → Undo** to reverse a recorded rename; another **Save tags** is not
needed. Pending tag edits follow the renamed file. When moving outside a
loaded folder, **Keep moved files** controls whether its row stays listed.
Inline rename moves only the audio file. For a reviewed batch layout or the
opt-in companion-file workflow, use
[Tag → Filename](../converters/tag-to-filename.md) and
[associated files](associated-files.md).

## Show, size and arrange columns

Right-click a column header to show/hide columns or choose column
customization. Keep at least one column visible. Drag column headers to
rearrange them, or drag their dividers to set widths. A hidden column's
metadata is still present in files.

**View → Auto-size columns** or Command+numeric-keypad + / Ctrl+numeric-keypad + fits
columns to measured contents within width limits. The shortcut requires
the numeric keypad plus key. Very long lists are sampled from the top, so
a later unusually long value can still need manual widening.
[View menu](../reference/menus/view.md) describes the column controls.

## Remove rows, refresh or delete files

- **Remove from list** (Delete; the Mac Backspace/delete key is also supported)
  removes selected rows from the workspace. The files remain on disk. Pending
  edits for removed rows are discarded after the configured confirmation.
- **File → Clear list** removes all loaded rows and their workspace state.
- **View → Refresh** (++f5++) rescans the loaded roots under the current
  subfolder preference and rereads loaded source playlists. Review its
  pending-change warning before proceeding. A fatal read, cancellation or
  stale result keeps the previous list and drafts; successful reads can
  change membership while keeping the display-order choice.
- **Reload tags** rereads selected files' tags; it is a different task from
  rebuilding the loaded file list.
- **Delete files…** and **Delete files permanently…** are actual filesystem
  operations. Read their confirmations; they are not substitutes for hiding
  unwanted rows from the current batch.

Check the [Edit menu](../reference/menus/edit.md) for deletion and transfer
commands, and keep their scope separate from the filter and selection.

Refresh retains the folder path you opened, including a symbolic link. If that
link now points to a different folder, a successful Refresh reads its current
target. When TagGroove itself renames an individually loaded file, it also
updates that file's Refresh source and displayed location—even if you opened
it through an alias. The saved workspace retains that updated location.
Renaming the file outside TagGroove is different: Refresh does not search for
its new name. Open the renamed file or its containing folder again.

## Gotchas

- Arrange the list before numbering or positional text import. A sorted
  preview is not evidence that tags on disk already contain those numbers.
- Moving rows does not create an undoable tag edit or restore an earlier
  shuffle through the **Unsorted** item. **Playlist order** can restore the
  remembered source rank of currently listed playlist members.
- Text focus changes shortcut meaning. Ctrl/Command+A in an editor selects
  text, not every loaded file.
- A rendered column can use an expression, while its editable **Field**
  targets a particular tag name. Inspect the column definition when a
  displayed value and its editor seem different.
- Save-on-moving-selection and tag-panel defaults can create or write
  pending edits during navigation; inspect their Options before assuming
  selection is only a visual action.

## Related

- [Filtering](filter.md)
- [Extended tags](extended-tags.md)
- [Working with covers](covers.md)
- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
- [View menu](../reference/menus/view.md)

<!-- Source baseline main4e4d5fc: main.ts select/listOperation/ordered/sortValueOf/reorderFiles/shuffleFiles/commitCell/renameFromList; list-commands.ts; list-behaviour.ts; columns.ts sortClick/sortRows/editKey; cell-editor.ts. Current UI/source contracts, not installed native runtime evidence. Canonical manage-list is history, covered by context-menus; it is not used as a source for file ordering. -->
