# Export template language

An export template mixes literal text, field placeholders and scripting
functions. A **loop** repeats a section for several files. Text outside a loop
is evaluated once, using the first input row where a field value is needed.
A report can therefore have one heading, many track lines and one summary.

The [format-string rules](../scripting/format-strings.md) apply to fields,
functions, optional sections and apostrophe quoting. Export adds commands and
counters below. It also accepts `%%` for one literal percent sign and `$$`
for one literal dollar sign. This export-specific escaping does not make `%%`
a valid ordinary tag-converter expression.

## Start with a repeatable row

For two selected files in the order `Paper Boats`, then `Lanterns on the Water`,
both by Juniper Vale, this complete template writes two artist/title lines:

```text
$filename(txt,utf-8)$loop(1)%artist% - %title%$char(10)$loopend()
```

The result has a line break after each title, including the last. Read it in
four pieces:

1. `$filename(txt,utf-8)` chooses the default extension and encoding.
2. `$loop(1)` starts a loop in the current input order.
3. `%artist% - %title%$char(10)` renders one row and an LF line break.
4. `$loopend()` closes the loop and prints nothing itself.

Literal template line breaks are also copied. `$char(10)` makes intentional
line breaks visible in a one-line template. Use `$char(13)$char(10)` for CRLF.
There is no automatic separator between iterations. Leading/trailing literal
spaces remain in the report.

## Command reference

| Command | Arguments and result |
| --- | --- |
| `$filename([name[,encoding]])` | Set the default output name/extension and optional encoding; print nothing |
| `$loop(key[,limit])` | Sort the current input set by a key and render its body; limit is per equal key |
| `$loopend()` | Close the matching loop; take no arguments |
| `$get(name)` | Return a stored export value, or empty when absent |
| `$put(name,value)` | Store a value and return it |
| `$puts(name,value)` | Store a value and return empty text |

!!! note "Variable text does not retain field presence"
    Variables store strings. Retrieving a stored tag value does not itself
    establish availability or truth. Use a comparison or a length test when
    the retrieved text should control a condition or optional section.

### `$filename`: name, extension and encoding

Put it once, before the first non-whitespace output. Do not put it inside a
loop. Leading whitespace can precede it but still belongs to the report.
A second or later `$filename` is an error.

| Template prefix | Default behavior |
| --- | --- |
| `$filename(csv,utf-8)` | Replace the current default extension with `.csv`; write UTF-8 |
| `$filename(catalogue.html,utf-8)` | Use this name beside the current default output |
| `$filename(reports/catalogue.txt,utf-8)` | Use the relative subfolder below the current default's folder |
| `$filename(,utf-16)` | Keep the default name; write UTF-16 little-endian |
| `$filename(txt,ansi)` | Choose `.txt`; encode through the ANSI codepage |

A bare argument without `.`, a slash, backslash or `:` is an extension, not a
complete filename. An absolute name selects an absolute location. Arguments
can use expressions evaluated in the first row of the output group.
**Export file name** overrides this name when nonempty, leaving its encoding
in effect.

Encoding names are `utf-8`, `utf-16` and `ansi`, ignoring case and surrounding
whitespace. Omitting the encoding argument selects ANSI: the system codepage
on Windows, and Windows-1252 on macOS/Linux. Bundled configurations already
name a Unicode encoding. Unrepresentable characters become `?`; use Unicode to preserve
all names. Template-file encoding and report encoding are independent.

### `$loop`: sort before rendering

Use a field or expression as the key:

```text
$filename(txt,utf-8)$loop($num(%track%,3))%track% - %title%$char(10)$loopend()
```

For tracks `10`, `2` and `1`, padded keys `010`, `002` and `001` give output
order `1`, `2`, `10`. Plain `%track%` is a text sort and can put `10` before
`2`. Current desktop export uses case-insensitive text keys; equal keys keep
their input order. `$loop(1)` gives ordinary rows one key, preserving file-list
order. A bare field name such as `$loop(artist)` also reads that field;
`%artist%` makes its purpose clearer.

The optional limit is a nonnegative integer evaluated in the parent context.
Omit it, use an empty result, or use `0` for all rows. Without a nested loop,
`$loop(%artist%,1)` prints the first row for each distinct artist key, rather
than only the first artist globally:

```text
$filename(txt,utf-8)$loop(%artist%,1)%artist%$char(10)$loopend()
```

Two Juniper Vale tracks and one Mira Solberg track produce one line per artist.
Only the first row of a key supplies ordinary fields. Consider that before
adding an album or year to a unique-artist summary.

### Nested loops: one heading per group

When a loop contains another loop, the outer loop renders once per key and
passes that key's entire member set to the inner loop. An album heading can
therefore be followed by all its tracks:

```text
$filename(txt,utf-8)$loop(%album%)%album% — files: %_total_files%$char(10)$loop($num(%track%,3))  $num(%track%,2) %title%$char(10)$loopend()$loopend()
```

For `Harbour Lights` with `Paper Boats` (track 1) and `Lanterns on the Water`
(track 2), plus `Low Tide` with `North Pier` (track 1), the result is:

```text
Harbour Lights — files: 2
  01 Paper Boats
  02 Lanterns on the Water
Low Tide — files: 1
  01 North Pier
```

The outer key chooses an album and its total is the album's member count. The
inner key sorts those members by track; its two leading spaces indent the
track lines. After the inner loop, the outer row and counter are restored.
Albums with identical names form one group even when their artists differ.
Add an artist level outside the album loop when that distinction matters.

### Variables: carry a result between steps

Variables exist for one output document. Names ignore ASCII letter case:
`count`, `Count` and `COUNT` name the same variable. Variables are not tags
and do not edit audio files. Each partition starts afresh.

```text
$filename(txt,utf-8)$puts(count,0)$loop(1)$puts(count,$add($get(count),1))%title%$char(10)$loopend()Rows written: $get(count)
```

After the three practice files, the final text is `Rows written: 3`. `$puts`
updates silently; `$put` would print its value as well. Only the text is
stored. `$get` retrieves that text without restoring the source field's
availability or truth. After `$puts(label,%title%)`, `[$get(label)]` is empty
and `$if($get(label),yes,no)` gives `no`, even with a stored title. Test text
length explicitly with `$iflonger($get(label),0,yes,no)` when that is the goal.

Arguments are evaluated eagerly. Field lookups or predicates inside a
variable name/value argument still affect evaluation flags; string-only
storage does not suppress those effects. A variable write in an "unused"
`$if` branch can still run. The same caution applies when a branch contains
an export loop. Build state changes as explicit steps in their intended order.
See the [condition rules](../scripting/functions/conditions.md) for nested
selectors and optional sections.

## Counters, totals and paths

| Placeholder | Meaning |
| --- | --- |
| `%_counter%` | One-based current iteration number; nested loops have their own counters |
| `%_max_counter%` | Iterations completed by the most recently finished loop; initially 0 |
| `%_total_files%` | Input rows in the current group |
| `%_total_size_raw%` | Sum of current-group file sizes in bytes |
| `%_total_size%` | Human-readable size using 1024-based units, with two decimals above bytes |
| `%_total_time_raw%` | Current-group duration in seconds, rounded after summing |
| `%_total_time%` | `HH:MM:SS`, with a day prefix after 24 hours |
| `%_total_time2%` | Duration with total hours, such as `25:01:01` |
| `%_filename_rel%` | Audio path relative to the actual report folder |
| `%_folderpath_rel%` | Audio folder relative to the report folder, with a trailing separator |

Outside loops, totals cover the output partition. Inside an iteration, totals
describe all rows with that loop key, including members omitted by a per-key
output limit. A unique-artist loop can finish with `_max_counter = 2` while
`_total_files = 3` after returning to the full report context.

Relative paths use the final destination, including a dialog override. For
`/Music/Harbour/01.mp3` reported in `/Reports/catalogue.txt`, the relative name
is `../Music/Harbour/01.mp3`. On Windows, a file on another drive or UNC share
stays absolute; there is no relative path between volumes.

`%_workingdir%` and `%_workingpath%` use the app’s working folder in both
report text and output-name expressions. They differ from each input audio
file’s own folder placeholders.

Normal [placeholders](../scripting/placeholders.md) and
[scripting functions](../scripting/functions/index.md) are available. Real file
hashes `%_crc%`, `%_md5%` and `%_md5audio%` require reading file data and can take
longer than tag-only output. Audio MD5 can be empty when the format has no such
value; hashes are not invented from tags.

## Troubleshoot in stages

Start with one row and field, then add a loop, sorting, grouping and formatting
separately. Inspect the saved report after each stage.

| Problem | Next check |
| --- | --- |
| `$loop without a matching $loopend` | Balance pairs from the inside out |
| Unexpected optional closing bracket | Quote literal `[` and `]`, especially for JSON |
| Wrong argument count | Quote literal commas or closing parentheses inside arguments |
| Empty replacement becomes an apostrophe | Leave the argument empty; `''` writes an apostrophe |
| Titles run together | Put an explicit line break inside the loop |
| Repeated album headings | Put the track loop inside the album loop |
| Template rejected for resource limits | Reduce nested work or split the selected input |

The renderer limits template size, nesting, output size and evaluation work.
These limits can reject an oversized report rather than hang the application.
They do not make every accepted template equally quick.
