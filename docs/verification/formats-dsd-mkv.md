# DSF, DSDIFF and Matroska/WebM tags

Branch `feat/formats-dsd-mkv` adds pure-Rust support for three containers that
lofty 0.25 does not parse: DSF (`.dsf`), DSDIFF (`.dff`) and Matroska/WebM
(`.mka`, `.mkv`, `.webm`). Files are detected by magic bytes, not by extension.
The new code lives in `crates/tag-core/src/container_formats.rs` (dispatch,
staging, verification), `dsd.rs` and `matroska.rs`. `tags.rs` and
`cover_ops.rs` only gained one early return per entry point (read, write
fields, remove tags, add/remove cover, cover transforms), so every app command
that uses those functions (save, remove tags, covers, actions, converters,
undo) reaches the new formats. Scanning includes the five new extensions.

Every mutation runs in the existing sibling staging copy. After writing, the
staged file is read again: written fields must read back (the ID3v2 rules of
`verify_readback` for DSF/DFF, verbatim values for Matroska), cover types and
descriptions must match, and the media payload is compared byte for byte with
the original. Any mismatch discards the staged copy and keeps the original.

## DSF

- Header, `fmt ` and `data` chunks are validated (chunk sizes, DSD raw format,
  data inside the file, metadata pointer inside the metadata area).
- The terminal ID3v2 tag is parsed with lofty by wrapping it in an in-memory
  AIFF form (lofty has no public parser for raw tag bytes) and mapped with the
  shared ID3v2 mapping, so user ID3v2 field mappings and the POPULARIMETER
  preference apply.
- Writing replaces everything from the metadata pointer to the end of the
  file; bytes between the sample data and an existing pointer are kept. The
  total file size and pointer are updated; removal sets the pointer to 0.
- Revision policy (our choice; Mp3tag documents the Mpeg options as applying to
  MP3 and TTA only): an existing tag keeps its revision, as WAV/AIFF already do
  here; a new tag uses the Options → Tags → Mpeg revision and encoding
  (ID3v2.3 UTF-16 by default).
- Technical fields: `_CODEC` `DSD64`/`DSD128`/… (rate ÷ 44100), sample rate in
  Hz (2822400 etc.), channels, `_BITSPERSAMPLE` from the header (1),
  duration = sample count ÷ rate, bitrate = rate × channels ÷ 1000 kbps.

## DSDIFF

- `FRM8`/`DSD ` form with big-endian 64-bit chunk sizes and even padding;
  `PROP`/`SND ` (`FS  `, `CHNL`, `CMPR`), one `DSD ` or `DST ` sound chunk
  (`FRTE` gives the DST frame count and rate), `DIIN` and `ID3 ` chunks.
  Trailing bytes after the form are preserved.
- Reading: the first ID3 chunk in file order (root or inside `PROP`) is used.
  `DIIN` `DIAR`/`DITI` fill `ARTIST`/`TITLE` only when the ID3v2 tag lacks
  them; `_TAG` lists `ID3v2.x` and `DIIN`.
- Writing: exactly one ID3 chunk is written, as the last root-level chunk;
  root and `PROP`-nested copies are removed (the `PROP` chunk is rebuilt). A
  file with more than one ID3 chunk refuses field/cover writes with a clear
  error so that nothing is merged silently; Remove tag still works and removes
  all of them. Existing `DIAR`/`DITI` values are kept in sync when `ARTIST`
  or `TITLE` change (multiple values joined with `; `, UTF-8); `DIIN` is never
  created. Remove tag deletes the ID3 chunks and `DIAR`/`DITI` but keeps other
  `DIIN` content (for example `EMID`, markers).
- Only chunks after the first change are rewritten (copied from the original),
  so the common layout leaves the sound data untouched in place. If the sound
  data would move and the file has a `DSTI` index (absolute offsets), the write
  is refused.

## Matroska/WebM

- EBML header (`matroska`/`webm` DocType), Segment, SeekHead, Info (duration
  via TimestampScale), Tracks (first audio track: codec, sample rate,
  channels, bit depth), Tags and Attachments. Files whose clusters have
  unknown sizes (live recordings) are read through the SeekHead; writes to
  them are refused.
- Field mapping follows the Matroska column of the mapping table
  (`FIELD_MAPPINGS.matroska`): e.g. MKA `TITLE` = T=30 `TITLE`, `ALBUM` = T=50
  `TITLE`, `ARTIST` = T=30, `ALBUMARTIST` = T=50 `ARTIST`, `TRACK` = T=30
  `PART_NUMBER`, `DISCNUMBER` = T=50 `PART_NUMBER`, `YEAR` = T=50
  `DATE_RECORDED`, `RELEASETIME` = T=50 `DATE_RELEASED`, `TITLESORT`/`ALBUMSORT`
  = `SORT_WITH` nested in that level's `TITLE`. Note that the team brief said
  DATE_RELEASED→YEAR; the cached mapping table says `DATE_RECORDED`, which is
  what is implemented.
- "Use recommended mapping for Matroska video" is a new Options → Tags →
  Advanced checkbox (`tags.advanced.matroska_video_mapping`, default on).
  When on and the file has a video track, the MKV column applies (`TITLE`
  T=50, `ALBUM` T=60, `TVEPISODE` T=50 and `TVSEASON` T=60 `PART_NUMBER`).
- Unmapped fields are written at T=30 under their own name and read from any
  global target level. Mapped native names at undocumented levels stay hidden
  and untouched. A tag named after a mapped Mp3tag field that is not itself a
  Matroska name (ffmpeg writes `ALBUM` at T=50) is read as that field when the
  documented location is empty and is replaced on write. Writing a reserved
  native name (e.g. `PART_NUMBER`) as a field is rejected.
- Only global tags (no track/edition/chapter/attachment UID) are exposed;
  scoped tags such as mkvmerge/ffmpeg `DURATION` statistics are preserved
  byte for byte. Existing SimpleTags keep their language, default flag and
  nested tags when their value changes.
- Covers are attachments with an `image/*` MIME type. File names map to
  picture types (`cover.*`, `small_cover`, `cover_land` → front; `back.*`,
  `artist.*`, … → that type; other names → Other); new ones are named
  `cover.png`, `cover (2).png`, `back.jpg`, …. Unchanged pictures keep their
  original element (UID and name). Non-image attachments (fonts) are kept.
- Writing never moves clusters or cues. Old Tags/Attachments become Void
  elements; replacements go into a Void region that fits or at the end of the
  Segment (reusing the tail on repeated edits). The SeekHead is rebuilt in its
  own space plus the following Void padding (its CRC-32 child is dropped); if
  it cannot fit, the write is refused. The 8-byte Segment size is updated.
  Verification requires every other top-level element (clusters, cues, info,
  tracks, chapters) and the EBML header to be byte-identical at the same
  offset.
- Remove tag removes global tags and image attachments; scoped tags, chapters
  and non-image attachments stay.

## Tests

`crates/tag-core/tests/containers.rs` (10 tests) builds DSF, DSDIFF (DSD and
DST, DIIN, nested and duplicate ID3, DSTI) and Matroska files byte by byte and
covers read, write, remove, cover add/remove/property edits, revision policy,
malformed input, media preservation and scanning. The ffmpeg test generates
FLAC-in-MKA with a PNG attachment, Opus WebM and FFV1+FLAC MKV, edits fields
and covers, removes tags, and checks with ffprobe that the files still decode
without errors, that ffprobe reports the written tags, and that every audio and
video packet's MD5 is unchanged. It is skipped when ffmpeg/ffprobe are missing.
Four unit tests in `matroska.rs` cover mapping-rule uniqueness, EBML size/Void
encoding, picture names and alias fields. A 130 MB ffmpeg PCM MKA was edited in
0.35 s (release build) and still decoded.

## Known gaps

- No comparison against the original Windows Mp3tag; DSF behaviour beyond
  "ID3v2 read/write", DSDIFF behaviour (not listed by Mp3tag), DIIN
  precedence, picture-type file names and the exact Matroska remove semantics
  are our documented choices.
- Matroska chapters are neither shown nor removed; "List chapters as separate
  files" is not implemented. The Info `Title` (where ffmpeg stores `title`) is
  not shown or updated, so ffmpeg-made files show no `TITLE` until one is
  written; ffmpeg's global tags sit at T=50, so its `artist`/`track` appear as
  `ALBUMARTIST`/`DISCNUMBER` under the documented audio mapping.
- `TRACK` values such as `3/12` are stored verbatim in `PART_NUMBER`
  (no `TOTAL_PARTS` split). User field mappings do not apply to Matroska
  (there is no Matroska tag type in the mapping dialog).
- Matroska files without a SeekHead are written but the new Tags are not
  indexed; a SeekHead without spare room makes the write fail safely.
- No real-world DSF/DFF files from other taggers were available; fixtures are
  specification-derived.
