# Remaining parity gaps versus Mp3tag 3.36.1

Current as of 2026-10-17 at `main` `9d3764b`. This register lists only open items. Everything the 2026-10-15 audit found that later batches closed has been removed; the closing evidence lives in the [verification notes](../verification/). The [parity ledger](../PARITY.md) summarises this list.

Method: the original audit compared every documentation page against the code. On 2026-10-17 the Windows binary's own tables were checked as well: script function names, placeholder names, field names, toolbar command ids and persisted setting names. Nothing was launched. "Unverified" means the behaviour is not documented and has not been compared with the original.

Legend: **Impact** H = daily tagging, M = regular, L = rare or legacy. **Size** S = hours, M = a day or two, L = a batch of its own.

## Open items

| Area | Item | Detail | Impact | Size |
|---|---|---|---|---|
| Toolbars | Toolbar command catalogue | The binary lists 75 toolbar command ids (`file_*`, `edit_*`, `cover_*`, `utils_*`, `help_*`, `view_*`, `convert_*`, `tagsources_*`). `TOOLBAR_COMMANDS` in `src/toolbar.ts` has 28. Not assignable: copy/move/rename/delete files, directory up, favorites, Open folder in file manager, copy format, select similar (and next), invert and unselect, cover add/delete/replace/save/adjust/description/cut/copy/paste, Utils commands, Help commands, customize columns and toolbars, random sort, save and open configuration folder, playlist selected and all-multiple, freedb and local freedb, quick tag source, properties | M | M |
| Toolbars | Movable toolbar bands | One fixed band per toolbar | L | M |
| Menus | File → Exit | No menu item (macOS quits from the app menu) | L | S |
| Menus | Alt+Enter | Opens Extended Tags; the original opens the Windows properties dialog (Windows-only) | L | S |
| Tag panel | Floating panel | In-window overlay, not a separate OS window | L | M |
| File table | Find regex | Uses JavaScript `RegExp`, not the Boost bridge (`src/find.ts`) | L | S |
| File table | `columns.ini` import | Known keys only; the file layout is undocumented | L | S |
| Scripting | `$caps`, `$caps2`, `$caps3` word starts | Only whitespace starts a word, so `o'brien (live)` becomes `O'brien (live)`. Original set unverified | M | S |
| Scripting | Truth of non-boolean conditions | Literal `1` is false; functions are true when an argument field exists (`script.rs`). Unverified | M | M |
| Scripting | Errors inside a format string | `$div(x,0)`, bad `$char` or an unknown function abort the whole string. The original probably degrades inline | L | S |
| Scripting | `%_length%` | Always `m:ss` (`75:03`) and truncates while `_length_seconds` rounds | L | S |
| Scripting | `%_tool%` | Vorbis vendor only; no encoder string for other formats | L | M |
| Scripting | `%_app%` in filters | Empty in the filter context | L | S |
| Scripting | `%_counter%` for tag-source cover names | 0 there (`src-tauri/src/sources/mod.rs`) | L | S |
| Filters | Mixed quoted and bare words | Rejected as a syntax error (`filter.rs`) | L | S |
| Filters | AND/OR precedence | AND binds tighter. Unverified | L | S |
| Actions | Adjust Cover min size and "Original" format | Min size only enlarges; GIF/BMP/WebP error; unchanged covers are re-encoded (`cover_ops.rs`) | L | S |
| Actions | Remove Fields Except | Unmapped frames (PRIV, GEOB, chapters) survive | L | M |
| Actions | Split Field | Keeps empty pieces (`a;;b`). Unverified | L | S |
| Actions | Convert Codepage default | Action group dialogs start at 1251 instead of the last choice (`src/action-ui.ts:285`) | L | S |
| Actions | `.mta` type IDs | Adjust Cover, Set Cover Properties and Generate Playlists export with OpenTag-only type IDs 1001 to 1003 (`crates/tag-core/src/actions.rs`); Mp3tag lists them as unknown actions | M | M |
| Actions | Mobile provider documents | File, cover-file and rename actions refused (`managed_actions.rs`) | L | L |
| Converters | Filename → Filename with an extension in the pattern | The extension is appended again | L | S |
| Converters | Text File → Tag record-count mismatch | Any mismatch refuses the import; the original applies records in order | L | M |
| File operations | Last Copy/Move destination | The folder picker does not reopen at the last destination (`src/file-ops.ts:186`) | L | S |
| File operations | Associated `.lrc` and `.cdg` files | Not renamed or moved with the audio file | L | M |
| Formats | TAK and OptimFROG | Implemented on a stand-in; only hand-built fixtures, no real-encoder files | L | S |
| Formats | Nero details | `udta/indx` kept but not shown or edited; covers always written to `covr`; a longer value for a Nero-held field in an MP4 file without chapters reaches only the iTunes tag ([formats-gaps](../verification/formats-gaps.md)) | L | S |
| Formats | ADTS and ID3 limits | ADTS after junk is found only in `.aac` files, within 64 KiB and after three frames. Compressed or encrypted ID3 frames cannot be kept in an unsynchronised tag or one with an extended header or footer, also for DSF and DSDIFF | L | M |
| Formats | MP4 cover type and description | Format limit | L | — |
| Formats | Reference files | No Mp3tag- or WMP-written fixtures | M | M |
| Covers | Resampler | Original resampler, quality and rounding equivalence unproven | L | M |
| Sources | Live-provider coverage | One default-HTTPS gnudb query passed; broader live read and artwork outcomes unverified ([gnudb](../verification/gnudb.md)) | M | S |
| Sources | Keyed services | AcoustID, Last.fm and fanart.tv need a build or user key | L | — |
| Sources | `.src` scripts | `[MinAppVersion*]` not enforced | L | S |
| Sources | Mobile | No keychain for the Discogs token | L | M |
| Settings | UI languages | English, Dutch and German bundled; the original ships 43 | M | L |
| Settings | Networking | Proxy credentials, SOCKS and PAC | L | S |
| Settings | Setting names in the binary with no confirmed equivalent | `ConvFTFShowDlg`, `ConvFTTShowDlg`, `ConvTFFShowDlg`, `ConvTTFShowDlg`, `ConvReplaceShowDlg` (per-converter "show dialog"), `ToolOverwriteDefault`, `CoverPopup`, `HideID3Bar`, `ContextMenuDisabled` (Windows shell entry). Feature meaning for the first four groups not documented | L | S |
| Settings | `ID3LeadingZeros` | The documented leading-zero option belongs to the numbering wizard; whether the binary also has a global switch is unknown | L | S |
| Undo | Persistence | History is not kept across sessions | L | L |
| Undo | Non-cloning volumes | 2 GiB full-size cap; the first unpadded save counts in full | L | M |
| CLI | `open -a … --args` | macOS drops arguments to a running app | L | — |
| CLI | File associations | Not registered on Windows or Linux | L | S |
| Platforms | Signing | Windows unsigned, macOS ad-hoc, iOS simulator-only | M | M |
| Platforms | Mobile | Directory trees, cloud providers and real-device qualification | L | L |
| Distribution | Audit | Final dependency license audit; reproducible builds | M | M |
| Performance | Comparison | No same-corpus comparison with the original; no Windows or Linux numbers; APE, RIFF, Ogg and other containers rewrite whole files | M | M |
| Native UI proof | Input | The headless suites use synthetic DOM events, so OS input, IME and hit-testing are untested; native screen-reader qualification open | M | L |
| All areas | Differential proof | No area has been compared run for run with the original (field ordering and mapping, padding and reset edge cases, export sort, locale and encoding, query corpus) | H | L |

## Out of scope

| Item | Reason |
|---|---|
| Windows Explorer shell extension, context menu, Ctrl-drag from Explorer | Windows shell integration; OpenTag uses the CLI forms and single-instance forwarding |
| Help → Donation and the vendor website and forum targets | Belong to the original vendor |
| Bundled Mp3tag `.src`/`.inc` scripts and the web-sources download service | Proprietary content |
| Mp3tag language files and the Appearance → Symbols icon set | Proprietary assets |
| Audio-CD lookup for freedb | Optical-drive I/O per platform |
| Discogs OAuth with an embedded consumer secret | An open-source app cannot keep the secret |
| ANSI `.m3u` on macOS and Linux | No system ANSI code page; OpenTag writes Windows-1252 when the text round-trips and falls back to UTF-8 otherwise |
| About → portable/Store install type | Windows packaging detail |

## Binary tables checked (2026-10-17)

Source: strings from `Mp3tag.exe`, compared with the code.

- Script functions: all 52 names in the function table exist in `crates/tag-core/src/script.rs`.
- Placeholders: every name in the placeholder table is produced (`src-tauri/src/evaluation.rs`, `crates/tag-core/src/technical.rs`, `mp4_chapters.rs`, `media_info.rs`).
- Field names: the 99 names in the Fields menu table are in the mapping registry.
- Toolbar command ids: 75 in the binary against 28 in OpenTag (first row above).
- Persisted setting names: about 60 checked; the unmatched ones are listed under Settings and File operations.
