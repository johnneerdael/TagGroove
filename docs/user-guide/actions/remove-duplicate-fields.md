# Remove Duplicate Fields

Use **Remove Duplicate Fields** to reduce a field's multiple values.
Choose carefully: the default keeps only the first value, even when the
later values are different artists or names that should be retained.

## Choose what to keep

1. Save pending edits and select a few copied files.
2. Choose **Actions → Actions (Quick)…**, then **Remove Duplicate Fields**.
3. Enter a field such as `ARTIST`, or `_ALL` for every tag field.
4. Set **Only duplicated fields with the same content** as needed.
5. Choose **Apply action** and inspect the values in **Extended tags…**.

| Option | What survives | Initial setting |
|---|---|---|
| Same-content option off | The first value of each affected field | Off |
| Same-content option on | The first occurrence of each exactly equal value | |

## Worked examples

| Values before | Same-content option | Values after |
|---|---|---|
| `Juniper Vale`, `Mira Solberg` | Off | `Juniper Vale` |
| `Juniper Vale`, `Mira Solberg` | On | Both artists |
| `Juniper Vale`, `Juniper Vale`, `Mira Solberg` | On | `Juniper Vale`, `Mira Solberg` |
| `Juniper Vale`, `JUNIPER VALE` | On | Both values: comparison is case-sensitive |

Remaining values keep their original order. A missing field is not
created. A field with one value stays as it is.

## Special targets

`_ALL` means all tag fields for this action. It does not rename a file or
remove artwork. Filename, directory and field-name selectors are not
supported. Use a specific field to avoid reducing unrelated multivalue
fields such as composers or genres.

## Gotchas

- Equal-looking strings may differ in case or whitespace. The same-content
  option compares the stored text exactly.
- Turning the option off is a deliberate loss of additional values. Keep a
  backup until the result is verified in the player that will use it.
- Actions save immediately. Check operation errors and the **Edit → Undo**
  entry for the recorded change.

## Related

- [Merge Duplicate Fields](merge-duplicate-fields.md)
- [Split Field by Separator](split-field.md)
- [Actions and action groups](index.md)
