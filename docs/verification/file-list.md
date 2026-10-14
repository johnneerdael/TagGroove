# File List parity: columns, inline editing and list commands

Scope: the Mp3tag reference pages `customization/file-list`, `getting-started/main-window`, `menus/view`, `menus/edit`, `find` and `keyboard`, limited to the File List.

## Implemented

### Configurable columns (`src/columns.ts`, `src-tauri/src/columns.rs`)
- Each column has a name, a value format string, an editable field placeholder (empty means read-only), an optional sort-by format string, a width, an alignment, a numeric flag and a visibility flag.
- Default set (visible): Filename, Title, Artist, Album, Track, Year, Genre, Length, Codec, Bitrate, Path. Hidden by default: Tag, Comment, Album Artist, Composer, Discnumber, Sample rate, Size, Last modified, Covers. Multi-value tag columns use `$meta_sep(field,' \\ ')`, so they show the same `\\` separator as the Tag Panel.
- The layout is stored in `<app config>/columns.json` through `load_columns` and `save_columns`. A missing file means the defaults are used. The backend validates the layout before saving: unique IDs, names present, value and sort-by templates compile, the field is either a tag placeholder or `%_filename%`/`%_filename_ext%`, widths are within 24–4000 px, at least one column is visible, and sort keys refer to existing columns.
- Utils-style load/save of a configuration: the Customize dialog has *Load configuration…* and *Save configuration…* (`import_columns` / `export_columns`). These read and write the OpenTag JSON layout.
- View → *Customize columns…* opens a dialog with New, Duplicate, Remove, Move up, Move down, Reset to defaults, visibility checkboxes, all column properties and a live value preview for the active file.
- Header: right-click opens a show/hide menu with a *Customize columns…* entry, and View lists every column as a toggle. Drag a header to reorder; drag a header edge to resize. Reorders and resizes are saved.
- Sorting: click a header to sort, click it again to reverse, and Shift+click to add or reverse a secondary criterion. Multi-criteria headers show the criterion position. Sorting uses the sort-by expression when one is set, otherwise the value. Numeric columns compare leading numbers. The sort is saved with the layout.
- Evaluation: values come from the Rust scripting engine through a single `evaluate_columns` command. Each request carries many files and many templates; every template is compiled once per request, and a failing cell is left empty with the error shown in the status bar. The input is the UI's merged fields (pending drafts included) plus file properties. As in the Tag Panel, Vorbis `TRACKNUMBER`/`DATE` answer `%track%`/`%year%`. `ColumnValueCache` keys each row on the identities of its entry, draft, override and document state, and batches up to 500 files per request. While a row is being re-evaluated after an edit, its old value stays on screen, so rows do not jump. A single-placeholder template is filled locally until the engine replies. Column evaluation does not take the file-operation lock.
- Actions, exports, filters and converters still get `columns` specs, now taken from the configured visible columns. Snapshot values are sent only when they are fresh; otherwise the backend evaluates the template itself.

### Inline cell editing (`src/cell-editor.ts`)
- F2 (or Edit → *Edit in file list*) edits the Filename column of the first selected file. If no filename column is visible, it edits the first editable column, as the docs describe. Clicking again, after a delay, on a cell of the already-selected file edits that cell; a double-click still opens Extended tags.
- Enter commits, Escape cancels, and focus loss commits. Tab and Shift+Tab move across editable cells and wrap to the next or previous file, selecting it.
- Tag field edits become pending drafts through `updateDraft`. Track and year use the same key mapping as the Tag Panel. Values are split on `\\` for multiple values. The cell shows as pending, and Ctrl+S saves the edit. An unchanged value creates no draft.
- Filename edits rename right away through `rename_files`, with the name passed as a quoted literal template. The extension must stay the same. Path separators are rejected, provider documents are refused, and pending drafts follow the renamed file.
- The editor floats inside the scrolling table, so virtual-row updates and full renders keep the typed text and caret.

### Selection and list commands (`src/list-commands.ts`, `src/find.ts`)
- Edit menu with the shortcuts from the keyboard reference (Cmd on macOS):

  | Command | Shortcut |
  | --- | --- |
  | Invert selection | Ctrl+Shift+A |
  | Unselect all | Ctrl+U |
  | Select next file | Ctrl+N |
  | Select similar files (by the active sort criteria) | Ctrl+G |
  | Select next similar files | Ctrl+Shift+G |
  | Find… | Ctrl+F |
  | Remove from list | Del |

- *Remove from list* only changes the list; files on disk are untouched. It asks first if any of the files have pending drafts, then selects the next file. File → *Clear list* empties the list.
- View → *Refresh* (F5) rescans every opened folder or file recursively. It asks before discarding drafts, drops files that are gone, keeps the selection where the files still exist, and reloads tags. F5 is always intercepted so the webview never reloads.
- F3 focuses the filter.
- Find is a non-modal window with Contains, Matches word, Matches content, Begins with, Ends with and Regular expression, plus a Case sensitive option. It searches every tag field value and the filename of the listed files. The results table (Filename / Field / Match) sorts on header click. Clicking a result selects that file in the list, and *Select all results* selects every matching file. Criteria history is kept for the session.

### Status bar
The selection-info bar shows:
- the selected count, with total length and size;
- the listed total, shown as "N of M files" while a filter is active, with total length and size;
- for a single selected file, its codec, bitrate, sample rate and channels.

## Tests
- Frontend: `tests/columns.test.mjs` (10 tests) and `tests/list-commands.test.mjs` (5 tests) cover:
  - defaults and editable-field rules;
  - normalizing and validating layouts;
  - toggle, move and resize;
  - sort clicks, numeric and stable multi-criteria sorting;
  - the Vorbis aliases and the quick-value fallback;
  - the value cache (batching, stale values kept while refreshing, re-evaluation, no endless retries for inactive templates, failure reporting);
  - shortcut mapping, the selection algorithms, status formatting, every Find strategy and result sorting, and Tab navigation.

  `tests/viewport-resize.test.mjs` only gains a `tableColumns` stub in its harness.
- Rust: five tests in `src-tauri/src/columns.rs` cover editable-field parsing, layout validation, the save/load/import round trip (including a minimal file and a corrupt file), batch evaluation through the real scripting engine with isolated failures, and a 1000-file batch.
- Results: `node --test tests/*.test.mjs` passes 116 tests (101 before). `cargo test --workspace` passes 284 tests (279 before). `npx tsc --noEmit` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- After merging `feat/core-parity` at b78b20d (tag panel, tag preferences, edit history, tag sources): 151 frontend and 345 Rust tests pass, and `tsc`, `clippy` and `fmt` are clean. The File List entries go into the existing View menu through `viewListItems`. Refresh follows the *subfolders* directory option.
- Build note: on this machine Homebrew's GNU `ar` comes first in `PATH`, and it produces a `libopentag_regex.a` that Apple `ld` rejects. Build with `AR=/usr/bin/ar` whenever `regex-compat` gets rebuilt.

## Performance (synthetic 1000-file list)
- Rust batch evaluation of 1000 files × 12 templates (the default visible columns plus sort templates): 30 ms in an unoptimized test build and 7 ms in release (`batch_evaluation_of_a_large_list_stays_fast`).
- In the browser, with a mocked Tauri bridge serving 1000 synthetic entries (from Vite dev in Chromium):
  - Open folder, then first rows and all sort values available: about 53 ms. Initial evaluation is two `evaluate_columns` batches of 500 files × 11 templates, about 340 KB of JSON each.
  - Re-sort on a header click, including the full workspace render: about 9 ms. Adding a secondary criterion: about 8 ms.
  - Scrolling 30 frames over the virtualized table: about 7.6 ms per frame.
  - Selecting a row (full render): about 7 ms.
- Sorting 1000 cached rows on two criteria averages well under 50 ms in the unit test bound (in practice a few ms).

## Known gaps vs Mp3tag
- Column configuration files use OpenTag's JSON layout, not Mp3tag's configuration format, and the editor is part of the Customize dialog rather than a Utils menu.
- Mp3tag's random-order click on the file-icon column and Ctrl+Num+ auto-sizing are not implemented. Alt+arrow manual reordering of files is out of scope.
- Inline filename edits cannot move a file to another folder (Mp3tag accepts `\` in the new name). Columns cannot use `$columnvalues()`/`$columnnames()` inside their own values (the engine reports a missing column context).
- Find uses JavaScript regular expressions, not the Boost-compatible engine in `regex-compat`. Its history is per session, not persisted, and it searches the filtered list.
- Refresh rescans the opened roots. A file added individually and later renamed is not found at its old root path, and the rescan reports it as an error.
- Cut/Copy/Paste of tags, Copy as Text, Delete, Copy/Move to directory, Undo/Redo, and the Tag Panel toggle (Ctrl+Q) are out of this batch.
- Global shortcuts are ignored while a modal dialog is open (existing behaviour). The non-modal Find window is exempt.
