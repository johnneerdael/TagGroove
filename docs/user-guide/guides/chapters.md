# Chapters and cue sheets as rows

A chapter row represents part of a containing file. A CUE-track row represents
a track described by a cue sheet. Neither is a newly split audio file. This
distinction matters when editing a title, creating playlists, or choosing a
command that acts on the whole containing file.

Use copied local files and sheets first. These instructions describe current
TagGroove source behavior; they do not establish every container/player's
support or a native application test on each platform.

## Show embedded chapters as rows

1. Keep a copy of a file with recognized chapter information. Save or discard
   pending edits deliberately before changing the view and reloading.
2. Open **File → Options… → Tags → Advanced** and enable **List chapters as
   separate files**, which is off by default.
3. Apply the options and reload the directory. TagGroove expands files with at
   least two recognized chapters into chapter-order rows. A file with fewer
   than two stays an ordinary file row.
4. Inspect a row's title and length, then compare the whole-file view by
   disabling the option and reloading again.

Recognized sources include MP4 Nero/QuickTime chapters, Matroska chapters,
ID3v2 CHAP frames, and cue sheets embedded in a `CUESHEET` field where supported.
Each row starts from file fields, overlays that chapter's own fields and
uses its chapter title. The same physical filename can therefore appear on
several rows. A row duration describes its segment, not a separate file size.

The information placeholders `%_subsong%` and `%_subsong_count%` help identify
these rows. The index is zero-based; the first of two rows has index 0 and
count 2. MP4 rows without explicit track numbers can be given row-based
`TRACK`/`TOTALTRACKS` values by the read model. Do not infer that new physical
tracks were created or that every format stores those synthesized values.

## Edit a chapter title deliberately

1. Select one copied chapter row and open **Extended tags…** (++alt+t++;
   Alt is ⌥ Option on macOS).
2. Change `TITLE`, then choose **Keep changes**. That stages text edits.
3. Choose **Save tags** to write the appropriate chapter metadata, inspect
   errors, and read the saved file back.

A title edit uses the existing chapter positions; it is not a timing editor
or an audio split. Check read-back positions alongside the title. Clearing
the title does not mean deleting the chapter. Other field changes can affect
shared storage:
for supported chapter formats, equal values across every row consolidate
as a file field, while differing values belong to the individual chapters.
Only changed fields are considered for that redistribution; format-specific
constraints can still reject a requested field or value.

`CHAPTERS` and `CUESHEET` are structural whole-file fields. A chapter row
refuses editing them as though they were an ordinary chapter label. Return to
the whole-file view for supported structural editing. MP4/Matroska `CHAPTERS`
values use time-and-title entries such as `00:00:01.000 Low Tide`; this is not
a universal way to create ID3 CHAP frames in every audio format.

## Load an external CUE sheet

Choose **File → Load playlist/cue sheet…** and select the copied `.cue` file.
This loader lists its AUDIO tracks with `INDEX 01` as rows, even when the
embedded-chapter display option is off. A sheet with one audio file per track
still represents cue-sheet tracks; it does not turn those rows into direct
edits of the individual audio files.

Here is a complete small illustration for a three-second `image.wav` beside
the sheet. `INDEX` uses minutes, seconds and 1/75-second CD frames, not
milliseconds. This example's second track starts at one second.

<!-- cue-guide-source:start -->
```text
REM COMMENT "Practice copy"
PERFORMER "Juniper Vale"
TITLE "Harbour Lights"
FILE "image.wav" WAVE
  TRACK 01 AUDIO
    TITLE "Paper Boats"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Low Tide"
    PERFORMER "Mira Solberg"
    INDEX 01 00:01:00
```
<!-- cue-guide-source:end -->

The expected rows are below. **Index** is `%_subsong%`, and **Count** is
`%_subsong_count%`.

Scroll this example table sideways on a narrow screen for the information columns.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Cue track example rows">

<!-- cue-guide-rows:start -->

| Track | Title | Artist | Album | Index | Count | Seconds |
|---|---|---|---|---|---|---|
| `1` | `Paper Boats` | `Juniper Vale` | `Harbour Lights` | `0` | `2` | `1` |
| `2` | `Low Tide` | `Mira Solberg` | `Harbour Lights` | `1` | `2` | `2` |

<!-- cue-guide-rows:end -->

</div>

The first row inherits the album performer because it has no track performer.
The second uses its own. Timing ends at the next applicable `INDEX 01` in the
same source file, or at the source file's end. A pregap is not an additional
file produced by this view. Missing FILE references and unsupported FILE URLs
are reported. DATA tracks and AUDIO tracks without INDEX 01 are not listed;
this view does not download or reconstruct their audio.

## Know which file a CUE edit changes

Changing the first row's `TITLE` to <!-- cue-title-edit -->`At the Pier`, keeping changes and saving
writes that TITLE line in the `.cue` file. It does not retitle `image.wav`.
A shared `ALBUM` edit to <!-- cue-album-edit -->`Harbour Sessions` changes the sheet's album TITLE
line. After reloading, both rows read that shared value. Different requested album values for
two rows cannot coexist in that one shared line.

CUE fields have defined storage, not arbitrary custom-field capacity:

| Scope | Supported field families |
|---|---|
| Album lines | `ALBUM`, `ALBUMARTIST`, `GENRE`, `YEAR`, `COMMENT`, `DISCNUMBER`, `TOTALDISCS`, album ReplayGain gain/peak. |
| Track lines | `TITLE`, `ARTIST`, `COMPOSER`, `ISRC`, track ReplayGain gain/peak. |
| Track number | Comes from TRACK lines. Requesting a different number through a tag edit is refused. |

Unsupported field names are refused. Album-scope edits in one batch must
agree. Original line order, unedited text and the detected encoding are
preserved by the sheet editor; newly written values follow its line/quoting
rules. Multi-line values are flattened for a cue line. New text that cannot
round-trip in a legacy encoding can fail the staged write; inspect the error
and retain the original rather than assuming the characters were stored.
Keep the original sheet because this is a metadata editor, not a general
text-history system.

## Commands that need the containing file

Some commands are unavailable for rows because they rename or list physical
files. Other commands can affect the whole containing file, even when only
one chapter row is selected. Read the operation label and scope first.

<!-- chapter-command-cases:start -->

| Row kind | Command | Scope or restriction |
|---|---|---|
| Embedded chapter | **Tag → Filename…** | Unavailable for the selected row. |
| Embedded chapter | **Playlist (selected files)** | Unavailable for the selected row. |
| Embedded chapter | **Playlist (all files)** | Unavailable when a chapter row remains in the loaded list. |
| Embedded chapter | **Remove tag…** | Acts on the containing file. |
| Embedded chapter | **Manage covers…** | Acts on the containing file. |
| CUE track | **Remove tag…** | Refused; the row edits the cue sheet. |
| CUE track | **Play** | Resolves to the referenced audio file. |

<!-- chapter-command-cases:end -->

Return to ordinary whole-file rows before playlist creation, renaming, moving,
copying or deleting audio. To do so for embedded chapters, disable the chapter
option and reload the directory. For an external CUE view, load the actual
audio files/directory instead of the sheet. A Play/reveal command resolving a
CUE source does not export its segment as a new recording.

## MP4 Nero metadata and artwork

Chapter-row field changes use MP4 field mappings and also update or remove
matching fields already present in the file's Nero global metadata. iTunes
metadata takes precedence, with Nero filling missing values. This prevents
a deleted value from reappearing from its Nero copy after saving.

Nero artwork is shown only when iTunes artwork is absent. A cover edit writes
the resulting covers to the iTunes artwork box and removes the Nero artwork
copy; a normal text-field edit preserves that copy. Unrelated MP4 index data
is retained. These rules do not add a separate Nero artwork editing mode.

## Remove MP4 chapters only when intended

**Tools → Strip chapters MP4…** operates on selected MP4 files with recognized
chapters and asks before writing. With chapter rows selected, the containing
file is the target. CUE-track rows do not support that whole-file command.
It removes chapter structures rather than merely hiding rows; the UI explains
that their boxes become free space while audio, video and ordinary tags stay.
This is not the same as clearing one chapter title or disabling the display
option, and it does not apply to every chapter format.

Inspect the returned file/row state after any whole-file operation. A stripped
file can replace its earlier chapter rows in the list. For recorded desktop
file edits, inspect **Edit → Undo** and its label; keep copied files and sheets
for longer-term recovery and failures. Do not assume generated external
playlists or every side effect are included in that history.

## Related

- [Playlists and cue sheets](playlists.md)
- [Extended tags](extended-tags.md)
- [Advanced tag options](../reference/options/tags-advanced.md)
- [File format capabilities](../reference/file-formats.md)
- [Working with covers](covers.md)

<!-- verified: playlist_chapter_guide.rs reads/edits the actual written external CUE against a generated WAV, checks row timing/shared fields/errors and unchanged audio bytes. guide-playlist-chapters.test.mjs executes actual row guards/replacement. Broader chapter/Nero/strip semantics source-checked; no native UI/player/all-format roundtrip claim. -->
