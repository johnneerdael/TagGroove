# Move tracks into artist and album folders

<div class="recipe-meta" markdown>
**Uses:** Tag → Filename, optional year text and padded track numbers.

**Works on:** Copied local audio files with checked artist, album and title tags.

**Time:** About 10 minutes for a small album.

**Undo:** Apply changes paths immediately; inspect recorded **Edit → Undo** in the same desktop session and keep originals.
</div>

## The problem

Your copied tracks sit together in an incoming folder. You want an artist
folder, an album folder and numbered filenames. This recipe builds each
relative destination under that track's current containing folder. It does
not gather files from several source folders into one archive automatically.

Check the tags first. Keep originals outside the practice folder, resolve
pending edits deliberately, and choose an album with useful `ARTIST`, `ALBUM`,
`TRACK` and `TITLE` values. The optional `YEAR` can be absent.

## The solution

1. Select only the practice audio files. If a filter is active, inspect the
   selection count; hidden selected files can still belong to an operation.
2. Choose **Convert → Tag → Filename…** (++alt+1++; Alt is ⌥ Option on macOS).
3. Enter this **Format string**, without adding an audio extension:

    ```text
    %artist%\%album%[ (%year%)]\$num(%track%,2) - %title%
    ```

4. Choose **Preview**. Check the full destination, including the root, every
   folder and the final filename. The following paths are relative to each
   input's containing folder; `/` shows folder levels for readability.

    Scroll the table sideways on a narrow screen to read the complete destinations.

    <div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Artist album folder examples">

    <!-- artist-folder-cases:start -->

    | Artist | Album | Year | Track | Title | Source name | Relative destination |
    |---|---|---|---|---|---|---|
    | `Juniper Vale` | `Harbour Lights` | `2024` | `3/8` | `Paper Boats` | `old.mp3` | `Juniper Vale/Harbour Lights (2024)/03 - Paper Boats.mp3` |
    | `Juniper Vale` | `Harbour Lights` | Absent | `3` | `Paper Boats` | `old.mp3` | `Juniper Vale/Harbour Lights/03 - Paper Boats.mp3` |
    | `Kofi/Mira` | `Harbour Lights` | `2024` | `3` | `Paper Boats` | `old.mp3` | `Kofi_Mira/Harbour Lights (2024)/03 - Paper Boats.mp3` |
    | `Juniper Vale` | `Harbour Lights` | `2024` | `3` | `Low Tide: Dawn` | `old.mp3` | `Juniper Vale/Harbour Lights (2024)/03 - Low Tide_ Dawn.mp3` |
    | `Juniper Vale` | `Harbour Lights` | `2024` | `100` | `Last Harbour` | `old.flac` | `Juniper Vale/Harbour Lights (2024)/100 - Last Harbour.flac` |

    <!-- artist-folder-cases:end -->

    </div>

5. Inspect the destination folders in the file manager if they already exist.
   Ensure your intended names are distinct and account for existing files.
   A preview is not a reservation or a guarantee about every filesystem alias.
6. When the small practice plan is correct, choose **Apply to … files**.
   The audio files move/rename immediately; no second **Save tags** step is
   needed for the paths. Inspect operation errors and the saved destinations.

## How it works

- `%artist%` supplies the artist folder. A literal backslash in the expression
  starts the next folder on all supported desktop platforms.
- `%album%[ (%year%)]` keeps the album, adding a space and parenthesized year
  only when the year field contributes a value. An absent year does not leave
  empty parentheses.
- `$num(%track%,2)` reads the leading track number and pads to at least two
  digits. It turns `3/8` into `03` and leaves `100` at three digits; it does
  not preserve a vinyl-side label such as `B2`.
- `%title%` supplies the final title text. The host retains the source audio
  extension. A FLAC input remains a FLAC file; this is not audio conversion.

A separator inside a tag value does not create the literal folder level:
with the default replacement, `Kofi/Mira` becomes `Kofi_Mira`. The setting is
**Options → Files → Replace folder separator in tag values with**. Other forbidden
value characters such as the example colon become underscores. These rules
do not certify that every generated component is suitable for every target
filesystem, network share or device. Preview and test the actual destination.

Required empty fields need attention before applying. A missing artist can
leave a leading separator and change the destination root: stop and correct
that field rather than applying the unexpected path. A missing album leaves
an empty component, a missing title can produce an incomplete name, and a
missing/nonnumeric track can become `00`. Do not treat a syntactically valid
format string as proof of correct metadata.

## Variations

For an artist folder based on album credit, use this expression only after
checking the album artist policy for the selection:

```text
$if2(%albumartist%,%artist%)\%album%[ (%year%)]\$num(%track%,2) - %title%
```

`$if2` uses the album artist when present, otherwise the first track artist.
With album artist `Various Artists` and the first practice row's other values,
the relative destination is <!-- artist-folder-albumartist-result -->`Various Artists/Harbour Lights (2024)/03 - Paper Boats.mp3`.
The [compilation recipe](rename-compilations.md) explains why per-track fallback
can split one album across several artist folders.

To use a shared archive root, prepend the intended absolute root and preview
it on that platform. A Windows drive path is not a portable absolute path on
macOS/Linux. Choose a real writable location; do not copy another machine's
root blindly. A complete date in `YEAR` remains a complete date here; use
[the year-only recipe](year-from-date.md) if a deliberate extraction is needed.

Reapplying this relative expression after moving files can nest the structure
again under their new folders. It does not promise to move sidecar pictures,
playlists or text files, or to remove old empty folders. **Keep moved files**
controls whether moved entries remain listed. Undo can meet newly occupied
paths, so verify the result and retain originals beyond the session.

## Related

- [Tag → Filename](../converters/tag-to-filename.md)
- [Rename files and folders lesson](../learn/rename-files-and-folders.md)
- [Multi-disc folder layouts](rename-multi-disc-albums.md)
- [Compilation layouts](rename-compilations.md)
- [Filename-mode rules](../reference/format-strings.md#file-and-folder-names)

<!-- verified: folder_recipe_guide.rs executes the written formulas/case tables under LF/CRLF with actual filename mode and isolated actual filename_target host helper. Source lib.rs/main.ts/rename_plan.rs defines timing and context. No real filesystem moves, native UI, all-platform alias/collision safety or player qualification. -->
