# Format strings

<p class="lead">A format string is a line of text with blanks in it. For each file, OpenTag fills the blanks with that file's values: <code>%artist% - %title%</code> becomes <code>Juniper Vale - Low Tide</code> for one file and <code>Juniper Vale - Paper Boats</code> for the next.</p>

You write format strings to rename files, to build new tag values from old
ones, to define columns, to name exports and playlists, and in many other
places. This page is the complete reference for the language: every
character with a special meaning, how conditions decide, and how file and
folder names are made safe.

!!! tip "New to this?"
    Start with [Your first tagging session](../learn/first-tagging-session.md)
    to try a naming pattern on a practice album. Use this reference when
    you need a syntax detail.

All examples on this page use track 2 of the guide's example album, unless a
row says otherwise:

| Field | Value |
|---|---|
| `TITLE` | `Lanterns on the Water` |
| `ARTIST`, `ALBUMARTIST` | `Juniper Vale` |
| `ALBUM` | `Harbour Lights` |
| `YEAR` | `2024` |
| `TRACK` | `2/8` |
| `GENRE` | `Indie Folk` |
| `DISCNUMBER` | *(not set)* |

## At a glance

| Syntax | Meaning | Example | Result |
|---|---|---|---|
| plain text | Copied as it is | <code>Live at </code> | <code>Live at </code> |
| `%field%` | The field's value ([placeholder](placeholders.md)) | `%artist%` | `Juniper Vale` |
| `%_name%` | Information about the file | `%_extension%` | `mp3` |
| `$function(…)` | A scripting function | `$upper(%album%)` | `HARBOUR LIGHTS` |
| `[…]` | Optional section: dropped when its fields are empty | `%album%[ (%year%)]` | `Harbour Lights (2024)` |
| `'…'` | Quoted text: everything inside is literal | `'%artist%'` | `%artist%` |
| `''` | One apostrophe | `%artist%''s best` | `Juniper Vale's best` |
| `\` or `/` | A folder level (only where the result is a file name) | `%artist%\%album%` | folder `Juniper Vale`, then `Harbour Lights` |

## Where you use format strings

The same language works everywhere. Only what happens to the result differs.

| Where | The result becomes |
|---|---|
| Tag → Filename | The new file name, and folders if it contains `\` |
| Tag → Tag and the Format value action | The new value of a field, or a new file or folder name for `_FILENAME` and `_DIRECTORY` |
| Columns (**Value**) | The text shown in that column |
| Copy as Text | One line per file on the clipboard |
| Export, playlists | Output file names, partitions and extended playlist info |
| Cover actions (import, export), Import text file, Text File → Tag | The name of the file to read or write |
| Guess values (source) | The text that the guessing pattern takes apart |
| External tools (parameter) | The argument passed to the tool |
| Filter (left side of an operator) | The value that is compared |

Two more places look similar but work the other way round: **Filename → Tag**
and **Guess values** read values *out of* text with a pattern. See
[Patterns](#patterns-reading-values-out-of-text) at the end of this page.

## The building blocks

### Plain text

Anything that isn't one of the special characters `%` `$` `[` `]` `'` is
copied unchanged, spaces included. Inside a function's brackets, `,` and `)`
are special too (see [Special characters](#special-characters)).

### Placeholders: `%field%` { #placeholders }

`%name%` is replaced by the value of the field called *name*:

- **Any field works.** There is no fixed list: `%mood%`, `%catalognumber%`,
  `%my own field%` all work if the file has that field.
- **Case doesn't matter.** `%Artist%`, `%ARTIST%` and `%artist%` are the same.
- **A missing field gives empty text.** `%discnumber%` on a file without a
  disc number gives nothing, not an error and not the text `%discnumber%`.
- **Multi-value fields give their first value.** A file with two artists,
  `Juniper Vale` and `Mira Solberg`, gives `Juniper Vale` for `%artist%`. Use
  [`$meta`](#multi-value-fields) to get all of them.
- **Names starting with `_` are information about the file**, not tag
  fields: `%_filename%`, `%_bitrate%`, `%_length%`, `%_covers%` and about
  sixty more. They are all listed on the [Placeholders](placeholders.md) page.

`%%` (an empty name) and an unclosed `%title` are errors.

### Scripting functions: `$name(…)` { #functions }

A function takes arguments, separated by commas, and returns text:

```text
$left(%title%,8)
```

gives `Lanterns`. Functions nest. Each argument is itself a format string:

```text
$upper($left(%title%,8))
```

gives `LANTERNS`.

- Function names are not case-sensitive: `$cutLeft` and `$cutleft` are the same.
- **Arguments are not trimmed.** In `$replace(%title%, on, at)` the search
  text is `·on` (with a leading space). Write `$replace(%title%,on,at)`
  unless you mean the spaces.
- An unknown function name, or the wrong number of arguments, is an error
  that the preview shows before anything is changed.

Scripting functions cover text, case, numbers, dates, conditions, regular
expressions and multi-value fields. The examples below show common uses.

### Optional sections: `[…]` { #optional-sections }

Square brackets mark a part that should disappear when it has nothing to say.
A section is **kept when at least one placeholder inside it has a value**,
and dropped completely, with its text, when none has:

| Format string | With these values | Result |
|---|---|---|
| `%album%[ (%year%)]` | `YEAR` = `2024` | `Harbour Lights (2024)` |
| `%album%[ (%year%)]` | `YEAR` not set | `Harbour Lights` |
| `[CD%discnumber% - ]%title%` | `DISCNUMBER` not set | `Lanterns on the Water` |
| `[CD%discnumber% - ]%title%` | `DISCNUMBER` = `2` | `CD2 - Lanterns on the Water` |
| `[%artist% & %albumartist%]` | `ARTIST` set, `ALBUMARTIST` not set | <code>Juniper Vale &amp; </code> |
| `[(live)]` | anything | *(empty)* |

Things to know:

- **"Has a value" means a placeholder inside produced text.** In the fifth
  row, `ARTIST` is enough to keep the section. Its text is kept even though
  `ALBUMARTIST` is empty. To require both, use
  `$and` with [`$if`](#conditions).
- **A section with only plain text is always dropped**, because no
  placeholder in it has a value. Write plain text outside brackets.
- **Functions count through their arguments.** `[CD$num(%discnumber%,1)]` is
  dropped when `DISCNUMBER` is empty, even though `$num` would have
  returned `0`.
- Sections can be nested: `[%album%[ (%year%)] - ]%title%`.
- Commas and brackets inside a section that sits in a function argument are
  plain text: in `$upper([%year%, ]%album%)` the comma does not end the argument.

### Quoted text: `'…'` { #quotes }

Text between apostrophes is copied exactly, special characters included:

| Format string | Result |
|---|---|
| `'[%year%]'` | `[%year%]` |
| `'['%year%'] '%album%` | `[2024] Harbour Lights` |
| `'100% '%genre%` | `100% Indie Folk` |
| `%artist%''s favourites` | `Juniper Vale's favourites` |
| `'Rock ''n'' roll'` | `Rock 'n' roll` |

`''` (two apostrophes) gives one apostrophe, both inside and outside a quoted
part. An apostrophe that is never closed is an error ("Unterminated quoted
literal").

!!! warning "Gotcha: apostrophes in plain text"
    A single `'` in your text starts a quoted part. `%artist%'s best` is an
    error, because the quote never closes. Write `%artist%''s best`. Values
    that *contain* apostrophes, such as a title `The Keeper's Song`, are fine:
    only what you type in the format string is parsed.

## Special characters { #special-characters }

These characters mean something. To get them as plain text, quote them.

| Character | Special meaning | Write it literally as |
|---|---|---|
| `%` | Starts and ends a placeholder | `'%'` |
| `$` | Starts a function | `'$'` |
| `[` `]` | Optional section | `'['` `']'` |
| `'` | Starts and ends quoted text | `''` |
| `,` | Separates function arguments (only inside a function's brackets) | `','` |
| `)` | Ends a function (only inside a function's brackets) | `')'` |
| `(` | Nothing: always plain text | `(` |
| `\` `/` | A folder level, when the result is a file name | `_` or another character (see [file names](#file-and-folder-names)) |

Backslash is **not** an escape character in format strings. `\%` is a
backslash followed by the start of a placeholder.

!!! warning "Gotcha: `)` and `,` inside function arguments"
    `$replace(%title%, (Interlude),)` doesn't do what it looks like: the `)`
    after *Interlude* ends the function early, and OpenTag reports the
    wrong number of arguments. Quote the text that contains it:

    ```text
    $replace(%title%,' (Interlude)',)
    ```

    `Fog Horn (Interlude)` becomes `Fog Horn`. The same applies to commas:
    `$replace(%artist%,' & ',', ')` turns `Kofi & the Tidewater Band` into
    `Kofi, the Tidewater Band`.

## True and false: how conditions decide { #conditions }

Functions such as `$if`, `$if2`, `$and`, `$or`
and `$not` test whether something is *true*. OpenTag doesn't treat text like
`1` or `yes` as true. Truth comes from where a value came from:

- **A placeholder is true when the field has a value**, and false when it is
  missing or empty.
- **Plain text is never true**, not even `1`.
- **Comparison functions are true or false by their answer:** `$eql`,
  `$neql`, `$grtr`, `$geql`, `$less`, `$leql`, `$strcmp`, `$stricmp`, `$and`,
  `$or`, `$not`, `$odd`, `$isdigit`.
- **Most other functions are true when any of their arguments is**, so
  `$upper(%title%)` is true when `TITLE` has a value.

| Format string | With these values | Result |
|---|---|---|
| `$if(%year%,%year%,unknown year)` | `YEAR` = `2024` | `2024` |
| `$if(%year%,%year%,unknown year)` | `YEAR` not set | `unknown year` |
| `$if(1,yes,no)` | anything | `no` |
| `$if($eql(%year%,2024),this year,older)` | `YEAR` = `2024` | `this year` |
| `$if2(%albumartist%,%artist%)` | `ALBUMARTIST` not set | `Juniper Vale` (from `ARTIST`) |
| `$if($and(%artist%,%discnumber%),both,not both)` | `DISCNUMBER` not set | `not both` |

!!! warning "Gotcha: search functions return positions, not true or false"
    `$strstr(%title%,Water)` returns the position where *Water* starts
    (`17`), or `0` when it isn't found. Because `%title%` has a value,
    `$if($strstr(%title%,Snow),yes,no)` is **true** even though *Snow* isn't
    in the title. Compare the position instead:

    ```text
    $if($grtr($strstr(%title%,Snow),0),yes,no)
    ```

## Multi-value fields { #multi-value-fields }

A field can hold several values, for example two artists. A plain
placeholder only gives the first one. These functions give you the rest:

| Format string | With `ARTIST` = `Juniper Vale`, `Mira Solberg` | Result |
|---|---|---|
| `%artist%` | | `Juniper Vale` |
| `$meta(artist)` | | `Juniper Vale, Mira Solberg` |
| `$meta(artist,1)` | (counting starts at 0) | `Mira Solberg` |
| `$meta_sep(artist,' & ')` | | `Juniper Vale & Mira Solberg` |

Note that `$meta` takes the field **name**, without `%`. In the tag panel and
the file list, OpenTag shows multiple values joined with ` \\ `. That is only
a display convention; format strings never produce it. Storage of multiple
values depends on the tag format.

## Numbers inside text

Numeric functions read the whole number at the **start** of a value and
ignore the rest. `TRACK` = `2/8` counts as 2, and a year of `2024-05-01`
counts as 2024. Text that doesn't start with a digit counts as 0.

| Format string | With these values | Result |
|---|---|---|
| `$num(%track%,2)` | `TRACK` = `2/8` | `02` |
| `$num(%track%,3)` | `TRACK` = `2/8` | `002` |
| `$add(%track%,10)` | `TRACK` = `2/8` | `12` |
| `$num(%track%,2)` | `TRACK` = `B2` (a vinyl side) | `00` |

Use `$num` for padding and arithmetic functions such as `$add` for calculations.

## File and folder names { #file-and-folder-names }

When the result of a format string is a file name (in
Tag → Filename, and in
Format value or
Tag → Tag on `_FILENAME` or `_DIRECTORY`),
OpenTag applies three extra rules.

**1. `\` and `/` that you type make folders.** Both work on every platform,
and OpenTag uses the right one for your system. The path is relative to the
file's current folder, unless it starts at the root of a drive.

```text
%artist%\%album%\$num(%track%,2) - %title%
```

moves track 2 into `Juniper Vale/Harbour Lights/` under its current folder,
as `02 - Lanterns on the Water.mp3`.

**2. `\` and `/` that come from a value are replaced**, so a value can never
create a folder by accident. The replacement is `_` unless you change it under
**Options → Files → Replace folder separator in tag values with**.

**3. Characters that file systems reject are replaced with `_`.** These are
`:` `*` `?` `"` `<` `>` `|` and control characters, and the rule applies to
values only. Characters you type yourself are kept.

| Format string | With these values | Result |
|---|---|---|
| `%artist%\%title%` | `ARTIST` = `AC/DC`, `TITLE` = `Live: Encore` | `AC_DC\Live_ Encore` |
| `$replace(%artist%,/,-)\%title%` | `ARTIST` = `AC/DC` | `AC-DC\…` |
| `%genre%\%artist%` | **Replace folder separator** set to <code> - </code>, `GENRE` = `Folk/Rock` | `Folk - Rock\Juniper Vale` |
| `[CD$num(%discnumber%,1)\]%title%` | `DISCNUMBER` = `2` | `CD2\Lanterns on the Water` |
| `[CD$num(%discnumber%,1)\]%title%` | `DISCNUMBER` not set | `Lanterns on the Water` |
| `$if(%discnumber%,CD%discnumber%\,)%title%` | `DISCNUMBER` = `2` | `CD2\Lanterns on the Water` |
| `$upper('a/b')` | anything | `A_B` |

A few more details:

- **Conditions keep your folders.** The branch that `$if`, `$if2`,
  `$ifgreater` or `$iflonger` returns keeps the `\` you typed in it, as the
  sixth row shows. Other functions treat their result as a value, so in the
  last row the `/` inside `$upper(…)` becomes `_`.
- **The extension is kept.** OpenTag adds the file's extension, unless your
  result already ends with it, so `%title%.mp3` doesn't become `.mp3.mp3`.
- **An empty result is an error.** Nothing is renamed.

!!! info "Coming from Mp3tag"
    Mp3tag on Windows uses `\` for folders. OpenTag accepts `\` and `/` on
    macOS, Linux and Windows alike, so action groups and format strings you
    bring along keep working.

## Patterns: reading values out of text { #patterns-reading-values-out-of-text }

Filename → Tag and
Guess values use format strings in reverse.
You describe what the text looks like, and OpenTag works out the values.

```text
%track% - %artist% - %title%
```

applied to the file name `03 - Juniper Vale - Paper Boats.mp3` sets `TRACK`
to `03`, `ARTIST` to `Juniper Vale` and `TITLE` to `Paper Boats`.

Patterns have their own, simpler rules:

- Only placeholders and plain text. Functions, `[…]` and quotes have no
  special meaning in a pattern.
- **The whole name must match**, from start to end. If it doesn't, the file
  is reported and left unchanged.
- `%dummy%` matches text that you want to skip.
- Each placeholder matches as **little** as possible. With `%artist% - %title%`,
  the name `Juniper Vale - Salt - Cedar` gives `ARTIST` = `Juniper Vale` and
  `TITLE` = `Salt - Cedar`.
- Two placeholders right next to each other (`%track%%title%`) are an error,
  because OpenTag can't know where one ends.
- The same placeholder twice adds a second value: `%artist% feat. %artist%`
  gives a multi-value `ARTIST`.

Filename → Tag adds two rules for file names, which Guess values doesn't need:

- The extension is left out, unless your pattern ends with it.
- `\` in a pattern matches folder levels: `%artist%\%album%\%track% %title%`
  reads the two folders above the file too. Use `\` here on every platform.

For a worked folder pattern, see
[Tags from folder and file names](../cookbook/tags-from-folder-and-filename.md).

## The ► helper menu and history

Wherever you can type a format string, a **►** button sits next to the
input. Its menu inserts placeholders, technical fields and functions at the
cursor, and remembers the format strings you used before. Open it from the
keyboard with ++ctrl+space++ (Control on every platform; on a Mac,
++ctrl+option+space++ also works). In the history list, ++shift+del++
removes the highlighted entry.

## Errors and limits

A format string that can't be read is reported when you preview it, and
nothing is changed. The messages are:

| Message | Cause | Fix |
|---|---|---|
| Unterminated field placeholder | A `%` without its closing `%` | Close it, or write `'%'` for a percent sign |
| Empty field placeholder | `%%` | Write `'%'` |
| Unterminated quoted literal | An odd number of `'` | Close the quote, or write `''` for an apostrophe |
| Unterminated optional section / Unexpected closing optional bracket | Unbalanced `[` `]` | Balance them, or quote them: `'['` |
| Expected function call after $… | A `$` not followed by a name and `(` | Write `'$'` for a dollar sign |
| Unsupported scripting function: $… | A misspelt or unknown function | Check the function name in the ► helper menu |
| $… expects n..=m arguments | Too many or too few arguments, often from an unquoted `,` or `)` | Quote commas and brackets in arguments |
| $replace requires complete search/replacement pairs | An odd number of search and replace arguments | Add the missing replacement (it may be empty) |

Format strings can nest up to 128 levels deep and produce at most 1 MiB of
text per file. You'll only reach these limits by accident, for example with
a runaway `$repeat`.

## Worked examples

??? example "Track number and title, with the disc number for multi-disc albums"

    ```text
    [$num(%discnumber%,1)-]$num(%track%,2) %title%
    ```

    | With these values | Result |
    |---|---|
    | `DISCNUMBER` not set, `TRACK` = `2/8` | `02 Lanterns on the Water` |
    | `DISCNUMBER` = `2`, `TRACK` = `5` | `2-05 Salt & Cedar` |

??? example "Album artist folder with a fallback to the artist"

    ```text
    $if2(%albumartist%,%artist%)\%album%[ (%year%)]\$num(%track%,2) - %title%
    ```

    Files without an album artist are filed under their artist; the year is
    added only when it is known. Track 2 becomes
    `Juniper Vale/Harbour Lights (2024)/02 - Lanterns on the Water.mp3`.

??? example "A-to-Z folders by the first letter of the artist"

    ```text
    $upper($left(%artist%,1))\%artist%\%album%
    ```

    gives `J/Juniper Vale/Harbour Lights`.

??? example "A column that flags tracks longer than five minutes"

    Use this as the **Value** of a custom column:

    ```text
    $ifgreater(%_length_seconds%,300,long,)
    ```

    `Harbour Lights` (5:01) shows `long`; every other track shows nothing.

## Related

- [Placeholders](placeholders.md)
- [Regular expressions](regular-expressions.md)
- [Zero-padded track numbers](../cookbook/rename-zero-padded-track-numbers.md)
