# Playlists, cue sheets and export options

Scope: loading playlists and cue sheets, File → Playlist (all files / selected files), File → Generate Playlists, Options → Playlist, Options → Export, Export with Shift and opening the file after export. References: `playlists`, `customization/options/playlist`, `customization/options/export`, `export`, `actions/generate-playlists`, `menus/file`, `keyboard`, `format` (Playlist placeholders) and `command-line`.

## Implemented

**Loading playlists and cue sheets** (`crates/tag-core/src/playlist.rs`, command `read_playlist`)
- Formats: `.m3u`, `.m3u8`, `.pls` and `.cue`. The documentation lists M3U/M3U8 and cue sheets; PLS is an OpenTag addition.
- Entry points:
  - File → Load playlist/cue sheet…, which replaces the list and asks about pending edits first.
  - Dropping a playlist on the window.
  - Add files….
  - The command line: `/fn:list.m3u8`, a bare path, or a second launch that is forwarded to the running window. `/add` appends.
  - Folder loading is unchanged.
- **Order and duplicates:** files appear in playlist order. A file referenced twice is listed once, at its first position, as the documentation describes.
- **Entry forms:**
  - Relative entries are resolved against the playlist's folder.
  - Windows `\` separators are also tried as `/` on macOS and Linux.
  - `file://` URLs are percent-decoded. This covers `file:///C:/…`, `file://localhost/…` and UNC hosts.
  - Plain entries containing `%XX` escapes are tried both literally and decoded.
  - `..` is normalised.
- **Text encoding:** a byte order mark decides when present. Otherwise `.m3u8` is read as UTF-8, and other files are read as UTF-8 when the bytes are valid UTF-8 and as the ANSI codepage otherwise (Windows-1252 when no codepage is known).
- **Unresolvable entries:** missing files and stream URLs are listed in the error panel as `<playlist>: Line N: <entry>: file not found`. The status line counts the entries that were not loaded. All other entries still load.
- **Cue sheets:** loading a cue sheet lists one row per `TRACK` of a `FILE` that holds several tracks (an image file), with TITLE/PERFORMER/INDEX and the other cue fields; a `FILE` with one track loads that file itself. See [chapters.md](chapters.md).
- **List order:** loading a playlist clears the column sort, so the list shows the playlist order. **View → Unsorted (load and playlist order)** brings this order back after a header click. Without a sort, the list keeps the order in which files were loaded.
- **Working directory:** the loaded playlist provides `%_playlist_filename%`, `%_playlist_filename_ext%` and `%_playlist_folderpath%` to the playlist filename format. Its folder becomes the folder for relative playlist names.

**Writing playlists** (`render_playlist`, `src-tauri/src/playlists.rs`)
- **Format and encoding from the extension:**
  - `.m3u8` is written as UTF-8.
  - `.pls` is written as `[playlist]` with `FileN`, `TitleN` and `LengthN` entries, `NumberOfEntries` and `Version=2`, in UTF-8.
  - `.m3u` uses the Windows ANSI codepage. A path that the codepage cannot store is an error that suggests `.m3u8`; nothing is replaced with `?`. On macOS and Linux, `.m3u` is written as UTF-8.
  - Other extensions are written as UTF-8 M3U.
  - Lines end with CR LF.
- With **Write extended info** on, the playlist starts with `#EXTM3U` and each entry gets `#EXTINF:<seconds>,<format string>`. The default format string is `%artist% - %title%`. Unknown durations are written as `-1`.
- Paths are relative to the playlist's folder when the file and the playlist share a root. Targets are resolved through symlinks (macOS `/var` → `/private/var`) before relative paths are computed.
- A playlist that would replace an input audio file is refused. Generator partitions that map to the same file are refused before anything is written.
- **Don't create directories** turns a missing folder into an error. All of a batch's playlists are rendered before any of them is written.
- File → Playlist (all files) (⌘/Ctrl P) uses the files in the File List, after filtering, in list order. Playlist (selected files) uses the selected files.
  - The playlist name comes from Options → Playlist → Filename of playlist. A relative name is resolved against the loaded playlist's folder, or else the working directory, or else the first file's folder.
  - With **Show confirm filename dialog** on, a dialog shows the evaluated name. The edited name is used literally; placeholders typed there are not evaluated, as the documentation describes.
- **Generate playlists…** (⌘/Ctrl ⇧ P) acts on the selection, or on all files when nothing is selected.
  - It has fields for the playlist file name and the partition format, and a live preview of each playlist and its file count.
  - It remembers both patterns.
  - The Generate Playlists action now shares the same writer and Options → Playlist settings. Its previous output format (`#EXTINF` with the title, `/` separators, LF) changed to the options-driven format above.
- **Create playlist automatically:** after Save tags, or after any operation that renames files, OpenTag writes the "all files" playlist without dialogs.
- Options → Messages → "at exporting" now covers created playlists and offers to open a single written file. The old Create playlist… save dialog and the `export_playlist` command are replaced.

**Options → Playlist** (`pages.playlist`). The page holds only settings that change behaviour:
- Write extended info, and its format string
- Entries relative to the playlist folder
- Create playlist automatically
- Filename of playlist (validated as non-empty)
- Show confirm filename dialog
- Write BOM for UTF-8 playlists (the BOM option comes from the release notes)
- Don't create directories

**Options → Export** (`pages.export`)
- **Show selection dialog:** when off, File → Export (⌘/Ctrl E) runs the active configuration at once.
- **Don't create directories** and **Write BOM:** these replace the two checkboxes that used to be in the Export dialog. The Export dialog now points to Options instead. Values that were never saved on the page fall back to the earlier per-library preferences in `exports.json`.
- **Shift (⌘/Ctrl ⇧ E)** exports one file per directory. The partition is `%_folderpath%`. The file name is placed in each folder: the configured name, or the `$filename(...)` default of the configuration, or `export.txt`. This works with and without the dialog. The saved partition is not changed.
- **Open after export:** File → Export reports the written files (`ActionResult.outputs`). With Options → Messages → "at exporting" on, OpenTag offers to open a single output file.
- **Built-in "Cue sheet" export configuration.** OpenTag wrote this template; it is not an Mp3tag template. It writes `TITLE`/`PERFORMER` and one `FILE`/`TRACK`/`INDEX 01 00:00:00` block per file, in list order, with paths relative to the cue sheet. File → Load playlist/cue sheet reads it back. It appears for new configuration libraries; existing `exports.json` libraries keep their own list.

## Verification

- `AR=/usr/bin/ar cargo test --workspace`: 445 passed, 0 failed (baseline 427). The new tests cover:
  - M3U, PLS and cue parsing.
  - Encoding detection.
  - Relative, `\`, `file://`, `%XX` and missing entries, and stream URLs.
  - Round trips of written M3U8, absolute PLS and `../` M3U playlists.
  - ANSI `.m3u` output and its unmappable-character error.
  - Options page parsing.
  - `%_playlist_*%` values.
  - Partitioned creation in list order.
  - Literal confirmed names.
  - Input protection and collisions.
  - Don't create directories.
  - Unreadable files.
  - Symlinked working directories.
  - The Export options fallback.
  - Per-directory export names.
  - Cue sheet export followed by reload.
- `AR=/usr/bin/ar cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and `npx tsc --noEmit` are clean.
- `node --test tests/*.test.mjs`: 201 passed (baseline 196). The new tests cover playlist detection, option page contents and defaults, expansion order and errors, the creation request, and the Export run without the dialog, including per directory.
- Native e2e `OPENTAG_E2E_ID=org.opentag.editor.e2e.playlists scripts/e2e-native.sh tests/e2e/playlists.js`: **9/9 passed**. The script drives the real app and backend:
  1. The Options → Playlist and Options → Export pages save their values.
  2. Playlist (all files) shows the evaluated name in the confirm dialog and writes a descending list order with `#EXTINF:3,Sine B by Test Artist`.
  3. A second launch with `/fn:<album>/list.m3u8` loads the playlist in its order and clears the sort. The launch is started through `run_tool`, and the single-instance handler forwards it to the running window.
  4. View → Unsorted restores the playlist order after a column sort.
  5. Generate Playlists previews "2 playlists" and writes `.pls` files.
  6. ⌘E without the dialog exports at once.
  7. ⌘⇧E with the Cue sheet configuration writes `export.cue` in the folder, and a command-line launch loads it back in order.
  8. A playlist with a missing entry loads the other two files and reports `Line 2: gone.mp3: file not found`.
  9. Create playlist automatically writes `auto-album.m3u8` after ⌘S.
- The existing native suites `core-workflows` (13/13), `options-utils` (13/13) and `actions` (6/6) still pass on this build.

## Gaps

- **Loading:**
  - Cue sheets load the referenced audio files. They are not split into per-track virtual entries, and cue `TITLE`/`PERFORMER` values are not imported as tags. Mp3tag's own behaviour for multi-track images was not verified.
  - XSPF, WPL, ASX and other playlist formats are not read.
  - Stream URLs are reported, not loaded.
  - Windows absolute paths (`C:\…`) in playlists cannot resolve on macOS or Linux and are reported as missing.
  - The playlist is not re-read on Refresh (F5). F5 rescans the files that were loaded.
  - Loading a playlist clears the saved column sort; the unsorted order is the list order. A dedicated "Playlist order" sort criterion is not stored.
  - When a playlist is added to a non-empty list (drop, Add files, `/add`), files that are already listed keep their earlier position. Replacing the list (File → Load playlist/cue sheet, or a launch without `/add`) shows the exact playlist order.
- **Writing:**
  - `.m3u` on macOS and Linux is written as UTF-8 because those systems have no ANSI codepage.
  - The ANSI path error is stricter than Mp3tag, which presumably writes `?` (not verified).
  - "Entries relative to working directory" is implemented as relative to the playlist's folder. That is the form players resolve, and Mp3tag creates playlists in the working directory, so the two normally coincide.
  - Relative paths are written with native separators (`\` on Windows, `/` elsewhere).
  - Mp3tag's default playlist filename is not documented. OpenTag uses `%_directory%.m3u8` and, for the generator, `%_folderpath%%_directory%.m3u8` with partition `%_folderpath%`.
  - Automatic playlist creation follows every successful Save tags and every rename. It overwrites the playlist without asking, and its status replaces the "Saved tags" status.
  - Provider documents (Android/iOS) cannot load or create playlists.
- **Export:**
  - Open-after-export is offered only when exactly one file was written.
  - Per-directory export ignores a configured output folder and keeps only its file name.
  - Options → Export has no other settings, because the documentation lists none.
  - Users who had set BOM or directory creation in the old Export dialog keep those values until they save Options → Export. After that, the page values apply.
- The PARITY and FEATURE-INVENTORY documents are not updated here.
