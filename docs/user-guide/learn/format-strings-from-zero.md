# Format strings from zero

Build a title from existing tags, add a function, then choose a fallback
when a field is missing. This lesson takes about 15 minutes. You will use
the live preview before saving a practice change.

- [ ] OpenTag installed and a few copied audio files available.
- [ ] Pending edits saved or discarded.
- [ ] One selected practice file with `ARTIST` = `Juniper Vale`,
  `TITLE` = `Paper Boats`, `ALBUM` = `Harbour Lights`, `TRACK` = `3/8`
  and `YEAR` = `2024`.
- [ ] `ALBUMARTIST` and `DISCNUMBER` absent on that practice file.

Use **Extended tags…** (++alt+t++; Alt is ⌥ Option on a Mac) to inspect the
fields first. Keep the original file untouched elsewhere.

## 1. Read a field into the preview

Choose **Convert → Tag → Tag…** (++alt+5++). Set **Field** to `COMMENT`
and **Format string** to:

```text
%title%
```

Wait for the preview. It should show `COMMENT: Paper Boats`. Nothing has
been saved while you are only editing the expression.

The percent signs mark a placeholder. `%title%` reads `TITLE` from the
current file. It does not mean the word "title", and it does not select
the destination; **Field** does that separately.

## 2. Add ordinary text

Replace the expression with:

```text
%artist% - %title%
```

The preview becomes `Juniper Vale - Paper Boats`. OpenTag fills both
placeholders for each selected file and copies the spaces and hyphen
between them literally.

| Expression | Result for the practice file |
|---|---|
| `%title%` | `Paper Boats` |
| `%artist% - %title%` | `Juniper Vale - Paper Boats` |
| `From %album%` | `From Harbour Lights` |
| `Missing: %discnumber%` | <code>Missing: </code> |

A missing field contributes empty text. It does not remove nearby fixed
text automatically. The last row still contains a trailing space.

## 3. Transform a value with a function

Try:

```text
$upper(%title%)
```

The preview becomes `PAPER BOATS`. Read it from the inside out:

1. `%title%` produces `Paper Boats`.
2. `$upper(…)` receives that text as its argument.
3. It returns `PAPER BOATS` to the rest of the format string.

The original `TITLE` is still `Paper Boats`. The function calculates a
result for `COMMENT`; it does not edit its source field on its own.

Functions can take several arguments, separated by commas:

```text
$num(%track%,2) - %title%
```

Here `%track%` supplies `3/8`, and `2` requests a minimum width of two
digits. The result is `03 - Paper Boats`. The total `/8` is discarded by
this numeric conversion.

## 4. Nest two small steps

Try:

```text
$upper($left(%title%,5))
```

`$left(%title%,5)` returns the first five characters, `Paper`.
`$upper` then turns that into `PAPER`. The closing parentheses end the
inner and outer functions in that order.

When an expression fails, build it in these small stages. First preview
the placeholder, then the inner function, then the outer function. A long
expression is easier to understand once each piece has a known result.

??? question "Try it: a shorter album label"
    Build `HARBOUR` from the first seven characters of `ALBUM`.

??? success "Answer"
    `$upper($left(%album%,7))` produces `HARBOUR`.

## 5. Omit an optional piece

Try:

```text
%album%[ (%year%)]
```

The preview shows `Harbour Lights (2024)`. Square brackets mark an optional
section. The year is present, so OpenTag includes that section, including
its space and round brackets.

Now try a missing field:

```text
%album%[ (disc %discnumber%)]
```

The result is `Harbour Lights`; the whole optional section disappears.
Putting the leading space inside the square brackets keeps the output
from ending in an unwanted space.

An optional section depends on field availability, not on whether its
literal text looks nonempty. `[bonus]` alone disappears. Use ordinary
`bonus` when the text should always be present.

## 6. Choose a fallback

For a missing album artist, try:

```text
$if2(%albumartist%,%artist%)
```

The result is `Juniper Vale`. `$if2` takes the first argument when its
availability/truth flag is true; otherwise it uses the second. A field
placeholder is true when it has a nonempty first value, which makes this
a useful field fallback.

Conditions use those flags, not the visible digits `1` and `0`. Compare:

| Expression | Result | Reason |
|---|---|---|
| `$if(%year%,dated,undated)` | `dated` | The year field is present |
| `$if(1,dated,undated)` | `undated` | Literal text carries no true flag |
| `$if($eql(%year%,2024),current,other)` | `current` | The comparison supplies a true flag |

Use a comparison when the condition asks a question about a value.
For example, `$grtr($len(%title%),10)` asks whether the title is longer
than ten characters. Do not assume a function returning the text `0`
has the same flag as a false comparison.

## 7. Quote characters that belong to the text

Inside a function call, commas separate arguments and `)` closes the
call. Quote them when they belong to the text instead:

```text
$replace(%title%,'Paper','Paper, folded')
```

The result is `Paper, folded Boats`. The comma inside quotes is part of
the replacement, not an extra function argument.

| Expression | Result |
|---|---|
| `'%title%'` | `%title%` |
| `It''s %title%` | `It's Paper Boats` |
| `$replace(%title%,Paper,)` | <code> Boats</code> |
| `$replace(%title%,Paper,'')` | `' Boats` |

`''` writes one apostrophe. To supply an empty replacement argument,
leave that argument empty, as in the third row. The space before `Boats`
remains because the search text did not include it.

## 8. Save one reviewed result

Return to:

```text
%artist% - %title%
```

Check that **Field** is still `COMMENT`, wait for the preview, then choose
**Apply to … files**. This writes the comment immediately. Inspect it in
**Extended tags…**. Use **Edit → Undo** in the current desktop session to
reverse the recorded change when you finish practicing.

An empty result can remove a destination field. A filename destination
has different rules: an empty name is rejected and separators may create
folders. Learn the text operations first, then use the filename rules
when that is the actual goal.

## What you learned

- Placeholders read values; the destination chooses where a result goes.
- Functions transform arguments and can be nested from the inside out.
- Optional sections keep punctuation with the field that controls it.
- Comparisons and field availability supply condition flags.
- Quoting protects special characters; an empty argument differs from `''`.
- A preview is a calculation; applying this converter writes the change.

## Next steps

- [Format strings reference](../reference/format-strings.md)
- [Regular expressions from zero](regex-from-zero.md)
- [Regular expressions reference](../reference/regular-expressions.md)

<!-- verified: crates/tag-core/src/script.rs:run, function; src/tag-tag-ui.ts:openTagToTag -->
