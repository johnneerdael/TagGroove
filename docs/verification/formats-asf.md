# Windows Media (WMA/ASF) read and write

Scope: the WMA entry of the supported file formats (read, write and remove "ASF/Windows Media"), the ASF column of the field mapping table, covers, and the technical fields. References: `file-formats`, `mapping`, `mapping-table`, the Microsoft ASF specification (object GUIDs and layouts). The C++ TagLib bridge proposed in `docs/research/missing-formats.md` was not used. The implementation is pure Rust, so it cross-compiles like the rest of tag-core.

## Implemented

**Codec** (`crates/tag-core/src/asf/header.rs`). Decodes only the Header Object. The Data Object and the index objects are never parsed.
- File Properties gives the duration (play duration minus preroll; broadcast files have none) and the bitrate fallback.
- Stream Properties gives codec tag, channels, sample rate, bits per sample and bitrate from WAVEFORMATEX. The audio stream's Stream Properties may be top-level or embedded in an Extended Stream Properties object.
- Codec List gives the codec name used for `%_codec%`. Without it, the name comes from the format tag (WMA v1/v2/Pro/Lossless/Voice).
- Content Description (Title, Author, Copyright, Description, Rating), Extended Content Description, and the Header Extension's Metadata and Metadata Library objects become one ordered attribute list. Each attribute keeps its type, stream number, language index and source object.
- Types: Unicode, byte array, BOOL (4 bytes in the ECD, 2 bytes elsewhere), DWORD, QWORD, WORD and GUID (Metadata Library only). Unknown type codes, or sizes that do not match the type, are kept verbatim.
- WM/Picture payloads (type, MIME, description, data) are decoded and encoded. Malformed payloads stay opaque attributes.
- Rendering:
  - Header objects without attributes (Stream Properties, Codec List, padding, unknown GUIDs, unknown objects inside the Header Extension) are copied byte for byte.
  - An attribute object whose attribute list is unchanged is also copied byte for byte. An unchanged header renders identically, which a unit test checks.
  - Untouched attributes stay in their object.
  - New or edited attributes go to the Metadata Library when they are repeated, larger than 64 KiB, have a language index or are GUIDs. Otherwise they go to the Metadata Object when they are stream-specific, to the Content Description for its five names when single and non-empty, and to the Extended Content Description for everything else.
  - A Header Extension is created when needed. The header object count, header size and File Properties file size are updated.

**Entry model** (`crates/tag-core/src/asf/mod.rs`). ASF files produce the same `AudioEntry` as other formats, with format `WMA`.
- Attribute names map to Mp3tag fields through the registry's ASF column, case-insensitively. For example, Author→ARTIST, Description→COMMENT, WM/AlbumTitle→ALBUM, WM/TrackNumber→TRACK, WM/PartOfSet→DISCNUMBER and WM/SharedUserRating→RATING WMP. Other attributes keep their native names.
- Numeric, BOOL and GUID values are shown as text. Binary attributes other than covers are hidden but preserved.
- A copy of the same value under the same field in a second object (FFmpeg writes Title/Author to both the Content Description and the ECD) is shown once.
- RATING WMP shows the 0–99 DWORD as 0–5 stars. It accepts digits or `*` and writes 1/25/50/75/99.
- User field mappings with tag type `WMA` now apply to reads and writes. Before this change, the type was accepted but did nothing.
- `%_codec%`, `%_bitrate%`, `%_samplerate%`, `%_mode%`, `%_bitspersample%`, `%_length%`, and `%_tag%`/`%_tag_read%` (`WMA`) are filled. The cover, size, time and path fields are shared with other formats through `technical::stream_fields`/`file_fields`.

**Writes**
- A field edit replaces every attribute that displays as that field and inserts the new values at the first old position. Type, stream and language are kept. A value that would not read back identically in the old type, such as `3/12` in a DWORD track, is stored as Unicode.
- Writing a field whose literal native name belongs to another field is refused. For example, DESCRIPTION would be stored as `Description`, which is COMMENT. Removing such a field is a no-op.
- Covers can be added, removed, have their properties set, removed in bulk, or adjusted. Unchanged pictures reuse their stored attribute and object.
- "Remove tag" drops every attribute that is not stream-specific.
- Each write goes through the shared staging path (`staged_write`, split out of `staged_mutation_with_mappings`). The staging path writes to a sibling temp file, keeps permissions and the optional modification time, and checks for concurrent change. The writer:
  - writes the new header,
  - streams everything after the old header from the original,
  - re-reads the staged file and compares the media bytes with the original byte for byte,
  - checks the fields or covers it wrote, then re-reads the entry before the original is replaced.

  Data, Simple Index and Index objects never move relative to each other. Their offsets are relative to the Data Object, so a header size change is safe.

**Dispatch** (`tags.rs`, `cover_ops.rs`). An ASF magic check runs before lofty in `read_description`, the step behind every read, scan, reload and post-write description. Lofty's MPEG sync search matched noise inside a large picture in the header, so the check has to come first. Every mutation entry point also starts with the check: write fields, remove tags, add/remove cover, and the cover transforms. As a result, src-tauri commands, actions, converters and the undo journal (which snapshots files) work unchanged. `.wma` and `.asf` are scanned from folders. `file-formats` lists only WMA, so `.wmv` is not added.

## Tests

- Unit tests (`crates/tag-core/src/asf/tests.rs`, 7): byte-identical render of an unchanged header; typed parsing from all four attribute objects (stream, language, GUID, unknown type, BOOL sizes); placement rules and verbatim preservation of unknown top-level and nested objects and of untouched objects; Header Extension creation and object count; type-preserving text parsing and rating/GUID conversions; WM/Picture encode/parse and malformed rejection; malformed header rejection.
- Integration tests (`crates/tag-core/tests/asf.rs`, 8) with a hand-built specification file (every object and value type, unknown object, Data Object and Simple Index; no external tools needed):
  - reads fields, properties, technical fields and covers;
  - field writes: non-ASCII, repeated values, removal, star rating, a value over 64 KiB that must land in the Metadata Library, custom fields, BOOL. Data and index bytes are identical, header count and File Properties size are consistent, the unknown object is preserved, and restoring the values restores the original fields;
  - covers: a PNG over 64 KiB, set properties, remove by index, remove all, with media bytes identical;
  - remove tag keeps stream attributes, unknown objects and media;
  - folder scan of `.wma`/`.asf` and `WMA` user mappings;
  - refused writes leave the file untouched, and truncated files fail to read and write.
- With ffmpeg/ffprobe (skipped when absent): a `wmav2` file encoded by ffmpeg is read, edited, given a cover and stripped. After each step:
  - media bytes are identical;
  - `ffmpeg -f null` decodes without errors;
  - ffprobe shows the new title/artist/album_artist/WM/Year/composer and the removed genre, and sees the WM/Picture as an attached picture.

  A second ffmpeg test checks that ffprobe reads a 70,000-character value and repeated values from the Metadata Library. ffmpeg keeps only the last repeated value.

Full suite on macOS: `AR=/usr/bin/ar cargo test --workspace`, clippy `-D warnings`, `cargo fmt --check`, `npx tsc --noEmit` and `node --test tests/*.test.mjs` (165) pass.

## Gaps and risks

- Not compared against the original executable. Mp3tag's choices are inferred from the documentation and the specification and have not been verified for:
  - the `%_codec%` text, `%_tag%` = `WMA`, star conversion for RATING WMP, and BOOL/GUID text forms;
  - whether repeated values go to the Metadata Library (we follow the Windows Media Format SDK rule) and whether the first Author/Title stays in the Content Description;
  - dedupe of cross-object copies, and whether "Remove tag" should keep stream-specific attributes.
- No WMA from Windows Media Player or Mp3tag was available. Fixtures come from ffmpeg and from the hand-built specification file. DRM-protected files, files with Content Encryption objects and files over 4 GiB were not tested. Those objects are preserved as unknown objects, but nothing checks that protected content still plays.
- Players built on libavformat before 62 (ffmpeg 6.x/7.x) reject Metadata Library values over 65535 bytes and skip the rest of that object. Values that large, such as long lyrics or big covers, are therefore invisible to them. To keep everything else visible, a re-encoded Metadata or Metadata Library object writes its oversized records last; a unit test checks the order, and the ffprobe test checks that repeated values stay readable next to an oversized one. The specification requires the Metadata Library for such values, and ffmpeg 8.0+ reads them. The ffprobe check for this case depends on the libavformat version.
- Headers larger than 512 MiB are refused. Every write rewrites the whole file (header plus streamed copy). No padding object is reused for in-place edits.
- Legacy `WM/Track` (0-based) is shown under its native name, not as TRACK. Language-specific duplicates are not filtered by language.
- `.wmv` and other ASF video files are not scanned by default, but files with a `.wmv` extension can still be added explicitly or through "Restrict incoming files".
