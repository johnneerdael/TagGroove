# Supported file formats

TagGroove reads and edits metadata in the formats below. The audio codec describes how samples are encoded; the container holds that stream and its tags. AAC and ALAC inside an MP4/M4A file therefore use MP4 metadata, while a raw `.aac` stream uses the ID3/APE policy.

!!! tip "New to this?"
    Identify the file's actual format and tag type before choosing fields or compatibility settings. Renaming `.flac` to `.mp3` does not convert the audio or its metadata.

## Read, write, and remove capabilities

**Read** describes exposed metadata. **Write** describes supported tag output, not a request to create every listed tag type. **Remove** describes types the applicable removal policy can strip. MPEG and APE-family policies under **Options → Tags** choose the actual read/write/remove sets.

| Format | Folder-scan extensions | Read | Write | Remove |
|---|---|---|---|---|
| AAC, raw ADTS | `.aac` | ID3v1, ID3v2, APEv1/v2 | ID3v1, ID3v2, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| AIFF / AIFC | `.aif`, `.aiff`, `.aifc` | ID3v2 | ID3v2 | ID3v2 |
| Monkey's Audio | `.ape` | ID3v1, APEv1/v2 | ID3v1, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| DSF | `.dsf` | ID3v2 | ID3v2 | ID3v2 |
| DSDIFF, a TagGroove addition | `.dff` | ID3v2 plus available DIIN artist/title | ID3v2; synchronize existing DIIN artist/title | ID3v2 and supported DIIN tag text |
| FLAC | `.flac` | Vorbis Comments | Vorbis Comments | Vorbis Comments and prepended ID3v2 |
| Matroska audio/video | `.mka`, `.mkv` | Matroska tags, supported chapters, image attachments | Same supported structures | Global tags, supported chapter structures/tags, image attachments; see limits below |
| MPEG audio | `.mp1`, `.mp2`, `.mp3` | ID3v1, ID3v2, APEv1/v2 | ID3v1, ID3v2, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| MP4 family, including AAC/ALAC audio | `.mp4`, `.m4a`, `.m4b`, `.m4v`, `.m4p` | MP4/iTunes metadata; supported chapters separately | MP4/iTunes metadata; supported chapter editing | MP4/iTunes metadata; use the chapter operation for explicit chapter removal |
| Musepack | `.mpc`, `.mp+`, `.mpp` | ID3v1, APEv1/v2 | ID3v1, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| Ogg Vorbis | `.ogg`, `.oga` | Vorbis Comments | Vorbis Comments | Vorbis Comments |
| Opus | `.opus` | Vorbis-style comments | Vorbis-style comments | Vorbis-style comments |
| OptimFROG / DualStream | `.ofr`, `.ofs` | ID3v1, APEv1/v2 | ID3v1, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| Speex | `.spx` | Vorbis-style comments | Vorbis-style comments | Vorbis-style comments |
| TAK | `.tak` | ID3v1, APEv1/v2 | ID3v1, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| True Audio | `.tta` | ID3v1, ID3v2, APEv1/v2 | ID3v1, ID3v2, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| WAV | `.wav`, `.wave` | RIFF INFO and ID3v2 | RIFF INFO and ID3v2 | RIFF INFO and ID3v2 |
| WavPack | `.wv` | ID3v1, APEv1/v2 | ID3v1, APEv2 | ID3v1, ID3v2, APEv1/v2 |
| Windows Media / ASF | `.wma`, `.asf` | ASF/Windows Media attributes | ASF/Windows Media attributes | Non-stream-specific editable attributes and cover metadata; stream-specific attributes can remain |
| WebM | `.webm` | Matroska-family tags, supported chapters, image attachments | Same supported structures | Same Matroska-family removal behavior |

ALAC is Apple Lossless audio carried in the MP4 family, not a second tag format requiring a standalone `.alac` scan extension. Likewise, a video track inside MP4, Matroska, or WebM does not turn this into a video-editing or transcoding application.

The table includes every extension admitted by the baseline's default folder scanner. Explicitly adding a file or changing the incoming extension restriction can attempt other suffixes, but does not add a parser for a different format. `.wmv`, for example, is not scanned by default; an explicitly added file with actual ASF content can use the ASF reader. File contents are checked, so a permitted extension alone is insufficient.

## Tag families and priorities

The **Mpeg** policy serves MPEG audio, raw AAC, and TTA. When several enabled tags are present, APE takes priority over ID3v2, then ID3v1. TagGroove displays one type rather than merging values from every block.

The **Ape, Mpc** policy serves Monkey's Audio, Musepack, WavPack, TAK, and OptimFROG. APE takes priority over ID3v1. An ID3v2 prefix in these files can remain physically present even though this reading policy does not display it; choose its removal explicitly if needed.

Saving can synchronize written types from the displayed values and remove a type selected for removal but not writing. Keep a copy before changing policies on files that intentionally contain conflicting tag blocks. ID3v1 is especially limited: it cannot preserve arbitrary long text, custom fields, multiple values, or embedded images. APEv1 reading does not mean new writes stay APEv1; the writer normally emits APEv2.

ID3v2.3 UTF-16 is the default MPEG write representation, with v2.4 UTF-8 and legacy v2.3 Latin-1 available. An existing ID3v2 tag in non-MPEG containers follows that format's preservation rules rather than being blindly converted by the MPEG checkbox. DSF/DFF preserve an existing revision; a new tag uses the configured revision. Test the actual player/device before choosing a non-Unicode representation.

## Artwork

| Metadata family | Artwork behavior |
|---|---|
| ID3v2 | Embedded picture frames; available in supported MPEG, AIFF, DSF/DFF, WAV, and other ID3 paths. |
| ID3v1 | No embedded artwork capacity. Enable a richer write type when adding covers. |
| APEv2 | Embedded APE cover items. |
| FLAC / Vorbis-style comments | Format-specific embedded picture representation. |
| MP4 | `covr` artwork. |
| ASF | `WM/Picture` attributes. |
| Matroska / WebM | Attachments with an image MIME type; non-image attachments such as fonts are separate. |
| RIFF INFO | No arbitrary embedded-cover field; a WAV can carry artwork in ID3v2. |

A picture displayed from the containing folder is not evidence of an embedded image. The tag-panel fallback can show a local image when the file has no cover. Import or embed it explicitly if it must travel inside the audio file.

## Chapters and cue sheets

Supported MP4 and Matroska chapters appear through `CHAPTERS` and chapter information. Enable **Options → Tags → Advanced → List chapters as separate files**, then reload the directory, to show supported subsongs as rows. At least two chapters are needed for row expansion in the inspected implementation. These rows reference one parent file; they are not split audio files.

Supported sources also include ID3v2 `CHAP` frames and embedded `CUESHEET` content. Loading a standalone cue sheet can list its tracks. `%_subsong%` is the zero-based index and `%_subsong_count%` the count. Metadata common to every chapter row can be stored at file level; differing values use supported per-chapter storage.

MP4 reads Nero and QuickTime chapter representations, preferring Nero when both exist. **Tools → Strip chapters MP4…** explicitly removes supported chapter references/structures. This is separate from removing ordinary MP4 tag atoms; unreferenced chapter text bytes can remain in media storage after stripping, so it is not a secure-erasure operation.

For supported MP4/Matroska chapter-list edits, a `CHAPTERS` value has a time and title, such as `00:00:00.000 Opening`. Other containers do not gain chapter support by adding arbitrary text called CHAPTERS.

## Technical information

TagGroove reports available format, codec, duration, bitrate, sample rate, channels/mode, file size, and tag information. Bit depth is meaningful where the stream provides it; a lossy codec does not acquire a valid PCM bit-depth measurement merely because a custom tag contains a number. Video/container-specific information is supplied where the reader can measure it.

Metadata fields such as LENGTH, FILETYPE, or ENCODERSETTINGS do not change the measured stream properties. Use [information placeholders](placeholders.md) when filtering or exporting actual measurements. Missing or unavailable properties should remain unknown rather than being inferred from an extension.

## Important format limits

### FLAC with prepended ID3v2

TagGroove reads the native Vorbis Comments and hides an unexpected prepended ID3v2 tag. A subsequent FLAC metadata write strips that prefix while preserving the FLAC stream through the supported write path. Hidden ID3 information is therefore not guaranteed to remain after an unrelated edit. Back up a nonstandard file before normalizing it.

### Raw streams and unusual tag layouts

Raw `.aac` detection accepts up to 64 KiB of padding or junk after leading ID3v2 tags, followed by three consecutive compatible ADTS frames. This search applies only to `.aac` and stops at an MPEG audio frame. Recognized streams use the applicable APE/ID3v1 options and retain the intervening bytes. Malformed headers or data outside that bound can still be refused.

TAK and OptimFROG tag paths do not decode/re-encode the audio. Existing tests use specification-built streams and independent probes; real original-encoder/device qualification is separate. OptimFROG DualStream uses the same recognized OFR header and codec label in this implementation.

Footer-only APEv2 tags are read without treating the preceding 32 audio bytes as a header; an internal `TAG` sequence is not an ID3v1 tag. APE tags left untouched by the selected write policy retain their bytes. Some other unusual APE footer/header layouts and compressed/encrypted ID3 frames have additional restrictions. Unsupported encrypted-frame revision conversion is refused to keep the original file. A successful read is not a promise that every malformed or encrypted structure can be rewritten.

Supported ID3v2.3 extended headers, with or without a CRC checksum, can be read
without losing ordinary fields or unedited compressed/encrypted records. A
same-revision metadata edit can normalize the tag to a plain ID3v2.3 header;
the optional extended header/checksum is then absent, while unedited opaque
records and audio remain preserved. Recorded Undo restores the original bytes,
including that header/checksum. This does not decrypt encrypted metadata.

Supported non-unsynchronised ID3v2.4 CRC extensions and footers can also
preserve unedited compressed/encrypted records in the qualified same-revision
edits. A successful save normalizes these optional envelopes to plain v2.4;
it does not retain their checksum/footer representation. Exact Undo restores
the original bytes.

Ordinary top-level ID3v2.4 frame unsynchronisation is also preserved for
unchanged private/unknown records in the tested MPEG, DSF, DSDIFF, WAV and AIFF
paths. This is byte escaping inside a frame, separate from the optional tag
headers above. The supported MPEG v2.4→v2.3 conversion retains the decoded
payload instead of marking already-decoded bytes as escaped again.

Ordinary experimental frame names such as `Y000` are retained as real opaque
metadata. They do not become encrypted merely because their names resemble an
internal placeholder. Genuine encrypted records keep their existing decryption and revision-conversion
restrictions.

Global unsynchronization combined with optional headers, update/restriction
declarations and arbitrary frame/layout combinations remain unqualified. A
refused operation should leave the original file intact. Keep a backup rather
than converting an encrypted tag's revision to get around a refusal.

### MP4 metadata preservation

MP4 iTunes fields take precedence over Nero metadata; Nero supplies missing values. Editing or deleting a mapped field also updates or removes its existing Nero copy. TagGroove does not add newly created fields to the Nero set. For a file without chapters, an edited value too long for its existing Nero space is written to iTunes only; the older Nero value remains hidden by iTunes precedence.

Nero artwork is displayed when iTunes artwork is absent. A cover edit writes the result to iTunes artwork and removes the Nero copy, even if it was hidden. Ordinary text-field edits preserve that artwork copy. Unrelated `udta/indx` data is retained but is not exposed for editing.

Nonstandard `plID` items survive unrelated edits. Values of 1–8 bytes can appear as `ITUNESALBUMID`; editing that field writes the standard 8-byte representation. These constructed-fixture checks do not certify every original Nero or Mp3tag version’s output. Keep copies of unusual files and inspect saved tags in the receiving application.

For supported saves, items whose values and storage mapping are unchanged retain
their original data records, including stored locale words and UTF-16 encoding.
Changed values, cover edits and mapping rewrites use normal serialization;
deleted values are not restored from the old records. Item lengths and container
headers can change to keep the result valid. This preserves the named metadata
records rather than the complete file's bytes or every possible MP4 layout.

### DSD

DSF metadata uses its terminal ID3v2 area. DSF and DSDIFF can preserve unedited compressed or opaque encrypted ID3v2 frames byte for byte, including the supported ID3v2.3 extended-header and ID3v2.4 CRC/footer cases above. That does not decrypt them; combined unsynchronization/header transformations, unsupported footer layouts and merged duplicate frames still limit preservation. DSDIFF can additionally supply DIIN artist/title when ID3 lacks them. Duplicate DSDIFF ID3 chunks are refused for field/cover writes instead of being silently merged; removal can still handle them. A DST index can prevent edits that would move indexed sound data. These are container restrictions, not a request to decode DSD audio to PCM.

### Matroska and WebM

Tag target levels matter: the same native name can represent a track, album, disc, or episode depending on its target. **Use recommended mapping for Matroska video** changes that interpretation when an actual video track exists. The [mapping reference](tag-mapping.md) details the levels.

Global tags, supported chapter tags, and image attachments can be edited without treating unrelated track/edition metadata or fonts as disposable. Remove tag preserves non-image attachments and unrelated scoped metadata while removing supported global/chapter metadata and images. The Segment Info Title is a different structure from a mapped TITLE tag and is not exposed as that field by this baseline.

Live/unknown-sized cluster layouts can be read through their metadata index but are refused for writing. A SeekHead without space for the updated metadata index can also make a write fail with the original preserved. Uninterpreted edition/segment chapter relationships are not a promise of full authoring support for every Matroska feature.

### ASF and protected files

ASF can contain attributes with binary types, language/stream scope, and values larger than 64 KiB. TagGroove preserves opaque structures it cannot expose as text; Remove tag intentionally does not mean erasing every ASF object. Older consuming readers may omit large Metadata Library values even when the file remains valid. Test long lyrics or large covers in the target player.

An extension such as `.m4p`, or encryption objects in ASF, does not establish DRM support. Editing a container's visible metadata cannot remove protection or guarantee protected playback. Damaged metadata, unsupported encrypted layouts, read-only sources, and access-provider restrictions can prevent a write even when the general family is listed.

## Platform and verification scope

The portable metadata core uses the same family parsers across supported builds. Access to a file is a separate requirement: mobile document providers can lack ordinary filesystem paths or permissions used by desktop folder, rename, and external-tool workflows. Consult the installed app's source-access message rather than assuming desktop filesystem operations apply to a provider URI.

This table is checked against the baseline source and relevant format tests, not a claim that every listed format has been exercised on every released platform or compared with the original Mp3tag executable. Tag editing does not establish playback support; TagGroove can delegate playback to another application.

## Related

- [Native mapping tables](tag-mapping.md)
- [Field names](field-names.md)
- [Information placeholders](placeholders.md)
- [Filter syntax](filter-syntax.md)

<!-- verified: current main4e4d5fc with merged26 format-preservation notes; original baseline007a806 crates/tag-core/src/tags.rs audio_extension, ordered_tags, write_tag_fields; tag_policy.rs; raw_streams.rs; container_formats.rs/dsd.rs; matroska.rs describe/write/mapping_rules; asf/mod.rs; mp4_chapters.rs/subsongs.rs; tests/{tag_policy,raw_streams,formats_edge,containers,asf,mp4_chapters,flac_id3}.rs. No current all-platform/runtime format certification. Historical verification notes were checked against source where later chapter/legacy-track behavior superseded them. -->
