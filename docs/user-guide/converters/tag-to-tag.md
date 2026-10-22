# Tag → Tag

Use **Tag → Tag** to copy or calculate one destination field from existing
tags. Its preview refreshes as you change the field or format string, so you
can learn an expression before saving it in an action.

<!-- figure:tag-to-tag -->
<figure markdown>
![The Tag → Tag dialog with a destination field, the format string $upper(%title%) and a live preview of the result](../assets/screens/tag-to-tag-light.png#only-light)
![The Tag → Tag dialog with a destination field, the format string $upper(%title%) and a live preview of the result](../assets/screens/tag-to-tag-dark.png#only-dark)
<figcaption>The preview refreshes as you type; nothing is written until you apply it.</figcaption>
</figure>
<!-- /figure -->

## Fill missing album artists

1. Keep copies, save pending edits and select practice files.
2. Choose **Convert → Tag → Tag…** (++alt+5++; Alt is ⌥ Option on a Mac).
3. Enter `ALBUMARTIST` in **Field**.
4. Enter this **Format string**:

    ```text
    $if2(%albumartist%,%artist%)
    ```

5. Wait for the preview to finish. An existing album artist should remain;
   a missing one should receive the artist value.
6. Choose **Apply to … files** to write the reviewed values immediately.

`$if2` chooses the first nonempty argument. With `ARTIST` = `Juniper Vale`,
the result is `Juniper Vale` only when `ALBUMARTIST` is empty. Entering
`%artist%` alone would replace existing album artists as well.

## Expressions to try

| Destination field | Format string | Input | Result |
|---|---|---|---|
| `ALBUMARTIST` | `%artist%` | `ARTIST` = `Juniper Vale` | `Juniper Vale` |
| `YEAR` | `$left(%year%,4)` | `YEAR` = `2024-05-01` | `2024` |
| `TITLE` | `$trim(%title%)` | `TITLE` = `  Paper Boats  ` | `Paper Boats` |
| `COMMENT` | `Library copy` | Any existing comment | `Library copy` |
| `TRACK` | `$num(%track%,2)` | `TRACK` = `3/8` | `03` |

For ordinary metadata fields, two consecutive backslashes (`\\`) in the
evaluated output separate values, as in [Format Value](../actions/format-value.md#write-one-or-several-field-values).
With that delimiter, each component is trimmed and empty components are
dropped; if no values remain, the field is removed. The preview shows the
surviving values joined with ` \\ `, or **Field will be removed** if none
remain. Without the delimiter, a nonempty output is one value whose
surrounding whitespace is kept.

A bare placeholder reads only the first existing value. For example,
`$meta_sep(artist,\\)` writes the existing artists as separate values, while
`$meta_sep(artist,' & ')` writes one joined value. Read the
[multivalue rules](../scripting/format-strings.md#multi-value-fields) before
replacing artist or composer lists. Filename/directory targets use the naming
rules below instead of splitting metadata values.

An empty result removes the destination field. The preview labels that
outcome **Field will be removed**. `''` produces an apostrophe, not empty
text; see [quoting](../scripting/format-strings.md).

## Rename from the same dialog

The two special destinations are `_FILENAME` and `_DIRECTORY`:

| Field | Format string | Effect |
|---|---|---|
| `_FILENAME` | `$num(%track%,2) - %title%` | Build a filename and retain the audio extension |
| `_DIRECTORY` | `%album%` | Rename the containing folder from the album |

These destinations use filename evaluation: literal separators can create
folders, while separators from field values use the configured replacement.
An empty generated name disables applying the conversion.

A folder rename affects the folder itself, potentially including files
outside the selection. Practice with a copied folder, and use
[Tag → Filename](tag-to-filename.md) when reviewing a full file-and-folder
layout is the goal.

## Gotchas

- Applying saves immediately, unlike editing the tag panel.
- Changing the expression or field invalidates the previous result until
  the automatic preview finishes. Review the final version before applying.
- At most the first 100 preview rows are displayed, even when more files
  are selected. Split a large unfamiliar collection into smaller batches.
- `_TAG`, `_ALL` and technical information placeholders are not destination
  fields. Choose an ordinary tag field or one of the two naming targets.
- Keep copies and inspect operation errors and **Edit → Undo** for recorded
  changes in the current desktop session.

<!-- verified: src/tag-tag-ui.ts:openTagToTag, tagTagOutcome; crates/tag-core/src/actions.rs:FormatValue -->

## Related

- [Format strings](../scripting/format-strings.md)
- [Regular expressions](../scripting/regular-expressions.md)
- [Tag → Filename](tag-to-filename.md)
- [Converters](index.md)

## Filename changes with companion files

When the associated-files option is enabled, using `_FILENAME` here carries
matching `.lrc`/`.cdg` files through the same guarded filename effect as actions.
Tag → Tag runs immediately; it does not show a companion destination preview.
Use a reviewed filename converter to inspect exact paths before applying.
See [associated files](../tagging/associated-files.md#actions-and-tag-tag) for
membership checks, collision/rollback errors and recorded history. Inline
filename editing remains audio-only.
