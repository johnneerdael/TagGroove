# Merge Duplicate Fields

Use **Merge Duplicate Fields** when a field contains several separate
values and the target player expects one combined value. It joins the
existing values with a chosen separator. It does not remove repeated text.

## Join a field's values

1. Save pending edits and select a few copied files.
2. Open **Actions → Actions (Quick)…** and choose **Merge Duplicate Fields**.
3. Set **Field** to `ARTIST` and **Separator** to `; `, including the space.
4. Choose **Apply action** and inspect the field in **Extended tags…**.

| Setting | Meaning |
|---|---|
| **Field** | One tag field, or `_TAG` for every tag field |
| **Separator** | Literal text inserted between values; leave empty to concatenate them |

## Worked examples

| Separate values before | Separator | One value after |
|---|---|---|
| `Juniper Vale`, `Mira Solberg` | Semicolon and space | `Juniper Vale; Mira Solberg` |
| `Juniper Vale`, `Mira Solberg` | ` & ` | `Juniper Vale & Mira Solberg` |
| `Low`, `Tide` | One space | `Low Tide` |
| `Juniper Vale`, `Juniper Vale` | `; ` | `Juniper Vale; Juniper Vale` |

In the last example, *duplicate fields* means multiple values of one
field, which need not be equal. To remove repeated values, use
**Remove Duplicate Fields** with its same-content option instead.

A field with zero or one value is unchanged. Existing order is preserved.
An empty separator joins directly, so `Low` and `Tide` become `LowTide`.
The separator is text, not a format string: `%artist%` would be inserted
literally between the values.

## Gotchas

- `_TAG` can collapse every multivalue field, including ones whose separate
  values a player needs. Prefer a named field.
- `_FILENAME`, `_DIRECTORY` and `_FIELDNAME` are not supported targets.
- Merging changes the stored structure. A later split can only recover the
  original values if the separator never appeared inside any original value.
- The action writes when run. Practice on copies and check **Edit → Undo**
  for the recorded file change.

## Related

- [Split Field by Separator](split-field.md)
- [Remove Duplicate Fields](remove-duplicate-fields.md)
- [Actions and action groups](index.md)
