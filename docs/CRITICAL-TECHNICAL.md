# Critical technical placeholder production

The [official format placeholder inventory](https://docs.mp3tag.de/format/) and [date formatting contract](https://docs.mp3tag.de/scripting/#date-and-time-formatting) were checked on 2026-10-14. This ledger covers all 72 names in the scripting audit. Resolution of supplied fields and production of real values are separate obligations; this document records production, including intentional absence.

`AudioEntry.technical` is a defaultable `Fields` map with uppercase underscore-prefixed names and string-array values. Stored tag metadata stays in `AudioEntry.fields`. Callers merge technical values into evaluation context while retaining `fields` as raw metadata for `$list`. `CoverInfo` adds defaultable `size: u64`, `width: Option<u32>`, and `height: Option<u32>`. Summary results keep `data_base64` empty; all other descriptors and technical properties match full reads. Old serialized entries lacking the new fields still deserialize.

The native evaluation adapter now merges measured file properties after its legacy derived fallbacks. This reaches scripting, computed columns, filename templates, action state and export-row context through their shared adapter. The file map cannot replace application/selection/clock/export-owned fields such as `_APP`, `_COUNTER`, loop totals, hashes or playlist context. Names are normalized before this check, and ordinary stored tag names are not accepted as technical overrides. Raw tags remain separately available to `$list`.

The filter RPC evaluates a caller-supplied field map. The frontend now merges optional `AudioEntry.technical` through `src/filter-fields.ts`, after its existing derived fallbacks. Its ownership guard matches the native adapter: measured names use ASCII uppercase, ordinary tag names cannot enter from the technical map, and application/selection/clock/export-owned fields cannot be replaced. The RPC still receives merged raw tags separately as `raw_fields`, and the current table-order `_COUNTER` is applied after the projection. Legacy entries without a technical map retain their prior fallback behavior. This closes the filter-payload integration gap; no GUI or document-provider workflow is qualified by these source/helper checks.

| Placeholder | Production status | Source / absence reason |
| --- | --- | --- |
| `%_bitrate%` | Measured, conditional where noted | Parsed audio bitrate in kbit/s; absent if the parser cannot measure it. |
| `%_bitspersample%` | Measured, conditional where noted | Parsed bit depth when available; absent for perceptual codecs without meaningful PCM bit depth. |
| `%_codec%` | Measured, conditional where noted | Concrete MPEG layer/version, MP4 codec, WAV format, AIFF compression, or recognized elementary codec. Unknown native IDs remain literal; no container-to-codec guess for MP4. |
| `%_cover_description%` | Measured, conditional where noted | First cover description; absent when no cover exists. |
| `%_cover_mimetype%` | Measured, conditional where noted | First cover MIME type; absent when no cover exists. |
| `%_cover_size%` | Measured, conditional where noted | First cover encoded byte length, before base64. |
| `%_cover_type%` | Measured, conditional where noted | First cover native type rendered as a readable label. |
| `%_cover_height%` | Measured, conditional where noted | JPEG/PNG header dimension only; absent for unsupported/malformed headers. |
| `%_cover_width%` | Measured, conditional where noted | JPEG/PNG header dimension only; absent for unsupported/malformed headers. |
| `%_covers%` | Measured, conditional where noted | Measured cover count, including zero. |
| `%_id3v2_character_encoding%` | Measured, conditional where noted | Distinct encodings from parsed ID3 string frames, including retained v2.3 IPLS. |
| `%_id3v2_unknown_frames%` | Measured, conditional where noted | Sorted native IDs retained as opaque binary by this backend, excluding recognized PCST/IPLS. Original-app unsupported-frame classification remains unqualified. |
| `%_length%` | Measured, conditional where noted | Parsed duration rendered as minutes:seconds, whole seconds. |
| `%_length_seconds%` | Measured, conditional where noted | Parsed duration rounded to whole seconds. |
| `%_mode%` | Measured, conditional where noted | Actual MPEG channel mode, otherwise measured channel count rendered as Mono/Stereo/N channels. |
| `%_mp4_creation_time%` | Measured, conditional where noted | `mvhd` creation time from the MP4 header; empty when absent or zero (`mp4_chapters.rs`). |
| `%_mp4_creation_time_raw%` | Measured, conditional where noted | Unix seconds of the `mvhd` creation time. |
| `%_mp4_modification_time%` | Measured, conditional where noted | `mvhd` modification time from the MP4 header. |
| `%_mp4_modification_time_raw%` | Measured, conditional where noted | Unix seconds of the `mvhd` modification time. |
| `%_samplerate%` | Measured, conditional where noted | Parsed audio sample rate in Hz; absent if unavailable. |
| `%_subsong%` | Measured, conditional where noted | Chapter index on chapter rows (`subsongs.rs`). |
| `%_subsong_count%` | Measured, conditional where noted | Chapter count for MP4, Matroska and cue-sheet files. |
| `%_tag%` | Measured, conditional where noted | Available concrete tag kinds; ID3 original version retained. Display spelling/separators need original-app differential qualification. |
| `%_tag_read%` | Measured, conditional where noted | First tag according to the existing primary/secondary read precedence. Mixed-tag fields can also include secondary unique fields. |
| `%_tag_size%` | Measured, conditional where noted | Total tag bytes in the file (`media_info.rs`). |
| `%_tag_size_appended%` | Measured, conditional where noted | Tag bytes after the audio data. |
| `%_tag_size_prepended%` | Measured, conditional where noted | Tag bytes before the audio data. |
| `%_tool%` | Measured, conditional where noted | Literal stored Vorbis vendor for FLAC/Vorbis/Opus/Speex, when present; no invented ENCODER metadata field. |
| `%_vbr%` | Absent from audio technical map | Not yet measured: no bitrate-mode guess is made from an averaged bitrate or codec name. |
| `%_video_bitrate%` | Measured, conditional where noted | Video stream property for MP4 and Matroska files (`mp4_chapters.rs`, `matroska.rs`); absent for audio-only files. |
| `%_video_codec%` | Measured, conditional where noted | Video stream property for MP4 and Matroska files (`mp4_chapters.rs`, `matroska.rs`); absent for audio-only files. |
| `%_video_height%` | Measured, conditional where noted | Video stream property for MP4 and Matroska files (`mp4_chapters.rs`, `matroska.rs`); absent for audio-only files. |
| `%_video_width%` | Measured, conditional where noted | Video stream property for MP4 and Matroska files (`mp4_chapters.rs`, `matroska.rs`); absent for audio-only files. |
| `%_directory%` | Measured, conditional where noted | Canonical source parent directory name. |
| `%_extension%` | Measured, conditional where noted | Actual source extension, preserving spelling. |
| `%_filename%` | Measured, conditional where noted | Canonical source filename without extension. |
| `%_filename_ext%` | Measured, conditional where noted | Canonical source filename with extension. |
| `%_folderpath%` | Measured, conditional where noted | Canonical parent path with one trailing platform separator. |
| `%_path%` | Measured, conditional where noted | Canonical source path; staging names are refreshed after atomic persistence. |
| `%_parent_directory%` | Measured, conditional where noted | Canonical source grandparent directory name. |
| `%_volume%` | Absent from audio technical map | Not yet measured: platform volume label. |
| `%_workingdir%` | Absent from audio technical map | Caller-owned runtime context; not produced by audio reads. |
| `%_workingpath%` | Absent from audio technical map | Caller-owned runtime context; not produced by audio reads. |
| `%_file_create_date%` | Measured, conditional where noted | Filesystem birth time, local short date; absent when the filesystem does not supply creation time. |
| `%_file_create_datetime%` | Measured, conditional where noted | Filesystem birth time, local date/time; absent when unavailable. |
| `%_file_create_datetime_raw%` | Measured, conditional where noted | Filesystem birth time in signed Unix seconds, absent when unavailable. |
| `%_file_mod_date%` | Measured, conditional where noted | Filesystem modification time, local short date, if available. |
| `%_file_mod_datetime%` | Measured, conditional where noted | Filesystem modification time, local date/time, if available. |
| `%_file_mod_datetime_raw%` | Measured, conditional where noted | Filesystem modification time in signed Unix seconds, if available. |
| `%_file_size%` | Measured, conditional where noted | Actual filesystem byte size, readable B/KB/MB representation. |
| `%_file_size_bytes%` | Measured, conditional where noted | Actual filesystem byte size. |
| `%_file_size_kb%` | Measured, conditional where noted | Actual bytes divided by 1024, integer. |
| `%_file_size_mb%` | Measured, conditional where noted | Actual bytes divided by 1024 squared, two decimal places. |
| `%_app%` | Native context | Native evaluation application identity. |
| `%_counter%` | Native context | Native evaluation selection index; export uses loop index. |
| `%_date%` | Absent from audio technical map | Caller-owned clock/locale context; not snapshotted into file metadata. |
| `%_datetime%` | Absent from audio technical map | Caller-owned clock/locale context; not snapshotted into file metadata. |
| `%_filename_rel%` | Export only | Relative path supplied by pure export renderer. |
| `%_folderpath_rel%` | Export only | Relative folder supplied by pure export renderer. |
| `%_crc%` | Export only | Native export host streams CRC32 only when requested. |
| `%_md5%` | Export only | Native export host streams full-file MD5 only when requested. |
| `%_md5audio%` | Absent from audio technical map | Explicitly unsupported by native export host until codec-specific audio hashing is qualified; no fake hash. |
| `%_max_counter%` | Export only | Export loop state. |
| `%_total_files%` | Export only | Export loop totals. |
| `%_total_size%` | Export only | Export loop totals and formatting. |
| `%_total_size_raw%` | Export only | Export loop byte totals. |
| `%_total_time%` | Export only | Export loop duration totals with days. |
| `%_total_time2%` | Export only | Export loop duration totals without days. |
| `%_total_time_raw%` | Export only | Export loop seconds. |
| `%_playlist_filename%` | Absent from audio technical map | Playlist output context; not produced by audio reads. |
| `%_playlist_filename_ext%` | Absent from audio technical map | Playlist output context; not produced by audio reads. |
| `%_playlist_folderpath%` | Absent from audio technical map | Playlist output context; not produced by audio reads. |

## Timestamp provenance and limits

The official docs identify raw file timestamps but do not specify their numeric epoch. Direct Mp3tag users report Unix seconds and provide reproducible conversions: [the 2012 file-date filter examples](https://community.mp3tag.de/t/a-tip-for-filtering-by-file-dates/13545) and [the working Unix-to-FILETIME conversion](https://community.mp3tag.de/t/convert-date-and-time-string-into-raw-epoch-time/59512). These are firsthand user evidence, not an explicit implementation guarantee from the author. [Florian's introduction of fmtDate](https://community.mp3tag.de/t/date-transformation-function-requested/71556/2) confirms that the function consumes the same raw timestamp family. Our implementation uses filesystem `SystemTime` converted to signed UTC Unix seconds and tests a known 1700000000 modification time through the real `$fmtDate` evaluator.

Formatted file dates use the local timezone and the system locale's Chrono locale patterns. Birth time is omitted when unavailable; Unix ctime is never substituted for creation time. User-customized Windows regional overrides and native original executable comparisons remain unqualified. Movie timestamps in MP4 have a separate epoch and are read from the `mvhd` header by `mp4_chapters::header_technical`.

## Performance and preservation

All supported formats now use one concrete metadata/properties parse for each summary or full read. In particular, FLAC native custom fields, multivalues, ID3 read-only metadata, and picture payloads are retained from one `FlacFile` parse. Native tag snapshots are retained before moving the concrete file into the generic representation; no second metadata read is needed. APE pictures are enriched from the already parsed native binary items. Summary reads serialize no encoded image data. Picture dimensions use Lofty's JPEG/PNG header inspection without decoding full images; other image types return absent dimensions rather than fabricated zeroes. A bounded JPEG-header guard prevents Lofty 0.25.4's unchecked segment-length subtraction and precision multiplication from panicking on malformed embedded pictures; the regression was observed failing before the guard and passes afterward.

The current native metadata library still reads embedded picture bytes internally. This work removes repeated parses and serialized picture payloads; it does not claim a zero-allocation or picture-byte-skipping parser. Technical maps contain small measured strings and do not duplicate image bytes. Hashes are not calculated during scans.

The tag names, cover-type labels, mixed-tag precedence, duration rounding, file-size formatting, opaque-frame classification, and codec-name presentation require original-executable differential qualification for exact display parity. The source measurements and absence behavior are covered by generated-audio tests. This is not full technical-placeholder parity: bitrate/compression-profile labels, platform volume labels, runtime clock/working-directory context, playlist context, and audio-only MD5 remain separate obligations.

## Verification

Native integration tests in `src-tauri/src/evaluation.rs` use disposable PCM WAV files with custom multivalue tags, embedded PNG art and a known modification timestamp. They verify real codec, bit depth, cover dimensions/type and raw timestamp values through the native scripting/column/filename adapters; measured values override stale legacy projections while application context remains authoritative. A native action-group test writes evaluated fields, removes artwork, evaluates fresh post-removal properties, exports the result, and checks that unrelated tag values and PCM bytes survive. These tests failed on the missing merge before the adapter change. They establish the native integration contract, not original-application runtime equivalence or GUI/device user-testing.

Adapter checkpoint validation used matching Rust/Cargo/Rustdoc 1.95 binaries and the isolated `target/technical-integration` directory: `cargo test -p opentag --lib --locked` passed all 23 native tests, `cargo clippy -p opentag --all-targets --locked -- -D warnings` passed, and the touched Rust file passed rustfmt checking. `npm run build` also passed to supply the frontend assets needed by native context generation. No GUI or user music was accessed for this integration checkpoint.

Frontend filter-payload validation: `npm run test:ui` passed all 40 Node tests, including five focused filter-field cases, and `npx tsc --noEmit` passed. The tests exercise measured codec/bit depth/file-time/cover values, measurement-over-fallback precedence, exact raw metadata/multivalue preservation, all reserved context names including mixed-case variants, older serialized entries, Windows path fallbacks, and absent unmeasured properties/aliases. No Vite build, native rebuild, GUI interaction or user-library access was performed for this frontend checkpoint; bundled frontend assets remain unchanged until the coordinated build.

`cargo test -p tag-core --test technical --test metadata --test mappings` covers measured WAV properties, known filesystem timestamps usable by `$fmtDate`, serialization compatibility, actual ID3v2.3 version/encoding/opaque-frame IDs, JPEG/PNG header dimensions, absent unsupported image dimensions, summary/full and mutation equality, raw-field separation, and actual FLAC/AAC/ALAC/MP3/Vorbis/Opus/AIFF codec fixtures. Existing tests also retain native mapping transport and encoded-audio packet invariance. Encoded fixtures use ffmpeg, with an explicit skip notice if ffmpeg is absent; the verification machine had ffmpeg and executed those cases. Fixture writes are disposable. The user's music library remains read-only.

Read-only library transport check: the 434-file Beatport folder produced 434 entries, zero errors, 1,073,982 JSON bytes, and zero encoded-cover bytes with the new technical maps. Debug timings were measured while other builds were active and are not treated as comparative performance evidence.

Native UI checkpoint: the actual macOS arm64 debug build with executable SHA-256 `ea605ca210c821eb49e7720001877f775c6454a5159522b9e41c5b57ae149307` exercised measured `_CODEC`, `_COVER_WIDTH` and `_COVERS` filters on six disposable fixtures. Retained AX records show AAC selecting one M4A, width 600 selecting two covered MP3s, and cover removal refreshing matches from two to one to zero; the final zero-cover filter displayed all six rows. `_BITSPERSAMPLE IS 16` selecting FLAC/WAV was operator-observed without retained AX. [Workflow and receipt](verification/technical-cover-workflows.md) distinguish those evidence levels and verify packet/PCM and format-tag-dictionary preservation. This closes these bounded desktop filter/cover-refresh UI cases; original equivalence, other placeholders, other cover-mutation formats and mobile/provider workflows remain unqualified.
