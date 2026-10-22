# Export a CD catalogue

Build a readable album-and-track catalogue, then save a spreadsheet copy.
This lesson takes about 15 minutes and uses only practice files. You will see
how one template becomes a report before adding groups or multiple outputs.

- [ ] TagGroove desktop app installed, with copied audio files loaded.
- [ ] Pending edits saved or discarded, and the three practice files selected.
- [ ] A separate practice folder available for reports.

Use these tags and select the files in the order shown:

| Album artist / Artist | Album | Track | Title |
| --- | --- | --- | --- |
| Juniper Vale | Harbour Lights | 2 | Lanterns on the Water |
| Juniper Vale | Harbour Lights | 1 | Paper Boats |
| Mira Solberg | Low Tide | 1 | North Pier |

## 1. Make a copy of the text configuration

Choose **File → Export…** (++ctrl+e++, or ++cmd+e++ on a Mac). Select **Text**,
choose **Duplicate**, then **Rename** the copy to `Practice catalogue`.
Keep **Append data** and **Create one output file per** off.

Set **Export file name** to `practice-catalogue.txt` in your separate report
folder using **Browse…**. That location is the output; the configuration is
the reusable set of instructions.

## 2. Repeat one simple line

Replace **Export template** with:

```text
$filename(txt,utf-8)$loop(1)%artist% - %title%$char(10)$loopend()
```

Choose **Export**, then open the report in a text editor. It should contain:

```text
Juniper Vale - Lanterns on the Water
Juniper Vale - Paper Boats
Mira Solberg - North Pier
```

`$loop(1)` preserves the selected file-list order. The line break belongs
inside the loop so every track gets one. The original audio files are not
changed by this report.

## 3. Group albums and sort their tracks

Reopen Export, select `Practice catalogue`, and replace the template with:

```text
$filename(txt,utf-8)$loop(%album%)%album% — files: %_total_files%$char(10)$loop($num(%track%,3))  $num(%track%,2) %title%$char(10)$loopend()$loopend()
```

Export to the same practice report, keeping append off. The output becomes:

```text
Harbour Lights — files: 2
  01 Paper Boats
  02 Lanterns on the Water
Low Tide — files: 1
  01 North Pier
```

The outer loop chooses an album and writes its heading once. The inner loop
receives only that album's rows and sorts by padded track number. The two
spaces before `$num` indent the lines. `%_total_files%` in an album heading
counts that album, rather than the whole selection.

Albums with the same title would combine here. This practice set has distinct
titles; for a mixed library, add an album-artist grouping level or choose a
partition key that identifies each album edition.

## 4. Write a separate file for each practice album

Reopen Export and enable **Create one output file per**. Set **Partition
format** to `%album%`, then set **Export file name** to `%album%.txt` in the
same report folder. Keep the grouped template and append off.

Export and inspect `Harbour Lights.txt` and `Low Tide.txt`. Each report should
contain only its album's heading and tracks. Partitioning divides the input
into documents before the template renders. The album loop then sees one
album within each document.

The name must vary between groups. If both used `practice-catalogue.txt`,
TagGroove would reject the output collision rather than overwrite one album
with the other. Empty or inconsistent album values need attention too.

## 5. Save a spreadsheet copy

Reopen Export, turn **Create one output file per** off, select
**CSV (spreadsheet)** and choose `practice-catalogue.csv` in the report folder.
Export, then use the spreadsheet's text/CSV import command. Select UTF-8,
comma-separated fields and double quotes. Import track-like and identifier
columns as Text if their exact spelling must survive.

Check that there are three data records plus a header. This is a separate
configuration and output format. Editing the spreadsheet does not write tags
back to the audio files.

## What you learned

- A configuration stores the template; an output file stores its result.
- A loop repeats a row, and an inner loop can repeat tracks under one heading.
- A sort key controls ordering independently of printed track formatting.
- A partition creates separate reports and needs a distinct name for each.
- CSV import settings determine how a spreadsheet interprets text values.

## Next steps

- [Export controls, naming and append](index.md)
- [Template language, variables and totals](template-language.md)
- [CSV quoting and spreadsheet import](csv.md)
- [HTML and other report templates](template-gallery.md)
