# Sort a leading “The” without changing the artist credit

<div class="recipe-meta" markdown>

**Uses** Tag → Tag, `$regexp`, `ARTISTSORT`

**Works on** Copied local files with a reviewed primary Artist

**Time** About 5 minutes

**Undo** Recorded converter changes in the current desktop session

</div>

## The problem

Keep the displayed credit `The Lanterns`, but offer `Lanterns, The` as its
sort name. Write `ARTISTSORT`; do not rewrite `ARTIST` merely to alter an
alphabetical order. Whether a player uses that field remains player-specific.

This recipe derives one sort name from the first Artist value. It leaves
the full Artist value list alone. It does not invent personal surname order
or sort each member of a multi-artist collaboration separately.

The following are primary Artist input/output examples. `↵` denotes a newline;
`·` makes a space visible. The result is the proposed `ARTISTSORT`, with the
Artist credit unchanged.

<div class="native-mapping-table" markdown>

<!-- artist-sort-cases:start -->

| First `ARTIST` value | Proposed `ARTISTSORT` |
| --- | --- |
| `The Lanterns` | `Lanterns, The` |
| `the Lanterns` | `Lanterns, the` |
| `Juniper Vale` | `Juniper Vale` |
| `The` | `The` |
| `The  Lanterns` | `·Lanterns, The` |
| `The Lanterns↵Mira Solberg` | `The Lanterns↵Mira Solberg` |
| (empty text) | (empty text) |

<!-- artist-sort-cases:end -->

</div>

The extra-space and empty examples are reasons to stop and correct the
source, not desirable sort names. The rule preserves text it does not
recognize. It matches one literal space after a leading “The”; extra spaces
belong to the captured name and survive.

## The solution

1. Work on copied files. Save or discard pending changes and inspect the
   existing Artist and Artist Sort values. Keep manually curated sort names.
2. Choose **Edit → Unselect all**, show the filter, and enter:

    <!-- artist-sort-filter:start -->
    ```text
    ARTISTSORT MISSING AND ARTIST PRESENT
    ```
    <!-- artist-sort-filter:end -->

    Wait for auto-apply or press Enter in manual mode. A present empty Artist
    still passes `PRESENT`; inspect the preview. A present empty Artist Sort
    is not `MISSING` and is deliberately outside this selection.
3. Select only the intended visible rows, then open **Convert → Tag → Tag…**.
   Enter `ARTISTSORT` in **Field** and use:

    <!-- artist-sort-format:start -->
    ```text
    $regexp(%artist%,'(?-m)^(The) ([^\r\n]+)$','$2, $1')
    ```
    <!-- artist-sort-format:end -->

4. Inspect the before/after preview. Exclude malformed credits, empty results
   and collaborations for which the first artist is the wrong sorting policy.
5. Apply to the practice selection. The converter writes immediately;
   no extra **Save tags** is required for its successful results. Reload and
   check that `ARTIST` is still the original credit and `ARTISTSORT` has the
   intended value. Inspect any per-file errors before a larger run.

If preferred, save the same field/expression as a **Format Value** action.
Its apply command also writes immediately; preserve the filter/selection
precondition rather than running it over existing curated sort fields.

## How it works

`%artist%` reads the first value. `$regexp` is case-insensitive by default,
so both `The` and `the` match. Capture 1 remembers the original spelling of
the article, and capture 2 remembers the remaining name. The quoted
replacement `$2, $1` places a comma and space between them. Quoting prevents
the comma from becoming another scripting-function argument.

`(?-m)` keeps the anchors tied to the whole value. `[^\r\n]+` requires a
nonempty name without line breaks; a multi-line credit is left untouched.
A nonmatch returns the original text, which supplies the unchanged-credit
fallback for names such as Juniper Vale.

The filter excludes existing Artist Sort value lists, including present
empty strings. If a file has two Artist values, the new sort name still uses
only the first. Its Artist values remain separate and unchanged. For example:

<div class="native-mapping-table" markdown>

<!-- artist-sort-list-cases:start -->

| `ARTIST` values | Existing `ARTISTSORT` | Pass main filter? | Resulting `ARTISTSORT` |
| --- | --- | --- | --- |
| `["The Lanterns","Mira Solberg"]` | Absent | Yes | `["Lanterns, The"]` |
| `["The Lanterns"]` | `["Lanterns"]` | No | `["Lanterns"]` |
| `["The Lanterns"]` | `[""]` | No | `[""]` |
| `[""]` | Absent | Yes | Absent |

<!-- artist-sort-list-cases:end -->

</div>

## Variations

### Only match exactly capitalized “The”

Add a fourth `$regexp` argument of `0` to respect case:

<!-- artist-sort-sensitive:start -->
```text
$regexp(%artist%,'(?-m)^(The) ([^\r\n]+)$','$2, $1',0)
```
<!-- artist-sort-sensitive:end -->

<div class="native-mapping-table" markdown>

<!-- artist-sort-sensitive-cases:start -->

| Input | Result with `0` |
| --- | --- |
| `The Lanterns` | `Lanterns, The` |
| `the Lanterns` | `the Lanterns` |
| `Juniper Vale` | `Juniper Vale` |

<!-- artist-sort-sensitive-cases:end -->

</div>

This is the shared scripting-core flag, not Find's checkbox or a web-source
parser default.

### Album artist sorting

Use the separate `ALBUMARTISTSORT` field and an expression reading
`%albumartist%` when the album-level credit needs sorting. Review its existing
values and select its missing-field targets separately. Do not substitute a
track artist for a compilation's album credit merely to fill a blank.

### Make OpenTag's list use a sort field

Writing `ARTISTSORT` does not automatically change every column's sorting
behavior. Configure an appropriate column sort expression and click its
header as described in [Columns](../customize/columns.md). A player's own
sorting preference is separate. [Sort fields and compilations](../concepts/sort-fields.md)
explains native mappings, fallbacks and format-specific behavior.

### Recover the old value

Read the recorded **Undo …** step and file count in the current desktop
session. Inspect pending edits afterward so a later save does not replace a
restored value. Session history is bounded and is not a restart-proof backup.
Keep original copies and review a player's actual behavior before a library run.

## Related

- [Sort fields and compilations](../concepts/sort-fields.md)
- [Tag → Tag](../converters/tag-to-tag.md)
- [Regular expressions](../reference/regular-expressions.md)
- [Fill missing album artists](album-artist-from-artist.md)
