# Text and case

Use these functions to transform text, select characters and control capitalization.
Functions read values and return text; they do not change a source tag until the
surrounding converter or action writes the result.

!!! tip "New to this?"
    Follow [Format strings from zero](../../learn/format-strings-from-zero.md),
    then return here for individual argument rules.

Examples using `%title%` assume `TITLE` = `Paper Boats`. A middle dot `·` in a
result marks a significant space; type an actual space when reproducing it.
Character counts and positions use Unicode scalar values. Combining characters
can therefore count separately from the character they decorate.

### `$lower(x)` { #lower }

**Returns** `x` with lowercase letters.

| Argument | Meaning |
|---|---|
| `x` | Text to transform. |

| Format string | With these values | Result |
|---|---|---|
| `$lower(%title%)` | `TITLE` = `Paper Boats` | `paper boats` |
| `$lower(ÉTÉ)` | Any file | `été` |
| `$lower($left(%title%,5))` | `TITLE` = `Paper Boats` | `paper` |

Case conversion can change a name's intended spelling. It does not remove accents.

**Related:** [`$upper`](#upper).

### `$upper(x)` { #upper }

**Returns** `x` with uppercase letters.

| Argument | Meaning |
|---|---|
| `x` | Text to transform. |

| Format string | With these values | Result |
|---|---|---|
| `$upper(%title%)` | `TITLE` = `Paper Boats` | `PAPER BOATS` |
| `$upper(été)` | Any file | `ÉTÉ` |
| `$upper($left(%title%,5))` | `TITLE` = `Paper Boats` | `PAPER` |

Unicode uppercasing can change the number of characters; do not assume it preserves string length.

**Related:** [`$lower`](#lower).

### `$caps(x[,separators])` { #caps }

**Returns** text with word starts uppercased and other characters lowercased.

| Argument | Meaning |
|---|---|
| `x` | Text to transform. |
| `separators` | Characters that start the next word. Omit for an ordinary space; a supplied set replaces that default, including an empty set. |

| Format string | With these values | Result |
|---|---|---|
| `$caps(PAPER BOATS)` | Any file | `Paper Boats` |
| `$caps(DJ NIMBUS)` | Any file | `Dj Nimbus` |
| `$caps($replace(low_tide,_,' '))` | Any file | `Low Tide` |

This is mechanical capitalization, not a language-aware title style. `DJ`
becomes `Dj`. Separators are individual characters, not a regex. ASCII letters
in that set match either case: `d` also recognizes `D`. A separator at the
start of the text is kept as written; a non-separator starts uppercase at the
start or immediately after a separator.

The default is only the ordinary space (U+0020). Tabs and line breaks are not
word starts unless included explicitly. For example, a second argument of
`' '$char(9)$char(10)$char(13)` includes space, tab, LF and CR.
To retain spaces as word starts when adding a hyphen, include both characters:

<!-- caps-separator-examples:start -->
| Format string | With these values | Result |
|---|---|---|
| `$caps('LOW-TIDE HIGH WIND')` | Any file | `Low-tide High Wind` |
| `$caps('LOW-TIDE HIGH WIND',-)` | Any file | `Low-Tide high wind` |
| `$caps('LOW-TIDE HIGH WIND','- ')` | Any file | `Low-Tide High Wind` |
| `$caps('LOW TIDE',)` | Any file | `Low tide` |
| `$caps(aDb,d)` | Any file | `AdB` |
| `$caps(dABC,d)` | Any file | `dAbc` |
<!-- caps-separator-examples:end -->

**Related:** [`$caps2`](#caps2).

### `$caps2(x[,separators])` { #caps2 }

**Returns** text with word starts uppercased while preserving other characters.

| Argument | Meaning |
|---|---|
| `x` | Text to transform. |
| `separators` | The same replacement trigger set as `$caps`: omitted means ordinary space; an explicit empty set means no separators. ASCII letter triggers match either case. |

| Format string | With these values | Result |
|---|---|---|
| `$caps2(pAPER bOATS)` | Any file | `PAPER BOATS` |
| `$caps2(DJ NIMBUS)` | Any file | `DJ NIMBUS` |
| `$caps2($lower(%title%))` | `TITLE` = `Paper Boats` | `Paper Boats` |

This does not repair arbitrary uppercase text: existing uppercase letters
after the first character remain uppercase. As with `$caps`, adding `-`
replaces the default space trigger unless the set includes a space too:

<!-- caps2-separator-examples:start -->
| Format string | With these values | Result |
|---|---|---|
| `$caps2('low-tide dj')` | Any file | `Low-tide Dj` |
| `$caps2('low-tide dj',-)` | Any file | `Low-Tide dj` |
| `$caps2('low-tide dj','- ')` | Any file | `Low-Tide Dj` |
| `$caps2('a b',)` | Any file | `A b` |
| `$caps2(aDb,d)` | Any file | `ADB` |
| `$caps2(dABC,d)` | Any file | `dABC` |
<!-- caps2-separator-examples:end -->

**Related:** [`$caps`](#caps).

### `$caps3(x[,triggers])` { #caps3 }

**Returns** text with a non-trigger uppercased at the first position or
immediately after a trigger. Other later characters are lowercased. An initial
trigger is kept as written.

| Argument | Meaning |
|---|---|
| `x` | Text to transform. |
| `triggers` | Optional characters that trigger the next position. Omitted means no triggers. Matching is exact, including ASCII case. |

| Format string | With these values | Result |
|---|---|---|
| `$caps3(PAPER BOATS)` | Any file | `Paper boats` |
| `$caps3('FIRST. SECOND',.)` | Any file | `First. second` |
| `$caps3($upper(%title%))` | `TITLE` = `Paper Boats` | `Paper boats` |

A trigger affects the next position without skipping spaces. In the period
example it triggers the space, and `second` remains lowercase. Consecutive
triggers are lowercased after the initial position, and can trigger the
following non-trigger. Unlike `$caps` and `$caps2`, `d` does not recognize `D`:

<!-- caps3-trigger-examples:start -->
| Format string | With these values | Result |
|---|---|---|
| `$caps3(dABC,d)` | Any file | `dAbc` |
| `$caps3(ddA,d)` | Any file | `ddA` |
| `$caps3(aDb,d)` | Any file | `Adb` |
| `$caps3('FIRST..SECOND',.)` | Any file | `First..Second` |
<!-- caps3-trigger-examples:end -->

<!-- verified: crates/tag-core/tests/guide_scripting_details.rs:published_caps_examples_cover_replacement_ascii_folding_and_distinct_sentence_case -->

**Related:** [`$caps`](#caps).

### `$len(x)` { #len }

**Returns** the number of Unicode characters in `x`.

| Argument | Meaning |
|---|---|
| `x` | Text to count. |

| Format string | With these values | Result |
|---|---|---|
| `$len(%title%)` | `TITLE` = `Paper Boats` | `11` |
| `$len(é猫🙂)` | Any file | `3` |
| `$ifgreater($len(%title%),10,long,short)` | `TITLE` = `Paper Boats` | `long` |

The count uses Unicode scalar values, not bytes or user-perceived grapheme clusters. A combining accent can count separately.

**Related:** [`$left`](#left).

### `$left(x,n)` { #left }

**Returns** at most the first `n` characters of `x`.

| Argument | Meaning |
|---|---|
| `x` | Text to read. |
| `n` | Character count; zero or negative means none. |

| Format string | With these values | Result |
|---|---|---|
| `$left(%title%,5)` | `TITLE` = `Paper Boats` | `Paper` |
| `$left(abc,0)` | Any file | *(empty)* |
| `$upper($left(%title%,5))` | `TITLE` = `Paper Boats` | `PAPER` |

Asking for more characters than exist returns the available text. It does not pad the result.

**Related:** [`$right`](#right).

### `$right(x,n)` { #right }

**Returns** at most the final `n` characters of `x`.

| Argument | Meaning |
|---|---|
| `x` | Text to read. |
| `n` | Character count; zero or negative means none. |

| Format string | With these values | Result |
|---|---|---|
| `$right(%title%,5)` | `TITLE` = `Paper Boats` | `Boats` |
| `$right(abc,9)` | Any file | `abc` |
| `$lower($right(%title%,5))` | `TITLE` = `Paper Boats` | `boats` |

The count is in Unicode characters. For a known prefix to discard, `$cutleft` often states the intent more clearly.

**Related:** [`$cutleft`](#cutleft).

### `$mid(x,start[,n])` { #mid }

**Returns** characters beginning at position `start`, counting from 1. If `n` is omitted, return one character.

| Argument | Meaning |
|---|---|
| `x` | Text to read. |
| `start` | First position, starting at 1; zero or negative produces empty text. |
| `n` | Optional maximum character count, initially 1. |

| Format string | With these values | Result |
|---|---|---|
| `$mid(%title%,7,5)` | `TITLE` = `Paper Boats` | `Boats` |
| `$mid(abc,0,2)` | Any file | *(empty)* |
| `$upper($mid(%title%,7))` | `TITLE` = `Paper Boats` | `B` |

Omitting the length does not mean the rest of the string. Positions beyond the end return empty text.

**Related:** [`$len`](#len).

### `$cutleft(x,n)` { #cutleft }

**Returns** `x` after removing up to `n` characters from its beginning.

| Argument | Meaning |
|---|---|
| `x` | Text to shorten. |
| `n` | Number of initial characters to remove. |

| Format string | With these values | Result |
|---|---|---|
| `$cutleft(%title%,6)` | `TITLE` = `Paper Boats` | `Boats` |
| `$cutleft(abc,9)` | Any file | *(empty)* |
| `$upper($cutleft(%title%,6))` | `TITLE` = `Paper Boats` | `BOATS` |

A count beyond the text's length removes everything. A zero or negative count leaves it unchanged.

**Related:** [`$cutright`](#cutright).

### `$cutright(x,n)` { #cutright }

**Returns** `x` after removing up to `n` characters from its end.

| Argument | Meaning |
|---|---|
| `x` | Text to shorten. |
| `n` | Number of final characters to remove. |

| Format string | With these values | Result |
|---|---|---|
| `$cutright(%title%,6)` | `TITLE` = `Paper Boats` | `Paper` |
| `$cutright(abc,0)` | Any file | `abc` |
| `$lower($cutright(%title%,6))` | `TITLE` = `Paper Boats` | `paper` |

The function counts characters, not suffix words. Use a regex if the suffix varies in length or should be checked before removal.

**Related:** [`$cutleft`](#cutleft).

### `$trim(x[,chars])` { #trim }

**Returns** `x` with matching characters removed from both ends.

| Argument | Meaning |
|---|---|
| `x` | Text to trim. |
| `chars` | Optional set of characters to remove; otherwise trim whitespace. |

| Format string | With these values | Result |
|---|---|---|
| `$trim('  Paper Boats  ')` | Any file | `Paper Boats` |
| `$trim('--Title--',-)` | Any file | `Title` |
| `$upper($trim(' Paper Boats '))` | Any file | `PAPER BOATS` |

The optional argument is a character set, not a substring. An explicitly empty set removes nothing; interior whitespace is not collapsed.

**Related:** [`$trimleft`](#trimleft).

### `$trimleft(x[,chars])` { #trimleft }

**Returns** `x` with matching characters removed only from its beginning.

| Argument | Meaning |
|---|---|
| `x` | Text to trim. |
| `chars` | Optional trim-character set; otherwise whitespace. |

| Format string | With these values | Result |
|---|---|---|
| `$trimleft('  Paper Boats  ')` | Any file | `Paper Boats··` |
| `$trimleft('--Title--',-)` | Any file | `Title--` |
| `$upper($trimleft(' Paper Boats '))` | Any file | `PAPER BOATS·` |

Trailing text and spaces remain, as the first result shows. The optional set is not a literal prefix to remove once.

**Related:** [`$trimright`](#trimright).

### `$trimright(x[,chars])` { #trimright }

**Returns** `x` with matching characters removed only from its end.

| Argument | Meaning |
|---|---|
| `x` | Text to trim. |
| `chars` | Optional trim-character set; otherwise whitespace. |

| Format string | With these values | Result |
|---|---|---|
| `$trimright('  Paper Boats  ')` | Any file | `··Paper Boats` |
| `$trimright('--Title--',-)` | Any file | `--Title` |
| `$upper($trimright(' Paper Boats '))` | Any file | `·PAPER BOATS` |

Leading spaces remain. Use `$trim` when both ends should be cleaned.

**Related:** [`$trim`](#trim).

### `$repeat(x,n)` { #repeat }

**Returns** `x` repeated `n` times.

| Argument | Meaning |
|---|---|
| `x` | Text to repeat. |
| `n` | Repeat count; zero or negative produces empty text. |

| Format string | With these values | Result |
|---|---|---|
| `$repeat(la,3)` | Any file | `lalala` |
| `$repeat(x,0)` | Any file | *(empty)* |
| `$upper($repeat(la,2))` | Any file | `LALA` |

The function adds no separators. Excessively large results report an output-limit error.

**Related:** [`$replace`](#replace).

### `$reverse(x)` { #reverse }

**Returns** `x` with its character order reversed.

| Argument | Meaning |
|---|---|
| `x` | Text to reverse. |

| Format string | With these values | Result |
|---|---|---|
| `$reverse(%title%)` | `TITLE` = `Paper Boats` | `staoB repaP` |
| `$reverse(%missing%)` | `MISSING` absent | *(empty)* |
| `$upper($reverse(ab))` | Any file | `BA` |

Reversal works on Unicode scalar values, so combining sequences and other grapheme clusters may be split.

**Related:** [`$len`](#len).

### `$replace(x,from,to[,from,to…])` { #replace }

**Returns** `x` with every exact occurrence of each search string replaced, processing the pairs in order.

| Argument | Meaning |
|---|---|
| `x` | Original text. |
| `from` | Nonempty literal search string; matching is case-sensitive. |
| `to` | Literal replacement; an empty argument deletes matches. |

| Format string | With these values | Result |
|---|---|---|
| `$replace(%title%,Boats,Ships)` | `TITLE` = `Paper Boats` | `Paper Ships` |
| `$replace(aaa,a,b,b,c)` | Any file | `ccc` |
| `$upper($replace(%title%,' ',_))` | `TITLE` = `Paper Boats` | `PAPER_BOATS` |

Later pairs see earlier replacements. This is not regex replacement: `$1` is literal text here. Supply complete search/replacement pairs, with 3–63 arguments in total; an empty search string is an error.

**Related:** [`$trim`](#trim).

### `$char(n)` { #char }

**Returns** the Unicode character at numeric code point `n`.

| Argument | Meaning |
|---|---|
| `n` | A valid Unicode scalar value interpreted as an integer. |

| Format string | With these values | Result |
|---|---|---|
| `$char(65)` | Any file | `A` |
| `$char(233)` | Any file | `é` |
| `$lower($char(65))` | Any file | `a` |

Negative, surrogate or out-of-range code points report errors. Although `$char` supports supplementary characters, `$ord` is limited to U+FFFF.

**Related:** [`$ord`](#ord).

### `$ord(x)` { #ord }

**Returns** the numeric code point of the first character, limited to U+0000–U+FFFF. An empty input returns 0.

| Argument | Meaning |
|---|---|
| `x` | Text whose first character is inspected. |

| Format string | With these values | Result |
|---|---|---|
| `$ord(A)` | Any file | `65` |
| `$ord(%missing%)` | `MISSING` absent | `0` |
| `$add($ord(a),1)` | Any file | `98` |

Later characters are ignored. A first character above U+FFFF, such as many emoji, reports an error rather than returning its full code point.

**Related:** [`$char`](#char).

### `$ansi(x)` { #ansi }

**Returns** text retaining characters representable in the application's ANSI codepage, with unrepresentable characters replaced by `?`.

| Argument | Meaning |
|---|---|
| `x` | Unicode text to round-trip through the system ANSI encoding. |

| Format string | With these values | Result |
|---|---|---|
| `$ansi(Café)` | ANSI codepage windows-1252 | `Café` |
| `$ansi(猫)` | ANSI codepage windows-1252 | `?` |
| `$upper($ansi(Café))` | ANSI codepage windows-1252 | `CAFÉ` |

These examples assume Windows-1252. In a UTF-8 context the text is preserved. This operation can lose characters; it is not a repair for already-misdecoded legacy bytes. Missing or unsupported codepage context reports an error.

**Related:** [`$replace`](#replace).

<!-- verified: crates/tag-core/src/script.rs:function, title_case, ansi, validate_arity -->

## Related

- [Search and compare](search-compare.md)
- [Conditions and logic](conditions.md)
- [Regular expressions](../regular-expressions.md)
- [Scripting functions](index.md)
