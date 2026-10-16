# Remove Fields

Use **Remove Fields** to delete selected metadata fields while keeping the
rest of the tag. It can also remove embedded artwork.

## Remove named fields

1. Save pending edits and select a few copied files.
2. Choose **Actions → Actions (Quick)…**, then **Remove Fields**.
3. In **Remove fields (separate with semicolons)**, enter:

    ```text
    COMMENT;ENCODEDBY
    ```

4. Choose **Apply action** and check **Extended tags…**.

`COMMENT` and `ENCODEDBY` disappear when present; other fields stay in place.
A name not present in a file has nothing to remove.

## More examples

| Enter | Effect |
|---|---|
| `COMMENT` | Remove the comment field |
| `COMMENT;RATING` | Remove both named fields |
| `COVER` | Remove every embedded picture |
| `COMMENT;PICTURE` | Remove comments and every embedded picture |

`COVER` and `PICTURE` are artwork selectors here. They do not refer to an
image file alongside the music. This action does not delete `folder.jpg`.

The list accepts field names separated by semicolons. It is not a filter,
regular expression or format string, and it does not support wildcard
field-name patterns. Do not enter `%comment%` or an information placeholder
such as `_BITRATE` to remove a tag field.

## Gotchas

- Field removal saves immediately. It does not blank the panel and wait for
  **Save tags**.
- Removing a field differs from removing the entire tag format. The action
  only targets the names listed, plus artwork when explicitly requested.
- Keep backups. For recorded file edits in the current desktop session,
  inspect **Edit → Undo** after applying.

## Related

- [Remove Fields Except](remove-fields-except.md)
- [Remove Duplicate Fields](remove-duplicate-fields.md)
- [Actions and action groups](index.md)
