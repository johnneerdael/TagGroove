# Number tracks separately in each folder

Give each album folder its own sequence, starting at `01`. The per-folder
shortcut can also store each contiguous run's final number as its total.
It uses the selected files' current list order, not filename guessing or
an audio-based track-order lookup.

## Arrange and select the intended tracks

Work on copied local files. Sort or manually arrange them so each folder's
tracks are together and each track is in the intended playing order.
Select every file that belongs in the run. The numbers below illustrate
five files in this order:

<!-- folder-numbering-cases:start -->
| File in list order | Existing TRACK | Pending TRACK after staging |
|---|---|---|
| `/Practice/Harbour Lights/Paper Boats.mp3` | `9` | `01/3` |
| `/Practice/Harbour Lights/North Pier.mp3` | `4` | `02/3` |
| `/Practice/Harbour Lights/Quiet Harbour.mp3` | `7` | `03/3` |
| `/Practice/Low Tide/Evening Ferry.mp3` | `8` | `01/2` |
| `/Practice/Low Tide/Lantern Wake.mp3` | `2` | `02/2` |
<!-- folder-numbering-cases:end -->

The paths illustrate folders, not paths to create verbatim. On Windows,
use the files' actual drive/folder paths. Existing track values do not decide
the new order: this example intentionally starts with inconsistent numbers.

## Open the per-folder run

1. Save or discard existing pending edits and check the selected count.
2. Press **⌘+Shift+K** on macOS or **Ctrl+Shift+K** on Windows/Linux to open
   the per-folder Auto-numbering run for local files.
3. Set the controls below. Check both field-change controls even if this
   is a familiar wizard: remembered settings can otherwise introduce resets.
4. Choose **Preview** and inspect the first and last row of each folder.
5. Choose **Stage numbers**, inspect the pending tags, then **File → Save tags**.

<!-- folder-numbering-options:start -->
| Control | Value |
|---|---|
| Begin at track | `1` |
| Minimum digits under Track numbers | `2` |
| Save total track count | On |
| Disc numbers | Off |
| Next disc on change of directory | On |
| Next disc on change of field | Do not change counter |
| Reset track counter on next disc | On |
| Reset disc counter on change of field | Do not change counter |
<!-- folder-numbering-options:end -->

The shortcut presets directory transitions, track resets and no disc writing.
This can restart tracks without changing an existing `DISCNUMBER`. With
**Save total track count** on in this shortcut mode, the entered **Total
tracks** box is disabled: the run determines the total.

**Stage numbers** is not a disk write. Discard the selected files' pending
changes if the sequence is wrong, or rerun a reviewed preview before saving.
An enabled selection-change saving preference can write pending values when
moving on; review [saving behavior](../../getting-started/editing-and-saving.md).

## Understand what “total” counts

The total is the last track number assigned before the counter restarts,
not always the number of selected files. Each example here is a separate
run in one folder with the same width/total settings:

<!-- folder-total-variants:start -->
| Begin at track | Selected files | First result | Last result |
|---|---|---|---|
| `1` | `3` | `01/3` | `03/3` |
| `5` | `3` | `05/7` | `07/7` |
| `1` | `1` | `01/1` | `01/1` |
<!-- folder-total-variants:end -->

A single selected bonus track does not inherit a twelve-track album's total.
A start of `5` with three selected tracks ends at `7`. If an entered fixed
total is required, use the regular **Convert → Auto-numbering…** dialog and
review its total separately; recreating its directory/reset switches does
not enable the shortcut's per-run total mode.

## Check folder boundaries and order

Grouping detects a change between adjacent selected files. It does not first
collect all equal folder paths. An order of Harbour, Low Tide, Harbour creates
three counter runs. The last Harbour item can therefore restart at `01`
with total `1` even though other Harbour files occurred earlier. For example:

<!-- folder-interleaved-cases:start -->
| File in list order | Pending TRACK |
|---|---|
| `/Practice/Harbour Lights/Paper Boats.mp3` | `01/1` |
| `/Practice/Low Tide/Evening Ferry.mp3` | `01/1` |
| `/Practice/Harbour Lights/North Pier.mp3` | `01/1` |
<!-- folder-interleaved-cases:end -->

A filter or partial selection can omit tracks from a run. Preview shows at
most 100 rows while the plan covers the selection; use smaller batches when
an important boundary is beyond those visible rows. Changing selection or
settings invalidates the previous preview; run Preview again.

Local folder grouping requires known local paths. Provider-backed documents
without those paths cannot use the directory control. Use the supported
regular grouping workflow or number each intended album selection separately.
Do not infer provider rename or full action support from this wizard.

## Check storage and recovery

Numbering updates metadata, not filenames. Some formats store track/disc
components as integers and can lose leading-zero padding on read-back. For
padded filenames, use `$num(%track%,2)` in a separate reviewed
[Tag → Filename](../tag-to-filename.md) conversion.

After saving, inspect errors and the saved tags. A partial failure can leave
some numbers pending. Use **Edit → Undo …** for recorded saved changes in the
current desktop session, and keep the original files for recovery afterward.

## Related

- [Auto-numbering reference](../auto-numbering.md)
- [The file list](../../tagging/file-list.md)
- [Editing, saving and undo](../../getting-started/editing-and-saving.md)
- [Zero-padded filenames](rename-zero-padded-track-numbers.md)

<!-- verified: crates/tag-core/src/numbering.rs:plan_numbering; src/numbering-review.ts:recallNumbering,numberingTotal,createNumberingReview; src/main.ts:numbering. Exact written controls/paths/results run through actual model/core; no native UI claim. -->
