# Find text in the file list

Use **Edit → Find…** (++cmd+f++ / ++ctrl+f++) to list matching filenames and
tag values, then select the files behind those results. Find leaves the
file list's contents in place. To reduce the visible list, use
[Filter](filter.md) instead.

<!-- figure:find-dialog -->
<figure markdown>
![The Find dialog with a search box, a strategy list and a results table](../assets/screens/find-dialog-light.png#only-light)
![The Find dialog with a search box, a strategy list and a results table](../assets/screens/find-dialog-dark.png#only-dark)
<figcaption>Find lists every match and selects the files you pick.</figcaption>
</figure>
<!-- /figure -->

The search runs on the currently listed, visible files, including their
pending field values. A filename means its leaf name with extension, not
its whole folder path. Technical information fields are not part of this
tag-value search; use the filter's information placeholders for those.

## Find a title and select its file

1. Load copied practice files, wait for any active filter to finish, and
   choose **Edit → Find…**.
2. Enter `Pier`, leave **Strategy** at **Contains**, and leave **Case
   sensitive** off.
3. Choose **Find** or press Enter in the search input.
4. Look for the result whose **Field** is TITLE and whose **Match** is
   `North Pier`.
5. Click that row. The corresponding audio file becomes selected in the
   main window. Close Find when ready to edit it.

Find is a non-modal window: the main workspace can remain available while
results are shown. A result selects a file; it is not an editable tag value
or a replacement instruction. Check the main selection before applying a
batch action.

## Choose a matching strategy

Ordinary strategies treat the criteria as literal text. A dot or bracket
has special meaning only when **Regular expression** is chosen. The default
is **Contains**, with case ignored.

<!-- find-strategy-examples:start -->
| Strategy | Criteria | Value being searched | Result |
| --- | --- | --- | --- |
| **Contains** | `Pier` | `North Pier` | Match |
| **Matches word** | `Pier` | `North Pier` | Match |
| **Matches content** | `Pier` | `North Pier` | No match |
| **Begins with** | `North` | `North Pier` | Match |
| **Ends with** | `Pier` | `North Pier` | Match |
| **Regular expression** | `^North\s+Pier$` | `North Pier` | Match |
| **Matches word** | `Pier` | `North Pier-West` | Match |
| **Matches word** | `Pier` | `North Pier_West` | No match |
<!-- find-strategy-examples:end -->

**Matches word** requires boundaries around the whole criteria. Letters,
numbers and underscore count as word characters for this strategy; a
hyphen separates words. **Matches content** requires equality with one
complete field value or filename. For a filename, the extension is part
of that value, so `North Pier` is not the complete content of
`04 - North Pier.flac`.

Enable **Case sensitive** when `Pier` must differ from `pier`. Spaces at
the start/end of the criteria are literal search characters; an entirely
empty criteria reports **Enter text to find** rather than listing every file.

## Read match counts correctly

One file can have several result rows. A filename, TITLE and COMMENT can
all match the same criteria. One matching field value creates one row,
even if that value contains the searched text several times.

For example, use **Contains** with `boats`, ignoring case, on this practice
selection:

<!-- find-result-example:start -->
| Filename | TITLE | COMMENT |
| --- | --- | --- |
| `03 - Paper Boats.flac` | `Paper Boats` | *(absent)* |
| `04 - North Pier.flac` | `North Pier` | `Notes about boats` |
<!-- find-result-example:end -->

Expect **3 matches in 2 files**: the first filename, its TITLE, and the
second file's COMMENT. **Select all results** selects the two distinct
files, not three copies of a file. A second ARTIST value can likewise
supply its own result independently of the field's first value.

## Sort and navigate results

Click the **Filename**, **Field** or **Match** column header to sort those
result rows. Clicking the active header again reverses that sort. Result
sorting does not reorder the main file list or change tag values.

Click a row, or focus it and press Enter/Space, to select its file in the
main list. Up/Down moves through result rows and selects their files.
The current result control provides individual-row navigation or **Select
all results**; it does not provide a separate Command/Ctrl-click range
selection of arbitrary result rows. For a subset, close Find and adjust
the selected files in the main list.

The table displays at most the first 2,000 matching rows. The status reports
the full count, and **Select all results** considers all matching files,
including ones represented only by unshown rows. Refine the query when
every match must be inspected before acting.

## Refresh an outdated result set

The result table is a snapshot from the last **Find** operation. Edit tags,
change a filter or remove files, then run Find again to refresh it. Clicking
a result whose file is no longer visible reports that it is no longer
listed. **Select all results** also excludes files that are no longer in
the current visible list.

Selecting results changes the main selection. Configured tag-panel defaults
can stage values for a multi-file selection; inspect pending changes before
saving them. Running a search is not itself a metadata-replacement operation.

## Regular expressions use the shared Boost engine

Find uses the same Boost-compatible regex engine as actions, filters, format
strings and web-source parser commands. Enter the raw pattern without the
apostrophe quoting used inside a `$regexp(...)` format string. Use **Case
sensitive** to control case, or inline options such as `(?i)` to override it.
`\A` and `\z` match the complete value; `^` and `$` also recognize line breaks,
and `.` matches line breaks by default.

An invalid expression is shown as an error and clears the result table for
that run. Changing the criteria, strategy or case setting also clears previous
results; press **Find** to search again. If a size limit is reported, filter
the File List to a smaller set. See the
[regex reference](../scripting/regular-expressions.md#find-dialect) for worked
examples and limits. Find has no replacement box: use an action or converter
when matching should lead to an edit. Quoting and case defaults still differ
between those contexts.

## Reuse or remove criteria

The input's drop-down history remembers used criteria through the normal
helper-list mechanism. The **►** helper has **Remove from history** and
**Remove all from history…** for this input. Removing history does not
remove tags or files. Physical Control+Space opens the focused helper;
Shift+Delete removes the current remembered value.

Reopening an already open Find window selects its input for a new query.
After closing and opening a new window, check the strategy and case option:
the new dialog starts with Contains and case sensitivity off. A remembered
criteria string does not carry those options with it.

## Gotchas

- Check whether a filter has hidden files that should participate in the
  search. Find searches this view, not every file elsewhere in the library.
- The result's **Field** names metadata; the file list's configurable column
  labels do not rename those fields.
- Several matching rows can refer to one file, while one field can contain
  several separately searched values. Read both the match and file counts.
- Close Find before relying on workspace keyboard shortcuts whose routing
  changes while a dialog is open.

## Related

- [Filtering](filter.md)
- [The file list](file-list.md)
- [Find regular-expression dialect](../scripting/regular-expressions.md#find-dialect)
- [Context and helper menus](../reference/menus/context-menus.md)
- [Extended tags](extended-tags.md)

<!-- Source baseline maina9cd900; find.ts findMatcher/findInList/sortFindResults/FindUI and main.ts FindUI host. All canonical find strategies, case control, result navigation/sorting and history tasks are independently explained. Actual current limits (visible scope, individual/all selection and 2000 displayed rows) are explicit. No native/application launch or filesystem mutation was exercised for this guide. -->
