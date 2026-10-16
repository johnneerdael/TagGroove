# Find files with missing fields

<div class="recipe-meta" markdown>
**Uses:** Filter, `MISSING`, `PRESENT`, `IS`, optional scripting.
**Works on:** Loaded files in the installed app's full filter engine.
**Time:** About 5 minutes to inspect a small practice set.
**Undo:** Filtering writes nothing; clear the query to restore the view.
</div>

## The problem

You want to find tracks without an album artist before filling it in.
A blank-looking cell can mean no value, an empty string, or whitespace.
Those are different states to the filter engine. Work on a copied practice
set before using the results for a tagging operation.

This table describes evaluation snapshots, not a promise that a tag format
will preserve every empty value when saved:

<!-- missing-fields-snapshots:start -->
| ID | `ALBUMARTIST` values | Meaning |
|---|---|---|
| A | Absent | No field/value. |
| B | `[""]` | One present empty string. |
| C | `["   "]` | One present value containing three spaces. |
| D | `["Juniper Vale"]` | A useful credit. |
| E | `["Juniper Vale",""]` | A credit plus an empty second value. |
<!-- missing-fields-snapshots:end -->

## The solution

1. Load the files you want to inspect. Resolve pending edits deliberately:
   save them if they are intended, or discard them before investigating the
   saved tags. Filtering can use the current pending metadata snapshot.
2. Choose **View → Show filter**, or press ++f3++ when it is hidden.
3. Enter this query:

```text
ALBUMARTIST MISSING
```

4. With **Auto-apply filter** enabled, wait for filtering to finish. If it is
   disabled in the helper menu, press Enter. In the example above, only A
   matches. Check **No matching files** versus **Invalid filter**; these are
   different outcomes.
5. Inspect one result in **Extended tags** before deciding what value belongs
   there. A compilation does not necessarily want the first track's artist
   copied into every album-artist field.
6. Clear the query to show the original loaded list. In manual mode, press
   Enter to apply the empty query.

Filtering changes visibility, not the selected-file set. Before any later
write, choose **Edit → Unselect all** and deliberately select the intended
visible rows. A selected file hidden by the filter can otherwise still be
part of a batch operation.

## How it works

`ALBUMARTIST` names a field. `MISSING` asks whether it supplies no values.
`PRESENT` asks the inverse; it does not require nonempty text. Write these
keywords in uppercase. Field names themselves are case-insensitive.

Scroll the table sideways on a narrow screen to read each complete query.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Missing-field filter results">

<!-- missing-fields-queries:start -->
| Query | Matching IDs |
|---|---|
| `ALBUMARTIST MISSING` | `A` |
| `ALBUMARTIST PRESENT` | `B,C,D,E` |
| `ALBUMARTIST IS ""` | `B,E` |
| `ALBUMARTIST MISSING OR ALBUMARTIST IS ""` | `A,B,E` |
| `"$trim(%albumartist%)" IS ""` | `A,B,C` |
<!-- missing-fields-queries:end -->

</div>

`IS ""` does not match three spaces. Also notice E: equality tests any value,
so an empty second entry matches even when another entry contains a credit.
A plain `%albumartist%` placeholder reads the first value, which explains why
the last query does not match E. Choose the question that fits the cleanup;
there is no universal “blank” test that silently means every one of these.

The final query encloses a format-string selector in double quotes. `$trim`
calculates a result for comparison; it does not trim or save the field.
Its whitespace rules are explained in [Trim surrounding spaces](trim-spaces.md).

## Variations

- Replace `ALBUMARTIST` with `TITLE`, `ALBUM` or another actual field name.
  Add `OR` between missing-field checks to find files missing either field;
  use `AND` when both must be absent.
- Limit the task to one album with `(ALBUMARTIST MISSING) AND ALBUM IS
  "Harbour Lights"`. Use the real loaded album value and quote a complete
  multiword operand.
- Use `ALBUMARTIST ABSENT` as an equivalent spelling of `MISSING`. Prefer
  one convention in saved favorites so the intent stays clear.

A numeric zero is still a value, not a missing field. A tag reader/save path
can also remove empty metadata rather than preserving this synthetic
snapshot. Reload and inspect the actual saved result when investigating a
container-specific issue. The browser-only app preview uses a simpler search;
these expressions describe the installed backend's filter engine.

## Related

- [Filtering the file list](../guides/filter.md)
- [Complete filter syntax](../reference/filter-syntax.md)
- [Multi-value fields](../concepts/multi-value-fields.md)
- [Sort fields and compilations](../concepts/sort-fields.md)

<!-- verified: filter_recipe_guide.rs runs the actual five snapshots/five written queries and main solution under LF/CRLF. main.ts supplies current merged fields to filtering; no native selection/write session claim. -->
