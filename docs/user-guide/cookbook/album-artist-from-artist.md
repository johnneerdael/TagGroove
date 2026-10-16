# Fill missing album artists from a reviewed track credit

<div class="recipe-meta" markdown>

**Uses** Filter and Tag → Tag / Format Value

**Works on** Copied ordinary local files with a reviewed primary artist

**Time** About 5 minutes

**Undo** Recorded applied changes in the current desktop session; keep originals

</div>

## The problem

Your album has `ARTIST`, but no `ALBUMARTIST`. A player may group that album
using Album Artist. Copying each track's artist is suitable only when that is
the intended album-level credit. A compilation with a different artist per
track usually needs one deliberately chosen common album artist instead.

This recipe uses the **first** Artist value as that primary credit. It does
not decide who the album artist ought to be. Existing Album Artist values are
excluded from the main selection.

The tables use JSON lists to show distinct values, including empty strings.
`Absent` means no field. They describe core metadata snapshots; native formats
can normalize empty values differently when saved.

<div class="native-mapping-table" markdown>

<!-- album-credit-cases:start -->

| ID | `ARTIST` before | `ALBUMARTIST` before | Pass main filter? | `ALBUMARTIST` after applying to matching rows |
| --- | --- | --- | --- | --- |
| A | `["Juniper Vale"]` | Absent | Yes | `["Juniper Vale"]` |
| B | `["Juniper Vale"]` | `["Harbour Collective"]` | No | `["Harbour Collective"]` |
| C | `["Juniper Vale","Mira Solberg"]` | Absent | Yes | `["Juniper Vale"]` |
| D | Absent | Absent | No | Absent |
| E | `[""]` | Absent | Yes | Absent |
| F | `["Juniper Vale"]` | `[""]` | No | `[""]` |
| G | `["   "]` | Absent | Yes | `["   "]` |
| H | `["","Mira Solberg"]` | Absent | Yes | Absent |

<!-- album-credit-cases:end -->

</div>

E, G and H pass a presence test but do not supply a useful primary credit. Stop at the
preview and fix or exclude those files. C intentionally uses only the first
artist; preserve a joint credit with the list-aware variation below instead.

## The solution

1. Work on a copied album. Save or discard pending edits before opening the
   converter. Inspect its track artists and decide whether one primary artist
   per file is the right album-level policy. For a compilation, enter the
   common reviewed Album Artist through the tag panel and save instead.
2. Choose **Edit → Unselect all**, show the filter and enter:

    <!-- album-credit-filter:start -->
    ```text
    ALBUMARTIST MISSING AND ARTIST PRESENT
    ```
    <!-- album-credit-filter:end -->

    Wait for filtering, or press Enter when auto-apply is disabled. `MISSING`
    means no values; a present empty string is not missing. Filtering can
    inspect pending metadata, which is another reason to resolve edits first.
3. Select only the intended visible practice rows. A filter does not clear
   earlier hidden selections. Exclude empty or whitespace-only artists and
   credits that should not become an album artist.
4. Choose **Convert → Tag → Tag…**, enter `ALBUMARTIST` in **Field**, and use:

    <!-- album-credit-format:start -->
    ```text
    $if2(%albumartist%,%artist%)
    ```
    <!-- album-credit-format:end -->

5. Inspect the preview. `%artist%` is the first Artist value. An empty result
   is a reason to stop and investigate, not a request to guess a name.
6. Apply only the reviewed practice selection. This converter writes
   immediately; it does not leave the result waiting for **Save tags**.
   Reload and inspect the saved fields before processing the rest of the album.

The same format string can be a **Format Value** action with Field
`ALBUMARTIST`. A quick action's **Apply action** also writes immediately;
it is not the converter's preview. Test the definition on a copy before
putting it into a larger group.

## How it works

The filter chooses files with no Album Artist values and at least one Artist
value. `PRESENT` does not require useful nonempty text; E and G demonstrate
why a preview and a human credit decision are still required.

`$if2` returns its first nonempty argument. It reads Album Artist first and
uses Artist when that first text is empty. Ordinary placeholders read the
first value, so this is a primary-credit rule, not a copy-every-artist rule.
If the first Artist value is empty, a later nonempty Artist is not chosen
automatically, as H shows. Spaces are nonempty text. The single-value Format Value result is not
implicitly trimmed.

The filter is what protects existing lists here. Running the scalar formula
on every file could reduce a multi-value Album Artist to its first value.
Do not treat `$if2` as an exact whole-field/list preservation operation.

## Variations

### Preserve all credited Artist values

If the album genuinely has a joint credit, keep the same reviewed missing-field
selection but preview this expression:

<!-- album-credit-list-format:start -->
```text
$meta_sep(ARTIST,$char(92)$char(92))
```
<!-- album-credit-list-format:end -->

`$meta_sep` joins every Artist value with two backslashes. Format Value splits
that result into separate metadata values, trims each part, and drops empty
parts **when the separator is present**. Without it, a nonempty single result
retains its spaces. It is not a universal lossless copy of arbitrary text
containing literal double-backslash pairs.

<div class="native-mapping-table" markdown>

<!-- album-credit-list-cases:start -->

| `ARTIST` values | Resulting `ALBUMARTIST` values |
| --- | --- |
| `["Juniper Vale","Mira Solberg"]` | `["Juniper Vale","Mira Solberg"]` |
| `[" Juniper Vale ","", " Mira Solberg "]` | `["Juniper Vale","Mira Solberg"]` |
| `[""]` | Absent |
| `["   "]` | `["   "]` |

<!-- album-credit-list-cases:end -->

</div>

### Include blank-looking existing fields

A field containing `""` or spaces is present. Inspect its full value list in
Extended tags before changing the filter or clearing it. A test for an empty
value can also match a list that has useful other values. Review those lists
rather than applying a broader query to the whole library.

### Recover from the wrong credit

Read **Edit → Undo …**'s operation label and count in the current desktop
session. History is not a permanent backup and may cover a larger batch than
the current selection. Inspect any pending edits afterward; saving them can
replace the restored values. Keep copies for recovery beyond that session.

## Related

- [Sort fields and compilations](../concepts/sort-fields.md)
- [Find missing fields](find-missing-fields.md)
- [Tag → Tag](../converters/tag-to-tag.md)
- [Multi-value fields](../concepts/multi-value-fields.md)
