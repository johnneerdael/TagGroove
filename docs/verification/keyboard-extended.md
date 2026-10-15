# Keyboard shortcuts, macOS Option keys and Extended Tags

Branch `feat/keyboard-extended`. It closes gaps 4, 5, 6 and 21 of `docs/research/remaining-gaps.md` (Extended Tags dialog, the macOS Option-key bug, unbound keys for existing operations, Ctrl+Shift+K) and the runner-up "Remove from list on Mac keyboards". Column auto-size, manual list order and save on move came from `feat/list-behaviour`, merged here. Reference pages: `keyboard` (97 rows), `extended-tags`, `menus/view`, `menus/file`, `menus/edit`.

Nothing was launched locally. The evidence is unit tests and code reading. The native suite `tests/e2e/keyboard-extended.js` runs headless in the CI `native-e2e` job.

## macOS Option keys (gap 5)

On macOS, Option+key types characters: Option+I is the ˆ dead key, Option+A å, Option+G ©, Option+R ®, Option+M µ, Option+6 §, Option+T †. Alt+6, Alt+Shift+6 and the tag panel's Alt+mnemonic jumps used to fire while a text field had focus, so those characters could not be typed.

The rule now lives in `src/keyboard.ts` (`altChord`) and applies to every Alt shortcut: Alt+1…5, Alt+6, Alt+Shift+6, Alt+T, Alt+A, the tag panel mnemonics and Alt+Enter.

| Platform | Focus in a text field | Elsewhere |
|---|---|---|
| macOS | Control+Option+key triggers the shortcut; Option+key types | Option+key or Control+Option+key |
| Windows, Linux | Alt+key (Alt never types there) | Alt+key |
| Any | Ctrl+Alt (AltGr on Windows) and the AltGraph modifier never count as Alt | same |

The physical key comes from `KeyboardEvent.code` when Option or a non-Latin layout changes `key` (`chordKey`), so Option+6 still matches 6 outside text fields. "Text field" means a text-like `<input>`, `<textarea>` or editable content (`isTextEntry`). Checkboxes no longer count as typing.

The same review found more of the AltGr bug on Windows. AltGr is reported as Ctrl+Alt there, and these chords now reach the text field:
- Ctrl+Alt+M (Manage covers) while typing, when the chord types a character such as µ on a German layout.
- The Ctrl shortcut block in `main.ts`, where Polish AltGr+S (ś) used to save. It now ignores chords with Alt.
- Ctrl+Alt+R, which the Ctrl+R handler merged from `feat/remove-mapping` used to swallow.

Deliberate deviation: Mp3tag has no macOS build, so there is no reference for Option. Control+Option was chosen because it types nothing on macOS, while Option alone does. VoiceOver users have Control+Option as the VO modifier, so with VoiceOver running these chords go to VoiceOver first.

## Keyboard reference coverage (gap 6, gap 21)

New bindings, all routed by `windowShortcut` in `src/keyboard.ts` unless noted:

- **⌘/Ctrl+T**: File → Reload tags (Read tag).
- **Alt+T**: Extended tags. The menu item moved from Edit to View, as in Mp3tag.
- **⌘/Ctrl+B**: Edit → Discard pending changes (Restore all input fields). It also works while typing in the tag panel. The Options → Messages confirmation still applies.
- **⌘/Ctrl+Shift+K**: Auto-numbering with "Next disc on change of directory" and "Reset track counter on next disc" preset and disc numbers off. The track counter restarts in each folder and no DISCNUMBER is written (`plan_numbering` semantics). Plain ⌘/Ctrl+K is unchanged.
- **Ctrl+Tab / Ctrl+Shift+Tab** cycle Tag Panel → Filter → File List (`nextPane`). They use the physical Control key on macOS, because ⌘Tab switches applications. A hidden tag panel or an empty list is skipped.
- **Ctrl+Space** opens the helper menu of the focused input that has a ► button, such as converters, actions, the filter or field names (`helperKey` in `src/helper-menu.ts`). It uses the physical Control key on macOS, because ⌘Space is Spotlight. macOS can claim ⌃Space for switching input sources, so Control+Option+Space works too.
- **Shift+Del** in an input with a drop-down history removes the current value from that history.
- **View → Auto-size columns** (new menu item) runs the column auto-size from `feat/list-behaviour` (⌘/Ctrl+Num + and divider double-click). Most Mac keyboards have no numeric keypad, so the menu item is the keyboard-free route there.
- **Alt+Enter** opens the file in Finder, Explorer or the file manager. Mp3tag opens the Windows file-properties sheet, which has no cross-platform equivalent, so this is the closest available target. Before, Alt+Enter fell through to Enter, which opens Extended tags.
- **⌫ on Mac keyboards** removes the selected files from the list. Most Mac keyboards have no forward Delete, and the key is labelled "delete". ⌘⌫ still deletes files, as in Finder.
- **Actions dialog** (`actionListKey` in `src/action-ui.ts`): Insert adds a group or action and Delete (⌫ on Mac keyboards) removes it. Space enables or disables the focused group. ⌘/Ctrl+E enables all groups, ⌘/Ctrl+D disables all, and ⌘/Ctrl+S saves the groups. Alt+↑/↓ move a group or action, and Alt+Page Up/Page Down move it to the beginning or end. F2 and F4 were already bound.
- **Tag Sources review**: Alt+Page Up and Alt+Page Down move the focused file to the beginning or end and keep the order of the other files (`moveSlotTo`).

Help → Keyboard shortcuts is generated from the menus plus `EXTRA_SHORTCUTS` (`src/utils-ui.ts`), so it lists all of these. It also gains rows for the Actions and Tag Sources dialogs, the tag panel mnemonics and the helpers. Its introduction explains the Control+Option rule.

### All 97 rows

The status of every row is also written into the `ui_evidence` of the `shortcuts` section of `docs/feature-inventory.json`. Acceptance stays OPEN, as the inventory's policy requires.

Totals: 60 OK, 21 new in this batch, 3 partial, 13 missing. Before this batch the audit counted 53 OK, 10 partial or differently bound and 34 unbound. Seven of the OK rows came from sibling batches merged through `feat/core-parity`: Ctrl+R from `feat/remove-mapping`, and Alt+drag, the four Alt+arrow/Page keys and Ctrl+Num + from `feat/list-behaviour`.

Still open:
- **Ctrl+Shift+Del** (permanent delete): there is no such operation.
- **Ctrl+A and Ctrl+Shift+A in the Actions and Tag Sources dialogs**: those lists select one item at a time.
- **Toolbar customization (8 rows)**: the dialog does not exist.
- **Ctrl+Shift+P, Ctrl+I, Ctrl+Shift+I**: still partial, unchanged by this batch.

| Scope | Key | Command | State | Where / note |
|---|---|---|---|---|
| General | Ctrl+A | Select all | OK | src/main.ts keydown (⌘A on macOS) |
| General | Ctrl+Shift+A | Invert selection | OK | src/list-commands.ts listShortcut |
| General | Ctrl+B | Restore all input fields | New | src/keyboard.ts windowShortcut → Edit → Discard pending changes; works while typing in the tag panel |
| General | Ctrl+C | Copy tag or selected text | OK | src/edit-history.ts editShortcut; text fields keep native copy |
| General | Ctrl+Shift+C | Copy file | OK | src/file-ops.ts fileShortcut |
| General | Ctrl+D | Change directory | OK | src/file-ops.ts fileShortcut |
| General | Ctrl+Shift+D | Upper directory | OK | src/file-ops.ts fileShortcut |
| General | Del | Remove file (without deleting it) | OK | src/list-commands.ts listShortcut; on Mac keyboards ⌫ as well |
| General | Ctrl+Del | Delete file (to Windows Recycle Bin if activated in Windows and available on that drive) | OK | src/file-ops.ts fileShortcut (⌘⌫ on macOS) |
| General | Ctrl+Shift+Del | Delete file (WARNING: the file will be removed from your harddrive!) | Missing | no permanent-delete operation |
| General | Ctrl+E | Export | OK | src/main.ts keydown |
| General | Ctrl+Shift+E | Export (with an export file for each different directory) | OK | src/main.ts keydown |
| General | Ctrl+F | Find | OK | src/list-commands.ts listShortcut |
| General | Ctrl+G | Select similar files | OK | src/list-commands.ts listShortcut |
| General | Ctrl+Shift+G | Select next similar files | OK | src/list-commands.ts listShortcut |
| General | Ctrl+H | Add directory | OK | src/file-ops.ts fileShortcut (physical Ctrl on macOS: ⌘H hides the app) |
| General | Ctrl+K | Auto-numbering wizard | OK | src/main.ts keydown |
| General | Ctrl+Shift+K | Auto-numbering wizard (with counter reset for each subfolder) | New | src/keyboard.ts windowShortcut → Auto-numbering with Next disc on change of directory + Reset track counter preset (no DISCNUMBER written) |
| General | Ctrl+L | freedb (local) … | OK | src/main.ts keydown |
| General | Ctrl+M | Favorite directory | OK | src/file-ops.ts fileShortcut (physical Ctrl on macOS: ⌘M minimizes) |
| General | Ctrl+Alt+M | Manage covers… | OK | src/covers.ts install (⌘⌥M); AltGr+M typing a character (µ) is left to the text field |
| General | Ctrl+N | Select next file | OK | src/list-commands.ts listShortcut |
| General | Ctrl+O | Options … | OK | src/main.ts keydown |
| General | Ctrl+P | Playlist (all files) | OK | src/main.ts keydown |
| General | Ctrl+Shift+P | Playlist (all files) with one playlist for each subfolder | Partial | opens Generate playlists, which uses the selection when one exists |
| General | Ctrl+Q | Show/hide Tag Panel | OK | src/tag-panel.ts install (physical Ctrl on macOS: ⌘Q quits) |
| General | Ctrl+R | Remove tag | OK | src/remove-tag.ts removeTagShortcut (remove-mapping batch) |
| General | Ctrl+S | Save tag | OK | src/main.ts keydown |
| General | Ctrl+T | Read tag | New | src/keyboard.ts windowShortcut → File → Reload tags |
| General | Ctrl+Shift+T | Play | OK | src/file-ops.ts fileShortcut |
| General | Ctrl+U | Unselect all | OK | src/list-commands.ts listShortcut |
| General | Ctrl+V | Paste tag or text | OK | src/edit-history.ts editShortcut |
| General | Ctrl+Shift+V | Move file | OK | src/file-ops.ts fileShortcut |
| General | Ctrl+X | Cut tag or selected text | OK | src/edit-history.ts editShortcut |
| General | Ctrl+Z | Undo | OK | src/edit-history.ts editShortcut |
| General | Ctrl+1, 2, ..., 0 | User-defined Tool 1 to 10. | OK | src/tools.ts toolShortcutIndex |
| General | Del | Remove | OK | src/list-commands.ts listShortcut; on Mac keyboards ⌫ as well |
| General | F2 | Rename first file or edit first editable column | OK | src/list-commands.ts listShortcut |
| General | F3 | Filter | OK | focuses the filter, which is always shown |
| General | F4 | Play selected file(s) | OK | src/file-ops.ts fileShortcut |
| General | F5 | Refresh | OK | src/list-commands.ts listShortcut |
| General | Alt+Right arrow | Forward in directory history | OK | src/file-ops.ts fileShortcut (outside text fields) |
| General | Alt+Left arrow | Backward in directory history | OK | src/file-ops.ts fileShortcut (outside text fields) |
| General | Alt+Enter | File properties | New | src/keyboard.ts windowShortcut → Open in Finder/Explorer/file manager; no system properties sheet exists cross-platform |
| General | Alt+T | Extended tags dialog | New | src/keyboard.ts windowShortcut → View → Extended tags (⌃⌥T while typing on macOS) |
| General | Alt+Drag’n’Drop | Change order of files in list of files. | OK | src/list-behaviour.ts bindRowDrag (list-behaviour batch) |
| General | Alt+Arrow down | Change order of files in list of files. | OK | src/list-behaviour.ts reorderKey (list-behaviour batch) |
| General | Alt+Arrow up | Change order of files in list of files. | OK | src/list-behaviour.ts reorderKey (list-behaviour batch) |
| General | Alt+Page down | Move selected files to end in list of files. | OK | src/list-behaviour.ts reorderKey (list-behaviour batch) |
| General | Alt+Page up | Move selected files to beginning in list of files. | OK | src/list-behaviour.ts reorderKey (list-behaviour batch) |
| Convert | Alt+1 | Tag - Filename | OK | src/edit-history.ts editShortcut (⌃⌥digit while typing on macOS) |
| Convert | Alt+2 | Filename - Tag | OK | src/edit-history.ts editShortcut (⌃⌥digit while typing on macOS) |
| Convert | Alt+3 | Filename - Filename | OK | src/edit-history.ts editShortcut (⌃⌥digit while typing on macOS) |
| Convert | Alt+4 | Text file - Tag | OK | src/edit-history.ts editShortcut (⌃⌥digit while typing on macOS) |
| Convert | Alt+5 | Tag - Tag | OK | src/edit-history.ts editShortcut (⌃⌥digit while typing on macOS) |
| Actions | Alt+6 | Actions | OK | src/keyboard.ts windowShortcut (⌃⌥6 while typing on macOS; Option+6 types §) |
| Actions | Alt+Shift+6 | Quick action | OK | src/keyboard.ts windowShortcut (⌃⌥⇧6 while typing on macOS) |
| Actions | F2 | Rename action group | OK | src/action-ui.ts group row keydown |
| Actions | F4 | Edit action (group) | OK | src/action-ui.ts group row and group editor keydown |
| Actions | Insert | New action (group) | New | src/action-ui.ts actionListKey: New group / New action |
| Actions | Delete | Delete action (group) | New | src/action-ui.ts actionListKey: Remove group / action (⌫ on Mac keyboards) |
| Actions | Space | Enable action group | New | src/action-ui.ts actionListKey: toggles the focused group row |
| Actions | Ctrl+A | Select all actions or action groups | Missing | the lists select one item at a time |
| Actions | Ctrl+Shift+A | Invert selection of actions or action groups | Missing | the lists select one item at a time |
| Actions | Ctrl+E | Enable all action groups | New | src/action-ui.ts actionListKey: enable all groups |
| Actions | Ctrl+D | Disable all action groups | New | src/action-ui.ts actionListKey: disable all groups |
| Actions | Ctrl+S | Save current order and enabled state of action groups | New | src/action-ui.ts actionListKey: Save groups |
| Actions | Alt+Up | Move action or action group up | New | src/action-ui.ts actionListKey (groups and actions) |
| Actions | Alt+Down | Move action or action group down | New | src/action-ui.ts actionListKey (groups and actions) |
| Actions | Alt+Page Up | Move action or action group to beginning of list | New | src/action-ui.ts actionListKey: move to the beginning |
| Actions | Alt+Page Down | Move action or action group to end of list | New | src/action-ui.ts actionListKey: move to the end |
| Tag Sources | Ctrl+I | Last used Tag Source | Partial | remembers MusicBrainz or Discogs only |
| Tag Sources | Ctrl+Shift+I | Last used Tag Source (Quick) | Partial | same as Ctrl+I; the search step is not skipped |
| Tag Sources | F2 | Edit value of selected field | OK | src/tag-sources.ts review keydown |
| Tag Sources | Space | Enable selected field | OK | field checkboxes |
| Tag Sources | Ctrl+A | Select all fields | Missing | fields are not multi-selectable |
| Tag Sources | Ctrl+Shift+A | Invert selection of fields | Missing | fields are not multi-selectable |
| Tag Sources | Ctrl+E | Enable all fields | OK | src/tag-sources.ts review keydown |
| Tag Sources | Ctrl+D | Disable all fields | OK | src/tag-sources.ts review keydown |
| Tag Sources | Alt+Up | Move file up | OK | src/tag-sources.ts review keydown |
| Tag Sources | Alt+Down | Move file down | OK | src/tag-sources.ts review keydown |
| Tag Sources | Alt+Page Up | Move file to beginning of list | New | src/tag-sources.ts moveSlotTo: file to the beginning |
| Tag Sources | Alt+Page Down | Move file to end of list | New | src/tag-sources.ts moveSlotTo: file to the end |
| Toolbar Customization | F4 | Edit selected item | Missing | no toolbar customization dialog |
| Toolbar Customization | Space | Show/hide selected item | Missing | no toolbar customization dialog |
| Toolbar Customization | Ctrl+E | Show all items | Missing | no toolbar customization dialog |
| Toolbar Customization | Ctrl+D | Hide all items | Missing | no toolbar customization dialog |
| Toolbar Customization | Alt+Up | Move item up | Missing | no toolbar customization dialog |
| Toolbar Customization | Alt+Down | Move item down | Missing | no toolbar customization dialog |
| Toolbar Customization | Alt+Page Up | Move item to beginning of list | Missing | no toolbar customization dialog |
| Toolbar Customization | Alt+Page Down | Move item to end of list | Missing | no toolbar customization dialog |
| Useful helpers | Ctrl+Space | Displays format string helper menu (e.g., at converters or actions) | New | src/helper-menu.ts helperKey (physical Ctrl; ⌃⌥Space also on macOS) |
| Useful helpers | Ctrl+Tab | Quick forward navigation between sub-windows (Tag Panel, Filter, File View) | New | src/keyboard.ts windowShortcut + src/keyboard.ts nextPane: Tag Panel → Filter → File List (physical Ctrl on macOS) |
| Useful helpers | Ctrl+Shift+Tab | Quick backward navigation between sub-windows (Tag Panel, Filter, File View) | New | src/keyboard.ts windowShortcut + nextPane, backwards |
| Useful helpers | Ctrl+Num++ | Automatically size the File List columns to match the content | OK | src/list-behaviour.ts autoSizeShortcut and divider double-click (list-behaviour batch); View → Auto-size columns added here for keyboards without a keypad |
| Useful helpers | Shift+Del | Remove format strings which are not longer needed from drop-down lists. | New | src/helper-menu.ts helperKey: removes the input value from its drop-down history |
| Useful helpers | Shift+Enter | Add the contents of the current Tag Panel field to the list of custom list values for that field | OK | src/tag-panel.ts comboKey |

## Extended Tags dialog (gap 4)

The dialog moved from `main.ts` into `src/extended-tags.ts`. Pure helpers live in `src/tag-edits.ts` (`extendedCaption`, `neighbourFile`).

- **Caption**: for one file, the tag format and file name, for example "Extended tags — ID3v2.3 ID3v1 — a.mp3", from `_TAG`. For several files it shows the count.
- **<< and >>** (single file only) step through the File List in display order, including the current filter, sort and manual order. They are disabled at either end. Moving to another file stages the current file's edits as pending changes, the same as Keep changes, then selects the new file in the File List. The tag panel and the cover window follow. With Options → Tags "Save tags when using arrow keys or a single mouse click" on, the file left behind is saved, as for any other move. An invalid row (empty or duplicate field name) shows the error and does not move. When the dialog closes, the status bar reports how many files still have pending Extended Tags changes.
  - Deviation: Mp3tag asks, saves or discards when navigating, depending on Options → Tags → "Save mode at extended tag dialog", and OK saves to disk. OpenTag edits stay pending until Save tags, unless save on move is enabled, so nothing is lost. A separate save-mode option does not exist in OpenTag.
- **Field names** have the Fields drop-down (`data-helper="field"`). It lists the user's field list (the defaults include ALBUMARTIST, BPM, ISRC, …), every field of the selected files, and the ► Fields menu.
- **Values** auto-complete from the Field Values custom lists and the values of loaded files. This uses the File List cell completer and follows Options → General "auto-completion".
- **Cover art**: the dialog shows the cover window, which is the same `CoverUI` as the tag panel. It has Add cover…, Remove cover and Extract cover… buttons, the ⋯ and right-click context menu (opened inside the dialog because content outside a modal dialog is inert), paste, keyboard commands (Del, ←/→, Enter, ⌘/Ctrl C/X/V) and native image drops. The drop target is the dialog's cover window while the dialog is open. Cover commands write immediately and can be undone, as in the tag panel.
  - Deviation: in Mp3tag, cover edits apply when the dialog is confirmed.
  - Dragging a cover out of the window is not supported (here or in the tag panel).
- **Several files**: the semantics are unchanged. Differing values show "Mixed values — keep existing values" and are kept per file unless edited. Renaming a field keeps each file's values.
- Not done: the field list's Cut/Copy/Paste context menu.

## Tests

Unit tests (`node --test tests/*.test.mjs`, 243 passing after the merge):

- `tests/keyboard.test.mjs` (new):
  - Platform and text-entry detection, and chord keys under Option, AltGr and Cyrillic layouts.
  - Every Option character from the bug report is left to text fields on macOS and triggers outside them. Control+Option triggers while typing.
  - Alt works while typing on Windows/Linux. AltGr (Ctrl+Alt) and AltGraph chords never trigger.
  - Ctrl/⌘ mapping per platform for T, B, Shift+K and Num +. Ctrl+Tab keeps physical Control. Alt+Enter only outside text fields. Pane order.
- `tests/edit-history.test.mjs`: Control+Option+digit converters while typing on macOS. Ctrl+Alt and AltGraph digits are ignored.
- `tests/list-commands.test.mjs`: ⌫ removes from the list only on Mac keyboards and only outside text fields. ⌘⌫ is unaffected.
- `tests/action-ui.test.mjs`: the Actions dialog key map.
- `tests/tag-sources.test.mjs`: `moveSlotTo`.
- `tests/helper-menu.test.mjs`: Ctrl+Space and Shift+Del, and that ⌘Space and Shift+Backspace are not taken.
- `tests/tag-edits.test.mjs`: the caption and `<<`/`>>` neighbours.
- `tests/remove-mapping.test.mjs`: AltGr+R types its character.

Native suite `tests/e2e/keyboard-extended.js`, added to the CI suite list. It overrides `navigator.platform`, which the app reads on every key press, to exercise the macOS rules on the Linux runner. Steps:
1. Option+I/A/G/R/M/6/1/T and Option+Shift+6 in the Title field are not consumed: focus stays and no dialog opens.
2. Control+Option+I jumps to Artist while typing. Option+L jumps to Title from the File List. Control+Option+T opens Extended tags.
3. Windows: AltGr+M (µ) and AltGr+A (ą) stay in the field, and Alt+I still jumps.
4. Alt+T opens Extended tags with the file name in the caption, << disabled and >> enabled, the field-name drop-down (ALBUMARTIST, TITLE), the ► button and the cover window.
5. Edit TITLE, then >> (the File List selection follows and the next file's values show), then << (the edit is still there), then Cancel. The status says 1 file is pending, and ⌘/Ctrl+S writes "Sine A edited" to disk.
6. Typing "Test" in ARTIST completes to "Test Artist" with the completion selected.
7. Paste a PNG on the dialog's cover window (embedded and shown in the dialog), then Remove cover (removed, dialog stays open).
8. Ctrl+Tab ×3 then Ctrl+Shift+Tab: panel → filter → list → filter.
9. Alt+1 then Ctrl+Space in the format string opens the helper menu, and Escape closes it.
10. Ctrl+Shift+K presets the per-folder options, and Ctrl+K does not.
11. ⌘/Ctrl+T reloads.
12. The View/File/Edit menus show Alt T, ⌘/Ctrl T and ⌘/Ctrl B. Help → Keyboard shortcuts lists the new rows and the Control+Option note.
13. View → Auto-size columns (the menu item for keyboards without a keypad) changes the Filename width and the file name is no longer truncated.
14. With a Mac keyboard, ⌫ on a row removes it from the list and the file stays on disk.

Synthetic DOM key events show what the app does with a key. They cannot show that WebKit then inserts the character. On macOS the OS produces the character and the app now leaves the event alone, so the remaining risk is WebKit-specific dead-key handling, which no CI runner covers.

Checks after merging `feat/core-parity` (`7288e60`):
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- `cargo test --workspace --locked`: 513 passed, 0 failed, 4 ignored (31 suites).
- `npx tsc --noEmit` passes, and the unit tests pass (243).
