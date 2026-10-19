# Filter syntax

The installed TagGroove app evaluates these expressions in the Filter. A query selects matching rows from the loaded file list; it does not edit metadata or scan folders that have not been loaded.

!!! tip "New to this?"
    Follow [Filtering the file list](../guides/filter.md) for showing the Filter, applying a query, restoring all rows, and interpreting errors.

## Ordinary text search

An empty query matches every loaded file. A nonempty query without uppercase filter keywords is a plain search. Each word, separated by the ordinary space character, must occur as a substring in some tag value or the filename. Matching ignores case. Words may occur in different fields or in different values of a multi-value field.

`Juniper Harbour` can match an artist containing Juniper and an album containing Harbour. The same words in another order have the same all-words requirement. Punctuation is literal in plain search, including parentheses, brackets, and `*`; a regular expression is used only with `MATCHES`.

In the current TagGroove version, wrapping that whole plain query in double quotes still uses the all-words rule; it does not require an exact phrase in one field. To require the phrase in a particular field, use a regex filter such as `TITLE MATCHES "Juniper Harbour"`. This pattern contains literal letters and a space; ordinary `HAS` also uses the all-words rule. Whole-query quoted search currently differs from the traced Mp3tag behavior; a compatibility correction remains open.

Plain search includes metadata values plus the filename with extension. It excludes other technical values such as bitrate and full folder path. `* HAS text` searches tag values only, so it can differ from plain search when the text occurs only in the filename.

## Expression grammar

| Form | Question answered |
|---|---|
| `field HAS text` | Does every word in text occur somewhere among this field's values? |
| `field IS text` | Does at least one complete value equal text? |
| `field MATCHES regexp` | Does at least one value match the regular expression? |
| `field GREATER integer` | Is a value's leading integer greater than the threshold? |
| `field LESS integer` | Is a value's leading integer smaller than the threshold? |
| `field EQUAL integer` | Is a value's leading integer equal to the threshold? |
| `field PRESENT` | Does the field supply at least one value, including an empty string? |
| `field MISSING` | Does the field supply no values? |
| `field ABSENT` | Alias of MISSING. |
| `NOT expression` | Invert the result. |
| `expression AND expression` | Require both results. |
| `expression OR expression` | Require either result. |
| `(expression)` | Group conditions explicitly. |

Keywords must be uppercase. Field names and text comparisons ignore case. The all-tag-fields selector `*` is supported only with `HAS` and `MATCHES`; `* IS x` and `* PRESENT` are invalid. It excludes technical names beginning with `_` and does not add the filename.

`IS` compares individual values. If ARTIST has `Juniper Vale` and `Mira Solberg`, `ARTIST IS "Juniper Vale"` succeeds. It does not compare a separator-joined display string. `HAS` can find its different words across those values.

## Boolean precedence

The order is **NOT**, then **AND**, then **OR**. Parentheses override it.

```text
GENRE IS Ambient OR GENRE IS Folk AND TITLE HAS Live
```

This means Ambient files, plus Folk files whose title contains Live. To require Live for both genres, write:

```text
(GENRE IS Ambient OR GENRE IS Folk) AND TITLE HAS Live
```

An uppercase boolean word occurring as data must be quoted: `TITLE IS "AND OR NOT"`. Lowercase `and` is ordinary text, not an alternate spelling of the operator.

## Quote operands

Use one pair of double quotes around a field name or text containing spaces, such as `"RATING WMP" PRESENT` or `ARTIST IS "Juniper Vale"`. Unquoted simple multiword operands are accepted in some positions, but quoting avoids accidental interpretation of uppercase keywords and grouping punctuation. Quote the complete operand, rather than combining quoted and unquoted fragments.

The filter's quoted strings have no escape sequence for an embedded double quote. A backslash inside a quoted string does not turn the next quote into literal data. To find a quotation mark in a field, use an unquoted regex operand:

```text
TITLE MATCHES \"
```

Or build the character through a quoted format string:

```text
"$ifgreater($strstr(%title%,$char(34)),0,yes,no)" IS yes
```

The outer quotes belong to the filter. `$char(34)` produces the quotation mark inside the evaluated format string. Apostrophes inside scripting expressions have the scripting language's own rules; they do not replace the Filter's double-quote grouping.

## Format strings on the left

For `HAS`, `IS`, `GREATER`, `LESS`, `EQUAL`, or `MATCHES`, an operand containing `$` or `%` is evaluated as a format string instead of a literal field name. Enclose it in double quotes:

```text
"%_bitrate%" GREATER 300
```

The selector can combine placeholders and scripting functions. Its result is one comparison value, which can behave differently from comparing a multi-value field directly. Missing placeholders normally evaluate to empty text, so `"%albumartist%" IS ""` can match a missing field even though `ALBUMARTIST IS ""` does not.

Presence tests require a field name, not a calculation. A simple `%albumartist% MISSING` spelling is accepted as a field reference; `$len(...) PRESENT` is invalid. Prefer the clearer `ALBUMARTIST MISSING`.

Evaluated selectors do more work per file than a direct field comparison. Use a direct field when it expresses the same question, especially on a large loaded collection. Available information fields and context-dependent functions are described in [Placeholders](placeholders.md) and [Format strings](format-strings.md).

## Numeric comparisons

The threshold is a signed 64-bit integer. Decimal or duration syntax such as `3.5` or `3:30` is rejected. The field value is interpreted by its leading signed integer after leading whitespace; subsequent text is ignored:

| Stored value | Numeric part |
|---|---|
| `03/08` | 3 |
| `2024-03-15` | 2024 |
| `128.6` | 128 |
| `-6 dB` | -6 |
| `B2` | None; it does not satisfy a numeric comparison. |

This differs from comparing complete text with `IS`. `TRACK EQUAL 3` can match `03/08`, while `TRACK IS 3` does not. It does not extract the total 8. An excessively long digit run in a stored value saturates to the positive/negative numeric bound; it does not wrap around. A threshold outside the signed 64-bit range is invalid.

## Presence, missing values, and empty text

| Field snapshot | `FIELD PRESENT` | `FIELD MISSING` | `FIELD IS ""` |
|---|---|---|---|
| No field | No | Yes | No |
| Field with no values | No | Yes | No |
| One value containing empty text | Yes | No | Yes |
| One nonempty value | Yes | No | No |

The app's save paths can remove empty metadata rather than preserving this exact in-memory distinction. Inspect the values being filtered, including pending edits, before assuming an empty-looking control proves a missing on-disk field.

## Regular expressions

`MATCHES` uses the same Boost-based engine as TagGroove scripting, with case-insensitive matching by default. It finds a match anywhere unless anchors such as `^` and `$` constrain it. Quote a complete pattern when it contains spaces or filter grouping syntax. The word `MATCH` is rejected, rather than treated as another regex operator.

```text
TRACK MATCHES "^[0-9]+/[0-9]+$"
```

This finds a track/total pair of digits. A plain `\d/\d` searches for that shape anywhere and can also match a fragment of a longer value. Consult the [regex reference](regular-expressions.md) for escaping, grouping, assertions, and engine limits.

## Worked examples

The examples below use four invented snapshots. Missing means no field; empty means one empty string. The display names A–D are for explaining results, not field values.

| Row | ARTIST values | TITLE | GENRE | TRACK | YEAR | ALBUMARTIST | Bitrate | Filename |
|---|---|---|---|---|---|---|---|---|
| A | Juniper Vale | Harbour Lights | Indie Folk | 01/08 | 2024 | Missing | 320 | 01 - Harbour Lights.flac |
| B | Juniper Vale | Paper Boats (Live) | Folk | 2/8 | 2024-03-15 | Juniper Vale | 256 | 02 - Paper Boats.mp3 |
| C | Mira Solberg; Juniper Vale | North "Wind" | Ambient | 10 | 2023 | Mira Solberg | 1411 | 10 - North Wind.flac |
| D | The Lanterns | Low Tide | Folk | B2 | Empty | Missing | 128 | bonus.flac |

A and B also have `ALBUM=Harbour Lights`; C has `ALBUM=Night Harbour`. D has no ALBUM.

<!-- filter-examples:start -->
| Query | Matching rows | Why |
|---|---|---|
| `harbour vale` | A, B, C | All words can be found across fields/values. |
| `flac` | A, C, D | Ordinary search includes the filename extension. |
| `* HAS flac` | None | These tag values do not contain flac. |
| `ARTIST HAS Juniper Vale` | A, B, C | Both words occur among artist values. |
| `ARTIST IS "Juniper Vale"` | A, B, C | One complete artist value is enough. |
| `TITLE IS "Harbour Lights"` | A | Compare the complete value. |
| `GENRE IS Folk AND NOT TITLE HAS Live` | D | Exclude the live Folk title. |
| `GENRE IS Ambient OR GENRE IS Folk AND TITLE HAS Live` | B, C | AND binds more tightly than OR. |
| `(GENRE IS Ambient OR GENRE IS Folk) AND TITLE HAS Live` | B | Grouping changes which files must be live. |
| `TRACK GREATER 8` | C | 10 exceeds 8; B2 has no leading integer. |
| `TRACK LESS 3` | A, B | The leading track numbers are 1 and 2. |
| `TRACK EQUAL 2` | B | 2/8 compares numerically as 2. |
| `YEAR EQUAL 2024` | A, B | The leading year of a full date is 2024. |
| `"%_bitrate%" GREATER 300` | A, C | Evaluate a technical placeholder before comparing. |
| `ALBUMARTIST MISSING` | A, D | No values exist for this field. |
| `ALBUMARTIST ABSENT` | A, D | ABSENT has the same result as MISSING. |
| `ALBUMARTIST PRESENT` | B, C | An artist value is present. |
| `YEAR PRESENT` | A, B, C, D | Even the explicit empty string is a value. |
| `YEAR IS ""` | D | Only D has a present, empty year value. |
| `ALBUMARTIST IS ""` | None | Missing is not a stored empty value. |
| `"%albumartist%" IS ""` | A, D | A missing placeholder evaluates to empty text. |
| `TRACK MATCHES "^[0-9]+/[0-9]+$"` | A, B | Require the whole value to be digits/digits. |
| `TITLE MATCHES \"` | C | Find a literal quotation mark. |
| `"$ifgreater($strstr(%title%,$char(34)),0,yes,no)" IS yes` | C | Compute whether the title contains a quotation mark. |
<!-- filter-examples:end -->

## Errors and limits

Invalid syntax, an invalid regular expression, or an unavailable scripting context is reported as a filter error. Correct it rather than interpreting the empty view as a valid no-match result. The parser also rejects overly large queries (more than 1 MiB), more than 8,192 tokens, or excessive nesting (depth limit 128). Regex evaluation has its own limits.

These are TagGroove source-verified rules. They do not establish identical corner-case results in the original Mp3tag executable, in another regex engine, or in TagGroove's simplified browser preview.

## Related

- [Filtering workflow](../guides/filter.md)
- [Field names](field-names.md)
- [Regular expressions](regular-expressions.md)
- [Format strings](format-strings.md)

<!-- verified: baseline007a806 crates/tag-core/src/filter.rs compile_filter, Parser, Selector.values, Expression.matches, leading_integer; crates/tag-core/tests/filter.rs; public example table is executed by crates/tag-core/tests/user_guide_filter_reference.rs. -->

## Fix an invalid query

A filter error names the failed context: an unfinished double quotation,
an invalid format string in the selector, or an invalid `MATCHES` regular
expression. Fix the query and apply it again. A quotation error gives the
character where its opening quote occurred, counting non-ASCII characters
as one character each.

Mixed quoted and bare text in one search or operand is currently rejected.
For example, replace `hidden "orchestra"` with `"hidden orchestra"`, or
`artist HAS hidden "orchestra"` with `artist HAS "hidden orchestra"`.
This is the app's current restriction; exact Mp3tag behavior for mixed
fragments remains unqualified.
