# Split Field by Separator

Use **Split Field by Separator** to turn text containing several names into
separate field values. For example, one `ARTIST` value containing
`Juniper Vale; Mira Solberg` becomes two artist values.

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

Each existing value is split, with surrounding whitespace trimmed from
each piece. The pieces keep their original order. The separator itself is
not kept and is not interpreted as a regular expression.

| Before | Separator | Values after splitting |
|---|---|---|
| `Juniper Vale; Mira Solberg` | `;` | `Juniper Vale`, `Mira Solberg` |
| `Juniper Vale / Mira Solberg` | ` / ` | `Juniper Vale`, `Mira Solberg` |
| `Low Tide` | `;` | `Low Tide` |

If a field already has two values, both are processed and their resulting
pieces are collected in order. A separator at the start or end, or two
separators together, can produce empty pieces. Clean the input first when
those empty values are unwanted. The current action also trims surrounding whitespace when no separator is found. For a field with one padded value and no separator, the traced Mp3tag action leaves that value unchanged. With several existing values, its decision depends on the field’s total split-piece count. This compatibility difference remains open; check imported actions on copies before applying them to a collection.

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
- [Multivalue format strings](../reference/format-strings.md#multi-value-fields)
- [Actions and action groups](index.md)
