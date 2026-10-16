# Keep compilation tracks together when renaming

<div class="recipe-meta" markdown>
**Uses:** Tag → Filename, `ALBUMARTIST` fallback and explicit track credits.

**Works on:** Copied local album tracks with reviewed artist/album metadata.

**Time:** About 10 minutes for a small compilation.

**Undo:** Apply changes paths immediately; inspect recorded session **Edit → Undo** and retain untouched copies.
</div>

## The problem

A compilation has a common album title but different artists on its tracks.
An outer `%artist%` folder can scatter those tracks across several folders.
Choose an album-level folder credit while retaining each track's own artist
in the filename. A compilation flag and an album-artist credit are separate
fields; one does not automatically supply the other.

Use copies, check the selection and resolve pending tag edits first. This
practice album is `Harbour Sessions`. Where it represents your intended
credit policy, give all its tracks `ALBUMARTIST` = `Various Artists` while
keeping their different `ARTIST` values. That text is a chosen credit, not a
special command or a guarantee about player grouping.

## The solution

1. Inspect and deliberately save the chosen `ALBUMARTIST`, `ARTIST`, `ALBUM`,
   `TRACK` and `TITLE` values on the copied album. Do not change credits just
   to force a filename convention without deciding what those fields mean.
2. Select the intended copies. Hidden selected rows can remain in a batch,
   so use **Edit → Unselect all** first if the active filter obscures scope.
3. Choose **Convert → Tag → Filename…** (++alt+1++; Alt is ⌥ Option on macOS).
4. Enter:

    ```text
    $if2(%albumartist%,%artist%)\%album%\$num(%track%,2) - %artist% - %title%
    ```

5. Choose **Preview**. All rows below use `ALBUM` = `Harbour Sessions`.
   Relative paths resolve under each source file's current containing folder;
   `/` shows folder levels. The rows without album artist demonstrate fallback
   splitting even when `COMPILATION` is set.

    Scroll the table sideways on a narrow screen to read the complete destinations.

    <div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Compilation path examples">

    <!-- compilation-folder-cases:start -->

    | Artist | Album artist | Compilation | Track | Title | Source name | Relative destination |
    |---|---|---|---|---|---|---|
    | `Juniper Vale` | `Various Artists` | `1` | `1` | `Paper Boats` | `old.mp3` | `Various Artists/Harbour Sessions/01 - Juniper Vale - Paper Boats.mp3` |
    | `Mira Solberg` | `Various Artists` | `1` | `2` | `Low Tide` | `old.mp3` | `Various Artists/Harbour Sessions/02 - Mira Solberg - Low Tide.mp3` |
    | `Juniper Vale` | Absent | `1` | `1` | `Paper Boats` | `old.mp3` | `Juniper Vale/Harbour Sessions/01 - Juniper Vale - Paper Boats.mp3` |
    | `Mira Solberg` | Absent | `1` | `2` | `Low Tide` | `old.mp3` | `Mira Solberg/Harbour Sessions/02 - Mira Solberg - Low Tide.mp3` |
    | `Mira Solberg` | `Various Artists` | Absent | `2` | `Low Tide` | `old.flac` | `Various Artists/Harbour Sessions/02 - Mira Solberg - Low Tide.flac` |

    <!-- compilation-folder-cases:end -->

    </div>

6. Verify the common album folder, individual filenames, source extension,
   destination root and existing destination contents. Correct any missing
   album/artist/title data or repeated names before applying.
7. Choose **Apply to … files** for the checked practice plan. Inspect the
   results and errors. The converter changes paths immediately and leaves
   metadata credits as supplied; it does not need a second **Save tags** step.

## How it works

`$if2(%albumartist%,%artist%)` chooses the first album-artist value when it
contributes text. If it does not, the first track artist becomes the outer
folder. It is a per-file fallback, so a compilation with missing album artists
can split exactly as the third and fourth rows show.

The rest of the format creates the album folder, a padded track number and
an explicit track credit/title. `COMPILATION` does not appear in this format,
so changing that flag does not change these calculated destinations. In its
constrained ID3/MP4 representations, enable the flag with `1` or remove it;
see [sort fields and compilations](../concepts/sort-fields.md) before editing.

A field with several values contributes its first value to a plain placeholder.
Do not expect the filename to list every artist unless you deliberately choose
and test an explicit join. Slashes inside ordinary values use the configured
folder-separator replacement; literal separators in the format create folders.

## Variations

If you want one fixed outer folder independent of any album-artist spelling,
use this complete alternative after checking that the whole selection belongs
there:

```text
Compilations\%album%\$num(%track%,2) - %artist% - %title%
```

The first practice row becomes
<!-- compilation-folder-fixed-result -->`Compilations/Harbour Sessions/01 - Juniper Vale - Paper Boats.mp3`.
This literal path policy does not write a compilation flag or album artist.

For a multi-disc compilation, retain disc identity too:

```text
$if2(%albumartist%,%artist%)\%album%\CD$num(%discnumber%,2)\$num(%track%,2) - %artist% - %title%
```

With the first practice row and `DISCNUMBER` = `2/2`, expect
<!-- compilation-folder-disc-result -->`Various Artists/Harbour Sessions/CD02/01 - Juniper Vale - Paper Boats.mp3`.
Check disc tags first: missing/nonnumeric values can produce `CD00`. The
[multi-disc recipe](rename-multi-disc-albums.md) explains optional disc folders.

## Recovery and common mistakes

All tracks sharing this relative format only meet in one destination when
their starting roots also suit that plan. Files from different source folders
can create parallel layouts; choose a real absolute archive root deliberately
when consolidation is intended. Reapplying a relative expression after a move
can nest the same hierarchy again.

A numbered filename is not proof of correct album order. `$num` reads each
stored track; it does not run auto-numbering or infer totals. Identical
metadata can produce identical target names. Inspect the preview and actual
folder contents without assuming every filesystem alias or collision is
covered by one naming convention.

Sidecar art, playlists and old empty folders are separate from this audio-file
move. Check **Keep moved files** when an entry leaves the list. Review any
partial operation errors, inspect recorded **Edit → Undo** while available,
and keep originals for recovery if paths later become occupied.

## Related

- [Sort fields and compilations](../concepts/sort-fields.md)
- [Artist/album folder layout](rename-into-artist-album-folders.md)
- [Multi-disc layout](rename-multi-disc-albums.md)
- [Multi-value fields](../concepts/multi-value-fields.md)
- [Tag → Filename](../converters/tag-to-filename.md)

<!-- verified: folder_recipe_guide.rs executes all five written primary rows and fixed-folder/disc variants with actual core filename mode and isolated actual filename_target. LF/CRLF; metadata inputs retained. No native UI/player/move or universal platform collision/alias proof. -->
