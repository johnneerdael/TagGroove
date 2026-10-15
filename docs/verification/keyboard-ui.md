# Remaining keyboard rows and main-window customization

Branch `feat/keyboard-ui`, based on `feat/core-parity` after the keyboard-extended batch (`docs/verification/keyboard-extended.md`). It closes the 13 unbound and 3 partial rows of the keyboard reference and the open Main window, Tag Panel, Extended Tags and File List customization rows of `docs/research/remaining-gaps.md`. Reference pages: `keyboard`, `getting-started/main-window`, `menus/view`, `customization/tag-panel`, `customization/file-list`, `customization/options/tags` and `extended-tags`.

Nothing was launched locally. The evidence is unit tests, `cargo test`, clippy and the headless native suite `tests/e2e/keyboard-ui.js` in the CI `native-e2e` job.

## Keyboard reference (97 rows)

Totals: **97 OK**, 0 partial, 0 missing. Before this batch: 81 OK (60 earlier plus 21 from keyboard-extended), 3 partial, 13 missing. This batch changes 16 rows (13 new bindings, 3 partial rows completed). The other 81 rows are listed in `docs/verification/keyboard-extended.md` and unchanged. The `ui_evidence` of the `shortcuts` section of `docs/feature-inventory.json` is updated for these rows. Acceptance stays OPEN, as the inventory's policy requires.

Ctrl means ⌘ on macOS throughout, as in the earlier batches. Ctrl+Tab, Ctrl+Space, Ctrl+H, Ctrl+M and Ctrl+Q keep the physical Control key.

| Scope | Key | Command | Before | Now |
|---|---|---|---|---|
| General | Ctrl+Shift+Del | Delete file permanently | Missing | `windowShortcut` → Edit → Delete files permanently (⌘⇧⌫ or ⌘⇧Del on macOS). The new `delete_files_permanently` command removes regular files without the trash. Confirmation follows Options → Messages, like Delete files, and the text says the files cannot be restored |
| General | Ctrl+Shift+P | Playlist (all files), one per subfolder | Partial (opened Generate playlists) | File → Playlist per folder (all files): every listed file, partitioned by `%_folderpath%`, written into each folder with the Options → Playlist file name (`PlaylistUI.createPerFolder`). Generate playlists keeps its menu item, without a shortcut. The Mp3tag docs disagree here: the keyboard page gives Ctrl+Shift+P to this command, while the File menu page lists it next to Generate Playlists. The keyboard page wins, and `tests/e2e/playlists.js` now opens the generator from the File menu |
| Tag Sources | Ctrl+I | Last used Tag Source | Partial (MusicBrainz or Discogs only) | Remembers every source, tags or covers only (`lastSource`). Tag Sources menu: "Last used: …" |
| Tag Sources | Ctrl+Shift+I | Last used Tag Source (Quick) | Partial (same as Ctrl+I) | Opens the last source and searches at once with the prefilled values (artist and album, or the disc ID for freedb), so the search step is skipped. The result list is still shown |
| Actions | Ctrl+A | Select all groups or actions | Missing | `actionListKey` → `markAll`. The group list and the group editor's action list are multi-selectable (⌘/Ctrl+click toggles, Shift+click selects a range) |
| Actions | Ctrl+Shift+A | Invert the selection | Missing | `markAll(…, invert)` |
| Tag Sources | Ctrl+A | Select all fields | Missing | Field rows are selectable (click the field name, ⌘/Ctrl+click, Shift+click) |
| Tag Sources | Ctrl+Shift+A | Invert the selection of fields | Missing | Inverts the selected field rows |
| Toolbar Customization | F4 | Edit selected item | Missing | View → Toolbars → Customize toolbar (`src/toolbar.ts`): renames the item |
| Toolbar Customization | Space | Show/hide selected item | Missing | `toolbarKey` toggle |
| Toolbar Customization | Ctrl+E | Show all items | Missing | `toolbarKey` show-all |
| Toolbar Customization | Ctrl+D | Hide all items | Missing | `toolbarKey` hide-all |
| Toolbar Customization | Alt+Up | Move item up | Missing | `moveToolbarItem` |
| Toolbar Customization | Alt+Down | Move item down | Missing | `moveToolbarItem` |
| Toolbar Customization | Alt+Page Up | Move item to the beginning | Missing | `moveToolbarItem` top |
| Toolbar Customization | Alt+Page Down | Move item to the end | Missing | `moveToolbarItem` bottom |

With multi-selection, the existing Actions keys act on every selected item: Space enables or disables all selected groups, Delete removes them, and Alt+↑/↓/Page Up/Page Down move them as a block, keeping their order (`moveMarked`). The Move up and Move down buttons do the same. In Tag Sources, Space enables or disables every selected field.

Alt+Enter (file properties) stays as keyboard-extended bound it. It shows the files in Finder, Explorer or the file manager, because the Windows properties sheet has no macOS or Linux equivalent.

## Main window

- **F3** hides and shows the filter (View → Hide/Show filter). A hidden filter does not filter, so every file is listed. Its text comes back and gets focus when it is shown again. The setting is remembered on this device. Ctrl+Tab skips the filter while it is hidden. Before, F3 only focused the filter, which was always shown.
- **Ctrl+Tab / Ctrl+Shift+Tab** were bound by keyboard-extended. The suite checks them again with the filter hidden.
- **View → Sort by** lists the shown columns with a check mark and arrow on the primary sort column. Choosing a column sorts by it, and choosing it again reverses the order. The submenu follows header clicks (`refreshList`).
- **Toolbar hints**: hovering or focusing a toolbar button shows its description in the status bar, plus the reason when the button is disabled. Leaving restores the previous text (`TOOLBAR_COMMANDS[…].hint`).
- **View → Toolbars**: *Main toolbar* shows or hides the toolbar. *Customize toolbar…* shows, hides, renames and reorders items and adds separators, with the keys above. 28 commands can go on the toolbar. The defaults are the previous eight buttons. The configuration is saved in Options storage (`pages.toolbar`).
  - Deviation: Mp3tag also has a *Custom toolbar* for its tools. OpenTag has no second toolbar, and tools are in the Tools menu and the File List context menu.
  - Deviation: the earlier flexible gap before Actions and Numbering is now a plain separator.
- **Drop vs. Ctrl+drop**: dropping files, folders or playlists replaces the File List (with the Options → Messages unsaved-changes question). Holding Ctrl (⌘ or Control on macOS) adds them instead. Native drops carry no key state, so the new `modifier_keys` command reads the held modifiers from the OS: `NSEvent.modifierFlags` on macOS, `GetAsyncKeyState` on Windows, and the GDK keymap on Linux, on the main thread. Keys seen by the focused window count too. The drop overlay says which key adds. Before, every drop added.

## Tag Panel

- **Right-click** on the tag panel opens its menu at the pointer: fields to show or hide, Display cover art, Customize, Field values, Default values, Save/Load configuration, Hide. Text fields keep the system edit menu, and the cover window keeps its own menu.
- **Automatic default values**: when the selection changes to several files, every visible field whose Default value is not `< keep >` is filled in as a pending edit, as documented ("the value that is automatically filled when selecting multiple files"). This happens once per selection, never over a field that already has pending edits, and only while the panel is shown. Files that already have the value stay clean (`automaticDefaults`). View → Default values in input fields still applies them on demand.
- **Directory field (`_DIRECTORY`)** is editable. Typing a folder path and pressing Enter makes it the working directory (Change directory, with the unsaved-changes question). Escape restores the shown folder. The ▾ list (loaded folders and Open folder…) is unchanged (`typedDirectory`).

## Extended Tags

- **Options → Tags → Save mode at extended tag dialog**: *Keep changes pending* (default, OpenTag's behaviour so far), *Ask*, *Ignore changes* or *Save changes*. It applies when << or >> leaves a file with changed fields. Ask shows Save / Don't save / Cancel inside the app.
- **Cut, Copy and Paste fields** from the field list's context menu (right-click, ⇧F10 or the context-menu key). Paste replaces the values of a listed field or adds the field. Copied fields also go to the system clipboard as `FIELD=value` lines. Fields whose values differ between the selected files cannot be copied.
- **Field Values auto-complete** and the **caption with the tag format and file name** were done in keyboard-extended. The suite there covers them.
- Deviation kept: OK (Keep changes) stages edits as pending changes instead of writing them, unless the save mode writes on navigation.

## File List

- **Random order**: the ⤮ button at the start of the first column header (above the row checkboxes, OpenTag's icon column) shuffles the listed files. Like Alt+drag, the result is a manual order: sorting is cleared, and playlists and numbering use it (`shuffleOrder`). It is hidden in the compact layout.
- **Mp3tag column import**: Customize columns → Load accepts a `columns.ini` the user chooses from their own Mp3tag configuration folder. OpenTag ships no Mp3tag file. `src-tauri/src/mp3tag_columns.rs` reads UTF-16 (BOM), UTF-8 or Windows-1252 text with one section per column. It reads the name, value, field, sort by, width, numeric flag, alignment (0 left, 1 right, 2 center), visibility and position, accepting the usual key spellings. A bare field name becomes its placeholder. Fields that OpenTag cannot edit make the column read-only. Columns whose format string OpenTag cannot compile are skipped, and the status bar lists them. Unknown keys and sections are ignored.
  - Limitation: Mp3tag does not document this file's layout, so the reader follows the visible Customize columns settings. Files that use other key names import only the keys it recognises.

## Tests

Unit tests (`node --test tests/*.test.mjs`, 259 passing):

- `tests/keyboard.test.mjs`: Ctrl+Shift+Del (⌘⇧⌫ on macOS, not while typing, Ctrl+Del unchanged), Ctrl+P / Ctrl+Shift+P, Ctrl+I / Ctrl+Shift+I with ⌘ on macOS. Ctrl+T still works from a field.
- `tests/action-ui.test.mjs`: Ctrl+A / Ctrl+Shift+A in `actionListKey`, `markAll`, `clickMark`, and `moveMarked` block moves including the list edges.
- `tests/keyboard-ui.test.mjs` (new): toolbar keys, defaults, normalisation, moves and separator collapsing; `lastSource`; `shuffleOrder` and `dropAdds`; the header shuffle button; F3; automatic defaults and typed directories; the field clipboard; `createPerFolder` requests (all files, `%_folderpath%`, base folder per playlist).
- `tests/file-ops.test.mjs`: permanent delete (confirmation text, the command, list removal) and `replaceWith` for plain drops.
- `tests/list-behaviour.test.mjs`: the Options → Tags save-mode control and its values.

Rust (`cargo test -p opentag`, 199 passed): `mp3tag_columns` (position order, settings, read-only technical fields, skipped columns, UTF-16, no columns) and `file_ops::permanent_delete_removes_regular_files_only`. `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --check` pass on macOS. CI runs clippy on Windows and Linux, which compile the other `modifier_keys` implementations.

Native suite `tests/e2e/keyboard-ui.js` with `tests/e2e/keyboard-ui-fixtures.sh` (a subfolder, a file to delete and a hand-written UTF-16 `columns.ini`), added to the CI suite list:

1. The menus show the new shortcuts, and the View menu has Sort by and Toolbars.
2. F3 hides the filter and lists every file. Ctrl+Tab goes panel → list. F3 shows the filter again with its text and focus.
3. View → Sort by sorts by Title, then reverses the order.
4. The header ⤮ button shuffles the list and clears the sort.
5. A toolbar hint appears in the status bar, and the previous text comes back.
6. View → Toolbars → Main toolbar hides the toolbar and saves that, then shows it again.
7. Customize toolbar: Ctrl+D, Ctrl+E, Space, F4 rename and Alt+↑, then OK. The toolbar and the saved settings change. Reset restores the defaults.
8. Actions: Insert ×3, ⌘/Ctrl+click, Ctrl+Shift+A, Ctrl+A, Space disables all, Alt+↑ moves a non-adjacent selection, Delete removes it. In the group editor, Ctrl+A then Delete removes both actions.
9. Ctrl+Shift+P writes `album.m3u8` (3 files) and `sub/sub.m3u8` (1 file), checked with `read_playlist`. Ctrl+P asks for the file name.
10. Tag Sources (local freedb): field ⌘/Ctrl+click, Ctrl+Shift+A, Ctrl+A, then Space disables every field. Ctrl+I reopens freedb (local). Ctrl+Shift+I searches without pressing Search.
11. Extended Tags save mode: Save writes the file on >>, Ignore discards on <<, and Ask → Don't save leaves the file unchanged.
12. Extended Tags: Cut ARTIST, Paste restores it, and Copy/Paste ALBUM. Keep changes leaves no draft.
13. Tag Panel: right-click hides and shows Composer, and text fields keep the system menu.
14. A COMMENT default value fills in for a two-file selection but not for one file.
15. Ctrl+Shift+Del deletes `zz-delete.mp3` from the disk.
16. `import_mp3tag_columns` reads the fixture `columns.ini`: names, the hidden numeric right-aligned Track with its sort, and ARTIST as `%ARTIST%`.
17. Typing the `sub` folder into the Directory field and pressing Enter lists only it.
18. Drops: dropping the album and then `sub` replaces the list each time. Ctrl+drop of a file adds it. `modifier_keys` answers under xvfb.

Limits of the headless checks: a synthetic drop cannot hold a real key, so the suite sets Ctrl through the window's key tracking, and `modifier_keys` is only checked for its shape. Mp3tag's column import is checked through the backend command, because the file picker is native.

## Remaining gaps

- Custom toolbar (Mp3tag's second toolbar for tools) is not implemented.
- Tag Panel dock/undock and import of Mp3tag tag panel configurations (`remaining-gaps.md` row) were outside this batch.
- Extended Tags OK still stages edits instead of writing them (deliberate, documented above).
