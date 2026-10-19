# Edit history, tag clipboard and Convert → Tag - Tag

> Historical implementation checkpoint. Later Windows replacement-identity
> support is recorded in [its scoped verification](undo-windows-identity-2026-10-18.md).
> The later [C61 content-baseline repair](history-content-baselines-2026-10-19.md)
> supersedes the metadata-only concealed-edit gap; see the
> [current history guide](../user-guide/guides/undo-history.md) and
> [L61's represented scope](../research/remaining-gaps.md#l61). Older gap bullets
> below retain this checkpoint's scope and are not a current implementation backlog.

Branch `feat/edit-history`. Covers Mp3tag's Edit → Undo (Ctrl+Z), Redo
(Ctrl+Y), Cut/Copy/Paste tag (Ctrl+X/C/V) and Convert → Tag - Tag (Alt+5).

## Undo and redo

`tag_core::history::Journal` (`crates/tag-core/src/history.rs`) is a session
journal. Before a command writes, every local target file is snapshotted into
`<app cache>/undo/session-<pid>-<id>/`. Staged writes atomically replace the
file, so a hard link keeps the previous inode without copying audio. If the
snapshot is on another volume, the journal makes a copy and checks it. After the
command, the journal records each file's fingerprint (size, mtime, and device
and inode on Unix), any file renames, and folder moves in the order they ran.

- Restores put back the exact previous bytes, so they never rewrite tags and
  the user's MPEG/APE write preferences cannot change what is restored. The
  staged copy is checked by reading it with the current `TagOptions`, and the
  entries returned to the UI are read through `settings::read_summary`. The
  journaled commands themselves write through `settings::write_summary` and the
  `*_with_options` variants.
- Undo/redo checks that every file still matches the fingerprint the journal
  left. A file that was changed, replaced or deleted outside OpenTag is reported
  ("Changed outside OpenTag after …") and is not overwritten. It is then dropped
  from the step. The other files in the step are still restored.
- Contents come back from the snapshot through a sibling `.opentag-*` staging
  file. The staging file is checked for length and must parse as audio. Its
  permissions are copied from the target. The target must be unchanged before
  the atomic persist. Renames go back through `rename_no_clobber`, so an
  occupied original name is refused. Folder moves are reversed as one unit,
  and a failure rolls them back.
- Redo works the same way in the other direction. The first undo of a step
  snapshots the written state (a free hard link). Neighbouring steps update
  their expected fingerprints, so you can undo or redo several steps in a row.
- Limits: 50 steps (undo and redo together), with at most 2 GiB of retained
  audio. The oldest steps are evicted and their snapshots deleted. If one
  operation would retain more than the byte bound, it still runs but is not
  recorded. The Edit menu, and the status line after the operation, then show
  the notice "… cannot be undone: the operation is too large to undo". A new
  recorded change clears redo. An operation that changes nothing records
  nothing.
- Retained bytes are what a snapshot really keeps on disk, not the file size.
  Staged writes start from `tag_core::staging_clone`. It uses the same
  explicit `clonefile` as `staging_copy` and falls back to a plain copy. It
  reports whether that file really was a clone: on APFS it is; on Linux
  (reflinks cannot be confirmed) and on Windows it never counts as one. While
  planning, `tag_core::clones_in` is checked once per folder. Writers report what they
  rewrote: the FLAC padding path reports its metadata region, every other
  writer (lofty full writes, ID3, tag removal, ID3v1) marks a full rewrite, and
  an unreported write or a changed length counts as full. A hard-linked
  snapshot whose writes were all clone-backed region rewrites costs the
  regions rounded to 4 KiB plus 16 KiB per file (capped at the file size);
  copied snapshots, full copies and full rewrites cost the whole file. After an
  undo or redo restores through a clone, the other snapshot is costed the same
  way. Files written after a rename in the same operation are counted in full.
- Before snapshotting, the journal plans the step: files on its own volume that
  clone are estimated at 1 MiB each, everything else at its full size. If the
  full-size part alone exceeds the cap, the step is not recorded. If the
  journal volume's free space is below the estimate plus a 2 GiB margin
  (`Limits::min_free`), journaling is refused with "there is not enough free
  disk space for undo". The actual cost is checked again at commit.
- Each running process locks its session directory. On startup, directories
  of sessions that are no longer running are removed. A clean exit also removes
  the current session's directory, because the `Drop` impl runs.

Journaled commands (`src-tauri/src/history.rs::record`) and their step labels:
`save_fields` (Save tags; each Save click passes one `historyGroup`, so a
multi-file save is one step), `remove_tags`, `add_cover`, `remove_cover`,
`filename_to_tags`, `rename_files` (Tag → Filename), `apply_filename_conversion`,
`apply_text_import`, `execute_action_groups` ("Action <name>" /
"Actions (n groups)"), `apply_action` (Quick action) and `convert_tag_to_tag`.
New commands: `history_state`, `undo_last`, `redo_last`.

UI: Edit → "Undo Save tags (3 files)" / "Redo …", Ctrl/⌘+Z, Ctrl/⌘+Y and
⌘+Shift+Z. These shortcuts act only when focus is not in a text field, so a
text field keeps its native text undo. Pending drafts are kept across an
undo. Restored files that are no longer listed are not added back to the list.
Provider (Android) documents are not journaled, and Undo/Redo are disabled
there with a reason.

## Tag clipboard (`src/tag-clipboard.ts`)

Copy captures the selected files' tags in table order, including pending
drafts. Fields that have been removed are dropped. A text form is also written
to the system clipboard when the webview allows it: one line per file, made of
the filename and then tab-separated `FIELD=value` items. Paste replaces each
target's tags as pending drafts that you save with Ctrl+S. With one source, it
pastes to every selected file. With n sources, it pastes 1:1 to n selected files
in table order. Any other selection count is refused with a message. Existing
native field spellings are kept, and fields that already match are left out.
Cut copies and then stages removal of every field. The shortcuts act only when
no text field has focus and no page text is selected. Paste is intercepted only
while the tag clipboard holds data.

## Convert → Tag - Tag (`src/tag-tag-ui.ts`, `convert_tag_to_tag`)

The dialog has an editable field combobox: the common fields, `_FILENAME`,
`_DIRECTORY`, and fields present in the selection. It also has a format string
input. A per-file preview (via `preview_format`) updates 250 ms after each
change. Apply stays enabled only while the preview matches the inputs and
selection. Apply runs the existing Format Value action through `action_host`,
so `_FILENAME` and `_DIRECTORY` renames and directory rebasing behave like
action groups. Changes are written immediately, as with Filename → Tag, and
can be undone. The last field and format are remembered in localStorage.
Convert menu order and shortcuts now follow Mp3tag: Tag → Filename Alt+1,
Filename → Tag Alt+2, Filename → Filename Alt+3, Text File → Tag Alt+4,
Tag → Tag Alt+5. On macOS, Option+digit is not intercepted while a text field
has focus, so you can still type characters such as ™.

## Tests

- `crates/tag-core/tests/history.rs` (10 tests) uses real WAV and hand-built
  MPEG-1 Layer III fixtures, plus FLAC through ffmpeg when it is available.
  The tests check the following:
  - Undo and redo restore the exact prior bytes, fields and cover data in all
    three formats.
  - Remove tags and add cover are undone in order, with file permissions kept.
  - A combined file rename, tag write and folder move reverses and reapplies,
    and an unselected file in the moved folder follows the folder.
  - A pure rename keeps no snapshot.
  - External edits before undo and before redo are reported and not overwritten.
  - An occupied original name is refused.
  - Grouped saves merge into one step, including the same file saved twice.
  - Failed writes and no-op operations record nothing, and a new change clears
    redo.
  - Operation and byte limits evict old steps and delete their snapshot files.
  - The too-large notice appears.
  - Stale session directories are cleaned up and a live one is kept.
  - A multi-step undo, redo, undo, redo chain restores the exact bytes at every
    step.
- `src-tauri/src/history.rs` has one command-level test. It covers a grouped
  two-file save, Tag → Tag to a field, Tag → Tag `_FILENAME` rename, three
  undos and three redos, a Tag → Filename rename, and refusal of an
  externally edited file. Only test roots it registers are journaled, so
  parallel command tests do not interleave steps.
- `tests/edit-history.test.mjs` (7 tests) covers history labels, shortcut
  mapping including text-field and selection exclusions, the macOS Option
  digits, the clipboard copy, text form, paste and cut rules, and the Tag → Tag
  field options, target validation and preview outcomes.
  `tests/document-coordination.test.mjs` now also extracts `refreshHistory`,
  which `run()` calls.
- Clone accounting (`crates/tag-core/src/{write_cost,history}.rs` unit
  tests, `crates/tag-core/tests/history.rs`): retained-cost rules,
  accumulation of several writes, sticky full rewrites, abandoned write
  scopes, free space and volume lookup. A WAV save retains the whole file. A FLAC
  padding write retains at most 64 KiB, and an overflowing FLAC write retains
  the whole file; after two undos both versions are counted. 48 clone-backed
  noise FLACs, together 24 times the test's cap of two files' size, are saved in
  parallel as one step that stays under the cap, then undone byte-exactly and
  redone. The free-space guard refuses journaling with its notice while the
  write still happens.
- Native: `scripts/e2e-undo-bulk.sh [files]` builds the debug app, generates
  N APFS clones of a ~23 MB noise FLAC (`cp -c`; the script is macOS-only), selects
  all, stages ALBUM, saves, checks that one "Undo Save tags (N files)" step
  exists without a notice, undoes, verifies every file through `scan_files`,
  and redoes. With 100 files (2.29 GB logical, above the 2 GiB cap) it passed
  5/5: the save took 614 ms, the undo 930 ms (after merging core-parity's
  clone staging: 515 ms and 1039 ms), and the volume's free space
  changed by about 57 MB. That figure is noisy because other processes were
  writing to the same disk. `OPENTAG_E2E_ID=org.opentag.editor.e2e.history
  scripts/e2e-native.sh` passed 13/13.
- Totals before merging `feat/core-parity`: frontend 108 tests (baseline
  101 + 7), Rust 290 (baseline 279 + 11). After the merge (tag panel and
  options): frontend 128 and Rust 312, all passing.
  `npx tsc --noEmit` and `cargo clippy --workspace --all-targets -D warnings`
  are clean.
- A manual browser check ran Vite with a mocked Tauri bridge. It exercised these
  flows: Save, then Ctrl+Z on a row, then Ctrl+Y; Ctrl+Z inside a text field did
  not undo the save. Ctrl+C on one file followed by Ctrl+V on another staged
  pending fields, and Ctrl+S saved them. Alt+5 opened Tag → Tag, which showed a
  live preview, applied, and produced the "Undo Tag → Tag (2 files)" label.
  The Edit menu was also checked in Dark theme.

Local note: with Homebrew binutils first on `PATH`, `regex-compat` builds a
`libopentag_regex.a` that Apple `ld` rejects ("invalid control bits"). Run
cargo with `AR=/usr/bin/ar` in that environment.

## Known gaps compared with Mp3tag

- Undo history lasts only for the session. Its depth is bounded by the 50-step
  and 2 GiB limits. Mp3tag's own limits are not documented.
- An empty folder created by a rename into a new directory is left behind
  after undo.
- Outside Unix, changes are detected by size and mtime only, with no inode.
  A same-size write within the filesystem's timestamp resolution would go
  unnoticed.
- Pasted tags do not include artwork. The status line says so when the copied
  files have covers. Mp3tag pastes covers as well.
- Edit → Copy as Text… (a format string to the clipboard) and Repeat (Ctrl+Y
  when there is nothing to redo) are not implemented. Paste reads only OpenTag's
  internal tag clipboard, not tag text from another application.
- Tag → Tag has no format-string history manager or Ctrl+Space helper menu. Its
  preview evaluates through `preview_format`, which matches Format Value
  evaluation but is not a dry run of the rename itself.
- Export-only actions are not undoable, and they do not change audio files.
