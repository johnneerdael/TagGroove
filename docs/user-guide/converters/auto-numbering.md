# Auto-numbering

Use **Auto-numbering** to assign track and optional disc numbers in the
current file-list order. Preview the sequence, stage it as pending edits,
then save. The wizard does not infer an album's intended order from audio.

## Number one album

1. Keep copies, save pending edits and arrange the files in their intended
   playing order. Select only the tracks to number.
2. Choose **Convert → Auto-numbering…** (++cmd+k++ / ++ctrl+k++).
3. Set **Begin at track** to `1` and **Minimum digits** to `2`.
4. For this trial, leave total count, disc numbers and counter changes off.
5. Choose **Preview**. Expect `01`, `02`, `03` and so on in file-list order.
6. Choose **Stage numbers**. Inspect the pending values in the main window.
7. Choose **File → Save tags** (++cmd+s++ / ++ctrl+s++) to write them.

Until the last step, the staged numbers are pending edits. Discard them if
the sequence is wrong rather than saving and trying to reconstruct the
original numbers.

## Track and disc settings

| Setting | Effect | Initial value |
|---|---|---|
| **Begin at track** | First track number and restart value | `1` |
| **Minimum digits** under Track numbers | Pad track numbers to this width | `2` |
| **Save total track count** | Add `/total` to every track value | Off |
| **Total tracks** | The total to store when enabled | Number of selected files |
| **Disc numbers** | Also write `DISCNUMBER` | Off |
| **Begin at disc** | First disc number and disc reset value | `1` |
| **Minimum digits** under Disc numbers | Pad disc numbers | `1` |
| **Save total disc count** | Add `/total` to every disc value | Off |
| **Total discs** | The total to store when enabled | `1` |

These are initial defaults. Most options are remembered after a staged
run, so check them when reopening the wizard. **Total tracks** starts with
the current selection count each time.

| Begin at track | Minimum digits | Total enabled/value | First three results |
|---|---|---|---|
| 1 | 2 | Off | `01`, `02`, `03` |
| 5 | 3 | Off | `005`, `006`, `007` |
| 1 | 2 | On, 8 | `01/8`, `02/8`, `03/8` |

In the regular wizard, totals are the numbers entered, not an automatic
count of each group. For different per-disc totals, number each disc
separately or use the per-folder shortcut described below. A minimum
width does not truncate larger numbers: width 2 still allows track `100`.

Some tag formats store track/disc components as integers and may not
retain leading zeros after saving. For consistently padded filenames,
use `$num(%track%,2)` in a filename format.

## Restart tracks in each folder

For local files, ++cmd+shift+k++ / ++ctrl+shift+k++ opens a per-folder run.
It enables directory changes and track resets, while leaving disc-number
writing off. Arrange the selection so each folder's tracks are together.
When **Save total track count** is enabled in this shortcut mode, each run
uses its own final assigned track number. Starting at `1`, a 12-track folder
and an 8-track folder receive totals `12` and `8`, rather than both `20`.
Starting at `5` with three tracks instead ends at `7`, so its total is `7`.
A selected subset or another configured reset can shorten a run; select
all intended tracks and check every boundary in the preview.

For the same track resets in the regular wizard, enable **Next disc
on change of directory** and **Reset track counter on next disc** while
leaving **Disc numbers** off. That regular mode keeps the entered total;
it does not enable the shortcut’s per-run totals. An internal disc boundary
can restart tracks without writing a disc field.

These small examples use the per-folder shortcut with total count enabled:

<!-- per-folder-numbering-examples:start -->
| Begin at track | Folder | Selected tracks | First result | Last result |
| --- | --- | --- | --- | --- |
| `1` | `Harbour` | `3` | `01/3` | `03/3` |
| `1` | `Low Tide` | `2` | `01/2` | `02/2` |
| `5` | `Bonus` | `3` | `05/7` | `07/7` |
<!-- per-folder-numbering-examples:end -->

The first two folders can share one run starting at `1`; the Bonus example
is a separate run starting at `5`. The entered **Total tracks** box is
disabled in this shortcut mode because the run determines it.

Directory grouping requires known local folders. For provider documents
without those paths, the directory control is unavailable; use a reliable
tag-field grouping or process each album separately.

## Number a multi-disc collection

The wizard compares each file with the previous file in list order:

| Counter-change setting | When it acts |
|---|---|
| **Next disc on change of directory** | Parent folder changes |
| **Next disc on change of field** | The chosen field's values change |
| **Reset track counter on next disc** | A next-disc boundary restarts tracks at **Begin at track** |
| **Reset disc counter on change of field** | Reset discs to **Begin at disc** and tracks to **Begin at track** |

For two album folders, each with two disc subfolders:

1. Sort by album, disc and intended track order.
2. Enable **Disc numbers** and begin both counters at `1`.
3. Set **Next disc on change of field** to `DISCNUMBER`, using the
   existing disc tags to identify the boundaries.
4. Enable **Reset track counter on next disc**.
5. Set **Reset disc counter on change of field** to `ALBUM`.
6. Preview the first and last rows of every disc before staging.

A disc reset takes priority if both a next-disc and reset condition occur
at the same boundary. A field's missing value also participates in these
comparisons; clean inconsistent album/disc tags first.

## Gotchas

- Grouping detects transitions, not all equal values across the selection.
  Interleaved albums cause repeated resets; keep groups together.
- Disc settings do not change tracks unless their configured boundary or
  reset rule acts. Preview instead of assuming enabling discs groups files.
- Preview shows at most 100 rows, although the plan covers the selection.
  Use smaller selections when the unshown boundaries need inspection.
- Start and total values must be positive whole numbers. Minimum digits
  must be 1–10; counter overflow reports an error.
- Keep original copies and inspect **Edit → Undo** for recorded changes
  after saving. Staging alone is not proof that tags were written.

<!-- verified: crates/tag-core/src/numbering.rs:plan_numbering; src/numbering-review.ts:numberingDefaults, recallNumbering; src/main.ts:numbering -->

## Related

- [Tag → Filename](tag-to-filename.md)
- [Tag → Tag](tag-to-tag.md)
- [Zero-padded track numbers](recipes/rename-zero-padded-track-numbers.md)
- [Converters](index.md)
