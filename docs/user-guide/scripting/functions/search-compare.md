# Search and compare

Use these functions to locate text, compare values or check a naming rule.
Search positions start at **1**; 0 means no match. Boolean comparisons
produce empty text and replace the current availability and truth flags with
their answer. Wrap a predicate in `$if(predicate,1,0)` to display its answer.

!!! tip "New to this?"
    Work through [Format strings from zero](../format-strings-from-zero.md)
    before building a nested search condition.

Examples use `TITLE` = `Paper Boats`. Searches are case-sensitive and count
Unicode characters rather than UTF-8 bytes. A search result inherits its
input's truth flag; test the returned position with `$grtr(position,0)`
instead of passing the position directly to `$if`.

### `$strstr(x,y)` { #strstr }

**Returns** the first position of the whole substring `y` in `x`, or 0.

| Argument | Meaning |
|---|---|
| `x` | Text to search |
| `y` | Exact substring to find |

| Format string | With these values | Result |
|---|---|---|
| `$strstr(%title%,Boats)` | `TITLE` = `Paper Boats` | `7` |
| `$strstr(%title%,boats)` | `TITLE` = `Paper Boats` | `0` |
| `$if($grtr($strstr(%title%,Boats),0),found,missing)` | `TITLE` = `Paper Boats` | `found` |

An empty search string returns 0. To search without case sensitivity,
normalize both inputs with `$lower` first. **Related:** [`$strrstr`](#strrstr).

### `$strrstr(x,y)` { #strrstr }

**Returns** the last starting position of substring `y` in `x`, or 0.

| Argument | Meaning |
|---|---|
| `x` | Text to search |
| `y` | Exact substring to locate from the right |

| Format string | With these values | Result |
|---|---|---|
| `$strrstr(la-la-la,la)` | Any file | `7` |
| `$strrstr(Paper Boats,xyz)` | Any file | `0` |
| `$if($grtr($strrstr(%title%,a),0),has a,no a)` | `TITLE` = `Paper Boats` | `has a` |

The returned number is still measured from the left. It is not a count
of characters from the end. **Related:** [`$strstr`](#strstr).

### `$strchr(x,c)` { #strchr }

**Returns** the first position of the first character in `c`, or 0.

| Argument | Meaning |
|---|---|
| `x` | Text to search |
| `c` | Character to find; only its first character is used |

| Format string | With these values | Result |
|---|---|---|
| `$strchr(%title%,a)` | `TITLE` = `Paper Boats` | `2` |
| `$strchr(%title%,Bo)` | `TITLE` = `Paper Boats` | `7` |
| `$if($grtr($strchr(%title%,z),0),found,missing)` | `TITLE` = `Paper Boats` | `missing` |

The second example searches for `B`, not for either `B` or `o`, and not
for the complete string `Bo`. An empty `c` returns 0.
**Related:** [`$strrchr`](#strrchr), [`$strstr`](#strstr).

### `$strrchr(x,c)` { #strrchr }

**Returns** the last position of the first character in `c`, or 0.

| Argument | Meaning |
|---|---|
| `x` | Text to search |
| `c` | Character whose last occurrence is wanted |

| Format string | With these values | Result |
|---|---|---|
| `$strrchr(%title%,a)` | `TITLE` = `Paper Boats` | `9` |
| `$strrchr(%title%,)` | `TITLE` = `Paper Boats` | `0` |
| `$if($grtr($strrchr(%title%,a),5),late,early)` | `TITLE` = `Paper Boats` | `late` |

Only the first character of `c` participates, even when more text is
supplied. **Related:** [`$strchr`](#strchr), [`$strrstr`](#strrstr).

### `$strcmp(x,y)` { #strcmp }

**Sets the flag** to true when both strings are exactly equal, otherwise
false. This is an equality test, not a sorting comparator.

| Argument | Meaning |
|---|---|
| `x`, `y` | Strings to compare, including their case and spaces |

| Format string | With these values | Result |
|---|---|---|
| `$if($strcmp(%title%,Paper Boats),1,0)` | `TITLE` = `Paper Boats` | `1` |
| `$if($strcmp(03,3),1,0)` | Any file | `0` |
| `$if($strcmp($lower(%title%),paper boats),same,different)` | `TITLE` = `Paper Boats` | `same` |

`03` and `3` differ as strings. Use `$eql` when integer equality is the
intended rule. **Related:** [`$stricmp`](#stricmp),
[conditions and logic](conditions.md).

### `$stricmp(x,y)` { #stricmp }

**Sets the flag** to true when the strings are equal after lowercase
conversion, otherwise false.

| Argument | Meaning |
|---|---|
| `x`, `y` | Strings to compare without letter-case differences |

| Format string | With these values | Result |
|---|---|---|
| `$if($stricmp(%title%,PAPER BOATS),1,0)` | `TITLE` = `Paper Boats` | `1` |
| `$if($stricmp('Paper Boats ','Paper Boats'),1,0)` | Any file | `0` |
| `$if($stricmp($trim(' Paper Boats '),PAPER BOATS),same,different)` | Any file | `same` |

Whitespace still matters. Lowercase conversion is not a general
language-aware equivalence or accent-removal rule.
**Related:** [`$strcmp`](#strcmp).

### `$distance(x,y)` { #distance }

**Returns** the Levenshtein edit distance: the fewest single-character
insertions, deletions or substitutions needed to turn `x` into `y`.

| Argument | Meaning |
|---|---|
| `x`, `y` | The two strings to compare |

| Format string | With these values | Result |
|---|---|---|
| `$distance(%title%,Paper Boat)` | `TITLE` = `Paper Boats` | `1` |
| `$distance(,abc)` | Any file | `3` |
| `$if($eql($distance(%title%,Paper Boats),0),exact,changed)` | `TITLE` = `Paper Boats` | `exact` |

Case differences count as changes. The result measures spelling
differences, not whether two titles refer to the same recording. Very
large comparisons are rejected by a work limit.
**Related:** [`$stricmp`](#stricmp).

### `$validate(x,repl)` { #validate }

**Returns** `x` with each of `/ ? * " < > | :` replaced by `repl`.
Use it to clean a component under that specific rule.

| Argument | Meaning |
|---|---|
| `x` | Text to clean |
| `repl` | Literal replacement for each forbidden character; may be empty |

| Format string | With these values | Result |
|---|---|---|
| `$validate('Paper: Boats?',_)` | Any file | `Paper_ Boats_` |
| `$validate('A/B',)` | Any file | `AB` |
| `$upper($validate('a:b',-))` | Any file | `A-B` |

This function does not create a path, remove all platform-specific invalid
names, or check whether a destination exists. Backslashes are not in this
function's replacement set. Filename evaluation has additional rules;
see [file and folder names](../format-strings.md#file-and-folder-names).

### `$isdigit(x)` { #isdigit }

**Sets the flag** to true when the first character is an ASCII digit `0`–`9`,
otherwise false. It does not validate the entire string as a number.

| Argument | Meaning |
|---|---|
| `x` | Text whose first character is checked |

| Format string | With these values | Result |
|---|---|---|
| `$if($isdigit(3/8),1,0)` | Any file | `1` |
| `$if($isdigit(-3),1,0)` | Any file | `0` |
| `$if($isdigit($trim(' 3/8')),numeric prefix,other)` | Any file | `numeric prefix` |

`12x` is true, while an empty string or one beginning with whitespace is
false. To require a complete numeric form, use a suitable anchored regular
expression. **Related:** [Regular expressions](../regular-expressions.md).

<!-- verified: crates/tag-core/src/script.rs:function; all positions count Unicode scalar values -->

## Related

- [Conditions and logic](conditions.md)
- [Text and case](text.md)
- [Regular expressions](../regular-expressions.md)
- [Scripting functions](index.md)
