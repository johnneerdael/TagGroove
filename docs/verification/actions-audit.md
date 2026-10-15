# Actions, helper menus and action-group UX audit

Checkpoint: 2026-10-14, branch `feat/actions-audit` (from `feat/core-parity`). macOS arm64 debug build, isolated identifier `org.opentag.editor.e2e.actions`.

Sources compared: every page under the Mp3tag 3.36.1 documentation `actions/`, `menus/actions`, `menus/fields`, `menus/format-strings`, `format`, `scripting`, `regex` and `filter`. Labels missing from the docs came from the shipped `lang/English.lng` (keys such as `_M_STR_ACTION_IMPORTCOVER_TYPE`, `_M_STR_ACTION_ADJUSTCOVER_QUALITY_*`, `_C_OPT_ACTIONGROUPS_SORTBY*`, `_M_STR_TAGFIELDS_*`, `_C_MNU_FS_*`, `_M_STR_COPYOF`) and from the dialog resource strings in `Mp3tag.exe` (Import cover: "Import cover as:", "Delete existing covers"). No documentation prose or proprietary files were copied into the app.

## Option audit

`Rust` = `crates/tag-core/src/actions.rs` unit test; `host` = `src-tauri/src/action_host.rs` real-file test; `node` = `tests/*.test.mjs`; `e2e` = `tests/e2e/actions.js`. "Before" is the state before this branch.

| Action | Documented option or behaviour | Before | Now | Test |
|---|---|---|---|---|
| Adjust Cover | Original / JPEG / PNG format | yes | yes | existing cover_ops tests |
| | Quality (dialog labels Best/High/Medium/Low) | 0–100 number only | preset drop-down (Best 100, High 90, Medium 75, Low 50; the numeric mapping is OpenTag's choice) plus 0–100 value | node, e2e |
| | Size on/off; max. size / min. size; scale proportionally / to square / crop to square | yes | yes | existing |
| Case Conversion | lower / Mixed / Sentence / UPPER | yes | yes | Rust (existing) |
| | Words begin after (separator characters) | yes | yes | Rust (existing) |
| | `_TAG`, `_FILENAME`, `_DIRECTORY`, `_ALL` (not directory), `_FIELDNAME` (APE/Vorbis) | yes | yes; offered in the field drop-down | Rust (existing) |
| Convert Codepage | Choose codepage from a drop-down list | free-text field | drop-down of 32 Windows codepages; an unlisted saved value stays selectable | node (every choice exists in the native decoder table), e2e |
| Export | Runs after all other actions in the group | yes | yes | host (existing) |
| Export Cover to File | Format-string filename, extension from MIME type | yes | yes; format helper | host (existing) |
| | Export only covers of type | yes | yes; unknown type now rejected before any effect | Rust |
| | Export duplicate covers | yes | yes | host (existing) |
| Format Value | Field (entered or from drop-down), `_FILENAME`, `_DIRECTORY`, format strings + scripting | yes | yes; field and format helpers, history | Rust (existing), e2e |
| Generate Playlists | Runs after all other actions | yes | yes | host (existing) |
| Guess Values | Source format, guessing pattern, `%dummy%`, same field on both sides; a source that does not match leaves the file unchanged and the group continues | yes | yes; pattern helper offers `%dummy%` | Rust (existing) |
| Import Cover from File | Format-string filename, relative or absolute | yes | yes | host |
| | Import cover as (cover type) | **missing** | implemented (`picture_type`, default front cover) | Rust, host (real WAV) |
| | Delete existing covers | **missing** | implemented; the image is decoded first so missing/invalid images never remove art | host (file bytes unchanged on failure) |
| Import Text File | Field + format-string filename | yes | yes | host (existing) |
| Merge Duplicate Fields | Separator, `_TAG` | yes | yes | Rust (existing) |
| Remove Duplicate Fields | `_ALL` or one field; only fields with same content | yes | yes | Rust (existing) |
| Remove Fields / Remove Fields Except | Semicolon list; COVER/PICTURE | yes | yes | Rust (existing) |
| Replace | Original / replace (empty removes); only as whole word (`.,;:!?` + whitespace); case-sensitive; `_TAG/_FILENAME/_DIRECTORY/_ALL` | yes | yes | Rust (existing), e2e (Quick) |
| Replace with Regular Expression | Pattern, `$N` references, case-sensitive, `_FILENAME` includes extension | yes | yes; regex snippet helper and history | Rust (existing), e2e |
| Set Cover Properties | Type (`< keep >`), description (keep / empty removes), apply to type or all covers | yes | yes; unknown types rejected before any effect | Rust |
| Split Field by Separator | Separator, trimming, `_TAG` | yes | yes | Rust (existing) |

Saved libraries without the new Import Cover keys still load (`#[serde(default)]`; Rust test).

## Helper menus

Implemented once in `src/helper-menu.ts` and attached declaratively (`data-helper`, `data-history`, `data-specials`); a `MutationObserver` binds inputs in dialogs and re-rendered views without moving the input element, so focus and caret are kept.

**Fields helper menu** (`menus/fields`): Remove field, Remove all fields…, Reset fields… (confirmations use the original wording) operate on a persistent field list that feeds every field-name drop-down together with the selection's fields and the input's special names. Names entered in a field input are added to the list. A "Choose field" submenu was added for touch devices, where `<datalist>` drop-downs are hard to reach (OpenTag addition).

**Format string helper menu** (`menus/format-strings`): tag panel fields first (inserting `%field%` at the caret), then Extended fields, Information fields (documented groups; Export and Playlist groups only on export/playlist inputs), Functions (all 64 documented functions, 65 signatures, grouped as in the scripting reference; the first parameter is selected after insertion). History: Remove from history / Remove all from history…, or Manage history… (add, edit, remove, reorder, named entries with description, use) for converters and the filter. Values are remembered when the dialog's primary button is pressed (Return for the filter). The filter menu adds **Auto-apply filter**; with it off, typing edits a draft and Return applies it. Regex inputs get a regular-expression snippet submenu, an OpenTag addition the team requested.

Attached to: Tag → Filename, Filename → Tag, Filename → Filename (history only), Text File → Tag mask, Tag → Tag (field + format), the filter, every action-editor field, format string, guessing pattern, regex and filename input, the columns editor (value/field/sort by), and the Export dialog (template, file name, partition).

## Action group UX (`menus/actions`, `actions/`)

| Item | State | Evidence |
|---|---|---|
| Actions (Alt+6), Actions (Quick) (Alt+Shift+6), groups listed in the Actions menu | existing | e2e |
| `#` submenus; `&` mnemonics via Alt+A | existing; the menu now underlines the mnemonic character | node, e2e (Alt+A, C runs the group and writes the file) |
| New / Duplicate / Remove / Rename (F2, delayed click) / Edit (F4, double click), move up/down with Alt to extremes, enable checkboxes, selection presets | existing | existing node tests |
| Duplicate default name "Copy of …" (`_M_STR_COPYOF`) | new | node, e2e |
| Sort by name / Sort by state (`_C_OPT_ACTIONGROUPS_SORTBY*`) | new | node, e2e |
| "N action groups enabled" status (`_C_OPT_ACTIONGROUPS_SEL_*`) | new | node, e2e |
| Import `.mta` | existing | existing |
| Export `.mta…` | new: UTF-16LE + BOM + CRLF like the shipped files; only schemas the importer verifies (Case Conversion Mixed, Replace, Format Value, Guess Values, Merge, Split, Remove Fields (Except)) and preserved unresolved records; other types are refused with the action number rather than guessed | Rust round trip incl. escapes, host file round trip |
| Undo of applied groups | verified: `execute_action_groups` runs inside `history::record` | e2e (Apply, then ⌘Z restores COMMENT on disk) |

## Native run

`OPENTAG_E2E_ID=org.opentag.editor.e2e.actions scripts/e2e-native.sh tests/e2e/actions.js` on generated MP3 fixtures: **6/6 PASS**

1. Filter ► menu lists panel fields, Extended/Information fields, Functions, Manage history…, Auto-apply filter; with auto-apply off the list only changes on Return.
2. Tag → Filename: ► inserts `%album%` at the caret and `$num(x,y)` with `x` selected; Manage history… adds a named entry, Use fills the field, and the drop-down list shows it by name.
3. Actions dialog: New group `&Comment fill`, Format Value built with the Fields menu (Choose field → COMMENT) and the format helper (`%artist% (%album%)`); Apply writes only the selected file; ⌘Z restores it on disk.
4. The Actions menu shows `<u>C</u>omment fill`; Alt+A, C applies it to another file.
5. Duplicate proposes "Copy of &Comment fill"; Sort by state/name reorder; the summary reads "1 action group enabled"; Export .mta… is present.
6. Quick action editor: Import Cover type and delete option, quality preset → 75, 32 codepages, regex helper; a Replace quick action writes `Tone A`.

Run notes:
- The runner now builds e2e apps with `backgroundThrottling: disabled`. When several native runs overlapped, WebKit suspended timers in this run's covered window: the script stalled inside `setTimeout` at 0% CPU while IPC still answered. With the policy disabled the run is stable.
- Each run starts from a clean profile (`~/Library/{Application Support,WebKit,Caches}/org.opentag.editor.e2e.actions`). Field lists, histories and the auto-apply choice persist in WebKit storage.

## Gaps and limits

- Mp3tag's `.mta` IDs and option schemas for Adjust Cover, Convert Codepage, Export, Export/Import Cover, Import Text, Remove Duplicates, Regex Replace, Set Cover Properties and Case modes other than Mixed are still unverified, so those actions can't be exported to `.mta` (import keeps them as unresolved records).
- The quality preset numbers and the default field list are OpenTag choices. Mp3tag's internal values were not observed.
- The field list and format-string histories are stored per device in WebView storage, not in `settings.json`, so configuration export/import does not include them yet.
- Information fields are listed as documented. A few (e.g. `%_volume%`, the MP4 timestamps) may evaluate empty because OpenTag has no producer for them yet.
- Edit → Repeat (`_C_MNU_EDIT_REPEAT`) and the original's separate "Select action type" step for Quick actions were not implemented.
- `tagTagFieldOptions` in `src/tag-tag-ui.ts` is now unused at runtime (the Tag → Tag field uses the shared field list) but kept for its existing tests.
- The e2e uses synthetic DOM events, not OS input. Native save dialogs (Export .mta…) and cover image import were covered by Rust/host tests, not the UI run. Windows/Linux/mobile not run.
