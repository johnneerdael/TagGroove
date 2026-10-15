# Chapters: MP4 chapter writing, chapters as rows, Matroska trees, cue sheet tracks

Scope: writing MP4 chapters, Options → Tags → Advanced → List chapters as separate files, nested Matroska chapters and the Segment Info title, and per-track entries for loaded cue sheets. References: `customization/options/tags/advanced`, `format` (`%_subsong%`, `%_subsong_count%`), `file-formats`, `playlists`, `menus/file`, `utils`.

Nothing was launched on a desktop. The evidence is unit and integration tests (`cargo test`, `node --test`) and the headless CI suite `tests/e2e/chapters.js`.

## MP4 chapter writing

**Field.** MP4 chapters are now the multi-value field `CHAPTERS`, one `HH:MM:SS.mmm Title` value per chapter, the same form as Matroska (`crates/tag-core/src/mp4_chapters.rs`). It replaces the read-only `_CHAPTERS` information field. `_SUBSONG_COUNT` is unchanged. Values are normalised before writing, so shorthand such as `0:02.5 Title` reads back as `00:00:02.500 Title`, and they are written in start order.

**What is written** (`crates/tag-core/src/mp4_layout.rs`, `write_chapters`):
- A Nero `chpl` box in `moov/udta`: version 1, a count byte, and per chapter a start in 100 ns units and a length-prefixed UTF-8 title. When a chapter's start still shows the same millisecond, its stored 100 ns start is kept.
- A QuickTime chapter text track laid out as ffmpeg writes one: `tkhd` (in movie, not enabled), an edit list, `mdhd` in the movie time scale, a `text` handler, a `gmhd` header, the QuickTime text sample description and one chunk of samples. Each sample is a 16-bit length, the UTF-8 title and an `encd` atom. Every audio and video track gets a `tref/chap` reference to it, and `mvhd`'s next track ID is updated.
- Earlier `chpl` boxes, chapter tracks and `chap` references are removed first. The text samples of a replaced ffmpeg track stay, unreferenced, inside its `mdat`.

**Placement.** Writes avoid moving media:
- If everything after `moov` is free space or earlier chapter data, only `moov` and the new chapter `mdat` at the end of the file are rewritten (ffmpeg writes `moov` last, so this is the common case).
- If `moov` comes first and the free space and old chapter data after it can hold the new `moov` and chapter data, they are written in place; any rest becomes a `free` box or `mdat` padding.
- Otherwise the file is rewritten once: `moov` stays first, the chapter `mdat` and 1 KiB of free space follow it, and every chunk offset is relocated and verified against the original bytes. Later edits then fit in place.

**Removing.** An empty `CHAPTERS` turns every chapter box into a `free` box of the same size, exactly like Utils → Strip chapters MP4. The file length does not change.

**Verification after each write.** The staged copy is read back. The Nero list must equal the request and the QuickTime track must have the same titles and starts within 1 ms (it starts at 0, see below). For in-place writes, every retained chunk offset must be unchanged and the samples must be at their offset. Errors leave the original file untouched.

**Limits.** At most 255 chapters, titles of at most 255 bytes, and starts must differ. These are Nero box limits and are refused with a message.

**Evidence** (`crates/tag-core/tests/mp4_chapters.rs`, ffmpeg `.mp4` AAC, `.m4b` AAC and `.m4a` ALAC):
- Writing three chapters with ALBUM in the same save. ffprobe then lists the new starts and titles, a copy with `chpl` disabled reads the same titles from the QuickTime track, the audio packet hashes are unchanged, and every byte before `moov` is identical.
- Title edits stay in place. Equal starts and 256-byte titles are refused and leave the file unchanged.
- An empty `CHAPTERS` keeps the file length, ffprobe lists no chapters and the packets are unchanged. Adding a chapter again works.
- A `+faststart` file with no free space is rewritten once with `moov` first. A later edit keeps the file length and leaves every byte from the media `mdat` on unchanged.
- `crates/tag-core/src/mp4_layout.rs` unit tests do the same on hand-built files without ffmpeg (tail and fast-start layouts, track IDs, removal of the old track, refused lists).

## List chapters as separate files

**Option.** Options → Tags → Advanced → List chapters as separate files, off by default as in Mp3tag (`tags.advanced.list_chapters`; `src/options-ui.ts`, `crates/tag-core/src/preferences.rs`). As the documentation says, the directory has to be reloaded after changing it.

**Sources:**
- MP4: Nero chapters or the QuickTime track.
- Matroska and WebM: chapters of the default edition, nested ones in document order.
- ID3v2 `CHAP` frames (MP3, WAV, AIFF, AAC and other lofty-read ID3v2 files), in start order.
- A cue sheet in a `CUESHEET` field (FLAC Vorbis comment, APEv2 item, and similar).

**Rows** (`crates/tag-core/src/subsongs.rs`). With the option on, a file with chapters is listed as one row per chapter instead of the file itself, as in Mp3tag.
- A row's path is the file path, U+0001 and the 1-based chapter number. Windows paths cannot contain U+0001 and other systems practically never do.
- A row shows the file's name, format and fields, with `TITLE` set to the chapter title. Cue sheet tracks also set `ARTIST` (PERFORMER), `COMPOSER` (SONGWRITER), `ISRC`, track ReplayGain and `TRACK`.
- `CHAPTERS` and `CUESHEET` are hidden on rows.
- Rows add `%_subsong%` (1-based), `%_subsong_count%`, and `%_length%`/`%_length_seconds%` and the duration of the chapter: to the next chapter, the stored end, or the file's end.
- Covers and the other information fields are the file's. `%_path%` is the file path.

**Editing rows.** Saving a row writes into the chapter:
- `TITLE` updates that chapter's title: the `CHAPTERS` value for MP4 and Matroska, the `TIT2` sub-frame of the `CHAP` frame (keeping the ID3v2 revision of non-MPEG files), or the TRACK block of the embedded cue sheet.
- For embedded cue sheets, ARTIST, COMPOSER and ISRC are also stored per track.
- Every other field is a file field, shared by all rows of the file. Rows saved together must give it the same value; otherwise the save is refused with "… is stored once for <file>, not per chapter".
- Rows of one file are written together in one undo step (`save_fields_batch` and `save_fields` group them; `history::record` snapshots the file). Undo and Redo return the rows again.
- `CHAPTERS`, `CUESHEET` and covers cannot be edited on rows. Whole-file commands are disabled with a reason for rows (`src/chapters.ts`): Remove tag, covers, Strip chapters MP4, the Utils, Tag → Filename, Filename → Filename, Save a copy, file copy/move/delete/reveal and playlists. Tag-core refuses them too.
- Actions, Tag → Tag, Filename → Tag (named after the file), Text File → Tag, tag sources, Extended Tags, Copy/Paste tags and exports reach rows through the same row paths and the backend's row reading and writing. Of these, only the tag panel save and Undo are exercised end to end.

**Evidence:**
- `crates/tag-core/tests/mp4_chapters.rs` (`chapters_are_listed_and_edited_as_rows`): scanning with the option lists two rows with the right paths, title, artist, `_SUBSONG`, `_SUBSONG_COUNT` and length. A row edit renames the chapter in the file (ffprobe agrees) and writes ALBUM to the file. Rows saved together agree; conflicting values and a missing chapter are refused without changes. Remove tag and Strip chapters MP4 are refused for rows.
- `crates/tag-core/tests/chapter_rows.rs`: ffmpeg MP3s with ID3v2.3 and v2.4 `CHAP`/`CTOC` frames list as rows, and a row title edit changes only that chapter (ffprobe lists the new title) while the file title stays. A FLAC with an embedded cue sheet lists its tracks, and row edits change only the edited lines of the sheet.
- `crates/tag-core/src/subsongs.rs` unit tests: row paths, row fields and lengths.
- `tests/chapters.test.mjs`: row paths in the frontend, the command guard and the option.
- `src-tauri/src/lib.rs` (`chapter_rows_are_grouped_by_their_file`).
- `tests/e2e/chapters.js`, headless in CI:
  - With the option on, the M4B and MP3 list two rows each and not the files.
  - A title typed in the tag panel for chapter 2 is saved into the M4B, and Undo restores it.
  - An ID3v2 chapter title is saved.
  - Remove tag, Strip chapters MP4 and Tag → Filename are disabled for rows with the reason, but not for an ordinary file.
  - A loaded cue sheet lists its two tracks, and a title edit is written into the sheet without touching the image.

## Matroska

**Nested chapters** (`crates/tag-core/src/matroska.rs`). Chapters are kept as a tree, and `CHAPTERS` lists them in document order.
- A list of the same length maps onto the tree by position. Title and time edits, including row edits, keep every chapter's nesting, UID, flags, languages and other children.
- When values are added or removed, chapters are paired by start time (a longest common subsequence). A removed chapter hands its nested chapters to its parent. A new chapter follows the paired chapter before it: as its first nested chapter when that one has nested chapters, otherwise as its next sibling. This keeps the document order equal to the edited list.
- A chapter keeps its stored end while its start and the next start are unchanged. Otherwise it ends at the next sibling, its parent's end or the file's end.

**Segment Info title.** The `Title` of the Segment Info element is shown as the read-only information field `%_segment_title%`, which can be used as a column. It is not mixed into `TITLE`: a fallback would make an emptied TITLE reappear and fail the write verification.

**Evidence** (`crates/tag-core/tests/containers.rs`, `nested_matroska_chapters_stay_nested_and_list_as_rows`):
- A hand-built MKA with `Part 1[Scene 1, Scene 2], Part 2` keeps that tree after a title edit, with the UIDs and the cluster bytes unchanged.
- Removing a nested chapter and adding two gives `Act 1[Scene A, Scene C], Act 2, Coda`.
- Its five rows list in document order, and a row edit of a nested chapter keeps the tree.
- `_SEGMENT_TITLE` reads the Segment title. The existing Matroska chapter tests still pass.

## Cue sheet tracks

**Reading** (`crates/tag-core/src/cue.rs`, `crates/tag-core/src/playlist.rs`). Loading a cue sheet (File → Load playlist/cue sheet, dropping it, or `/fn:`) now reads its TRACK blocks.
- A `FILE` that holds more than one track (an image) is listed as one row per track. A `FILE` with a single track still loads that file itself, so cue sheets exported by OpenTag load back as before.
- Fields of a track row:
  - From the TRACK block: `TITLE`, `ARTIST` (PERFORMER), `COMPOSER` (SONGWRITER), `ISRC`, track ReplayGain (`REM`) and `TRACK` (its number).
  - From the album lines: `ALBUM` (TITLE), `ALBUMARTIST` (PERFORMER, also the ARTIST of tracks without one), `GENRE`, `YEAR` (`REM DATE`), `COMMENT`, `DISCNUMBER`, `TOTALDISCS` and album ReplayGain.
- Track times come from `INDEX 01` (75 frames per second). A track ends at the next track on the same FILE or with the file.
- Technical fields, covers and the file name are the image file's.
- Quoted values run to the last quote, so titles with inner quotes read back.
- Text is decoded from a BOM, else UTF-8, else the system ANSI codepage (set at startup and on each load), as before.

**Writing.** Saving a track row edits the cue sheet, not the audio file.
- Only the lines of the edited keywords change. New lines go after the TRACK line or before the first FILE line, and everything else keeps its text, order and line endings. An empty value removes the line.
- Album fields must agree between rows. `TRACK` and fields a cue sheet has no line for are refused.
- The sheet is written to a sibling temporary file in its original encoding (ANSI text that no longer fits becomes UTF-8 with a BOM), read back, and moved over the original. It is not written if it changed on disk meanwhile.
- Undo snapshots the cue sheet and returns its rows again.

**Evidence:**
- `crates/tag-core/src/cue.rs` unit tests: fields, times and an unchanged render; edits that touch only their lines; refused fields; new lines in a sheet without album lines or a final newline; encodings.
- `crates/tag-core/src/playlist.rs`: an image FILE lists track rows, a single-track FILE loads its file, and a missing FILE is reported by its line.
- `crates/tag-core/tests/chapter_rows.rs`: a loaded sheet's rows read their fields and durations from the image. Editing TITLE and ALBUM rewrites exactly those two lines and leaves the image byte for byte, and LYRICIST and Remove tag are refused.
- The cue sheet export of `src-tauri/src/exports.rs` still loads back as files.

## Decisions

- **Rows replace the file.** With the option on, the file row is not listed, as in Mp3tag. File fields are edited through any chapter row and apply to all of them.
- **Per-chapter storage.** MP4 `chpl`, Matroska chapters and ID3v2 `CHAP` rows store only TITLE per chapter. Mp3tag's Nero per-chapter tag sets (`tseg`/`tshd`), Matroska tags bound to a ChapterUID, and other `CHAP` sub-frames are not read or written; existing sub-frames and children are kept. A single-row save of another field (for example from an action) writes the file field, so with different values per row the last one wins. Rows saved together are checked.
- **Late first chapter.** A first chapter that starts after zero keeps its start in the Nero box. The QuickTime track starts at zero, exactly as ffmpeg writes it, so ffprobe (which prefers the track) shows 0 there.
- **ID3v2 revision.** MPEG saves convert the tag to the configured ID3v2 revision, as before; chapter frames are converted with it.
- **`%_subsong%` is 1-based.** The documentation does not say; this is unverified against Mp3tag.

## Remaining gaps

- Per-chapter fields other than TITLE (Nero per-chapter tags, Matroska chapter-targeted tags, ID3v2 `CHAP` sub-frames) and their consolidation into file tags.
- Cue sheets with more than one FILE per track (tracks spanning files), and `INDEX 00` pregaps, which belong to the previous track. FLAC's binary CUESHEET block has no titles and is not listed.
- Whole-file commands on rows are refused instead of acting on the file.
- `%_subsong%` and `%_subsong_count%` are not checked against Mp3tag output.
- No differential test against Mp3tag-written files.
