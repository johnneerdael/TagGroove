# Regular expressions

<p class="lead">A regular expression (regex) describes a <em>shape</em> of text rather than exact text: “a number at the start”, “anything in brackets at the end”, “two words separated by a comma”. TagGroove uses them to find that shape in your tags and file names, and to rewrite it.</p>

This page takes you from your first pattern to lookarounds, then documents
the main places TagGroove accepts regular expressions. The web-source parser
commands have their own [reference](../tag-sources/web-source-reference.md).
Each section builds on the one before. If you already know regular expressions from another program,
skip to [How TagGroove's regular expressions differ](#how-taggroove-differs) and
the [`$regexp` reference](#regexp).

!!! tip "Try every example as you read"
    Select a file, open **Convert → Tag → Tag…** (++alt+5++; Alt is ⌥ Option
    on a Mac), choose a field such as `TITLE` and type a `$regexp(…)` format
    string. The preview updates as you type, and nothing changes until you
    apply it. Start with the examples below on a copy of your files.

## Where you can use regular expressions

| Where | What it does | Case |
|---|---|---|
| [`$regexp(…)`](#regexp) in any format string | Replaces every match and returns the result | Ignored by default; pass `0` as the fourth argument to respect it |
| [**Replace with regular expression**](#the-replace-with-regular-expression-action) action | Rewrites the values of a field, or the file name | **Case-sensitive comparison** checkbox, off by default |
| [Filter](#in-the-filter-and-find): `field MATCHES pattern` | Shows only files whose field matches | Always ignored; write `(?-i)` to respect it |
| [Find](#in-the-filter-and-find), strategy **Regular expression** | Lists matching filename/field values | **Case sensitive** checkbox |
| [Web-source parser commands](../tag-sources/web-source-reference.md#sayregexp), such as SayRegexp and RegexpReplace | Read or transform downloaded text and parser outputs | Case-sensitive by default |

Format strings, actions, filters, Find and web-source parser commands use the
shared Boost-compatible core. Case defaults, quoting and replacement processing
still depend on the context. Find searches without replacing text;
[its case control and limits](#find-dialect) are described separately.

The examples use `$regexp(text,pattern,replacement)` and show the text,
pattern and replacement in separate columns. You'd normally use a placeholder
such as `%title%` as the text.

## 1. Plain text is a pattern too

The simplest regular expression is ordinary text. It finds that text
wherever it occurs, and **every** occurrence is replaced:

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Salt & Cedar` | `&` | `and` | `Salt and Cedar` |
| `Low Tide, Low Light` | `Low` | `High` | `High Tide, High Light` |
| `Paper Boats` | `boats` | `Planes` | `Paper Planes` |
| `Paper Boats` | `Snow` | `Rain` | `Paper Boats` |

Two things to notice:

- In `$regexp`, case is ignored unless you ask otherwise, so `boats` found
  `Boats`.
- When nothing matches, you get the text back unchanged. That's convenient,
  but it also means a typo in a pattern fails silently. Check the preview.

Some characters have a special meaning in patterns: `. ^ $ * + ? ( ) [ ] { } | \`.
To match one of them literally, put a backslash in front: `\.`, `\(`, `\?`.

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Fog Horn (Interlude)` | <code> &#92;(Interlude&#92;)</code> | *(empty)* | `Fog Horn` |
| `Who's There?` | `\?` | *(empty)* | `Who's There` |

## 2. Character classes: one character from a set

A class matches **one** character from a set.

| Pattern | Matches one… | Example match in `Track 07: Low Tide` |
|---|---|---|
| `.` | character of any kind | `T` (the first character) |
| `\d` | digit `0`–`9` | `0` |
| `\w` | word character: letter, digit or `_` | `T` |
| `\s` | space, tab or line break | the space after `Track` |
| `\D` `\W` `\S` | character that is **not** a digit / word character / space | `T` / the space / `T` |
| `[aeiou]` | of the listed characters | `a` |
| `[0-9a-f]` | in one of the ranges | `a` |
| `[^0-9]` | **not** in the set (`^` right after `[`) | `T` |
| `[[:punct:]]` | punctuation (also `[[:alpha:]]`, `[[:upper:]]`, `[[:digit:]]`…) | `:` |

### Case classes, control characters and code points

Some escapes have different meanings in a pattern and a replacement.
The following table describes their **pattern** meaning:

| Pattern | Matches |
|---|---|
| `\l` | A lowercase character |
| `\u` | An uppercase character |
| `\L` | A character that is not lowercase |
| `\U` | A character that is not uppercase |
| `\t` | A tab |
| `\n` | A newline/line-feed character |
| `\r` | A carriage-return character |
| `\r\n` | A Windows-style line ending, carriage return followed by line feed |
| `\xnn` | A character identified by a two-digit hexadecimal code point, such as `\x41` for `A` |
| `\x{nnnn}` | A character identified by a braced hexadecimal code point, such as `\x{732B}` for `猫` |

Case-insensitive matching also affects case classes. Use `(?-i)` when
distinguishing uppercase from lowercase is the purpose of the pattern:

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `aB1` | `(?-i)\l` | `#` | `#B1` |
| `aB1` | `(?-i)\u` | `#` | `a#1` |
| `aB1` | `(?-i)\L` | `#` | `a##` |
| `aB1` | `(?-i)\U` | `#` | `#B#` |
| `A` | `\x41` | `#` | `#` |
| `猫` | `\x{732B}` | `#` | `#` |

In a replacement, `\u` and `\l` instead change the next character's case,
and `\U` / `\L` change a run of output until `\E`. See
[The replacement](#6-the-replacement) for that separate language.

Inside `[…]`, most special characters lose their meaning: `[.?]` matches a
dot or a question mark. To include `]`, `\` or `^` (at the start), escape
them: `[\]]`. A `-` is literal at the start or end of the class: `[_-]`.

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `paper_boats` | `_` | <code> </code> | `paper boats` |
| `06_the_keeper's_song` | `[_.]` | <code> </code> | `06 the keeper's song` |
| `Track 07` | `\d` | `#` | `Track ##` |
| `Salt & Cedar!` | `[^\w\s]` | *(empty)* | `Salt  Cedar` |

## 3. Repetition: how many times

A quantifier after a character or class says how often it may repeat.

| Quantifier | Means |
|---|---|
| `*` | 0 or more times |
| `+` | 1 or more times |
| `?` | 0 or 1 time (optional) |
| `{3}` | exactly 3 times |
| `{2,}` | 2 or more times |
| `{1,3}` | between 1 and 3 times |

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Track 07` | `\d+` | `#` | `Track #` |
| `Low··Tide···` (`·` is a space) | `\s{2,}` | <code> </code> | <code>Low Tide </code> |
| `colour, color` | `colou?r` | `hue` | `hue, hue` |
| `03 - Paper Boats` | <code>&#92;d+ - </code> | *(empty)* | `Paper Boats` |

### Greedy and lazy

Quantifiers are **greedy**: they take as much as they can and still let the
rest of the pattern match. Add `?` to make one **lazy**, taking as little as
possible:

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Juniper Vale - Salt - Cedar` | `^(.*) - .*$` | `$1` | `Juniper Vale - Salt` |
| `Juniper Vale - Salt - Cedar` | `^(.*?) - .*$` | `$1` | `Juniper Vale` |
| `(a) and (b)` | `\(.*\)` | `X` | `X` |
| `(a) and (b)` | `\(.*?\)` | `X` | `X and X` |

The `(…)` and `$1` in the first two rows are explained in section 5.

## 4. Anchors: where the match may be

Anchors match a *position*, not a character.

| Anchor | Position |
|---|---|
| `^` | Start of the text (and of every line in it) |
| `$` | End of the text (and of every line in it) |
| `\A`, `\z` | Start and end of the whole text only |
| `\b` | A word boundary: between a word character and a non-word character |
| `\B` | Not a word boundary |

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `07 Fog Horn 07` | <code>^&#92;d+ </code> | *(empty)* | `Fog Horn 07` |
| `07 Fog Horn 07` | <code> &#92;d+$</code> | *(empty)* | `07 Fog Horn` |
| `the other theme` | `\bthe\b` | `a` | `a other theme` |
| `··Low Tide·` | `^\s+|\s+$` | *(empty)* | `Low Tide` |

The last row uses `|`, which section 5 introduces: "spaces at the start, or
spaces at the end". (The table shows it as `|`; you type a plain `|`.)

## 5. Groups, alternatives and capturing

Parentheses do three jobs:

1. **Grouping**, so a quantifier applies to several characters: `(ab)+`.
2. **Alternatives** with `|`: `(Live|Demo|Remix)` matches any one of those words.
3. **Capturing**: the text a group matched is remembered and numbered from
   the left, starting at 1, so the replacement can reuse it.

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Vale, Juniper` | `^(\w+), (\w+)$` | `$2 $1` | `Juniper Vale` |
| `The Lanterns` | `^The (.+)$` | `$1, The` | `Lanterns, The` |
| `North Pier (Live)` | <code> &#92;((Live&#124;Demo)&#92;)$</code> | <code> [$1]</code> | `North Pier [Live]` |
| `2024-05-01` | `^(\d{4}).*` | `$1` | `2024` |

Use `(?:…)` for a group that only groups and doesn't capture. Its contents
don't get a number, which keeps your `$1`, `$2` easier to count:

```text
$regexp(%title%,'^(?:\d+[ ._-]+)?(.+)$','$1')
```

removes an optional leading track number: `03_Paper Boats` and `Paper Boats`
both become `Paper Boats`.

Inside the pattern itself, `\1` means "the same text group 1 matched".
That finds repeated words:

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Low Low Tide` | `\b(\w+) \1\b` | `$1` | `Low Tide` |

## 6. The replacement

The replacement is text with a few codes in it:

| Code | Inserts |
|---|---|
| `$1` … `$9`, `${12}` | The text of that group. Use braces when a digit follows: `${1}0` |
| `\1` … `\9` | The same as `$1` … `$9` |
| `$&` or `$0` | The whole match |
| `` $` `` | The text before the match |
| `$'` | The text after the match |
| `$$` | A literal `$` |
| `\U` … `\E` | Upper-case everything up to `\E` (or to the end) |
| `\L` … `\E` | Lower-case everything up to `\E` |
| `\u`, `\l` | Upper- or lower-case only the next character |
| `\t`, `\n` | A tab, a line break |

With the text `Juniper Vale` and the pattern `(\w+) (\w+)`:

| Replacement | Result |
|---|---|
| `$2, $1` | `Vale, Juniper` |
| `\2, \1` | `Vale, Juniper` |
| `[$&]` | `[Juniper Vale]` |
| `\U$1\E $2` | `JUNIPER Vale` |
| `\L$0` | `juniper vale` |
| `${1}_$2` | `Juniper_Vale` |
| `$$5` | `$5` |

Case codes can capitalise words, a job a regex alone can't do. Section 7
refines this example:

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `paper boats` | `\b(\w)` | `\u$1` | `Paper Boats` |
| `the keeper's song` | `\b(\w)` | `\u$1` | `The Keeper'S Song` |

## 7. Lookaround: conditions without consuming

A lookaround checks what comes before or after a position without making it
part of the match. That means it isn't replaced.

| Syntax | Means |
|---|---|
| `X(?=Y)` | `X`, if followed by `Y` |
| `X(?!Y)` | `X`, if **not** followed by `Y` |
| `(?<=Y)X` | `X`, if preceded by `Y` |
| `(?<!Y)X` | `X`, if **not** preceded by `Y` |

| Text | Pattern | Replacement | Result |
|---|---|---|---|
| `Track 12 of 15` | `\d+(?= of)` | `[$&]` | `Track [12] of 15` |
| `Track 12 of 15` | `(?<=of )\d+` | `[$&]` | `Track 12 of [15]` |
| `the keeper's song` | `(?<![\w'])(\w)` | `\u$1` | `The Keeper's Song` |
| `photo.final.jpg` | `\.(?=.*\.)` | `_` | `photo_final.jpg` |

The third row fixes the problem from section 6. A letter only starts a word
if neither a word character nor an apostrophe comes before it. The fourth
row replaces every dot that has another dot somewhere after it, so only the
last one (before the extension) survives.

A lookbehind must have a fixed length: `(?<=of )` works, `(?<=of\s+)` doesn't.

## 8. Case and other modifiers

Modifiers at the start of a pattern, or around part of it, change how it
matches:

| Modifier | Effect |
|---|---|
| `(?i)` | Ignore case from here on |
| `(?-i)` | Respect case from here on |
| `(?i:…)` | Ignore case inside the group only |
| `(?-m)` | `^` and `$` match only at the start and end of the whole text, not at line breaks |
| `(?-s)` | `.` no longer matches line breaks |
| `(?x)` | Ignore spaces in the pattern and allow `#` comments (for long patterns) |

Modifiers win over the setting of the place you use the pattern:

| Format string | Result |
|---|---|
| `$regexp('Live At Home','live',x)` | `x At Home` |
| `$regexp('Live At Home','live',x,0)` | `Live At Home` |
| `$regexp('Live At Home','(?-i)live',x)` | `Live At Home` |
| `$regexp('Live At Home','(?i)LIVE',x,0)` | `x At Home` |
| `$regexp('Live at home','(?-i)(?i:AT) home',x)` | `Live x` |

<!-- verified: crates/tag-core/tests/script.rs regexp_ignores_case_by_default_and_honours_the_flag_and_inline_modifiers -->

## `$regexp(x,expr,repl[,c])` { #regexp }

Replaces every match of a regular expression in a text.

| Argument | Meaning |
|---|---|
| `x` | The text to search, usually a placeholder such as `%title%` |
| `expr` | The regular expression |
| `repl` | The replacement, with the codes from [section 6](#6-the-replacement) |
| `c` | Optional. `0` respects case; leave it out (or use any other value) to ignore case |

**Returns** `x` with every match replaced, or `x` unchanged when nothing matches.

| Format string | With these values | Result |
|---|---|---|
| `$regexp(%title%,'\s*\(feat\. [^)]+\)',)` | `TITLE` = `Paper Boats (feat. Mira Solberg)` | `Paper Boats` |
| `$regexp(%title%,'^.*\(feat\. ([^)]+)\).*$','$1')` | same | `Mira Solberg` |
| `$regexp(%year%,'^(\d{4}).*$','$1')` | `YEAR` = `2024-05-01` | `2024` |
| `$regexp(%artist%,'^The (.+)$','$1, The')` | `ARTIST` = `The Lanterns` | `Lanterns, The` |
| `$regexp(%title%,'abc',x)` | `TITLE` = `ABC abc` | `x x` |
| `$regexp(%title%,'abc',x,0)` | `TITLE` = `ABC abc` | `ABC x` |

### Quote the pattern and the replacement

Inside a format string, `$`, `%`, `[`, `]`, `'`, `,` and `)` have their own
meaning (see [Special characters](format-strings.md#special-characters)), and
regular expressions are full of them. **Put the pattern and the replacement
in single quotes** to keep their syntax literal:

```text
$regexp(%title%,'^(\d+)[ ._-]+(.+)$','$2 [$1]')
```

Balanced unquoted groups can also work: argument parentheses nest, and
dollar captures such as `$2` remain literal. Quotes still protect other
format syntax, including optional-section brackets and argument commas.
Inside quotes, write an apostrophe as `''`:

```text
$regexp(%title%,'(?<![\w''])(\w)','\u$1')
```

To remove matches, leave the replacement argument empty after its comma:

```text
$regexp(%title%,'\s*\(feat\. [^)]+\)',)
```

Do not use `''` as an empty replacement: it means one literal apostrophe.
Backslashes need no doubling: format strings don't treat `\` specially.

!!! warning "Gotcha: no match means no change"
    `$regexp(%title%,'^.*\(feat\. ([^)]+)\).*$','$1')` returns the
    *whole title* for a song without “feat.”, because nothing matched. If you
    store the result in `ARTIST`, songs without a guest get their title as
    artist. Restrict the files first (filter
    `title MATCHES "\(feat\."`), or guard the expression:

    ```text
    $if($grtr($strstr($lower(%title%),'(feat. '),0),$regexp(%title%,'^.*\(feat\. ([^)]+)\).*$','$1'),%artist%)
    ```

    The search checks a lowercase copy so `FEAT.` follows the same
    case-insensitive rule as the regex. The extraction still reads the
    original title, preserving the guest's spelling.

!!! warning "Gotcha: only the first value of a multi-value field"
    `%artist%` gives only the first artist, so `$regexp(%artist%,…)` only sees
    that one. To transform every value, use the
    [Replace with regular expression](#the-replace-with-regular-expression-action)
    action on the field.

## The Replace with regular expression action { #the-replace-with-regular-expression-action }

The action rewrites a field in place. Its
settings:

| Setting | Meaning |
|---|---|
| **Field** | The field to change. `_ALL` changes every field, `_FILENAME` the file name, `_DIRECTORY` the folder name |
| **Regular expression** | The pattern |
| **Replace matches with** | The replacement, with the codes from [section 6](#6-the-replacement) |
| **Case-sensitive comparison** | Off: case is ignored (the default). On: case must match |

Differences from `$regexp`:

- **No quoting needed.** The pattern and the replacement are plain text, not
  format strings. Type `$1` as is, and `%` is just a percent sign.
- **Every value of a multi-value field** is rewritten separately.
- **On `_FILENAME` the extension is part of the text.** A pattern anchored
  with `$` meets the end of `.mp3`, not the end of the name. To trim trailing
  spaces from a name, look ahead to the extension:

    | Field | Regular expression | Replace matches with | `Low Tide  .mp3` becomes |
    |---|---|---|---|
    | `_FILENAME` | `\s+$` | *(empty)* | `Low Tide  .mp3` (no change) |
    | `_FILENAME` | `\s+(?=\.\w+$)` | *(empty)* | `Low Tide.mp3` |

Like every action, it can be undone with ++cmd+z++ / ++ctrl+z++ after it has
written the files.

## In the filter and Find { #in-the-filter-and-find }

In the filter, `MATCHES` tests a field against a regular
expression. The match **always ignores case**, so add `(?-i)` when case
matters. Put the pattern in double quotes when it contains spaces or
parentheses:

| Filter | Shows files where |
|---|---|
| `title MATCHES "\(live\)"` | the title contains `(live)` |
| `track MATCHES ^\d+$` | the track has no total (`7`, not `7/12`) |
| `title MATCHES "(?-i)^[^a-z]+$"` | the title has no lower-case letters (ALL CAPS) |
| `* MATCHES "\s{2,}"` | any field contains a double space |
| `artist MATCHES "^the "` | the artist starts with "The" |

### Find uses the shared Boost engine { #find-dialect }

In **Edit → Find…** (++cmd+f++ / ++ctrl+f++), choose **Regular expression**
to search filenames and individual tag values. Type the raw pattern without
format-string apostrophes. Use **Case sensitive** to control letter case;
inline options such as `(?i)` override that setting in the same way as in
other regex contexts. The shared multiline/dot-all rules also apply.

<!-- find-regex-examples:start -->
| Pattern | Case sensitive | Value | Result |
| --- | --- | --- | --- |
| `boats` | Off | `Paper Boats` | Match |
| `boats` | On | `Paper Boats` | No match |
| `(?i)boats` | On | `Paper Boats` | Match |
| `\APaper Boats\z` | Off | `Paper Boats` | Match |
| `\APaper Boats\z` | Off | `Paper Boats Live` | No match |
| `(` | Off | `Paper Boats` | Invalid pattern |
<!-- find-regex-examples:end -->

Find reports an invalid expression instead of treating it as no matches.
A failed search clears its table, and editing the criteria, strategy or case
setting clears previous results until **Find** runs again. Search work runs
in the native backend; an earlier search cannot replace results from a newer
query. In a browser-only preview without that backend, regex Find is unavailable.

For one search, the native backend accepts up to 100,000 values, 1 MiB per
value and 64 MiB of text overall; patterns can use up to 64 KiB. A limit error
asks for a smaller File List or simpler pattern. It returns no partial results.

Find does not directly rewrite tags. Selecting a result changes the main
selection, where configured tag-panel defaults or save-on-move settings can
still act. Review the selection before applying an action.

<!-- verified: src-tauri/src/find.rs tests execute this table through the native Boost matcher; tests/guide-find-regex.test.mjs checks batching, errors and stale UI requests. -->

## A pattern library

Copy, paste, adapt. Each row is ready to use in `$regexp` (with the quotes
shown) or, without the quotes, in the action. **Leave empty** means no text
after the last comma in `$regexp`, or an empty replacement box in the action.

| Goal | Pattern | Replacement | Before → After |
|---|---|---|---|
| Remove a leading track number | `'^\d+[\s._-]*'` | **Leave empty** | `03_Paper Boats` → `Paper Boats` |
| Underscores to spaces | `'_'` | `' '` | `the_keeper's_song` → `the keeper's song` |
| Collapse repeated spaces | `'\s{2,}'` | `' '` | `Low··Tide` → `Low Tide` |
| Trim spaces at both ends | `'^\s+|\s+$'` | **Leave empty** | `·Low Tide·` → `Low Tide` |
| Remove a "(Remastered …)" suffix | `'\s*\((Remaster(ed)?|Live)[^)]*\)$'` | **Leave empty** | `North Pier (Remastered 2021)` → `North Pier` |
| Remove anything in square brackets | `'\s*\[[^\]]*\]'` | **Leave empty** | `Low Tide [Bonus]` → `Low Tide` |
| Remove "feat." from a title | `'\s*\(feat\. [^)]+\)'` | **Leave empty** | `Paper Boats (feat. Mira Solberg)` → `Paper Boats` |
| Swap "Last, First" | `'^([^,]+), (.+)$'` | `'$2 $1'` | `Vale, Juniper` → `Juniper Vale` |
| Move "The" to the end | `'^The (.+)$'` | `'$1, The'` | `The Lanterns` → `Lanterns, The` |
| Keep the year of a date | `'^(\d{4}).*$'` | `'$1'` | `2024-05-01` → `2024` |
| "&" to "and" | `'\s*&\s*'` | `' and '` | `Salt & Cedar` → `Salt and Cedar` |
| Capitalise each word | `'(?<![\w''])(\w)'` | `'\u$1'` | `the keeper's song` → `The Keeper's Song` |
| Upper-case a whole word | `'\b(dj)\b'` | `'\U$1'` | `dj nimbus` → `DJ nimbus` |

(`·` stands for a space, and `|` for a plain `|`.)

## How TagGroove's regular expressions differ { #how-taggroove-differs }

The shared core used by format strings, actions, filters, Find and web-source
parser commands accepts Boost’s Perl-compatible syntax. If you come from
another tool, watch for these:

- **Case is ignored by default** in `$regexp`, the action and the filter.
  JavaScript, Python and most editors respect case by default.
- **Replacements are global.** Every match is replaced, not just the first.
- **`\d` means only `0`–`9`.** Full-width and other non-ASCII digits don't
  count as digits.
- **`^` and `$` also match at line breaks**, and `.` matches line breaks.
  This only matters for multi-line values such as comments and lyrics. Use
  `\A` and `\z`, or `(?-m)` and `(?-s)`, for the stricter behaviour.
- **`\u` and `\l` in a pattern are classes** ("an upper-case letter", "a
  lower-case letter"), not Unicode escapes. Write a code point as `\x{732B}`.
  With case ignored, `\u` matches any letter. In a *replacement*, `\u` and
  `\l` change the case of the next character.
- **An empty pattern matches between every character:**
  `$regexp(ab,,_)` gives `_a_b_`. The pattern argument is empty; `''`
  would search for an apostrophe instead.
- **Characters outside the Basic Multilingual Plane**, such as emoji, count
  as two characters for `.` and `{n}`.
- Case-insensitive matching of non-English letters follows your system's
  language settings.
- Boost's extended conditional replacement syntax is not enabled. Use
  format-string conditions around `$regexp` when the output needs a decision.

## Related

- [Format strings](format-strings.md)
- [Move guest artists to titles](../cookbook/feat-artists-to-title.md)
