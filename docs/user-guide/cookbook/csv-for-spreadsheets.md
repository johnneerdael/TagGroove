# Your library in a spreadsheet

<div class="recipe-meta" markdown>
**Uses:** File → Export and a CSV import in your spreadsheet · **Works on:** selected files in the current file list ·
**Time:** about 5 minutes for a sample · **Undo:** audio is unchanged; keep earlier reports because export can replace them
</div>

## The problem

You want a table of your music to inspect missing titles, share an album
list or compare tags outside OpenTag. Export creates a text report from
the selected files. It does not scan a whole collection automatically,
even when the Library cache is enabled.

| In OpenTag | In a carefully imported spreadsheet |
|---|---|
| Title `Paper, "folded" Boats` | One title cell containing that comma and those quotes |
| Track `03` | Text `03`, retaining the leading zero |
| Artist `Juniper Vale` | Its own Artist column |

## The solution

1. Save pending edits so the report represents your saved metadata. Select
   a small practice set first. Check the selection count and file-list order.
2. Choose **File → Export…**, then select **CSV (spreadsheet)**.
3. Set **Export file name** to a new destination ending in `.csv`. Choose
   a separate report location and filename; do not point it at audio,
   lyrics sidecars or a report you still need.
4. Leave **Append data** off and **Create one output file per** off for
   this first single table. Choose **Export**. It also saves the current
   export configuration and preferences.
5. In your spreadsheet, use its **text/CSV import** facility to open the
   report. Choose UTF-8, a comma delimiter and double quote as text qualifier.
   Exact dialog labels depend on the spreadsheet and platform.
6. Import columns whose spelling matters as **Text**, including Track,
   Disc, Year and identifiers. Inspect the preview before loading the table.
7. Check the first and last selected files, a quoted title, an accented
   name and a track with a leading zero. Save your working spreadsheet as
   a separate file if you will add calculations or notes.

The bundled export has Path, Title, Artist, Album, Album artist, Track,
Disc, Year, Genre and Length columns. It uses UTF-8, CRLF record endings
and the current file-list input order. **Write BOM** is a separate setting
under **Options → Export**; enable it if your receiving program needs a BOM
to recognize Unicode when opening the file directly.

## How it works

CSV protects a comma inside a cell by surrounding the cell with double
quotes. A quote inside that cell becomes two quotes. For example:

| Field value | Text written for that CSV cell |
|---|---|
| `Paper Boats` | `"Paper Boats"` |
| `Paper, "folded" Boats` | `"Paper, ""folded"" Boats"` |
| `03` | `"03"` |

Quotes describe the table structure; they do not force the spreadsheet's
data type. Automatic conversion can still turn `03` into `3`, `2/8` into
a date, or a long numeric identifier into a rounded number. Reimport as
Text if needed. Formatting a converted cell afterward may not recover its
original spelling. Treat arbitrary imported metadata as text when you do
not intend it to become spreadsheet formulas.

A multiline field can span physical text-file lines inside one quoted
cell. The number of lines in a text editor can therefore exceed the number
of table records. Use the spreadsheet's actual import preview to inspect
the table rather than splitting the file at every comma or line break.

## Variations

### Make a three-column table with a stable file key

The built-in report already includes Path. Keep a file key in a smaller
report too, especially if you will sort the spreadsheet. Duplicate an
export configuration and replace the copy's **Export template** with:

<!-- spreadsheet-template:start -->
```text
$filename(csv,utf-8)"Path","Title","Track"$char(13)$char(10)$loop(1)"$replace(%_path%,","")","$replace(%title%,","")","$replace(%track%,","")"$char(13)$char(10)$loopend()
```
<!-- spreadsheet-template:end -->

For `/Music/01.flac`, title `Paper, "folded" Boats`, track `03`, followed
by `/Music/02.flac`, title `Café at Low Tide`, track `04`, it writes:

<!-- spreadsheet-output:start -->
```text
"Path","Title","Track"
"/Music/01.flac","Paper, ""folded"" Boats","03"
"/Music/02.flac","Café at Low Tide","04"
```
<!-- spreadsheet-output:end -->

These example paths illustrate the template; real output uses the actual
audio paths on your platform. `$loop(1)` repeats the row in input order.
The literal header outside the loop appears once. `$char(13)$char(10)`
adds each CRLF record ending, and `$replace` doubles quotes in every cell.
Keep that escaping for every added arbitrary field, including Track.

### Use a semicolon delimiter

In the three-column template, replace the literal commas between quoted
cells in both header and body with semicolons. Keep the commas inside
`$replace(...)`; those separate scripting arguments, not CSV columns.
Choose semicolon in the spreadsheet's import dialog too. Regional defaults
do not automatically change OpenTag's comma-based built-in template.

### Repeat the report later

Use a new report name when you want a dated snapshot. Keep **Append data**
off for a complete replacement table: appending a whole CSV template writes
another header and repeats the selected records. Export does not deduplicate
tracks or merge spreadsheet changes. See the [export controls](../export/index.md)
before using partitioned reports for several albums.

## Can I edit the spreadsheet and import it back?

Editing or saving this report does not update the audio files. Keep a
separate original report, retain file keys and preserve text types while
planning changes. There is no automatic two-way link.

**Text File → Tag** can import deliberately prepared one-record-per-file
text using an explicit mask and filename/path keys. It does not decode
general spreadsheet CSV quoting, doubled quotes or multiline cells.
Do not feed the exported CSV back unchanged and assume a round trip.
Prepare a simpler unambiguous listing, try copied files, and inspect every
previewed association with the [tracklist recipe](tracklist-from-text-file.md).

## Troubleshooting

| Result | Next step |
|---|---|
| One huge column | Choose the actual comma or semicolon delimiter during import |
| Shifted columns around a title | Check quote escaping and the text qualifier; do not split at every comma |
| `03` became `3` | Reimport as Text from the original CSV |
| Garbled accents | Select UTF-8; use **Write BOM** if direct opening needs it |
| Missing files | Check which rows were selected at export time; the cache is not a collection-wide export source |
| Repeated headers | Start a new output or replace it with **Append data** off |
| Audio unchanged after spreadsheet edits | Export is a report; a separate reviewed import must write tags |

Export creates or replaces an external file immediately. **Edit → Undo**
for tag edits is not a general recovery mechanism for a replaced CSV.
Keep older reports you need before writing their paths again.

## Related

- [Export CSV reference](../export/csv.md)
- [Export template language](../export/template-language.md)
- [Export controls and configuration](../export/index.md)
- [Library cache](../guides/library.md)
- [Apply a tracklist](tracklist-from-text-file.md)
