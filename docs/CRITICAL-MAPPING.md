# Critical field mapping audit

The registry records all 118 named fields from the [published Mp3tag mapping](https://docs.mp3tag.de/mapping/) and [compact table](https://docs.mp3tag.de/mapping-table/), inspected on 2026-10-14. It contains 85 ID3v2.3, 90 ID3v2.4, 78 MP4, 9 RIFF, 40 ASF, and 76 Matroska nonempty mapping cells. Identifiers below are functional format facts, not copied application code. Blank cells mean the documentation specifies no fixed mapping for that format; they do not imply that a custom field is forbidden.

The portable backend implements concrete ID3v2.3/v2.4, MP4, Vorbis Comment, APEv2, and RIFF metadata paths. ASF/WMA and Matroska are implemented by native pure-Rust readers and writers ([formats-asf](verification/formats-asf.md), [formats-dsd-mkv](verification/formats-dsd-mkv.md)). Full field-mapping parity stays **incomplete** until original-executable differential results exist.

## Native names and optional user mappings

Vorbis Comment and APEv2 names are exposed as stored. DATE and YEAR, TRACKNUMBER and TRACK, and DISCTOTAL and TOTALDISCS remain independent. The Vorbis vendor string is not an ENCODER field. The [user mapping documentation](https://docs.mp3tag.de/customization/options/tags/mapping/) makes DATE-to-YEAR an opt-in example, not a default alias. Explicit rules contain `tag_type`, `source`, and `target`; reading renames source fields, and writing a target updates every mapped source. One source cannot map to multiple targets. Supported rule identifiers are ID3v2, MP4, VorbisComment, APEv2, and WMA (the latter has no container I/O).

Core APIs are `read_entry_with_mappings`, `read_entry_summary_with_mappings`, `write_fields_with_mappings`, and `write_fields_summary_with_mappings`. The unmapped APIs remain available. `map_read_fields` and `map_write_fields` expose the same transformation for callers. Application preference persistence and mapping configuration UI are separate integration requirements.

## Registry status

Every populated ID3 cell except bare UNIQUEFILEID has a generated real-WAV native-frame write/read sweep. Every populated MP4 cell has a generated real-M4A native-atom sweep. All nine RIFF cells have a native INFO-chunk sweep. These tests verify that our codec writes the documented native identifier and survives independent concrete parsing; they are not an original-executable differential test. ASF and Matroska cells are covered by the registry and by the native ASF and Matroska writers ([formats-asf](verification/formats-asf.md), [formats-dsd-mkv](verification/formats-dsd-mkv.md)).

`Codec` below means the populated supported-container cells pass the sweeps. `UFID pending` means the bare UNIQUEFILEID value syntax is not established and writes explicitly fail; existing opaque UFID frames remain preserved. MusicBrainz recording identifiers with their documented UFID owner are implemented separately.

| Field | ID3v2.3 | ID3v2.4 | MP4 | RIFF | ASF | Matroska | Supported codec status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ACOUSTID_ID | TXXX:Acoustid Id | TXXX:Acoustid Id | ----:com.apple.iTunes:Acoustid Id | — | — | — | Codec |
| ACOUSTID_FINGERPRINT | TXXX:Acoustid Fingerprint | TXXX:Acoustid Fingerprint | ----:com.apple.iTunes:Acoustid Fingerprint | — | — | — | Codec |
| ALBUM | TALB | TALB | ©alb | IPRD | WM/AlbumTitle | MKA:T=50 TITLE MKV: T=60 TITLE | Codec |
| ALBUMSORT | TSOA | TSOA | soal | — | WM/AlbumSortOrder | MKA: T=50 SORT_WITH MKV: T=60 SORT_WITH | Codec |
| ALBUMARTIST | TPE2 | TPE2 | aART | — | WM/AlbumArtist | MKA: T=50 ARTIST MKV: T=60 ARTIST | Codec |
| ALBUMARTISTSORT | TSO2 | TSO2 | soaa | — | — | T=30 | Codec |
| ARTIST | TPE1 | TPE1 | ©art | IART | Author | MKA: T=30 ARTIST MKV: T=50 ARTIST | Codec |
| ARTISTSORT | TSOP | TSOP | soar | — | WM/ArtistSortOrder | T=30 | Codec |
| BARCODE | TXXX:BARCODE | TXXX:BARCODE | ----:com.apple.iTunes:BARCODE | — | WM/Barcode | T=30 | Codec |
| BPM | TBPM | TBPM | tmpo | — | WM/BeatsPerMinute | T=30 | Codec |
| CATALOGNUMBER | TXXX:CATALOGNUMBER | TXXX:CATALOGNUMBER | ----:com.apple.iTunes:CATALOGNUMBER | — | WM/CatalogNo | T=30 | Codec |
| COMMENT | COMM | COMM | ©cmt | ICMT | Description | T=30 | Codec |
| COMPILATION | TCMP | TCMP | cpil | — | — | T=30 | Codec |
| COMPOSER | TCOM | TCOM | ©wrt | — | WM/Composer | T=30 | Codec |
| COMPOSERSORT | TSOC | TSOC | soco | — | — | T=30 | Codec |
| CONDUCTOR | TPE3 | TPE3 | ©con | — | WM/Conductor | T=30 | Codec |
| CONTENTGROUP | TIT1 | TIT1 | ©grp | — | WM/ContentGroupDescription | T=30 | Codec |
| COPYRIGHT | TCOP | TCOP | cprt | ICOP | Copyright | T=30 | Codec |
| DATE | TDAT | TXXX:DATE | — | — | — | — | Codec |
| DESCRIPTION | — | — | desc | — | — | T=30 | Codec |
| DISCNUMBER | TPOS | TPOS | disk | — | WM/PartOfSet | T=50 PART_NUMBER | Codec |
| ENCODEDBY | TENC | TENC | ©enc | — | WM/EncodedBy | MKA: T=30 ENCODED_BY MKV: T=50 ENCODED_BY | Codec |
| ENCODERSETTINGS | TSSE | TSSE | — | — | WM/EncodingSettings | MKA: T=30 MKV: T=50 | Codec |
| ENCODINGTIME | — | TDEN | — | — | WM/EncodingTime | T=30 | Codec |
| FILEOWNER | TOWN | TOWN | — | — | — | MKA: T=30 PURCHASE_OWNER MKV: T=50 PURCHASE_OWNER | Codec |
| FILETYPE | TFLT | TFLT | — | — | — | T=30 | Codec |
| GENRE | TCON | TCON | ©gen \| gnre | IGNR | WM/Genre | T=30 | Codec |
| GROUPING | GRP1 | GRP1 | — | — | — | T=30 | Codec |
| INITIALKEY | TKEY | TKEY | — | — | WM/InitialKey | T=30 INITIAL_KEY | Codec |
| INVOLVEDPEOPLE | IPLS | TIPL | — | — | — | T=30 | Codec |
| ISRC | TSRC | TSRC | — | — | WM/ISRC | T=30 | Codec |
| LANGUAGE | TLAN | TLAN | — | ILNG | WM/Language | T=30 | Codec |
| LENGTH | TLEN | TLEN | — | — | — | T=30 | Codec |
| LYRICIST | TEXT | TEXT | — | — | — | T=30 | Codec |
| MEDIATYPE | TMED | TMED | — | — | — | T=50 ORIGINAL_MEDIA_TYPE | Codec |
| MIXARTIST | TPE4 | TPE4 | — | — | — | T=30 REMIXED_BY | Codec |
| MOOD | — | TMOO | — | — | WM/Mood | T=30 | Codec |
| MOVEMENTNAME | MVNM | MVNM | ©mvn | — | — | T=20 TITLE | Codec |
| MOVEMENT | MVIN | MVIN | ©mvi | — | — | T=20 PART_NUMBER | Codec |
| MOVEMENTTOTAL | MVIN | MVIN | ©mvc | — | — | T=30 | Codec |
| MUSICBRAINZ_ALBUMARTISTID | TXXX:MusicBrainz Album Artist Id | TXXX:MusicBrainz Album Artist Id | ----:com.apple.iTunes:MusicBrainz Album Artist Id | — | — | — | Codec |
| MUSICBRAINZ_ALBUMID | TXXX:MusicBrainz Album Id | TXXX:MusicBrainz Album Id | ----:com.apple.iTunes:MusicBrainz Album Id | — | — | — | Codec |
| MUSICBRAINZ_ALBUMRELEASECOUNTRY | TXXX:MusicBrainz Album Release Country | TXXX:MusicBrainz Album Release Country | ----:com.apple.iTunes:MusicBrainz Album Release Country | — | — | — | Codec |
| MUSICBRAINZ_ALBUMSTATUS | TXXX:MusicBrainz Album Status | TXXX:MusicBrainz Album Status | ----:com.apple.iTunes:MusicBrainz Album Status | — | — | — | Codec |
| MUSICBRAINZ_ALBUMTYPE | TXXX:MusicBrainz Album Type | TXXX:MusicBrainz Album Type | ----:com.apple.iTunes:MusicBrainz Album Type | — | — | — | Codec |
| MUSICBRAINZ_ARTISTID | TXXX:MusicBrainz Artist Id | TXXX:MusicBrainz Artist Id | ----:com.apple.iTunes:MusicBrainz Artist Id | — | — | — | Codec |
| MUSICBRAINZ_DISCID | TXXX:MusicBrainz Disc Id | TXXX:MusicBrainz Disc Id | ----:com.apple.iTunes:MusicBrainz Disc Id | — | — | — | Codec |
| MUSICBRAINZ_ORIGINALALBUMID | TXXX:MusicBrainz Original Album Id | TXXX:MusicBrainz Original Album Id | ----:com.apple.iTunes:MusicBrainz Original Album Id | — | — | — | Codec |
| MUSICBRAINZ_ORIGINALARTISTID | TXXX:MusicBrainz Original Artist Id | TXXX:MusicBrainz Original Artist Id | ----:com.apple.iTunes:MusicBrainz Original Artist Id | — | — | — | Codec |
| MUSICBRAINZ_RELEASEGROUPID | TXXX:MusicBrainz Release Group Id | TXXX:MusicBrainz Release Group Id | ----:com.apple.iTunes:MusicBrainz Release Group Id | — | — | — | Codec |
| MUSICBRAINZ_RELEASETRACKID | TXXX:MusicBrainz Release Track Id | TXXX:MusicBrainz Release Track Id | ----:com.apple.iTunes:MusicBrainz Release Track Id | — | — | — | Codec |
| MUSICBRAINZ_TRACKID | UFID:http://musicbrainz.org | UFID:http://musicbrainz.org | ----:com.apple.iTunes:MusicBrainz Track Id | — | — | — | Codec |
| MUSICBRAINZ_TRMID | TXXX:MusicBrainz TRM Id | TXXX:MusicBrainz TRM Id | ----:com.apple.iTunes:MusicBrainz TRM Id | — | — | — | Codec |
| MUSICBRAINZ_WORKID | TXXX:MusicBrainz Work Id | TXXX:MusicBrainz Work Id | ----:com.apple.iTunes:MusicBrainz Work Id | — | — | — | Codec |
| MUSICIANCREDITS | — | TMCL | — | — | — | T=30 | Codec |
| NARRATOR | — | — | ©nrt | — | — | T=30 | Codec |
| NETRADIOOWNER | TRSO | TRSO | — | — | — | T=30 | Codec |
| NETRADIOSTATION | TRSN | TRSN | — | — | — | T=30 | Codec |
| ORIGALBUM | TOAL | TOAL | — | — | WM/OriginalAlbumTitle | T=30 | Codec |
| ORIGARTIST | TOPE | TOPE | — | — | WM/OriginalArtist | T=30 | Codec |
| ORIGFILENAME | TOFN | TOFN | — | — | WM/OriginalFilename | T=30 | Codec |
| ORIGLYRICIST | TOLY | TOLY | — | — | WM/OriginalLyricist | T=30 | Codec |
| ORIGYEAR | TORY | TDOR | — | — | WM/OriginalReleaseYear | T=30 | Codec |
| PODCAST | PCST | PCST | pcst | — | — | T=30 | Codec |
| PODCASTCATEGORY | TCAT | TCAT | catg | — | — | T=30 | Codec |
| PODCASTDESC | TDES | TDES | ldes | — | — | T=30 | Codec |
| PODCASTID | TGID | TGID | egid | — | — | T=30 | Codec |
| PODCASTKEYWORDS | TKWD | TKWD | keyw | — | — | T=30 | Codec |
| PODCASTURL | WFED | WFED | purl | — | — | T=30 | Codec |
| POPULARIMETER | POPM | POPM | — | — | — | T=30 | Codec |
| PUBLISHER | TPUB | TPUB | ©pub | — | WM/Publisher | T=30 | Codec |
| RATING MM | POPM | POPM | — | — | — | T=30 | Codec |
| RATING WMP | POPM | POPM | — | — | WM/SharedUserRating | T=30 | Codec |
| RELEASETIME | TDRL | TDRL | — | — | — | T=50 DATE_RELEASED | Codec |
| SETSUBTITLE | — | TSST | — | — | — | MKA: T=50 SUBTITLE MKV: T=60 SUBTITLE | Codec |
| SUBTITLE | TIT3 | TIT3 | — | — | WM/SubTitle | MKA: T=30 MKV: T=50 | Codec |
| TAGGINGTIME | — | TDTG | — | — | — | T=30 DATE_TAGGED | Codec |
| TITLE | TIT2 | TIT2 | ©nam | INAM | Title | MKA: T=30 MKV: T=50 | Codec |
| TITLESORT | TSOT | TSOT | sonm | — | WM/TitleSortOrder | MKA: T=30 SORT_WITH MKV: T=50 SORT_WITH | Codec |
| TRACK | TRCK | TRCK | trkn | ITRK | WM/TrackNumber | T=30 PART_NUMBER | Codec |
| UNIQUEFILEID | UFID | UFID | — | — | — | — | UFID pending |
| UNSYNCEDLYRICS | USLT | USLT | ©lyr | — | WM/Lyrics | T=30 LYRICS | Codec |
| WWW | WXXX | WXXX | — | — | — | T=30 | Codec |
| WWWARTIST | WOAR | WOAR | — | — | WM/AuthorURL | T=30 | Codec |
| WWWAUDIOFILE | WOAF | WOAF | — | — | WM/AudioFileURL | T=30 | Codec |
| WWWAUDIOSOURCE | WOAS | WOAS | — | — | WM/AudioSourceURL | T=30 | Codec |
| WWWCOMMERCIALINFO | WCOM | WCOM | — | — | WM/PromotionURL | MKA: T=30 PURCHASE_INFO MKV: T=50 PURCHASE_INFO | Codec |
| WWWCOPYRIGHT | WCOP | WCOP | — | — | CopyrightURL | T=30 | Codec |
| WWWPAYMENT | WPAY | WPAY | — | — | — | MKA: T=30 PURCHASE_ITEM MKV: T=50 PURCHASE_ITEM | Codec |
| WWWPUBLISHER | WPUB | WPUB | — | — | — | T=30 | Codec |
| WWWRADIOPAGE | WORS | WORS | — | — | — | T=30 | Codec |
| YEAR | TYER | TDRC | ©day | ICRD | WM/Year | T=50 DATE_RECORDED | Codec |
| DIRECTOR | — | — | ©dir | — | — | — | Codec |
| ITUNESACCOUNT | — | — | apID | — | — | — | Codec |
| ITUNESADVISORY | — | — | rtng | — | — | — | Codec |
| ITUNESALBUMID | — | — | plID | — | — | — | Codec |
| ITUNESARTISTID | — | — | atID | — | — | — | Codec |
| ITUNESCATALOGID | — | — | cnID | — | — | — | Codec |
| ITUNESCOMPOSERID | — | — | cmID | — | — | — | Codec |
| ITUNESCOUNTRYID | — | — | sfID | — | — | — | Codec |
| ITUNESGAPLESS | — | — | pgap | — | — | — | Codec |
| ITUNESGENREID | — | — | geID | — | — | — | Codec |
| ITUNESHDVIDEO | — | — | hdvd | — | — | — | Codec |
| ITUNESMEDIATYPE | — | — | stik | — | — | — | Codec |
| ITUNESOWNER | — | — | ownr | — | — | — | Codec |
| ITUNESPURCHASEDATE | — | — | purd | — | — | — | Codec |
| RATE | — | — | rate | — | — | — | Codec |
| SHOWMOVEMENT | — | — | shwm | — | — | — | Codec |
| STOREDESCRIPTION | — | — | sdes | — | — | — | Codec |
| TOTALDISCS | — | — | disk | — | — | — | Codec |
| TOTALTRACKS | — | — | trkn | — | — | — | Codec |
| TVEPISODE | — | — | tves | — | — | T=50 PART_NUMBER | Codec |
| TVEPISODEID | — | — | tven | — | — | — | Codec |
| TVNETWORK | — | — | tvnn | — | — | — | Codec |
| TVSEASON | — | — | tvsn | — | — | T=60 PART_NUMBER | Codec |
| TVSHOW | — | — | tvsh | — | — | — | Codec |
| TVSHOWSORT | — | — | sosn | — | — | — | Codec |
| WORK | — | — | ©wrk | — | — | — | Codec |

## Other fields and concrete value rules

The documented other-field fallback is `TXXX:<fieldname>` for ID3 and `----:com.apple.iTunes:<fieldname>` for MP4. Custom TXXX descriptions and freeform `mean`/`name` are preserved, including their spelling. Explicit MP4 namespaces can be addressed as `----:<mean>:<name>`. The ASF fallback is the literal fieldname and Matroska is a level-30 fieldname; those containers remain unsupported. RIFF has no documented arbitrary-name fallback; existing four-character INFO identifiers are preserved and can be edited directly.

- CONTENTGROUP maps to TIT1 and MP4 ©grp; GROUPING maps to ID3 GRP1. They are independent fields. ENCODERSETTINGS maps to TSSE. ENCODER has no listed fixed mapping and therefore uses the custom-field fallback. Existing unlisted MP4 atoms remain preserved; their display and editing behavior has not been exhaustively compared with Mp3tag.
- ID3 YEAR and DATE retain TYER and TDAT independently in v2.3. In v2.4 YEAR uses TDRC and DATE uses TXXX:DATE. Native v2.3 tags stay v2.3. A small independently authored serializer retains documented v2.3 extensions that Lofty's downgrade writer discards, maps TIPL to IPLS, writes Latin-1 URL payloads correctly, and changes only the metadata region in staging.
- ID3 TRACK and DISCNUMBER store compound strings in TRCK and TPOS. No synthetic TOTALTRACKS/TOTALDISCS fields are invented for ID3. Such explicitly supplied names use TXXX fallback. MP4 trkn/disk retain number/total components and expose the documented total fields; integer storage normalizes zero padding. The [author's explanation](https://community.mp3tag.de/t/itunes-metadata/9596) confirms compound numbering semantics.
- MP4 media types support Normal, Audiobook, Bookmark, Music Video, Movie, TV Show, Booklet, Ringtone, Podcast, and iTunes U. Native codes are independently checked against the [ExifTool QuickTime registry](https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/QuickTime.pm). Unknown codes are returned numerically; their exact Mp3tag display remains unverified. Apple historically stores ARTIST as ©ART, whereas the Mp3tag table spells ©art; reads recognize both spellings and writes use ©ART.
- MP4 boolean fields COMPILATION, PODCAST, ITUNESGAPLESS, and SHOWMOVEMENT accept 1 or deletion; ITUNESHDVIDEO accepts 0/1 and ITUNESADVISORY accepts 0/1/2. Numeric native atoms validate unsigned bounds. ITUNESALBUMID uses the native unsigned 64-bit type; historical eight-byte values labeled with a 32-bit data type can be rejected by Lofty's strict parser and remain a compatibility gap. PODCAST's ID3 PCST frame is binary integer data, as independently described by [Mutagen's frame definitions](https://raw.githubusercontent.com/quodlibet/mutagen/master/mutagen/id3/_frames.py).
- Native numeric ID3 genres display their standard names without rewriting the untouched TCON frame. INVOLVEDPEOPLE and MUSICIANCREDITS use role/person pairs. MOVEMENT/MOVEMENTTOTAL share MVIN. MusicBrainz track IDs use UFID owner `http://musicbrainz.org`. Other UFID payloads remain opaque.
- POPULARIMETER accepts email|rating|counter. Default owner abstractions expose RATING MM and RATING WMP. MM half-star raw values are 0,13,1,54,64,118,128,186,196,242,255; WMP star values are 0,1,64,128,196,255. The [author confirmed the changed MM endpoints](https://community.mp3tag.de/t/f-rating-mm-sometimes-not-work/16276). Existing counters remain unchanged for owner-specific rating edits. The preference to always display POPULARIMETER is implemented (Options → Advanced).
- UNSYNCEDLYRICS currently implements the language||text form with an empty description. Existing nonempty USLT descriptions remain on disk when unedited but are not fully represented in the current string API. Their exact exposed value syntax remains a parity gap.

## Preservation and failure boundaries

Edits are applied to a sibling temporary copy, reparsed and verified, synced, and atomically replaced. Source permissions are preserved. Malformed input, invalid requested values, failed representability, and failed verification leave the source intact. Native unknown ID3 frames, APE binary items, custom Vorbis fields, and MP4 atoms remain untouched unless addressed. Summary reads and mutation results retain cover descriptors with empty encoded data; full reads retain image payloads. Existing MP3/FLAC/M4A/Ogg/Opus/AIFF/WavPack/AAC packet-invariance tests cover representative metadata/cover changes, not every native option combination.

The v2.3 adapter rejects compressed/encrypted/data-length-indicated frame rewriting and combined tag CRC plus unsynchronisation instead of risking metadata loss. Uncommon frame flags, ID3v2.2 upgrades, legacy ANSI/RIFF encoding options, chapter frames, and rare historical native payloads need a broader differential corpus. Ordinary v2.3 strings, Unicode credits, URLs, documented extension frames, and unknown binary frames have repeated-edit regression coverage. Lofty 0.25.4 has an EOF-shrink splice bug; the adapter handles that boundary explicitly with the correct final absolute file length.

WMA/ASF and Matroska I/O, the POPULARIMETER display preference and application user-mapping rules are implemented since this note was written ([formats-asf](verification/formats-asf.md), [remove-mapping](verification/remove-mapping.md), [formats-edge](verification/formats-edge.md)). Original-executable differential qualification still prevents a full-parity claim.

## Repeatable checks

`cargo test -p tag-core --test mappings --test metadata` runs 13 mapping tests and 14 metadata tests. Encoded fixture tests use ffmpeg when available and report an explicit skip when absent. Fixtures are generated under temporary directories; the user's Beatport library is read-only throughout this work.
