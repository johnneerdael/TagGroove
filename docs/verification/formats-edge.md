# File formats and mapping edge cases

Branch `feat/formats-edge`. Covers the "File formats and mapping" rows and the
Options → Tags → Advanced write toggles from `docs/research/remaining-gaps.md`.
Reference behaviour: the private documentation cache (`file-formats`,
`mapping`, `mapping-table`, `options/tags/advanced`, `options/tags/mpeg`,
`options/tags/ape`) and the Mp3tag 3.33.1 release notes. Nothing was launched
on a desktop; UI checks run headless in CI only.

## What changed

| Area | Before | Now |
| --- | --- | --- |
| AAC (ADTS) | lofty path: ID3v1/ID3v2 read, ID3v2 written, APE ignored (and lost on save), no tag policy, Utils → Remove tag types refused AAC | Raw stream with an MPEG stand-in (`raw_streams.rs`): reads ID3v1, ID3v2, APEv1/v2; writes ID3v1/ID3v2/APEv2 and removes per Options → Tags → Mpeg; Remove tag types accepts AAC. Stream properties still come from lofty, read from the audio bytes alone |
| WavPack | A prepended ID3v2 tag made the file unreadable (lofty's probe does not look past it) | Raw stream with a Monkey's Audio stand-in: the ID3v2 tag is parsed and listed in `%_tag%`, never displayed (Ape, Mpc policy), kept on save unless Remove ID3v2 is set, removable with File → Remove tag and Utils → Remove tag types |
| ASF | Legacy zero-based `WM/Track` ignored | Shown as TRACK (value + 1) when there is no WM/TrackNumber; writing TRACK moves WM/Track along in its stored type, removing TRACK removes it |
| MP4 integer atoms | lofty 0.25.4 already reads the 8-byte `plID`; any other integer item that is not 1–4 bytes (for example an 8-byte `cnID` labelled as a 32-bit integer) made the whole file unreadable in strict mode | Such files are re-read leniently and the dropped items restored verbatim; the item count is checked so no other malformed item is silently lost |
| ID3v2 COMM / USLT | Every COMM frame was folded into COMMENT (iTunNORM included) and editing COMMENT deleted all of them; USLT descriptions were not representable | `COMMENT <description>` and `UNSYNCEDLYRICS <description>` fields; COMMENT edits only touch the frame without a description |
| ID3v2 UFID | Bare UNIQUEFILEID writes were refused; non-MusicBrainz UFIDs were opaque | `UNIQUEFILEID` = `owner|identifier` for text identifiers (one per owner, at most 64 bytes, MusicBrainz owner stays in MUSICBRAINZ_TRACKID); binary identifiers stay opaque |
| ID3v2 compressed / encrypted frames | Compressed frames were decoded but any rewrite failed or corrupted them; encrypted ID3v2.3 frames made the file unreadable | `id3_frames.rs`: unreadable tags are re-read through an overlay that renames encrypted frames to placeholders; every save copies the compressed and encrypted frames it did not edit byte for byte; conversion to the other ID3v2 revision writes compressed frames plain and refuses encrypted ones (file untouched) |
| APE tag of exactly 131 bytes at the end | Its header's "TAG" sits where an ID3v1 tag starts; lofty read an ID3v1 tag made of APE bytes and missed the APE tag, and its writers then damaged the file (found with Ubuntu's ffmpeg 6.1, whose WavPack `encoder` item made a tag of that size after one edit) | `ape_footer.rs`: reads mask that "TAG" through the same overlay; before a write, and after every APE write, such a tag loses its header (marked APEv1, since lofty assumes every APEv2 footer has a header); the next APE write stores APEv2 again |
| Options → Tags → Advanced | "Single MP4 atom" and "reuse unmodified atoms/frames" were implicitly always on | Three settings: `mp4_single_atom` (default on), `reuse_mp4_atoms` and `reuse_id3v2_frames` (default off, as Mp3tag since 3.33.1) |

## Decisions

- AAC follows Options → Tags → Mpeg. The Mpeg page names MP3 and TTA only,
  but AAC carries exactly the MP3 tag set (`file-formats`), so it is the only
  policy that applies. The Options note now says "MP3, AAC and TTA".
- Mp3tag's syntax for described comments is `COMMENT <description>` (as used
  for MediaMonkey's `Songs-DB_Custom1`); lyrics use the same form with the
  documented `xxx||text` value. ID3v2 tells COMM frames apart by language and
  description and lofty stops reading a comment at a NUL, so a description
  holds one comment: a second COMMENT value is refused with a message that
  points to `COMMENT <description>` (previously written as `Comment 1`… frames,
  which would now read back as separate fields). Lyrics keep one text per
  language.
- UNIQUEFILEID uses `owner|identifier`, like POPULARIMETER's `|` syntax.
- Reuse off means: on every save a TXXX frame named after a documented field
  that lives elsewhere (TXXX:MOOD in ID3v2.4, TXXX:MUSICBRAINZ_ALBUMID) and a
  freeform/alternative MP4 atom (`----:com.apple.iTunes:COMPOSER`, `©art`)
  move to the documented frame or atom, unless that field is being edited,
  its documented frame already exists, or the value would not fit it
  (COMMENT/UNSYNCEDLYRICS aliases are never moved). Reuse on keeps them until
  the field changes, which was the old behaviour.
- "Single MP4 atom" defaults to on (the previous behaviour; the documentation
  does not state Mp3tag's default). Off splits every multi-value item except
  `covr` into one atom per value after lofty encodes the tag.

## Verification

All on this Mac without launching the app, plus CI.

- `AR=/usr/bin/ar cargo test -p tag-core` — every suite green, including the
  new `tests/formats_edge.rs` (11 tests) and unit tests in
  `src/id3_frames.rs` and `src/asf/tests.rs`:
  - AAC: APE/ID3v2/ID3v1 read priority and each read toggle; default save
    writes ID3v2.3 + ID3v1, carries APE values over and removes APE; APEv2-only
    writes; Remove tag types; ID3v1-only writes; the ADTS bytes are compared
    byte for byte after every write.
  - WavPack: prepended ID3v2 listed but not displayed, kept byte for byte on
    save, kept by File → Remove tag with the defaults, removed by Remove tag
    types and by the Remove ID3v2 option; properties from the stream.
  - ffmpeg-encoded AAC and WavPack: decoded audio MD5 unchanged after two
    saves (ffmpeg cannot decode WavPack behind an ID3v2 tag, so that run
    removes it).
  - MP4: 8-byte `plID` and `cnID` labelled type 21 read, preserved byte for
    byte across an unrelated edit, rewritten in their documented types when
    edited; strict lofty refuses the fixture, which proves the case.
  - ID3v2 compressed (zlib) and encrypted frames in v2.3 and v2.4: read,
    `%_id3v2_unknown_frames%` names the stored identifier, unrelated edits copy
    both frames byte for byte (v2.3 tag rewritten in place), editing the
    compressed field writes it plain, the default v2.3 conversion of a v2.4 tag
    with an encrypted frame is refused with the file unchanged, Remove
    unsupported fields drops encrypted frames.
  - COMMENT/UNSYNCEDLYRICS with descriptions and UNIQUEFILEID: read, edit,
    add, delete in v2.3 and v2.4; invalid values refused with the file
    unchanged; MUSICBRAINZ_TRACKID untouched.
  - A 131-byte APE tag after WavPack and MPEG streams (with an ID3v2 prefix)
  is read as APE, and multi-value writes succeed and read back; unit tests
  for the collision check and the header removal.
- Reuse toggles: ID3v2.4 TXXX:MOOD → TMOO and TXXX:MUSICBRAINZ_ALBUMID →
    TXXX:MusicBrainz Album Id only when reuse is off; MP4 freeform COMPOSER →
    ©wrt only when reuse is off; single atom on/off checked on the encoded
    `©ART` bytes.
  - ASF: DWORD and string WM/Track, WM/TrackNumber precedence, TRACK writes
    and removal.
- `AR=/usr/bin/ar cargo test -p opentag` — the settings fixture test covers the
  new defaults.
- `node --test tests/*.test.mjs` — 247 passing.
- `AR=/usr/bin/ar cargo clippy -p tag-core --all-targets -- -D warnings`.
- Native e2e `tests/e2e/formats-edge.js` (fixtures
  `tests/e2e/formats-edge-fixtures.sh`), run headless in CI: the three Advanced
  checkboxes, their defaults and that they are stored; AAC APE display, a tag
  panel save producing ID3v2.3 + ID3v1; Remove tag types on AAC and on the
  ID3v2-prefixed WavPack.

## Remaining gaps

- A `plID` whose data is not 8 bytes is still dropped by lofty without an
  error; finding it would need a raw `ilst` scan on every MP4 read.
- AAC with junk or padding between the ID3v2 tag and the first ADTS frame is
  not detected as a raw stream and keeps the lofty path (no APE, no policy).
- Compressed/encrypted frames cannot be copied into an unsynchronised tag or
  one with an extended header or footer (the save is refused); frames lofty
  merged as duplicates on read cannot be matched to their stored bytes, so an
  encrypted one blocks the save.
- A 131-byte APE tag whose items (not its header) put "TAG" where ID3v1
  starts is still misread by lofty; only the header case is handled. lofty
  also mishandles header-less APEv2 tags in general (it always assumes a
  header), which this branch does not change.
- DSF/DFF ID3v2 writes decompress compressed frames and refuse encrypted ones
  (no byte copy there).
- Mp3tag's own default for "Use single MP4 atom for multiple values" is not
  documented; OpenTag keeps it on.
