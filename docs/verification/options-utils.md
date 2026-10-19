# Options pages, Tools, Utils, Edit and Help extras

> **Historical record.** The gaps below were written when this note was made; later batches closed most of them. The current uncovered list is the [parity ledger](../PARITY.md); the [coverage companion](../research/remaining-gaps.md) records represented and delivered scopes.

The earlier No action fallback recorded below is superseded by the
[2026-10-19 double-click repair](default-tool-no-action-2026-10-19.md).

Scope: the remaining Options pages that can change behaviour (Messages, Tag Panel, Field Values, Tools), configuration backup and restore, the Utils menu, Edit → Copy as Text… and Repeat, and the Help menu. References: `customization/options/{messages,tools,tag-panel,field-values,general,language}`, `utils`, `menus/{edit,file,help}` and `keyboard`.

## Implemented

**Options dialog controls** (`src/options-ui.ts`)
- Two new control kinds:
  - `button` runs a callback, for example to open a dedicated editor.
  - `custom` renders its own markup and binds it. Its `paths` are reset by Restore page defaults.
- Registered pages can declare `defaults` for `pages.<id>.*`. `pageValues()` and `withPageDefaults()` fill in values that were never saved. The dialog and Restore page defaults use the same defaults, so a checkbox that defaults to on is shown as checked.
- `OptionsUI.page(id)` returns a page's saved values. `OptionsUI.updatePage(id, values)` saves values without opening the dialog. Copy as Text uses it to remember format strings.

**Tag Panel and Field Values pages.** These pages appear in the Options tree, as they do in Mp3tag. Each has a button that opens the existing editor (Customize tag panel…, Field values…) on top of Options. Those editors save themselves, independently of OK/Cancel in Options. The pages say so.

**Messages page** (`src/messages.ts`, stored in `pages.messages`). Every checkbox is wired to real behaviour.

| Message | Default | Wired to |
|---|---|---|
| after save | off | notification after Save tags writes at least one file |
| after change | off | notification after action groups, Tag → Filename, Filename → Tag, Filename → Filename, Text file → Tag and Tag → Tag write files. Renaming in the file list does not notify. |
| after delete | off | notification after Remove tags or Remove tag types |
| at removing tags | on | confirmation before Remove tags… |
| at cutting tags | off | confirmation before Cut tags |
| at pasting tags | off | confirmation before Paste tags |
| at exporting | off | after Export CSV or Create playlist, offers to open the file with its default application |
| at deleting files | on | the Delete files… confirmation. Files still go to the Trash or Recycle Bin. |
| at unsaved changes | on | every prompt that discards pending edits: closing the window, Discard, Reload, Refresh, Remove from list, Clear list, and changing directory |

Notifications are queued while an operation runs and are shown as native message dialogs after it finishes. The defaults are a judgement call; the documentation does not state them. The file-ops confirmations are matched by their dialog titles ("Delete files", "Unsaved changes") in `main.ts`, so `src/file-ops.ts` is unchanged.

**Tools page and menu** (`src/tools.ts`, `src-tauri/src/utilities.rs`, stored in `pages.tools`)
- The page holds up to 99 tools, each with a name, a program path (with Browse…), parameters, and "for all selected files". It has New, Remove, Move up and Move down, and a Default tool setting. Validation checks for a name, an absolute path, balanced quotes and the 99-tool limit, and names the Tools page when something is wrong.
- **Placeholders:** `%_path%`, `%_filename_ext%`, `%_filename%`, `%_filename_rel%`, `%_directory%`, `%_folderpath%` and `%_folderpath_rel%`. Matching is case-insensitive. Relative paths are relative to the list root when exactly one folder is loaded. `%_folderpath%` ends with a separator, as in Mp3tag.
- **`run_tool`:**
  - Splits the parameters into arguments: whitespace separates them and double quotes group them. Backslashes are literal.
  - Then expands placeholders in each argument. Values are inserted verbatim. They are never expanded again and never reach a shell.
  - Starts the program with `std::process::Command` and an argument list. stdin, stdout and stderr are null, and the working directory is the file's folder.
  - Reaps finished processes on a background thread.
  - On macOS, an `.app` bundle is started with `/usr/bin/open -a <app> --args …`.
  - "For all selected files" runs the program once per file, up to 100 files, and the UI asks first when more than 10 are selected. Otherwise it runs once, for the first selected file.
  - Empty parameters pass the file path. This is an OpenTag decision.
- **Where tools appear:**
  - A **Tools** menu in the menubar.
  - The new **File List context menu** (right-click a row, or press Shift+F10 or the context-menu key). Under its Tools and Utils headings it lists the tools, Remove tag types…, Extended tags…, Copy as Text…, Remove tags…, Remove from list, Play and Open in Finder/Explorer. Header right-clicks still open the column menu.
  - **Ctrl+1 … Ctrl+0** (⌘ on macOS) run tools 1–10.
  - **Double-click** runs the default tool. With "No action" it opens Extended tags, as before.

**Utils → Remove tag types…** (`crates/tag-core/src/tag_types.rs`, command `remove_tag_types`)
- The dialog counts how many selected files contain ID3v1, ID3v2 and APEv2 (from `_TAG`). Kinds that no selected file has are disabled.
- It warns that removing APEv2 erases MP3GAIN_UNDO and ReplayGain data.
- Removal ignores Options → Tags and carries nothing over into the remaining tags, as documented.
- Files that have none of the chosen kinds are not rewritten. Formats other than MPEG, APE, Musepack and WavPack are reported as errors.
- The command is journaled as "Remove tag types", so Edit → Undo restores the files.

**Edit → Copy as Text…**
- The dialog takes a format string with a live preview of the first five files. The string is evaluated with the Rust scripting engine (`evaluate_columns`) against the displayed values, so pending edits are included.
- It copies one line per selected file.
- The last 10 format strings are remembered in `pages.copy-as-text` and offered as suggestions.
- The text is written with a native clipboard command (`write_clipboard_text`, using `arboard`). WebKit rejected `navigator.clipboard.writeText` once the click had awaited the evaluation (NotAllowedError, seen in the native run).

**Edit → Redo / Repeat (Ctrl+Y).**
- Mp3tag puts Redo and Repeat on one command. While a step can be redone, the item reads "Redo …".
- Otherwise it reads "Repeat <action group>" (or "Repeat N action groups") and runs the last action groups or quick action again on the current selection.

**Help menu**
- Documentation, Report an issue, and "Releases (check for updates)". Each opens a fixed GitHub URL through `open_help_page`, which accepts only known page ids. There is no update-check service; the menu item says so.
- **Keyboard shortcuts.** The menu rows are read from the menus as they are currently rendered, so they cannot drift from the real bindings. A short list adds shortcuts that are not on any menu: F3, list navigation, Enter and double-click, Ctrl+1…0, Alt A and a letter, Shift+Enter, Alt+↓ and Esc.
- About OpenTag is unchanged.

**File → Save configuration…, Load configuration…, Open configuration folder**
- The backup is a JSON bundle, not Mp3tag's zip: `{"format":"opentag-configuration","version":1,"files":{…}}`. It contains `settings.json`, `actions.json`, `exports.json`, `columns.json`, `tag-panel.json` and `directories.json`.
- Discogs credentials (`tag-sources.json`) are always left out.
- Loading a bundle:
  - Rejects the wrong format or version, unknown file names (including path-like names), and empty bundles.
  - Validates every file with its own module's loader before anything is written.
  - Then replaces only the files in the bundle, atomically.
  - Reloads the in-memory settings, and the window reloads after a confirmation that names what will be replaced.

## Tests

- Rust:
  - `crates/tag-core/tests/remove_tag_types.rs` (2): each kind is removed alone and in combination, audio bytes are kept, MP3GAIN_UNDO is gone with APE, absent kinds leave the bytes and mtime untouched, an empty choice is an error, and FLAC is rejected.
  - `src-tauri/src/utilities.rs` (6): parameter splitting, placeholder expansion (including a file named `%_path% $(x) ;rm.mp3`), the for-each switch and limit, a real process launched with a hostile filename that proves no shell is involved, rejection of a missing or relative program, the bundle round trip (credentials excluded, other files kept), all-or-nothing rejection of invalid bundles, and the `remove_tag_types` command.
- Frontend: `tests/options-utils.test.mjs` (10) covers page defaults, the button and custom markup, message defaults and gating, tool normalization, validation, shortcuts and runs, tag-kind counting, the format history, Redo/Repeat selection, menu-derived shortcut rows and the context-menu markup. The `document-coordination` harness gained stubs for the notice queue.
- **Native e2e** (`tests/e2e/options-utils.js`, `OPENTAG_E2E_ID=org.opentag.editor.e2e.options`): **13/13 PASS**, 5 consecutive runs on macOS arm64.
  1. A bundle is exported and settings are changed. The import restores them and rejects an MP3 given as a bundle. The window reloads and the startup "previous directory" from the restored `directories.json` loads the fixtures.
  2. The Options tree contains Messages, Tag Panel, Field Values and Tools, and the saved Messages values are shown.
  3. The Tag Panel and Field Values buttons open their editors over Options.
  4. On the Tools page, the list name updates live. A relative path is rejected with "enter the full path", and the saved list is correct.
  5. ⌘1 runs `/bin/cp "%_path%" "%_folderpath%copy-%_filename_ext%"`, and `copy-sine-a.mp3` appears.
  6. Remove tag types (ID3v1) goes from `ID3v2.3 ID3v1` to `ID3v2.3`, and ⌘Z restores `ID3v2.3 ID3v1`.
  7. Remove tags runs without a native dialog when "at removing tags" is off, and ⌘Z restores the tags.
  8. Copy as Text previews `01. Sine A Saved / 02. Sine B`, copies, and remembers the format. `pbpaste` after the run showed exactly those two lines.
  9. An action group appends `+` to COMMENT. The Edit item reads "Repeat Append plus", and ⌘Y makes the comment `++`.
  10. A row right-click selects the clicked row, lists the Tools and Utils entries, closes on Esc, and the tool entry runs the tool.
  11. Keyboard shortcuts lists 60 rows.
  12. The Tools, Help and File configuration entries are present.
- `tests/e2e/core-workflows.js` still passes 13/13 on this branch.
- Totals before the merge with `feat/core-parity`: Rust 364 (baseline 356), frontend 175 (baseline 165). Clippy `-D warnings`, `cargo fmt` and `tsc` are clean.
- After merging `feat/core-parity` (covers, progress jobs, batch save): Rust 427, frontend 196, all passing. `options-utils.js` and `core-workflows.js` both pass 13/13 natively. `arboard` is shared with the cover clipboard, which enables `image-data`.

## Findings

- Reading the clipboard from the webview (`navigator.clipboard.readText`) can open WebKit's native paste prompt, which stalls a scripted run. The e2e therefore checks the clipboard with `pbpaste` after the run.
- When the app window is hidden behind other windows, WebKit throttles its timers. That made two early runs stall, with an idle main thread in a `sample` trace. Keeping the window on top fixed the stall but broke `core-workflows.js` (stray text appeared in a field), so that change was not kept.

## Gaps (not implemented, no dead controls)

- **Messages:** "at warnings from Actions" (action errors are always shown), "at importing covers" (Tag Sources cover preservation), and the "at exporting tags" prompt for File → Export… (the output name comes from a template and the command does not report the path).
- **General:** auto-select next file, auto-completion, select files automatically, keep moved files, notification area. These belong to the file-list and file-ops areas or have no cross-platform equivalent.
- **Language:** there are no translations, so the page is not shown.
- **Networking, Library, Updates and Playlist pages:** not implemented. Help offers only a link to the releases page.
- **Utils:** Remove Unsupported Fields (MP3), Optimize (FLAC padding, MP4 free atoms and moov placement) and Strip Chapters (MP4). lofty offers no safe primitive for these, so they would need format-specific writers.
- **Tools:** the Windows `.lnk` and `.bat` forms are not special-cased. Tools receive only the documented path placeholders, not tag fields.
- **Configuration:** the backup is a JSON bundle, not Mp3tag's zip, and Mp3tag configuration zips cannot be imported. Files missing from a bundle are kept, not reset.
