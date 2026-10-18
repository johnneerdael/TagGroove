# Playlist Refresh and source order — 2026-10-18

Scope: L45 implementation in consolidated draft PR64, layered on the existing local playlist readers. Local production-function/DOM tests and actual disposable-file scanner tests establish the named behavior; no original program, native app, provider or Windows runtime was used.

## Resulting behavior

The session retains original ordered input roots (playlist/CUE files, folders and explicit audio paths) separately from expanded rows. Refresh re-expands these inputs and stages a new scan. It commits only after a successful, current result. Fatal reads/scans, cancellation and changed workspace/draft context preserve the prior list and pending field/artwork edits. Valid playlists with missing members retain valid members plus diagnostics; a valid empty playlist can intentionally empty the list.

Source row rank, mutable manual insertion order and saved column sort are distinct. Loading a playlist selects session source order without erasing saved column sort. **View → Sort by → Playlist order** restores source rank among currently listed rows after column sorting, shuffling or manual moves. Removed rows remain removed until Refresh. Added groups append stably; the first duplicate input/member wins. Exact identities retain CUE/chapter suffixes. Width/visibility changes keep the selected display mode.

The native scanner now preserves the actual mixed CUE/audio input sequence instead of appending CUE rows after ordinary rows. Known app-reported file/folder moves rebase rank and source roots, loaded playlist/path-valued location and current folder. Only explicit backend mappings can imply a directory move; neighboring path prefixes are protected. Source playlist text and recent/favorite/navigation persistence are not rewritten.

Legacy profile-workspace v1 contains neither display mode nor source rank. Restoring it preserves its actual stored rows and resets to the saved column-sort mode; it cannot invent rank or original grouping. A general portable workspace backup is separate L57a work.

## Evidence

- [Production orchestration](../../src/main.ts), [source-order helpers](../../src/playlist-order.ts) and [native scanner](../../crates/tag-core/src/tags.rs).
- [Production-path orchestration tests](../../tests/playlist-refresh.test.mjs), [ordering tests](../../tests/playlist-order.test.mjs) and [real-files mixed scanner regression](../../crates/tag-core/tests/playlist_scan_order.rs).
- The first local focused batch passed 109 frontend tests and the TypeScript/Vite production build. The native mixed scanner plus adjacent chapter/metadata/guide integrations passed 27 tests; parser/security controls passed 24 tests. Follow-up actual move-context controls also passed.
- Final review reproduced and fixed two more defects: chapter parent/row replacement now retains source rank via the shared row-replacement policy, and cancellation during pending playlist expansion avoids starting an audio scan. The actual Options reload/accept/splice path and a deferred expansion/zero-new-scans control passed in the 73-test focused follow-up. A third control covers the actual re-add/scan path: transform old parent/row ranks before appending new ranks, including collapse and unknown legacy rank. These checks are included in the final combined matrix.

Original ordering/refresh semantics, native picker/input/keyboard/layout, external alias identity, remote/provider routes and released/installed behavior remain unqualified under L25/L74/L78/L79. This scope does not implement source-playlist editing or output XML formats. Local checks and an open draft are not a clean GitHub Codex code review, hosted validation, merge or publication.
