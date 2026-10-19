# Scripting, filter, placeholder and export parity — 2026-10-15

Branch `feat/script-export`, started from `feat/core-parity` at `8a8d6f9` and merged with `feat/core-parity` at `bd5e632` before this historical report. Its former closure claims describe the implementation checkpoint, not current original-runtime acceptance. Use the [uncovered ledger](../PARITY.md) and [coverage companion](../research/remaining-gaps.md) for remaining work. Nothing here was checked against the original program; each decision below names the documentation it follows and the reading taken where the documentation leaves room.

## Decisions

### `$regexp` case handling

The scripting reference says the fourth argument "enables ignore case (1) or disables the ignore case setting (0)". Disabling a setting only makes sense if it is on by default, so `$regexp(x,e,r)` now ignores case, `c=1` keeps ignoring it and `c=0` matches case. Any other value of `c` is treated like the default. Boost's inline modifiers still override either default: `(?-i)` and `(?i)`, and scoped forms such as `(?i:…)`.

Boost applies case-insensitivity to its character classes too. With the new default `\u` (an upper-case letter in Mp3tag's regex reference) also matches lower-case letters; `$regexp(x,'\u',y,0)` or `(?-i)\u` keeps the strict class. The existing test was changed to pin this.

Only `$regexp` changed. Replace with regular expression actions, the filter's `MATCHES` and the converters keep their own case options.

### Filter `GREATER`, `LESS`, `EQUAL`

The numeric operand on the right must still be an integer, as the filter reference says. Field values are compared by their leading integer: optional leading spaces and sign, then digits, and anything after that is ignored. `3/12` compares as 3, ` 02 / 03` as 2, `2024-05-01` as 2024 and `128.6` as 128. A value that does not start with a number takes no part in the comparison. It is not treated as 0, so `mood EQUAL 0` does not match `calm`. Digit runs too long for 64 bits saturate rather than drop out.

### Placeholders

| Placeholder | Source | Notes |
|---|---|---|
| `%_workingdir%`, `%_workingpath%` | The folder the file list was last changed to (Change directory, Upper/Previous/Next directory, a launch with `/fp:`). The window reports each change with the new `set_working_directory` command. | `_workingpath` ends with a separator, like `_folderpath`. Both are absent before a folder is opened, and added folders (Add directory) do not change them. They reach format strings, columns, actions and filters, both direct (`_WORKINGDIR IS rock`) and scripted. Correction: when this was written they did not reach exports (the export context had no working folder; the e2e check covered only the format preview), Filename → Filename or tag-source cover names. `convert-export.md` closes that. |
| `%_vbr%` | MPEG: `Xing` → VBR, `Info` → CBR, `VBRI` → VBR, none → CBR. When a LAME tag is present its VBR method decides: 1/8 CBR, 2/9 ABR, 3–6 VBR. Monkey's Audio: the compression level (Fast, Normal, High, Extra High, Insane). | Other formats leave it absent. |
| `%_tool%` | The encoder string stored after the MPEG Xing/Info header (for example `LAME3.100`, or `Lavc63.1.` from ffmpeg). The Vorbis vendor string is kept for Ogg and FLAC. | |
| `%_tag_size%`, `%_tag_size_prepended%`, `%_tag_size_appended%` | ID3v2 tags at the start; APEv1/v2 (with or without a header), Lyrics3v2, ID3v1 and footer-marked ID3v2 at the end, in any order. FLAC adds its Vorbis comment and picture blocks, with their block headers, to the prepended size. | MPEG, AAC/ADTS, Monkey's Audio, Musepack, WavPack, FLAC, and TTA/TAK/OptimFROG through their stand-in. Chunk-based and box-based containers (WAV, AIFF, MP4, Ogg, Matroska, ASF, DSD) leave them absent. |
| `%_mp4_creation_time%`, `%_mp4_modification_time%` and `_raw` | `mvhd`, version 0 and 1. | The formatted value is the long local date and time, like `%_file_mod_datetime%`. `_raw` is Unix seconds, which is what OpenTag's `$fmtDate` takes; the scripting reference calls `$fmtDate`'s input "the raw timestamp". A zero timestamp means "not set" and leaves both absent. |
| `%_video_codec%`, `%_video_width%`, `%_video_height%`, `%_video_bitrate%` | MP4/M4V: the first track whose handler is `vide`. The codec comes from its sample entry (H.264, H.265, MPEG-4 Visual, AV1, VP8/VP9, H.263, Motion JPEG, Apple ProRes, else the four-character code). The size comes from the visual sample entry, or from `tkhd` if that is missing. The bitrate is the sum of the `stsz` sizes over the `mdhd` duration, in kbit/s. Matroska: the first video track's codec ID, `PixelWidth`/`PixelHeight`, and the bitrate from a `BPS` statistics tag aimed at that track. | Matroska files without `BPS` (ffmpeg writes none) have no video bitrate. |
| `%_volume%` | macOS: the `/Volumes/<name>` folder, or the `/Volumes` entry that links to `/` for the startup disk. Linux: the longest mount point holding the file, matched to `/dev/disk/by-label`. Windows: `GetVolumeInformationW` on the drive root. | Absent when no label is found. The root label and Linux mount table are cached for 30 seconds. |
| `%_subsong_count%` | Now also counts ID3v2 `CHAP` frames (MP3, AIFF, WAV with ID3). | MP4 and Matroska were already covered. |
| `%_md5audio%` (exports) | FLAC: the STREAMINFO MD5. WavPack: the `ID_MD5_CHECKSUM` sub-block of the first block. MPEG, AAC/ADTS, Monkey's Audio, Musepack, TTA, TAK and OptimFROG: MD5 of the bytes between the prepended and appended tags, computed in the same pass as `%_crc%`/`%_md5%`. | Other formats, a FLAC file with an all-zero checksum and a WavPack file without the block get an empty value. The export no longer stops. |
| `%_length%` | `m:ss` below an hour, `h:mm:ss` from one hour on. | It now uses the same rounded seconds as `%_length_seconds%`, so 59.6 s is `1:00` and 60 everywhere. |
| `%_app%` in filters | The filter cache now adds it next to `%_counter%`. | |
| `%_counter%` in tag-source cover names | 1-based position among the written files. | |

Not implemented: `%_subsong%`. Mp3tag fills it when it lists each chapter of a file as its own entry. OpenTag lists a file once, so no per-entry chapter index exists to report.

### Export: ANSI off Windows and dropped `.mte` files

`$filename(…,ansi)` and non-UTF-8 `.mte`, export and Text File → Tag sources fall back to Windows-1252 when the system has no ANSI codepage (macOS, Linux). Playlist reading already did this. An explicitly given codepage still has to decode cleanly.

The export library now supports two kinds of configuration side by side:

- Configurations made in OpenTag keep their `<id>.mte` file named by UUID (or `text`, `cue-sheet`).
- Any other `.mte` file in the export folder, for example one copied from Mp3tag, is listed under its file name. Its identity, `mte-<slug>-<crc32 of the file name>`, stays stable across loads, so it can be selected before the library is saved. Saving records the file name as `file` in `exports.json`.

When the library is saved, a file whose decoded text is unchanged is not rewritten, so an untouched ANSI file keeps its bytes. An edited folder file is written as UTF-8 with a BOM so that Mp3tag does not read it as ANSI. Removing a configuration moves its file to `export/removed/`. A folder file deleted outside OpenTag leaves the list, and the selection falls back to the first configuration. `file` must be a plain `.mte` name inside the export folder and unique without regard to case. Duplicating a configuration in the dialog gives the copy its own UUID file.

One limit: if a file is dropped into the folder while the Export dialog is open and the dialog then saves, that file is moved to `export/removed/`, because the saved library does not list it. Nothing is deleted.

### `.mta` settings for every action type

Only types 1 and 2 appear in Mp3tag's shipped action files. Their structure is a `[#n]` section per action, `T=<type>`, `F=<field>` and numbered settings, with Replace flags written as `whole word|case sensitive`. The other documented type IDs use that layout, with their settings in the order the action dialog shows them. These schemas are OpenTag's reading and have not been checked against files saved by Mp3tag.

| T | Action | F | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|---|
| 1 | Case Conversion | field | mode: 0 lower, 1 Mixed, 2 Sentence, 3 UPPER (documented order; 1 verified) | word separators | | | |
| 2 | Replace | field | search | replace | `whole word\|case sensitive` (verified) | | |
| 4 | Replace with regular expression | field | pattern | replacement | case sensitive 0/1 | | |
| 5 | Format value | field | format string | | | | |
| 6 | Remove duplicate fields | field | only same content 0/1 | | | | |
| 7 | Guess values | source format | guessing pattern | | | | |
| 8 | Merge duplicate fields | field | separator | | | | |
| 9 / 10 | Remove fields / Remove fields except | `;`-separated fields | | | | | |
| 11 | Convert codepage | field | codepage | | | | |
| 12 | Import cover from file | file name format | picture type (ID3 number, empty = front cover) | delete existing 0/1 | | | |
| 13 | Export cover to file | file name format | picture type (ID3 number, empty = all) | export duplicates 0/1 | | | |
| 14 | Import text file | field | file name format | | | | |
| 15 | Export | configuration | file name format | append 0/1 | partition format (absent = none) | | |
| 16 | Split field by separator | field | separator | | | | |
| 1001 | Adjust cover (OpenTag ID) | | format: 0 original, 1 JPEG, 2 PNG | quality | size in pixels (absent/0 = no resize) | 0 max., 1 min. size | 0 proportional, 1 square, 2 crop |
| 1002 | Set cover properties (OpenTag ID) | | picture type (empty = keep) | description (absent = keep) | only this picture type (empty = all) | | |
| 1003 | Generate playlists (OpenTag ID) | file name format | partition format | | | | |

The last three types use application-specific IDs1001–1003. Their original equivalents, collision behavior and original reader/import/edit/re-import results have not been established. Export warns about these one-way extensions; do not treat the numeric range as proof of interoperability. Import stays strict: unsupported keys/options or invalid recognized settings remain disabled unresolved records. Later [opaque-record preservation](mta-opaque-preservation-2026-10-18.md) preserves accepted decoded values and record order, including opaque CR/LF and adjacent literal backslashes. It does not promise original bytes, source encoding or escape spelling. Recognized-action settings containing line breaks remain refused where their schema cannot represent them.

### Export Cover and Adjust Cover

Export Cover always ends the file name with the image type's extension (`jpg`, `png`, `gif`, `bmp`, `webp`, `tif`; the bytes decide when the MIME type is unknown, else `img`). A name already ending in that extension (`jpeg` counts for JPEG) is kept. Another image extension is replaced (`cover.jpg` holding PNG data → `cover.png`). Anything else is appended, so `Vol. 2` gives `Vol. 2.jpg`. Because an image extension is always present, a name aimed at the input audio file now writes `input.wav.png` beside it.

Adjust Cover with max. size leaves a cover whose larger side is within the size untouched, in all three scale modes. Larger covers scale proportionally, to a square, or are cropped to a square. With min. size the smaller side is matched up or down, and square and crop give a square of that size. A cover that ends up with its original size and format is kept byte for byte instead of being re-encoded. Original format now also resizes GIF, BMP and WebP covers in their own format; TIFF and unknown formats still ask for JPEG or PNG.

### Text File → Tag source

The source file field accepts a format string such as `%_folderpath%tracks.txt`. It is evaluated for the first selected file; a relative result is relative to that file's folder, and an empty result is refused. A file that exists under the literal name is used as named, even if the name contains `%` or `$`. The preview reports which file it read ("Reviewed 2 files from …/tracks.txt"), and the field offers the format-string helper. Apply uses the snapshot taken at preview, as before.

## Verification

All on macOS 15 (Darwin 24.6), after the merge with `feat/core-parity`:

- `cargo test --workspace` (with `AR=/usr/bin/ar`): 552 passed, 0 failed, 4 ignored.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. The platform-specific `%_volume%` code was also linted on its own for `aarch64-unknown-linux-musl` (with its tests) and `aarch64-pc-windows-msvc`, because the workspace cannot cross-compile the Boost bridge here.
- `cargo fmt --all`, `npx tsc --noEmit`: clean.
- `node --test tests/*.test.mjs`: 246 passed.

New or changed unit tests:

- `crates/tag-core/tests/script.rs`: `regexp_ignores_case_by_default_and_honours_the_flag_and_inline_modifiers`, `working_directory_placeholders_come_from_the_context_and_reach_filters`, and the Boost `\u` case.
- `crates/tag-core/tests/filter.rs`: `numeric_filters_compare_the_numeric_part_of_values`.
- `crates/tag-core/src/media_info.rs`: tag spans (ID3v2, APE with header, Lyrics3v2, ID3v1), Xing/Info/VBRI/LAME bitrate types, Monkey's Audio levels (old and new headers), FLAC and WavPack stored MD5s, the MPEG audio range, FLAC tag blocks, macOS volume names and Linux escape decoding.
- `crates/tag-core/tests/placeholders.rs`: ffmpeg-encoded files for VBR/CBR and the encoder string, tag sizes against the real file layout, FLAC STREAMINFO MD5, MP4 `mvhd` time (`creation_time=2024-01-02T03:04:05Z` → `1704164645`) and video properties, Matroska video properties, ID3 `CHAP` counts, and `%_length%` with hours. These are skipped without ffmpeg.
- `src-tauri/src/file_hashes.rs`: audio MD5 over the bytes between the tags, matching an untagged copy, and an empty value for non-audio.
- `src-tauri/src/filter_cache.rs`: `%_app%` and `%_workingdir%` in cached filters. `src-tauri/src/sources/mod.rs`: 1-based `%_counter%`.
- `crates/tag-core/tests/export.rs`, `crates/tag-core/tests/converters.rs`: the Windows-1252 fallback.
- `src-tauri/src/exports.rs`: dropped `.mte` listing, stable identities, saving under the file name, untouched ANSI bytes, a file dropped after saving, deletion in the folder, retiring on removal, and path-escape refusal.
- `crates/tag-core/src/actions.rs`: every action type round-trips through `.mta`, Mp3tag's layout is checked for type 12 and Upper case, and strictness covers unknown values and keys. `src-tauri/src/action_host.rs`: UTF-16 `.mta` round trip for Adjust Cover, and refusal without a file for line breaks.
- `src-tauri/src/action_host.rs`: `export_cover_appends_or_replaces_the_image_extension` and the changed input-alias case. `crates/tag-core/src/cover_ops.rs`: max./min. size in all scale modes, no re-encoding of unchanged covers, and GIF kept as GIF.
- `src-tauri/src/converters.rs`: `text_source_can_be_a_format_string_for_the_first_selected_file`.
- `tests/file-ops.test.mjs`: working-directory notifications. `tests/export-ui.test.mjs`: a duplicate gets its own file.

Native UI: `tests/e2e/script-export.js` was added to the `native-e2e` job in `.github/workflows/build.yml`. It checks the working-folder placeholders after `/fp:`, MP3 `%_vbr%` and tag sizes, `$regexp` case handling, an ANSI export with `%_md5audio%`, a `.mte` file written into the export folder appearing in the library, `.mta` round trips of every action type, and the Text File → Tag dialog previewing a format-string source. Following the no-local-windows rule, it has not been run outside CI.

## Open

- No differential evidence against Mp3tag for any decision above. The `.mta` schemas of types 4, 6 and 11–15 are the main risk if Mp3tag orders those settings differently.
- `%_vbr%` for Musepack and WavPack profiles; `%_tool%` beyond MPEG encoder strings and Vorbis vendors; tag sizes for chunk and box containers.
- `%_subsong%` (see above).
- A file dropped into the export folder while the Export dialog is open is retired if the dialog saves.
