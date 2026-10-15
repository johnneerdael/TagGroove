# Template gallery

Copy a complete template into a duplicated export configuration, select a few
practice files and inspect the written output before expanding the selection.
Each example chooses UTF-8 and uses explicit line breaks. Leave **Export file
name** empty to use its default, or enter a deliberate override. See the
[template reference](template-language.md) for commands and scopes.

## Numbered track list

```text
$filename(txt,utf-8)$loop($num(%track%,3))$num(%track%,2). %title% — %artist%$char(10)$loopend()Files: %_total_files%; duration: %_total_time%
```

Select one album. Padded track keys make track 2 precede track 10; `$num` in
the body formats the printed number separately. The summary is outside the
loop and therefore covers the full report. For multi-disc albums, sort by a
padded disc key followed by a padded track key.

## One line per album

```text
$filename(txt,utf-8)$loop(%albumartist%)$loop(%album%,1)$if2(%albumartist%,%artist%) — %album%[ (%year%)]$char(10)$loopend()$loopend()
```

The outer level groups album artists. The inner level emits only the first
row of each album key. Optional year punctuation disappears when that row's
`YEAR` is missing. Complete `ALBUMARTIST` consistently first: a missing album
artist forms one empty outer group, even though the printed name falls back
to `ARTIST`. Also distinguish separate editions before using a title as a key.

## A small HTML catalogue

```text
$filename(html,utf-8)<!doctype html><meta charset="utf-8"><title>Track catalogue</title><h1>Track catalogue</h1><ul>$char(10)$loop(1)<li>$replace(%artist%,&,&amp;,<,&lt;,>,&gt;) — $replace(%title%,&,&amp;,<,&lt;,>,&gt;)</li>$char(10)$loopend()</ul><p>Files: %_total_files%</p>
```

For artist `Kofi & the Tidewater Band` and title `Low <Tide>`, the HTML contains
`Kofi &amp; the Tidewater Band — Low &lt;Tide&gt;` inside a list item. A browser
displays the original characters as text instead of interpreting a tag.
The replacements process `&` first so later entities do not get escaped twice.

This example places values only in text content. HTML attributes, links,
JavaScript and CSS have different escaping requirements; do not move the same
expression into those contexts without changing the design. No covers are
embedded or extracted by this template. An image element requires an actual
image file at the referenced path.

## A CSV table

Use **CSV (spreadsheet)** for a complete table, or start with the smaller
worked template on the [CSV page](csv.md). Quote fields and double embedded
quotes; replacing commas alone is not a CSV encoder. The built-in CSV export
already performs the necessary field quoting.

## JSON and playlists

The bundled **JSON** configuration writes an array with a per-file object.
It escapes backslashes, quotes, line breaks and tabs in its text fields.
Literal array brackets are quoted because `[` and `]` are format-string
syntax. Validate the resulting JSON with the receiving program, especially
after adding new fields or unusual control characters. A hand-written report
that merely resembles JSON is not a substitute for a valid structured file.

The bundled **Extended M3U playlist** uses relative audio paths, durations and
display titles. **Cue sheet** lists one FILE/TRACK entry per selected file in
input order. A cue sheet does not combine audio files or infer musical track
boundaries inside a long recording.

## Adapt the examples progressively

1. Verify one field on one practice file.
2. Verify the repeated row on two files, including a title with punctuation.
3. Add sorting, then grouping, and check the first and last row of each group.
4. Change naming or partitioning only after the report body is correct.
5. Test the actual receiving program and its encoding/import settings.

Keep the unmodified copy of a working template. A template that renders
without an error can still choose the wrong grouping key, repeat headers,
point to nonexistent images or produce a format the receiver cannot parse.
