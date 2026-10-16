# Write a web source from local JSON

Build a source that reads an album and two track titles from JSON you control.
This lesson takes about 25 minutes. It keeps network data predictable while
you learn source headers, JSON selection, output buffers and settings.

- [ ] OpenTag desktop app with source installation/folder access available.
- [ ] Two copied practice audio files selected in this order: Paper Boats,
      then Lanterns on the Water; save pending edits first.
- [ ] Python available to serve a separate practice-data folder on localhost.
- [ ] A plain-text editor that can save UTF-8 without changing file extensions.

The local server serves only the practice folder and binds to `127.0.0.1`.
It is part of this exercise, not an instruction to run the source against a
personal music directory. The parser examples are also checked directly by
tests without launching OpenTag or making network requests.

## 1. Create one predictable response

Make a practice folder with a `release` subfolder. Save the following as
`release/harbour.json`:

<!-- tutorial-release-json:start -->
```json
{
  "album": "Harbour Lights",
  "artist": "Juniper Vale",
  "year": 2024,
  "tracks": [
    {"title": "Paper Boats"},
    {"title": "Lanterns on the Water"}
  ]
}
```
<!-- tutorial-release-json:end -->

In a terminal, serve that practice folder. Replace its path below. On Windows
PowerShell:

```powershell
python -m http.server 8766 --bind 127.0.0.1 --directory 'C:\PracticeCatalogue'
```

On macOS/Linux:

```sh
python3 -m http.server 8766 --bind 127.0.0.1 --directory "$HOME/PracticeCatalogue"
```

Open `http://127.0.0.1:8766/release/harbour.json` in a browser. It must show the
JSON above. Keep the terminal open; the server runs until stopped with Ctrl+C.
If the address fails, resolve the path/port problem before involving OpenTag.

## 2. Write the source description

Save this UTF-8 text as `Practice#&Harbour.src`, keeping its `.src` extension:

<!-- tutorial-source:start -->
```ini
[Name]=Practice catalogue
[AlbumUrl]=http://127.0.0.1:8766/release/%s.json
[SearchBy]=Release ID||harbour||%s
[Encoding]=url-utf-8
[Settings]=Practice#Settings.settings

[ParserScriptAlbum]=...
json "on"
OutputTo "ALBUM"
json_select "album"
SayRest
OutputTo "ARTIST"
json_select "artist"
SayRest
IfVar "includeYear" "true"
OutputTo "YEAR"
json_select "year"
SayRest
Endif
json_foreach "tracks"
OutputTo "TITLE"
json_select "title"
SayRest
Say "|"
OutputTo "TRACK"
json_foreach_counter
SayRest
Say "|"
json_foreach_end
```
<!-- tutorial-source:end -->

There is no index URL/script, so this is a direct lookup. The **Release ID**
field starts as the literal `harbour`. Its value replaces `%s` in `[AlbumUrl]`.
That `%s` belongs to URL construction; `%artist%` in a normal format string
has a different job.

`[ParserScriptAlbum]=...` begins a multiline command section. `json "on"`
parses the response. `json_select` puts a selected value on the parser's text
input, and `SayRest` appends that text to the current output buffer.
`OutputTo "ALBUM"` chooses that buffer; it does not write an audio tag yet.

The array loop changes the JSON context to each track. `TITLE` and `TRACK`
collect per-track values, each followed by `|`. The final pipe matters: it
marks a track-valued output rather than a single album-wide value. The
counter command puts its number on the input; `SayRest` is still needed to
copy it to output.

## 3. Give the optional year a setting

Save this JSON as `Practice#Settings.settings`:

<!-- tutorial-settings:start -->
```json
{
  "key": "practiceCatalogue",
  "title": "Practice catalogue settings",
  "settings": [
    {
      "type": "bool",
      "key": "includeYear",
      "title": "Import the release year",
      "description": "Include YEAR from the local response.",
      "default": false
    }
  ]
}
```
<!-- tutorial-settings:end -->

The root key identifies this source's saved settings. `includeYear` names one
setting. The schema stores a boolean; the parser receives it as `true` or
`false` text. `IfVar` compares that configured value and either fills `YEAR`
or skips that branch. The script does not depend on a field named includeYear
in the selected audio files.

## 4. Install and inspect the lookup

Choose **Tag Sources → Install tag sources…** and install both files. Open
**Tag Sources → Practice → Settings…**, leave the checkbox off and choose
**Save**. Then choose **Tag Sources → Practice → Harbour…**.

Confirm **Release ID** is `harbour`, then run the lookup. It uses the local
response directly. In the parser, the expected buffers are:

<!-- tutorial-buffer-results:start -->
- **ALBUM**: `Harbour Lights`
- **ARTIST**: `Juniper Vale`
- **TITLE**: `Paper Boats|Lanterns on the Water|`
- **TRACK**: `1|2|`
<!-- tutorial-buffer-results:end -->

`YEAR` is absent with the default setting. At review, check that the two local
files map to the matching source titles in the order intended. Select only
the proposed fields you want to apply. Do not rely on automatic duration or
ordering suggestions for this exercise; its response deliberately contains
no duration data.

After applying, inspect the tags and the result/errors. Successful parsing
and successful writing are separate checks. Keep the original audio untouched
elsewhere so a practice result can be compared safely.

## 5. Change one thing and observe the difference

Open **Practice → Settings…**, enable **Import the release year**, save and
repeat the lookup. The `YEAR` output should now be `2024`. The title/track
buffers remain the same. This change demonstrates a setting-controlled branch
without changing the response or the selected files.

To inspect the loop progressively, temporarily give the response's tracks
array one item, then restore the second item. Confirm that TITLE/TRACK gain
one aligned entry each. If an error names a command line, use its input/cursor
context rather than guessing at a different JSON field.

## Go further: a search list

A search-and-select source adds `[IndexUrl]`, `[IndexFormat]` and
`[ParserScriptIndex]`. The index script emits one line per result, with pipe
separators matching IndexFormat. `%_url%` identifies the album address;
`%_coverurl%` can provide a thumbnail and `%_preview%` a preview address.
The selected album then passes through the same album-parser stage.

Build the index separately from a known response before combining both
stages. For a runnable extension to this local exercise, save `search.json`
at the root of the practice folder:

<!-- tutorial-index-json:start -->
```json
{
  "releases": [
    {"path": "/release/harbour.json", "album": "Harbour Lights", "year": 2024}
  ]
}
```
<!-- tutorial-index-json:end -->

Remove the old AlbumUrl and SearchBy header lines from the working source,
keep its album parser/settings, then append this block:

<!-- tutorial-index-source:start -->
```ini
[AlbumUrl]=http://127.0.0.1:8766
[IndexUrl]=http://127.0.0.1:8766/search.json?q=%s
[IndexFormat]=%_url%|%Album%|%Year%
[SearchBy]=Album||Harbour Lights||%s

[ParserScriptIndex]=...
OutputTo ""
json "on"
json_foreach "releases"
json_select "path"
SayRest
Say "|"
json_select "album"
SayRest
Say "|"
json_select "year"
SayRest
SayNewline
json_foreach_end
```
<!-- tutorial-index-source:end -->

**Index row:** `/release/harbour.json|Harbour Lights|2024`

The row ends with CRLF from SayNewline. AlbumUrl plus the relative path points
to the same local album response used earlier. The search URL accepts the
encoded query; this small static fixture returns the same search.json for any
query. It demonstrates request/parsing structure rather than a real search
algorithm.

Reinstall the changed source and repeat the exercise. There is now a
result-list stage with Album and Year columns. Choose Harbour Lights to reach
the existing album parser. A year displayed in a search column is not itself
an instruction to import YEAR: the album parser's includeYear setting still
controls that output.

The [reference](web-source-reference.md) explains the headers, commands, URL
encoding and contexts used by both stages.

## What you learned

- Headers describe the source; parser commands process its downloaded text.
- JSON selection changes text input without immediately importing a tag.
- Output buffers distinguish album values from pipe-delimited track values.
- Settings are separate typed configuration, read by explicit commands.
- Review, track matching and writing require checks after the parser succeeds.

## Next steps

- [Install sources, settings and debug output](web-sources.md)
- [Header and parser-command reference](web-source-reference.md)
- [Format-string quoting and functions](../reference/format-strings.md)

Stop the practice HTTP server with Ctrl+C when finished. Keep a working copy
of the source before adapting its URL or parser for another response.
