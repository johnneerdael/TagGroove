# Split semicolon-separated artist credits

Turn `Juniper Vale; Mira Solberg` into two stored `ARTIST` values. This
recipe uses semicolons as an agreed separator; it leaves `/` and ` & `
inside names such as `AC/DC` or `Kofi & the Tidewater Band` alone.

## Inspect the source structure first

Open **Extended tags…** (++alt+t++; Alt is ⌥ Option on macOS) on a copied
practice file. Confirm whether `ARTIST` has one delimited value or several
existing values. The group below processes each existing value and then
removes exact duplicates across the resulting list.

In this table, each quoted item is one stored value. The brackets illustrate
a list, not characters to type into a tag-panel field.

<!-- artist-list-cases:start -->
| ARTIST before | ARTIST after |
|---|---|
| `["Juniper Vale; Mira Solberg"]` | `["Juniper Vale","Mira Solberg"]` |
| `[" ; Juniper Vale ; ; Mira Solberg; "]` | `["Juniper Vale","Mira Solberg"]` |
| `["AC/DC; Kofi & the Tidewater Band"]` | `["AC/DC","Kofi & the Tidewater Band"]` |
| `["Juniper Vale; Mira Solberg","Juniper Vale"]` | `["Juniper Vale","Mira Solberg"]` |
| `["Juniper Vale; juniper vale"]` | `["Juniper Vale","juniper vale"]` |
<!-- artist-list-cases:end -->

The last row keeps differently capitalized strings. Review whether they
mean the same person before choosing one spelling; an exact duplicate
removal is not an artist-identity lookup.

## Build the ordered group

1. Save or discard pending edits, then select one copied practice file.
2. Open **Actions → Actions…**, create `Practice split artist credits`,
   and open **Edit actions…**.
3. Add these rows in order. Set **Case-sensitive comparison** to **Off**
   for the regex rows. `;` is one literal semicolon.
4. For **Remove Duplicate Fields**, turn **Only duplicated fields with the
   same content** **On**. Keep every action and the group, then **Save groups**.
5. Check only this practice group and choose **Apply checked groups**.

<!-- artist-list-actions:start -->
| Step | Action | Field / source | Pattern / format / separator | Replacement / option |
|---|---|---|---|---|
| 1 | Replace with Regular Expression | `ARTIST` | `\A[ \t;]+` | Leave empty |
| 2 | Replace with Regular Expression | `ARTIST` | `[ \t;]+\z` | Leave empty |
| 3 | Replace with Regular Expression | `ARTIST` | `[ \t]*;[ \t;]*` | `;` |
| 4 | Split Field by Separator | `ARTIST` | `;` | Not used |
| 5 | Remove Duplicate Fields | `ARTIST` | Only duplicated fields with the same content | On |
<!-- artist-list-actions:end -->

The edge rows remove empty semicolon segments at the start/end of each
value (`\A` and `\z` are whole-text anchors). The third row compresses internal runs of semicolons and adjacent
spaces/tabs to one separator. Only then does **Split Field by Separator**
turn text into separate values; it trims each captured piece. The final row
keeps the first occurrence of each exact value, preserving its order.

Turning the final checkbox off would keep only the first artist, including
when later values name different people. It is not an alternative spelling
of “deduplicate.” Check the saved setting before running the group again.

## Why split after cleaning

A bare split can retain empty pieces when the source begins/ends with a
separator or contains repeated separators. Deduplicating those pieces does
not make them meaningful artist names. Cleaning delimiters before the split
avoids that problem for the nonempty credits in this recipe.

A value containing only semicolons/spaces is still not a valid credit.
This group does not invent a name or validate empty input. Inspect and fix
those files separately. It also has no escape syntax for a semicolon that
belongs inside one artist's actual name; exclude such credits from this batch.

If the source already has correct separate values, a delimiter-splitting job
may be unnecessary. To join a list for a particular player, use
[Merge Duplicate Fields](../merge-duplicate-fields.md) deliberately
instead of replacing the list with its first `%artist%` value.

## Check the saved values and the player

After applying, inspect **Extended tags…**: the first practice file should
show Value 1 `Juniper Vale` and Value 2 `Mira Solberg`. Actions write when run;
there is no later Save-tags commit for this group. Inspect errors and use
**Edit → Undo …** for the recorded operation in the current desktop session.

The writer must represent the list in the destination format. A player can
display the same logical values differently, and some fields/formats have
single-value restrictions. Read the file back and test the intended player
before applying this convention throughout a library. See
[multi-value storage](../../tagging/concepts/multi-value-fields.md#storage-depends-on-the-tag-format).
No part of this recipe establishes player compatibility or a native save-back
result on every platform.

## Related

- [Multi-value fields](../../tagging/concepts/multi-value-fields.md)
- [Split Field by Separator](../split-field.md)
- [Remove Duplicate Fields](../remove-duplicate-fields.md)
- [Extended tags](../../tagging/extended-tags.md)

<!-- verified: crates/tag-core/src/actions.rs:RegexReplace, SplitField, RemoveDuplicateFields. Exact written five-row recipe executes in order on all list cases under LF/CRLF. No native UI claim. -->
