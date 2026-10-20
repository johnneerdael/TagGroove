# Save before navigation: L47c working branch

This records unmerged work on `codex/refresh-file-identity`. The intake stays
open until a covering PR exists. Local review and checks do not substitute
for final-head GitHub Codex review, Linux CI, merge or native acceptance.

## Static original evidence and limits

The original is Windows Mp3tag 3.36.1 x64, executable SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Read-only PE disassembly establishes these call paths:

- Navigation guard `0x1400db9ec` calls the named
  `CMTMainFrame::HandleFileSaveWithWait` at `0x14017acf8` before permitting
  navigation. Its completion check can refuse navigation. The exact runtime
  meaning of completion result `2` remains unqualified.
- Key wrapper `0x1400c4c80` guards Page Up/Down, Home/End and Up/Down.
  Mouse wrapper `0x1400c6fb0` guards the relevant left-button list messages.
- Select Next (`0x140182444` → `0x14018bad8`), Select Similar
  (`0x14018c71c`), Select Next Similar (`0x14018ca64`) and Invert Selection
  (`0x140185a30`) call the guard before selection changes.
- Named OnViewSortBy (`0x14018b150`) and OnViewSortByRandom
  (`0x14018b27c`) call `0x1400de064` → the same guard before ordering changes.

The original's AutoSave/MsgNotSaved branches depend on their caller; they do
not establish one universal prompt/discard policy. Inspected Select All and
Find handlers show no direct guard. Original inline Enter/Tab and directory
selection remain unqualified. Their existing TagGroove exclusions are
retained, not presented as newly proved defects. Extended Tags owns its
separate save mode (L73). No original executable or Windows UI was run.

## Working implementation

[The shared guard](../../src/main.ts) computes the proposed selection before
mutation, holds a pending-change lock and waits for the current local
selection's save. It requires successful completion and no remaining target
drafts/artwork before applying the change. Failed, canceled, partial, busy or
superseded selection requests retain the current selection/order. Successfully
saved members of a partial batch remain saved; other members retain drafts.

This uses the existing off-by-default save-on-move preference and excludes
provider documents. It does not replace TagGroove's pending-draft model with
an unproved original prompt/discard model. Same-selection sorting, shuffle,
load order and playlist order use the guard explicitly. Direct column-divider
resizing does not cause a save; applying Customize Columns from a manual or
playlist order can switch to column sorting even if only a width was edited.
Refused order-affecting column layouts reject
the application promise so Customize Columns preserves the edited layout
and can retry through its existing error handler.

Mouse and keyboard selection, extension/toggle, Ctrl+N, invert/similarity and
unselect routes apply only after the guard accepts them. The guard preserves
relevant row/tag-field focus across save rendering. Separate selection owners
are not overwritten by an older pending request. `saveDrafts` includes
cover-only changes and reports actual completion.

[Row dragging](../../src/list-behaviour.ts) waits for accepted selection before
freezing moved members. Gesture listeners live on the stable owner document;
the host supplies the current list wrapper after a save replaces the view.
Release/cancellation while waiting prevents a drop. Busy/disconnected views
cannot accept the reorder.

## Focused validation

The initial production-function tests reproduced selection changing before a
pending/failed save, busy selection bypass, same-selection sort/shuffle bypass,
missing cover-only save items and unguarded order modes. Read-only review then
identified the detached drag wrapper and Customize Columns losing an unapplied
layout; separate regressions failed before both fixes and passed afterward.
An additional production click-handler regression keeps rejected busy checkbox
activation consistent with the retained selection.

The final affected headless cohort passed **138 tests** across navigation,
dragging, playlist Refresh/order, list behavior, Extended Tags and document
coordination. It runs actual production functions with controlled IPC, pending
promises and DOM replacement. It does not open the application, run native UI,
write audio or contact a service. TypeScript also passed. The final source
re-review found no actionable issues. Catalog extraction, ten focused locale
tests, the strict guide build and diff checks passed; the five ledger views
retain matching active identities.

The guide now explains writing on sort/shuffle, failure/partial-save behavior,
drag cancellation and the explicit exclusions. Windows-original outcomes,
physical keyboard/pointer/focus acceptance and other desktop qualification
remain separate under L78/L79 and the platform handover.
