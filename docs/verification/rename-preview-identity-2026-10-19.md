# Rename preview replacement identity — 2026-10-19

Consolidated PR64 fixes a reproduced changed-file admission gap in ordinary
Tag → Filename previews when associated-file handling is off. The receipt stored
size and modification time only. Replacing the source with a different file of
the same size and timestamp admitted the old destination computed from the old
file's tags. The opt-in companion path already checked Unix identity.

The [ordinary receipt](../../src-tauri/src/rename_plan.rs) now retains and checks
the same Unix device/inode identity alongside its existing size/time checks.
Mismatch uses the existing changed-file refusal and requires a fresh preview.
Selection/template/expiry and single-use behavior remain unchanged. No new
dependency, persistent schema or public error is introduced.

An actual-source disposable probe first demonstrated acceptance of a replacement
with different bytes and inode but exactly equal length/mtime; unchanged,
changed-length and changed-mtime controls behaved as expected. The permanent
unit regression then failed on the old implementation for that same acceptance.

The [production command test](../../src-tauri/src/lib.rs) uses two valid WAV files
with equal-length `AAAA`/`BBBB` titles. It obtains the frozen destination through
the actual metadata/naming code, replaces the file, restores its exact mtime and
proves size/time equality plus different identity. The ordinary reviewed rename
refuses without altering either audio file or publishing either destination.
A fresh review names the replacement `BBBB.wav`; journal Undo and Redo restore
the corresponding full bytes exactly. Both focused regressions and all 24
backend tests selected by `rename` pass. Strict all-target Clippy, formatting,
whitespace and strict MkDocs pass. Independent local reviews found no remaining
confident finding in this slice.

This is a local Unix backend/filesystem result, not native GUI or original-app
qualification. Non-Unix behavior is unchanged; Windows work is deferred.
Same-object edits restoring size/time and concurrent mutations between validation
and rename remain outside the bounded guard. L22a/L22b/L78/L79 remain open for
their distinct companion, transfer and original/native contracts. Final genuine
GitHub Codex code review and current-main validation remain required before merge.
