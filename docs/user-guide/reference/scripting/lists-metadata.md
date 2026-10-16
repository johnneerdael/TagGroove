# Lists, metadata and environment

Use these functions to read multiple field values, inspect the file-list
columns, transform delimited lists and access application context.

!!! tip "New to this?"
    A bare `%artist%` reads the first artist value. Follow
    [Format strings from zero](../../learn/format-strings-from-zero.md), then
    use `$meta` or `$meta_sep` when the remaining values matter.

Unless a row says otherwise, examples on this page use only these tag
fields: `ARTIST` has two values, `Juniper Vale` and `Mira Solberg`, and
`TITLE` has one value, `Paper Boats`. The example file-list columns are
**Title** = `Paper Boats`, then **Artist** = `Juniper Vale`.

### `$meta(field[,index])` { #meta }

**Returns** all values of a field joined with comma-space when `index` is
omitted, or one value at the zero-based index when it is supplied.

| Argument | Meaning |
|---|---|
| `field` | Field name such as `artist`, without percent signs |
| `index` | Optional zero-based value index: 0 is first, 1 is second |

| Format string | With these values | Result |
|---|---|---|
| `$meta(artist)` | Two artist values from the example | `Juniper Vale, Mira Solberg` |
| `$meta(artist,9)` | Two artist values from the example | *(empty)* |
| `$upper($meta(artist,1))` | Two artist values from the example | `MIRA SOLBERG` |
| `$meta(artist,-1)` | Two artist values from the example | `Juniper Vale` |

A missing field or out-of-range index returns empty text. Use
`$meta(artist)`, not `$meta(%artist%)`: the latter treats the artist's
value as a field name. The index uses a nonnegative leading-integer count;
negative or nonnumeric input becomes 0, as the last example shows.
**Related:** [`$meta_sep`](#meta_sep).

### `$meta_sep(field,separator)` { #meta_sep }

**Returns** all values of a field joined with the chosen separator.

| Argument | Meaning |
|---|---|
| `field` | Field name without percent signs |
| `separator` | Literal text to put between values; may be empty |

| Format string | With these values | Result |
|---|---|---|
| `$meta_sep(artist,' & ')` | Two artist values from the example | `Juniper Vale & Mira Solberg` |
| `$meta_sep(missing,'; ')` | `MISSING` absent | *(empty)* |
| `$upper($meta_sep(artist,/))` | Two artist values from the example | `JUNIPER VALE/MIRA SOLBERG` |

Joining produces one string; it does not create separate destination
values. Use a field-splitting action for that structural change.
**Related:** [`$meta`](#meta).

### `$list(prefix,separator,suffix[,value_separator])` { #list }

**Returns** a formatted entry for each raw tag field, joining the entries
without an additional delimiter.

| Argument | Meaning |
|---|---|
| `prefix` | Text before each field name |
| `separator` | Text between the name and its values |
| `suffix` | Text after each field's values |
| `value_separator` | Text between that field's values; defaults to a comma |

| Format string | With these values | Result |
|---|---|---|
| `$list('<',=,'>')` | Only example `ARTIST` and `TITLE` fields | `<ARTIST=Juniper Vale,Mira Solberg><TITLE=Paper Boats>` |
| `$list('<',=,'>',';')` | Only example `ARTIST` and `TITLE` fields | `<ARTIST=Juniper Vale;Mira Solberg><TITLE=Paper Boats>` |
| `$upper($list(,=,';',' & '))` | Only example `ARTIST` and `TITLE` fields | `ARTIST=JUNIPER VALE & MIRA SOLBERG;TITLE=PAPER BOATS;` |

The field names and values come from the raw tag-field map, not visible
column names or all information placeholders. The examples show the map's
field-name order. Quoting the `<` and `>` here makes the intended literal
text explicit; if the result is used in HTML, escape its data for that
output format rather than assuming this function performs HTML escaping.
**Related:** [`$columnnames`](#columnnames), [`$meta_sep`](#meta_sep).

### `$sort(x,delimiter)` { #sort }

**Returns** the segments of `x` sorted lexicographically and rejoined with
the same literal delimiter.

| Argument | Meaning |
|---|---|
| `x` | Text containing segments |
| `delimiter` | Nonempty literal separator; may contain several characters |

| Format string | With these values | Result |
|---|---|---|
| `$sort(b;a;c,;)` | Any file | `a;b;c` |
| `$sort(10;2,;)` | Any file | `10;2` |
| `$upper($sort(b;a,;))` | Any file | `A;B` |

Sorting is case-sensitive, does not trim segments, and is not a numeric
or locale-aware natural sort. Empty segments participate too. An empty
delimiter reports an error. **Related:** [`$dedup`](#dedup).

### `$dedup(x,delimiter[,ignore_case])` { #dedup }

**Returns** the first occurrence of each distinct segment, preserving the
original order and spelling.

| Argument | Meaning |
|---|---|
| `x` | Text containing segments |
| `delimiter` | Nonempty literal separator |
| `ignore_case` | Optional literal `1` ignores case; omitted/other text compares exactly |

| Format string | With these values | Result |
|---|---|---|
| `$dedup(a;b;a,;)` | Any file | `a;b` |
| `$dedup(A;a;B;A,;,1)` | Any file | `A;B` |
| `$upper($dedup(a;b;a,;))` | Any file | `A;B` |

Segments are not trimmed. `A` and ` A` differ even with case ignored.
The optional setting compares its text to `1`; it does not use a typed
condition flag. Empty delimiters are rejected.
**Related:** [`$sort`](#sort).

### `$columnnames(prefix,suffix)` { #columnnames }

**Returns** each file-list column's name wrapped in the prefix and suffix,
in configured column order.

| Argument | Meaning |
|---|---|
| `prefix` | Text before every name |
| `suffix` | Text after every name; also useful as a separator |

| Format string | With these values | Result |
|---|---|---|
| `$columnnames('[',']')` | Columns Title, Artist | `[Title][Artist]` |
| `$columnnames(,';')` | No columns in supplied context | *(empty)* |
| `$upper($columnnames(,';'))` | Columns Title, Artist | `TITLE;ARTIST;` |

This function requires the application's column context. An empty column
list returns empty text; unavailable context reports an error. A trailing
separator in the last example comes from the suffix after the final column.
The **Value** expression of a file-list column does not itself receive
that column context, so this function cannot be used there to list its peers.
**Related:** [`$columnvalues`](#columnvalues).

### `$columnvalues(prefix,suffix)` { #columnvalues }

**Returns** the current file's rendered values for the file-list columns,
wrapped individually and kept in configured column order.

| Argument | Meaning |
|---|---|
| `prefix` | Text before every rendered value |
| `suffix` | Text after every rendered value |

| Format string | With these values | Result |
|---|---|---|
| `$columnvalues('[',']')` | Example Title and Artist columns | `[Paper Boats][Juniper Vale]` |
| `$columnvalues(,';')` | No columns in supplied context | *(empty)* |
| `$lower($columnvalues(,';'))` | Example Title and Artist columns | `paper boats;juniper vale;` |

These are display results from column definitions, which can differ from
raw tag values. Missing column context reports an error. Do not assume a
standalone expression evaluator has an application's rendered columns.
The **Value** expression inside a column is evaluated without that context;
it cannot call this function to read the other columns' rendered results.
**Related:** [`$columnnames`](#columnnames), [`$list`](#list).

### `$folderdepth(path)` { #folderdepth }

**Returns** the number of folder components, excluding a drive or UNC
server/share prefix and excluding the last component when it looks like
a file path without a trailing separator.

| Argument | Meaning |
|---|---|
| `path` | Path text using slash or backslash separators |

| Format string | With these values | Result |
|---|---|---|
| `$folderdepth('C:\Music\Album\03.mp3')` | Any file | `2` |
| `$folderdepth('\\server\share\Album\03.mp3')` | Any file | `1` |
| `$add($folderdepth('C:\Music\Album\03.mp3'),1)` | Any file | `3` |
| `$folderdepth('Music\Album\')` | Any file | `2` |

The function inspects text; it does not check the filesystem. A folder
path without its trailing separator can have its last component treated
as a filename. **Related:** [Path placeholders](../placeholders.md).

### `$getenv(name)` { #getenv }

**Returns** the named environment value. A missing ordinary variable gives
empty text. The two compatibility names below read application context.

| Argument | Meaning |
|---|---|
| `name` | Environment-variable name, or `MP3TAGAPP` / `MP3TAGAPPDATA` |

These examples supply `GUIDE_READER=listener`, application directory
`/opt/OpenTag`, and configuration directory `/home/listener/.config/opentag`:

| Format string | With these values | Result |
|---|---|---|
| `$getenv(GUIDE_READER)` | Supplied example environment | `listener` |
| `$getenv(GUIDE_MISSING_VARIABLE)` | Variable absent | *(empty)* |
| `$upper($getenv(GUIDE_READER))` | Supplied example environment | `LISTENER` |
| `$getenv(MP3TAGAPP)` | Supplied application directory | `/opt/OpenTag` |
| `$getenv(MP3TAGAPPDATA)` | Supplied configuration directory | `/home/listener/.config/opentag` |

`MP3TAGAPP` and `MP3TAGAPPDATA` are compatibility identifiers in OpenTag;
their names do not refer to another running program. They require the
corresponding application context or report an error. Ordinary variable
names follow the host environment's naming rules.
**Related:** [`$folderdepth`](#folderdepth).

### `$uuid()` { #uuid }

**Returns** a newly generated version-4 UUID in hyphenated lowercase form.
There are no arguments; retain the empty parentheses.

| Format string | With these values | Result |
|---|---|---|
| `$uuid()` | Any file | A UUID in 8-4-4-4-12 hexadecimal form |
| `$len($uuid())` | Any file | `36` |
| `$upper($uuid())` | Any file | A UUID in uppercase 8-4-4-4-12 form |

Each call generates a value. Re-evaluating the expression can therefore
change it. Store a generated value once when later operations must reuse
that same identifier; do not assume two `$uuid()` calls agree.
**Related:** [`$rand`](numbers.md#rand).

<!-- verified: crates/tag-core/src/script.rs:function, folder_depth; column and environment examples require the stated supplied contexts -->

## Related

- [Placeholders](../placeholders.md)
- [Text and case](text.md)
- [Format strings](../format-strings.md)
- [Scripting functions](index.md)
