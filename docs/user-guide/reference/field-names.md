# Field names

A field name identifies a piece of editable metadata, such as `ARTIST`. Its values are text shown in the tag panel, file-list columns, or **Extended tags…**. A [mapping](tag-mapping.md) translates that logical name to the format's native storage.

!!! tip "New to this?"
    Keep `ARTIST` for the performer and `ALBUMARTIST` for the release credit. Use `TITLE` for the track name. Those distinctions make a mixed collection easier to sort and rename.

## Names, values, and placeholders

To inspect or add fields, select files and open **View → Extended tags…**. Frequently used fields can be added to the tag panel, and fields can be displayed through custom file-list columns. A field named `MOOD` is read as `%mood%` in a format string. A field with a space, such as `RATING WMP`, uses `%rating wmp%`; in filters quote the field name: `"RATING WMP" PRESENT`.

Lookups are generally case-insensitive, and the panel configuration normalizes names to uppercase. Vorbis Comments and APE retain their native field names unless a user mapping changes the displayed name. Thus a FLAC `DATE` is not automatically renamed to YEAR merely because an MP3 date appears as YEAR.

Multiple values belong to one field; they are not necessarily one text string joined with punctuation. For example, ARTIST can contain `Juniper Vale` and `Mira Solberg` separately. A player may combine them for display. Use the app's separate-value editing rather than inserting a semicolon and assuming every format will split it.

## Field vocabulary

The tables cover all 118 named entries in TagGroove's fixed native mapping vocabulary. The **meaning** describes the metadata, not a promise that every format has a dedicated native location or every player displays it. The [mapping tables](tag-mapping.md) and [format capabilities](file-formats.md) explain storage and limits.

### Core description

| Field | Meaning |
|---|---|
| `TITLE` | Name of this track or item. |
| `ARTIST` | Track performer; supports more than one value where the format allows it. |
| `ALBUM` | Release or collection title. |
| `ALBUMARTIST` | Artist credited for the album as a whole, which may differ from a track’s performer. |
| `YEAR` | Recording/release date text in the format’s mapped date field; it can contain more than four digits. |
| `DATE` | A separate date field, including ID3v2.3 TDAT; do not assume it is the same native field as YEAR. |
| `TRACK` | Track number, often with a total such as 3/8. |
| `DISCNUMBER` | Disc number, often with a total such as 1/2; Matroska video uses different episode semantics. |
| `GENRE` | Style or category text; old ID3v1 has a fixed indexed vocabulary. |
| `COMMENT` | Comment text; ID3v2 descriptions are addressed separately. |
| `DESCRIPTION` | Short description with a dedicated MP4 desc mapping; distinct from COMMENT. |
| `SUBTITLE` | Subtitle for the item. |
| `SETSUBTITLE` | Subtitle describing a disc/set rather than the individual track. |
| `CONTENTGROUP` | Content-group description, mapped to TIT1 in ID3v2. |
| `GROUPING` | Grouping field mapped to GRP1 in ID3v2; do not interchange with CONTENTGROUP automatically. |
| `COMPILATION` | Compilation flag: write 1 or remove the field in the constrained ID3/MP4 representations. |
| `COPYRIGHT` | Copyright notice. |
| `LANGUAGE` | Language of the content; it does not choose the interface language. |
| `LENGTH` | Stored length metadata, not the measured duration information placeholder. |
| `BPM` | Beats per minute; MP4 uses an integer representation. |
| `INITIALKEY` | Musical key text. |
| `MOOD` | Mood description. |
| `MEDIATYPE` | Original source medium/type description; distinct from the MP4 ITUNESMEDIATYPE enumeration. |
| `FILETYPE` | Stored file-type description; editing it does not transcode the audio. |

### People and credits

| Field | Meaning |
|---|---|
| `COMPOSER` | Composer credit. |
| `CONDUCTOR` | Conductor credit. |
| `LYRICIST` | Writer of the words. |
| `MIXARTIST` | Remixer or mix artist. |
| `NARRATOR` | Narration credit. |
| `INVOLVEDPEOPLE` | Role/person pairs in ID3v2 IPLS/TIPL; see the structured syntax below. |
| `MUSICIANCREDITS` | Musician role/person pairs; ID3v2.4 uses TMCL. |
| `PUBLISHER` | Publisher label/name. |
| `ENCODEDBY` | Person or organization credited with encoding. |
| `ENCODERSETTINGS` | Description of encoder settings; it does not reconfigure an encoder. |
| `FILEOWNER` | Owner/purchaser metadata; it does not change filesystem ownership. |

### Sorting and classical works

| Field | Meaning |
|---|---|
| `ALBUMSORT` | Alternate sort text for album title. |
| `ALBUMARTISTSORT` | Alternate sort text for the album artist. |
| `ARTISTSORT` | Alternate sort text for track artist. |
| `COMPOSERSORT` | Alternate sort text for composer. |
| `TITLESORT` | Alternate sort text for title. |
| `MOVEMENTNAME` | Movement title. |
| `MOVEMENT` | Movement number. |
| `MOVEMENTTOTAL` | Number of movements; shares ID3v2 MVIN with MOVEMENT. |
| `WORK` | Name of the larger work, with a dedicated MP4 atom. |
| `SHOWMOVEMENT` | MP4 flag to request movement-oriented presentation: 1 or deletion. |

### Identifiers and dates

| Field | Meaning |
|---|---|
| `ACOUSTID_ID` | AcoustID identifier text. |
| `ACOUSTID_FINGERPRINT` | Stored acoustic fingerprint text; editing it does not calculate a fingerprint. |
| `BARCODE` | Release barcode. The baseline MusicBrainz and Discogs importers can supply it. |
| `CATALOGNUMBER` | Label/catalog identifier. The baseline MusicBrainz and Discogs importers can supply it. |
| `ISRC` | International Standard Recording Code for the recording. |
| `UNIQUEFILEID` | ID3v2 owner/identifier pair for text identifiers; MusicBrainz recording IDs use their dedicated field. |
| `ENCODINGTIME` | Time the content was encoded. |
| `RELEASETIME` | Release date/time. |
| `TAGGINGTIME` | Time associated with tagging. |
| `ORIGYEAR` | Original recording/release year or date representation. |
| `ORIGALBUM` | Original album title. |
| `ORIGARTIST` | Original performer credit. |
| `ORIGFILENAME` | Original filename stored as metadata; changing it does not rename the file. |
| `ORIGLYRICIST` | Original lyricist credit. |

### MusicBrainz identifiers

| Field | Meaning |
|---|---|
| `MUSICBRAINZ_ALBUMARTISTID` | MusicBrainz artist ID(s) for the release credit. |
| `MUSICBRAINZ_ALBUMID` | MusicBrainz release ID. |
| `MUSICBRAINZ_ALBUMRELEASECOUNTRY` | Release country from MusicBrainz. |
| `MUSICBRAINZ_ALBUMSTATUS` | Release status, for example official. |
| `MUSICBRAINZ_ALBUMTYPE` | Release-group type information. |
| `MUSICBRAINZ_ARTISTID` | MusicBrainz artist ID(s) for the track credit. |
| `MUSICBRAINZ_DISCID` | MusicBrainz disc ID, distinct from a CDDB/freedb disc ID. |
| `MUSICBRAINZ_ORIGINALALBUMID` | Legacy/original album identifier field. |
| `MUSICBRAINZ_ORIGINALARTISTID` | Legacy/original artist identifier field. |
| `MUSICBRAINZ_RELEASEGROUPID` | MusicBrainz release-group ID, which groups related editions. |
| `MUSICBRAINZ_RELEASETRACKID` | MusicBrainz release-track ID, identifying the track on a particular release. |
| `MUSICBRAINZ_TRACKID` | MusicBrainz recording ID; do not confuse it with RELEASETRACKID. |
| `MUSICBRAINZ_TRMID` | Legacy MusicBrainz TRM identifier. |
| `MUSICBRAINZ_WORKID` | MusicBrainz work ID(s). |

### Ratings, lyrics, podcasts, and links

| Field | Meaning |
|---|---|
| `POPULARIMETER` | ID3v2 raw rating-owner/counter representation. |
| `RATING MM` | MediaMonkey-style ID3v2 rating convenience field. |
| `RATING WMP` | Windows Media Player-style rating convenience field, also mapped in ASF. |
| `UNSYNCEDLYRICS` | Unsynchronized lyrics; ID3v2 uses language||text syntax. |
| `PODCAST` | Podcast flag; 1 or deletion in constrained native representations. |
| `PODCASTCATEGORY` | Podcast category. |
| `PODCASTDESC` | Podcast description. |
| `PODCASTID` | Podcast identifier. |
| `PODCASTKEYWORDS` | Podcast keywords. |
| `PODCASTURL` | Podcast feed/location URL. |
| `NETRADIOOWNER` | Internet radio owner information. |
| `NETRADIOSTATION` | Internet radio station name. |
| `WWW` | General user URL. |
| `WWWARTIST` | Artist URL. |
| `WWWAUDIOFILE` | URL for the audio file. |
| `WWWAUDIOSOURCE` | URL for its source. |
| `WWWCOMMERCIALINFO` | Commercial/purchase information URL. |
| `WWWCOPYRIGHT` | Copyright-information URL. |
| `WWWPAYMENT` | Payment URL. |
| `WWWPUBLISHER` | Publisher URL. |
| `WWWRADIOPAGE` | Radio station page URL. |

### MP4 store, video, and totals

| Field | Meaning |
|---|---|
| `DIRECTOR` | Director credit with a dedicated MP4 atom. |
| `ITUNESACCOUNT` | Store account identifier. |
| `ITUNESADVISORY` | Content advisory: 0 none, 1 explicit, 2 clean. |
| `ITUNESALBUMID` | Store album ID; MP4 accepts an unsigned 64-bit integer. |
| `ITUNESARTISTID` | Store artist ID. |
| `ITUNESCATALOGID` | Store catalog/content ID. |
| `ITUNESCOMPOSERID` | Store composer ID. |
| `ITUNESCOUNTRYID` | Store country/storefront ID. |
| `ITUNESGAPLESS` | Gapless-album flag: 1 or deletion; it does not re-encode audio. |
| `ITUNESGENREID` | Store genre ID. |
| `ITUNESHDVIDEO` | HD-video flag: 0 or 1. |
| `ITUNESMEDIATYPE` | Named MP4 media-type enumeration, listed below. |
| `ITUNESOWNER` | Store owner metadata. |
| `ITUNESPURCHASEDATE` | Purchase date text. |
| `RATE` | MP4 native rating integer, separate from ID3 POPM convenience fields. |
| `STOREDESCRIPTION` | Store description. |
| `TOTALDISCS` | MP4 total in the disk pair, separate from its current-disc component. |
| `TOTALTRACKS` | MP4 total in the trkn pair, separate from its current-track component. |
| `TVEPISODE` | Episode number; MP4 tves or Matroska video target type 50 PART_NUMBER. |
| `TVEPISODEID` | Episode identifier text. |
| `TVNETWORK` | Television network text. |
| `TVSEASON` | Season number; MP4 tvsn or Matroska video target type 60 PART_NUMBER. |
| `TVSHOW` | Show title. |
| `TVSHOWSORT` | Alternate sort text for the show. |

## Structured values

### Flags and MP4 media types

For `COMPILATION`, `PODCAST`, `ITUNESGAPLESS`, and `SHOWMOVEMENT` in their constrained ID3/MP4 mappings, use `1` to enable or remove the field to disable. The writer rejects other values in those paths. `ITUNESHDVIDEO` separately accepts `0` or `1`; `ITUNESADVISORY` accepts `0` (none), `1` (explicit), or `2` (clean).

`ITUNESMEDIATYPE` accepts these names, ignoring case: `Normal`, `Audiobook`, `Bookmark`, `Music Video`, `Movie`, `TV Show`, `Booklet`, `Ringtone`, `Podcast`, and `iTunes U`. This is a metadata classification; it does not convert the codec or container.

### Credits

ID3v2 `INVOLVEDPEOPLE` and `MUSICIANCREDITS` use semicolon-separated role/person pairs, with a colon between role and person:

```text
Drums:Mira Solberg;Bass:Kofi Reed;
```

Do not substitute an unstructured list of names when the role relationships matter. `INVOLVEDPEOPLE` maps to IPLS in v2.3 and TIPL in v2.4. `MUSICIANCREDITS` has the dedicated TMCL frame in v2.4; a field without a dedicated frame in the selected revision can use custom text storage instead.

### Ratings

`POPULARIMETER` represents ID3v2 POPM as `owner|rating|counter`. The counter is optional and defaults to 0. The historical documented rating range runs from 1 (lowest) to 255 (highest); TagGroove additionally accepts 0. Its parser accepts an unsigned byte for rating and an unsigned 64-bit counter.

```text
listener@example.test|196|12
```

That is a raw byte rating, not a number of stars. With **Options → Tags → Advanced → Always display POPULARIMETER values for ID3v2 ratings** enabled, these raw values replace the app-specific convenience view after tags are reloaded.

`RATING MM` uses the MediaMonkey owner convention and accepts numeric star values or `*` for a star and `-` for a half star; `**-` means 2.5. `RATING WMP` accepts whole star values or `*` characters; `****` means 4. In the documented star interpretation, 1 is the lowest and 5 the highest. Both writers accept 0–5, with half steps only for MM. Each convenience field allows one rating for its owner and preserves the existing counter when changing the rating.

These mappings describe TagGroove's stored representations. They do not establish current playback/display behavior in MediaMonkey or Windows Media Player. The MP4 `RATE` field is a separate native integer, not automatically the same rating scale.

### Lyrics, comments, and identifiers

For ID3v2 `UNSYNCEDLYRICS`, prefix the text with a three-letter ISO-639-2 language code and two vertical bars:

```text
eng||Lanterns drift across the bay.
```

The example is invented text. ID3v2 stores one lyrics value per language and description. `UNSYNCEDLYRICS Translation` addresses a frame with description `Translation`; `COMMENT Session` similarly addresses a described comment. Editing plain `COMMENT` affects its undescribed frame, rather than deleting all described comments. A single ID3 comment description holds one value; use separate descriptions for separate comments. The `language||text` prefix is an ID3v2 representation, not a universal requirement for every lyrics field in every format.

For ID3v2 `UNIQUEFILEID`, use `owner|identifier`, with one text identifier per owner and at most 64 identifier bytes. The MusicBrainz owner `http://musicbrainz.org` belongs in `MUSICBRAINZ_TRACKID` instead. Binary identifiers can remain preserved without being exposed as editable text.

### Numbers and dates

A native MP4 track/disc atom contains a number and total together. A `TRACK` value such as `3/8` supplies both. `TOTALTRACKS` or `TOTALDISCS` can address the total separately; inconsistent simultaneous totals are rejected. The native pair uses unsigned 16-bit values. Do not assume these MP4 total fields are universally split out of an ID3 track string.

`MOVEMENT` and `MOVEMENTTOTAL` share ID3v2 MVIN as `number/total`. ID3v2.4 timestamp fields such as YEAR, RELEASETIME, ENCODINGTIME, and TAGGINGTIME require a timestamp the native writer can parse. A field-name change alone does not turn arbitrary date text into that representation.

## Custom fields

The fixed vocabulary is not the complete set of editable fields. Add a descriptive custom name such as `LISTENING_CONTEXT` when a supported format allows it. ID3v2 normally uses user-text frames, MP4 uses freeform atoms, APE/Vorbis retain their own names, and ASF keeps attributes. RIFF INFO has stricter four-character identifiers; arbitrary long custom names do not automatically gain an INFO entry.

Names must be nonempty and cannot contain NUL or line breaks. Vorbis names also cannot contain `=`. Values cannot contain NUL. The tag-panel editor applies its own additional validation and does not offer arbitrary technical `_` fields as editable metadata.

An empty-looking field and a missing field can filter differently before saving; see [Filter presence rules](filter-syntax.md#presence-missing-values-and-empty-text). Removing a field is the reliable way to request absence where a boolean convention requires deletion.

## Special action targets versus information fields

Some action types interpret these names as selectors or rename requests. They are not ordinary custom text fields, and not every action supports every selector.

| Target | Meaning in supporting actions |
|---|---|
| `_TAG` | Apply to metadata field values. This is separate from `%_tag%`, which reports tag-format information. |
| `_ALL` | Apply to tag values and the filename in text-transform actions that support that combination. It is not a promise to transform directories or binary covers. |
| `_FILENAME` | Request a filename transformation. Extension handling depends on the action type; inspect its preview/result before applying. |
| `_DIRECTORY` | Request a directory transformation. In the tag panel this special control instead acts as a folder switcher. |
| `_FIELDNAME` | Transform field names, only for APE/Vorbis-style names. The result must remain nonempty and cannot collide case-insensitively with another field. |

Technical placeholders such as `%_length%`, `%_bitrate%`, and `%_path%` describe the file or application context. They are covered in [Placeholders](placeholders.md). Creating a custom `LENGTH` tag changes stored metadata, not the actual duration.

TagGroove also exposes `CHAPTERS` for supported MP4/Matroska chapter editing and `CUESHEET` for embedded cue-sheet content. These require their specific syntax and supported format; a custom field with that name is not automatically a chapter implementation in another container.

## Related

- [Native mapping tables](tag-mapping.md)
- [Supported file formats](file-formats.md)
- [Filter syntax](filter-syntax.md)
- [Placeholders](placeholders.md)

<!-- verified: baseline007a806 crates/tag-core/src/mapping.rs FIELD_MAPPINGS, patch_id3, patch_mp4, parse_stars, patch_described, patch_unique_file_ids; crates/tag-core/src/tags.rs tag_fields, normalized_updates, riff_key; crates/tag-core/src/actions.rs metadata_keys and transform; src/tag-panel.ts; MusicBrainz/Discogs importers. All118vocabulary names checked by documentation contract tests. -->
