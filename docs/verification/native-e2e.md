# Native end-to-end checks (debug build hook)

Checkpoint: 2026-10-14, branch `feat/core-parity` after merging tag-panel, tag-prefs, edit-history, tag-sources, file-list and file-ops. macOS arm64, WKWebView, debug bundle built with the isolated identifier `org.opentag.editor.usertest` so the regular profile's settings are untouched.

## How it works

External accessibility automation could not deliver clicks, keys or values into the WKWebView content (element presses on native buttons worked; table rows, coordinate clicks and AX value changes did not). A debug-only hook replaces it: when a **debug** build starts with `OPENTAG_E2E_SCRIPT=<file>`, `src-tauri/src/e2e.rs` evaluates the script in the main webview after page load, and the script reports through the `e2e_report` / `e2e_exit` commands. Release builds and debug builds without the variable ignore both commands and never read the variable's file.

Scripts drive the real UI code by dispatching DOM events (row clicks, input events, keyboard shortcuts, button clicks) and verify results by re-reading files through the real `scan_files` backend. Synthetic DOM events are not OS input: they prove the UI wiring, backend commands and file results, not native hit-testing, IME or focus rings.

```sh
scripts/e2e-native.sh                       # builds the usertest app, generates MP3 fixtures, runs tests/e2e/core-workflows.js
scripts/e2e-native.sh tests/e2e/core-workflows.js --no-build
```

## Results

`tests/e2e/core-workflows.js` on generated MP3 fixtures: **13/13 PASS**

- row selection updates the tag panel
- Title edit + ⌘S writes the file (independently confirmed with ffprobe; file rewritten as ID3v2.3 + ID3v1 per default preferences)
- ⌘Z restores the previous bytes on disk; ⌘Y re-applies
- ⌘C on one file, ⌘V on another stages drafts; ⌘S writes; ⌘Z restores
- Convert → Tag → Tag (Alt+5) writes `%artist% / %title%` into COMMENT
- ⌘O opens Options (General, Files, Directories/Startup, Appearance, Tags/Mpeg/Ape, Mpc/Advanced, Tag Sources)
- ⌘F opens Find; Del removes from list; F5 restores the list
- Tag Sources → MusicBrainz opens prefilled from tags; View → Customize columns opens; Light/Dark switch

`tests/e2e/musicbrainz-live.js` (live network, run once against a disposable copy of a purchased FLAC): prefilled search → 2 results → review (fields incl. MusicBrainz IDs, catalog number, barcode, ISRC; 1 track matched) → "Save tags to 1 file" → disk shows the release album, `MUSICBRAINZ_ALBUMID` and one cover.

## Findings

- ⌘Z pressed while focus is still in a tag panel field after saving is handled as text-field undo (by design: saved-operation undo only applies outside text fields). The test focuses the file list first, as a user would.
- "Saved tags" appears in the status bar slightly before the undo history refresh completes; an immediate ⌘Z can be ignored. Not observable at human speed; the test waits for Undo to become available.
- Tag panel labels underline their mnemonic letter with separate elements, so accessibility names read "TIT L E". Needs an `aria-label` fix.

## Not covered here

Native folder pickers (copy/move/change directory dialogs), drag-and-drop, column drag/resize by hand, Windows/Linux runs, mobile.
