# Web sources (`.src`) verification

OpenTag runs user-installed web source scripts (Mp3tag's Web Sources Framework)
with a clean-room interpreter written from the public description of the file
format and parser commands. No Mp3tag or community `.src`/`.inc` script is
included or used in tests; every test script was written for OpenTag.

Code: `src-tauri/src/sources/web/` (`source.rs` file format, `engine.rs`
parser language, `mod.rs` folder, commands, results and releases),
`src-tauri/src/sources/http.rs` (page download, charset decoding, address
registry), `src/web-sources.ts` (menu, access keys, settings dialog) and the
web branch of `src/tag-sources.ts` (search fields, results, shared review).

## Installing and finding sources

| Behaviour | Implementation | Evidence |
|---|---|---|
| Sources folder | `<configuration folder>/data/sources`, as in Mp3tag; created on demand | `web_sources_list`, e2e checks the folder |
| Discovery | Every `*.src` file (sorted ignoring `&` and case); `*.settings` files become configuration entries; unreadable files are listed as problems with the reason (menu note with a tooltip) | `listing_menu_names_problems_and_settings` |
| Install | Tag Sources → Install tag sources… copies chosen `.src`, `.inc`, `.settings` files (max 4 MB each) into the folder; other files are refused | `install_copies_only_source_files`, e2e |
| Open folder | Tag Sources → Open sources folder | menu item |
| Refresh | After installing (event `opentag://web-sources`), at start-up and whenever the window regains focus, so files copied by hand appear | e2e waits for the rebuilt menu |
| `#` submenus | File name parts before `#` are nested submenus | `web sources form # submenus …` (UI test), e2e |
| `&` access keys | Underlined in the menu; typing the keys while Tag Sources is open walks submenus and runs the source; `&&` is a literal `&` | UI test, e2e presses F then W |
| Last used source | Ctrl+I repeats the last tag source, including web sources (shortcut shown on its item) | e2e |
| Debug output option | Options → Tag Sources → Write debug output for web sources: every search and album run writes the downloaded page (`<source>-<index|album>-input.out`) and a step trace (`<source>-<index|album>.out`) to `<sources>/debug` | `search_and_album_scripts_run_against_local_pages`, e2e checks the error message names the debug folder |
| Errors | `<file> [ParserScriptIndex|ParserScriptAlbum] line N (<command>): <reason>`, plus the debug folder when output was written; compile errors (unknown command, parameter count, unbalanced blocks) are reported the same way before the script runs | `compile_reports_…`, e2e |

## Description file keys

| Key | Behaviour | Evidence |
|---|---|---|
| `[Name]` | Source name (falls back to the menu item name) | `keys_scripts_and_search_fields` |
| `[BasedOn]` | Read and kept | same |
| `[IndexUrl]` | `%s` replaced by the encoded search; appended when there is no `%s` | same |
| `[AlbumUrl]` | Prefix for `%_url%` of a result (absolute `%_url%` values are used as they are; relative ones resolve against the results page). Without an index step it is the album address with `%s` for the search | `index_output_becomes_search_results`, `defaults_validation_and_direct_sources`, e2e direct lookup |
| `[WordSeparator]` | Joins search words; the misspelt `[WordSeperator]` used by many scripts is accepted too; default `%20` | `keys_scripts_and_search_fields` |
| `[IndexFormat]` | Splits each output line of the index script at `|`; `%_url%` is required; `%_coverurl%` is the result thumbnail; `%_preview%` (if present) is the Preview page; other `_` fields are hidden; the rest are result columns | `index_output_becomes_search_results`, e2e |
| `[SearchBy]` | Up to three `Label||%format%||address part` triples joined by `$$`; each field is pre-filled by evaluating its format string for the first selected file with OpenTag's scripting engine; each value is encoded and placed in its address part (`%s`), the parts are concatenated into `[IndexUrl]` | `keys_scripts_and_search_fields`, e2e |
| `[Encoding]` | `url-utf-8` (default), `url`, `utf-8`, `iso-8859-1`, `ansi` (Windows-1252); unknown values are an error | `words_are_encoded_per_encoding` |
| `[UserAgent]` | `1` sends OpenTag's own user agent; otherwise a browser-compatible one naming OpenTag | `web_pages_are_fetched_with_redirects_and_user_agent` |
| `[ParserScriptIndex]` / `[ParserScriptAlbum]` | Multi-line scripts after `=...`; a one-line script after `=` also works | `defaults_validation_and_direct_sources` |
| `[Include]` | Included files (from the sources folder) replace the including file's keys, nested up to 8 levels; loops and missing files are reported | `includes_replace_keys_and_loops_are_reported`, e2e (`Fixture.inc`) |
| `[MinAppVersionWin]` / `[MinAppVersionMac]` | Read; not enforced (Mp3tag version numbers do not apply to OpenTag) | `SourceFile` fields |
| `[Settings]` | Names the `.settings` schema whose values `IfVar`/`IfNotVar`/`SayVar` read | `settings_merge_defaults_validate_and_reach_scripts`, e2e |

## Configuration settings (`.settings`)

The schema (`key`, `title`, `settings` of type `string` with optional
`choices`, `number`, `bool`, `heading`, `separator`, and `localizations` by
ISO 639-1 language) generates a dialog reached from the menu entry named by the
file (`My Source#Settings.settings` → My Source › Settings…, after the
sources). Values are stored per schema key in `<sources>/settings.json`,
other sources' entries are kept, missing or mistyped values fall back to the
defaults. Evidence: `settings_merge_defaults_validate_and_reach_scripts`,
`settings dialogs are generated …`, `settings are read back …`, e2e.

## Parser commands

Commands are case-insensitive. Parameters are quoted strings (`\"` is a quote)
or bare numbers; `#` starts a comment. `%name%` in a parameter is the content
of that output (or `CurrentUrl`); other percent signs stay. Unit tests are in
`src-tauri/src/sources/web/engine.rs`.

| Command | Implemented behaviour | Test |
|---|---|---|
| FindLine S n n | Nth line containing S from the cursor (the rest of the current line counts); cursor to its start; error unless the 3rd parameter is 1 | `find_and_move_commands_position_the_cursor` |
| FindLineNoCase S n n | Same, ignoring case | same |
| FindInLine S n n | Nth occurrence in the current line from the cursor; cursor after it | same |
| GotoChar N | Nth character (1-based) of the line | same |
| GotoLine N | Nth line (1-based) | same |
| MoveChar N | N characters right/left, clamped to the line | same |
| MoveLine N n | N lines down/up; leaving the input is an error unless n is 1 (then clamped) | same |
| Say S | Appends S | `say_commands_read_from_the_cursor` |
| SayUntil S | Up to S in the line (rest of line if absent); cursor at S | same |
| SayUntilML S | Up to S across lines (lines joined by CR LF) | same |
| SayRest | Rest of the line | same |
| SayNChars N | Next N characters | same |
| SayNextNumber | Next number in the line (sign and decimals) | same |
| SayNextWord | Next whitespace-delimited word | same |
| SayOutput S | Content of output S (`CurrentUrl` is the page address) | `outputs_set_and_conditions` |
| SayNewline | CR LF | `say_commands_read_from_the_cursor` |
| SayDate s | Number at the cursor as Unix seconds (or `ms`) → `YYYY-MM-DD` (UTC) | `dates_durations_and_regular_expressions` |
| SayDuration s n n | Number as seconds/`ms`/`m` → `m:ss`, `h:mm:ss` with n=1, `.mmm` with the 3rd n=1 | same |
| SayRegexp S s s | All matches (Boost Perl syntax, as in OpenTag's `$regexp`) in the rest of the line, joined by s (space); with a 3rd parameter only up to it, nothing when it is missing | same |
| SayFormat S n | Format string with outputs as fields and `%_current%` (whole input); n=1 offers only `%_current%`; `$verticalBar()` is a literal pipe | `formats_and_output_references` |
| SayVar S | Configuration value | `configuration_variables` |
| Set S s | Sets or clears an output | `outputs_set_and_conditions` |
| SkipChars S | Skips characters contained in S | `say_commands_read_from_the_cursor` |
| Insert S | Inserts S at the cursor | same |
| Use S | S becomes the input | `input_transformations` |
| UseFormat S n | Formatted text becomes the input | same |
| If / IfNot S | Input at the cursor starts / does not start with S | `outputs_set_and_conditions` |
| IfOutput / IfNotOutput S | Output has / lacks content | same |
| IfGreater / IfLess N | Number at the cursor compared with N (false without a number) | same |
| IfVar / IfNotVar S S | Configuration value equals / differs (case-insensitive) | `configuration_variables` |
| Else / Endif | Branches; nesting supported | `outputs_set_and_conditions` |
| OutputTo S | Current output buffer (names case-insensitive) | same |
| Do … While S n | Repeats while the input at the cursor starts with S, at most n times | `do_while_loops_with_limits` |
| Replace S S | All occurrences in the whole input | `input_transformations` |
| RegexpReplace S S | Regular expression over the whole input (`$1` references) | same |
| JoinUntil S | Joins following lines up to the one containing S | same |
| JoinLines N | Joins the next N lines (-1 or too many: all) | same |
| KillTag S s | Replaces HTML tag S (`*` any) in the line by s | same |
| Trim S | `off` stops trimming text read by SayUntil/SayUntilML/SayRest | `say_commands_read_from_the_cursor` |
| Unspace | Trims the current line | `input_transformations` |
| Debug S s n | `on` writes a step trace to file s (default `debug.out`) up to n MB in `<sources>/debug`; `off` closes it | `debug_traces_and_input_dumps` |
| DebugWriteInput s | Writes the current input to s | same |
| json S s | JSON mode on/off; `current` parses the transformed input instead of the download | `json_mode_errors_and_current_input` |
| json_foreach / json_foreach_reverse S | Iterates the array S (input = its size); empty arrays skip to json_foreach_end | `json_selection_and_iteration` |
| json_foreach_counter n | 1-based counter, zero-padded to n digits | same |
| json_foreach_end | Next element or end of the loop | same |
| json_select_object S | Selects object S (input = S, empty when missing) | same |
| json_unselect_object | Returns to the previous object | same, `json_mode_errors_and_current_input` |
| json_select S | Value of S on the input (dotted paths when no key matches) | same |
| json_select_array S n s | nth element (1-based); -1 joins all with s | same |
| json_select_many S S s s n n | Field of every array element joined by s, the 4th parameter before the last element, at most n items, empty values with the 6th = 1 | same |
| json_select_many_count S s n | Number of elements, or of elements with field s (empty included with n=1) | same |

Blocks are checked when a script is compiled. A run stops after 2,000,000
commands (endless `Do` loops) or 16 MB of output.

## Outputs to releases

- Index script: the default output; one result per line.
- Album script: an output containing `|` holds one value per track
  (`Title 1|Title 2|`); others apply to the album. `\\` separates multiple
  values. `COVERURL` lists cover images (`|`-separated, relative addresses
  resolve against the page); `_LENGTH` gives track lengths (`m:ss`, `h:mm:ss`
  or seconds); other outputs starting with `_` are not written. `TRACK` and
  `DISCNUMBER` give the track and disc used for matching files. A release with
  only album values (for example a covers-only source) applies them to every
  selected file.
- Results go through the same review as MusicBrainz and Discogs: field
  selection and editing, track ↔ file matching, cover choice, Save image to
  file, Preview, one undo step.

Evidence: `album_outputs_become_a_release`,
`search_and_album_scripts_run_against_local_pages` (local HTTP server, HTML
index + JSON album, debug files), e2e.

## Network rules

Web sources may contact any HTTP or HTTPS address their script produces
(no credentials in addresses, redirects only to HTTP/HTTPS, 8 MB per page,
Options → Networking proxy and timeout). Responses are decoded by their
declared charset, a byte order mark, a `<meta>` charset, UTF-8, or else
Windows-1252. Cover thumbnails, covers and Preview pages are fetched or opened
only for addresses a web source returned in this session (the webview cannot
request other addresses). Scripted native test runs with
`OPENTAG_E2E_OFFLINE` may only reach loopback addresses.
Evidence: `web_addresses_are_checked_and_registered`,
`web_pages_decode_by_declared_or_sniffed_charset`,
`web_pages_are_fetched_with_redirects_and_user_agent`.

## Native end-to-end suite

`tests/e2e/web-sources.js` with `tests/e2e/web-sources-fixtures.sh` runs
headless in CI (`native-e2e` job): a local site on 127.0.0.1:18765, test
sources installed through `web_sources_install`, the Fixture submenu with
access keys, the settings dialog, search with pre-filled fields, result
columns and thumbnail, review, import of fields and cover to two MP3 files,
the undo step, Ctrl+I, a direct lookup, and a failing script with debug output.

## Decisions and gaps

- Semantics the public description leaves open were chosen and are tested as
  listed above: FindLine also matches the rest of the current line;
  SayRegexp works on the rest of the current line; JoinUntil/JoinLines join
  without a separator; json_select_array counts from 1; the 4th
  json_select_many parameter separates the last element; a failing Find or
  MoveLine stops the script with an error unless its "no error" flag is set.
- `[SearchBy]` triples are separated by `$$`, as used by published scripts;
  the description does not spell out the separator.
- `[MinAppVersionWin]`/`[MinAppVersionMac]` are not enforced.
- There is no per-source host consent prompt: installing a script into the
  sources folder is the consent, as in Mp3tag.
- Mp3tag's online web sources archive/download service is not provided.
- The scripts' own `Debug` file names are reduced to plain file names inside
  `<sources>/debug`.
