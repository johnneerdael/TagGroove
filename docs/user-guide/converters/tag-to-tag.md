# Tag → Tag

Use **Tag → Tag** to copy or calculate one destination field from existing
tags. Its preview refreshes as you change the field or format string,
making it useful for learning an expression before saving it in an action.

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
dropped; no remaining values removes the field. Without it, a nonempty
output is one value whose surrounding whitespace is kept.

A bare placeholder reads only the first existing value. For example,
`$meta_sep(artist,\\)` writes the existing artists as separate values, while
`$meta_sep(artist,' & ')` writes one joined value. Read the
[multivalue rules](../reference/format-strings.md#multi-value-fields) before
replacing artist or composer lists. Filename/directory targets use the naming
rules below instead of splitting metadata values.

An empty result removes the destination field. The preview labels that
outcome **Field will be removed**. `''` produces an apostrophe, not empty
text; see [quoting](../reference/format-strings.md).

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

- [Format strings](../reference/format-strings.md)
- [Regular expressions](../reference/regular-expressions.md)
- [Tag → Filename](tag-to-filename.md)
- [Converters](index.md)
