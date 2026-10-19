# An HTML catalogue of your albums

<div class="recipe-meta" markdown>
**Uses:** Export, an export template and an existing folder image ·
**Works on:** copied local desktop files, one single-disc album per folder ·
**Time:** about 15 minutes for the first album ·
**Undo:** report export is not restored by tag-edit Undo; keep earlier reports and images separately
</div>

## The problem

You want an album page that opens in a browser and shows its cover and track
list. The tags supply the words; a separate image supplies the cover. An
HTML template writes a reference to that image. It does not extract a cover
from audio or put image bytes inside the page.

| Before | After |
| --- | --- |
| An album folder with saved tags and an embedded JPEG Front Cover | The same audio files, a separate `folder.jpg` and a `catalogue.html` page beside them |
| Track 2 appears before track 1 in the file list | The page lists tracks in padded numeric track order |
| An artist name contains `&` | The browser displays `&` as text because the HTML output encodes it safely |

Try this on one copied album first. Keep any existing `folder.jpg` and
`catalogue.html` somewhere separate before writing outputs with those names.
This recipe starts with a single-disc album whose track numbers and album
artist are consistent. It does not infer album editions or repair tags.

## The solution

### 1. Put the intended image beside the album

Save pending tag edits. Select one representative copied track with the
intended embedded JPEG **Front Cover** and follow
[Save embedded covers as folder.jpg](export-folder-jpg.md). For that action,
use output basename `folder`, cover type **Front Cover** and duplicate export
off. Inspect the action's result and the actual `folder.jpg`.

If the copied folder already has the right `folder.jpg`, use that image
instead of exporting another one. A **Folder image / Not embedded in the
file** display is not an embedded picture for the Export Cover action.

For a PNG source, the action writes `folder.png`; it does not convert it to
JPEG. Keep that real extension and make the PNG change in Variations below.
Choose the intended representative track rather than allowing different
embedded images to compete for one output name.

### 2. Make a separate HTML export configuration

1. Select every audio file in the copied album folder and check that only
   that album is selected.
2. Choose **File → Export…**. If a report runs immediately, turn **Show
   selection dialog** back on in [Export options](../reference/options/export.md).
3. Select **Text**, choose **Duplicate**, then **Rename** the copy to
   `Album page`.
4. Replace **Export template** with the complete template below.
5. Keep **Append data** and **Create one output file per** off.
6. After choosing the configuration, set **Export file name** to the relative
   name `catalogue.html`. With this one-folder selection, it resolves beside
   the album's first selected audio file.

```text
$filename(html,utf-8)<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Album catalogue</title>
<body>
<h1>$replace(%album%,&,&amp;,<,&lt;,>,&gt;)</h1>
<p>$replace($if2(%albumartist%,%artist%),&,&amp;,<,&lt;,>,&gt;)</p>
<p><img src="folder.jpg" alt="Album cover" width="240"></p>
<ul>
$loop($num(%track%,3))<li>$num(%track%,2). $replace(%title%,&,&amp;,<,&lt;,>,&gt;)</li>
$loopend()</ul>
<p>Tracks: %_total_files%</p>
</body>
</html>
```

The template uses only a fixed image filename. There are no tag-derived
image URLs or links to audio files, so spaces, quotes and path separators in
artist/title values cannot become URL syntax here. Keep the text expressions
in text content rather than moving them into an HTML attribute.

### 3. Write and inspect the page

Choose **Export** and read its result. Open `catalogue.html` in a browser.
For the guide's invented practice album, expect the Harbour Lights heading,
Juniper Vale credit, the folder image, and tracks such as `01. Paper Boats`
before `02. Lanterns on the Water`.

Check the first/last track, track count and image. Check an album with an
ampersand or angle bracket in its text too. The browser should display those
characters as text, rather than interpret the tag value as HTML.

The page is a snapshot. Later tag changes do not update it until you export
again. With append off, another export replaces the report. Tag-edit Undo
does not restore an earlier report, so keep any version you need separately.

## How it works

| Piece | Purpose |
| --- | --- |
| `$filename(html,utf-8)` | Select the default HTML extension and UTF-8 output. The explicit dialog filename still chooses this recipe's destination. |
| `%album%` | Supply the heading from the first input row. Select one consistent album for this recipe. |
| `$if2(%albumartist%,%artist%)` | Use the album artist, falling back to the first file's artist when absent. A fallback does not repair missing album-artist tags. |
| `$replace(value,&,&amp;,<,&lt;,>,&gt;)` | Encode text characters that could otherwise be interpreted as HTML markup. |
| `src="folder.jpg"` | Ask the browser to read that separate file from the page's folder. |
| `$loop($num(%track%,3))` | Sort and repeat track rows using a padded numeric key. |
| `$num(%track%,2)` | Format the printed track number independently of the sorting key. |
| `%_total_files%` outside the loop | Count all input rows in this report. |

Replace `&` first: later replacements create entities beginning with `&`.
Reversing that order could change `&lt;` into `&amp;lt;`, which displays the
entity spelling instead of the intended `<` character.

These replacements are for **HTML text content**. They are not a general URL
encoder, HTML-attribute encoder or JavaScript/CSS escaper. Keeping the image
path fixed avoids those additional contexts in this recipe. Do not invent a
function such as `$html()` or assume that a safe audio filename is already a
safe URL.

## Variations

### Use a PNG cover

Change only `src="folder.jpg"` to `src="folder.png"` after verifying that
`folder.png` exists beside the report. Renaming the extension alone does not
convert an image's bytes. Use one known basename/format consistently when
applying the recipe across several folders.

### Create a page in every album folder

Prepare a matching `folder.jpg` in each copied album folder, with one album
per folder. Select their audio files and use **Create one output file per**
with **Partition format**:

```text
%_folderpath%
```

Set **Export file name** to:

```text
%_folderpath%catalogue.html
```

Each output is rendered from the files in that folder, so its fixed image
reference still finds its neighboring cover. Inspect all written paths.
Do not send all groups to one fixed absolute filename: different reports
would collide. An album split across folders produces a separate page for
each folder with this partition.

### Include a multi-disc album in one folder

Replace the loop's sorting expression with a disc key followed by a track key:

```text
$loop($num(%discnumber%,3)$num(%track%,3))
```

Replace the printed `$num(%track%,2)` with
`$num(%discnumber%,1)-$num(%track%,2)` to make disc boundaries visible in the
text. Check disc numbers first: a missing or nonnumeric value can sort as
zero. Multi-disc folders and unusual track values need their own small trial.

## If the result is wrong

| Symptom | Check |
| --- | --- |
| Broken cover image | The real image's name/extension, its location beside the HTML, and whether the image opens independently. |
| Only one track appears | The selected files and both `$loop(...)` / `$loopend()` commands. |
| Two albums share one heading | The input selection or folder partition; this recipe expects one album per report. |
| The browser shows `&lt;` literally | The replacement order and whether text was already entity-encoded before export. |
| A copied page loses its image | Copy its image with it and preserve their relative location. |
| A report contains repeated whole pages | Turn **Append data** off and recreate the report from the template. |

Keep the HTML and its image together when moving a catalogue. Sharing a
report does not grant rights to redistribute its artwork, and it does not
make audio files available to the recipient.

## Related

- [Export controls and destinations](../export/index.md)
- [Export template language](../export/template-language.md)
- [Save embedded covers as folder.jpg](export-folder-jpg.md)
- [Export a catalogue step by step](../learn/export-a-cd-catalogue.md)
- [Template gallery](../export/template-gallery.md)

<!-- Candidate validation: validation/harness/guide_backend.rs (private drafting artifacts) executed this complete template and PNG, folder-partition/collision, multi-disc and missing/nonnumeric-disc variations through actual tag_core::render_export and encode_export_document. Five resulting HTML documents passed structural parsing and neighboring image-file checks; no browser rendering, native Export dialog or native report-writing path was exercised. $replace sequencing is from crates/tag-core/src/script.rs; existing guide tests also cover the same text-escaping expression. The fixed src path requires no dynamic filename/URL encoder. Keep draft pending integration, review and strict guide build. -->
