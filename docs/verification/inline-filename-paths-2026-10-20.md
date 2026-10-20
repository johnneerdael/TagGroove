# Literal inline filename destinations — 2026-10-20

C64 represents the inline-destination part of L47a. Implementation checkpoint:
`6465fb5572054b555b541a4c9436517c94c8b760`, based on merged PR67 main
`69cd9f063c66290adbb6b3db3a70b2d5a1ca4592`. Discovering an externally renamed,
individually loaded file on Refresh remains active L47a work.

## Original static contract

REA 6.4.0/Ghidra 12.1.4 inspected the copied Windows x64 Mp3tag 3.36.1
executable, SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The original was never executed. The 19-record retained bundle is
`/tmp/taggroove-l47a-mp3tag-inline-evidence-final-2026-10-20.json`, SHA-256
`9671b9337400c61062be33af6e08f00b2dcbedb54f40f17295a3bd5391642957`.
Provider profile:
`378a660c321a42ed1615fe68445f2948bc93110be7e9858c2809914591120567`.

These are **seen static facts**:

| Anchor | Observation | Evidence ID |
| --- | --- | --- |
| `0x1400ddcbc` | `CMTFileView::RenameSelectedFile` selects a row and calls `0x1400c74b4`. | `ev_c470957db72d3fedee3d94c194ffe0f8e69b711ba76305a7c76ba0194b6bfa18` |
| `0x1400c74b4` | The edit route prefers a filename/filename-with-extension column before another editable field. | `ev_30abf9b3238dfbfdb4d83406d7aa8e4c1bd74c2cf9a48c5badce355bcf8cc454` |
| `0x1400c3bb4` | The File List callback distinguishes `_filename` and `_filename_ext` and passes the corresponding flags to `0x1400d737c`. | `ev_ec7b94616f4d9af71a76dc242f96108ea2d2493361b657eec6238880919d86ec` |
| `0x1400d737c` | Its filename branch combines the current folder with edited text, appends the original extension when the suffix differs, splits folder/name, and calls folder creation before the document rename route. | `ev_56be27438c58475c30ed926cfc63ebb42ee52bd7eb42a4add350d693dd22b7a8` |
| `0x1402a29a8` | The extension helper searches for the last dot and obtains the suffix. | `ev_d664bf6cf2d384c13ac3e5e9adb81d4b16be914f50a0a8235fa3e25877a096c6` |
| `0x1402aaf8c` | `SFolder::CreateFolder` calls `CreateDirectoryW` and recurses over backslash-delimited parents. | `ev_c3f6fe2d9882602b7efbf82a9bc71f928f9467fe1526905e0b32b393c8d6223c` |
| `0x1400a1d08` | `CMTDocument::RenameFileInternal` updates model identities, name/folder fields and stored path references. | `ev_018c2a6da1cdedff36d38a2346fe61d0eee5f58c002a98f6e1ffb849789c94bf` |

Nested relative destinations are **inferred** from that route; exact GUI
cleanup, absolute/UNC/extended input, collisions, companions, cross-volume
behavior and external-rename discovery remain **unknown** original runtime
results. The separately retained Mac Mp3tag 1.14.1 trace substitutes slashes
in its inline route. It is a different platform/version contract, not proof
that the Windows target uses that policy.

## Implemented behavior

The [File List host](../../src/main.ts) sends raw text to the registered
`rename_file_from_list` command in the [desktop adapter](../../src-tauri/src/lib.rs).
It does not evaluate that text as a converter template. Explicit native folder
paths are supported across desktop platforms; this is TagGroove's stated
policy, not a claim of identical original Mac behavior. Windows accepts its
native backslash and normalizes forward slashes. Unix keeps literal
backslashes and uses `/` for folders.

The existing audio suffix is retained once when it matches without case, or
appended when it differs. Missing filename leaves, including an explicit final
`.` or `..`, are refused before suffix handling. Intermediate dot navigation
is allowed. Successful receipts use the canonical readback row identity, with
a filesystem fallback when readback fails, so destination aliases do not
produce duplicate rows or stale source roots.

Inline operations stay audio-only and share collision checking, no-clobber
filesystem execution and history with the existing rename executor. Converter
templates retain their existing separator/preview/companion policy. No
side-file behavior is invented for L22a. Pending edits, selection and source
roots follow known inline renames; unsignaled external renames are a separate
Refresh problem. The [guide](../user-guide/guides/file-list.md#edit-a-cell)
explains the immediate operation, extension examples and Undo.

## Focused execution evidence

- Frontend RED: three actual assertion failures for old converter transport,
  rejected folders and rejected backslashes/suffixes
  (`taggroove-l47a-inline-frontend-red-gzx9i0d3`).
- Backend RED through the old inline command: the owned Unix filename
  `Björk\Live.wav` became a directory path instead
  (`taggroove-l47a-inline-backend-red-yvgfzy9o`). The test was then migrated to
  the new registered literal command with the same filesystem oracle.
- Review RED: unresolved dot-navigation receipt identity and accepted terminal
  `/.` both failed actual assertions; two controls passed
  (`taggroove-l47a-inline-review-red-xpi962iw`).
- Corrected backend: eight focused tests pass, 592 unrelated tests filtered
  out, 1.70s harness time (`taggroove-l47a-inline-canonical-history-mnjrv9r9`).
  This includes byte-checked Undo/Redo through dot and owned Unix symlink
  destinations. Six related converter/rename regressions passed separately
  after the shared-executor extraction
  (`taggroove-l47a-inline-converter-regression-78rtz6yy`).
- Frontend: 60 focused production-host/unit tests pass
  (`taggroove-l47a-inline-frontend-verified-a70ny72e`), including one row,
  selection/draft/root identity and later Refresh after a canonical receipt.
  Before/after hashes agree in
  `/tmp/taggroove-l47a-frontend-final-source-binding.json`.
- TypeScript, production-library Clippy with warnings denied, Rust formatting,
  catalog extraction and the strict guide pass. The final Rust-only change
  after Clippy strengthens Undo/Redo assertions; hosted all-target validation
  remains a separate final-head requirement. The two obsolete refusal messages
  were removed from all three catalogs without reordering translations.

The final compiled adapter SHA-256 is
`abbce25acdd60e58e2387f693d285c07a78d1f6a2bd4c15c041be7fb0b2d993a`;
the retained GREEN binding matches it. This is macOS execution of TagGroove
policy and owned fixtures, not original Windows execution.

The existing postmerge [native suite](../../tests/e2e/rename-fixes.js) gains a
registered-command case for literal folder destinations, suffix retention,
native backslashes, canonical dot navigation, metadata readback, Undo and
typed collision refusal. It is syntax-checked only and has **not run locally**.
It does not prove a physical F2/keyboard/accessibility journey. No local app
build, UI replay or full Rust/frontend sweep was run for this follow-up.
Independent code review found two path edges, both reproduced and corrected;
re-review and the native-case review are clear.

Eight documentation publication-boundary tests and writing-plan validation
pass. The five ledger views agree on 48 active identities, 64 represented
scopes and the unchanged 14 qualification identities. L47a remains active for
external-rename discovery only. Documentation review corrected the stale W5a
assignment so implemented inline destinations are no longer described as
unfinished separator support.
The final local link audit checks nine changed Markdown files and 640
links/anchors with zero errors. The W5a correction has a clear re-review.

## Engineer handover and remaining work

Use Windows with the pinned original and a source/executable-bound TagGroove
build to compare both editable filename columns, nested folders, extension
case/doubling, literal syntax characters, invalid leaves, occupied targets and
Undo. Record actual input, result paths, bytes and readback. Include absolute,
drive-relative, UNC/extended, symlink/junction and cross-volume cases. Original
Windows execution remains owner-deferred.

Use each desktop OS for actual installed-path permissions and physical
keyboard/focus/accessibility acceptance under L78; Unix symlink fixture tests
do not certify every installed filesystem. Postmerge CI must execute its new
command case before an integration result is claimed. L47a still needs local
source-identity research/implementation for discovering an externally renamed
individually loaded file. Do not replace that file with a guessed neighbor or
hand off the unfinished portable implementation as Windows-only work.
