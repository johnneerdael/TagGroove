# TTA, TAK, OptimFROG, chapters and FLAC ID3v2

Branch `feat/formats-3` covers the remaining rows of Mp3tag's
`file-formats` page that OpenTag did not support: True Audio (TTA), TAK and
OptimFROG (`.ofr`, `.ofs`) streams, Matroska and MP4 chapters, Utils → Strip
chapters MP4, and removing ID3v2 from FLAC. Scanning now also lists `.tta`,
`.tak`, `.ofr` and `.ofs`. `.m4b`, `.m4v` (MP4 container), ALAC in `.m4a` and
`.spx` were already scanned; tests confirm `_CODEC` reads `AAC` for M4B and
M4V and `ALAC` for ALAC.

## TTA, TAK and OptimFROG

`crates/tag-core/src/raw_streams.rs`, detected in `container_formats.rs` by
the stream magic after any leading ID3v2 tags: `TTA1`, `tBaK` or `OFR `.
Extensions are not trusted.

Tag kinds and options follow the reference pages:

| Format | Reads | Writes | Removes | Options page |
|---|---|---|---|---|
| TTA | ID3v1, ID3v2, APEv1, APEv2 | ID3v1, ID3v2, APEv2 | all | Tags → Mpeg ("MP3 and TTA") |
| TAK, OptimFROG | ID3v1, APEv1, APEv2 | ID3v1, APEv2 | all, including ID3v2 | Tags → Ape, Mpc |

lofty cannot parse these streams, so the existing policies run unchanged on a
**stand-in**: the file's own leading ID3v2 bytes and trailing APE/Lyrics3/ID3v1
bytes around a minimal stream that lofty can parse. The stand-in for TTA is two
MPEG-1 Layer III frames, so lofty's MPEG path and `Policy::Mpeg` apply. For
TAK and OptimFROG it is a Monkey's Audio 3.99 header, which selects
`Policy::Ape`. Reading parses the stand-in in memory. A write copies it into
two small files next to the staging file and runs the normal lofty mutation on
them, including verification. The edited stand-in is then split back into
leading and trailing tag bytes and assembled around the original stream bytes
(`container_formats::assemble`), and the stream range is compared byte for byte
with the original.

As a result, every write path works with these formats: Save, covers (add,
remove, Manage covers, cover transforms), Remove tags, Utils → Remove tag
types, actions and converters.

Technical fields come from the stream headers:

- TTA1: channels, bits, sample rate and samples per channel.
- TAK STREAMINFO: a little-endian bitstream with the codec, frame size,
  35-bit sample count, sample rate − 6000, bits − 8 and channels − 1. This is
  the layout FFmpeg's `tak` parser reads. A sample count of all ones means the
  length is unknown.
- OFR: a 12- or 15-byte header with a 48-bit total sample count over all
  channels, the sample type (8, 16, 24 or 32 bits) and the channel count − 1.
- `_CODEC` is `True Audio`, `TAK` or `OptimFROG`. The format column shows
  `TTA`, `TAK` or `OptimFROG`. The bitrate is calculated over the stream
  bytes only, without tags.

Write cost: if the leading ID3v2 keeps its size, only the changed tag regions
are reported as rewritten (`write_cost::rewrote_region`), and the staged write
clones the original. If the ID3v2 size changes, the write counts as a full
rewrite. The stand-in's own lofty writes are kept out of the cost in a nested
`write_cost::Scope`.

## Matroska chapters

`matroska.rs`. Mp3tag's mapping pages document `%_subsong_count%` and
`%_subsong%` for files with chapters, but give no tag field for them. Mp3tag
edits chapters by listing them as separate files, which is outside this
branch. OpenTag adds:

- `CHAPTERS`, a multi-value field with one `HH:MM:SS.mmm Title` value per
  chapter of the default (or else the first) edition. Nested chapter atoms are
  flattened in order.
- `_SUBSONG_COUNT`, the chapter count.

Writing `CHAPTERS` rebuilds that edition and leaves other editions as they
were:

- Input may be shortened, for example `0:02.25 Title`. It is stored and read
  back in canonical form. Minutes or seconds of 60 or more, and values without
  a time, are refused.
- Chapter UIDs, flags and display languages are kept by position. A start time
  that still shows the same milliseconds keeps its exact nanoseconds.
- Each chapter ends where the next starts. The last one ends at the segment
  duration.
- An empty `CHAPTERS` removes the edition. Remove Tag drops the Chapters
  element along with the Tags and the image attachments, as `file-formats`
  lists.
- Placement reuses the existing Void/SeekHead logic. `verify_media` still
  requires every other element, clusters included, to be byte-identical at the
  same offset; Chapters is excluded only when the edit changes them.
- Ordinary tag edits do not touch Chapters.

Segment Info Title, where ffmpeg stores `title`, is not documented in the
mapping table, so it is not shown.

## MP4 chapters and Strip chapters MP4

`mp4_chapters.rs`. Chapters are read without loading moov in full. The box
walker reads only headers and descends into `udta`, `tref`, `tkhd`, and the
sample tables of the chapter track. Two sources are read:

- Nero `moov/udta/chpl`: 100 ns units. Version 1 has 4 reserved bytes, as in
  FFmpeg's reader.
- QuickTime chapter tracks: the track a `tref/chap` points at. Times come from
  `stts` in the `mdhd` time scale; sample positions from `stsz`, `stsc` and
  `stco`/`co64`. Samples are a 16-bit length followed by UTF-8 or BOM-marked
  UTF-16 text.

Nero chapters are used when both exist. They appear as the `CHAPTERS` field
and `_SUBSONG_COUNT`; writing chapters and listing them as rows are covered in
[chapters.md](chapters.md).

**Utils → Strip chapters MP4** is in the Tools menu and in the file list's
Utils section, and is disabled with a reason when no selected MP4 has
chapters. It asks the "at removing tags" confirmation, then runs through
`strip_mp4_chapters` (`src-tauri/src/utilities.rs`) as one journaled undo step
named "Strip chapters MP4". The function turns `chpl`, the chapter `trak`, and
the `tref` box (or just its `chap` child when the `tref` also names other
references) into `free` boxes of the same size with zeroed bodies. No offset
moves, the file length is unchanged, and only those regions are reported as
rewritten. Afterwards every byte outside the blanked type and body ranges is
compared with the original. The chapter text samples stay in `mdat` as
unreferenced bytes. Non-MP4 files and MP4 files without chapters are left
untouched.

## FLAC with a prepended ID3v2 tag

`file-formats` lists FLAC as reading Vorbis Comments only and removing Vorbis
Comments and ID3v2. `ordered_tags` now hides a FLAC ID3v2 tag, so it gives no
fields, covers, `_TAG` entry or `_ID3V2_*` details. Every FLAC save strips it
after the normal mutation: fields, covers, Manage covers and Remove Tag. The
FLAC stream after it is kept byte for byte.

## Verification

Automated (all pass):

- `crates/tag-core/tests/raw_streams.rs` (8 tests). Fixtures are hand-built
  from the TTA1, TAK and OFR layouts.
  - Technical fields for `.tta`, `.tak`, `.ofr` and `.ofs`, and scanning.
  - TTA under the default Mpeg policy: APE is removed, ID3v2.3 and ID3v1 are
    written, the stream bytes are identical, and Remove Tag leaves exactly the
    stream.
  - TAK and OFR under the Ape policy: the ID3v2 prefix is neither read nor
    written, ID3v1 is displayed until APE exists, APEv2 is written with a
    header, ID3v1 is updated, covers go into APE, and the `remove_id3v2`
    option strips the prefix.
  - Remove tag types and Remove Tag; a damaged stream is refused and left
    unchanged.
  - A same-size APE edit on a 4 MiB TAK keeps at most 64 KiB in the undo
    journal (clone filesystems), and undo restores the exact bytes.
  - With ffmpeg: an ffmpeg TTA decodes to the same MD5 after tagging, and
    ffprobe reads the new title and album.
  - With ffprobe: ffprobe's TAK demuxer reports the same sample rate,
    channels and `duration_ts` as OpenTag for the hand-built header. The CRC-24
    in the fixture is the one FFmpeg checks.
  - With mutagen 1.48: OptimFROG stream info and APEv2 items, and the TTA
    stream info and ID3 `TIT2`, read back as written.
- `crates/tag-core/tests/containers.rs`: Matroska chapters read, edit, add,
  clear and remove on a hand-built file with the cluster unchanged; invalid
  values are refused. An ffmpeg MKA with chapters is edited, ffprobe
  `-show_chapters` shows the new titles and times, the audio packet hashes are
  unchanged, and Remove Tag leaves no chapters.
- `crates/tag-core/tests/mp4_chapters.rs`: ffmpeg `.mp4` (AAC), `.m4b` (AAC)
  and `.m4a` (ALAC) files with both chapter kinds.
  - `CHAPTERS`, including UTF-8 titles, is read from Nero chapters, and from
    the QuickTime track once `chpl` is disabled.
  - Strip keeps the file length; ffprobe then lists no chapters and identical
    packet hashes.
  - Tags can be written afterwards; stripping again, or stripping a FLAC,
    changes nothing.
  - M4B and M4V are scanned as MP4 with `_CODEC` AAC.
- `crates/tag-core/tests/flac_id3.rs`: the ID3v2 prefix is not read; saving
  fields, adding a cover and Remove Tag each strip it with the frames
  unchanged. Both tests fail without the fix.
- Unit tests: stand-ins parse as MPEG/APE with full properties; TAK
  STREAMINFO bit order; Nero `chpl` parsing (also truncated); text samples.
- `tests/options-utils.test.mjs`: `mp4ChapterFiles` counts only MP4 entries
  with `_SUBSONG_COUNT`; the confirmation prompt text.

Native UI (`OPENTAG_E2E_ID=org.opentag.editor.e2e.formats3
OPENTAG_E2E_FIXTURES=tests/e2e/formats-3-fixtures.sh scripts/e2e-native.sh
tests/e2e/formats-3.js`), **6/6 PASS**:

1. `.tta`, `.tak`, `.m4b` and `.mka` fixtures are listed with
   `TTA/True Audio`, `TAK/TAK`, `MP4/AAC` and `Matroska/FLAC`.
2. A TTA title edit in the tag panel and ⌘S writes `ID3v2.3 ID3v1`; duration
   2.00 s, 44.1 kHz.
3. A TAK title edit writes an APEv2 tag (2 ch, 2.00 s).
4. Extended tags shows the MKA's `CHAPTERS` values. Editing the first to
   `0:00 Prologue` and saving stores `00:00:00.000 Prologue`.
5. Strip chapters MP4 is disabled for a TTA selection ("No selected MP4 file
   has chapters.").
6. Strip chapters MP4 on the M4B removes `_SUBSONG_COUNT` without changing
   the size or the title, and ⌘Z ("Undid Strip chapters MP4: 1 file")
   restores both chapters.

`scripts/e2e-native.sh` gained one opt-in hook, `OPENTAG_E2E_FIXTURES`, an
executable that receives the album folder.

## Gaps

- No real TAK or OptimFROG encoder runs on macOS, so those streams are
  hand-built headers with opaque payload. Tag writes never decode audio, and
  the header layouts were checked against ffprobe (TAK) and mutagen
  (OptimFROG), but no file from the original encoders was tested.
- `.ofs` (OptimFROG DualStream) is detected by the same `OFR ` header, and its
  `_CODEC` is also `OptimFROG`.
- Matroska `CHAPTERS` editing keeps nested chapters nested since the chapters
  batch ([chapters.md](chapters.md)). Chapters bound to other segments or
  ordered editions keep their other children but are not interpreted.
- Stripping leaves the chapter text samples in `mdat`. MP4 chapter writing
  and "List chapters as separate files" are in [chapters.md](chapters.md).
- Segment Info Title is not shown because the mapping docs do not document
  it.
