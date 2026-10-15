# Chapter fields: per-chapter tags, cue sheets across files, whole-file commands on rows, subsong placeholders

Scope: the remaining gaps of [chapters](chapters.md). That covers fields other than TITLE stored per chapter, how row edits are split between the file and its chapters, cue sheet tracks that span FILE lines and their pregaps, FLAC cue sheets, whole-file commands on chapter rows, and `%_subsong%` / `%_subsong_count%`.

References: `customization/options/tags/advanced` (List chapters as separate files, `%_subsong%`, `%_subsong_count%`), `format`, `playlists`, `file-formats`, and the release notes for 2.84 (Nero-style MP4 chapters and sub-track tags). Where the documentation is silent, Mp3tag 3.36.1 (`Mp3tag.exe`) was examined statically for its behaviour and data layouts. It was never run. No code or text was taken from it.

Nothing was launched on a desktop. The evidence is `cargo test` and `node --test` (all passing locally) and the headless CI suites `tests/e2e/chapter-fields.js` and `tests/e2e/chapters.js`.

## How row edits are stored

The 2.84 release notes give the rule: fields with the same content on every sub-track are written as file tags. The examined `Mp3tag.exe` follows it on write. File-wide fields are recomputed as the fields that are present and equal in every subsong. Each chapter then stores the fields where it differs.

OpenTag applies the same rule when chapter rows are saved (`crates/tag-core/src/subsongs.rs`, `consolidate`):
- **Inputs.** A row's view is the file's fields overlaid with the chapter's own fields. The edits are applied to the views of the saved rows. Only the edited field names are considered, so fields nobody touched keep their current storage.
- **Same value on every row.** The field becomes a file field, and any per-chapter copies are removed.
- **Different values.** The file loses the field, and every chapter stores its own value. A chapter with an empty value stores nothing.
- **TITLE.** Always the chapter's title, never a file field.

This replaces the old refusal ("… is stored once for <file>, not per chapter") for MP4, Matroska and ID3v2 rows. It also changes single-row saves: a field typed into one row now belongs to that chapter, while before it went to the file and so to every row. Cue sheet rows are unchanged: their track lines hold TITLE, ARTIST, COMPOSER, ISRC and the track ReplayGain values, and album lines must still agree.

## Where each format stores per-chapter fields

### MP4: Nero tags

Mp3tag reads and writes Nero tags in `moov/udta/tags` (`crates/tag-core/src/nero_tags.rs`, `mp4_chapters.rs`, `mp4_layout.rs`).

**Layout**, as the examined binary reads and writes it:
- The `tags` box holds a `meta` box with file-wide fields, a `cvrx` box with covers, and one `tseg` box per chapter.
- Each `tseg` is a `tshd` box holding a 32-bit big-endian chapter index, then a `meta` box.
- The index is 0-based and refers to the chapter list in use.
- A `meta` payload has no version or flags. It is a 32-bit field count, then per field:
  - a flag byte: 0x80 for text, 0x00 for the `replaygain_*` names;
  - a zero byte;
  - the name, prefixed with its length;
  - a 16-bit value count;
  - per value, a 32-bit length and UTF-8 bytes.
- Fields with other flag bytes are skipped by Mp3tag. OpenTag keeps them byte for byte.

**Names.**
- On read, `date` is YEAR, `track`/`tracknumber` is TRACK, `tempo` is BPM, `disc`/`discnumber` is DISCNUMBER and `lyrics` is UNSYNCEDLYRICS. Other names are upper-cased.
- On write, names are lower case; UNSYNCEDLYRICS becomes `lyrics`, BPM `tempo`, TRACK `track` and DISCNUMBER `disc`.

**Reading.**
- Chapter *i* of the CHAPTERS list gets the fields of `tseg` *i*. A `tseg` title is not shown, because the chapter list supplies the title.
- File-wide Nero fields fill in fields the iTunes tag does not have.

**Writing.**
- When a chapter needs fields of its own, or the file already has Nero tags, every chapter's set is written together with its title, as Mp3tag writes it.
- The box goes after the Nero chapter list in the same `moov` rewrite that writes the chapters. That rewrite is in place when it fits, and every chunk offset is verified.
- The covers box, unknown boxes and the file-wide `meta` are kept.
- The write is checked by reading the `tags` box back.
- A title-only edit of a file without Nero tags still writes no `tags` box. This is more conservative than Mp3tag, which writes Nero tags for every save of a file with at least two subsongs.

**Remove tag** on an MP4 file now also turns its Nero tags into a `free` box of the same size. Mp3tag clears its iTunes and Nero tag objects together.

**Numbering.** Following the examined binary, MP4 sub-tracks without TRACK or TOTALTRACKS show their number and the chapter count. These are display defaults and are written only when edited.

### Matroska: chapter-targeted tags

Tags whose Targets carry a TagChapterUID are the fields of that chapter's row (`crates/tag-core/src/matroska.rs`).
- **Names.** A SimpleTag at target type 30 maps through the documented mapping where one exists. Otherwise its upper-cased name is the field name.
- **TITLE.** The chapter's display string. A TITLE SimpleTag inside a chapter tag is neither shown nor changed.
- **Writing.** Changes go into the existing tag for that UID, or a new `Tag` with TargetTypeValue 30 and the TagChapterUID. The file fields, chapter titles and chapter tags are written in one rewrite of the Tags and Chapters elements; clusters never move. The read-back is verified per chapter.
- **Cleanup.** Removing chapters through CHAPTERS drops the tags of chapters that no longer exist. Remove tag drops chapter tags together with the chapters.
- **Not supported.** A chapter without a UID cannot hold fields of its own, and such a save is refused.

### ID3v2: CHAP sub-frames

The sub-frames of a `CHAP` frame are read as a tag of their own through the normal ID3v2 field mapping (`subsongs.rs`, `id3_chapters` and `write_id3_rows`).
- **Read.** TIT2 is the title, TIT3 is SUBTITLE, TPE1 is ARTIST, WXXX is WWW, and so on.
- **Write.** Changes are applied with the same frame patching as file tags, in the configured revision for MPEG files (as every MPEG save does) and in the existing revision otherwise.
- **Pictures.** APIC and other sub-frames are kept but not shown. Row covers are the file's.
- **Verification.** The file fields and the chapter frames are written in one staged write, and each edited chapter is read back and checked.

The examined Mp3tag 3.36.1 counts subsongs only for MP4, Matroska and cue sheet files; its MP3 reader has no subsong support. ID3v2 chapter rows and rows for a CUESHEET field are OpenTag extensions that were added earlier and kept.

## Cue sheets

### Tracks across FILE lines and pregaps

(`crates/tag-core/src/cue.rs`)
- **Track file.** INDEX lines after a FILE line still belong to the last TRACK. A track plays from the FILE of its INDEX 01, so a track whose pregap (INDEX 00) ends one file and whose INDEX 01 starts the next lists the second file.
- **Start.** A track starts at its INDEX 01.
- **End.** A track ends at the INDEX 01 of the next listed track on the same file, so a pregap belongs to the track before it; otherwise it ends with its file. This matches what the examined binary does.
- **Pregap.** The INDEX 00 time is kept with the track.
- **Listing.** Only AUDIO tracks with an INDEX 01 are listed. Data tracks and tracks without INDEX 01 are skipped, as in Mp3tag.

### Every track is a row

The examined binary lists a loaded cue sheet as subsongs of the sheet, even when each FILE holds one track (`crates/tag-core/src/playlist.rs`). OpenTag now does the same: a cue sheet exported by OpenTag loads back as track rows that show and play their files, and editing them writes the sheet. Earlier, a FILE with one track loaded the file itself.

### FLAC CUESHEET block

The examined binary has no subsong support for FLAC; the `CUESHEET` name there belongs to the bundled FLAC library's block names. The binary FLAC CUESHEET block is therefore not listed as tracks, matching Mp3tag.

## Whole-file commands on chapter rows

**Run once on the row's file** (`src-tauri/src/lib.rs`, `whole_file_batch`; `src/chapters.ts`):
- Remove tag, Remove tag types, Strip chapters MP4, Remove unsupported fields, Optimize, Add cover, Remove cover, Manage covers and dropped covers.
- The file comes back as its rows, or as the file when it no longer has chapters (for example after Strip chapters MP4).
- The list replaces that file's rows in place (`replaceFileRows`), and the selection moves with them.
- Undo brings the rows back in place of the file.

**Pasted or edited covers** saved with rows are applied to the file once, when the rows that bring covers agree.

**Show in file manager and Play** use the audio file of a row, including the FILE of a cue sheet track (`tag_core::subsong_file`).

**Still refused for chapter rows:**
- Tag → Filename, Filename → Filename, copy/move/delete files and Save a copy. A chapter is not a file of its own; renaming or moving it would rename or move every other row's file.
- Playlist (selected/all files) and Generate playlists. A row is not a playlist entry. Mp3tag has some subsong-specific path handling, but its entry format is unknown.

**Rows of a loaded cue sheet** edit the sheet, so whole-file commands are refused for them with their own reason.

## `%_subsong%` and `%_subsong_count%`

The examined binary keeps a 0-based subsong index. A file without chapters is subsong 0 of 1, and a file with fewer than two chapters is a single subsong.

OpenTag now does the same (`src-tauri/src/evaluation.rs`, `subsongs.rs`):
- Chapter and cue sheet rows have `%_subsong%` from 0.
- Every file without chapters has `%_subsong%` 0 and `%_subsong_count%` 1.
- Files with at least two chapters have `%_subsong_count%` equal to the chapter count, whether or not they are listed as rows.
- Files with one chapter are not split into rows.

This changes the earlier, unverified 1-based `%_subsong%`. Whether Mp3tag's scripting adds one before display was not found.

## Evidence

**`crates/tag-core/src/nero_tags.rs`**
- Segments round-trip byte for byte, with the flag bytes, the 0-based `tshd` index and lower-case names.
- Covers and odd fields are kept, names map as listed above, values end at NUL, and malformed boxes are rejected.

**`crates/tag-core/src/subsongs.rs`**
- `consolidate`: equal values become file fields and leave the chapters; differing values leave the file and are stored with each chapter.
- MP4 rows are numbered.
- A single chapter is not a subsong.

**`crates/tag-core/src/cue.rs`**
- A track whose pregap ends `01.wav` plays from `02.wav`.
- Track ends use the next INDEX 01.
- The data track is skipped, and an edit renders back unchanged except for its line.

**`crates/tag-core/src/playlist.rs`**: every AUDIO track with INDEX 01 is a row, including one-track FILEs.

**`crates/tag-core/tests/mp4_chapters.rs` (ffmpeg M4B)**
- One row's ALBUM is stored in a Nero `tseg` (the bytes contain `0x80 0x00 0x05 album`); the file has none, and ffprobe still lists the chapters.
- GENRE equal on both rows becomes an iTunes field. GENRE A/B is stored per chapter, and ALBUM then equal on both rows moves back to the file.
- Remove tag removes the Nero tags and keeps the chapters.
- Audio packets are unchanged throughout.

**`crates/tag-core/tests/chapter_rows.rs`**
- MP3s with ID3v2.3 and v2.4 `CHAP` frames:
  - ARTIST, SUBTITLE and WWW that differ between rows are stored in the CHAP sub-frames, and ALBUM equal on both rows goes into the tag.
  - Making ARTIST equal moves it back into the tag.
  - Chapter times stay as ffprobe lists them, and the audio is unchanged.
- An ffmpeg MKA: a COMMENT on one row becomes a chapter tag, and the same COMMENT on both rows becomes a file tag. ffprobe still lists both chapters, and the audio is unchanged.
- A cue sheet across two FLAC files: each track is 2.0 s and plays from its own file, and `subsong_file` resolves it.

**`crates/tag-core/tests/containers.rs`** (hand-built nested MKA)
- Chapter tags are bound to the right UID, and the tree and clusters are unchanged.
- Equal values on all five rows move into the file.
- Removing a chapter removes its tags.

**`src-tauri/src/lib.rs`**: whole-file commands run once per file for any number of its rows, and are refused for cue sheet rows.

**`src-tauri/src/exports.rs`**: the cue sheet export loads back as track rows that resolve to their files.

**`tests/chapters.test.mjs`**: the new command rules and `replaceFileRows` (unchanged rows, saves, Strip, fewer chapters, Undo).

**`tests/document-coordination.test.mjs`**: `accept()` with the row splicing, unchanged provider behaviour.

**`tests/e2e/chapter-fields.js`** (new, headless in CI on Linux, macOS and Windows; fixtures from `chapter-fields-fixtures.sh`)
- `%_subsong%/%_subsong_count%` is `1/2` for a chapter row and `0/1` for a plain file.
- An ARTIST typed for one MP3 chapter row stays with it.
- An MP4 ALBUM differs per chapter until both rows agree.
- An MKA COMMENT stays with its chapter.
- Remove tag on an M4B row clears the file and keeps its rows.
- Strip chapters MP4 on a row lists the file in the rows' place, and Undo brings the rows back.
- Tag → Filename stays refused.
- A cue sheet whose second track starts in the next FILE lists `disc-2.flac` for it, and a title edit there is saved.

**`tests/e2e/chapters.js`** and **`tests/e2e/playlists.js`**: updated for the 0-based `%_subsong%`, the enabled whole-file commands and cue sheets that load as track rows.

## Decisions

- **Row consolidation follows Mp3tag's sub-track rule.** It is applied only to the edited fields.
- **Nero tags are written only when needed** (a chapter has fields of its own, or the file already has Nero tags) rather than on every chapter save.
- **TRACK n/m is written to Nero as `track` unchanged.** Mp3tag splits it into `track` and `totaltracks`, which would not read back as the same TRACK field.
- **Kept OpenTag extensions:** ID3v2 CHAP rows and rows for an embedded CUESHEET field.
- **FLAC CUESHEET blocks are not listed**, as in the examined Mp3tag.
- **`%_subsong%` is 0-based.**
- **Renaming, moving, deleting, Save a copy and playlists stay refused for chapter rows.**

## Remaining gaps

The Nero items are addressed in [formats-gaps](formats-gaps.md); its own remaining gaps replace them.

- **Nero tags:**
  - Global Nero `meta` fields only fill gaps. Editing or removing such a field with List chapters off changes the iTunes tag, and a removed field reappears from Nero.
  - Nero covers (`cvrx`) are kept but not read.
  - The `indx` box is not interpreted.
- **QuickTime chapter tracks** carry only titles; their `href`/URL samples are not read.
- **ID3v2 chapter pictures** (APIC in CHAP) are kept but not shown or edited per row.
- **Unconfirmed against Mp3tag output:**
  - Whether Mp3tag displays `%_subsong%` 0-based.
  - The case of Nero field names Mp3tag writes for fields outside its fixed list.
  - The second 16-bit value of a `cvrx` entry.
- **Playlists of chapter rows**: Mp3tag's subsong path format is unknown, so playlists stay refused for rows.
- **No differential test against files written by Mp3tag or Nero.**
