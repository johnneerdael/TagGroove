# Keep multi-disc albums distinct when renaming

<div class="recipe-meta" markdown>
**Uses:** Tag → Filename, `DISCNUMBER`, `TRACK` and `$num`.

**Works on:** A copied local album with checked disc and track numbers.

**Time:** About 10 minutes for a two-disc practice selection.

**Undo:** Apply changes paths immediately; use recorded session **Edit → Undo** where available and keep untouched originals.
</div>

## The problem

Disc 1 and disc 2 both start with track 1. They may even both contain a title
such as `Intro`. A filename made from track and title alone can therefore
produce the same target twice. Keep the disc number in the folder or filename
instead of changing the original track numbering to hide that information.

Start with copied files in one practice folder. Save or deliberately discard
pending edits. Inspect `ALBUM`, `DISCNUMBER`, `TRACK` and `TITLE` in **Extended
tags**; the converter reads tags, not a guessed disc number from the filename.

## The solution

1. Select the copied album. Confirm the intended selection, including any
   rows hidden by a filter.
2. Choose **Convert → Tag → Filename…** (++alt+1++; Alt is ⌥ Option on macOS).
3. Enter:

    ```text
    %album%\CD$num(%discnumber%,2)\$num(%track%,2) - %title%
    ```

4. Choose **Preview**. In the following table, all inputs use `ALBUM` =
   `Harbour Lights`. Destinations are relative to each source file's current
   containing folder, with `/` showing folder levels. The absent-disc row
   demonstrates a result to correct before applying, not an inferred disc.

    Scroll the table sideways on a narrow screen to read the complete destinations.

    <div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Multi disc path examples">

    <!-- disc-folder-cases:start -->

    | Disc number | Track | Title | Source name | Relative destination |
    |---|---|---|---|---|
    | `1/2` | `1/8` | `Intro` | `old.mp3` | `Harbour Lights/CD01/01 - Intro.mp3` |
    | `2/2` | `1/9` | `Intro` | `old.mp3` | `Harbour Lights/CD02/01 - Intro.mp3` |
    | `12` | `10` | `Last Harbour` | `old.flac` | `Harbour Lights/CD12/10 - Last Harbour.flac` |
    | Absent | `1` | `Intro` | `old.mp3` | `Harbour Lights/CD00/01 - Intro.mp3` |

    <!-- disc-folder-cases:end -->

    </div>

5. Confirm actual disc metadata, folder roots and every complete destination.
   If a disc value is missing or wrong, leave the preview and correct/save
   that metadata deliberately. Then run Preview again.
6. Apply only the checked small plan with **Apply to … files**. Inspect both
   disc folders and operation errors. Paths change immediately; **Save tags**
   is not an additional rename confirmation.

## How it works

`%album%` supplies the album folder. The literal `CD` and the padded leading
disc number create `CD01`, `CD02`, and so on. The next backslash starts the
filename within that disc folder. Track padding starts again from the actual
track tag of each file; the converter does not renumber the album.

A stored slash value such as `2/2` is one text value: `$num` reads its leading
2 and does not include the total. An absent/nonnumeric disc also yields
numeric zero, producing `CD00`. Width 2 is a minimum, not a two-digit limit.
No total is inferred from how many files are currently selected.

The two `Intro` examples now have distinct written destinations because their
disc folders differ. This illustrates the naming rule; it is not a universal
collision or overwrite guarantee. Review existing files, actual filesystem
behavior and any error reports before trusting a larger reorganization.

## Variations

If disc folders should be omitted for files whose disc field is absent,
choose this explicit conditional after reviewing that policy:

```text
%album%\$if(%discnumber%,CD$num(%discnumber%,2)\,)$num(%track%,2) - %title%
```

For `TRACK` = `1`, `TITLE` = `Intro` and no disc field, the relative result is
<!-- disc-folder-no-disc-result -->`Harbour Lights/01 - Intro.mp3`. With disc `2/2`, it is
<!-- disc-folder-with-disc-result -->`Harbour Lights/CD02/01 - Intro.mp3`. The backslash belongs inside the selected
branch; the empty branch contributes no extra disc folder. A present text
value `0` still takes the present branch and creates `CD00`; this is a presence
condition, not a test that the number is valid.

To keep one album folder while preserving disc identity in each filename:

```text
%album%\$num(%discnumber%,2)-$num(%track%,2) - %title%
```

For the second practice row, expect <!-- disc-folder-flat-result -->`Harbour Lights/02-01 - Intro.mp3`.
This changes the path convention, not the saved `DISCNUMBER` or `TRACK`.
If you prefer artist folders too, build that outer level after following the
[artist/album recipe](rename-into-artist-album-folders.md).

## Recovery and common mistakes

A relative path is anchored at each input's containing folder. Files already
in separate disc folders do not automatically converge on one album root;
a relative expression can create another nested album beneath each disc.
Use an intentionally chosen absolute root when consolidation is the goal.

Do not include the source extension unnecessarily: the host retains it.
Moving only audio files does not promise to move folder images, cue sheets,
playlists or other sidecars. Depending on **Keep moved files**, a moved track
can leave the list while still existing at the shown destination.

Changing the expression or selection requires a new preview. Applying can
report partial outcomes; inspect which files moved. Use **Edit → Undo** for
recorded renames in the same desktop session, verify the original paths, and
keep copies for recovery if a later file occupies an old name.

## Related

- [Track and disc numbers](../concepts/track-and-disc-numbers.md)
- [Disc numbers from folders](disc-numbers-from-folders.md)
- [Rename files and folders lesson](../learn/rename-files-and-folders.md)
- [Tag → Filename](../converters/tag-to-filename.md)

<!-- verified: folder_recipe_guide.rs executes four written primary rows, conditional missing/present/zero disc cases and flat naming through actual filename mode; isolated actual filename_target supplies extension/root policy. LF/CRLF tested. No real moves or universal collision/alias claim. -->
