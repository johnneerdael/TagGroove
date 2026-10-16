# File format gaps: Nero tags, MP4 `plID`, ADTS after junk, APE, DSD frames

Branch `feat/formats-gaps`. Covers the format items left open in
[formats-edge](formats-edge.md) and the Nero items of
[chapter-fields](chapter-fields.md). Backend only (`crates/tag-core`); no UI
strings were added. Reference behaviour comes from the documentation cache
(`file-formats`, `options/tags/advanced`, the 2.84 release notes) and, where
it says nothing, from a static look at Mp3tag 3.36.1 (`Mp3tag.exe`). Mp3tag
was never run, and no code or text was taken from it. Nothing was launched on
a desktop.

## What changed

| Area | Before | Now |
| --- | --- | --- |
| Nero file-wide fields | Filled fields the iTunes tag lacks, but an edit or removal only reached the iTunes tag, so a removed field came back from Nero | Editing or removing a field the Nero `meta` box holds changes it there too (`NeroTags::sync_global`). Fields Nero does not hold are not added, and Nero's value limits only apply to the fields it holds. MP4 user mappings apply to these fields on read and write as they do to the iTunes tag. The `tags` box is rewritten in place with a `free` box after it when it fits, otherwise together with the chapters. Chapter rows apply the same rule (`subsongs::write_mp4_rows`) |
| `_TAG` of MP4 files with Nero tags | `MP4` | `MP4 Nero`, as the examined binary lists them; `_TAG_READ` stays `MP4` |
| Nero covers (`cvrx`) | Kept, not read | Read (`NeroTags::covers`) and shown as the file's covers when the iTunes tag has none. Any cover edit stores the shown covers in `covr` and then drops the `cvrx` box, so an edited or removed cover cannot come back from Nero. Field edits leave `cvrx` byte for byte |
| `udta/indx` | Kept, not looked at | Still kept byte for byte through chapter and Nero rewrites (now covered by a test). The examined binary reads it as a count and per entry a 64-bit value and a length-prefixed string, and writes it back |
| MP4 `plID` not 8 bytes long | lofty drops it silently, even in strict mode, so it was not shown and the next save deleted it | Restored verbatim on every MP4 read (`mp4_metadata::odd_album_id`, which reads only atom headers and the `plID` item). Shown as ITUNESALBUMID when it is 1–8 bytes, kept byte for byte by other edits, written in the documented 8-byte form when edited |
| AAC with junk or padding after the ID3v2 tag | Not a raw stream: no APE, no tag policy | An `.aac` file whose first ADTS frame follows the leading ID3v2 tags after up to 64 KiB of other bytes is a raw AAC stream. The frame must start three consecutive ADTS frames of one format, and the search stops at an MPEG audio frame. The junk is kept with the audio bytes and verified with them; lofty reads the stream properties from the first frame on |
| Header-less APEv2 tags (footer only) | lofty takes every APEv2 footer to follow a header: reads cut the stream 32 bytes short and APE writes deleted the 32 bytes before the tag | Readers see the footer marked APEv1, which lofty measures correctly. Writes mark the staged file the same way and mark the tag APEv2 again when it was not rewritten (`ape_footer::Guard`) |
| "TAG" 128 bytes before the end inside an APE tag | Handled only for the header of a 131-byte tag, by removing that header (and marking the tag APEv1) before and after every write | Handled for any APE tag that ends the file and reaches that far back, in its header or its items. Readers see an extra empty ID3v1 tag after the file, so lofty finds the APE footer where it looks behind ID3v1; offsets and APE bytes stay as they are, and that ID3v1 tag is dropped from the result. Writes append the same tag to the staged file and remove it afterwards unless the write replaced it with a real one. Headers are no longer removed |
| DSF and DSDIFF ID3v2 tags | Compressed frames were written decompressed; encrypted ID3v2.3 frames made the file unreadable or the save refused | Read through the same placeholder fallback as other formats; unedited compressed and encrypted frames are copied byte for byte (`mapping::render_id3` with the stored tag) |

## Decisions

- **Nero precedence.** The iTunes tag still wins and Nero only fills gaps, as
  before. The binary keeps the Nero tags in an object of their own; how it
  merges them with the iTunes fields was not traced, so this stays
  unconfirmed. Mirroring edits into fields Nero already holds keeps both sets
  consistent without inventing Nero fields Mp3tag may not write.
- **Nero rewrites without chapters.** A file with Nero tags but no chapters
  cannot go through the chapter rewrite. Removals and shorter values always
  fit in place. A longer value that does not fit stays in the iTunes tag only;
  the stale Nero value is hidden, because the iTunes tag takes precedence.
- **`cvrx` layout.** Read and written by the binary as: a 32-bit zero, a
  32-bit count, then per cover a 16-bit length and a narrow string, a 16-bit
  value and a 32-bit length (at most 64 MiB) and the image bytes. The string
  is taken to be the MIME type (an empty one is detected from the image
  bytes) and the 16-bit value the ID3v2 picture type. Both are inferred from
  the layout, not from documentation.
- **Nero covers on edit.** Moving them into `covr` and dropping `cvrx` is
  what makes every cover edit (add, remove, adjust, properties) behave the
  same whichever box the covers came from. `cvrx` is dropped also when
  iTunes covers hid it; kept, it would surface as soon as those covers were
  removed.
- **ADTS search.** Only `.aac` files are searched: Mp3tag tells formats by
  extension, and other streams can look like ADTS (a FLAC frame header passes
  the ADTS sync test). The 64 KiB window and the three-frame check are
  OpenTag's limits; Mp3tag's are not documented.
- **Phantom ID3v1 tag.** Its title has a byte after the terminating NUL, which
  no ID3v1 writer produces, so it is never mistaken for a written tag. Its
  year is "0000" because lofty's strict reader refuses an empty year.
- **131-byte APE tags keep their header.** Mp3tag writes APEv2 tags with a
  header, so a 131-byte one collides in files it writes as well; reading
  around the collision is the general fix, and stripping headers is no longer
  needed.

## Verification

All on this Mac without launching the app.

- `AR=/usr/bin/ar cargo test -p tag-core`: every suite green, including the
  new `tests/formats_gaps.rs` (12 tests, all fixtures built by hand). The
  first 11 were run against the `main` sources: 10 fail, and the remaining
  one checks that the hand-built MP4 reads like a real file.
  - Nero with MP4 user mappings (ALBUM → COLLECTION, GENRE → STYLE): the
    Nero value fills the mapped name, and deleting the mapped field (from the
    file or from chapter rows) removes the Nero value; a COMMENT longer than
    Nero's limit is written when Nero does not hold COMMENT.
  - MP4: a minimal one-track AAC file built box by box. File-wide Nero
    fields fill gaps, `_TAG` is `MP4 Nero`; an unrelated edit leaves the
    `tags` box byte for byte; removing a Nero-held field shrinks the box in
    place (file length unchanged); a longer value rewrites it with the
    chapters; `indx` and every sample byte survive all of it. The same
    removal in a file without chapters.
  - Nero covers: a PNG in `cvrx` is shown (type, MIME, size); field edits keep
    `cvrx`; adding a cover stores both in `covr` and drops `cvrx`; removing
    all covers leaves none; a cover without a MIME type is detected as PNG.
  - `plID` with 4 bytes: lofty's strict reader loses it (asserted), OpenTag
    shows 16909060, keeps the item byte for byte across a TITLE edit and
    writes the 8-byte form when it is edited.
  - ADTS after 300 zero bytes and junk text behind an ID3v2 tag: AAC, APE
    read, the APE/ID3v1 policy applies on save, junk plus frames unchanged.
  - Header-less APEv2 after MPEG frames and after a WavPack block: read, an
    ID3v2-only save keeps the APE bytes, an APE save keeps every audio byte.
  - "TAG" inside an APE item: read as APE without ID3v1; ID3v2-only saves
    keep the APE bytes; APE saves keep the item; an ID3v1 save adds the tag
    after the intact APE tag.
  - A 131-byte APE tag keeps its header on an ID3v2-only save.
  - DSF and DSDIFF with a zlib-compressed TALB and an encrypted TXXX frame:
    read; an unrelated edit copies both frames byte for byte and keeps the
    sound data; editing ALBUM writes it plain and keeps the encrypted frame.
- Unit tests: `ape_footer` (collision in header and items, staged phantom,
  guard restores an untouched tag byte for byte and leaves a rewritten one),
  `id3_frames` (overlay tail and seeking from the end), `nero_tags` (`cvrx`
  parsing and round trip, file-wide sync), `raw_streams` (ADTS search limits
  and the extension check).
- The existing ffmpeg-based MP4 chapter, chapter row and formats-edge tests
  ran with ffmpeg present and pass unchanged.
- `AR=/usr/bin/ar cargo clippy -p tag-core --all-targets -- -D warnings` and
  `cargo fmt --all -- --check`.

## Remaining gaps

- How Mp3tag merges file-wide Nero fields with the iTunes tag is unconfirmed;
  OpenTag keeps "iTunes first, Nero fills gaps".
- The meaning of the `cvrx` string and 16-bit value is inferred (MIME type,
  picture type). Covers are always written to `covr`, never to `cvrx`.
- `udta/indx` is kept but not shown or edited.
- A longer value for a Nero-held field in an MP4 file without chapters only
  reaches the iTunes tag.
- ADTS after junk is only found in `.aac` files, within 64 KiB, and needs
  three frames.
- Compressed or encrypted frames still cannot be kept in an unsynchronised
  tag or one with an extended header or footer, and frames lofty merged as
  duplicates still cannot be matched to their stored bytes (unchanged from
  formats-edge). This also applies to DSF and DSDIFF now.
- No differential test against files written by Mp3tag or Nero.

## Review follow-up (2026-10-16)

- Apply MP4 user mappings to each Nero chapter field set on read and reverse
  them before writing `tseg` metadata. A constructed two-chapter fixture with
  `GENRE` → `STYLE` verifies mapped display, a native `genre` write, deletion,
  preservation of the other chapter and unchanged audio. The regression first
  failed on read, then independently on write after only the read path was fixed.
- Collect all repeated `plID` item atoms when restoring values that lofty drops,
  rather than selecting only the first item. A regression places standard and
  short IDs in both orders, verifies every displayed value before and after an
  unrelated save, and compares each data atom's type, width and bytes plus the
  audio. The standard-first case originally exposed only the first ID.
- All 14 focused `formats_gaps` regressions passed locally. These remain
  constructed-fixture checks rather than a differential test of another tagger.
- Final local checks: 312 UI tests and 711 Rust workspace tests passed, with
  four live-network tests ignored; workspace Clippy, formatting, strict guide
  build and diff checks passed. The final Rust run used a task-specific
  `TMPDIR` and retained the normal test concurrency and assertions.
- Intermediate full runs failed undo tests, including explicit insufficient
  disk-space messages in the core history tests. A serial app-only run passed
  263 tests separately. Temporary-directory isolation alone did not fix the
  full run; it passed unchanged after free-space headroom recovered above the
  history journal's 2 GiB reserve. No history code or tests were weakened.
