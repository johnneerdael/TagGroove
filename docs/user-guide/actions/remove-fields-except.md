# Remove Fields Except

Use **Remove Fields Except** to keep an explicit list of fields and remove
all other fields. This is useful for a controlled export copy of your music;
it can also remove useful identifiers, sort fields and artwork if they are
missing from the keep list.

## Keep a chosen set of fields

1. Save pending edits and select copied files.
2. Choose **Actions → Actions (Quick)…**, then **Remove Fields Except**.
3. Enter the field names in **Keep fields (separate with semicolons)**.
4. Choose **Apply action** and inspect the full tag with **Extended tags…**.

For example:

```text
TITLE;ARTIST;ALBUM;ALBUMARTIST;YEAR;DATE;TRACK;TRACKNUMBER;DISCNUMBER;GENRE;COVER
```

This preserves the named fields if present. It does not add missing ones.
`COVER` preserves embedded artwork; `PICTURE` can be used for the same
purpose. Without either artwork selector, embedded pictures are removed.

## Build the list from actual files

Different formats can expose similar values under different names. For
example, a file may use `DATE` rather than `YEAR`, or `TRACKNUMBER` rather
than `TRACK`. Open **Extended tags…** before preparing the keep list, and
include the names used in the files being processed.

| Keep list | Effect |
|---|---|
| `TITLE;ARTIST;COVER` | Keep only title, artist and embedded artwork |
| `TITLE;ARTIST` | Keep only title and artist; remove embedded artwork |
| `COVER` | Keep embedded pictures while removing all tag fields |

The list is literal field names separated by semicolons. It does not
accept placeholders or patterns such as `MUSICBRAINZ_*`. Add each field
that must survive explicitly.

## Gotchas

- An empty keep list removes every field and embedded cover. Review it
  before applying; keep an untouched copy of the files.
- Identifiers, ReplayGain values, lyrics, sort names and totals are metadata
  too. Include them if another application depends on them.
- The action writes immediately. Check operation errors and the session's
  **Edit → Undo** entry; do not rely on an unspecified whole-batch rollback.

## Related

- [Remove Fields](remove-fields.md)
- [Remove Duplicate Fields](remove-duplicate-fields.md)
- [Actions and action groups](index.md)
