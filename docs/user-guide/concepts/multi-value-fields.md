# Multi-value fields

Why does a player show `Juniper Vale; Mira Solberg` as one artist when you
expected two? A field can contain one text value with punctuation, or a
list of separate values. Those are different structures even when a screen
renders them similarly.

## One field can hold several values

Compare three possible `ARTIST` fields:

| Stored structure | What it says |
|---|---|
| One value: `Juniper Vale` | One credited artist |
| One value: `Juniper Vale; Mira Solberg` | One string containing a semicolon |
| Two values: `Juniper Vale` and `Mira Solberg` | Two separate artist values |

A semicolon is ordinary text until an operation explicitly splits on it.
The same is true of ` & `, `/` and commas. Splitting every slash would
damage names such as `AC/DC`; the punctuation alone cannot reveal whether
it belongs inside one name or separates two names.

## Display separators are not stored punctuation

OpenTag can display multiple values as:

```text
Juniper Vale \\ Mira Solberg
```

The two backslashes separate values in that editing/display convention.
They do not mean that the raw tag stores those exact characters between
the names. In the tag panel, entering `Juniper Vale \\ Mira Solberg`
creates two values and trims surrounding whitespace from each piece.

**Extended tags…** (++alt+t++; Alt is ⌥ Option on a Mac) makes the structure
explicit with **Value 1**, **Value 2** and **＋ Add value**. Put one whole
name in each value box. A semicolon typed inside one box remains part of
that value.

Selecting several files introduces another kind of difference: the files
may have different artists. **Mixed values — keep existing values** means
the selection differs; it does not mean every file contains the union of
all those artists. Leaving a mixed field unchanged preserves each file's
existing values.

## Format strings choose which values to read

For two stored artist values, `Juniper Vale` followed by `Mira Solberg`:

| Format string | Result |
|---|---|
| `%artist%` | `Juniper Vale` |
| `$meta(artist)` | `Juniper Vale, Mira Solberg` |
| `$meta(artist,0)` | `Juniper Vale` |
| `$meta(artist,1)` | `Mira Solberg` |
| `$meta_sep(artist,' & ')` | `Juniper Vale & Mira Solberg` |

A bare placeholder reads the first value. `$meta` without an index reads
all values using comma-space; indexed `$meta` starts at 0. `$meta_sep`
lets the expression choose how to join them. These functions calculate
text; joining text does not preserve the original list structure if that
result is written back as one field value.

## Storage depends on the tag format

OpenTag's editing model is a list of values per field. The writer must
represent that list in the destination format:

| Tag storage | Representation relevant to multiple values |
|---|---|
| Vorbis Comments in FLAC/Ogg-family files | Repeated entries with the same field name |
| APEv2 | Text values separated inside the item by NUL characters |
| ID3v2 | Text-frame value lists and format-specific frames; some fields have a single-value restriction |
| MP4 metadata | Multiple data values; the advanced setting can keep them inside one item atom or write separate item atoms |
| RIFF INFO | One string per INFO identifier; OpenTag joins multiple values with semicolon-space |
| ID3v1 | Limited legacy fields; it cannot represent a general list of values |

The two MP4 layouts can contain the same logical values while being
interpreted differently by a player. ID3 revisions and special-purpose
fields also have their own constraints. A field that supports several
artists does not imply that a numeric field or identifier accepts several
values. Read the saved file back and verify the intended player before
applying a storage convention to a whole library.

## Splitting, joining and deduplicating are different jobs

| Operation | Input | Result |
|---|---|---|
| Split on `;` | One value: `Juniper Vale; Mira Solberg` | Two values: `Juniper Vale`, `Mira Solberg` |
| Merge with `; ` | Two values: `Juniper Vale`, `Mira Solberg` | One value: `Juniper Vale; Mira Solberg` |
| Remove equal duplicates | Three values: `Juniper Vale`, `Juniper Vale`, `Mira Solberg` | Two distinct values |

The **Split Field by Separator** action trims the resulting pieces.
**Merge Duplicate Fields** joins every value, even when the values are
different. **Remove Duplicate Fields** is more destructive with its
same-content option off: it keeps only the first value, including when
the discarded values were different people. Check that option carefully.

## What this means in OpenTag

Decide whether the destination player needs separate artists or one
display credit. Preserve separate values when they carry useful structure;
join them only for a specific display or compatibility goal. Work on copies
until the saved tags and player agree.

Editing in **Extended tags…** stages pending changes; **Save tags** writes
them. An action writes when run. A successful preview or a joined display
is not proof that the final file still contains the intended value list.

<!-- verified: src/tag-panel.ts:splitValues; src/main.ts:field; src/extended-tags.ts:addRow, commit; crates/tag-core/src/tags.rs:Fields, write_native; crates/tag-core/src/mapping.rs:patch_id3, patch_mp4 -->

## Related

- [Format strings and multiple values](../reference/format-strings.md#multi-value-fields)
- [Text encodings and codepages](text-encodings.md)
- [Your first tagging session](../learn/first-tagging-session.md)
