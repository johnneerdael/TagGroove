# Export CSV

CSV stores a table as text: one record per track, with quoted fields separated
by commas. Use it to inspect or share metadata in a spreadsheet. Exporting
does not change the selected audio files, and editing the spreadsheet does not
automatically write values back to them.

## Use the built-in configuration

1. Save pending edits and select the tracks.
2. Choose **File → Export…** and select **CSV (spreadsheet)**.
3. Choose a destination ending in `.csv`, or leave **Export file name** empty
   to use the configuration's default extension.
4. Keep **Append data** off for one table with one header. Choose **Export**.

The bundled table contains Path, Title, Artist, Album, Album artist, Track,
Disc, Year, Genre and Length. It preserves file-list order, uses UTF-8 and CRLF
record endings, quotes every value and doubles embedded double quotes.

For a title `Paper, "folded" Boats`, the title cell is written as
`"Paper, ""folded"" Boats"`. The comma remains inside that cell; doubled quotes
represent one quote. A value containing a real line break can span physical
lines inside a quoted cell. Counting text-file lines is therefore not always
the same as counting CSV records.

## Import deliberately in the spreadsheet

Use the spreadsheet's text/CSV import command when double-clicking the file
guesses incorrectly. Choose UTF-8, comma as the delimiter and double quote as
the text qualifier. A program configured for semicolon-separated regional
CSV may otherwise put the entire record in one column.

Import tracks, disc numbers, identifiers and date-like strings as **Text**
when their spelling matters. Automatic conversion can turn `03` into `3`,
interpret `2/8` as a date, or round a long numeric identifier. CSV quotes keep
delimiters inside fields; they do not tell a spreadsheet which data type to
use. Check several records before saving another copy.

If accented names are misread, first select UTF-8 in the import dialog. Some
programs use a BOM to recognize UTF-8 when opening files directly. Enable
**Write BOM** in [Options → Export](../reference/options/export.md) if that
program requires it. BOM and delimiter are independent settings.

## Build a smaller three-column table

Duplicate a configuration before replacing its template. This one exports
Artist, Title and Track with an explicit header and CRLF records:

```text
$filename(csv,utf-8)"Artist","Title","Track"$char(13)$char(10)$loop(1)"$replace(%artist%,","")","$replace(%title%,","")","$replace(%track%,","")"$char(13)$char(10)$loopend()
```

The first quoted row is literal text outside the loop, so it appears once.
Each loop iteration wraps the field in double quotes. `$replace` changes every
embedded `"` to `""`; the surrounding CSV quotes are then added by the template.
Every field uses that rule, including `TRACK`: metadata remains editable text
even when it usually contains a number. For example, `TRACK = 03"take` becomes
the CSV cell `"03""take"`. Apply the same escaping when adding another field.

For one track with artist Juniper Vale, title `Paper, "folded" Boats` and
track `03`, the records are:

```text
"Artist","Title","Track"
"Juniper Vale","Paper, ""folded"" Boats","03"
```

To use semicolons, change the literal separators between quoted cells in both
the header and body. Keep the commas inside `$replace(...)`: those separate
function arguments and have a different purpose.

## Change ordering or split tables

`$loop(1)` keeps input order. Replace its key with `$num(%track%,3)` to sort
numerically by track within the selected set. For several albums, use a
grouping or partition strategy rather than interleaving every album's track 1.

**Create one output file per** can make a separate CSV per album. Give each
partition a distinct filename; the template emits a header for every new
document. Appending the same template repeats that header, which most
spreadsheet imports treat as another data record.

## Common problems

| Result | Remedy |
| --- | --- |
| Everything is in one column | Select the actual delimiter during import |
| Leading zeros disappear | Reimport the column as Text; changing display format after conversion may not recover the original spelling |
| Accents look garbled | Select UTF-8 or use the receiving program's required BOM |
| Columns shift after a quoted title | Double embedded quotes, and quote every arbitrary text cell |
| Header repeats halfway through | Replace the output instead of appending the full template |
| Changes in a spreadsheet do not appear in OpenTag | Export is one-way; a separate import workflow must map fields and match files deliberately |

See [export controls](index.md) and the [template language](template-language.md)
for partitioning, naming, loops and encodings.
