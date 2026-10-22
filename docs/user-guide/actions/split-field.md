# Split Field by Separator

Use **Split Field by Separator** to turn text containing several names into
separate field values. For example, one `ARTIST` value containing
`Juniper Vale; Mira Solberg` becomes two artist values.

<!-- figure:action-split-field -->
<figure markdown>
![The Edit action dialog with the action type set to Split Field by Separator and its settings](../assets/screens/action-split-field-light.png#only-light)
![The Edit action dialog with the action type set to Split Field by Separator and its settings](../assets/screens/action-split-field-dark.png#only-dark)
<figcaption>The Split Field by Separator action's settings in an action group.</figcaption>
</figure>
<!-- /figure -->

## Split a field

1. Save pending edits and select a few copied files.
2. Choose **Actions → Actions (Quick)…**, then **Split Field by Separator**.
3. Set **Field** to `ARTIST` and **Separator** to `;`.
4. Choose **Apply action**, then open **Extended tags…** (++alt+t++) to
   inspect the separate values.

For repeated use, keep these settings in an action group.

| Setting | Meaning |
|---|---|
| **Field** | A tag field, or `_TAG` for every tag field |
| **Separator** | A nonempty literal string; it can contain several characters |

## Understand the result

Each existing value is split on the literal separator. The resulting values
keep their original order, including repeated names. The separator is not a
regular expression: `.*` means those two characters, not a wildcard.

| Before | Separator | Values after splitting |
|---|---|---|
| `Juniper Vale; Mira Solberg` | `;` | `Juniper Vale`, `Mira Solberg` |
| `Juniper Vale / Mira Solberg` | ` / ` | `Juniper Vale`, `Mira Solberg` |
| `Low Tide` | `;` | `Low Tide` |

If a field already has several values, their raw split pieces are counted
together. When that count differs from the number of original values,
TagGroove trims each piece, removes empty pieces and stores the remaining
values. If none remain, it removes the field. Leading, trailing and repeated
separators therefore do not create empty stored values when this branch runs.

When the raw piece count equals the original value count, the field stays
exactly as it was. For example, splitting one padded value ` Low Tide ` on `;`
keeps its spaces because no split occurred. This count rule applies to the
whole field: existing empty values can offset newly split pieces, so inspect
unusual multivalue input in Extended tags before applying a batch action.

TagGroove's display may join values with ` \\ `. That is a display convention;
do not use it as a separator unless those characters actually occur inside
one value. Whether another player displays multiple values separately
depends on the tag format and the player.

## Gotchas

- Splitting `AC/DC` on `/` would divide a single artist name. Select only
  the files that follow the intended convention.
- `_TAG` applies the same separator to every tag field, including comments
  or lyrics. Use a named field whenever that is the actual goal.
- `_FILENAME`, `_DIRECTORY` and `_FIELDNAME` are not split targets.
- The action writes immediately. Use practice copies and check the recorded
  **Edit → Undo** operation before relying on it.

## Related

- [Merge Duplicate Fields](merge-duplicate-fields.md)
- [Remove Duplicate Fields](remove-duplicate-fields.md)
- [Multivalue format strings](../scripting/format-strings.md#multi-value-fields)
- [Actions and action groups](index.md)
