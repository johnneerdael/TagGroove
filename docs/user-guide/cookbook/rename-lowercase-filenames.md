# Lower-case, dash-separated filenames

<div class="recipe-meta" markdown>
**Uses:** Filename → Filename · **Works on:** copied local files with useful existing names ·
**Time:** about 5 minutes for a small batch · **Undo:** recorded renames in the current desktop session; keep originals
</div>

## The problem

You want a consistent lowercase **filename stem** with hyphens instead
of spaces or underscores. Keep the mixed-case title in the tag: a filename
policy does not need to become a metadata policy. This can be useful for
an established transfer or naming convention, but it does not certify
compatibility with a particular website, USB device or filesystem.

<!-- lowercase-name-cases:start -->
| Existing filename | Expected destination |
|---|---|
| `03 Paper Boats.mp3` | `03-paper-boats.mp3` |
| `04_Lanterns On The Water.flac` | `04-lanterns-on-the-water.flac` |
| `05 Café At Low Tide.MP3` | `05-café-at-low-tide.MP3` |
| `06 Paper  Boats.mp3` | `06-paper--boats.mp3` |
<!-- lowercase-name-cases:end -->

The converter retains the original audio extension, including its spelling.
The third example lowers the stem while keeping `.MP3`. Accents remain;
this is not an ASCII-only or accent-removal recipe. The last row deliberately
shows that replacing individual spaces can produce repeated hyphens.

## The solution

1. Keep the originals elsewhere and select a few copied local audio files.
   Save or discard pending tag edits before renaming. If a filter is active,
   inspect the intended selection, including any previously selected hidden rows.
2. Choose **Convert → Filename → Filename…**.
3. Set **Old filename pattern** to:

    <!-- lowercase-old-pattern:start -->
    ```text
    %1
    ```
    <!-- lowercase-old-pattern:end -->

4. Set **New filename pattern** to:

    <!-- lowercase-new-pattern:start -->
    ```text
    $lower($replace(%1, ,-,_,-))
    ```
    <!-- lowercase-new-pattern:end -->

5. Choose **Preview**. Compare every full destination with the table and
   your actual folder contents. The single space after the first comma
   is the search text; it must remain a real space in the expression.
6. Choose **Rename … files** only for the reviewed small plan. Inspect the
   operation result, the actual destination names and the unchanged tag fields.

Paths change when the rename runs. There is no subsequent **Save tags**
step for these names. Editing an input or changing the selection requires
another preview; an expired plan or changed file can also require it.
A preview is not a reservation of the destination.

## How it works

The old pattern `%1` captures the current basename without the audio
extension. It is a **numbered capture**, with no trailing percent sign:
`%1%` is not the same syntax. It does not read `TITLE` or `ARTIST`.

Read the new pattern from the inside out:

| Part | Meaning |
|---|---|
| `%1` | Reuse the captured basename |
| `$replace(%1, ,-,_,-)` | Replace each ordinary space with `-`, then each underscore with `-` |
| `$lower(...)` | Lowercase the resulting text |

Commas separate function arguments. The first pair has a space as its
search string and a hyphen as its replacement. The second pair does the
same for `_`. Existing hyphens, punctuation and non-ASCII letters are not
removed by those replacements. Tabs are not ordinary spaces either.

This route works from names even if the title tags are empty. It does not
lowercase tags or rename the parent album folder. To derive names from
reviewed tags instead, use the variation below and its different converter.

## Variations

### Collapse runs of spaces, underscores and hyphens

If repeated hyphens are unwanted, keep **Old filename pattern** `%1` and use:

<!-- lowercase-collapsed-pattern:start -->
```text
$lower($regexp(%1,'[ _-]+',-))
```
<!-- lowercase-collapsed-pattern:end -->

For `06 Paper  Boats.mp3`, this produces `06-paper-boats.mp3`.
`[ _-]+` matches one or more ordinary spaces, underscores or hyphens;
putting the hyphen last makes it literal in that character class. This
also collapses an intentionally doubled hyphen. It does not trim a
leading/trailing separator or normalize all Unicode whitespace.

### Derive a name from saved tags

When `TRACK` = `3/8` and `TITLE` = `Paper Boats`, use **Tag → Filename…**
instead, with:

<!-- lowercase-tag-pattern:start -->
```text
$num(%track%,2)-$lower($replace(%title%, ,-,_,-))
```
<!-- lowercase-tag-pattern:end -->

For an MP3 this gives `03-paper-boats.mp3`. This route reads the tags,
not the captured old basename. Check missing titles, nonnumeric tracks
and multi-value fields before applying it. `$num` reads a leading number;
it does not preserve a vinyl-side label such as `B2`.

### Keep the original spelling when case is significant

Remove the outer `$lower(...)` and use only the replacement expression.
That is a different naming policy, useful when deliberate capitals matter.
Do not rename a larger set merely because lowercase looked acceptable in
one example. A case-only spelling change can behave differently on different
volumes; check the actual operation result and the name your file manager shows.

## Before using the result elsewhere

Two different names can collapse to one destination after lowercasing or
separator replacement. Include an established distinguishing track/disc
part and compare the complete paths. Do not delete another file to make a
refused rename succeed. If the only intended change is capitalization,
try one copy first on the actual destination volume rather than treating
it as proof for every platform.

A lowercase name can still be too long or contain characters a receiving
system dislikes. Follow the [portable-name recipe](rename-safe-characters.md)
for a more explicit character and length policy. Test the resulting copies
on the receiving device; a successful rename is not a playback test.

Use **Edit → Undo** and its operation label for recorded renames in the
current desktop session. Keep original copies for recovery after quitting.

## Related

- [Filename → Filename reference](../converters/filename-to-filename.md)
- [Tag → Filename reference](../converters/tag-to-filename.md)
- [Text functions](../reference/scripting/text.md)
- [Regular expressions](../reference/regular-expressions.md)
