# Mp3tag 3.36.1 parity gaps

This ledger lists only what is still missing, partial or unverified. Anything not listed is implemented, with the evidence in the linked verification notes and the [feature inventory](FEATURE-INVENTORY.md). No area is certified as exact parity, because none has been compared run for run against the original program.

Reference target: `../Mp3tag.exe`, SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`. Current as of 2026-10-17 at `main` `9d3764b`. The item-level list with impact and size is the [remaining-gaps register](research/remaining-gaps.md).

| Area | Remaining gap |
| --- | --- |
| Toolbars | The original registers 75 toolbar command ids. OpenTag's custom toolbar offers 30 commands plus tools and action groups, so about 45 menu commands (copy/move/rename/delete files, most cover commands, selection commands, Utils, Help, file-list and configuration commands) cannot be placed on a toolbar. Movable toolbar bands are missing |
| Menus and keyboard | No File → Exit item (the macOS app menu quits). Alt+Enter opens Extended Tags instead of the Windows properties dialog |
| Tag panel | The floating panel is an in-window overlay, not a separate OS window |
| File table | Find uses JavaScript `RegExp`, not the Boost bridge. Mp3tag `columns.ini` import covers known keys only because the layout is undocumented |
| Scripting | Unverified against the original: `$caps` word-start characters, truth of non-boolean conditions, and errors inside a format string (OpenTag aborts the whole string). `%_length%` always uses `m:ss`, `%_tool%` reports only the Vorbis vendor, `%_app%` is empty in filters |
| Filters | A plain search that mixes quoted and bare words is rejected. AND/OR precedence and locale behaviour are unverified |
| Actions | Adjust Cover min size and "Original" format edge cases. Remove Fields Except leaves unmapped frames (PRIV, GEOB). Split Field keeps empty pieces (unverified). Convert Codepage starts at 1251 in action group dialogs instead of the last choice. File, cover-file and rename actions on mobile provider documents |
| Converters | Filename → Filename appends the extension again when the new pattern contains one. Text File → Tag refuses any record-count mismatch by design |
| File operations | Copy and Move do not reopen at the last destination folder. Associated `.lrc` and `.cdg` files do not follow a rename |
| Formats | TAK and OptimFROG run on a stand-in and are tested only with hand-built headers. Global Nero fields and Nero covers. MP4 cover type and description (format limit). No Mp3tag- or WMP-written fixtures |
| Covers | Original resampler, quality and rounding equivalence |
| Sources | Live-provider checks are bounded (one default-HTTPS gnudb query passed). Keyed services need a build or user key. `[MinAppVersion*]` is not enforced in `.src` scripts. No keychain on mobile. Audio-CD lookup, Discogs OAuth and the web-sources download service are out of scope |
| Settings | Only English, Dutch and German UI catalogs (the original ships 43 languages). Proxy credentials, SOCKS and PAC. Setting names found in the binary with no confirmed OpenTag equivalent: per-converter "show dialog" flags, `ToolOverwriteDefault`, `CoverPopup`, `HideID3Bar`, `ContextMenuDisabled` (Windows shell entry) |
| Undo/redo | History is not kept across sessions. Non-cloning volumes keep the 2 GiB full-size cap, and the first unpadded save counts in full |
| CLI and OS integration | `open -a … --args` cannot reach a running app on macOS. No Windows or Linux file associations. The Windows shell extension is out of scope |
| Platforms and distribution | Windows builds are unsigned, macOS builds are ad-hoc signed, iOS is simulator-only. Mobile needs real-device qualification, directory trees and cloud providers. Final dependency license audit and reproducible builds |
| Performance | No same-corpus comparison with the original, no Windows or Linux numbers, and APE, RIFF, Ogg and other containers still rewrite whole files |
| Native UI proof | The headless suites use synthetic DOM events, so OS input, IME and hit-testing are untested. Native screen-reader qualification is open |

Differential evidence against the original (field ordering and mapping, padding and reset edge cases, export sort, locale and encoding, query corpus) is open for every area.

## Beyond Mp3tag

- Runs on Windows, macOS, Linux, Android and iOS, with phone and tablet layouts and Android document access with save-back.
- Extra tag sources: iTunes, Deezer, LRCLIB lyrics, Last.fm, fanart.tv and AcoustID with fingerprints computed inside the app (no external `fpcalc`).
- A Tools menu, Load configuration, Previous/Next cover and a larger cover viewer.
- DFF/DSDIFF and AIFF text chunks beyond the documented format list.
- Language catalogs are plain JSON files, so users can add a language or override wording without a rebuild. The UI language switches live.
- Undo accounting is clone-aware on APFS, so a 100-file FLAC save stays one undoable step. FLAC and ID3v2/MP4 edits are written in place inside existing padding.
- Configuration backup and restore as one validated JSON bundle.
- Open source (MIT) with a published user guide at <https://opentag.neerdael.nl/>.
