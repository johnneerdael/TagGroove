# File menu, directories and file operations — 2026-10-14

Reference pages: [File menu](https://docs.mp3tag.de/menus/file/), [Edit menu](https://docs.mp3tag.de/menus/edit/), [keyboard shortcuts](https://docs.mp3tag.de/keyboard/), [Options → Directories](https://docs.mp3tag.de/customization/options/directories/), [Options → Messages](https://docs.mp3tag.de/customization/options/messages/) and [command line](https://docs.mp3tag.de/command-line/).

## Delivered behavior

**Directories.** Change directory (File menu, toolbar *Open folder*, Ctrl/Cmd D; Ctrl/Cmd O is File → Options as in Mp3tag) now replaces the file list, as it does in Mp3tag. Before this change it appended. The old list is cleared only after the new scan succeeds. If any file has pending edits, OpenTag asks before discarding them. Add directory (Ctrl H) and drag-and-drop append to the list without changing the working directory. Other directory commands:

- Upper directory (Ctrl/Cmd Shift D).
- Back and forward history (Alt ←/→), kept in memory for the session.
- Favorite directory (Ctrl M), which opens the first favorite.
- Recent directories and Favorite directories submenus in the File menu.
- A *Favorite directories…* dialog. It can add the current or a chosen folder, make a favorite first, remove or open favorites, and clear the recent list.

The File menu follows Mp3tag's order: save/read/playlist/export, then the directory commands (Change, Add, Add files, Upper, Previous, Next, Favorite directory, Favorite directories…, Recent and Favorite submenus), Clear list, Open in Finder/Explorer, Play, and Options last. In the Edit menu, Copy files to… and Move files to… follow Edit in file list (Mp3tag's Rename), then Remove from list and Delete files….

The file list (from `feat/file-list`) and file operations share one implementation. `removeEntries()` drops entries and their covers, drafts, overrides and cached column values; Remove from list, Clear list, Refresh, Delete and Change directory all use it. `scan(paths,{recursive,replace,location})` records the list roots: Change directory, Upper, history, favorites and replacing launches reset them to the new folder, while Add directory, Add files, drag-and-drop and `/add` launches append. Refresh rescans exactly those roots with the Read subfolders option.

Settings live in the shared Options store. Directory scans and dropped folders follow the existing *Read subfolders* option on File → Options → Directories. The startup directory (empty file list, previous directory or first favorite) is a *Startup* page under Directories, registered with `registerOptionsPage` and stored under `pages.directories-startup`. Recent and favorite folders are data rather than options, so they are saved in `directories.json` in the app configuration directory, next to `actions.json` and `exports.json`. That file has a version number, and the recent list is limited to 10 entries with no duplicates. Older files that still contain `subfolders` or `startup` fields load without error.

**File operations.** These work on selected files:

- Delete files… (Ctrl Del; Cmd Backspace or Cmd Delete on macOS) moves files to the system trash with the `trash` crate. On macOS it uses NSFileManager, so no permission to control Finder is needed. OpenTag never hard-deletes. A confirmation lists the count and up to five names. Each file is deleted separately: files that are deleted leave the list, and failures appear in the errors panel.
- Copy files to… (Ctrl/Cmd Shift C) and Move files to… (Ctrl/Cmd Shift V) ask for a destination folder, then for confirmation.
  - The whole plan is checked before any file is touched. If any target exists, two selected files share a name (compared case-insensitively), a source is already in the destination, or the destination is missing, nothing is copied or moved and each conflict is reported.
  - Copies use the new `tag_core::operations::copy_no_clobber`, which creates the destination exclusively and removes an incomplete copy. Moves use `rename_no_clobber`, including its cross-volume exclusive-copy fallback.
  - Moved entries get their new paths through the existing `applyRenamed`, so selection is kept. Copies are not added to the list.
- Open in Finder/Explorer selects the chosen files in the file manager. With nothing selected, it opens the working directory, as Mp3tag does.
- Play (F4, Ctrl/Cmd Shift T) opens each selected file with the default application. More than 10 files needs confirmation, and the backend caps a request at 100.

**Pending edits.** Copy, move and delete are disabled while the selection has pending edits ("Save pending changes first."). This is the safer choice than carrying drafts along. Reveal and play stay available. Provider (mobile document) entries disable all `file-*` operations with the provider reason, and non-desktop platforms disable them too. Folder commands follow the existing folder-picker availability.

**Command line.** OpenTag accepts `/fp:<folder>`, `/fn:<file>` and `/add`, case-insensitive, also with a `-` prefix. An empty switch value takes the next argument. Bare paths count as files or folders. Relative paths resolve against the launching working directory. Without `/add`, the list is replaced and the working directory becomes the folder, or the file's parent. `tauri-plugin-single-instance` forwards a second desktop launch to the running window: it focuses the window, queues the request and emits `opentag://launch`. The UI drains the queue at startup and on each event, so early requests are not lost. A launch request takes priority over the startup-directory option.

**Plugins and permissions.** `tauri-plugin-opener` is used only through its Rust API (`reveal_items_in_dir`, `open_path`), from app commands that check paths. It is deliberately not registered for the webview, and no `opener:*` capability is granted, so page script cannot open arbitrary paths or URLs. `trash`, `tauri-plugin-opener` and `tauri-plugin-single-instance` are desktop-only dependencies. Mobile builds get stub commands that return an error.

## Automated checks

**165 frontend and 353 Rust tests pass** after merging the latest `feat/core-parity`, with 2 ignored Rust tests: the real-trash probe here and one from another feature. Before this work the counts were 101 and 279.

New tests:

- **Rust, `src-tauri/src/file_ops.rs`:** recent-list dedupe, cap and order; settings round-trip, partial and corrupt files; per-file trash failures, where folders and missing files are never passed to the remover; copy keeps originals; move relocates and reports per-file read errors; conflicts (existing target, case-insensitive duplicate name, same folder, missing destination) reject the whole plan before any file is touched; launch-switch parsing.
- **Rust, tag-core:** `copy_no_clobber` never replaces an existing destination and refuses folders.
- **`tests/file-ops.test.mjs` (14 cases):** parent directories, including Windows drive roots; history; the shortcut map for Windows/Linux and macOS, including the text-editing guards; summaries; the change/add/up/back, delete, copy/move, launch, startup and Read-subfolders flows against a fake host; and the Startup options page definition and its default.

`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `npx tsc --noEmit` pass. Locally, `cargo test --workspace` needs `AR=/usr/bin/ar` because Homebrew GNU `ar` produces a `regex-compat` archive that Apple `ld` rejects. This is an environment issue, not a code issue.

## Native and preview checks

**Native macOS.** A debug bundle was built with the test identifier `org.opentag.editor.usertest.fileops` and run on generated MP3/FLAC fixtures in a scratch folder. The real library was not touched. It verified:

1. `opentag /fp:music/AlbumA` started in that folder and loaded 2 files.
2. A second launch, `opentag /fn:extra/extra.flac /add`, exited immediately and appended the file to the running window without changing the location.
3. Cmd Shift D moved up to `music` and loaded 4 files recursively.
4. Cmd Shift V showed the "Move files to" panel, and Cancel changed nothing.
5. The toolbar *Open folder* showed the "Change directory" panel, and Cancel kept the list.
6. Cmd Backspace showed the "Move 1 file to the Trash?" sheet. Yes removed the file from disk and from the list ("Moved 1 file to the Trash").
7. `directories.json` recorded both folders.
8. After a restart with startup set to *previous*, the app opened `music`. This native run used the pre-merge build, which kept the startup choice in `directories.json`. Ctrl M opened the favorite, and Alt ← went back.

**Browser preview.** In Chromium with a mocked Tauri IPC, the following rendered correctly in Dark and in Light at 390×844: the File menu, the Recent and Favorite submenus with long paths shortened, and the Directories and favorites dialog. Through the preview UI it also verified:

- Making a favorite first, then saving.
- Favorite directory, then back.
- A delete where one file failed and appeared in the errors panel.
- After the final merge: Ctrl/Cmd O opens Options and Ctrl/Cmd D opens the Change directory picker; the File menu renders in the order above; Refresh after the startup favorite and after opening another favorite rescans only that folder, non-recursively when Read subfolders is off.
- After the first merge: the Options tree lists Directories → Startup, its saved *First favorite directory* choice opens the first favorite at startup, and *Read subfolders* off produces a non-recursive scan.
- Copy, move and delete disabled with "Save pending changes first." while reveal and play stayed enabled.

## Known gaps compared with Mp3tag

- **Not driven natively:** the menus, the destination panel and the draft-blocking check, because the automation tool could not click inside the WebKit view. Keyboard shortcuts and native panels were driven natively. Play and Open in Finder were not launched natively, to avoid starting other apps on the test machine.
- **Associated files:** the reference pages do not describe how `.lrc`/`.cdg` or cover files move with audio, so they are not moved. This needs evidence from the original program.
- **Permanent delete:** Ctrl Shift Del is intentionally not implemented, and neither is the Options → Messages switch to skip the delete confirmation.
- **No subfolders checkbox in the native dialog:** the native folder dialog cannot show one, so subfolder reading is the Options → Directories setting.
- **History:** directory history is not persisted. F5 refresh, Load Playlist/Cuesheet and directory-tree browsing are out of scope.
- **macOS command line:** macOS LaunchServices does not pass arguments to an app that is already running (for example `open -a OpenTag --args …`). Forwarding works when the binary is executed directly and on Windows/Linux. Finder "Open With" file associations (`RunEvent::Opened`) are not handled.
- **Restoring from the trash:** with NSFileManager, macOS may not offer Finder's "Put Back" for trashed files. They can still be restored from the Trash by hand.
- **Shortcuts differ from Mp3tag:** on macOS, Hide and Minimize take Cmd H and Cmd M, so Add directory and Favorite directory use Ctrl H and Ctrl M there.
- **Play:** each file is opened separately; no temporary playlist is built.
