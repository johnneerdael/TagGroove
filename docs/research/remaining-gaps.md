# Remaining parity gaps versus Mp3tag 3.36.1

Audit of `main` at `d220ad9` (2026-10-15), after the formats-3, freedb, playlists/export, general-options and perf-2 batches. Method: every menu page, all 97 keyboard rows, every scripting function, placeholder, filter operator and regex feature, every action type option, every converter, the export/playlist/cover/utils/find/manage-list/command-line pages, every options page and the tag-source, file-format and mapping pages of the private documentation cache were compared against the code. Nothing was launched. The evidence is code reading plus these unit suites, all passing: `node --test` (shortcuts, options, tag panel, columns, export/playlist/covers/utils/helper menus, action UI, converter and numbering review) and `cargo test` (tag-core script, filter, export, raw streams, chapters, containers, ASF, mappings, actions; `opentag` sources).

This is a gap register, not a parity certificate. "OK" means the code implements the documented behaviour as far as reading can tell; no item has differential evidence against the original program.

Legend: **Impact** H = daily tagging workflows, M = regular but not daily, L = rare or legacy. **Size** S = hours, M = a day or two, L = a batch of its own. Docs pages are relative to `docs.mp3tag.de`.

## Counts per area

| Area | Items checked | OK | Partial | Missing | Notes |
|---|---:|---:|---:|---:|---|
| Menu commands (10 menu pages) | 85 | 69 | 12 | 4 | 1 missing item (Donation) out of scope |
| Keyboard reference rows | 97 | 53 | 10 | 34 | 10 = 5 bound to a different action + 5 partial |
| Scripting functions | 64 | 61 | 3 | 0 | |
| Placeholders (`format`) | 72 | 50 | 5 | 17 | |
| Filter syntax | 19 | 17 | 2 | 0 | |
| Regular expressions | 19 | 18 | 1 | 0 | |
| Action types and action groups | 114 | 96 | 17 | 1 | plus a cross-cutting rename bug (X1) |
| Converters and auto-numbering | 67 | 57 | 6 | 4 | |
| Export, playlists, covers, utils, find, manage list, CLI | 97 | 82 | 11 | 4 | |
| Options pages and UI customization | 147 | 101 | 17 | 24 | plus 4 mapping items implemented in the core but never wired, 1 out of scope |
| Tag sources | 33 | 20 | 10 | 3 | |
| File formats (tag kinds per format) | 21 | 16 | 3 | 2 | |
| Mapping table (registry cells) | 118 rows | 118 | 0 | 0 | 7 gap categories outside the registry |

Some gaps appear in more than one area (for example Remove tag and Ctrl+R, manual reordering and Alt+arrows, `%_md5audio%`). The priority list counts each once.

## Top 25 by impact

| # | Gap | Impact | Size |
|---:|---|---|---|
| 1 | Renames that change only letter case fail on macOS and Windows (hard-link no-clobber, then "destination exists"). This breaks Case Conversion on `_FILENAME`/`_DIRECTORY` and case-only Tag→Filename and Filename→Filename runs | H | M |
| 2 | File → Remove tag strips every tag kind instead of the kinds chosen under Options → Tags, and Ctrl+R is unbound (on WebView2 it probably reloads the page) | H | M |
| 3 | Options → Tags → Mapping (user-defined field mappings per tag type) has no UI or storage. The engine in `mapping.rs` exists, but every call site passes an empty rule list | H | M |
| 4 | The Extended Tags dialog has no cover-art panel, no `<<`/`>>` navigation between files, no Alt+T, no field-name drop-down and no Field Values auto-complete | H | M |
| 5 | macOS: Option+6 and the tag panel's Alt+letter jumps fire while typing, which swallows characters such as `ˆ` (Option+I), `å`, `©`, `®` and `µ` | M–H | S |
| 6 | Unbound keys for operations that already exist: Ctrl+T (read tags), Alt+T (Extended Tags), Ctrl+B (restore panel), Ctrl+Space (format-string helper), Ctrl+Tab (cycle panes) | M | S |
| 7 | Options → Tags "save tags when using arrow keys / single click" is missing. Edits stay pending until Ctrl+S | M–H | M |
| 8 | No manual file-list reordering (Alt+Up/Down/PgUp/PgDn, Alt+drag), so playlist order can only come from sorting or load order | M | M |
| 9 | The cover pane never shows the folder's first image when nothing is embedded (option "Don't display first folder image as cover" is missing, and the fallback with it) | M | M |
| 10 | The `$regexp` 4th argument defaults to case-sensitive. The doc implies case is ignored unless `c=0`. Needs a differential check, then a flipped default | M–H | S |
| 11 | Filter GREATER/LESS/EQUAL only compare whole integers, so `track GREATER 10` misses `3/12`-style values | M | S |
| 12 | Copy and Cut tags leave out cover art | M | M |
| 13 | Case Conversion on `_FIELDNAME` always errors (expects `_TAG` = `APEv2`/`VorbisComments`, but the reader reports `APE`/`VorbisComment`), and case-only key changes are not written | M | M |
| 14 | Format Value, Guess Values and Tag→Tag targeting `_FILENAME` evaluate as plain text, not filename mode: `/` makes folders, illegal characters are not replaced, `%title%.mp3` doubles the extension | M | S |
| 15 | Missing placeholders `%_workingdir%` and `%_workingpath%` (reserved names, never filled) | M | S |
| 16 | Missing `%_vbr%` and `%_md5audio%`. The latter makes exports that use it stop with an error; FLAC/WavPack already carry the value | M | M |
| 17 | MTE `$filename(…,ansi)` fails on macOS/Linux instead of falling back to Windows-1252 as playlist reading does | M | S |
| 18 | Export configurations are UUID-named and listed only from `exports.json`, so `.mte` files dropped into the export folder are ignored | M | M |
| 19 | `.mta` import/export covers only the verified action IDs. Other types import as disabled, unresolved records and export is refused | M | M |
| 20 | Export Cover adds the image extension only when the name has none (`Vol. 2`, `.jpg` holding PNG, `.img` for webp/bmp). Adjust Cover "max size" enlarges small covers in square/crop modes | M | S |
| 21 | Ctrl+Shift+K (auto-numbering restarting per folder) acts like Ctrl+K; wizard options are not remembered | M | S |
| 22 | Text File → Tag cannot take its source path as a format string (e.g. `%_folderpath%tracks.txt`) | M | S |
| 23 | Options → Files "Replace folder separator in tag values" is fixed to `_`; there is no setting and no way to remove the separator | M | S |
| 24 | WAV: RIFF INFO is updated only if it already exists (new files get ID3v2 only), and fields without a RIFF id are dropped silently | M | S |
| 25 | No `.src` web-source interpreter, so users cannot install community sources (Bandcamp, Beatport, lyrics…); only the native MusicBrainz, Discogs and freedb clients exist | M | L |

Runners-up: column auto-size (Ctrl+Num+ or double-click on a divider, M/S); Mac keyboards have no way to trigger Remove from list (only forward-Delete is mapped, M/S); gnudb rejects OpenTag until the application is registered (M/S, no code); Finder "Open With" and `open --args` forwarding on macOS (M/M); Library database (M/L); UI translations (M/L); "List chapters as separate files" (M/L).

## Menus

85 commands across File, Edit, View, Convert, Actions, Tag Sources, Help, the cover context menu and the Fields and Format-string helper menus. Convert, Actions, both helper menus and the cover context menu match the documented commands completely.

| Command | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| File → Remove tag (Ctrl+R) | Removes the tag kinds chosen under Options → Tags | **Partial**: strips all kinds (`crates/tag-core/src/tags.rs:1458`). The options' remove switches apply only when saving (`src/options-ui.ts:103`). No key | H | M | menus/file |
| File → Read tag (Ctrl+T) | Rereads tags of the selection | **Partial**: "Reload tags" exists (`src/main.ts:1336`), no key | M | S | menus/file |
| File → Exit | Quit | **Partial**: no menu item; macOS default app menu only | L | S | menus/file |
| File → Export with Shift | Shift-click exports one file per folder | **Partial**: only Ctrl+Shift+E, not Shift-click | L | S | menus/file |
| Edit → Copy / Cut (tags) | Copies the tags, including artwork | **Partial**: text fields only, artwork left out (`src/main.ts:1190`) | M | M | menus/edit |
| Edit → Cover submenu | Cover commands from Edit | **Partial**: only Manage covers…; the rest are in the cover context menu | L | S | menus/edit |
| View → Sort by | Pick sort criteria from a menu | **Partial**: header clicks and Unsorted only | M | S | menus/view |
| View → Extended Tags (Alt+T) | Extended fields and cover art | **Partial**: under Edit; no Alt+T; no cover editing (`src/main.ts:982`) | H | M | menus/view |
| View → Filter (F3) | Toggles the filter bar | **Differs**: always shown; F3 focuses it | L | S | menus/view |
| View → Toolbars | Show/hide main and custom toolbars; customize | **Missing**: one fixed toolbar | L | M | menus/view |
| Tag Sources → freedb | Lookup by audio CD, files, disc ID or text | **Partial**: files and disc ID only (`src/tag-sources.ts:432`) | L | M | menus/tag-sources |
| Tag Sources → web sources (Ctrl+I) | Script-defined sources, user-installable | **Partial**: native MusicBrainz and Discogs only | M | L | menus/tag-sources |
| Help → First Steps | Opens the getting-started guide | **Missing** | L | S | menus/help |
| Help → Website | Project website | **Missing**: the URL is in `HELP_PAGES` (`src-tauri/src/utilities.rs:494`) but has no menu item | L | S | menus/help |
| Cover → Add cover… start folder | Starts in the cover-art folder, then a folder with an image, then the working folder | **Partial**: always the selected file's folder (`src/covers.ts:184`) | L | S | menus/cover |

OpenTag-only additions: Tools menu, Load configuration…, Previous/Next cover and View larger in the cover menu.

## Keyboard shortcuts (97 rows of `keyboard`)

OK (53): Ctrl+A, Ctrl+Shift+A, Ctrl+C, Ctrl+Shift+C, Ctrl+D, Ctrl+Shift+D, Ctrl+Del, Ctrl+E, Ctrl+Shift+E, Ctrl+F, Ctrl+G, Ctrl+Shift+G, Ctrl+H, Ctrl+K, Ctrl+L, Ctrl+M, Ctrl+Alt+M, Ctrl+N, **Ctrl+O (Options, as documented)**, **Ctrl+P (playlist of all files)**, Ctrl+Q, Ctrl+S, Ctrl+Shift+T, Ctrl+U, Ctrl+V, Ctrl+Shift+V, Ctrl+X, Ctrl+Z, Ctrl+1…0, Del (both rows), F2, F3, F4, F5, Alt+Left/Right, Alt+1…5, Alt+6, Alt+Shift+6, Actions F2/F4, Tag Sources F2/Space/Ctrl+E/Ctrl+D/Alt+Up/Alt+Down, Shift+Enter. The Ctrl+O contradiction recorded in `FEATURE-INVENTORY.md` is resolved.

| Key | Mp3tag | OpenTag state | Impact | Size |
|---|---|---|---|---|
| Ctrl+R | Remove tag | Unbound (operation exists); WebView2 may reload | H | S |
| Ctrl+T | Read tags | Unbound (operation exists) | M | S |
| Ctrl+B | Restore Tag Panel inputs | Unbound; Edit → Discard pending changes has no key | M | S |
| Alt+T | Extended Tags | Unbound | M | S |
| Ctrl+Space | Format-string helper menu in an input | Unbound | M | S |
| Ctrl+Tab / Ctrl+Shift+Tab | Cycle Tag Panel, Filter, File List | Unbound | M | S |
| Alt+Up/Down, Alt+PgUp/PgDn | Move selected files in the list | **Bound to something else**: moves the selection (`src/main.ts:843`) | M | M |
| Alt+drag | Reorder files by hand | Unbound; no manual order | M | M |
| Alt+Enter | System file-properties dialog | **Bound to something else**: opens Extended Tags (`src/main.ts:848`). Windows-only target; reveal-in-file-manager is the closest match | L | S |
| Ctrl+Shift+K | Numbering restarting per folder | **Partial**: Shift ignored (`src/main.ts:1459`) | M | S |
| Ctrl+Shift+P | Playlist per subfolder of all files | **Partial**: opens Generate Playlists, which uses the selection when one exists and remembers an edited partition | L | S |
| Ctrl+I | Last used tag source | **Partial**: remembers only MusicBrainz/Discogs | M | S |
| Ctrl+Shift+I | Last source, skipping the search step | **Partial**: same as Ctrl+I | L | M |
| Ctrl+Shift+Del | Permanent delete | Unbound; no permanent-delete operation | L | M |
| Ctrl+Num+ | Auto-size columns | Unbound; no auto-size | L–M | M |
| Shift+Del in a history drop-down | Remove one history entry | Unbound | L | S |
| Actions dialog: Insert, Delete, Space, Ctrl+A, Ctrl+Shift+A, Ctrl+E, Ctrl+D, Ctrl+S, Alt+Up/Down/PgUp/PgDn | Keyboard management of groups and actions | 10 unbound, Space partial (only once the checkbox has focus); buttons exist | L | S–M |
| Tag Sources dialog: Ctrl+A, Ctrl+Shift+A, Alt+PgUp/PgDn | Multi-select fields; move file to top/bottom | Unbound | L | S |
| Toolbar customization: F4, Space, Ctrl+E, Ctrl+D, Alt+arrows (8 rows) | Customize toolbar dialog | Unbound; the dialog does not exist | L | M |

Platform notes: ⌘ replaces Ctrl on macOS, except Ctrl+H, Ctrl+M and Ctrl+Q, which keep the physical Ctrl key because macOS reserves the ⌘ versions. On Mac keyboards Remove from list needs fn+⌫ (plain ⌫ is not mapped). Function keys need fn and there is no ⌘ fallback (for example ⌘R for refresh). Unbound browser keys on Windows (Ctrl+R reload, Ctrl+P print while typing) are not suppressed. The `shortcuts` section of `docs/feature-inventory.json` still reports most rows as unhandled and needs regenerating.

## Scripting, placeholders, filter and regex

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| `$regexp(x,e,r,c)` default | Case ignored unless `c=0` (implied) | **Partial**: case-sensitive unless `c=1` (`crates/tag-core/src/script.rs:1280`, pinned by a test). Confirm on the original first | M–H | S | scripting |
| `$caps`/`$caps2` default word starts | Built-in separator set undocumented | **Unverified**: only whitespace starts a word, so `o'brien (live)` → `O'brien (live)` | M | S | scripting |
| Truth of non-boolean conditions | Undocumented | **Unverified model**: literal `1` is false; functions are true when an argument field exists (`script.rs:532`) | M | M | scripting |
| Errors inside a format string | Probably degrade inline | **Differs**: `$div(x,0)`, bad `$char` or unknown `$func` abort the whole string | L | S | scripting |
| `%_workingdir%`, `%_workingpath%` | Current working folder name and path | **Missing** (reserved only, `src-tauri/src/evaluation.rs:320`) | M | S | format |
| `%_vbr%` | CBR/VBR or profile | **Missing** | M | M | format |
| `%_md5audio%` | MD5 of the audio data | **Missing**: errors in exports (`src-tauri/src/file_hashes.rs:10`) | M | M | format |
| `%_tag_size%`, `%_tag_size_appended%`, `%_tag_size_prepended%` | Tag byte sizes | **Missing** | L | M | format |
| `%_video_bitrate%`, `%_video_codec%`, `%_video_width%`, `%_video_height%` | Video stream properties | **Missing** | L | M | format |
| `%_mp4_creation_time%`, `%_mp4_modification_time%` (+ `_raw`) | MP4 header timestamps | **Missing** | L | S | format |
| `%_subsong%`, `%_volume%` | Chapter index; volume label | **Missing** | L | S–M | format |
| `%_subsong_count%` | Chapter count | **Partial**: MP4 and MKV only | L | S | format |
| `%_tool%` | Encoder detail (e.g. LAME string) | **Partial**: Vorbis vendor only | L–M | M | format |
| `%_length%` | Formatted length | **Partial**: always `m:ss` (`75:03`), truncates while `_length_seconds` rounds | L | S | format |
| `%_app%` in filters | Application name | **Partial**: empty in the filter context | L | S | format |
| `%_counter%` for tag-source cover names | 1-based index | **Partial**: 0 there (`src-tauri/src/sources/mod.rs:465`) | L | S | format |
| Filter GREATER/LESS/EQUAL | Numeric comparison | **Partial**: whole integers only (`crates/tag-core/src/filter.rs:232`) | M | S | filter |
| Plain search mixing quoted and bare words | Searches all words | **Partial**: rejected as a syntax error (`filter.rs:54`) | L | S | filter |
| AND/OR precedence | Undocumented | **Unverified**: AND binds tighter | L | S | filter |
| Find dialog regex | Mp3tag's regex engine | **Partial**: JavaScript `RegExp` (`src/find.ts:12`), not the Boost bridge | L | S | find |

Implemented as documented: 61 of 64 functions (all except the three above), 50 placeholders, every filter operator (HAS, IS, MATCHES, PRESENT, MISSING/ABSENT, GREATER/LESS/EQUAL with the caveat above, AND/OR/NOT, parentheses, quoting, `*`, format-string operands, auto-apply), and the regex dialect through vendored Boost.Regex with `\u\l\U\L\E` replacement case conversion. `$sortvalues`, `$meta_test`, `$abbr` and `$fmt` are not in the 3.36.1 docs.

## Actions

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| X1. Case-only renames (every rename path) | Changing only the letter case renames the file or folder | **Broken on case-insensitive volumes**: `rename_no_clobber` hard-links and gets "exists" (`crates/tag-core/src/operations.rs:88`); Tag→Filename (`src-tauri/src/lib.rs:773`), Filename→Filename (`src-tauri/src/converters.rs:342`) and folder renames (`src-tauri/src/action_host.rs:312`) refuse it as an existing destination | H | M | actions/case-conversion, converters/rename-files |
| Case Conversion `_FIELDNAME` | Changes the case of APEv2/Vorbis field names | **Missing in practice**: tag-kind check never matches (`crates/tag-core/src/actions.rs:413` vs `technical.rs:287`); case-only key changes are not detected (`action_host.rs:116`) | M | M | actions/case-conversion |
| `_FILENAME` as Format Value / Guess Values / Tag→Tag target | Renames like Tag→Filename | **Partial**: plain-text evaluation (`actions.rs:739`) | M | S | actions/format |
| Export Cover extension | Always adds the image type's extension | **Partial**: only when the name has none; webp/bmp become `.img` (`action_host.rs:365`) | M | S | actions/export-cover |
| Adjust Cover max. size | Smaller covers are left alone | **Partial**: square/crop enlarge them (`crates/tag-core/src/cover_ops.rs:187`) | M | S | actions/adjust-cover |
| Adjust Cover min. size | Smallest side matched | **Partial**: only enlarges (`cover_ops.rs:190`) | L | S | actions/adjust-cover |
| Adjust Cover "Original" format | Keeps the cover format | **Partial**: GIF/BMP/WebP error; unchanged covers re-encoded (`cover_ops.rs:240`) | L | S | actions/adjust-cover |
| Convert Codepage list | System code pages | **Partial**: 32 pages; OEM 437/850/852 missing (`src/action-ui.ts:65`) | L | M | actions/convert-codepage |
| Import Text File | Reads a text file into a field | **Partial**: UTF-8 and BOM UTF-16 only (`action_host.rs:430`) | L | S | actions/import-text |
| Remove Fields Except | Removes all other fields | **Partial**: unmapped frames (PRIV, GEOB, chapters) survive (`actions.rs:858`) | L | M | actions/remove-fields-except |
| Split Field | Splits and trims | **Unverified**: keeps empty pieces (`a;;b`) | L | S | actions/split-field |
| `.mta` import/export | Exchange groups with Mp3tag | **Partial**: verified IDs 1, 2, 5, 7, 8, 9, 10, 16 and case mode 1 only | M | M | actions |
| Actions (Quick) | Reopens on the last type and settings | **Partial**: always Format Value with defaults (`src/action-ui.ts:184`) | L | S | menus/actions |
| Actions on mobile provider documents | n/a | **Partial**: file, cover-file and rename actions refused (`src-tauri/src/managed_actions.rs:28`) | L | L | actions |

All other options of the 18 types are implemented, including Replace whole-word/case/`_ALL`, Regex replace, Guess Values with `%dummy%`, Import/Export cover options, Set Cover Properties, Merge/Remove duplicates, Remove fields, Generate Playlists and Export as actions, groups with `#` submenus, `&` mnemonics, sorting and Repeat. `docs/verification/actions-audit.md` is stale on four rows: Repeat now exists; `_FIELDNAME`, Export Cover extension and Adjust Cover sizing are marked yes but are partial as above.

## Converters and auto-numbering

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| Case-only renames | — | See X1 | H | M | converters/rename-files |
| Folder separator replacement in Tag→Filename | Configurable string, may be empty | **Missing**: hard-coded `_` (`crates/tag-core/src/script.rs:434`) | M | S | customization/options/files |
| One clashing destination in Tag→Filename | That file is skipped and reported | **Partial**: the whole batch stops (`src-tauri/src/lib.rs:760`) | L | M | converters/rename-files |
| Filename→Filename with an extension in the new pattern | Undocumented | **Partial**: extension appended again | L | S | converters/rearrange-filenames |
| Text File→Tag source as a format string | e.g. `%_folderpath%list.txt` | **Missing**: literal path or pasted text (`src-tauri/src/converters.rs:507`) | M | S | converters/import-tags-from-text-files |
| Text File→Tag line count mismatch | Applies records in order | **Differs by design**: any mismatch refuses the import | L | M | converters/import-tags-from-text-files |
| Ctrl+Shift+K / per-directory counter reset | Track counter restarts in each folder | **Missing**: Shift ignored; emulation via disc-per-folder has no per-folder totals | M | S | converters/auto-numbering |
| Wizard remembers options | Persisted | **Missing** | L | S | converters/auto-numbering |

## Export, playlists, covers, utils, find, manage list and CLI

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| `$filename(…,ansi)` | ANSI output | **Partial**: fails off Windows (`crates/tag-core/src/export.rs:1119`) | M | S | export |
| Export configurations as editable files in the export folder | New files show up | **Partial**: UUID file names, only ids in `exports.json` load (`src-tauri/src/exports.rs:158`) | M | M | export |
| `%_md5audio%` in exports | Audio MD5 | **Partial**: export stops with an error | M | M | export, format |
| Open file after export | Offered after export | **Partial**: only when exactly one file was written (`src/main.ts:199`) | L | S | export |
| Rename configuration in the list | F2 or delayed click in place | **Partial**: separate name field (`src/export-ui.ts:93`) | L | S | export |
| Bundled templates | A set of ready-made templates | **Partial**: Text and Cue sheet only (`exports.rs:44`) | L | S | export |
| Per-directory export with an output folder | — | **Partial**: keeps only the file name | L | S | export |
| Manual order (Alt+drag, Alt+arrows) for playlists | Sets playlist order | **Missing** | M | M | playlists |
| Cue sheets | Loads the referenced files | **Partial**: `FILE` lines only; no per-track entries, TITLE/PERFORMER ignored (`crates/tag-core/src/playlist.rs:21`) | L | M | playlists |
| Entries relative to the working directory | Relative to the working folder | **Partial**: relative to the playlist folder (normally the same) | L | S | options/playlist |
| Adjust cover inside Manage Covers | Per-cover resize/re-encode | **Missing** (`src/covers.ts:447`) | L–M | M | manage-covers |
| Cover-art directory for Add cover | Option under Directories | **Missing** (`src/options-ui.ts:117`) | L | S | options/directories |
| Cover drag-out | Drag a cover to the desktop | **Missing**: drop-in only | L | M | getting-started/main-window |
| Find history | Remove one or all terms via ► | **Partial**: Clear history only; not persisted (`src/find.ts:46`) | L | S | find |
| Manage List favorites | Ticked entries become favorites in ► | **Missing** (`src/helper-menu.ts:162`) | L–M | S–M | manage-list |
| Manage List duplicate | Copy an entry | **Missing** | L | S | manage-list |
| History storage | Part of the configuration | **Partial**: WebView localStorage, not in config backup | L | S | manage-list |
| macOS launch forwarding | n/a (Windows CLI) | **Missing**: Finder "Open With" and `open --args` to a running app are not handled (no `RunEvent::Opened`) | M | M | command-line |

OK: all 8 Utils items (Remove tag types, Remove unsupported fields, Optimize FLAC/MP4, Strip chapters MP4, each journaled), every CLI switch (`/fp:`, `/fn:`, `/add`, forwarding), every Find match mode, MTE `$loop` sorting/nesting, `$put(s)`/`$get`, totals and relative placeholders, BOM/append/partition, playlist options (extended info, filename, confirm dialog, auto-create after save/rename, no-directory creation), Manage Covers aggregate/apply-all/reorder/replace, and the full cover context menu.

## Options pages and UI customization

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| Tags → Mapping (rules per APEv2/ID3v2/MP4/Vorbis/WMA, read rename + reverse on write, many-to-one) | User field mappings | **Core-only**: `UserFieldMapping` in `crates/tag-core/src/mapping.rs:28`; empty rules at `src-tauri/src/settings.rs:314,317,320`, `src-tauri/src/lib.rs:465`, `src-tauri/src/utilities.rs:727` | H | M | options/tags/mapping |
| Tags → save when using arrow keys / single click | Saves the panel on selection change | **Missing** | M–H | M | options/tags |
| Tags → save mode in Extended Tags | Ask / ignore / save when moving between files | **Missing** (no navigation either) | L | S | options/tags |
| Tags → don't show first folder image | Folder-image fallback in the cover pane | **Missing**: no fallback exists | M | M | options/tags |
| Advanced → list chapters as separate files | Chapters as rows | **Missing** | M | L | options/tags/advanced |
| Advanced → single MP4 atom for multiple values | Toggle | **Partial**: always one atom (`mapping.rs:2246`) | L | M | options/tags/advanced |
| Advanced → reuse unmodified MP4 atoms / ID3v2 frames | Toggles | **Partial**: always on, no toggle | L | S | options/tags/advanced |
| Files → replace folder separator | Configurable | **Partial**: fixed `_` | M | S | options/files |
| Directories → cover art directory | Default folder for Add cover | **Missing** | L | S | options/directories |
| Library (enable, include directories, cleanup) | Metadata database for large collections | **Missing**; no recorded decision to exclude | M | L | options/library |
| Language | UI translations | **Missing**; needs an OpenTag translation layer (Mp3tag's language files are proprietary) | M | L | options/language |
| General → tray icon, minimize to tray | Notification area | **Missing**; Windows-centric, a product decision | L | S–M | options/general |
| Field Values in Extended Tags | Auto-complete there too | **Partial**: File List and Tag Panel only | L | S | options/field-values |
| File List: random sort (icon column), auto-size, manual reorder, Mp3tag column file import | — | **Missing / partial**: OpenTag JSON import/export only (`src/columns.ts:257`) | M | S–M | customization/file-list |
| Tag Panel: right-click menu, auto default values on multi-select, typing a path into `_DIRECTORY`, Ctrl+B, dock/undock, Mp3tag config import | — | **Partial**: ⋯ menu, explicit "Default values" command, folder list, Discard without key; fixed left panel (`src/tag-panel.ts:210`, `src/main.ts:119`) | L–M | S–L | customization/tag-panel |
| Main window: toolbar status-bar hints, toolbars menu/customize, F3 toggle, Ctrl+Tab, Sort by submenu, Ctrl+drop vs. drop replace | — | **Partial / missing** | L–M | S–M | getting-started/main-window |
| Extended Tags: title with format and file name, `<<`/`>>`, OK writes immediately, field drop-down, cut/copy/paste fields, cover art, auto-complete, Alt+T | — | **Partial / missing** (`src/main.ts:982-1015`); OK stages edits by design | H | M | extended-tags |
| Networking proxy credentials, SOCKS, PAC | — | **Partial** (general-options gaps) | L | S | options/networking |

OK: General (title, next file, auto-completion, natural sort, select added, keep moved), all 11 Messages, Files (extensions, timestamps), Directories (subfolders, favorites, startup), Appearance (color mode, toolbar/UI scaling, grid lines), all 13 Mpeg and 7 Ape/Mpc controls, Advanced (comment language, POPM, Matroska video), Tag Panel page, Field Values (lists, prefer tag values, user genres, import/export), Tools (99 tools, Ctrl+1…0, placeholders, no shell), Networking (timeout, proxy modes), Updates (startup check, betas, release notes).

## Tag sources

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| `.src` web-source framework | User-installed script sources, `#` submenus, `&` accelerators | **Missing**. Optional large work, not out of scope: the DSL is publicly documented, so a clean-room interpreter is legitimate. It needs format-string emulation (partly in `script.rs`), HTML scraping and per-script host consent, because `src-tauri/src/sources/http.rs:42` allows only trusted hosts | M | L | tag-sources/development, tag-sources/archive |
| freedb remote (gnudb) | Works out of the box | **Partial**: gnudb answers "Unknown application" until OpenTag is registered (ignored live test) | M | S (registration) | options/tag-sources/freedb |
| freedb server list | Named servers plus a server-list button | **Partial**: one server and defaults | L | S | options/tag-sources/freedb |
| freedb by audio CD / free text | — | **Partial**: files and disc ID only | L | M | menus/tag-sources |
| Local freedb text search | Needs a user-built index | **Missing** | L | M | options/tag-sources/local-freedb |
| Discogs authorization | OAuth plus a verifier code | **Partial**: personal token stored in plain text (`src-tauri/src/sources/credentials.rs`), not the OS keychain | L | S (keychain) | tag-sources/discogs-auth |
| Ctrl+I / Ctrl+Shift+I last source | Any source; quick variant | **Partial** | M | S | tag-sources/import |
| Review step field multi-select | Click applies to every selected field; Ctrl+A/Ctrl+Shift+A | **Missing** | L | S | tag-sources/import |
| Track↔file drag and drop; Alt+PgUp/PgDn | — | **Partial**: buttons and Alt+↑/↓ only | L | S | tag-sources/import |
| "Suggest sorting" as persistent checkboxes | — | **Partial**: one-shot buttons | L | S | tag-sources/import |
| Discogs Release ID as its own menu item | — | **Partial**: mode inside the Discogs dialog | L | S | tag-sources/import |
| Saved cover files and undo | — | **Partial**: cover files written next to the audio are not undone | L | S | tag-sources/import |

OK (20): search by artist/album, text, ID/URL, barcode and catalog number; scored, paged results with preview; per-field checkboxes and editable values; auto-match and By title/By length; cover choice, replace and save-to-file; one undo step; cover-only providers; MusicBrainz cover size; freedb remote and local options; disc ID copy to COMMENT/DISCID.

## File formats and mapping

| Item | Mp3tag | OpenTag state | Impact | Size | Docs |
|---|---|---|---|---|---|
| WAV RIFF INFO | Read, write, remove with ID3v2 | **Partial**: only updates an existing INFO chunk; unmapped fields dropped silently (`crates/tag-core/src/tags.rs:1191,1384`) | M | S | file-formats |
| AAC (ADTS) APE tags, ID3v1 creation, Remove tag types | Reads ID3v1/2 + APEv1/2; writes ID3v1/2 + APEv2 | **Partial**: no APE (lofty), no policy, Utils rejects AAC (`crates/tag-core/src/tag_types.rs:36`) | L | M | file-formats |
| WavPack ID3v2 removal | Removes ID3v1, ID3v2, APE | **Partial**: ID3v2 on WV is not parsed, so it cannot be removed | L | S | file-formats |
| Chapters as separate files; MP4 chapter writing | — | **Missing**: MP4 chapters read and strip only | M | L | options/tags/advanced |
| TAK and OptimFROG | Supported formats | **Implemented on a stand-in**; only hand-built fixtures, no real-encoder files | L | S (fixtures) | file-formats |
| Matroska nested chapters and Segment Info title | — | **Partial**: nested chapters flattened on edit; title not shown | L | M | file-formats |
| ID3 edge cases | — | **Partial**: bare `UNIQUEFILEID` write refused, USLT descriptions not representable, compressed/encrypted frames refused | L | S–M | mapping |
| MP4 freeform | — | **Partial**: legacy 8-byte `plID` rejected | L | S | mapping-table |
| ASF | — | **Partial**: legacy `WM/Track` (0-based) not mapped to TRACK | L | S | mapping-table |

The mapping registry matches all 118 rows of the mapping table (92 common, 24 MP4-only, 2 MKV-only) cell for cell. Every format on the `file-formats` page reads and writes its documented tag kinds except the rows above. DFF and AIFF text chunks go beyond the reference.

## Out of scope

| Item | Reason |
|---|---|
| Windows Explorer shell extension, context menu, Ctrl-drag from Explorer | Windows shell integration; OpenTag uses the CLI forms and single-instance forwarding |
| Help → Donation, Mp3tag website/forum targets | Belong to the original vendor; OpenTag links its own repository and issues |
| Bundled Mp3tag `.src`/`.inc` scripts and the web-sources download service | Proprietary content. The interpreter itself is optional work (see Tag sources) |
| Mp3tag language files and Appearance → Symbols icon set | Proprietary assets; a translation layer of OpenTag's own is a separate product decision |
| Audio-CD lookup for freedb, gnudb web text search | Optical-drive I/O per platform; gnudb's web search sits behind a browser check |
| Discogs OAuth with an embedded consumer secret | An open-source app cannot keep the secret; personal tokens are the supported path |
| ANSI `.m3u` on macOS/Linux | No system ANSI code page; UTF-8 is written (the export fallback above is a separate gap) |
| Alt+Enter → system Properties | Windows-only dialog |
| About → portable/Store install type | Windows packaging detail |

## Stale documentation found during the audit

- `docs/feature-inventory.json` `shortcuts`: rows 2–55 and 60–97 still say no handler exists.
- `docs/verification/actions-audit.md`: Repeat, `_FIELDNAME`, Export Cover extension, Adjust Cover sizing (see Actions).
- `docs/verification/tag-preferences.md` and `options-utils.md` gap lists: General extras, Appearance scaling, Messages and Utils are now done; TTA and Matroska mapping are supported.
- `docs/CRITICAL-MAPPING.md`: ASF and Matroska are no longer registry-only; the POPM preference exists.
- `docs/CRITICAL-TECHNICAL.md`: `%_subsong_count%` is produced for MP4/MKV.
- `docs/FEATURE-INVENTORY.md`: the Ctrl+O contradiction is resolved (Options, as documented).

## Closed after this audit (batch 5, 2026-10-15)

Closed by implementation and unit tests. Their native suites run headless in CI. None of these has differential evidence against the original.

| Top-25 item | Closed by | Evidence |
|---|---|---|
| 1 case-only renames; 13 `_FIELDNAME`; 14 `_FILENAME` filename mode; 23 folder-separator option; X1 | rename-fixes | [verification](../verification/rename-fixes.md) |
| 2 Remove tag follows options / Ctrl+R; 3 Options → Tags → Mapping; 24 WAV RIFF INFO | remove-mapping | [verification](../verification/remove-mapping.md) |
| 4 Extended Tags parity; 5 macOS Option-key swallowing; 6 missing shortcuts; 21 Ctrl+Shift+K | keyboard-extended | [verification](../verification/keyboard-extended.md) |
| 7 save on move; 8 manual reordering; 9 folder-image fallback; 12 artwork in copy/paste; column auto-size | list-behaviour | [verification](../verification/list-behaviour.md) |
| 10 `$regexp` case; 11 numeric filter compare; 15–17 placeholders, `$filename(…,ansi)`; 18 dropped `.mte`; 19 `.mta` for all types; 20 Export/Adjust Cover; 22 Text File → Tag source format string | script-export | [verification](../verification/script-export.md) |

The keyboard reference now has 60 rows OK, 3 partial and 13 missing (was 53 / 10 / 34). Still open from the top 25: 25 (the `.src` web-source interpreter), and the runners-up such as Library, translations, Finder "Open With", and listing chapters as separate files.
