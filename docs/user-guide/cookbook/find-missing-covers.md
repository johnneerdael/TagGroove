# Find files without embedded covers

<div class="recipe-meta" markdown>
**Uses:** Filter and `%_covers%`.
**Works on:** Loaded audio entries with cover information in the installed app.
**Time:** About 5 minutes for a small copied album.
**Undo:** Filtering writes nothing; clearing it restores the view.
</div>

## The problem

The cover window shows artwork, but you need to know which tracks actually
carry an embedded image. A `folder.jpg` fallback can make a track look
illustrated even when its embedded-cover count is zero. Use copied files
before following the results with an import or removal operation.

This practice snapshot separates the count from the external image:

<!-- missing-covers-snapshots:start -->
| ID | `ALBUM` | `_COVERS` | Folder image available |
|---|---|---|---|
| A | `Harbour Lights` | `0` | Yes |
| B | `Harbour Lights` | `1` | No |
| C | `Harbour Lights` | `2` | Yes |
| D | `Harbour Sessions` | `0` | No |
<!-- missing-covers-snapshots:end -->

## The solution

1. Load the copied album and wait for its entries to finish loading. Resolve
   any file-read errors instead of treating a failed read as proof of no art.
2. Show the Filter using **View → Show filter** or ++f3++.
3. Enter:

```text
"%_covers%" EQUAL 0
```

4. Wait for automatic application, or press Enter if **Auto-apply filter**
   is disabled in the helper menu. A and D match the table above. A still
   matches when OpenTag can display its folder fallback.
5. Select one result and inspect the cover window's label. **Not embedded in
   the file** explains a folder-image preview; it is not an embedded cover.
   For mixed selections, inspect individual files or **Manage covers…**.
6. Clear the filter to restore the list; press Enter in manual mode.

The query does not embed the available folder image. To do that deliberately,
use [Embed a folder image](embed-folder-jpg.md). Before any batch cover change,
choose **Edit → Unselect all**, then select only the intended visible files.
Hidden selected rows remain a separate selection concern.

## How it works

`%_covers%` is a computed information placeholder. In this recipe it counts
embedded cover entries, not external images, distinct colors, pixels or front
cover roles. The filter evaluates the quoted selector, then `EQUAL` compares
its leading integer with zero.

Scroll the table sideways on a narrow screen to read each complete query.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Embedded-cover filter results">

<!-- missing-covers-queries:start -->
| Query | Matching IDs | Meaning |
|---|---|---|
| `"%_covers%" EQUAL 0` | `A,D` | No embedded cover entries. |
| `"%_covers%" GREATER 0` | `B,C` | At least one embedded cover entry. |
| `"%_covers%" GREATER 1` | `C` | More than one embedded entry. |
| `"%_covers%" EQUAL 0 AND ALBUM IS "Harbour Lights"` | `A` | The missing-cover subset of this album. |
<!-- missing-covers-queries:end -->

</div>

A count of two does not prove that one is Front Cover and the other Back
Cover, or that their image bytes differ. A nonzero count also does not prove
that every embedded image can be decoded or that a receiving player supports
its encoding. Conversely, lack of embedded art need not be a mistake: some
collections intentionally use only folder images.

Do not use `_COVERS MISSING` to ask for zero covers. Zero is a supplied count,
so a presence query asks a different question. Renaming a metadata field to
`_COVERS` is not a way to embed artwork; information fields are derived from
the loaded entry and its actual file measurements.

## Variations

- Use the positive-count query to review files before removing oversized or
  unwanted artwork. Inspection does not mean all covers should be removed.
- Add an album/artist constraint to work through one group at a time. Keep
  boolean grouping explicit when mixing `AND` and `OR`.
- If a folder image should travel with every track, embed a tested copy in
  supported tag formats. If it should stay external, copy that image along
  with the audio files instead.

Direct cover-window mutations and cover actions have their own write paths;
**Manage covers…** stages a plan until **OK**. A filter itself needs no tag
undo. After a subsequent recorded edit, inspect **Edit → Undo** and retain
original files/images for recovery beyond the session. Clearing a query does
not reverse an import, removal or external extraction.

## Related

- [Cover-art concepts](../concepts/cover-art.md)
- [Working with covers](../guides/covers.md)
- [Embed a folder image](embed-folder-jpg.md)
- [Filtering the file list](../guides/filter.md)

<!-- verified: filter_recipe_guide.rs executes actual four snapshots/four queries and the solution under LF/CRLF, plus real generated MP3 technical count before/after a PNG import while preserving external image/audio. Host evaluation.rs provides derived counts; no native fallback/player test. -->
