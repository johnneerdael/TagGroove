# Tag mapping tables

A mapping connects the editable name `ARTIST` to native storage such as the ID3v2 frame `TPE1` or MP4 atom `©ART`. Use the field name in OpenTag; inspect the native identifier when troubleshooting another reader or an existing file.

!!! tip "New to this?"
    Read [Field names](field-names.md) for the meanings and special value syntax. A mapping changes where a value is stored; it does not calculate or translate that value.

## How to read these tables

The tables contain all 118 named fields from OpenTag's source-checked mapping vocabulary. **—** means no dedicated mapping in that cell. It does not always mean the field is forbidden: the [custom-field fallback](#custom-field-fallback) explains what is used instead. APE and Vorbis Comments store actual field names rather than using this fixed native-identifier table.

`TXXX:description` denotes an ID3 user-text frame with that description. `UFID:owner` denotes an owner-specific ID3 identifier. `----:namespace:name` denotes an MP4 freeform atom. The `©` character is part of a four-character MP4 identifier, not decorative punctuation.

These are supported metadata representations, not guarantees that another player uses them or that changing the filename extension converts one format into another. See [File formats](file-formats.md) for which tag family a file uses.

## ID3v2 and MP4

The ID3 columns describe the selected revision's dedicated mapping. A write in another revision can use custom text where no dedicated frame is defined. MP4 `ARTIST` writes `©ART` and reads the alternative `©art` spelling too. GENRE can read `gnre` as well as the text atom `©gen`; ordinary writes use the text representation. Existing alternative/freeform storage can also be normalized on save unless the Advanced reuse options preserve an unchanged field.

<!-- native-mapping-id3-mp4:start -->
| Field | ID3v2.3 | ID3v2.4 | MP4 |
|---|---|---|---|
| `ACOUSTID_ID` | `TXXX:Acoustid Id` | `TXXX:Acoustid Id` | `----:com.apple.iTunes:Acoustid Id` |
| `ACOUSTID_FINGERPRINT` | `TXXX:Acoustid Fingerprint` | `TXXX:Acoustid Fingerprint` | `----:com.apple.iTunes:Acoustid Fingerprint` |
| `ALBUM` | `TALB` | `TALB` | `©alb` |
| `ALBUMSORT` | `TSOA` | `TSOA` | `soal` |
| `ALBUMARTIST` | `TPE2` | `TPE2` | `aART` |
| `ALBUMARTISTSORT` | `TSO2` | `TSO2` | `soaa` |
| `ARTIST` | `TPE1` | `TPE1` | `©ART` |
| `ARTISTSORT` | `TSOP` | `TSOP` | `soar` |
| `BARCODE` | `TXXX:BARCODE` | `TXXX:BARCODE` | `----:com.apple.iTunes:BARCODE` |
| `BPM` | `TBPM` | `TBPM` | `tmpo` |
| `CATALOGNUMBER` | `TXXX:CATALOGNUMBER` | `TXXX:CATALOGNUMBER` | `----:com.apple.iTunes:CATALOGNUMBER` |
| `COMMENT` | `COMM` | `COMM` | `©cmt` |
| `COMPILATION` | `TCMP` | `TCMP` | `cpil` |
| `COMPOSER` | `TCOM` | `TCOM` | `©wrt` |
| `COMPOSERSORT` | `TSOC` | `TSOC` | `soco` |
| `CONDUCTOR` | `TPE3` | `TPE3` | `©con` |
| `CONTENTGROUP` | `TIT1` | `TIT1` | `©grp` |
| `COPYRIGHT` | `TCOP` | `TCOP` | `cprt` |
| `DATE` | `TDAT` | `TXXX:DATE` | — |
| `DESCRIPTION` | — | — | `desc` |
| `DISCNUMBER` | `TPOS` | `TPOS` | `disk` |
| `ENCODEDBY` | `TENC` | `TENC` | `©enc` |
| `ENCODERSETTINGS` | `TSSE` | `TSSE` | — |
| `ENCODINGTIME` | — | `TDEN` | — |
| `FILEOWNER` | `TOWN` | `TOWN` | — |
| `FILETYPE` | `TFLT` | `TFLT` | — |
| `GENRE` | `TCON` | `TCON` | `©gen; gnre` |
| `GROUPING` | `GRP1` | `GRP1` | — |
| `INITIALKEY` | `TKEY` | `TKEY` | — |
| `INVOLVEDPEOPLE` | `IPLS` | `TIPL` | — |
| `ISRC` | `TSRC` | `TSRC` | — |
| `LANGUAGE` | `TLAN` | `TLAN` | — |
| `LENGTH` | `TLEN` | `TLEN` | — |
| `LYRICIST` | `TEXT` | `TEXT` | — |
| `MEDIATYPE` | `TMED` | `TMED` | — |
| `MIXARTIST` | `TPE4` | `TPE4` | — |
| `MOOD` | — | `TMOO` | — |
| `MOVEMENTNAME` | `MVNM` | `MVNM` | `©mvn` |
| `MOVEMENT` | `MVIN` | `MVIN` | `©mvi` |
| `MOVEMENTTOTAL` | `MVIN` | `MVIN` | `©mvc` |
| `MUSICBRAINZ_ALBUMARTISTID` | `TXXX:MusicBrainz Album Artist Id` | `TXXX:MusicBrainz Album Artist Id` | `----:com.apple.iTunes:MusicBrainz Album Artist Id` |
| `MUSICBRAINZ_ALBUMID` | `TXXX:MusicBrainz Album Id` | `TXXX:MusicBrainz Album Id` | `----:com.apple.iTunes:MusicBrainz Album Id` |
| `MUSICBRAINZ_ALBUMRELEASECOUNTRY` | `TXXX:MusicBrainz Album Release Country` | `TXXX:MusicBrainz Album Release Country` | `----:com.apple.iTunes:MusicBrainz Album Release Country` |
| `MUSICBRAINZ_ALBUMSTATUS` | `TXXX:MusicBrainz Album Status` | `TXXX:MusicBrainz Album Status` | `----:com.apple.iTunes:MusicBrainz Album Status` |
| `MUSICBRAINZ_ALBUMTYPE` | `TXXX:MusicBrainz Album Type` | `TXXX:MusicBrainz Album Type` | `----:com.apple.iTunes:MusicBrainz Album Type` |
| `MUSICBRAINZ_ARTISTID` | `TXXX:MusicBrainz Artist Id` | `TXXX:MusicBrainz Artist Id` | `----:com.apple.iTunes:MusicBrainz Artist Id` |
| `MUSICBRAINZ_DISCID` | `TXXX:MusicBrainz Disc Id` | `TXXX:MusicBrainz Disc Id` | `----:com.apple.iTunes:MusicBrainz Disc Id` |
| `MUSICBRAINZ_ORIGINALALBUMID` | `TXXX:MusicBrainz Original Album Id` | `TXXX:MusicBrainz Original Album Id` | `----:com.apple.iTunes:MusicBrainz Original Album Id` |
| `MUSICBRAINZ_ORIGINALARTISTID` | `TXXX:MusicBrainz Original Artist Id` | `TXXX:MusicBrainz Original Artist Id` | `----:com.apple.iTunes:MusicBrainz Original Artist Id` |
| `MUSICBRAINZ_RELEASEGROUPID` | `TXXX:MusicBrainz Release Group Id` | `TXXX:MusicBrainz Release Group Id` | `----:com.apple.iTunes:MusicBrainz Release Group Id` |
| `MUSICBRAINZ_RELEASETRACKID` | `TXXX:MusicBrainz Release Track Id` | `TXXX:MusicBrainz Release Track Id` | `----:com.apple.iTunes:MusicBrainz Release Track Id` |
| `MUSICBRAINZ_TRACKID` | `UFID:http://musicbrainz.org` | `UFID:http://musicbrainz.org` | `----:com.apple.iTunes:MusicBrainz Track Id` |
| `MUSICBRAINZ_TRMID` | `TXXX:MusicBrainz TRM Id` | `TXXX:MusicBrainz TRM Id` | `----:com.apple.iTunes:MusicBrainz TRM Id` |
| `MUSICBRAINZ_WORKID` | `TXXX:MusicBrainz Work Id` | `TXXX:MusicBrainz Work Id` | `----:com.apple.iTunes:MusicBrainz Work Id` |
| `MUSICIANCREDITS` | — | `TMCL` | — |
| `NARRATOR` | — | — | `©nrt` |
| `NETRADIOOWNER` | `TRSO` | `TRSO` | — |
| `NETRADIOSTATION` | `TRSN` | `TRSN` | — |
| `ORIGALBUM` | `TOAL` | `TOAL` | — |
| `ORIGARTIST` | `TOPE` | `TOPE` | — |
| `ORIGFILENAME` | `TOFN` | `TOFN` | — |
| `ORIGLYRICIST` | `TOLY` | `TOLY` | — |
| `ORIGYEAR` | `TORY` | `TDOR` | — |
| `PODCAST` | `PCST` | `PCST` | `pcst` |
| `PODCASTCATEGORY` | `TCAT` | `TCAT` | `catg` |
| `PODCASTDESC` | `TDES` | `TDES` | `ldes` |
| `PODCASTID` | `TGID` | `TGID` | `egid` |
| `PODCASTKEYWORDS` | `TKWD` | `TKWD` | `keyw` |
| `PODCASTURL` | `WFED` | `WFED` | `purl` |
| `POPULARIMETER` | `POPM` | `POPM` | — |
| `PUBLISHER` | `TPUB` | `TPUB` | `©pub` |
| `RATING MM` | `POPM` | `POPM` | — |
| `RATING WMP` | `POPM` | `POPM` | — |
| `RELEASETIME` | `TDRL` | `TDRL` | — |
| `SETSUBTITLE` | — | `TSST` | — |
| `SUBTITLE` | `TIT3` | `TIT3` | — |
| `TAGGINGTIME` | — | `TDTG` | — |
| `TITLE` | `TIT2` | `TIT2` | `©nam` |
| `TITLESORT` | `TSOT` | `TSOT` | `sonm` |
| `TRACK` | `TRCK` | `TRCK` | `trkn` |
| `UNIQUEFILEID` | `UFID` | `UFID` | — |
| `UNSYNCEDLYRICS` | `USLT` | `USLT` | `©lyr` |
| `WWW` | `WXXX` | `WXXX` | — |
| `WWWARTIST` | `WOAR` | `WOAR` | — |
| `WWWAUDIOFILE` | `WOAF` | `WOAF` | — |
| `WWWAUDIOSOURCE` | `WOAS` | `WOAS` | — |
| `WWWCOMMERCIALINFO` | `WCOM` | `WCOM` | — |
| `WWWCOPYRIGHT` | `WCOP` | `WCOP` | — |
| `WWWPAYMENT` | `WPAY` | `WPAY` | — |
| `WWWPUBLISHER` | `WPUB` | `WPUB` | — |
| `WWWRADIOPAGE` | `WORS` | `WORS` | — |
| `YEAR` | `TYER` | `TDRC` | `©day` |
| `DIRECTOR` | — | — | `©dir` |
| `ITUNESACCOUNT` | — | — | `apID` |
| `ITUNESADVISORY` | — | — | `rtng` |
| `ITUNESALBUMID` | — | — | `plID` |
| `ITUNESARTISTID` | — | — | `atID` |
| `ITUNESCATALOGID` | — | — | `cnID` |
| `ITUNESCOMPOSERID` | — | — | `cmID` |
| `ITUNESCOUNTRYID` | — | — | `sfID` |
| `ITUNESGAPLESS` | — | — | `pgap` |
| `ITUNESGENREID` | — | — | `geID` |
| `ITUNESHDVIDEO` | — | — | `hdvd` |
| `ITUNESMEDIATYPE` | — | — | `stik` |
| `ITUNESOWNER` | — | — | `ownr` |
| `ITUNESPURCHASEDATE` | — | — | `purd` |
| `RATE` | — | — | `rate` |
| `SHOWMOVEMENT` | — | — | `shwm` |
| `STOREDESCRIPTION` | — | — | `sdes` |
| `TOTALDISCS` | — | — | `disk` |
| `TOTALTRACKS` | — | — | `trkn` |
| `TVEPISODE` | — | — | `tves` |
| `TVEPISODEID` | — | — | `tven` |
| `TVNETWORK` | — | — | `tvnn` |
| `TVSEASON` | — | — | `tvsn` |
| `TVSHOW` | — | — | `tvsh` |
| `TVSHOWSORT` | — | — | `sosn` |
| `WORK` | — | — | `©wrk` |
<!-- native-mapping-id3-mp4:end -->

## Matroska, ASF, and RIFF INFO

`T=30`, `T=50`, and similar Matroska entries are target type values. When no native name follows a target, use the field's own name. MKA entries describe the audio mapping; MKV entries describe the recommended video mapping when a video track is present and that Advanced option is enabled. A `.mkv` suffix alone does not select it.

`SORT_WITH` is nested under that level's TITLE. For example, album sorting is nested under the album-level TITLE rather than stored as an unrelated global text tag. `PART_NUMBER` at target 30 represents a track; target 50 represents a disc in audio mapping, or an episode in the recommended video mapping.

<!-- native-mapping-containers:start -->
| Field | Matroska | ASF / Windows Media | RIFF INFO |
|---|---|---|---|
| `ACOUSTID_ID` | — | — | — |
| `ACOUSTID_FINGERPRINT` | — | — | — |
| `ALBUM` | `MKA:T=50 TITLE MKV: T=60 TITLE` | `WM/AlbumTitle` | `IPRD` |
| `ALBUMSORT` | `MKA: T=50 SORT_WITH MKV: T=60 SORT_WITH` | `WM/AlbumSortOrder` | — |
| `ALBUMARTIST` | `MKA: T=50 ARTIST MKV: T=60 ARTIST` | `WM/AlbumArtist` | — |
| `ALBUMARTISTSORT` | `T=30` | — | — |
| `ARTIST` | `MKA: T=30 ARTIST MKV: T=50 ARTIST` | `Author` | `IART` |
| `ARTISTSORT` | `T=30` | `WM/ArtistSortOrder` | — |
| `BARCODE` | `T=30` | `WM/Barcode` | — |
| `BPM` | `T=30` | `WM/BeatsPerMinute` | — |
| `CATALOGNUMBER` | `T=30` | `WM/CatalogNo` | — |
| `COMMENT` | `T=30` | `Description` | `ICMT` |
| `COMPILATION` | `T=30` | — | — |
| `COMPOSER` | `T=30` | `WM/Composer` | — |
| `COMPOSERSORT` | `T=30` | — | — |
| `CONDUCTOR` | `T=30` | `WM/Conductor` | — |
| `CONTENTGROUP` | `T=30` | `WM/ContentGroupDescription` | — |
| `COPYRIGHT` | `T=30` | `Copyright` | `ICOP` |
| `DATE` | — | — | — |
| `DESCRIPTION` | `T=30` | — | — |
| `DISCNUMBER` | `T=50 PART_NUMBER` | `WM/PartOfSet` | — |
| `ENCODEDBY` | `MKA: T=30 ENCODED_BY MKV: T=50 ENCODED_BY` | `WM/EncodedBy` | — |
| `ENCODERSETTINGS` | `MKA: T=30 MKV: T=50` | `WM/EncodingSettings` | — |
| `ENCODINGTIME` | `T=30` | `WM/EncodingTime` | — |
| `FILEOWNER` | `MKA: T=30 PURCHASE_OWNER MKV: T=50 PURCHASE_OWNER` | — | — |
| `FILETYPE` | `T=30` | — | — |
| `GENRE` | `T=30` | `WM/Genre` | `IGNR` |
| `GROUPING` | `T=30` | — | — |
| `INITIALKEY` | `T=30 INITIAL_KEY` | `WM/InitialKey` | — |
| `INVOLVEDPEOPLE` | `T=30` | — | — |
| `ISRC` | `T=30` | `WM/ISRC` | — |
| `LANGUAGE` | `T=30` | `WM/Language` | `ILNG` |
| `LENGTH` | `T=30` | — | — |
| `LYRICIST` | `T=30` | — | — |
| `MEDIATYPE` | `T=50 ORIGINAL_MEDIA_TYPE` | — | — |
| `MIXARTIST` | `T=30 REMIXED_BY` | — | — |
| `MOOD` | `T=30` | `WM/Mood` | — |
| `MOVEMENTNAME` | `T=20 TITLE` | — | — |
| `MOVEMENT` | `T=20 PART_NUMBER` | — | — |
| `MOVEMENTTOTAL` | `T=30` | — | — |
| `MUSICBRAINZ_ALBUMARTISTID` | — | — | — |
| `MUSICBRAINZ_ALBUMID` | — | — | — |
| `MUSICBRAINZ_ALBUMRELEASECOUNTRY` | — | — | — |
| `MUSICBRAINZ_ALBUMSTATUS` | — | — | — |
| `MUSICBRAINZ_ALBUMTYPE` | — | — | — |
| `MUSICBRAINZ_ARTISTID` | — | — | — |
| `MUSICBRAINZ_DISCID` | — | — | — |
| `MUSICBRAINZ_ORIGINALALBUMID` | — | — | — |
| `MUSICBRAINZ_ORIGINALARTISTID` | — | — | — |
| `MUSICBRAINZ_RELEASEGROUPID` | — | — | — |
| `MUSICBRAINZ_RELEASETRACKID` | — | — | — |
| `MUSICBRAINZ_TRACKID` | — | — | — |
| `MUSICBRAINZ_TRMID` | — | — | — |
| `MUSICBRAINZ_WORKID` | — | — | — |
| `MUSICIANCREDITS` | `T=30` | — | — |
| `NARRATOR` | `T=30` | — | — |
| `NETRADIOOWNER` | `T=30` | — | — |
| `NETRADIOSTATION` | `T=30` | — | — |
| `ORIGALBUM` | `T=30` | `WM/OriginalAlbumTitle` | — |
| `ORIGARTIST` | `T=30` | `WM/OriginalArtist` | — |
| `ORIGFILENAME` | `T=30` | `WM/OriginalFilename` | — |
| `ORIGLYRICIST` | `T=30` | `WM/OriginalLyricist` | — |
| `ORIGYEAR` | `T=30` | `WM/OriginalReleaseYear` | — |
| `PODCAST` | `T=30` | — | — |
| `PODCASTCATEGORY` | `T=30` | — | — |
| `PODCASTDESC` | `T=30` | — | — |
| `PODCASTID` | `T=30` | — | — |
| `PODCASTKEYWORDS` | `T=30` | — | — |
| `PODCASTURL` | `T=30` | — | — |
| `POPULARIMETER` | `T=30` | — | — |
| `PUBLISHER` | `T=30` | `WM/Publisher` | — |
| `RATING MM` | `T=30` | — | — |
| `RATING WMP` | `T=30` | `WM/SharedUserRating` | — |
| `RELEASETIME` | `T=50 DATE_RELEASED` | — | — |
| `SETSUBTITLE` | `MKA: T=50 SUBTITLE MKV: T=60 SUBTITLE` | — | — |
| `SUBTITLE` | `MKA: T=30 MKV: T=50` | `WM/SubTitle` | — |
| `TAGGINGTIME` | `T=30 DATE_TAGGED` | — | — |
| `TITLE` | `MKA: T=30 MKV: T=50` | `Title` | `INAM` |
| `TITLESORT` | `MKA: T=30 SORT_WITH MKV: T=50 SORT_WITH` | `WM/TitleSortOrder` | — |
| `TRACK` | `T=30 PART_NUMBER` | `WM/TrackNumber` | `ITRK` |
| `UNIQUEFILEID` | — | — | — |
| `UNSYNCEDLYRICS` | `T=30 LYRICS` | `WM/Lyrics` | — |
| `WWW` | `T=30` | — | — |
| `WWWARTIST` | `T=30` | `WM/AuthorURL` | — |
| `WWWAUDIOFILE` | `T=30` | `WM/AudioFileURL` | — |
| `WWWAUDIOSOURCE` | `T=30` | `WM/AudioSourceURL` | — |
| `WWWCOMMERCIALINFO` | `MKA: T=30 PURCHASE_INFO MKV: T=50 PURCHASE_INFO` | `WM/PromotionURL` | — |
| `WWWCOPYRIGHT` | `T=30` | `CopyrightURL` | — |
| `WWWPAYMENT` | `MKA: T=30 PURCHASE_ITEM MKV: T=50 PURCHASE_ITEM` | — | — |
| `WWWPUBLISHER` | `T=30` | — | — |
| `WWWRADIOPAGE` | `T=30` | — | — |
| `YEAR` | `T=50 DATE_RECORDED` | `WM/Year` | `ICRD` |
| `DIRECTOR` | — | — | — |
| `ITUNESACCOUNT` | — | — | — |
| `ITUNESADVISORY` | — | — | — |
| `ITUNESALBUMID` | — | — | — |
| `ITUNESARTISTID` | — | — | — |
| `ITUNESCATALOGID` | — | — | — |
| `ITUNESCOMPOSERID` | — | — | — |
| `ITUNESCOUNTRYID` | — | — | — |
| `ITUNESGAPLESS` | — | — | — |
| `ITUNESGENREID` | — | — | — |
| `ITUNESHDVIDEO` | — | — | — |
| `ITUNESMEDIATYPE` | — | — | — |
| `ITUNESOWNER` | — | — | — |
| `ITUNESPURCHASEDATE` | — | — | — |
| `RATE` | — | — | — |
| `SHOWMOVEMENT` | — | — | — |
| `STOREDESCRIPTION` | — | — | — |
| `TOTALDISCS` | — | — | — |
| `TOTALTRACKS` | — | — | — |
| `TVEPISODE` | `T=50 PART_NUMBER` | — | — |
| `TVEPISODEID` | — | — | — |
| `TVNETWORK` | — | — | — |
| `TVSEASON` | `T=60 PART_NUMBER` | — | — |
| `TVSHOW` | — | — | — |
| `TVSHOWSORT` | — | — | — |
| `WORK` | — | — | — |
<!-- native-mapping-containers:end -->

In recommended Matroska video mapping, target 50 `PART_NUMBER` is exposed as `TVEPISODE`, and target 60 `PART_NUMBER` as `TVSEASON`. The audio `DISCNUMBER` rule is not also applied to that episode value. Mapped native names stored at other, unrecognized levels may remain hidden and preserved. Track/edition/attachment-scoped metadata is not automatically merged into global editable fields; supported chapter-targeted tags belong to their chapter rows.

ASF `WM/TrackNumber` is the normal track mapping. OpenTag also reads legacy zero-based `WM/Track` as TRACK plus one when no modern value is present, and keeps an existing legacy attribute synchronized on a TRACK edit. This fallback is not a separate field shown in the table.

RIFF INFO holds one string per identifier. Multiple edited values are joined with `; ` there, so it cannot preserve separate values in the same way as a multi-value tag. A WAV file can also carry ID3v2 metadata alongside INFO.

## Custom-field fallback

| Tag family | A field without a dedicated mapping |
|---|---|
| ID3v2 | User text `TXXX:FIELDNAME`. Described comments/lyrics and owner-specific identifiers have their own syntax. |
| MP4 | Freeform `----:com.apple.iTunes:FIELDNAME`. An explicit freeform namespace/name is also supported. |
| Vorbis Comments | The actual field name stored in the comment list. |
| APE | The actual field name stored in the APE item. |
| ASF / Windows Media | An attribute under its own name, unless it conflicts with another mapped native attribute. |
| Matroska | An unmapped name is written as a global target-30 SimpleTag; reserved native names cannot be used to bypass their mapped meaning. |
| RIFF INFO | No general long-name fallback. An unmapped four-character uppercase/digit identifier beginning with `I`, such as `IENG`, can identify an INFO field. |

For example, ASF’s native `Description` belongs to COMMENT; trying to use a separate custom `DESCRIPTION` field can conflict with that mapping and be refused.

An empty fixed-table cell can therefore produce generic storage on an edit. This is different from claiming a dedicated standard mapping exists. Conversely, binary or unsupported native structures can be preserved without being exposed as a writable text field.

## Field-specific storage rules

The tables identify native locations. Several fields also impose value structure:

- `TRACK` and `DISCNUMBER` share their MP4 atoms with `TOTALTRACKS` and `TOTALDISCS`; inconsistent simultaneous totals are rejected.
- `MOVEMENT` and `MOVEMENTTOTAL` share ID3v2 MVIN.
- `COMPILATION`, `PODCAST`, gapless and movement flags, MP4 store IDs, media types, ratings, credits, and lyrics have their own allowed value forms. See [Structured values](field-names.md#structured-values).
- `MUSICBRAINZ_TRACKID` means recording ID; its ID3 storage is the `http://musicbrainz.org` UFID owner. `MUSICBRAINZ_RELEASETRACKID` is a different identifier.
- `YEAR` and `DATE` are not interchangeable date transformations. A user-defined alias changes a name while leaving the text unchanged.

The source importers can supply BARCODE and CATALOGNUMBER from release data. Historical Mp3tag version annotations about introducing those fields do not describe OpenTag release availability.

## User-defined field names

Use **File → Options… → Tags → Mapping** for per-format Source → Target aliases. On read, the source name is shown as the target. On write, the target is translated back to each source mapped to it. Several sources can share one target; one source cannot map to several different targets. A Vorbis `DATE` → `YEAR` rule is an example of harmonizing names without changing the date text.

These aliases operate above the native identifier mappings. Enter OpenTag's readable field name as Source for ID3v2/MP4, rather than a frame or atom code. A custom alias does not replace the built-in mapping from `TPE1` to ARTIST. Matroska is not a selectable tag type in that alias editor.

## Related

- [Field vocabulary and structured values](field-names.md)
- [Format capabilities](file-formats.md)
- [Filter syntax](filter-syntax.md)

<!-- verified: baseline007a806 crates/tag-core/src/mapping.rs FIELD_MAPPINGS, mp4_ident (ARTIST writer override), mp4_name, patch_id3, patch_mp4, map_read/map_write; crates/tag-core/src/tags.rs tag_fields/riff_key; crates/tag-core/src/matroska.rs mapping_rules/Model; crates/tag-core/src/asf/mod.rs legacy WM/Track. Source-generated factual tables are checked by crates/tag-core/tests/user_guide_filter_reference.rs. Registry ARTIST spelling discrepancy recorded separately in semantic ledger; no app change. -->
