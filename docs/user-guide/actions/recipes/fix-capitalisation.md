# Normalize title case and restore chosen exceptions

Combine mechanical Mixed Case with a short, explicit exception policy.
This recipe keeps `DJ` and `EP` uppercase and lowercases interior `and`
and `the` when surrounded by spaces/tabs. It is a practice style, not an
English grammar engine or a recognizer of every artist's preferred spelling.

## Inspect what the policy would do

Work on copied files with single-line title values. In the table, `·` marks
an ordinary space; do not type that visible marker literally.

<!-- capitalisation-cases:start -->
| TITLE before | TITLE after the complete group |
|---|---|
| `THE DJ AND THE SEA (EP)` | `The DJ and the Sea (EP)` |
| `··DJ KOFI AND THE TIDEWATER BAND··` | `DJ Kofi and the Tidewater Band` |
| `ADJACENT EPISODE` | `Adjacent Episode` |
| `THE END AND` | `The End And` |
| `AND THE TIDE` | `And the Tide` |
| `iOS AT THE HARBOUR` | `Ios At the Harbour` |
| `AC/DC LIVE` | `Ac/dc Live` |
<!-- capitalisation-cases:end -->

The last two show why review is essential. If `iOS` or `AC/DC` must keep
that spelling, exclude those files or add a separately tested exception.
This group's two acronym rules do not protect every brand or proper name.

Repair a confirmed encoding error before capitalization: changing the case
of garbled surrogate characters can destroy clues to their original bytes.
See [mojibake repair](fix-mojibake.md). Do not put unrelated artist names,
identifiers or URLs through this title policy with `_TAG` or `_ALL`.

## Build the ordered action group

1. Save or discard pending edits and select one copied practice file.
2. Open **Actions → Actions…**, create `Practice title case exceptions`,
   and open **Edit actions…**.
3. Add the rows below. For every regex row, set **Case-sensitive comparison**
   to **Off**. For Mixed Case, **Words begin after** is the literal set `(-_`.
4. Keep each action and the group, then **Save groups**.
5. Check only this group and choose **Apply checked groups**. Compare the
   saved title and any operation errors before trying the remaining copies.

<!-- capitalisation-actions:start -->
| Step | Action | Field | Pattern or case | Replacement or word separators |
|---|---|---|---|---|
| 1 | Replace with Regular Expression | `TITLE` | `\A[ \t]+` | Leave empty |
| 2 | Replace with Regular Expression | `TITLE` | `[ \t]+\z` | Leave empty |
| 3 | Case Conversion | `TITLE` | Mixed Case | `(-_` |
| 4 | Replace with Regular Expression | `TITLE` | `\bDj\b` | `DJ` |
| 5 | Replace with Regular Expression | `TITLE` | `\bEp\b` | `EP` |
| 6 | Replace with Regular Expression | `TITLE` | `([ \t])And(?=[ \t])` | `$1and` |
| 7 | Replace with Regular Expression | `TITLE` | `([ \t])The(?=[ \t])` | `$1the` |
<!-- capitalisation-actions:end -->

In the action boxes, patterns and replacements are entered directly, without
format-string quotes. The two trim rows leave no unwanted edge spacing.
They matter because the interior-word rules use surrounding whitespace to
distinguish the first and last words from interior ones.

## Why this order works

Mixed Case lowercases the rest of each word and starts words after whitespace
or a configured character. `(-_` adds an opening parenthesis, hyphen and
underscore as word starts. It is a character set, not a regex or an exception
list. The first practice title becomes `The Dj And The Sea (Ep)` at this stage.

The following acronym rules restore exact whole words. `\b` prevents the
`Ep` rule from changing the beginning of `Episode`, or the `Dj` rule from
changing letters inside `Adjacent`.

For a small word, `([ \t])` captures its preceding space/tab. The lookahead
`(?=[ \t])` requires following whitespace without consuming it. `$1and`
or `$1the` puts the leading whitespace back with the chosen lowercase word.
First words have no leading whitespace after trimming, and last words have
no following whitespace, so those words keep their capitals.

Putting Mixed Case after the exception rows would turn `DJ` back into `Dj`
and `EP` back into `Ep`. A rule that depends on a previous result belongs
after the action that creates that result.

## Add exceptions carefully

An acronym list is an editorial choice. Restore `III`, a specific place
name or an unusual artist spelling only when that rule fits the selected
material. Match whole words or an intentionally narrower context, not an
unreviewed substring everywhere.

Words touching punctuation are not all handled by the whitespace-only small
word rules. The group does not parse sentence structure or guarantee a house
style for quotations and parentheses. Inspect those cases and keep them for
manual editing when a general rule would alter meaning.

Every current value of `TITLE` is processed independently. A file with several
title values retains a list rather than being reduced through a bare
`%title%` placeholder. This does not override native format/value restrictions.

## Check writes and recovery

There is no full group dry-run in this recipe. The group writes when applied;
**Save groups** only stores it. Use one copy, compare expected results, then
expand the selection. An action warning does not stop later actions/groups,
so inspect the full reported result before repeating the workflow.

Use **Edit → Undo …** for the recorded operation during the current desktop
session, and keep original files for recovery beyond it. A capitalization
rule cannot reliably reconstruct spelling that an earlier cleanup discarded.

## Related

- [Case Conversion](../case-conversion.md)
- [Regex replacements](../replace-regexp.md)
- [Remove extra spaces](trim-spaces.md)
- [Build your first action group](../first-action-group.md)

<!-- verified: crates/tag-core/src/actions.rs:case_value,RegexReplace,transform. Exact written seven-row sequence, whole-word counterexamples and reverse-order caution execute against core under LF/CRLF. No native UI claim. -->
