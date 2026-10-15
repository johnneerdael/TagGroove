# Export, playlists, Find, Manage List and cover drag-out (export-lists)

Branch `feat/export-lists`, based on `feat/core-parity` at `d67e5c8`. This covers the remaining-gaps rows for export, playlists, Find, Manage List and cover drag-out that were still open after batch 5. Mp3tag documentation was used for behaviour only. The export templates are original OpenTag work.

## What changed

| Item | Before | Now |
|---|---|---|
| Open file after export | Offered only when exactly one file was written, through a native yes/no box | Offered for every export, playlist and Export CSV run when Options → Messages "at exporting and creating playlists" is on. With one file the dialog asks to open it. With several files (partitions, one file per folder, Generate Playlists) the user picks one from a list. Both forms have **Open** and **Show in Finder/Explorer/file manager**. The Export dialog closes first (`src/export-ui.ts` `offerOpenOutputs`) |
| Rename a configuration | Separate "Configuration name" field | Renamed in place in the list: F2, a second click on the selected name (500 ms delay, a double-click edits the template instead) or the Rename button. Enter or leaving the field keeps the name; Escape cancels and leaves the dialog open. Names must be nonempty and unique, compared without case (`renameExportConfiguration`) |
| Bundled templates | Text and Cue sheet | Also CSV (spreadsheet), HTML table, Extended M3U playlist, JSON and Album list. All are written for OpenTag (`src-tauri/src/exports.rs`). A `bundled` counter in `exports.json` gives existing libraries the new set once. A bundled configuration the user removes stays removed, and names or ids already in use are not touched |
| Per-directory export with an output folder | Only the file name was kept | A plain name, or a name in a relative folder, is created in each folder (`%_folderpath%sub/name`). A name in an absolute folder, or one below `%_workingpath%`, keeps that folder and mirrors each source directory below it, relative to the deepest folder shared by all exported files (`$cutLeft(%_folderpath%,N)`). Files on different drives fall back to one subfolder per directory name |
| Playlists in manual order | Manual order (Alt+arrows, Alt+drag) came in batch 5 and already reordered the list that playlists read | Covered by a native test: Alt+↓ followed by Playlist (all files) writes the new order |
| Entries relative to the working directory | Entries were relative to the playlist folder whenever the paths shared a root, so `..` could climb anywhere | The option is labelled "Entries relative to working directory (if possible)". An entry is relative (to the playlist folder) only when both the audio file and the playlist are inside the working directory. Otherwise the path is absolute. With no working directory the old rule applies. The same rule covers File → Playlist and the Generate Playlists action (`PlaylistWriteOptions::root`) |
| Find history | Kept for the session only, with a "Clear history" button | Saved in the configuration's helper lists (`opentag.history.find`). The ► button next to the search field has **Remove from history** (the current term) and **Remove all from history…**. Criteria are remembered on Find and on Enter |
| Manage List favorites | Missing | A checkbox on each entry, and in the edit form. Favorites show in that input's ► menu and replace its value when chosen. They are never dropped by the 25-entry history limit |
| Manage List duplicate | Missing | **Duplicate…** opens the edit form with a copy (named entries get "copy", "copy 2"…) and inserts it after the source. Each format string can appear once, so the copy must be changed before it is kept |
| Helper list storage | WebView `localStorage`, outside the configuration backup | `lists.json` in the configuration folder (`src-tauri/src/lists.rs`), validated, and included in File → Save/Load configuration. On the first run without `lists.json`, the field list and every `opentag.history.*` list move out of local storage, and the moved keys are removed only after the save succeeds. Changes made before the lists finish loading are kept. Saving pauses while a configuration is being restored |
| Cover drag-out | Drop-in only | Dragging the shown cover out of the cover window writes it to a temporary file named after the album (or the audio file), with the image's extension. A native file drag then starts through the `drag` crate (AppKit, OLE, GTK). Drag folders older than an hour are removed. While the drag is running, drops on OpenTag's own window are ignored |

## Decisions

- **Per-directory export into an absolute folder mirrors the folder tree.** Writing every folder's file into one flat folder would collide (for example two `CD1` folders), and the exporter refuses colliding outputs.
- **"Relative to working directory" keeps paths resolvable.** Players resolve relative entries against the playlist's own folder, so entries stay relative to that folder. The working directory sets how far up they may point.
- **The open-after-export offer is an in-app dialog instead of a native message box.** It can then list several files and be tested headless.
- **Duplicate requires a changed copy.** Histories are keyed by their value, and removing an entry or choosing it from the drop-down list identifies it by that value.
- **Cover drag-out uses a temporary file.** Every platform accepts a file drop, so this works on all three. Scripted (e2e) runs only prepare the file and report "Cover file ready to drag", because a native drag needs a real pointer button held down.

## Verification

Unit tests (`AR=/usr/bin/ar npm test`, all passing locally on macOS):

- `src-tauri/src/exports.rs`: the bundled templates validate. CSV quotes every value and doubles quotes. HTML escapes `& < >` and adds totals. The extended M3U loads back in order. JSON parses with escaped quotes, backslashes and line breaks. Album list names each album once. Older libraries receive the new set once, and a removed one stays removed. Per-directory naming covers relative, absolute, `%_workingpath%` and different drives. A rendered per-directory export writes `out/a/CD1/list.txt` and `out/b/CD1/list.txt`.
- `crates/tag-core/src/playlist.rs`: relative entries stay inside the working directory, and a playlist outside it gets absolute entries.
- `src-tauri/src/lists.rs`: lists round-trip with names, descriptions and favorites. Unknown keys, malformed entries, oversized lists and unknown versions are refused.
- `src-tauri/src/utilities.rs`: configuration bundles carry `lists.json`, and an invalid `lists.json` rejects the bundle.
- `src-tauri/src/cover_drag.rs`: safe file names with the right extension, prepared files and clean-up of old folders, small drag previews.
- `tests/helper-menu.test.mjs`: favorites in the ► menu and under the history limit, duplicate naming, lists loaded from and saved to the configuration, one-time migration from local storage, changes made before loading.
- `tests/export-ui.test.mjs`: in-place rename rules.

Native UI suite `tests/e2e/export-lists.js` (fixtures `tests/e2e/export-lists-fixtures.sh`) runs headless in the `native-e2e` CI job, and was never run on a desktop. Its steps:

1. Lists placed in local storage before a restart are in `lists.json` afterwards, and the local copies are gone.
2. The Export dialog lists the seven bundled configurations. F2 renames in place, a second click starts a rename and Escape cancels it, a duplicate name is refused, and the new name is saved.
3. The JSON configuration exports three files as valid JSON. The one-file offer appears with Open and Show in file manager.
4. Shift-export into `<work>/exports/list.txt` writes `exports/list.txt` and `exports/disc2/list.txt`, each with its own folder's files, and the offer lists both.
5. Alt+↓ then Playlist (all files) writes the new list order, with `disc2/sine-c.mp3` relative.
6. The Options → Playlist label reads "working directory". A playlist written outside the working directory has absolute entries.
7. Find terms are saved to `lists.json`. ► Remove from history removes the current term, and a reopened Find offers only the remaining one.
8. A pointer drag on the cover window prepares `Test_Album.png`, which is the 16×16 embedded cover.
9. Manage history: ticking an entry saves it as a favorite. An unchanged duplicate is refused and a changed one is saved after its source. Both favorites show in the ► menu, and choosing one sets the filter.

CI evidence: see the commit and run recorded in the report for this branch.

## Remaining gaps

- The native drag-out itself (AppKit, OLE or GTK drag session, drop into Finder, Explorer or a Linux file manager) is not exercised automatically. CI proves that it compiles on every desktop target and that the file is prepared. A person still needs to drag a cover to the desktop on each platform.
- The GTK drag source writes `file://` URIs without percent-encoding. On Linux the temporary file name is therefore limited to ASCII letters, digits and `-_.()`.
- "Remove all from history…" asks through a native confirmation, which the headless suite cannot answer. The suite checks that the item is present. Its behaviour is the existing, unit-tested `history-clear` command.
- Cue sheet entries (TITLE/PERFORMER/INDEX per track), XSPF/WPL/ASX, the Adjust cover step inside Manage Covers and the cover-art directory option are outside this workstream.
