# Audio MD5: implemented scope and remaining qualification

The [official format-string reference](https://docs.mp3tag.de/format/) lists `%_md5audio%` in its Export section. It describes stored audio checksums for FLAC/WavPack and calculated audio-region checksums for ID3/APE-tagged streams. This documentation observation does not establish use in other contexts or support for MP4/Ogg.

The current implementation already supplies that named export route:

1. The native [action host](../../src-tauri/src/action_host.rs) obtains requested hash fields from the template/options, computes them into each row's context fields, then invokes the export renderer.
2. The [hash adapter](../../src-tauri/src/file_hashes.rs) uses a stored checksum, hashes the selected audio range or emits an empty value when the core reports it unavailable. Whole-file CRC/MD5 requests share the streaming pass; tag values cannot substitute for these context fields.
3. The [core classifier](../../crates/tag-core/src/media_info.rs) recognizes FLAC STREAMINFO, a WavPack MD5 metadata block and bounded audio ranges for the named ID3/APE stream families. All-zero FLAC checksums, WavPack files without the inspected MD5 block, and other formats remain explicitly unavailable. The WavPack path accepts a present 16-byte checksum without the FLAC zero-value guard.
4. The [export renderer](../../crates/tag-core/src/export.rs) resolves `_MD5AUDIO` through export context fields. Existing [adapter tests](../../src-tauri/src/file_hashes.rs) inspect a known checksum vector and ID3-tag exclusion; [core tests](../../crates/tag-core/tests/placeholders.rs) inspect the stored FLAC checksum, and [export tests](../../crates/tag-core/tests/export.rs) cover hash requests through output/partition expressions. Those tests were inspected, not rerun for this source audit.

The hash adapter's merged lineage is `abc72c60f15047a6d3053f86e8c31e6d09a32405`; the audit inspected PR64 head `2a5b8dd6d83df7a3a8d93d49baca9d93b053ca8c`. No product code changed, hash runtime ran or original application executed.

The fetched documentation HTML, extracted section record and seven source/test bindings are retained in `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-export-audio-hash-scope-s8jzm0cf`. HTML SHA-256: `ce27f4da8eb9305b439522ce63c5aaebcf510a6e7ea76e54c17b8e373eabf0d1`; receipt SHA-256: `e31bb9276e9db976b178d294244cf98dd34b75f401bd76912eb382757c3d14f8`. The live reference is not a version-pinned original-runtime oracle.

L42 belongs in qualification alongside the covering implementation, rather than asking to implement the existing documented route again. Paired original checksum results, broader real-file layouts, absent-checksum behavior and any demonstrated requirement outside the documented context remain unqualified. No outside-export or additional-container implementation is justified by the evidence here; a concrete discovered defect can return as its own narrowed remainder.
