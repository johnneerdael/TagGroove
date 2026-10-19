# Bounded ID3 optional-envelope preservation — 2026-10-18

Scope: a concrete metadata-loss fix within L26 in consolidated draft PR64. Independently authored local files and the actual backend establish the defect and bounded support below. No original program was run, and Windows work is deferred by the owner.

## Defect and resulting behavior

Valid ID3v2.3 extended headers, with and without a correctly calculated CRC, previously produced accepted but empty metadata reads in authored MP3, DSF and DSDIFF files. An unrelated ARTIST save then removed TITLE, compressed TALB, encrypted TXXX and its ENCR registration. Media bytes were unchanged, and Undo restored the original file. This was accepted destructive output, so refusing every valid file would not deliver the required support.

The fix validates the non-unsynchronised v2.3 extension, then supplies the existing reader with a read-only normalized view: clear the extension flag, shift frames/padding left inside the unchanged declared body, and zero-pad the freed tail. The declared tag size, physical media and container offsets stay unchanged. Translate encrypted fallback patches into that view; retain raw original offsets/bytes for opaque-frame preservation. Validate extension length/flags, padding, CRC and physical file/owning ID3 chunk bounds before normalization. Allocation is fallible, and read intersections avoid copying the complete metadata on every small read.

Successful saves produce a plain v2.3 envelope without the optional extension/CRC. That is an explicit serialization change, not checksum retention or proof of Mp3tag output equivalence. Unedited compressed/encrypted records and ENCR registration retain their original bytes. Undo restores the complete original file including extension/checksum; Redo restores the successful output.

## Executed evidence

- [Reader/overlay](../../crates/tag-core/src/id3_frames.rs) and [actual-file preservation tests](../../crates/tag-core/tests/id3_opaque_preservation.rs).
- Four focused reader units and 43 adjacent integrations passed before the additional PCM-container controls.
- At the initial v3-fix checkpoint, the 24-case MP3/DSF/DSDIFF matrix required successful read/save for 15 supported cases and explicit protected refusal for nine unsupported cases; the v4 follow-up below expands that coverage. Checks include fields, opaque records, media and exact Undo/Redo.
- Another 24 malformed/truncated/flags/padding/CRC/combined-unsync cases require refusal with original bytes, prior Undo and retained budget intact. An actual headerless APE overlay-composition control retains both tag systems and MPEG payload.
- Additional valid CRC-v3 WAV/AIFF controls and owning-chunk overruns exercise PCM/container bounds. Both valid controls require successful writes and passed preservation/Undo/Redo; both malformed controls require refusal with original bytes/history intact. Refusal alone cannot pass a positive control.
- Python-standard-library inspection independently checked authored CRCs, zlib data, record/media/whole-file hashes and backend field maps. Fixtures are synthetic preservation inputs with registered encryption markers, not a third-party encryption implementation.

The exact direct `crc32fast =1.5.2` dependency was already present in the lockfile; no package upgrade was made. Existing splice/refusal guards remain.

## Remaining scope

L26 stays open: broader ADTS probing, combined global unsync/optional-header transformations, update/restriction declarations, arbitrary compression/encryption/grouping and other wrappers/layouts are unqualified. The following bounded v2.4 slice supplies the named CRC/footer write support; it is not universal envelope support. In the named matrix, refused cases preserve input/history; that does not claim complete support. The extension-only empty-body adapter unit tests offset safety, not standards validity of a zero-frame tag. Original/native/platform parity remains separate acceptance.

Primary format definitions: [ID3v2.3](https://id3.org/id3v2.3.0) and [ID3v2.4 structure](https://id3.org/id3v2.4.0-structure).

## ID3v2.4 CRC/footer follow-up

The required-success CRC-v4 MP3 regression first refused an unrelated ARTIST save. The installed writer retained extension bytes but omitted their header flag; footer inputs reached the unchanged destination-splice envelope guard. The new read-only v4 view validates the extension/checksum and exact header-matching footer before normalization. Fixed-prefix/file/owning-chunk checks precede fallible metadata allocation. Frame IDs/synchsafe lengths, CRC width/data and footer/no-padding rules are checked before use.

Removing the optional extension shifts frames inside the same physical metadata extent. For a footer, its ten bytes become padding and the view's body length grows by ten; media/container offsets remain unchanged. Stored opaque frames are still copied from the original source, with encrypted fallback patches translated after the view patch. The v3 view and destination splice guard are unchanged. Successful same-revision saves deliberately produce plain v4 without the original checksum/footer; exact Undo restores the original representation.

The expanded real-file matrix has 30 MP3/DSF/DSDIFF paths: **27 required successful saves and three retained v3 global-unsync refusals**. It includes checksum-free v4 extensions and CRC+footer combinations. Each successful save checks ordinary fields, original compressed/encrypted/ENCR bytes, sound and exact Undo/Redo. Fourteen malformed v4 variants across three carriers add **42 read/write refusal paths** with original file bytes, prior Undo and retained budget intact. Existing v3 malformed and WAV/AIFF owning-chunk controls remain.

Five focused reader units and 45 adjacent integrations passed in the author batch, plus targeted Clippy, formatting and diff checks. Independent Python-standard-library inspection checked actual retained native frames/flags/CRC/zlib/media/whole-file stages against backend outputs; these are authored files and local backend evidence, not original-program files. Local static security review found no remaining issue on the frozen source. The root whole-core/combined validation is recorded separately after execution.

The format basis is [ID3v2.4 structure, sections 3.1–3.4 and 6](https://id3.org/id3v2.4.0-structure). CRC covers stored frame/padding data outside the extension/footer; the footer matches the header and excludes padding. Broader transformation/flag/location combinations and original/native-platform behavior remain open.

Root verification on the frozen v4 core source: `cargo test -p tag-core --locked` passed **704 tests, zero failures/ignored**, across 65 result groups. This includes the three new actual-Markdown scripting tests. Whole-workspace/hosted/native/original qualification is separate and remains pending.
