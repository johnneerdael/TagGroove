# Bounded ID3v2.3 extended-header preservation — 2026-10-18

Scope: a concrete metadata-loss fix within L26 in consolidated draft PR64. Independently authored local files and the actual backend establish the defect and bounded support below. No original program was run, and Windows work is deferred by the owner.

## Defect and resulting behavior

Valid ID3v2.3 extended headers, with and without a correctly calculated CRC, previously produced accepted but empty metadata reads in authored MP3, DSF and DSDIFF files. An unrelated ARTIST save then removed TITLE, compressed TALB, encrypted TXXX and its ENCR registration. Media bytes were unchanged, and Undo restored the original file. This was accepted destructive output, so refusing every valid file would not deliver the required support.

The fix validates the non-unsynchronised v2.3 extension, then supplies the existing reader with a read-only normalized view: clear the extension flag, shift frames/padding left inside the unchanged declared body, and zero-pad the freed tail. The declared tag size, physical media and container offsets stay unchanged. Translate encrypted fallback patches into that view; retain raw original offsets/bytes for opaque-frame preservation. Validate extension length/flags, padding, CRC and physical file/owning ID3 chunk bounds before normalization. Allocation is fallible, and read intersections avoid copying the complete metadata on every small read.

Successful saves produce a plain v2.3 envelope without the optional extension/CRC. That is an explicit serialization change, not checksum retention or proof of Mp3tag output equivalence. Unedited compressed/encrypted records and ENCR registration retain their original bytes. Undo restores the complete original file including extension/checksum; Redo restores the successful output.

## Executed evidence

- [Reader/overlay](../../crates/tag-core/src/id3_frames.rs) and [actual-file preservation tests](../../crates/tag-core/tests/id3_opaque_preservation.rs).
- Four focused reader units and 43 adjacent integrations passed before the additional PCM-container controls.
- The 24-case MP3/DSF/DSDIFF matrix requires successful read/save for its 15 supported cases, and explicit protected refusal for nine unsupported cases. Checks include fields, opaque records, media and exact Undo/Redo.
- Another 24 malformed/truncated/flags/padding/CRC/combined-unsync cases require refusal with original bytes, prior Undo and retained budget intact. An actual headerless APE overlay-composition control retains both tag systems and MPEG payload.
- Additional valid CRC-v3 WAV/AIFF controls and owning-chunk overruns exercise PCM/container bounds. Both valid controls require successful writes and passed preservation/Undo/Redo; both malformed controls require refusal with original bytes/history intact. Refusal alone cannot pass a positive control.
- Python-standard-library inspection independently checked authored CRCs, zlib data, record/media/whole-file hashes and backend field maps. Fixtures are synthetic preservation inputs with registered encryption markers, not a third-party encryption implementation.

The exact direct `crc32fast =1.5.2` dependency was already present in the lockfile; no package upgrade was made. Existing splice/refusal guards remain.

## Remaining scope

L26 stays open: broader ADTS probing, combined global unsync/extended-header transformations, v2.4 CRC/footer opaque writes, arbitrary compression/encryption/grouping and other wrappers/layouts are unqualified. In the named matrix, refused cases preserve input/history; that does not claim complete support. The extension-only empty-body adapter unit tests offset safety, not standards validity of a zero-frame tag. Original/native/platform parity remains separate acceptance.

Primary format definitions: [ID3v2.3](https://id3.org/id3v2.3.0) and [ID3v2.4 structure](https://id3.org/id3v2.4.0-structure).
