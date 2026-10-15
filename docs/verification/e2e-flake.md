# core-workflows first-save flake (Linux native e2e)

## Symptom

`tests/e2e/core-workflows.js` sometimes failed on Linux CI with the same five
failures: the first save (`edit title + Cmd+S writes file`) timed out, then
undo, redo, Refresh and the MusicBrainz dialog failed too. The last three are
knock-on failures. The first edit stayed pending, so F5 asked a native
"discard pending changes?" question and Tag Sources was disabled ("Save
pending changes first").

## Root cause (app bug)

When you click the only selected file a second time, the File List schedules
its in-place cell editor 550 ms later, the Explorer-style slow-click rename.
Nothing cancelled that timer when focus moved somewhere else in the meantime.
The suite does exactly this. The file is auto-selected after loading, step 1
clicks its row, and step 2 focuses the tag panel's Title field and types. If
step 2 reached Ctrl+S more than 550 ms after the click, which happens on a
slow cold first launch, the timer fired first. The cell editor took focus from
the Title field. Its keydown handler stops propagation, so the window shortcut
never saw Ctrl+S and the save never ran.

A real user hits the same thing by clicking a selected file, then quickly
clicking into a tag panel field and typing. The keystrokes end up in the
filename editor.

Evidence from a reproduction with focus tracing (CI run 38723040059, runner 18,
run 2):

```
990ms  focusin INPUT#tag-panel-0:TITLE
1239ms focus() INPUT.cell-editor[Edit Filename] from CellEditor.start < slow-click timer
1240ms focusout INPUT#tag-panel-0:TITLE -> INPUT.cell-editor
1271ms keydown@document s target=INPUT.cell-editor   (never reaches window)
FAIL edit title + Cmd+S writes file — … focus=INPUT.cell-editor[Edit Filename]
```

It only reproduced on cold runs: the first launch on a fresh runner, or after
clearing the per-user caches and the page cache. Warm repeats on one runner
passed 45 of 45 times. Run 38722293598 (15 fresh runners) failed once, on the
first launch.

## Fix

- `slowEditStillWanted` (src/cell-editor.ts): the delayed edit starts only if
  the clicked row still has the keyboard focus, the file is still the only
  selection and no operation is running.
- Any key press cancels a pending delayed edit (src/main.ts).
- Unit test in tests/list-commands.test.mjs. core-workflows has a new step
  that clicks the selected row, focuses Title and waits 800 ms. It fails
  every time without the fix.
- core-workflows timeouts now report the window state at the timeout and a
  timeline of changes (busy, status text, open dialogs, pending count,
  selection, enabled save/undo/redo, focused element).
