# Tag panel parity

Reference pages: `customization/tag-panel`, `customization/options/tag-panel`,
`customization/options/field-values`, `getting-started/main-window`, `menus/view`.

## Implemented

- **Configurable fields** (`src/tag-panel.ts`, `src-tauri/src/tag_panel.rs`). Each entry has a
  name (with `&` marking the Alt shortcut letter, `&&` for a literal ampersand), a field, a
  default value (`< keep >`, `< blank >` or text), a type (Text, Check, Multiline), a size of
  field (Full … Tiny on a 12-column grid, so Year and Track at Normal share a row), the rows for
  Multiline, and a visible flag. `_DIRECTORY` adds a folder switcher and `_SEPARATOR` starts a new
  row. Other technical fields and duplicate fields are rejected by the same rules in the UI and
  the backend.
- **Defaults**: Title, Artist, Album, Year + Track, Genre, Comment (multiline), Album Artist,
  Composer, Discnumber, Directory, with cover art shown. A node test checks the frontend
  defaults against the Rust defaults.
- **Customize tag panel** dialog (View → Customize tag panel…, or the panel's ⋯ menu): New,
  Remove, Move up/down (also Alt+Arrow in the list), checkbox show/hide, Restore defaults,
  Display cover art, Save configuration…/Load configuration… (a versioned JSON file).
- **Panel ⋯ menu** (stands in for Mp3tag's right-click menu): show or hide each field, show or hide
  cover art, save or load the configuration, default values, hide the panel.
- **Comboboxes**: every Text and Multiline field has a drop-down button. The list offers
  `< keep >` when more than one file is selected (it restores each file's loaded value and drops
  the draft), `< blank >` (stages an empty field), the distinct values of the selection, then the
  custom values, then the standard genre list for GENRE. The input uses the ARIA combobox pattern
  with `aria-activedescendant`. ArrowDown/ArrowUp open the list or move through it (Alt+ArrowDown
  in multiline fields), Enter chooses, Escape closes and Tab leaves. The list is a fixed popup, so
  the panel's scroll area does not clip it. On the compact layout options and buttons are at
  least 44px tall. Typing still stages drafts through `draftField`, and ` \\ ` still separates
  multiple values.
- **Check fields** stage `1` or an empty value. A mixed selection shows an indeterminate checkbox.
- **Field values** dialog (View → Field values…): one list per field with one value per line,
  Sort A–Z, Import… and Export… for text files (UTF-8 with BOM on export; on import, BOM-marked
  UTF-8/16 and plain UTF-8 are read first, with Windows-1252 as the fallback). It also has *Show
  only user-defined genres* (the GENRE custom values replace the standard list) and *Prefer
  values from tags in list at tag panel* (custom values appear only when the selection has no
  value). Shift+Enter in a panel field adds its value to that field's list.
- **Default values in input fields** (View menu and ⋯ menu) stages each visible field's
  non-`< keep >` default value as a pending edit on the selected files.
- **Show/hide panel**: View → Hide/Show tag panel, or Ctrl+Q. The setting is saved.
- **Alt+letter** focuses the field whose name marks that letter.
- **Persistence**: `tag-panel.json` in the app config directory. The `load_tag_panel`,
  `save_tag_panel`, `import_tag_panel_layout`, `export_tag_panel_layout`, `import_value_list` and
  `export_value_list` commands write atomically through `action_host::atomic_output`. Older or
  partial files fill missing options with defaults.

## Tests

- `tests/tag-panel.test.mjs` (12 tests): selection state and keep/blank semantics, distinct values,
  custom-value and genre merging, only-user-genres and prefer-tag-values, check state, mnemonics,
  default parity with Rust, layout/settings validation, reordering, value-list normalisation and
  round-trip, default-value staging, directory options.
- `src-tauri/src/tag_panel.rs` (4 tests): default load and save round trip, older files, validation,
  layout export/import, and value-list encodings (UTF-8, Windows-1252, UTF-16LE).
- Manual browser check with a mocked Tauri bridge (Vite dev server, desktop dark and phone light
  at 390px): keyboard and mouse selection of a Genre value, `< keep >` revert, `< blank >`,
  Shift+Enter, Ctrl+Q, Alt+R, adding a Check field and a separator, the duplicate-field error,
  field-value import with only-user-genres, and the directory switcher selecting a folder's files.
  The page did not scroll horizontally at 390px.

## Known gaps vs Mp3tag

- **Filename field**: Mp3tag's panel cannot show technical fields other than `_DIRECTORY`, so no
  filename field was added. Renaming stays in Convert.
- **Directory switcher**: Mp3tag changes the working directory. OpenTag lists the folders of the
  loaded files, selects a folder's files when you choose one, and offers *Open folder…*. Typing a
  path is not supported.
- **Default values**: Mp3tag fills defaults into the inputs when you select several files. OpenTag
  stages them only through the explicit *Default values in input fields* command, so changing the
  selection never stages edits silently. Ctrl+W is not bound, because it closes windows on many
  platforms.
- **Hide panel on the compact layout**: hiding applies only to the desktop layout. The phone layout
  keeps its Files/Tags switcher.
- **Context menu**: the ⋯ menu replaces the right-click menu. Restore all input fields (Ctrl+B) is
  not implemented.
- **Field values autocomplete** in file-list in-place editing and in Extended Tags is not
  implemented. The extended tag values are textareas, which cannot use datalists.
- **Format-string fields** are not implemented, because the reference does not document them.
- **Mnemonic conflicts**: Alt+A opens the Actions menu when focus is not in a text field, so
  `&Album` works only while typing in another field. On macOS, Option+letter is captured only for
  letters that a field marks.
- The standard genre list leaves out one offensive legacy Winamp entry. Docking, orientation and
  panel width are not configurable.
