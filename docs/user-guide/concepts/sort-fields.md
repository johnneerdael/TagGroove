# Sort fields and compilations

Why does a player file **The Lanterns** under T, and why does a guest
appearance split an album into several artists? Display credits, ordering
hints and album grouping are separate metadata choices. Preserve the credit
people should read, then add the information the receiving player needs.

## A sort name is an extra value

`ARTIST` is the displayed track credit. `ARTISTSORT` offers an alternate
ordering name without replacing that credit. Similarly, `ALBUMARTIST` can
identify the album-level credit while `ALBUMARTISTSORT` supplies its sorting
form. A player may support these fields, ignore them, or apply its own
article rules. Test the actual player before retagging a library.

This invented practice record keeps the visible artist intact:

<!-- sort-concept-fields:start -->
| Field | Stored value |
|---|---|
| `ARTIST` | `The Lanterns` |
| `ARTISTSORT` | `Lanterns, The` |
| `ALBUMARTIST` | `The Lanterns` |
| `ALBUMARTISTSORT` | `Lanterns, The` |
| `ALBUM` | `Harbour Lights` |
| `ALBUMSORT` | `2024 - Harbour Lights` |
| `TITLE` | `Paper Boats` |
<!-- sort-concept-fields:end -->

The alternate album text is an ordering convention chosen for this example;
it does not change `ALBUM`, the release year, or the audio. There is no
universal rule that every name beginning with “The” should be rewritten.
Names in other languages and intentional artist spellings need a deliberate
policy. Keep accents and punctuation unless the target system requires a
different sorting form.

The remaining common counterparts are `TITLESORT` for `TITLE` and
`COMPOSERSORT` for `COMPOSER`. These are separate fields, not switches that
transform their associated display values. See the [native mapping table](../reference/tag-mapping.md)
for their storage in each tag family; an ID3 or MP4 name is not a promise
that another format or player has the same native facility.

## File-list sorting is a separate decision

TagGroove's default Artist column displays the artist values and sorts that
column's evaluated display value. It does not automatically substitute
`ARTISTSORT`. A custom column can display the original credit while using a
different **SORT BY (EMPTY = VALUE)** expression.

For the practice record above:

<!-- sort-concept-expressions:start -->
| Purpose | Format string | Result |
|---|---|---|
| Display credit | `%artist%` | `The Lanterns` |
| Use the alternate key when present | `$if2(%artistsort%,%artist%)` | `Lanterns, The` |
| Album ordering key | `$if2(%albumsort%,%album%)` | `2024 - Harbour Lights` |
<!-- sort-concept-expressions:end -->

`$if2` uses the second expression when the first has no value. Removing
`ARTISTSORT` therefore makes the second example fall back to `The Lanterns`.
Evaluating these expressions, or clicking a column heading, does not write a
sort field into the audio file. Configure columns through the [column guide](../customize/columns.md).
For a list of several artists, choose an explicit join if the ordering key
should include all of them; a plain placeholder reads the first value.

## Album artist and compilation answer different questions

Imagine a collection called *Harbour Sessions*: one track credits
`Juniper Vale`, another `Mira Solberg`. Their `ARTIST` values should stay
different. Giving both `ALBUMARTIST` = `Various Artists` can supply a common
album credit where a player uses album artist for grouping. Keep the album
name and disc/track numbers consistent too. Grouping behavior still belongs
to the player; the text `Various Artists` is not a special TagGroove command.

`COMPILATION` is a separate flag. It neither fills `ALBUMARTIST` nor changes
track credits. In TagGroove's constrained ID3v2 and MP4 mappings, enable it
with `1`, or remove the field to disable it:

<!-- compilation-concept-cases:start -->
| Requested field value | Result in the ID3v2 practice file |
|---|---|
| `1` | `1` |
| `0` | Error; existing `1` remains |
| `true` | Error; existing `1` remains |
| Remove field | Absent |
<!-- compilation-concept-cases:end -->

An absent flag and a text value of `0` are different write requests. Native
APEv2/Vorbis text fields do not acquire these ID3/MP4 constraints merely
because they have the same name. Inspect the [structured-field rules](../reference/field-names.md#structured-values)
and saved result for the actual file format.

## What this means in TagGroove

Use [Extended tags](../guides/extended-tags.md) to inspect or add fields that
are not in the tag panel. **Keep changes** stages its text edits; finish with
**Save tags**. A Tag → Tag converter or action that writes these values has
its own immediate write path. Try a copied album first, read the saved values
back, and refresh the consuming player's library if necessary. Session
**Edit → Undo** is useful for recorded operations; retain originals for
recovery beyond that session.

## Related

- [What tags are](what-are-tags.md)
- [Configure file-list columns](../customize/columns.md)
- [Extended tags](../guides/extended-tags.md)
- [Field names and structured values](../reference/field-names.md)

<!-- verified: media_concept_guide.rs executes written fields, expressions and ID3 flag success/error/deletion cases under LF/CRLF; guide-media-concepts.test.mjs checks actual column sort-template selection. Source mapping.rs/columns.ts; no player-grouping test. -->
