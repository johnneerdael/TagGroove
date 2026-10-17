# Move “feat.” artists out of the artist field

<div class="recipe-meta" markdown>
<span>**Uses** filter, action group, [regular expressions](../reference/regular-expressions.md)</span>
<span>**Works on** any tag format</span>
<span>**Time** 5 minutes, then one click each time</span>
<span>**Undo** ++cmd+z++ / ++ctrl+z++</span>
</div>

## The problem

Guest artists are written into the `ARTIST` field, so your player lists
*Juniper Vale feat. Mira Solberg* as an artist of its own, separate from
*Juniper Vale*. You want the guest in the title instead, which is how most
music databases and streaming services do it:

| | Before | After |
|---|---|---|
| `ARTIST` | `Juniper Vale feat. Mira Solberg` | `Juniper Vale` |
| `TITLE` | `Paper Boats` | `Paper Boats (feat. Mira Solberg)` |
| `ARTIST` | `Juniper Vale ft Kofi & the Tidewater Band` | `Juniper Vale` |
| `TITLE` | `North Pier` | `North Pier (feat. Kofi & the Tidewater Band)` |
| `ARTIST` | `Juniper Vale` | `Juniper Vale` (unchanged) |

## The solution

Try this on a copy of a few files first. Check the results before applying
it to your library; use **Edit → Undo** in the same session to reverse the
recorded changes. Keep the originals as a backup.

You'll find the affected files with the filter, then build an action group
with two actions. Order matters: the title has to read the guest's name
before the artist field loses it.

**Find the files**

1. Show the filter with ++f3++ and type:

    ```text
    artist MATCHES "\s(feat\.?|ft\.?|featuring)\s"
    ```

2. Only files with a guest in the artist field remain. Select them all
   (++cmd+a++ / ++ctrl+a++).

**Create the action group**

1. Choose **Actions → Actions…** (++alt+6++; Alt is ⌥ Option on a Mac).
2. Create a new action group and name it `Guests: artist → title`.
3. Add a **Format value** action:
    - **Field:** `TITLE`
    - **Format string:**

        ```text
        %title% (feat. $regexp(%artist%,'^.*?\s+(?:feat\.?|ft\.?|featuring)\s+',))
        ```

4. Add a **Replace with regular expression** action below it:
    - **Field:** `ARTIST`
    - **Regular expression:**

        ```text
        \s+(feat\.?|ft\.?|featuring)\s+.*$
        ```

    - **Replace matches with:** leave it empty
    - **Case-sensitive comparison:** off

5. Save the group, make sure only this group is checked, and run it.

The action group stays in the **Actions** menu. Next time, filter, select and
pick it from the menu. Actions write immediately. If one action reports a
warning for a file, its remaining actions and later checked groups still
run. Review the warnings and the resulting fields before repeating a group.

## How it works

The pattern `(feat\.?|ft\.?|featuring)` covers the common spellings: `feat.`,
`feat`, `ft.`, `ft` and `featuring`. `\.?` makes the dot optional, and
because case is ignored by default, `Feat.` and `FT` match too. `\s` on both
sides requires whole words, so a name like *Kraftwerk* is safe.

**Action 1** builds the new title from three parts:

| Part | Gives |
|---|---|
| `%title%` | `Paper Boats` |
| <code> (feat. </code> | plain text |
| `$regexp(%artist%,'^.*?\s+(?:feat\.?|ft\.?|featuring)\s+',)` | the artist with everything up to and including `feat.` removed: `Mira Solberg` |
| `)` | plain text (outside a function, `)` has no special meaning) |

`.*?` is *lazy*: it takes as little as possible, so the match stops at the
first `feat.`. The pattern and replacement are in single quotes because they
contain characters that format strings treat specially. See
[Quote the pattern and the replacement](../reference/regular-expressions.md#regexp).

**Action 2** deletes everything from the first `feat.` to the end of the
artist field. The action takes its pattern as plain text, so no quotes are
needed here.

!!! warning "Why the filter comes first"
    `$regexp` returns its input unchanged when nothing matches. Run action 1
    on a file *without* a guest, and its title becomes
    `Low Tide (feat. Juniper Vale)`. The filter guarantees that every
    selected file has a guest. If you put the group on a toolbar for regular
    use, keep the filter step.

## Variations

**Keep the guest as a second artist value instead**

Some players handle multi-value artists well. Replace both actions with:

1. **Replace with regular expression** on `ARTIST`: pattern
   `\s+(feat\.?|ft\.?|featuring)\s+`, replacement `;`
2. **Split field by separator** on `ARTIST`, separator `;`

`ARTIST` then holds `Juniper Vale` and `Mira Solberg` as two values. Check
that your player displays multiple artists as intended.

**Use “ft.” or “with” in titles**

Change the text in action 1, for example `%title% (with $regexp(…))`.

**Guests that are already in brackets in the artist field**

For `Juniper Vale (feat. Mira Solberg)`, insert a **Replace with regular
expression** action at the top of the group. Set **Field** to `ARTIST`,
**Regular expression** to `[()]`, and leave **Replace matches with** empty.
This removes all round brackets from the artist field, so use it only on
files where those brackets wrap the guest credit.

The complete group must run in this order; leave **Case-sensitive
comparison** off for both replacement actions:

| Order | Action | Field | Pattern or format string | Replacement |
|---|---|---|---|---|
| 1 | Replace with regular expression | `ARTIST` | `[()]` | *(empty)* |
| 2 | Format value | `TITLE` | <code>%title% (feat. $regexp(%artist%,'^.*?\s+(?:feat\.?&#124;ft\.?&#124;featuring)\s+',))</code> | — |
| 3 | Replace with regular expression | `ARTIST` | <code>\s+(feat\.?&#124;ft\.?&#124;featuring)\s+.*$</code> | *(empty)* |

Step 1 produces `Juniper Vale feat. Mira Solberg`. Step 2 then produces
`Paper Boats (feat. Mira Solberg)`, and step 3 leaves `Juniper Vale` in
`ARTIST`. Putting bracket removal last cannot repair a closing bracket
already copied into the title.

<!-- verified: crates/tag-core/tests/user_guide.rs:published_bracketed_guest_recipe_runs_the_written_actions_in_order -->

## Related

- [Regular expressions](../reference/regular-expressions.md)
- [Format strings](../reference/format-strings.md)
