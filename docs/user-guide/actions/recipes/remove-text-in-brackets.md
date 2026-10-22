# Remove a trailing remaster annotation

Remove a known `Remastered` suffix while retaining parentheses that belong
to a title. A rule that deletes every bracketed phrase would also remove
useful information such as `(Live)` or part of a work's name.

## Start with known practice titles

Each row below starts from a separate copied file with one title value.
The recipe accepts parentheses or square brackets around `Remastered`,
optionally followed by a space and a four-digit year. Matching ignores case.
`↵` represents a line break within one title value.

<!-- bracket-cases:start -->
| TITLE before | TITLE after |
|---|---|
| `Paper Boats (Remastered 2021)` | `Paper Boats` |
| `North Pier [remastered]` | `North Pier` |
| `Quiet Harbour (Live)` | `Quiet Harbour (Live)` |
| `Remastered Memories` | `Remastered Memories` |
| `Paper Boats (Remastered edition)` | `Paper Boats (Remastered edition)` |
| `Quiet Harbour (Part (One))` | `Quiet Harbour (Part (One))` |
| `Paper Boats (Remastered 2021)↵Notes` | `Paper Boats (Remastered 2021)↵Notes` |
<!-- bracket-cases:end -->

The last five are intentional nonmatches. Inspect a sample of the titles
that should stay unchanged as well as those that should change.

## Build the two-action group

1. Save or discard pending edits and select one copied practice file.
2. Open **Actions → Actions…**, create `Practice remove remaster suffix`,
   and open **Edit actions…**.
3. Add these rows in order. Set **Case-sensitive comparison** to **Off**
   in both regex actions. Leave the replacement box completely empty.
4. Keep the actions and group, then **Save groups**.
5. Check only this group and choose **Apply checked groups**. Inspect the
   saved title before trying the remaining practice copies.

<!-- bracket-actions:start -->
| Step | Action | Field / source | Pattern / format / separator | Replacement / option |
|---|---|---|---|---|
| 1 | Replace with Regular Expression | `TITLE` | `[ \t]*\(Remastered(?: [0-9]{4})?\)\z` | Leave empty |
| 2 | Replace with Regular Expression | `TITLE` | `[ \t]*\[Remastered(?: [0-9]{4})?\]\z` | Leave empty |
<!-- bracket-actions:end -->

Type the pattern directly in the action's regex box, without wrapping it
in format-string quotes. A regex action's replacement box is also literal:
`''` would insert apostrophes rather than requesting an empty replacement.

## Understand what the patterns permit

- `[ \t]*` includes spaces/tabs immediately before the annotation, so the
  deletion does not leave an extra trailing space.
- `\(` and `\)` match literal parentheses. The second row uses `\[` and
  `\]` for literal square brackets; the pairs must match.
- `Remastered` is the exact annotation word, with case ignored by the setting.
- `(?: [0-9]{4})?` allows either nothing or one space followed by exactly
  four digits. `(?:…)` groups those pieces without a capture number.
- `\z` restricts the annotation to the end of the whole value, including
  values containing a line break. `$` would also match at line ends here. `Remastered Memories` therefore
  does not match the suffix pattern.

Normalize unwanted trailing spaces first with the [spacing recipe](trim-spaces.md),
or a space after the closing bracket can prevent this end-anchored match.
These are two ordered replacements, not a repeated “remove all annotations”
loop. For example, removing a final parenthesized annotation can expose a
square-bracketed one for the next row; two consecutive parenthesized suffixes
need a separate decision before repeating the group.

## When removing any parenthesized suffix is intended

For a carefully reviewed selection, a broader pattern is:

<!-- broad-bracket-examples:start -->
| Pattern | Input | Result |
|---|---|---|
| `[ \t]*\([^()]*\)\z` | `Paper Boats (Live)` | `Paper Boats` |
| `[ \t]*\([^()]*\)\z` | `Quiet Harbour (Part (One))` | `Quiet Harbour (Part (One))` |
| `[ \t]*\([^()]*\)\z` | `Paper Boats (Remastered 2021)↵Notes` | `Paper Boats (Remastered 2021)↵Notes` |
<!-- broad-bracket-examples:end -->

Use this as a separate regex action with an empty replacement, not as a
silent substitute for the remaster rule. `[^()]*` permits characters other
than parentheses, so it does not parse nested parentheses. A broad `.*`
can instead span from an earlier opening bracket to the final closing one
and remove more of the title than expected.

## Verify and recover

A regex action runs immediately; there is no full-group dry-run preview in
this recipe. Start with one copy, compare the before/after title, and check
warnings. A no-match file remains unchanged by these replacements. An
annotation-only title can leave no useful title; inspect those cases before
using a cleaned value to rename files.

Use **Edit → Undo …** promptly for a recorded action write. History belongs
to the current desktop session; a file backup remains the recovery route
beyond it. Do not combine this cleanup with a destructive field-removal action
until its intermediate result has been reviewed.

## Related

- [Regex syntax and replacements](../../scripting/regular-expressions.md)
- [Replace with Regular Expression](../replace-regexp.md)
- [Remove extra spaces](trim-spaces.md)
- [Actions and action groups](../index.md)

<!-- verified: crates/tag-core/src/actions.rs:RegexReplace, regex_values; crates/regex-compat. Every written practice and broader pattern example executes against core under LF/CRLF. No native UI claim. -->
