# The file list

The file list is the workspace's ordered set of loaded audio rows. It shows
field values and technical information, lets you select the scope of an
operation, and supplies the processing order for tasks such as numbering
and positional text import. The order on screen is separate from filenames
and track tags on disk.

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
criteria. This returns to the underlying workspace order, which may already
have been manually rearranged. It does not reconstruct an earlier order
after shuffling or manual moves.

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

## Edit a cell

Choose **Edit → Edit in file list** or ++f2++. This starts the first selected
file's editable filename column when one exists, otherwise its first
editable column. It does not bulk-fill every selected row.

To edit a particular tag cell, select one file, then make a separate delayed
second click on that editable cell. A fast double-click normally opens
Extended tags, or a configured default external tool. Technical/read-only
columns cannot be made writable merely by clicking them.

| Key in the cell editor | Effect |
| --- | --- |
| Enter | Accept the value; the General option can advance to the next file in the same column. |
| Escape | Cancel the current cell edit. |
| Tab | Accept and move to the next editable cell, wrapping to the next row. |
| Shift+Tab | Accept and move backward through editable cells/rows. |

Ordinary field edits become pending changes. Save tags explicitly, or use
the configured save-on-selection-change behavior, when they must reach
disk. For multiple separate values or multiline text,
[Extended tags](extended-tags.md) has dedicated controls.

A filename cell is different: accepting it requests an immediate filesystem
rename. Keep the existing audio extension in a filename-with-extension
column. Path separators are not accepted there; use
[Tag → Filename](../converters/tag-to-filename.md) for a reviewed folder layout.
A title field edit does not rename the file, and a filename edit does not
automatically change TITLE.

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
  subfolder preference. Review its pending-change warning before proceeding.
- **Reload tags** rereads selected files' tags; it is a different task from
  rebuilding the loaded file list.
- **Delete files…** and **Delete files permanently…** are actual filesystem
  operations. Read their confirmations; they are not substitutes for hiding
  unwanted rows from the current batch.

Check the [Edit menu](../reference/menus/edit.md) for deletion and transfer
commands, and keep their scope separate from the filter and selection.

## Gotchas

- Arrange the list before numbering or positional text import. A sorted
  preview is not evidence that tags on disk already contain those numbers.
- Moving rows does not create an undoable tag edit or restore an earlier
  shuffle through the **Unsorted** item.
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
