# Regular expressions from zero

Clean `03 - Paper_Boats (Live 2024)` into `Paper Boats`, while learning why
each piece matches. This lesson takes about 20 minutes. Use a practice
copy and the live **Tag → Tag** preview; save only after the whole result
makes sense.

- [ ] A copied audio file with `TITLE` = `03 - Paper_Boats (Live 2024)`.
- [ ] Pending edits saved or discarded.
- [ ] Familiarity with placeholders and function arguments from
  [Format strings from zero](format-strings-from-zero.md).

## 1. Give the pattern one small job

Open **Convert → Tag → Tag…** (++alt+5++; Alt is ⌥ Option on a Mac), set
**Field** to `COMMENT`, and enter:

```text
$regexp(%title%,'_',' ')
```

The preview should show `03 - Paper Boats (Live 2024)`.

Read the three arguments separately:

| Argument | Job |
|---|---|
| `%title%` | Supply the text to inspect |
| `'_'` | Match the underscore character |
| `' '` | Replace every match with one space |

A regular expression is a pattern describing the text to match.
`$regexp` searches the input and replaces matches. An underscore has no
special pattern meaning, so this first example behaves like literal
replacement. Keep **Field** as `COMMENT` for now so the source title stays
available while you experiment.

## 2. Recognize a changing number

Replace the expression with:

```text
$regexp(%title%,'^\d+',)
```

The result is ` - Paper_Boats (Live 2024)`, including the leading space.
The final argument is empty because the goal is deletion. Do not substitute
`''`: that means one literal apostrophe in a format string.

| Pattern part | Meaning | What it matches here |
|---|---|---|
| `^` | Start of the input | Position before `0` |
| `\d` | A digit | `0` or `3` |
| `+` | One or more of the preceding item | Both digits, `03` |

The `+` lets the same pattern handle `3`, `03` or `103`. The `^` protects
numbers elsewhere, including the year at the end.

## 3. Include the separator in the match

Deleting the number left its separator behind. Replace the pattern with:

```text
$regexp(%title%,'^\d+\s*-\s*',)
```

The result is `Paper_Boats (Live 2024)`.

`\s*` means zero or more whitespace characters. The first one consumes
spaces between the number and hyphen; the second consumes spaces after
the hyphen. `-` is literal here. Requiring the hyphen helps distinguish a
track prefix from a title beginning with a meaningful number.

| Input | Pattern | Replacement | Result |
|---|---|---|---|
| `03 - Paper_Boats (Live 2024)` | `^\d+\s*-\s*` | Leave empty | `Paper_Boats (Live 2024)` |
| `3-Paper Boats` | `^\d+\s*-\s*` | Leave empty | `Paper Boats` |
| `2024 Nights` | `^\d+\s*-\s*` | Leave empty | `2024 Nights` |

In the last row, the required hyphen is absent. No match leaves the input
unchanged; it is not a regex error.

??? question "Try it: require at least one space"
    Which quantifier would require whitespace instead of allowing none?

??? success "Answer"
    `\s+` requires one or more whitespace characters. `\s*` permits zero.

## 4. Capture text to reuse it

Instead of deleting a prefix, capture the pieces:

```text
$regexp(%title%,'^(\d+)\s*-\s*(.*)$','$2')
```

The result is again `Paper_Boats (Live 2024)`. The round brackets create
numbered capture groups:

| Group | Captured text |
|---|---|
| `$1` | `03` |
| `$2` | `Paper_Boats (Live 2024)` |

`.*` captures zero or more of any ordinary character. The final `$` marks
the end, so the pattern describes the whole title. `$2` in the replacement
copies the second captured group. The quotes around it protect the dollar
sign from the outer format-string parser.

For comparison, replacing with `'$2 — track $1'` produces
`Paper_Boats (Live 2024) — track 03`. Capture references rearrange matched
text; they do not read tag fields.

## 5. Match literal brackets and a year

The version suffix is another independent job:

```text
$regexp(%title%,'\s+\(Live \d{4}\)$',)
```

The result is `03 - Paper_Boats`.

| Pattern part | Why it is present |
|---|---|
| `\s+` | Remove the space before the suffix too |
| `\(` and `\)` | Match actual round brackets rather than create a group |
| `Live ` | Match that word and its following space |
| `\d{4}` | Match exactly four digits |
| `$` | Require the suffix at the end |

OpenTag ignores case by default in `$regexp`, so `live` also matches.
Add a fourth argument `0` when the case must be significant:
`$regexp(%title%,'\s+\(Live \d{4}\)$',,0)`.

This pattern removes a very specific suffix. It will not remove `(Live)`
without a year or `(Studio 2024)`. Add new rules only after deciding those
values should also change.

## 6. Combine the three verified steps

Use the prefix and suffix cleanup as nested functions, then change
underscores into spaces:

```text
$replace($regexp($regexp(%title%,'^\d+\s*-\s*',),'\s+\(Live \d{4}\)$',),_,' ')
```

Read from the inside outward:

| Step | Intermediate text |
|---|---|
| Read `%title%` | `03 - Paper_Boats (Live 2024)` |
| Remove track prefix | `Paper_Boats (Live 2024)` |
| Remove the version suffix | `Paper_Boats` |
| Replace underscores | `Paper Boats` |

The preview should now show `Paper Boats`. Keeping the jobs separate
made the combined expression explainable; it also makes it easier to find
which step needs changing when a new naming convention appears.

## 7. Look around without consuming text

For one more pattern technique, try this standalone expression:

```text
$regexp(%title%,'\d{4}(?=\)$)',YEAR)
```

It produces `03 - Paper_Boats (Live YEAR)`. `(?=\)$)` is a lookahead: the
digits must be followed by a closing bracket at the end, but that bracket
is checked rather than included in the match. It therefore survives the
replacement without being copied back.

Lookarounds are useful when the surrounding context decides whether a
match is valid but should remain unchanged. Use an ordinary capture group
when that communicates the task more clearly.

## 8. Apply the reviewed cleanup

Return to the combined expression in step 6. Change **Field** from
`COMMENT` to `TITLE`, wait for the preview and check the exact output.
Choose **Apply to … files** to save `Paper Boats` into the title.
Inspect the result and use **Edit → Undo** to reverse the recorded practice
change in the current desktop session.

Before applying the expression to a collection, test copies that include
missing prefixes, unusual punctuation and suffixes you want to retain.
Successful matching on this lesson's one input is not proof that every
title in a library follows the same convention.

## What you learned

- Match fixed characters first, then describe the parts that vary.
- Use anchors to protect unrelated numbers elsewhere in the title.
- Include unwanted separators in a deletion pattern.
- Capture groups save text for `$1`, `$2` and later references.
- Escape literal regex punctuation and quote patterns inside `$regexp`.
- Lookarounds check context while leaving that context outside the match.
- Verify each transformation before nesting or saving it.

## Next steps

- [Regular expressions reference](../reference/regular-expressions.md)
- [Format strings reference](../reference/format-strings.md)
- [Move guest artists to titles](../cookbook/feat-artists-to-title.md)

<!-- verified: crates/regex-compat; crates/tag-core/src/script.rs:regexp; src/tag-tag-ui.ts:openTagToTag -->
