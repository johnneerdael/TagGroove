# Remove extra spaces without flattening every field

Clean the edges and repeated horizontal spacing of a title while keeping
its words and punctuation. This recipe targets `TITLE`; it does not turn
every line break in comments or lyrics into a space.

## Check the practice result

Use copied files with these titles. In this table, `·` represents one ordinary
space, `⇥` a tab and `↵` a line break; do not type those visible markers literally.
Each input is one `TITLE` value.

<!-- spacing-cases:start -->
| TITLE before | TITLE after |
|---|---|
| `··Paper···Boats··` | `Paper Boats` |
| `⇥North⇥⇥Pier⇥` | `North Pier` |
| `Quiet Harbour` | `Quiet Harbour` |
| `AC/DC··-··Live` | `AC/DC - Live` |
| `Paper⇥↵⇥Boats` | `Paper·↵·Boats` |
<!-- spacing-cases:end -->

Spaces can carry meaning in a catalogue number or an identifier. Confirm
that normalizing them is wanted before adding another destination field.

## Try the expression in a preview

With one title value, **Convert → Tag → Tag…** can preview the calculation.
Set **Field** to `TITLE` and use one of these expressions. `·` marks spaces
in the result column too.

For `TITLE` = `··Paper···Boats··`:

<!-- spacing-formulas:start -->
| Format string | Result |
|---|---|
| `$trim(%title%)` | `Paper···Boats` |
| `$trim($regexp(%title%,'[ \t]+', ))` | `Paper Boats` |
<!-- spacing-formulas:end -->

The last argument of `$regexp` is one actual space between the comma and
closing parenthesis. It is neither an empty replacement nor the characters
`space`. `$trim` removes surrounding whitespace; it does not collapse the
spaces between words. The inner `$regexp` turns each run of spaces/tabs
into one space, then `$trim` handles the ends.

Wait for the preview and inspect it. **Apply to … files** writes immediately;
**Cancel** leaves the preview unapplied. A bare `%title%` reads only the first
stored value, so this converter variant replaces an existing multi-title list
with one calculated result. The action sequence below processes each current
value independently instead.

## Save a reusable cleanup group

1. Save or discard pending edits, then select one practice copy.
2. Open **Actions → Actions…**, choose **New…**, and name a group
   `Practice title spacing`.
3. In **Edit actions…**, add the rows below in order. For every regex row,
   set **Case-sensitive comparison** to **Off**.
4. Keep each action, keep the group, then choose **Save groups**. Saving the
   group does not process the audio file.
5. Check only this practice group and choose **Apply checked groups**.

<!-- spacing-actions:start -->
| Step | Action | Field / source | Pattern / format / separator | Replacement / option |
|---|---|---|---|---|
| 1 | Replace with Regular Expression | `TITLE` | `\A[ \t]+` | Leave empty |
| 2 | Replace with Regular Expression | `TITLE` | `[ \t]+\z` | Leave empty |
| 3 | Replace with Regular Expression | `TITLE` | `[ \t]+` | One space |
<!-- spacing-actions:end -->

`\A` anchors the first match to the beginning of the whole field value;
`\z` anchors the second to its end. Unlike `^` and `$` in this engine, they
do not also match at internal line breaks. `[ \t]` means an ordinary space or tab, and `+` requires at least one.
The third action compresses the remaining runs. Doing that row first would
leave one space at either edge until the later edge actions removed it.

Inspect the saved title and operation errors. Actions write when run; they
do not wait for a separate **Save tags**. Use **Edit → Undo …** for the
recorded operation in the current desktop session. Keep copies for recovery
after quitting.

## Common mistakes

- `\s{2,}` is a broader whitespace expression. It can include line breaks,
  so using it across `_TAG` can damage paragraph layout. This recipe uses
  spaces/tabs and the specific `TITLE` field deliberately.
- A single interior tab is also normalized by `[ \t]+`. Use a narrower
  rule if tabs have intentional structure in the destination field.
- Neither variant is a universal cleanup for nonbreaking spaces or every
  Unicode separator. Inspect an unexpected character before choosing a rule.
- An all-whitespace title can become empty. Decide on a real title or a
  deliberate removal rather than treating an empty result as successful naming.
- Adding capitalization or punctuation removal changes this recipe's goal.
  Test those as separate actions before combining them.

## Related

- [Format strings from zero](../../scripting/format-strings-from-zero.md)
- [Regular expressions from zero](../../scripting/regex-from-zero.md)
- [Replace with Regular Expression](../replace-regexp.md)
- [Editing, saving and undo](../../getting-started/editing-and-saving.md)

<!-- verified: crates/tag-core/src/actions.rs:RegexReplace, transform; crates/tag-core/src/script.rs:regexp, trim. Written sequence and formulas execute against core under LF/CRLF. No native UI claim. -->
