# Web source script reference

A source has two languages: header values describe requests and parser sections
read the returned text. Format strings appear only in the places that accept
them, such as SearchBy defaults, SayFormat and UseFormat. A parser command is
not a `$function(...)` call.

Start with the [local JSON lesson](writing-web-sources.md) for a complete
source and [installation guide](web-sources.md) for menus/settings. This page
covers every header in the canonical source format and all 59 commands accepted
by the checked TagGroove parser. Examples inspect parser output directly; they
do not certify a provider, track matching or an audio-file write.

## Description headers

Write `[Key]=value` on one line. Header names ignore case. A parser section
starts with `[ParserScriptAlbum]=...` or `[ParserScriptIndex]=...` and continues
until the next header. UTF-8 is the preferred source-file encoding; a BOM is
recognized and non-UTF-8 legacy text falls back to Windows-1252.

| Header | Meaning and important boundary |
| --- | --- |
| `[Name]` | Descriptive source name; the filename still determines its menu path |
| `[BasedOn]` | Original site/source metadata; use the actual request/base headers below rather than relying on this as URL resolution |
| `[IndexUrl]` | Search address containing `%s`, replaced with the assembled query |
| `[AlbumUrl]` | Album base/`%s` template for a result address, or the direct-lookup address when there is no index stage |
| `[PreviewUrl]` | Optional preview address for a direct lookup; `%s` uses its first search field |
| `[WordSeparator]` | String placed between separately encoded search words; absent/empty defaults to `%20` |
| `[IndexFormat]` | Pipe-separated result fields; `%_url%` is required for a result list |
| `[SearchBy]` | Up to three labelled search fields with defaults and URL fragments |
| `[Encoding]` | Encoding/escaping policy for search words in URLs; separate from source-file encoding |
| `[UserAgent]` | `1` uses the application's standard HTTP user agent; otherwise the web-source browser-style agent is used |
| `[ParserScriptIndex]` | Script that emits the result list into the default `OUTPUT` buffer |
| `[ParserScriptAlbum]` | Required script that emits release and track fields |
| `[Include]` | Plain companion filename whose keys replace this file's keys |
| `[MinAppVersionWin]` | Upstream Windows minimum-version declaration, retained as source metadata; not a TagGroove compatibility certificate |
| `[MinAppVersionMac]` | Equivalent upstream Mac declaration; check TagGroove commands independently |
| `[Settings]` | Plain `.settings` schema filename used by this source's configuration variables |

A source needs an album parser. A missing index URL or index parser makes it
a direct lookup; its search field may supply a complete web address when no
IndexUrl/AlbumUrl is set. Avoid adding an incomplete index stage when a known
release identifier or address is all that is needed.

The old misspelling `WordSeperator` is accepted as a fallback, but use
`WordSeparator` in new sources. Unknown headers can be retained diagnostically;
they do not automatically add a feature. In particular, do not infer a
source-order guarantee from an unverified header in an imported script.

### Search fields and URL encoding

A SearchBy entry has `Label||default format||URL fragment`. Triples can follow
one another with `||`, or separate entries can use `$$`. For example,
`Artist||%artist%||artist=%s||Album||%album%||&album=%s`
creates two fields. Defaults use the first selected file. Entered text is split
into words, encoded according to Encoding, joined with WordSeparator, inserted
into each fragment, then assembled into the IndexUrl/AlbumUrl `%s` position.

With `WordSeparator=+`, the words `Juniper Vale` become `Juniper+Vale`.
Characters inside each word, such as `&`, are encoded rather than becoming a
new query parameter. Repeated whitespace between words is normalized by this
word-splitting step. If a URL template lacks `%s`, the query is appended; an
explicit substitution point is clearer.

| Encoding value | Current search-word behavior |
| --- | --- |
| `url-utf-8` | Percent-encode UTF-8 bytes, retaining unreserved URL characters |
| `url` | Percent-encode ISO-8859-1 bytes; unrepresentable characters become `?` before escaping |
| Missing/empty, or `utf-8` | Keep ASCII punctuation such as `:` and `/`, while escaping `#`, `%`, `&`, `+`, `?`, controls and non-ASCII UTF-8 bytes |
| `iso-8859-1` | The corresponding Latin-1 byte policy |
| `ansi` | The web-source query implementation uses Windows-1252 with that ASCII escaping policy |

The `ansi` row concerns this URL builder. It is not a statement that every
TagGroove ANSI operation uses the same platform codepage. Prefer `url-utf-8` for
a UTF-8 API unless its actual requirements say otherwise.

A complete-address direct lookup is different from a search term. When there
is no album base and exactly one search field with the fragment `%s`, such as
`URL||%www%||%s`, TagGroove uses that field as the complete address. It preserves
query delimiters, fragments and existing percent escapes instead of applying
word splitting, WordSeparator or Encoding to the address. The normal HTTP(S)
address validation still applies; a stored value does not authorize file URLs
or embedded user credentials.

For the same complete-address field, `[PreviewUrl]=%s` also keeps that address.
A nested preview template such as `https://preview.example/?url=%s` still uses
the normal search-word escaping. Use `url-utf-8` when embedding a URL as that
parameter's value. ID templates, multiple search fields and ordinary index
searches continue to encode their input words normally.

### Index results

Scripts start in the `OUTPUT` buffer. `OutputTo "Output"` or `OutputTo ""`
returns to that default buffer, which supplies the index results. Finish each
result with SayNewline. Separate its values with `|` in the order defined by
IndexFormat. The field names beginning with `_` are internal rather than
visible result columns:

- `%_url%`: album address; an empty/invalid result address is not a usable row.
- `%_coverurl%`: optional thumbnail address for the result. Use a reasonably
  small image, about 250 × 250 pixels or less, rather than a full-size cover.
- `%_preview%`: optional address for previewing the result.
- Other named fields, such as `%Album%` and `%Year%`, become visible columns.

An absolute album URL is used directly. Otherwise AlbumUrl supplies its base, or substitutes the emitted address into
its `%s` position, and the result is resolved against the downloaded page.
Do not duplicate a path prefix in both the base and emitted result. Cover and
preview addresses resolve against the page too. Only accepted web addresses
are used by the request workflow.

### Album and track outputs

OutputTo chooses a named buffer. An album value such as ALBUM applies to the
release. Track-valued buffers contain `|`-separated entries, conventionally
ending with `|`, as in `Paper Boats|Lanterns on the Water|`. A single track
also needs its separator to distinguish a track list from one album value.

`TRACKS` establishes the track list/count when supplied, and its nonempty
values supply titles when `TITLE` is absent. Otherwise `TITLE`, or another
track-valued output, supplies the count. Keep TRACK, TITLE, per-track ARTIST
and other lists aligned. `COVERURL`/`_COVERURL` supplies image addresses.
`_LENGTH` accepts m:ss or h:mm:ss; a plain number is milliseconds at every
magnitude (`5000` means five seconds). A script supplying plain seconds must
convert them before output. Other
underscore-prefixed buffers are internal and are not written as ordinary tags.
Double backslashes separate multiple values within a field, as in `Juniper Vale\\Mira Solberg`.

Stored values decode semicolon-terminated HTML character references and `\uXXXX` escapes.
Named references use the complete HTML name set, including two-character
expansions such as `&NotEqualTilde;`; names are case-sensitive. Decoding makes
one pass, so `&amp;Alpha;` becomes the literal `&Alpha;`. Decimal and hexadecimal numeric references use the HTML replacement rules:
`&#128;` becomes `€`, while null, surrogate and out-of-range values become
`�`. Other valid code points are retained, including C1 controls without a
replacement. Unknown or malformed references and references without their
terminating semicolon stay unchanged. A
single-track release can take album-wide `TITLE`; `ALBUM` may be per track.
`TRACKS` and `OUTPUT` are control buffers, not ordinary saved tag fields.

For a pipe that belongs inside a value, use `$verticalBar()` in a SayFormat
or UseFormat expression. It emits a protected marker that the release/result
conversion restores to a literal pipe after splitting. Literal `$verticalBar()`
text supplied by a script is handled the same way. A raw `|` splits the output
instead.

This parser-level description does not establish later duration-unit handling
or automatic file-order suggestions. Check titles/positions and review the
mapping manually where needed; parser output, release conversion and matching
are separate stages.

### Includes and settings

Include resolution happens after reading this file's own keys. Included keys
replace local keys regardless of where the Include line appeared. With several
includes, later includes can replace earlier included keys. Keep source-specific
values out of shared includes unless that override is intended. Avoid cycles;
the checked loader permits at most eight nested include levels.

A settings schema needs a nonempty root `key` and a `settings` array. Give each
entry a stable `key`, a useful `title`, optional `description` and a correctly
typed `default` where it holds a value:

| Type | Control/value |
| --- | --- |
| `string` | Text input, or a dropdown when `choices` supplies strings |
| `number` | Numeric input; saved values must be numeric |
| `bool` | Checkbox with a boolean value |
| `heading` | Heading only; no stored value |
| `separator` | Separator with optional explanatory text; no stored value |

Saved values live under the schema's root key in the shared sources
`settings.json`. A saved value of the wrong type falls back to a default.
Absent bool/number/string defaults fall back to false/0/empty text. Adding a
new key can therefore work with an old saved settings file. Different schema
root keys keep sources' values separate.

Optional `localizations` map two-letter ISO-639-1 codes to entries identified
by those same keys. The root key localizes the dialog title; entry keys localize
titles/descriptions. In the checked baseline, the settings dialog uses the
system/browser language code and falls back to unlocalized text when no entry
matches. A schema is not an automatic translation of arbitrary parser output.

IfVar/IfNotVar compare configured values and SayVar emits one. Booleans reach
the parser as `true`/`false`, numbers as text and strings as themselves. Variable
keys are exact; value comparisons ignore ASCII case. These settings are
separate from named outputs and selected-file metadata.

## Choose a command family

| Task | Commands to start with |
| --- | --- |
| [Locate data](#move-and-locate) | FindLine, FindInLine, GotoLine, MoveChar |
| [Read or emit](#read-and-emit-text) | SayRest, SayUntil, SayFormat, SayOutput |
| [Change working state](#change-buffers-or-input) | Set, OutputTo, Use, UseFormat |
| [Branch or repeat](#conditions-and-loops) | If, IfVar, Do, While |
| [Clean text](#transform-text-and-whitespace) | Replace, RegexpReplace, KillTag, Trim |
| [Inspect execution](#debugging) | Debug, DebugWriteInput |
| [Read JSON](#json-mode-and-selection) | json, json_select, json_foreach |

## Read a command example

Each example below states its input, script and expected output buffer, using
the marked visible-space notation where applicable.
They run in a fresh parser context, with page URL
`https://catalogue.example.invalid/release/harbour`. That address is context
text for the examples; no request to it is needed. A Settings line supplies
only the named example variable. Debug files are additional in-memory parser
results; the app writes them under its sources/debug folder.

Commands ignore case. Double quotes delimit parser string parameters; `\"`
puts a quote inside one, and `\\` becomes one backslash. Other backslashes remain literal, so a regex `\d`
needs one backslash at this layer. `#` outside quotes starts a comment.

Ordinary string parameters substitute existing `%output%` values. Unknown
output references can remain literal text; a buffer emptied by `Set` expands
to empty text. Bare `%20` is not a field.
SayFormat and UseFormat instead evaluate their first argument as a format
string with `%_current%` and, unless restricted, named outputs as fields.
Apostrophe quoting protects literal brackets, commas and other syntax in
expressions. Balanced unquoted parentheses in an argument nest; commas inside
that text stay in the argument. Escaped `\(` and `\)` neither nest nor close
the call. A dollar sign starting no function name, such as an end anchor `$`
or replacement `$1`, is literal. Quoting remains clearest for complex patterns. See [format strings](../reference/format-strings.md)
and [regular expressions](../reference/regular-expressions.md). The case-sensitive
parser regex commands below differ from the default case policy of the
`$regexp(...)` formatting function.

Character/line positions are one-based; relative moves can be negative. Counts,
positions and loop limits require whole numbers. Read trimming is on by default
and applies to the documented read commands, not every emitted string. Code
blocks retain significant spaces. Examples labelled **Visible spaces** use
`␠` to show one ASCII space in input/output, so edge spaces can be seen;
substitute a real space when constructing that input. Their scripts remain
literal commands. Examples labelled CRLF make output line endings explicit.

## Move and locate

<!-- web-example:findline -->

### `FindLine`

**Syntax:** `FindLine "text" [occurrence] [optional]`

Find the next matching line from the current position and move to its first character. The occurrence counts matching lines, not several copies on one line. Use a third parameter of 1 to leave a missing match nonfatal; the cursor moves past the end of the input.

**Input**

```text
Header
Album: Old Harbour
Album: Harbour Lights
```

**Script**

```text
FindLine "Album:" 2
SayRest
```

**Output buffer:** `OUTPUT`

```text
Album: Harbour Lights
```

<!-- web-example:findlinenocase -->

### `FindLineNoCase`

**Syntax:** `FindLineNoCase "text" [occurrence] [optional]`

Use the same matching-line navigation without case sensitivity. Finding a line does not skip the label within it: use FindInLine next when the value starts after a marker.

**Input**

```text
header
ALBUM: Harbour Lights
```

**Script**

```text
FindLineNoCase "album:"
SayRest
```

**Output buffer:** `OUTPUT`

```text
ALBUM: Harbour Lights
```

<!-- web-example:findinline -->

### `FindInLine`

**Syntax:** `FindInLine "text" [occurrence] [optional]`

Search the current line from the cursor and place it after the chosen occurrence. A missing match is an error unless the third parameter is 1; an optional failure moves to the end of this line. It does not search later lines.

**Input**

```text
Title: old; Title: Paper Boats
```

**Script**

```text
FindInLine "Title: " 2
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:gotochar -->

### `GotoChar`

**Syntax:** `GotoChar position`

Choose a one-based character position in the current line. Characters are Unicode characters, not UTF-8 bytes. Positions before 1 clamp to 1, and positions beyond the line stop at its end.

**Input**

```text
Álbum: Lanterns
```

**Script**

```text
GotoChar 8
SayRest
```

**Output buffer:** `OUTPUT`

```text
Lanterns
```

<!-- web-example:gotoline -->

### `GotoLine`

**Syntax:** `GotoLine line`

Choose a one-based input line and reset the character cursor to its start. A line beyond the input is an error. This is useful for a known fixed format; marker searches survive inserted lines better.

**Input**

```text
Header
Paper Boats
```

**Script**

```text
GotoLine 2
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:movechar -->

### `MoveChar`

**Syntax:** `MoveChar distance`

Move relative to the current character position. Negative values move left. Movement stays on the current line and clamps at its ends.

**Input**

```text
XXPaper Boats
```

**Script**

```text
MoveChar 5
MoveChar -3
SayNChars 5
```

**Output buffer:** `OUTPUT`

```text
Paper
```

<!-- web-example:moveline -->

### `MoveLine`

**Syntax:** `MoveLine distance [optional]`

Move relative to the current input line and reset its character position. Negative values move up. With the optional parameter 1, out-of-range movement clamps at an edge instead of failing.

**Input**

```text
Header
Notes
Paper Boats
```

**Script**

```text
MoveLine 2
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

## Read and emit text

<!-- web-example:say -->

### `Say`

**Syntax:** `Say "text"`

Append the parameter text to the chosen output. This does not consume input. Ordinary parameters expand existing %output% references; they do not automatically evaluate scripting functions. Use SayFormat for a format string.

**Input**

```text
Ignored input
```

**Script**

```text
Say "Juniper Vale"
```

**Output buffer:** `OUTPUT`

```text
Juniper Vale
```

<!-- web-example:sayuntil -->

### `SayUntil`

**Syntax:** `SayUntil "delimiter"`

Append input up to the delimiter on this line, leaving the cursor at the delimiter. A missing delimiter reads to the line end. The default Trim setting trims the extracted text; the delimiter itself is not consumed.

**Input**

```text
Paper Boats|next
```

**Script**

```text
SayUntil "|"
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:sayuntilml -->

### `SayUntilML`

**Syntax:** `SayUntilML "delimiter"`

Read across lines until the delimiter or input end. Joined output lines use CRLF. The cursor stops at the delimiter; advance it before repeating the same extraction.

**Line endings:** `CRLF`

**Input**

```text
Paper
Boats</title>
```

**Script**

```text
SayUntilML "</title>"
```

**Output buffer:** `OUTPUT`

```text
Paper
Boats
```

<!-- web-example:sayrest -->

### `SayRest`

**Syntax:** `SayRest`

Append the remaining current line and consume it. Read text is trimmed by default; Trim "off" retains its edge whitespace. Later lines are not included.

**Input**

```text
Title: Paper Boats
```

**Script**

```text
FindInLine "Title: "
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:saynchars -->

### `SayNChars`

**Syntax:** `SayNChars count`

Copy and consume this many Unicode characters from the cursor, stopping at line end. This direct character copy retains spaces; it is not affected by automatic read trimming.

**Visible spaces:** `␠`

**Input**

```text
␠␠Paper
```

**Script**

```text
SayNChars 2
```

**Output buffer:** `OUTPUT`

```text
␠␠
```

<!-- web-example:saynextnumber -->

### `SayNextNumber`

**Syntax:** `SayNextNumber`

Find the next ASCII numeric value and emit it, advancing past it. An optional minus and decimal fraction are retained. This is not a localized thousands/decimal-number parser.

**Input**

```text
Take -03.5 bonus
```

**Script**

```text
SayNextNumber
```

**Output buffer:** `OUTPUT`

```text
-03.5
```

<!-- web-example:saynextword -->

### `SayNextWord`

**Syntax:** `SayNextWord`

Skip leading whitespace, then copy the next non-whitespace run. Punctuation belongs to that run. The cursor advances past the emitted text.

**Input**

```text
   Paper, Boats
```

**Script**

```text
SayNextWord
```

**Output buffer:** `OUTPUT`

```text
Paper,
```

<!-- web-example:sayoutput -->

### `SayOutput`

**Syntax:** `SayOutput "buffer"`

Append an existing output without consuming input. Names ignore case. CurrentUrl is supplied by the request context and can be read the same way.

**Input**

```text

```

**Script**

```text
Set "ALBUM" "Harbour Lights"
SayOutput "album"
Say " @ "
SayOutput "CurrentUrl"
```

**Output buffer:** `OUTPUT`

```text
Harbour Lights @ https://catalogue.example.invalid/release/harbour
```

<!-- web-example:saynewline -->

### `SayNewline`

**Syntax:** `SayNewline`

Append a CRLF pair. It does not move the input cursor. Index scripts use this to finish a result row; it is different from a pipe separating columns.

**Line endings:** `CRLF`

**Input**

```text

```

**Script**

```text
Say "Paper"
SayNewline
Say "Boats"
```

**Output buffer:** `OUTPUT`

```text
Paper
Boats
```

<!-- web-example:saydate -->

### `SayDate`

**Syntax:** `SayDate [unit]`

Consume a number at the cursor and emit its UTC calendar date. The default unit is seconds since 1970-01-01; "ms" selects milliseconds. The output uses YYYY-MM-DD.

**Input**

```text
1704067200000
```

**Script**

```text
SayDate "ms"
```

**Output buffer:** `OUTPUT`

```text
2024-01-01
```

<!-- web-example:sayduration -->

### `SayDuration`

**Syntax:** `SayDuration [unit] [hours] [milliseconds]`

Consume a numeric interval. Use seconds by default, "ms" for milliseconds or "m" for minutes. Set the next flag to 1 for h:mm:ss and the final flag to 1 for a .mmm suffix; otherwise minutes can exceed 59.

**Input**

```text
62005
```

**Script**

```text
SayDuration "ms" 1 1
```

**Output buffer:** `OUTPUT`

```text
0:01:02.005
```

<!-- web-example:sayregexp -->

### `SayRegexp`

**Syntax:** `SayRegexp "pattern" ["separator"] ["stop text"]`

Find every full regex match in the current remainder, joining them with a space by default. Omit the third parameter to search the entire current remainder. A supplied third parameter restricts matching to text before that stop text’s first occurrence; if the supplied stop text is absent from the input, this command emits nothing and leaves the cursor unchanged. The cursor stays where it was after matching, so later reads can inspect the same text. Matching is case-sensitive here; use an inline regex option when another policy is intended.

**Input**

```text
item2 item7; item9
```

**Script**

```text
SayRegexp "item\d" ", " ";"
Say "|"
SayRest
```

**Output buffer:** `OUTPUT`

```text
item2, item7|item2 item7; item9
```

<!-- web-example:sayformat -->

### `SayFormat`

**Syntax:** `SayFormat "format string" [current-only]`

Evaluate a format string and append it. %_current% is the entire current input, not only the cursor remainder. Named outputs are available as fields unless the second parameter is 1. Quote format-language punctuation inside the format string as usual. This command does not consume the input cursor.

**Input**

```text
paper boats
```

**Script**

```text
Set "ARTIST" "Juniper Vale"
SayFormat "$upper(%_current%) — %ARTIST%"
```

**Output buffer:** `OUTPUT`

```text
PAPER BOATS — Juniper Vale
```

<!-- web-example:sayvar -->

### `SayVar`

**Syntax:** `SayVar "key"`

Append a configured setting as text. The key matches the schema key exactly; a missing key gives empty text. This is a setting value, not an output buffer or selected audio tag.

**Settings:** `edition=deluxe`

**Input**

```text

```

**Script**

```text
SayVar "edition"
```

**Output buffer:** `OUTPUT`

```text
deluxe
```

## Change buffers or input

<!-- web-example:set -->

### `Set`

**Syntax:** `Set "buffer" ["value"]`

Replace the named buffer with a value. Omit the second parameter to empty that buffer; later `%buffer%` references expand to empty text. Unlike OutputTo, this does not switch the destination of later Say commands.

**Input**

```text

```

**Script**

```text
Set "NOTE" "old"
Set "NOTE"
IfNotOutput "NOTE"
Say "cleared"
Endif
```

**Output buffer:** `OUTPUT`

```text
cleared
```

<!-- web-example:outputto -->

### `OutputTo`

**Syntax:** `OutputTo "buffer"`

Choose the output used by later Say commands. Text is appended there; it does not clear an earlier value. Use an empty name or "Output" to return to the default `OUTPUT` index-result buffer.

**Input**

```text
Album: Harbour Lights
```

**Script**

```text
OutputTo "ALBUM"
FindInLine "Album: "
SayRest
```

**Output buffer:** `ALBUM`

```text
Harbour Lights
```

<!-- web-example:skipchars -->

### `SkipChars`

**Syntax:** `SkipChars "characters"`

Skip a prefix made from any characters in the supplied set. Stop at the first other character; the string is a set, not a literal word to find.

**Input**

```text
 _-Paper Boats
```

**Script**

```text
SkipChars " _-"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:insert -->

### `Insert`

**Syntax:** `Insert "text"`

Insert text at the current input cursor without consuming it. Reading afterward starts at that inserted text. This edits the parser input, not the downloaded website or audio file.

**Input**

```text
Boats
```

**Script**

```text
Insert "Paper "
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:use -->

### `Use`

**Syntax:** `Use "text"`

Replace the entire current input and reset the cursor. Use %buffer% to supply an output value; a bare buffer name is just literal text. The original downloaded input remains available to json "on".

**Input**

```text
Ignored input
```

**Script**

```text
Set "ALBUM" "Harbour Lights"
Use "%ALBUM%"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Harbour Lights
```

<!-- web-example:useformat -->

### `UseFormat`

**Syntax:** `UseFormat "format string" [current-only]`

Evaluate a format string, replace current input with its result and reset the cursor. The second parameter 1 excludes named outputs while keeping %_current% available.

**Input**

```text
paper boats
```

**Script**

```text
UseFormat "$upper(%_current%)"
SayRest
```

**Output buffer:** `OUTPUT`

```text
PAPER BOATS
```

## Conditions and loops

<!-- web-example:if -->

### `If`

**Syntax:** `If "prefix"`

Test whether the current remainder starts with this case-sensitive prefix. This is not a search over the whole line. A false branch jumps to Else or Endif without consuming input. With an empty prefix, If "" is true only when the current line has no text left at the cursor.

**Input**

```text
Album: Harbour Lights
```

**Script**

```text
If "Album:"
Say "album found"
Endif
```

**Output buffer:** `OUTPUT`

```text
album found
```

<!-- web-example:ifnot -->

### `IfNot`

**Syntax:** `IfNot "prefix"`

Take the branch when the current remainder does not start with the prefix. An empty prefix instead tests whether text remains: IfNot "" is true while the current remainder is nonempty. The condition itself does not move the cursor.

**Input**

```text
Paper Boats
```

**Script**

```text
IfNot "#"
SayRest
Endif
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:ifoutput -->

### `IfOutput`

**Syntax:** `IfOutput "buffer"`

Test whether the named output exists and contains nonempty text. A buffer containing only spaces is still nonempty. This does not inspect an audio tag unless the script placed that value in the buffer.

**Input**

```text

```

**Script**

```text
Set "ARTIST" "Juniper Vale"
IfOutput "ARTIST"
Say "present"
Endif
```

**Output buffer:** `OUTPUT`

```text
present
```

<!-- web-example:ifnotoutput -->

### `IfNotOutput`

**Syntax:** `IfNotOutput "buffer"`

Take the branch when an output is absent or empty. Buffer names are case-insensitive.

**Input**

```text

```

**Script**

```text
IfNotOutput "COMMENT"
Say "no comment"
Endif
```

**Output buffer:** `OUTPUT`

```text
no comment
```

<!-- web-example:ifgreater -->

### `IfGreater`

**Syntax:** `IfGreater number`

Compare a numeric value at the cursor with the threshold, without consuming it. No number at the cursor makes the condition false. This does not search ahead for a later number.

**Input**

```text
12 tracks
```

**Script**

```text
IfGreater 10
Say "large"
Endif
```

**Output buffer:** `OUTPUT`

```text
large
```

<!-- web-example:ifless -->

### `IfLess`

**Syntax:** `IfLess number`

Use the same cursor-based numeric comparison for a lower threshold. The threshold can be a decimal value; movement/count parameters elsewhere still require whole numbers.

**Input**

```text
2.5 tracks
```

**Script**

```text
IfLess 3
Say "small"
Endif
```

**Output buffer:** `OUTPUT`

```text
small
```

<!-- web-example:ifvar -->

### `IfVar`

**Syntax:** `IfVar "key" "value"`

Compare a configured setting with the supplied text, ignoring ASCII case in the values. Keys remain exact. An absent setting reads as empty text; provide a schema/default for meaningful choices.

**Settings:** `edition=deluxe`

**Input**

```text

```

**Script**

```text
IfVar "edition" "DELUXE"
Say "extra tracks"
Endif
```

**Output buffer:** `OUTPUT`

```text
extra tracks
```

<!-- web-example:ifnotvar -->

### `IfNotVar`

**Syntax:** `IfNotVar "key" "value"`

Take the branch when the configured value differs. It uses the same setting/key rules as IfVar.

**Settings:** `edition=deluxe`

**Input**

```text

```

**Script**

```text
IfNotVar "edition" "standard"
Say "alternate edition"
Endif
```

**Output buffer:** `OUTPUT`

```text
alternate edition
```

<!-- web-example:else -->

### `Else`

**Syntax:** `Else`

Separate the true and false parts of one condition. At most one Else belongs to a condition; nested blocks must close in order.

**Input**

```text
Live
```

**Script**

```text
If "Studio"
Say "studio"
Else
Say "other"
Endif
```

**Output buffer:** `OUTPUT`

```text
other
```

<!-- web-example:endif -->

### `Endif`

**Syntax:** `Endif`

Close an If-family condition. Execution resumes after it whether the condition was true or false; a missing or unmatched Endif is a compile error.

**Input**

```text
Paper
```

**Script**

```text
If "Paper"
Say "matched"
Endif
Say "."
```

**Output buffer:** `OUTPUT`

```text
matched.
```

<!-- web-example:do -->

### `Do`

**Syntax:** `Do`

Start a body that runs at least once. While closes the body and checks whether it repeats. Advance input or provide a useful limit; finding the same marker repeatedly does not make progress.

**Input**

```text
xxxstop
```

**Script**

```text
Do
SayNChars 1
While "x" 3
```

**Output buffer:** `OUTPUT`

```text
xxx
```

<!-- web-example:while -->

### `While`

**Syntax:** `While "prefix" [maximum]`

Repeat from the matching Do while the current remainder starts with the prefix. A supplied maximum bounds body executions. Use positive limits and keep Do/While loops unnested for portability to other implementations.

**Input**

```text
AAA
```

**Script**

```text
Do
SayNChars 1
While "A" 2
```

**Output buffer:** `OUTPUT`

```text
AA
```

## Transform text and whitespace

<!-- web-example:replace -->

### `Replace`

**Syntax:** `Replace "from" "to"`

Replace literal matches on the current line only and return the cursor to its start. Other lines are unchanged. An empty search performs no replacement.

**Input**

```text
Paper_Boats
Other_Label
```

**Script**

```text
FindInLine "_"
Replace "_" " "
SayRest
Say "|"
MoveLine 1
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats|Other_Label
```

<!-- web-example:regexpreplace -->

### `RegexpReplace`

**Syntax:** `RegexpReplace "pattern" "replacement"`

Replace regex matches on the current line only, using case-sensitive matching and Boost replacement captures, then return the cursor to the line start. These are parser parameters; their dollar captures do not invoke formatting functions.

**Input**

```text
03_Paper Boats
```

**Script**

```text
RegexpReplace "^([0-9]+)_(.*)$" "$2 [$1]"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats [03]
```

<!-- web-example:joinuntil -->

### `JoinUntil`

**Syntax:** `JoinUntil "text"`

Join the current line through the line containing the next marker, or through input end if absent. If the marker is already in the current remainder, no join is needed. No separator is inserted. Existing spaces at line boundaries determine whether words stay separated. The joined later lines remain in place for subsequent line movement.

**Input**

```text
<title>Paper
 Boats</title>
```

**Script**

```text
JoinUntil "</title>"
FindInLine ">"
SayUntil "</title>"
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:joinlines -->

### `JoinLines`

**Syntax:** `JoinLines count`

Join this line with the next count lines, without adding spaces. -1 or a count beyond the remaining range joins all remaining lines. It does not mean a total of count lines including this one. The later lines remain after their text is copied into the joined line.

**Visible spaces:** `␠`

**Input**

```text
Paper␠
Boats
```

**Script**

```text
JoinLines 1
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper␠Boats
```

<!-- web-example:killtag -->

### `KillTag`

**Syntax:** `KillTag "tag" ["replacement"]`

Remove matching opening/closing/self-closing tags on the current line, ignoring case in tag names. Use * for any tag and an optional replacement instead of removal. This is a text helper, not a complete HTML parser or entity decoder.

**Input**

```text
<em>Paper</em> Boats
```

**Script**

```text
KillTag "em"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:trim -->

### `Trim`

**Syntax:** `Trim "on"|"off"`

Control automatic edge-whitespace trimming for SayRest, SayUntil and SayUntilML. The default is on. Direct Say/SayNChars copies are not implicitly trimmed. The example keeps two spaces on each side.

**Visible spaces:** `␠`

**Input**

```text
␠␠Paper␠␠
```

**Script**

```text
Trim "off"
SayRest
```

**Output buffer:** `OUTPUT`

```text
␠␠Paper␠␠
```

<!-- web-example:unspace -->

### `Unspace`

**Syntax:** `Unspace`

Remove whitespace at the beginning/end of the current input line and adjust the cursor. Internal whitespace is unchanged. This edits input, unlike changing the Trim read policy.

**Visible spaces:** `␠`

**Input**

```text
␠␠Paper␠␠
```

**Script**

```text
Unspace
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper
```

## Debugging

<!-- web-example:debug -->

### `Debug`

**Syntax:** `Debug "on"|"off" ["filename"] [megabytes]`

Start/stop a command trace. The default name is debug.out and the default cap is 10 MB; a positive whole-number cap changes it. A trace forced by Options keeps its own filename and cannot be switched off by the script.

**Debug file:** `trace.out`

**Input**

```text
Paper Boats
```

**Script**

```text
Debug "on" "trace.out"
SayRest
Debug "off"
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:debugwriteinput -->

### `DebugWriteInput`

**Syntax:** `DebugWriteInput ["filename"]`

Capture the entire current, possibly transformed input. The default name is debug-input.out. In the application, debug filenames are reduced to safe plain names under the sources/debug folder.

**Debug file:** `snapshot.txt`

**Input**

```text
Paper_Boats
```

**Script**

```text
Replace "_" " "
DebugWriteInput "snapshot.txt"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

## JSON mode and selection

<!-- web-example:json -->

### `json`

**Syntax:** `json "on"|"off" ["current"]`

Enable JSON mode from the original downloaded text. Add "current" to parse transformed input from the cursor onward. Text after the first JSON value is ignored. Selection writes a value to text input so a later Say command can emit it. Turning JSON mode off restores the input and cursor saved when JSON mode began.

**Input**

```text
prefix:{"title":"Paper Boats"}
```

**Script**

```text
Replace "prefix:" ""
json "on" "current"
json_select "title"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats
```

<!-- web-example:json_foreach -->

### `json_foreach`

**Syntax:** `json_foreach "array"`

Enter each array item in its original order, or each object member in document order. The input initially contains the collection size; select the item/field before reading it. An empty/missing array skips the body. Close with json_foreach_end.

**Input**

```text
{"tracks":["Paper Boats","Lanterns"]}
```

**Script**

```text
json "on"
json_foreach "tracks"
json_select ""
SayRest
Say "|"
json_foreach_end
```

**Output buffer:** `OUTPUT`

```text
Paper Boats|Lanterns|
```

<!-- web-example:json_foreach_reverse -->

### `json_foreach_reverse`

**Syntax:** `json_foreach_reverse "array"`

Iterate the same array or object in reverse order. An array counter still starts at 1 for the first emitted item rather than its original index; object counters remain member names.

**Input**

```text
{"tracks":["Paper Boats","Lanterns"]}
```

**Script**

```text
json "on"
json_foreach_reverse "tracks"
json_select ""
SayRest
Say "|"
json_foreach_end
```

**Output buffer:** `OUTPUT`

```text
Lanterns|Paper Boats|
```

<!-- web-example:json_foreach_counter -->

### `json_foreach_counter`

**Syntax:** `json_foreach_counter [digits]`

Append the one-based array-loop counter directly to output, with optional leading-zero width. For an object loop, append the member name instead. It does not replace input and needs no SayRest afterwards. Calling it outside an active loop is an error.

**Input**

```text
{"tracks":["Paper Boats","Lanterns"]}
```

**Script**

```text
json "on"
json_foreach "tracks"
json_foreach_counter 2
Say "|"
json_foreach_end
```

**Output buffer:** `OUTPUT`

```text
01|02|
```

<!-- web-example:json_foreach_end -->

### `json_foreach_end`

**Syntax:** `json_foreach_end`

Close the last JSON collection loop, advance to its next item or leave it. Object selections made inside the iteration are unwound before the next item. Nested JSON loops must close in order.

**Input**

```text
{"tracks":["Paper","Boats"]}
```

**Script**

```text
json "on"
json_foreach "tracks"
json_select ""
SayRest
json_foreach_end
```

**Output buffer:** `OUTPUT`

```text
PaperBoats
```

<!-- web-example:json_select_object -->

### `json_select_object`

**Syntax:** `json_select_object "key"`

Enter an object context. When an object is found, its key becomes text input; otherwise input is empty. Leave that selected context with json_unselect_object before reading the parent again.

**Input**

```text
{"release":{"title":"Harbour Lights"}}
```

**Script**

```text
json "on"
json_select_object "release"
json_select "title"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Harbour Lights
```

<!-- web-example:json_unselect_object -->

### `json_unselect_object`

**Syntax:** `json_unselect_object`

Return to the preceding JSON context. This does not replace current text input by itself: select another value after returning. It cannot pop an active loop item as though it were a selected object. With no additional selected object it stays in the current context.

**Input**

```text
{"title":"root","release":{"title":"inner"}}
```

**Script**

```text
json "on"
json_select_object "release"
json_select "title"
SayRest
Say "/"
json_unselect_object
json_select "title"
SayRest
```

**Output buffer:** `OUTPUT`

```text
inner/root
```

<!-- web-example:json_select -->

### `json_select`

**Syntax:** `json_select "key"`

Select a value into text input. Strings lose JSON quotes; numbers/booleans become text; null/missing values become empty. An empty key selects the current value. Dotted paths are supported after checking for a literal key of that name.

**Input**

```text
{"artist":{"name":"Juniper Vale"}}
```

**Script**

```text
json "on"
json_select "artist.name"
SayRest
```

**Output buffer:** `OUTPUT`

```text
Juniper Vale
```

<!-- web-example:json_select_array -->

### `json_select_array`

**Syntax:** `json_select_array "array" index ["separator"]`

Select a one-based array element, or -1 for all elements joined by the supplied separator. An out-of-range index becomes empty. Without a separator, all-element mode concatenates the values. Selecting one object element also enters its context until json_unselect_object. This one-based index differs from a numeric component in a dotted JSON path.

**Input**

```text
{"tracks":["Paper Boats","Lanterns"]}
```

**Script**

```text
json "on"
json_select_array "tracks" -1 " / "
SayRest
```

**Output buffer:** `OUTPUT`

```text
Paper Boats / Lanterns
```

<!-- web-example:json_select_many -->

### `json_select_many`

**Syntax:** `json_select_many "array" "field" ["separator"] ["last separator"] [maximum] [include-empty]`

Extract one field from many array objects. The default separator is comma-space; a last separator can join the final pair differently. A positive maximum truncates selected values after empty filtering. Set the final flag to 1 to keep empty/missing field values as empty slots.

**Input**

```text
{"artists":[{"name":"Juniper Vale"},{"name":"Mira Solberg"},{"name":"Kofi"}]}
```

**Script**

```text
json "on"
json_select_many "artists" "name" ", " " & " 2
SayRest
```

**Output buffer:** `OUTPUT`

```text
Juniper Vale & Mira Solberg
```

<!-- web-example:json_select_many_count -->

### `json_select_many_count`

**Syntax:** `json_select_many_count "array" ["field"] [include-empty]`

Count array items, optionally requiring a named field. With a field, empty values are excluded unless the last flag is 1; missing keys remain excluded even with that flag. The count is put on input, not directly in output.

**Input**

```text
{"artists":[{"name":"Juniper"},{"name":""},{},{"name":"Mira"}]}
```

**Script**

```text
json "on"
json_select_many_count "artists" "name" 1
SayRest
```

**Output buffer:** `OUTPUT`

```text
3
```

## Troubleshoot and bound the work

Start with the saved input and one field. Verify a Find/Goto step, inspect its
cursor, then add one Say operation. Add a loop only after one iteration works.
Missing markers, an unexpected JSON shape, a changed URL or a login/error page
need different corrections; moving the cursor farther is not a general fix.

An unknown command, wrong argument count or unbalanced block fails compilation
with a source line. Runtime failures can name the command/input position.
DebugWriteInput captures transformed input; a forced Options trace also retains
the downloaded input. File names are sanitized and kept under sources/debug.
Check those files before changing an extraction pattern blindly.

The checked engine stops after 2,000,000 command steps and checks appended
output against a 16 MB cumulative byte budget. JoinLines and JoinUntil also
share a 16 MiB budget for bytes appended during one run. Every copy counts,
including repeated joins and overlapping retained lines; changing or replacing
the input does not reset it. A join that would exceed the allowance fails
before appending or moving the cursor. This bounds join growth rather than
setting a universal size limit for all input transformations. Debug traces have a default
10 MB cap. These guardrails do not certify
that a source handles every response size, provider rule or future page layout.
Keep requests and scripts small while developing, preserve a working copy and
inspect the final review/import outcome.
