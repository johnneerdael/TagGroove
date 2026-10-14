# Manage Covers and cover window verification

Checkpoint: 2026-10-14, branch `feat/covers`. macOS arm64, WKWebView, debug bundle with the isolated identifier `org.opentag.editor.e2e.covers`.

## Scope

Mp3tag references: Manage Covers dialog, Cover Window Context Menu, main window cover area (navigation, enlarged preview, drag and drop, varying covers), keyboard reference (Ctrl+Alt+M).

| Feature | OpenTag |
|---|---|
| Cover info | Type name, `W×H, size, FORMAT`, description, `n / m` navigation with ‹ › buttons and ←/→ on the focused cover art |
| Multi-selection | Covers are shown only when every selected file has the same list (content digest + type + description); otherwise a placeholder with "Covers vary" and how many files have artwork, like Mp3tag's varying-cover hint. Shared covers say "Shared by N files" |
| Context menu (right-click, Shift+F10, ContextMenu key, ⋯ button) | Add cover…, Remove cover, Replace cover…, Extract cover…, Adjust cover…, Manage covers…, Set cover type ▸ (21 ID3 types), Set cover description…, Cut/Copy/Paste cover, Previous/Next cover, View larger…, Correct aspect ratio. Arrow keys, Home/End, → / ← for the submenu, Escape |
| Varying covers | Remove, Set cover type and Set cover description act on every cover of every selected file (Remove asks first); Replace, Extract, Cut and Copy need a displayed cover |
| Keyboard on cover art | ←/→ navigate, Enter/Space viewer, Del remove, ⌘/Ctrl C/X/V copy/cut/paste |
| Clipboard | Native clipboard through `arboard` (desktop only). Copy puts decoded RGBA image data; Paste prefers a copied image file (original bytes), then image data (encoded as PNG), then a pasted file path. `paste` events carrying image files are also accepted |
| Drag and drop | Native drops whose position is over the cover window embed image files (JPEG/PNG/GIF/BMP/WebP) in the selected files, with a "Drop to embed…" overlay; drops elsewhere still add audio files |
| Extract cover | Dialog with a file name format string (default `folder`, remembered; e.g. `%album% front`) resolved with the real formatter; Save writes next to the audio file with the extension from the MIME type and asks before replacing; Save as… opens the native save dialog |
| Larger viewer | Double-click or Enter; non-modal window that follows cover and file navigation; ←/→ inside it |
| Manage covers (Edit menu, Ctrl+Alt+M / ⌘⌥M) | All distinct covers of the selection in first-seen order with ● (all files) / ◐ n/m presence, thumbnail, type, description, dimensions/size/format. Add…, Paste, Replace… (keeps position/type/description), Remove, Move up/down (also Alt+↑/↓ and pointer drag), Apply to all, Extract…, type/description editing for one or several covers, row context menu (Apply to all, Set cover type, Set cover description…, Replace, Extract, Remove). OK writes one journaled plan |
| Undo | `apply_cover_plan` and `add_cover_image` run through `history::record` ("Manage covers", "Set cover type", "Add cover", …) |
| Appearance | Menu, viewer and dialog use theme tokens; compact layout (≤1000 px) gets 44 px targets, inline submenus and a stacked dialog |

## Backend

- `tag-core` `cover_manage.rs`: `CoverInfo.digest` (content identity), `CoverRef`, `CoverChange`, `apply_cover_changes_with_options` (whole displayed list; untouched files are not rewritten; staged write then digest/type/description verification), `add_cover_bytes_with_options`, `find_cover_data`, `picture_from_bytes` (WebP stored as `image/webp`). MP4 artwork has no types/descriptions and is normalized to plain artwork.
- `src-tauri` `covers.rs`: `cover_set`, `apply_cover_plan`, `add_cover_image`, `stage_cover_file`, `stage_cover_data`, `extract_cover`, `copy_cover_to_clipboard`, `paste_cover_from_clipboard`. Staged images stay in memory (16 entries / 128 MB) so large images never round-trip through the webview.

## Automated checks

- `cargo test --workspace`: 397 passed after merging `feat/core-parity` (new: 7 `cover_manage` unit tests for reorder/retype/describe/remove, unchanged files left untouched, replace keeping position/type/description, apply-to-all, WebP and duplicate refusal, MP3 policy formats; ASF and DSF/Matroska plan tests in `tests/asf.rs` and `tests/containers.rs`; 3 command tests for `cover_set` aggregation, plans, extract and staging limits). Clippy `-D warnings` clean.
- `node --test tests/*.test.mjs`: 173 passed (new `tests/covers.test.mjs`: details formatting, selection aggregation, context plans, manage list presence/plan/moves, menu availability, extract names and drop hit-testing; `document-coordination` now renders the real cover window markup).
- Native: `OPENTAG_E2E_ID=org.opentag.editor.e2e.covers scripts/e2e-native.sh tests/e2e/covers.js` — **15/15 PASS**:
  - empty cover window hint; Edit ▸ Manage covers… present
  - `paste` event with a canvas PNG embeds it; info shows `300×200, 2 KB, PNG`
  - ⌘Z/⌘Y undo and redo the added cover on disk
  - context menu lists every documented command; Escape closes it
  - Shift+F10 → Set cover type ▸ Back Cover (keyboard) writes `CoverBack`
  - Set cover description… writes the description
  - ⌘C on the cover of file A, ⌘V on file B through the system clipboard writes a 300×200 PNG
  - A+B selected with different covers: "Covers vary" placeholder, no image
  - Ctrl+Alt+M: two rows "In 1 of 2 files", Select all + Apply to all → "In all 2 files", Move up, OK: both files get `CoverFront, CoverBack:Rear sleeve`
  - shared covers: "Shared by 2 files", `1 / 2` → → `2 / 2`; Enter opens the viewer, which follows ‹ navigation; close
  - ⌘Z undoes Manage covers on both files; ⌘Y redoes
  - Extract cover with format `%album% front` saves `Test Album front.png` next to the file
  - emitting native `tauri://drag-enter`/`tauri://drag-drop` over the cover window with that file shows the cover drop overlay and embeds it (3 covers)
  - Del on the focused cover art removes only the displayed cover
  - cover menu colors differ between Light and Dark

  Under heavy machine load (load average ~30 with sibling builds), some runs of both this script and `core-workflows.js` stalled until the 180 s limit at varying steps. The same stall happened with the unmodified `feat/core-parity` frontend, and immediate reruns passed (covers 15/15 in 8 s). The likely cause is suspended animation frames in an occluded window, which a sibling branch addresses in `e2e.rs`.

## Known gaps

- Adjust cover from the context menu opens Actions (Quick) preset to Adjust Cover, which adjusts every cover of the selected files; Mp3tag's per-cover Adjust inside Manage covers is not implemented.
- Add cover opens in the selected file's folder; Mp3tag's Options ▸ Directories cover art directory and existing-image lookup are not implemented.
- Dragging a cover out of the cover window (Mp3tag supports both directions) is not implemented.
- Setting a type or description on MP4 artwork is silently normalized (MP4 stores neither).
- Native file pickers and the native save dialog (Add…, Replace…, Save as…) cannot be driven by the e2e hook; their result paths go through the same commands that the e2e run exercises.
- Compact layout and touch were checked by CSS review only, not on a device or resized native window.
