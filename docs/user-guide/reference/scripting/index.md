# Scripting functions

Scripting functions calculate text inside a format string. They can read tags,
transform a value, compare inputs or choose an output. This index covers all
64 built-in functions in TagGroove's format-string engine, including both forms
of `$meta` through its optional index argument.

!!! tip "New to this?"
    Start with [Format strings from zero](../../learn/format-strings-from-zero.md)
    and [Regular expressions from zero](../../learn/regex-from-zero.md).
    The family pages below explain every argument and provide worked examples.

## Choose a family

| Goal | Reference |
|---|---|
| Change spelling, capitalization or character ranges | [Text and case](text.md) |
| Find a substring or compare two strings | [Search and compare](search-compare.md) |
| Select an output or combine conditions | [Conditions and logic](conditions.md) |
| Calculate integers or pad a number | [Numbers and maths](numbers.md) |
| Display a timestamp or elapsed time | [Dates and durations](dates-durations.md) |
| Read multiple values, columns or application context | [Lists, metadata and environment](lists-metadata.md) |
| Match varying text with a pattern | [Regular expressions](../regular-expressions.md#regexp) |

## Read a function signature

`$name(x,y)` means two arguments separated by a comma. A signature such as
`$mid(x,start[,n])` has an optional final argument; the square brackets in
this reference notation are not characters to type. Repeated arguments are
shown with an ellipsis, for example `$add(x,y[,z…])`.

Function names ignore case, so `$cutLeft` and `$cutleft` are equivalent.
Argument text can still be case-sensitive: the `MM` and `mm` date tokens,
for example, mean different things.

Ordinary spaces in arguments are text. Quote a pattern or replacement when
its commas, parentheses, brackets or dollar signs would otherwise become
format-string syntax. An empty argument is written by leaving that position
empty. `''` supplies an apostrophe; it is not an empty string.

## Alphabetical index

| Function | Use it to |
|---|---|
| [`$add`](numbers.md#add) | Add integers |
| [`$and`](conditions.md#and) | Require every condition flag |
| [`$ansi`](text.md#ansi) | Represent text in the system ANSI encoding |
| [`$caps`](text.md#caps) | Capitalize words and lowercase their remaining letters |
| [`$caps2`](text.md#caps2) | Capitalize words while preserving their remaining letters |
| [`$caps3`](text.md#caps3) | Capitalize the first character and characters after explicit triggers |
| [`$char`](text.md#char) | Create a Unicode character from its code point |
| [`$columnnames`](lists-metadata.md#columnnames) | Format the configured file-list column names |
| [`$columnvalues`](lists-metadata.md#columnvalues) | Format the current file's rendered column values |
| [`$cutleft`](text.md#cutleft) | Remove characters from the beginning |
| [`$cutright`](text.md#cutright) | Remove characters from the end |
| [`$dedup`](lists-metadata.md#dedup) | Keep the first distinct delimited segments |
| [`$distance`](search-compare.md#distance) | Count insertions, deletions and substitutions |
| [`$div`](numbers.md#div) | Divide integers in order |
| [`$eql`](conditions.md#eql) | Compare numeric or case-insensitive string equality |
| [`$fmtdate`](dates-durations.md#fmtdate) | Format a calendar timestamp |
| [`$fmtduration`](dates-durations.md#fmtduration) | Format elapsed minutes, seconds and milliseconds |
| [`$fmtnum`](numbers.md#fmtnum) | Format a grouped integer for the numeric locale |
| [`$folderdepth`](lists-metadata.md#folderdepth) | Count folder components in path text |
| [`$geql`](conditions.md#geql) | Test greater than or equal |
| [`$getenv`](lists-metadata.md#getenv) | Read environment or application directory context |
| [`$grtr`](conditions.md#grtr) | Test greater than |
| [`$if`](conditions.md#if) | Choose from two results using a flag |
| [`$if2`](conditions.md#if2) | Choose a field or expression fallback using its flag |
| [`$ifgreater`](conditions.md#ifgreater) | Choose based on integer magnitude |
| [`$iflonger`](conditions.md#iflonger) | Choose based on character count |
| [`$isdigit`](search-compare.md#isdigit) | Test whether the first character is an ASCII digit |
| [`$left`](text.md#left) | Read the first characters |
| [`$len`](text.md#len) | Count characters |
| [`$leql`](conditions.md#leql) | Test less than or equal |
| [`$less`](conditions.md#less) | Test less than |
| [`$list`](lists-metadata.md#list) | Format raw tag-field names and values |
| [`$lower`](text.md#lower) | Lowercase text |
| [`$meta`](lists-metadata.md#meta) | Read all field values or one indexed value |
| [`$meta_sep`](lists-metadata.md#meta_sep) | Join field values with a chosen separator |
| [`$mid`](text.md#mid) | Read characters from a position |
| [`$mod`](numbers.md#mod) | Calculate integer remainders |
| [`$mul`](numbers.md#mul) | Multiply integers |
| [`$neql`](conditions.md#neql) | Test inequality |
| [`$not`](conditions.md#not) | Invert a flag |
| [`$num`](numbers.md#num) | Zero-pad a leading integer |
| [`$odd`](conditions.md#odd) | Test integer oddness |
| [`$or`](conditions.md#or) | Accept any true condition flag |
| [`$ord`](text.md#ord) | Read the first character's code point up to U+FFFF |
| [`$rand`](numbers.md#rand) | Generate an integer from 0 through 32767 |
| [`$regexp`](../regular-expressions.md#regexp) | Replace pattern matches and reuse captures |
| [`$repeat`](text.md#repeat) | Repeat text |
| [`$replace`](text.md#replace) | Apply ordered literal replacements |
| [`$reverse`](text.md#reverse) | Reverse character order |
| [`$right`](text.md#right) | Read the final characters |
| [`$sort`](lists-metadata.md#sort) | Sort literal-delimited segments |
| [`$strchr`](search-compare.md#strchr) | Find the first occurrence of a character |
| [`$strcmp`](search-compare.md#strcmp) | Test exact string equality |
| [`$stricmp`](search-compare.md#stricmp) | Test string equality after lowercasing |
| [`$strrchr`](search-compare.md#strrchr) | Find the final occurrence of a character |
| [`$strrstr`](search-compare.md#strrstr) | Find the final occurrence of a substring |
| [`$strstr`](search-compare.md#strstr) | Find the first occurrence of a substring |
| [`$sub`](numbers.md#sub) | Subtract integers in order |
| [`$trim`](text.md#trim) | Trim both ends |
| [`$trimleft`](text.md#trimleft) | Trim the beginning |
| [`$trimright`](text.md#trimright) | Trim the end |
| [`$upper`](text.md#upper) | Uppercase text |
| [`$uuid`](lists-metadata.md#uuid) | Generate a version-4 UUID |
| [`$validate`](search-compare.md#validate) | Replace a defined set of forbidden filename characters |

!!! note "Bringing formulas from Mp3tag"
    This page describes current TagGroove behavior. Known compatibility gaps
    remain in nested conditions, optional sections and predicate output.
    TagGroove currently displays `1`/`0` for predicates and combines flags
    differently from the inspected Mp3tag evaluator. Check imported formulas
    in Preview before applying them to tags or filenames.

## Distinguish visible text, field availability and truth

A function can return the text `0` while retaining a true flag inherited
from a present source field. Conversely, literal `1` has no true flag.
Read [the condition rules](conditions.md#understand-what-counts-as-true)
when using `$if`, `$if2`, `$and`, `$or` or `$not`. Comparisons such as `$grtr`
produce explicit boolean flags.

Optional sections `[…]` use field availability, which is a separate part
of evaluation. Removing all text from a present field does not necessarily
make its enclosing optional section behave like a missing field. Keep
presence tests close to field placeholders; use explicit comparisons for
questions about calculated text.

## Diagnose an expression error

| Symptom | Check |
|---|---|
| Unknown function | Check this index and the function name; export-template commands such as `$loop` use a separate context |
| Wrong argument count | Check the signature, commas and any unquoted comma inside the intended text |
| Unterminated literal or function | Balance single quotes and closing parentheses; build the expression from the inside out |
| Valid expression, unexpected output | Check missing fields, case rules, leading-integer conversion, and condition flags |
| Context required | Functions such as `$columnvalues` and `$getenv(MP3TAGAPP)` need the application's supplied context |
| Output/work limit | Reduce repetition, padding, comparison size or nesting rather than repeating the same oversized expression |

A valid expression that finds no regex match normally returns its input
unchanged. A missing field normally produces empty text. Those are results,
not syntax errors. In a field-writing converter, an empty output can remove
the destination field; preview before applying.

Each family page describes its own errors. Supplying too few or too many
arguments reports an error rather than guessing which argument was intended.
All function arguments are evaluated, including an unused conditional branch.

<!-- verified: crates/tag-core/src/script.rs:SUPPORTED_FUNCTIONS, validate_arity, run; index coverage is checked against the runtime registry -->

## Related

- [Format strings](../format-strings.md)
- [Placeholders](../placeholders.md)
- [Conditions and logic](conditions.md)
- [Regular expressions](../regular-expressions.md)
