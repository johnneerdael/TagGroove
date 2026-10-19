# General, Appearance, Networking and Updates options, remaining Messages, and Utils

Scope: Options → General, Appearance, Networking and Updates; the Messages "at warnings from Actions" and "at importing covers"; Utils → Remove Unsupported Fields and Optimize; window state. References: `customization/options/{general,appearance,networking,updates,messages}`, `utils`.

## Implemented

**General** (`pages.general`, controls on the built-in General page in `src/options-ui.ts`, behaviour in `src/main.ts`)

| Option | Default | Behaviour |
|---|---|---|
| Automatically select next file when editing in file view | off | Enter in a File List cell commits it and opens the same column on the next file (`CellEditor` `advance`, move `'down'`). |
| Auto-completion at selection boxes | on | Typing at the end of a tag panel text field completes inline from its drop-down values (selection values, custom field values, genres); the completed rest is selected, so typing on replaces it. File List cells complete from custom field values and the values of loaded files. Deleting or pasting never completes. The field reports `aria-autocomplete="inline"`. |
| Select files automatically | off | Files added by a scan (open folder, add files, drag and drop, startup folder, command line) become the selection. |
| Keep moved files in the file list | on | When off, a file that a move or rename takes outside every listed folder leaves the list, unless it has pending edits. |

The defaults are a judgement call; the documentation does not state them. Completion is pure (`completion`, `completeInput` in `src/general-options.ts`).

**Appearance** (`pages.appearance`)
- Toolbar scaling 100–200 % (CSS `zoom` on the toolbar).
- User interface scaling 100–200 %: the webview zoom (`setZoom`, permission `core:webview:allow-set-webview-zoom`), so every layout computation stays in CSS pixels.
- Display grid lines: Off, Alternate row colors, Grid lines. Mp3tag's tri-state checkbox is a three-way choice here. Rows carry an `odd` class from their list index, so alternation survives virtual scrolling.
- The tag panel has a splitter (drag, or focus it and press ←/→). The width is kept in `localStorage` on this device, like the color mode.

**Window state** (`src-tauri/src/window_state.rs`). Size, position and the maximized state of the main window are saved to `window-state.json` in the config folder when the window closes and on exit, and restored at startup. A size is capped to the largest monitor; a position is dropped when the title bar would not be reachable on any connected monitor. The file is not part of configuration backups. Column widths were already persisted in `columns.json`.

**Networking** (`pages.networking`, validated in `settings.rs` as well as in the page)
- Timeout 5–300 s (default 30); the connect timeout is the smaller of it and 10 s.
- Proxy: system settings (default; reqwest's `system-proxy` feature reads macOS/Windows settings and the `HTTP(S)_PROXY` variables), no proxy, or a manual `host:port` / `http(s)://host:port` HTTP proxy. User names and passwords are refused because options are stored and exported as plain text.
- The tag source clients (MusicBrainz, Discogs, images, freedb) and the update check are built from these values and rebuilt when they change (`sources/http.rs` `builder`, `cached_client`).

**Updates** (`pages.updates`, `src-tauri/src/updates.rs`)
- Help → Check for updates… and Options → Updates → Check now… read `api.github.com/repos/johnneerdael/opentag/releases` (no redirects, 2 MB limit) and compare semantic versions. Drafts, unparsable tags and pages outside the repository are ignored; beta builds count only with "Also include development builds". When a newer beta is left out, the message says so.
- The dialog offers the release page, opened through `open_release_page`, which accepts only `https://github.com/johnneerdael/opentag/releases/…`. Nothing is downloaded or installed.
- Check for updates on startup (off): shows the dialog only when a newer release exists; failures go to the status bar.
- Display release notes for new versions (on): on the first start of a different version than the last one on this device (kept in `localStorage`, so startup never writes the options), the release page of the running version opens. A fresh install opens nothing.

**Messages** (`src/messages.ts`)
- At warnings from Actions (on): action group and quick action file errors are shown in the action dialog as before. Off: they are listed under the file list only.
- At importing covers (on): when a tag source import writes artwork and some target files already have embedded artwork, OpenTag asks whether to keep it. "Don't show this message again" stores the choice (`import_covers_choice`), which the Messages page shows while the message is off. Kept files receive no new cover (`ApplyCover.preserve_existing`, `cover_target` in `sources/mod.rs`); files without artwork get the import.

**Utils** (`crates/tag-core/src/optimize.rs`, `mp4_layout.rs`; command `apply_util`, journaled as one undo step; Tools menu and File List context menu)
- **Remove unsupported fields** (MP3, and ID3v2 in AAC, AIFF and WAV): removes the ID3v2 frames OpenTag shows neither as a field, cover nor chapter: unknown/binary frames (GEOB, SYLT, …; not IPLS or PCST), PRIV, RVA2, ETCO, OWNE, and UFID other than MusicBrainz. `%_id3v2_unknown_frames%` now lists exactly these. The tag keeps its revision and encoding; nothing outside the ID3v2 tag changes.
- **Optimize FLAC**: exactly 4096 bytes of padding as one last PADDING block; other blocks are kept verbatim. A region of the same size is rewritten in place (reported as a region write); otherwise the file is rewritten. The audio frames are compared with the original before the staged copy replaces it. ID3v2-prefixed FLAC files are refused.
- **Optimize MP4**: removes `free`, `skip` and `wide` boxes at every container level and moves `moov` before the first `mdat`. Every `stco`/`co64` offset is relocated; the output is re-parsed and must have identical media payloads and offsets that address the same bytes. Fragmented files, top-level `meta` and `iloc` boxes are refused, as is a 32-bit offset that would overflow.
- Files that need no change are not rewritten. Unsupported formats are skipped with a message saying what the util applies to. The confirmation is an in-app dialog.
- Strip chapters MP4 comes from `feat/core-parity` (formats-3); this branch's own variant was dropped in the merge. Optimize removes the free space it leaves.

## Tests

- Rust: `crates/tag-core/tests/utils_optimize.rs` (4): unsupported frames removed with fields, cover, MusicBrainz UFID, ID3v2.3 and every byte after the tag kept, second run untouched; ffmpeg FLAC from 8 KiB to 4 KiB padding and back after a hand-removed padding block, packet hashes (`framemd5`) unchanged; ffmpeg MP4 from `ftyp free mdat moov` to `ftyp moov mdat`, no `free` left, packets and chapters unchanged, second run untouched; format rejections. `mp4_layout` unit tests (2): synthetic file with nested free boxes and offset relocation; fragmented, truncated and free-space offsets refused. `updates.rs` (3), `window_state.rs` (3), `settings.rs` networking validation, `http.rs` client per proxy mode and a loopback proxy that receives `CONNECT musicbrainz.org:443`, `sources/mod.rs` preserved artwork, `utilities.rs` `apply_util`.
- Frontend: `tests/general-options.test.mjs` (7): completion, General/Appearance fallbacks equal the page defaults, number control markup, Networking validation, folder containment, update messages and release-note rules, new message defaults, Utils targets and prompt. `options-utils.test.mjs` and the `document-coordination` harness were updated for the new message choice and `movedOut`.
- Totals after merging `feat/core-parity` (23ed6da): Rust 494, frontend 212, all passing. `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings` (local toolchain 1.95) and `tsc --noEmit` are clean.
- Native e2e `tests/e2e/general-options.js` (fixtures `general-options-fixtures.sh`) is in the CI `native-e2e` suite list. Its earlier macOS form passed 18/18 locally before native runs on desktops were stopped: select added files at startup, splitter width across loads, page defaults and timeout validation, alternate rows/grid lines/toolbar zoom 1.5/innerWidth 1240 → 992 at 125 %, tag panel completion on and off, cell completion and Enter to the next file, Optimize FLAC −4096 bytes and Undo, Optimize MP4 and an unchanged second run, Utils skipping MP3, action warnings shown and suppressed for a read-only file, keep moved files on/off with a rename outside the folder, a manual proxy at 127.0.0.1:9 failing MusicBrainz and GitHub requests, the live update check, and the live import-covers question keeping existing artwork and remembering the choice. The CI version (Ctrl on Linux, fixtures from a script, without the chapter step) has not been run locally; CI results are the evidence for it. `core-workflows.js` and `options-utils.js` passed 13/13 locally before the merge.

## Gaps

- **Window state** has unit tests only; restoring across a restart was not exercised natively.
- **Notification area** options and **Symbols** (alternative icon set) are not implemented.
- **Proxy**: no credentials, no SOCKS. The system proxy relies on reqwest's detection (no PAC scripts).
- **Updates**: only GitHub releases of this repository; no download or installation. Release notes after an update are tracked per device.
- **Auto-completion**: PR64 adds automatic filtered whole-value choices alongside ordinary single-line inline completion in the panel, metadata cells and Extended Tags. See the [controlled-DOM verification](filtered-value-completion-2026-10-19.md) for exact staging, composition and accessibility-state controls. Native/original input, visual/screen-reader and large-list qualification remain open.
- **Optimize MP4** does not promote `stco` to `co64` and refuses fragmented files; **Optimize FLAC** refuses ID3v2-prefixed files.
