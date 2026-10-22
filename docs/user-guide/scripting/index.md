# Scripting

Scripting is how TagGroove builds text from your tags. A **format string** such
as `$num(%track%,2) - %title%` becomes `03 - Paper Boats` for a file with
track 3 and that title. The same idea names files, calculates new tag values,
shapes file-list columns, writes export lines and names playlists. When you
need to recognize a *pattern* in text instead of building text, you add a
**regular expression**.

You do not need scripting to edit ordinary tags. Reach for it when one rule
should describe many files.

## Start here

Work through these in order on copied practice files. Each takes about
15 minutes and uses a live preview, so nothing changes until you apply it.

1. [Format strings from zero](format-strings-from-zero.md): read a field, add
   a function, choose a fallback when a field is missing.
2. [Regular expressions from zero](regex-from-zero.md): match text, capture
   parts of it and replace it.
3. [Build your first action group](../actions/first-action-group.md): put
   verified steps in a repeatable order.

## The pieces

| Piece | What it does | Example | Reference |
|---|---|---|---|
| **Placeholder** | Reads one value for each file | `%artist%`, `%_bitrate%` | [Placeholders](placeholders.md) |
| **Function** | Calculates text from its arguments | `$upper(%title%)`, `$num(%track%,2)` | [Functions](functions/index.md) |
| **Format string** | Combines text, placeholders and functions | `%artist% - %title%` | [Format strings](format-strings.md) |
| **Regular expression** | Describes a shape of text to find or replace | `^[0-9]+` matches leading digits | [Regular expressions](regular-expressions.md) |

A placeholder reads a value and a function calculates text. Neither decides
what to overwrite: the converter, action, column or export that runs the
format string supplies that purpose.

An ordinary tag-panel text box does not calculate anything. Typing
`$upper(%artist%)` there stores that text literally. Expressions are evaluated
only in the places listed below.

## Where scripting is used

| You want to… | Open | Scripting used |
|---|---|---|
| Rename or move files from tags | [Tag → Filename](../converters/tag-to-filename.md) | Format string builds the new path |
| Calculate a tag from other tags | [Tag → Tag](../converters/tag-to-tag.md) | Format string builds the new value |
| Calculate a field or name in an action | [Format Value](../actions/format-value.md) | Format string |
| Rewrite text by pattern | [Replace with Regular Expression](../actions/replace-regexp.md) | Regular expression |
| Split structured text into fields | [Guess Values](../actions/guess-values.md) | Pattern with placeholders |
| Show a calculated value in the list | [Customize file-list columns](../customize/columns.md) | Display format string |
| Write a report or catalogue | [Export template language](../export/template-language.md) | Placeholders, functions and loops |
| Show only matching files | [Filter syntax](../reference/filter-syntax.md) | `MATCHES` takes a regular expression |
| Find text in the list | [Find](../tagging/find.md) | Optional regular expression |
| Read or transform downloaded text | [Parser reference](../tag-sources/web-source-reference.md) | Parser commands with regular expressions |

Import patterns, such as those in
[Filename → Tag](../converters/filename-to-tag.md), describe exact separators
and destination names. They do not run functions.

## Look up

- [Format strings](format-strings.md): every special character, conditions,
  optional sections and how file names are made safe.
- [Placeholders](placeholders.md): tag fields and every `%_information%` value.
- [Functions](functions/index.md): all 64 built-in functions in six families.
- [Regular expressions](regular-expressions.md): syntax, replacements and how
  TagGroove differs from other tools.

## Recipes that use scripting

- [Zero-padded track numbers in filenames](../converters/recipes/rename-zero-padded-track-numbers.md)
- [Lowercase names with hyphens](../converters/recipes/rename-lowercase-filenames.md)
- [Sort a leading “The” without changing the artist](../converters/recipes/the-artist-sort-names.md)
- [Keep only the year](../actions/recipes/year-from-date.md)
- [Remove a trailing remaster annotation](../actions/recipes/remove-text-in-brackets.md)
- [Capitalization with exceptions](../actions/recipes/fix-capitalisation.md)
