# Tag sources, round 3 — 2026-10-15

Branch `feat/sources-3`, started from `feat/core-parity` at `d67e5c8`. It covers the Tag sources rows of `docs/research/remaining-gaps.md` that were still open in the code: Ctrl+I / Ctrl+Shift+I, the review step (field multi-select, drag and drop, Suggest sorting, Discogs Release ID), saved cover files and undo, the freedb server list and text search, and keychain storage for the Discogs token. Nothing here was checked against the original program. Each decision names the documentation it follows (`tag-sources/import`, `menus/tag-sources`, `options/tag-sources/freedb`, `options/tag-sources/local-freedb`, the `keyboard` reference) and the reading taken where the documentation leaves room.

## What changed

### Ctrl+I and Ctrl+Shift+I

- Ctrl+I opens the last tag source used, of any kind: MusicBrainz, Discogs, Discogs Release ID, freedb, freedb (local), Covers: MusicBrainz and Covers: Discogs. The choice is stored per device as the menu command (`opentag.tagSources.lastSource`). The value older builds stored (`opentag.tagSources.last`, MusicBrainz or Discogs only) is still read when nothing newer exists.
- The Tag Sources menu shows the Ctrl+I hint next to whichever command it will repeat.
- Ctrl+Shift+I is the keyboard reference's "Last used Tag Source (Quick)". The documentation gives no detail, so OpenTag skips the step of entering the search: the dialog searches at once with the selection's album artist and album (freedb sources use the disc ID of the selected files), and a single result opens the review without showing the list. Without anything to search with (no album or artist in the selection, Discogs Release ID, or no disc ID because a length is missing) the dialog opens as with Ctrl+I.

### Review step

- Field rows can be selected like a list: click selects one, Ctrl/⌘+click toggles one, Shift+click selects a range, ↑/↓ move the selection. Ctrl+A selects every field, Ctrl+Shift+A inverts the selection. Neither reaches the file list, and both are left to the text box while a value is being edited.
- Clicking a checkbox on a selected field gives every selected field the new state (`tag-sources/import`: "If you click a checkbox while multiple fields are selected, the new state is applied to all selected fields"). A checkbox on a field outside the selection changes only that field. Space on a selected row does the same as clicking its checkbox.
- Files can be dragged onto another track in the track list. The dropped file takes that track and the files in between shift by one, which matches Alt+Page Up/Down. The ↑/↓ buttons, Alt+↑/↓ and Alt+Page Up/Down remain.
- Utils → Suggest sorting — Title and — Length are now checkboxes, remembered per device (`opentag.tagSources.suggestSorting`). When one is checked it decides the first matching of every release that opens; unchecking returns to auto-match. The two are exclusive: Mp3tag lists them as two checkable items, and OpenTag has no rule for combining an edit-distance order with a length order. Auto-match and Selection order stay as one-shot buttons.
- Tag Sources → Discogs Release ID… is its own menu command. Its dialog asks only for the release ID or address and goes straight to the review. The Discogs dialog keeps its own Release ID search type.

### Saved cover files in the undo step

Utils → Save image to disc writes an image next to the audio. Those files now belong to the import's undo step. The journal in `crates/tag-core/src/history.rs` gained side files: before an image is written, its current content (or its absence) is copied into the session journal, and the result is copied after the operation. Undo removes an image the import created or puts back the one it replaced; redo writes it again. An image changed by another program after the import is reported ("Changed outside OpenTag") and left alone, like audio files. An image whose bytes did not change is not recorded. Side files are small and are copied whole rather than hard-linked, because they may be rewritten in place.

### freedb

- Options → Tag Sources → freedb holds a named list of servers (name, protocol, address, path, port) with New and Delete. The selected entry is the server in use; it is written to the page's existing `protocol`/`address`/`path`/`port` values, so the backend reads the server exactly as before. A server saved by an earlier build that matches no list entry joins the list as "Custom server".
- "Current list of freedb servers" restores the list. Mp3tag downloads it; OpenTag has no list service, so the button restores the list shipped with OpenTag (gnudb over HTTPS and over HTTP), and like Mp3tag's it discards changes made to the list. The email address stays a single setting for every server.
- Local freedb text search. Options → Tag Sources → Local freedb has a Build index button with progress and Cancel. The index (`local-freedb-index.tsv` in the configuration folder) has one line per disc: category, disc ID, artist, title, year, genre and track count, read from both folder layouts. A build replaces the index only when it completes. freedb (local) gains the search type "Artist and album words": every word must occur in the disc's artist or title, ignoring case and accents. Results are paged 50 at a time with the total. Searching with an index built for another folder asks for a rebuild.
- freedb servers search by the files' track layout or, since `feat/gnudb`, by artist and album with gnudb's CDDB `search` command (`gnudb.md`). A typed disc ID is no longer sent to a server.

### Discogs token in the system keychain

The token dialog offers "Keep the token in the system keychain" on desktop builds (macOS Keychain, Windows Credential Manager, Secret Service on Linux; `keyring` 3.6.3 behind the `SecretStore` trait in `src-tauri/src/sources/credentials.rs`). Mobile builds keep the private file only.

- Nothing moves on its own. An existing token stays in the 0600 `tag-sources.json` file until the user ticks the option and saves.
- Moving to the keychain writes the token there, reads it back, and only then rewrites the file without the token (`discogs_token_in_keychain: true`). Moving back writes the file first and then deletes the keychain entry. A failure at any step keeps the token where it was.
- The keychain is not read to show the dialog. It is read only for Discogs requests, on a thread outside the async runtime because a keychain call may block or prompt. The entry is stored under the build's identifier, so a test build never sees the app's entry.
- Unit tests and scripted end-to-end runs use an in-memory store (`MemoryStore`; `crate::e2e::enabled()` selects it). No test or local run touches a real keychain.

## How it was verified

| Item | Unit tests | Native suite (`tests/e2e/sources-3.js`, headless in CI) |
|---|---|---|
| Ctrl+I any source, menu hint | `Ctrl+I repeats the last tag source of any kind` (`tests/tag-sources.test.mjs`) | Ctrl+I reopens Discogs Release ID, then freedb (local); the menu shows the hint |
| Ctrl+Shift+I quick | — | Opens the single disc `07000602` (two 3.000 s WAV fixtures) directly in the review |
| Field multi-select, Ctrl+A, Ctrl+Shift+A, Space | `review fields select like a list…` | Click, Ctrl+click, Shift+click, checkbox on the selection, Ctrl+A kept inside the dialog, Ctrl+Shift+A, Space, Ctrl+E |
| Drag and drop, Alt+Page Down | — | Synthetic drag events swap two files; Alt+Page Down moves the first file to the end |
| Suggest sorting checkboxes | `suggest sorting when checked decides the first matching` | Length checkbox stored and still checked in the next review |
| Discogs Release ID entry | `sourceRequest` round trip | Dialog asks only for the ID (offline: no lookup is made) |
| Cover files and undo | `side_files_created_or_replaced_are_removed_or_restored`, `unchanged_side_files_are_not_recorded` (`crates/tag-core/tests/history.rs`), `saved_cover_files_are_part_of_the_import_undo_step` (`src-tauri/src/sources/mod.rs`) | Not covered: no offline source returns artwork |
| freedb server list | `freedb servers are a named list…`, options page markup test | New, rename, Delete, choosing a server, Current list, saved list |
| Local text index and search | `index_covers_both_layouts_and_finds_discs_by_words` (both layouts, accents, paging, wrong folder, cancelled rebuild keeps the old index) | Build index (3 discs), status command, "cafe night" finds the Windows-layout disc, the import writes it |
| Keychain storage | `tokens_move_to_and_from_the_keychain_without_leaving_copies`, `a_failing_keychain_keeps_the_file_token`, `token_is_replaced_or_cleared_and_never_viewed` | Save to the keychain (in-memory store), move back, remove |

`tests/e2e/freedb-local.js` now ticks the Length checkbox where it pressed the removed By length button.

Local checks: `AR=/usr/bin/ar npm test` and `AR=/usr/bin/ar cargo clippy --workspace --all-targets -- -D warnings`. The native suites run only in the `native-e2e` CI job (`sources-3` was added to its list); no app window was opened on a workstation.

## Remaining gaps

- freedb servers: no audio-CD lookup (out of scope). Text search on servers was added later with gnudb's `search` command (`gnudb.md`).
- The server list is not downloaded; "Current list of freedb servers" restores OpenTag's built-in list.
- Ctrl+Shift+I's exact behaviour in Mp3tag is undocumented; the reading above is OpenTag's.
- Saved cover files are journaled only for local folders. Provider documents (Android) are not journaled, as before.
- The keychain option is not offered on Android or iOS.
- The cover-file undo path has unit coverage only; no offline native suite imports artwork.
