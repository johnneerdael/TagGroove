# Automation context qualification — 2026-10-18

Two regressions in the [actual evaluation/action adapters](../../src-tauri/src/evaluation.rs)
extend N04 using independently authored disposable WAV files. Neither test
reimplements scripting, regex or file persistence.

- Change TITLE, queue a deferred export, then store `$columnNames` and
  `$columnValues` in COMMENT and `$list` with an explicit fourth delimiter in
  AUDIT_LIST. Independently reread the saved fields and exported text. Both
  use recomputed final-state columns; preserved artwork and RIFF audio bytes
  are compared separately.
- Export an unchanged file with a deliberately different displayed column
  snapshot. The export uses that snapshot while raw `$list` values remain
  actual metadata. Exercise both explicit and default multivalue delimiters
  through the real column adapter. The entire source file remains unchanged.

The focused `automation_` run passed both tests. No production adapter defect
was reproduced, so no scripting semantics or adapter implementation was
changed. The broader integrated checks must be rerun after the consolidated
branch freezes. An intermediate broader rerun stopped before execution while
another owned change required lockfile reconciliation; that attempt supplies
no passing-test evidence.

This narrows the tested TagGroove backend-context matrix. It does not prove
original `$columnValues` recursion, literal/nonboolean truth, implicit case
defaults, native input, every context or original action-file re-import.
Existing snapshot/Boost/MTA preservation implementations are not reassigned.
N04 and the original-semantics/interchange requirements remain open.
