# Launch forwarding, Finder "Open With" and Mac Remove from list

Branch `feat/launch-forwarding`. It covers the remaining-gaps runners-up "Finder Open With and `open --args` forwarding on macOS" and "Mac keyboards have no way to trigger Remove from list", and re-checks single-instance forwarding on all desktop platforms. Reference page: `command-line` (the `/fp:`, `/fn:` and `/add` switches). `getting-started/main-window` says files loaded from the Explorer context menu replace the list, like a drop, and Ctrl+drop adds.

Nothing was launched locally. The evidence is unit tests, a config check and the native suite `tests/e2e/launch-forwarding.js`, which runs headless in the CI `native-e2e` job (Linux, xvfb). The macOS bundle's `Info.plist` is checked in the CI `desktop` job on `macos-latest`.

## What changed

| Path | Before | Now |
|---|---|---|
| Finder "Open With", files or folders dropped on the Dock icon, `open -a OpenTag <paths>` | macOS sends an open-documents event; nothing handled `RunEvent::Opened`, and the bundle declared no document types, so Finder did not offer OpenTag | `lib.rs` handles `RunEvent::Opened` on macOS. `file_ops::opened_request` turns `file:` URLs into a launch request: percent-encoding decoded, the trailing slash on folder URLs dropped, folders and files sorted apart, other URL schemes ignored. The request replaces the list, as Mp3tag's Explorer context menu does. It is queued and announced with the same `opentag://launch` event as a second launch, and the main window is unminimized and focused |
| Document types | none | `src-tauri/tauri.macos.conf.json`: audio (all 35 extensions `tag-core` reads), playlists and cue sheets (`m3u`, `m3u8`, `pls`, `cue`) and folders (`public.folder`). Audio is role Editor; playlists and folders are Viewer. Every entry has rank Alternate, so OpenTag shows up in Open With but never becomes the default app |
| Second launch with paths or `/fp:` `/fn:` `/add` | Already forwarded by `tauri-plugin-single-instance` (Unix socket on macOS, D-Bus on Linux, a named mutex and window message on Windows) | Unchanged, but now shares `forward`/`focus_main` with the Opened path |
| A forwarded request while an operation runs | `FileOpsUI.launch` returned early when the app was busy, so a request that arrived during a save or a scan was lost after it left the queue | `takeLaunches` runs requests one at a time in arrival order and waits until the running operation ends. A failed request does not block later ones |
| Remove from list on Mac keyboards | Done on `feat/keyboard-extended` (f79afcf): ⌫ removes from the list, ⌘⌫ deletes files | No change; see the decision below |

### Document types and Tauri's UTI inference

Tauri's bundler adds `LSItemContentTypes` for the extensions it knows (`mp3`, `wav`, `aac`, `m4a`, `mp4`, `mkv`, …). macOS ignores `CFBundleTypeExtensions` in any entry that also lists content types. One entry with every extension would therefore have matched only those five or six types. The config uses three kinds of entry instead:

1. Content types only: `public.audio` (MP3, AAC, M4A, WAV, AIFF, FLAC and every other type the system knows as audio), `public.mpeg-4`, `com.apple.m4v-video` and `org.matroska.mkv`.
2. Extensions only: every other `tag-core` extension, none of which Tauri maps to a type (`flac`, `ogg`, `opus`, `ape`, `wv`, `dsf`, `dff`, `tak`, `mpc`, `mp+`, `wma`, `webm`, `mka`, …).
3. Playlists by extension, and folders by `public.folder`.

`tests/launch-forwarding.test.mjs` reads the extension list from `crates/tag-core/src/tags.rs` and `PLAYLIST_EXTENSIONS` from `src/playlist-ui.ts`. It fails when an extension is not covered, when an extension-only entry contains an extension Tauri would infer, or when an entry is ranked anything other than Alternate. CI then checks the generated `Info.plist` for the same properties.

### Decisions

- **macOS only.** Tauri's NSIS installer writes `HKCR\.mp3` (default value) for every associated extension, which would replace the user's default player on Windows. Android would gain `VIEW`/`SEND` intent filters for audio that the document flow does not handle. On Linux, the generated `.desktop` file has `Exec=opentag` without `%F`, so "Open With" would start OpenTag without the file. Windows and Linux keep the CLI and single-instance path. Explorer and file-manager integration stays out of scope.
- **Opened replaces the list.** Mp3tag loads files from the Explorer context menu like a drop, which replaces the list. `/add` stays the way to append from the command line. With pending tag changes, the usual "discard pending changes" confirmation comes first. Opened events go through the launch queue (`FileOpsUI.launch`), not the window-drop path. The modifier lookup that `feat/keyboard-ui` added for drops (`modifier_keys`, where Ctrl or ⌘ while dropping adds) therefore does not apply: whatever key is held when Finder sends the event, an open-documents event always replaces the list, and a forwarded launch follows its own `/add`.
- **`open -a OpenTag --args …` with OpenTag running.** LaunchServices only activates a running app and drops `--args`. This is macOS behaviour and no app can receive them. Paths given without `--args` (`open -a OpenTag a.flac Album/`) arrive as an open-documents event and load. Switches reach a running instance with `open -n -a OpenTag --args /fp:… /add`: the new process forwards them over the single-instance socket and exits. On a cold start, `--args` are parsed as before.
- **Remove from list on Mac keyboards.** The brief suggested ⌘⌫. That chord already means Delete files (Mp3tag's Ctrl+Del, and Move to Trash in Finder), so remapping it would make a destructive action and a harmless one trade places. Plain ⌫, already shipped, matches Finder sidebars, Music and Photos, where Delete removes an item from a list and ⌘⌫ removes the file. The Edit menu shows "⌫ / Del". `feat/keyboard-ui`, merged here, added ⌘⇧⌫ for permanent delete, which completes the set: ⌫ removes from the list, ⌘⌫ moves to the Trash, ⌘⇧⌫ deletes permanently. The Mac rules are covered by `tests/list-commands.test.mjs` (⌫ → remove, ⌘⌫ not remove, ⌫ in text fields edits text) and `tests/file-ops.test.mjs` (⌘⌫ → Delete files), and natively by the last step of `tests/e2e/keyboard-extended.js`.

## Unit tests

| Test | Checks |
|---|---|
| `file_ops::tests::opened_urls_become_a_replacing_request` (Rust) | `file:` URLs with a space and `#` decode, folder URLs lose the trailing slash, a folder URL without a slash is classified by `is_dir`, `https:` and custom schemes are ignored, an empty or non-file event yields no request, `add` is false |
| `file_ops::tests::launch_arguments_follow_mp3tag_switches` (Rust, existing) | `/fp:`, `/fn:`, `/add`, `-` prefix, case, value in the next argument, `-psn_` ignored, bare paths |
| `forwarded launches wait for a running operation and open in arrival order` (`tests/file-ops.test.mjs`) | Two requests that arrive while busy are neither dropped nor loaded until the operation ends. They never load concurrently and open in order (replace, then `/add` append). `/add` keeps the working directory |
| `a failed launch does not block later requests` | A failing `take_launch_requests` rejects that call only |
| `startup opens queued launch requests before the startup directory` (existing) | Cold-start queue precedes the startup folder |
| `tests/launch-forwarding.test.mjs` | Association coverage and shape, as described above, and no `fileAssociations` in the shared `tauri.conf.json` |

Local results on the branch head: `node --test tests/*.test.mjs` passed 251 of 251, `cargo test --workspace --locked` passed with 0 failures, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` were clean, and `tsc --noEmit` was clean. Setting `rank` to an invalid value made `cargo check` fail in `tauri-build` ("unknown variant"), which confirms the build parses the macOS config.

## Native e2e (CI only)

`tests/e2e/launch-forwarding.js`, with `tests/e2e/launch-forwarding-fixtures.sh`. The app starts with `/fp:<album>` (two files). The fixtures create `Second Instance/`, `Open With/` (including `open #e.mp3`) and `Opened Folder/` next to the album.

1. **Real second launch.** The fixtures script waits until the first instance owns `org.opentag.editor.usertest.SingleInstance` on the session bus. It then runs the debug binary again from `Second Instance/` with `/add /fn:forward-c.mp3`, a relative path, without the e2e script. The step expects three rows and an unchanged working directory. It runs on Linux only, which is where CI runs it.
2. **Open With.** `e2e_open_urls` passes two file URLs (one with `%20` and `%23`) and an `https:` URL through `file_ops::open_urls`, the function `RunEvent::Opened` calls. The list shows exactly the two files, and the location is their folder.
3. **Folder dropped on the Dock icon.** A folder URL with a trailing slash. The list shows the folder's file, and the location is the folder path without a slash.
4. **Back-to-back events.** Two open events in a row. Both load in order, and the second replaces the first.
5. **Non-file URLs.** The list is unchanged.

`e2e_open_urls` exists only when the e2e hook is enabled (debug or profiling builds with `OPENTAG_E2E_SCRIPT`), like `e2e_report` and `e2e_exit`.

## CI run

First run on `a0ba6e5` ([run 38689894205](https://github.com/johnneerdael/opentag/actions/runs/38689894205)):

- `launch-forwarding`: 5 of 5 passed. A real second launch appended `forward-c.mp3` (3 rows, working directory still the album). Open With loaded `open #e.mp3` and `open-d.mp3`. The folder URL opened `/tmp/…/Opened Folder` without a trailing slash. Back-to-back events loaded in order. Non-file URLs left the list unchanged.
- macOS `desktop` job: the Info.plist check passed. `CFBundleDocumentTypes` has four entries (Audio file with `LSItemContentTypes`; Audio file by extension, and Playlist or cue sheet, with `CFBundleTypeExtensions` only; Folder with `public.folder`), all `LSHandlerRank` Alternate.
- Ubuntu, Windows, macOS desktop and Android jobs passed, and the other twelve native suites passed. `core-workflows` failed 5 of 13 steps on timeouts: the first save, undo and redo, F5 refresh, and the MusicBrainz dialog. That suite neither forwards launches nor opens documents, and the same steps passed in every earlier run on the base branch, so the branch was run again. The rerun on `83016c8` ([run 38698448953](https://github.com/johnneerdael/opentag/actions/runs/38698448953)) passed every job, including `core-workflows` and `launch-forwarding`.

The branch then merged `origin/feat/core-parity` at `8047b8d` (actions-polish, sources-3, keyboard-ui). The only conflict was the native suite list, which now keeps all suites and adds `launch-forwarding`. After the merge, these passed locally: `node --test` 271 of 271, `cargo test -p opentag --lib` 208 (4 ignored), workspace clippy with `-D warnings`, `cargo fmt --check` and `tsc --noEmit`. The CI run on the merge commit is in the hand-off report.

## Remaining gaps

- The cold-start Opened event on a real Mac (Finder launching OpenTag with a file) has no automated native test. CI's macOS job builds and checks the bundle but does not run UI suites. The code path is the same queue that the startup test covers, because Opened events that arrive before the page loads stay queued until the UI takes them.
- `.mkv` and `.webm` video files appear in Finder's Open With only when the system or another app declares their types (`org.matroska.mkv`), or through the extension entry for `webm`.
- Windows Explorer and Linux file-manager "Open With" integration is not declared (see Decisions).
