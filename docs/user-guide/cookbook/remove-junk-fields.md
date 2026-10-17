# Keep a reviewed field list on a cleanup copy

Use **Remove Fields Except** when the remaining field set is known and every
other exposed field should be removed. Build the list from the actual files;
TagGroove does not decide which identifiers, comments or custom fields are junk.
Use **Remove Fields** instead when only a few unwanted names are known.

## Inventory a copied file first

Open **Extended tags…** on a copied practice file and note the fields that
must survive. Include values needed by the intended player or library:
identifiers, sort names, original dates, lyrics, ReplayGain and totals can
all be useful metadata.

The example below keeps core album data and one chosen identifier, and
intentionally drops the shown comment and encoder annotation. It is a
practice policy for that known copy, not a universal master-library cleanup.

## Enter an explicit keep list

1. Save or discard pending edits and select one copied practice file.
2. Choose **Actions → Actions (Quick)…**, then **Remove Fields Except**.
3. In **Keep fields (separate with semicolons)**, enter the following list.
4. Review it against the full field inventory before choosing **Apply action**.
5. Inspect **Extended tags…**, embedded pictures and any operation errors.

<!-- keep-list-value:start -->
```text
TITLE;ARTIST;ALBUM;ALBUMARTIST;YEAR;DATE;TRACK;TRACKNUMBER;TRACKTOTAL;TOTALTRACKS;DISCNUMBER;TOTALDISCS;GENRE;MUSICBRAINZ_TRACKID;PICTURE
```
<!-- keep-list-value:end -->

These are literal names, not format strings. Do not add percent signs or
patterns such as `MUSICBRAINZ_*`. Each needed field must be named explicitly.
The action keeps a named field only when it already exists; it does not
create missing album artists or empty date fields.

The list includes several date/track spellings because native formats or
custom mappings can expose related data under different names. Inspect the
actual selection rather than assuming aliases are universally interchangeable.
For MP4, keep native total fields when the total is wanted: `TOTALTRACKS`
and `TOTALDISCS` share native pairs with their current-number fields.
A custom `TRACKTOTAL` field is a separate choice.

## Compare the practice inventory

Each row is one value in the copied file before the action. The kept names
retain their existing values; the other two fields disappear from the
editing model.

<!-- keep-list-cases:start -->
| Field before | Value before | Kept by the example list |
|---|---|---|
| `TITLE` | `Paper Boats` | Yes |
| `ARTIST` | `Juniper Vale` | Yes |
| `ALBUM` | `Harbour Lights` | Yes |
| `TRACK` | `3/8` | Yes |
| `TOTALTRACKS` | `8` | Yes |
| `DISCNUMBER` | `1/2` | Yes |
| `TOTALDISCS` | `2` | Yes |
| `YEAR` | `2024` | Yes |
| `MUSICBRAINZ_TRACKID` | `11111111-2222-4333-8444-555555555555` | Yes |
| `COMMENT` | `Practice-only note` | No |
| `ENCODEDBY` | `Old test encoder` | No |
<!-- keep-list-cases:end -->

If an unlisted lyrics, rating, original-date or ReplayGain field is needed,
add its actual name before running the action. A successful run is not proof
that the chosen policy was appropriate.

## Preserve artwork deliberately

`PICTURE` in this action is an artwork selector. `COVER` is an alternative.
It preserves embedded pictures; it does not refer to `folder.jpg` or rename
an image file. Without either selector, the action also removes embedded
artwork.

The following effects use the practice inventory above.

<!-- keep-artwork-cases:start -->
| Keep list | Metadata result | Artwork request |
|---|---|---|
| `TITLE;ARTIST;PICTURE` | Keep TITLE and ARTIST only | Keep embedded pictures |
| `TITLE;ARTIST` | Keep TITLE and ARTIST only | Remove embedded pictures |
| `COVER` | Remove all exposed tag fields | Keep embedded pictures |
| Leave empty | Remove all exposed tag fields | Remove embedded pictures |
<!-- keep-artwork-cases:end -->

An empty keep list is a destructive operation, not “leave everything alone.”
A folder-image fallback can appear after artwork removal; check the
**Not embedded in the file** label rather than interpreting the visible
picture as proof that embedded art survived.

## Check native storage and write timing

The action operates on exposed fields and artwork. It is not a general
container-data eraser: opaque or unsupported structures have format-specific
preservation/removal rules. Native writers also have their own field mappings
and tag-type policies. Read back the file and inspect the supported
[format limits](../reference/file-formats.md) after applying a cleanup copy.

The action writes when run; it does not wait for **Save tags**. A stored
group is only a workflow until applied. Start with one copy, inspect what
remained, then expand the selection only when the policy is confirmed.
An action warning does not stop later actions/groups, and a batch can partly
succeed. Read every reported file result before repeating it.

Use **Edit → Undo …** for a recorded change in the current desktop session.
Keep original files and wanted metadata outside that session's history.
A later metadata field cannot restore information that the cleanup discarded.

## Related

- [Remove Fields Except](../actions/remove-fields-except.md)
- [Remove Fields](../actions/remove-fields.md)
- [Field names and storage](../reference/field-names.md)
- [Working with covers](../guides/covers.md)

<!-- verified: crates/tag-core/src/actions.rs:RemoveFieldsExcept,is_cover; src/action-ui.ts:readAction; crates/tag-core/src/mapping.rs. Exact written list, eleven inventory rows and four artwork-effect cases execute against core under LF/CRLF. No native/container-erasure claim. -->
