//! Factual native identifiers audited against https://docs.mp3tag.de/mapping/.
//! Vorbis/APE names are deliberately not subject to this fixed registry.
use super::Fields;
#[path = "id3_v23.rs"]
mod id3_v23;
use lofty::{
    config::{ParsingMode, WriteOptions},
    id3::v2::{
        BinaryFrame, CommentFrame, ExtendedUrlFrame, Frame, FrameId, Id3v2Tag, Id3v2Version,
        KeyValueFrame, PopularimeterFrame, TextInformationFrame, TimestampFrame,
        UniqueFileIdentifierFrame, UnsynchronizedTextFrame, UrlLinkFrame,
    },
    mp4::{Atom, AtomData, AtomIdent, DataType, Ilst},
    tag::{items::Timestamp, TagExt, TagType},
    TextEncoding,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize)]
pub struct FieldMapping {
    pub field: &'static str,
    pub id3v23: &'static str,
    pub id3v24: &'static str,
    pub mp4: &'static str,
    pub matroska: &'static str,
    pub asf: &'static str,
    pub riff: &'static str,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFieldMapping {
    pub tag_type: String,
    pub source: String,
    pub target: String,
}

fn rule_matches(rule: &UserFieldMapping, kind: TagType) -> bool {
    match kind {
        TagType::Id3v2 => rule.tag_type.eq_ignore_ascii_case("ID3v2"),
        TagType::Mp4Ilst => rule.tag_type.eq_ignore_ascii_case("MP4"),
        TagType::VorbisComments => rule.tag_type.eq_ignore_ascii_case("VorbisComment"),
        TagType::Ape => rule.tag_type.eq_ignore_ascii_case("APEv2"),
        _ => false,
    }
}
pub fn validate_user_mappings(rules: &[UserFieldMapping]) -> Result<(), String> {
    let mut sources = std::collections::BTreeMap::new();
    for rule in rules {
        if !["ID3v2", "MP4", "VorbisComment", "APEv2", "WMA"]
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&rule.tag_type))
        {
            return Err(format!("Unknown mapping tag type: {}", rule.tag_type));
        }
        if [&rule.source, &rule.target]
            .iter()
            .any(|s| s.trim().is_empty() || s.contains(['\0', '\r', '\n']))
        {
            return Err("Mappings require nonempty source and target names".into());
        }
        let key = (rule.tag_type.to_lowercase(), rule.source.to_lowercase());
        if let Some(target) = sources.insert(key, rule.target.to_lowercase()) {
            if target != rule.target.to_lowercase() {
                return Err(format!(
                    "One source cannot map to multiple targets: {}",
                    rule.source
                ));
            }
        }
    }
    Ok(())
}
pub fn map_read_fields(
    fields: &Fields,
    kind: TagType,
    rules: &[UserFieldMapping],
) -> Result<Fields, String> {
    validate_user_mappings(rules)?;
    let mut output = Fields::new();
    for (source, values) in fields {
        let target = rules
            .iter()
            .find(|r| rule_matches(r, kind) && r.source.eq_ignore_ascii_case(source))
            .map(|r| r.target.as_str())
            .unwrap_or(source);
        let existing = output
            .keys()
            .find(|k| k.eq_ignore_ascii_case(target))
            .cloned();
        if let Some(existing) = existing {
            let list = output
                .get_mut(&existing)
                .ok_or("Mapping target disappeared")?;
            for v in values {
                if !list.contains(v) {
                    list.push(v.clone());
                }
            }
        } else {
            output.insert(target.to_owned(), values.clone());
        }
    }
    Ok(output)
}
pub fn map_write_fields(
    fields: &Fields,
    kind: TagType,
    rules: &[UserFieldMapping],
) -> Result<Fields, String> {
    validate_user_mappings(rules)?;
    let mut output = Fields::new();
    for (target, values) in fields {
        let sources: Vec<_> = rules
            .iter()
            .filter(|r| rule_matches(r, kind) && r.target.eq_ignore_ascii_case(target))
            .map(|r| r.source.as_str())
            .collect();
        let sources = if sources.is_empty() {
            vec![target.as_str()]
        } else {
            sources
        };
        for source in sources {
            if let Some(existing) = output
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(source))
                .map(|(_, v)| v)
            {
                if existing != values {
                    return Err(format!("Conflicting updates for mapped source {source}"));
                }
            }
            output.insert(source.to_owned(), values.clone());
        }
    }
    Ok(output)
}
pub const FIELD_MAPPINGS: &[FieldMapping] = &[
    FieldMapping {
        field: "ACOUSTID_ID",
        id3v23: "TXXX:Acoustid Id",
        id3v24: "TXXX:Acoustid Id",
        mp4: "----:com.apple.iTunes:Acoustid Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ACOUSTID_FINGERPRINT",
        id3v23: "TXXX:Acoustid Fingerprint",
        id3v24: "TXXX:Acoustid Fingerprint",
        mp4: "----:com.apple.iTunes:Acoustid Fingerprint",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ALBUM",
        id3v23: "TALB",
        id3v24: "TALB",
        mp4: "©alb",
        matroska: "MKA:T=50 TITLE MKV: T=60 TITLE",
        asf: "WM/AlbumTitle",
        riff: "IPRD",
    },
    FieldMapping {
        field: "ALBUMSORT",
        id3v23: "TSOA",
        id3v24: "TSOA",
        mp4: "soal",
        matroska: "MKA: T=50 SORT_WITH MKV: T=60 SORT_WITH",
        asf: "WM/AlbumSortOrder",
        riff: "",
    },
    FieldMapping {
        field: "ALBUMARTIST",
        id3v23: "TPE2",
        id3v24: "TPE2",
        mp4: "aART",
        matroska: "MKA: T=50 ARTIST MKV: T=60 ARTIST",
        asf: "WM/AlbumArtist",
        riff: "",
    },
    FieldMapping {
        field: "ALBUMARTISTSORT",
        id3v23: "TSO2",
        id3v24: "TSO2",
        mp4: "soaa",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ARTIST",
        id3v23: "TPE1",
        id3v24: "TPE1",
        mp4: "©art",
        matroska: "MKA: T=30 ARTIST MKV: T=50 ARTIST",
        asf: "Author",
        riff: "IART",
    },
    FieldMapping {
        field: "ARTISTSORT",
        id3v23: "TSOP",
        id3v24: "TSOP",
        mp4: "soar",
        matroska: "T=30",
        asf: "WM/ArtistSortOrder",
        riff: "",
    },
    FieldMapping {
        field: "BARCODE",
        id3v23: "TXXX:BARCODE",
        id3v24: "TXXX:BARCODE",
        mp4: "----:com.apple.iTunes:BARCODE",
        matroska: "T=30",
        asf: "WM/Barcode",
        riff: "",
    },
    FieldMapping {
        field: "BPM",
        id3v23: "TBPM",
        id3v24: "TBPM",
        mp4: "tmpo",
        matroska: "T=30",
        asf: "WM/BeatsPerMinute",
        riff: "",
    },
    FieldMapping {
        field: "CATALOGNUMBER",
        id3v23: "TXXX:CATALOGNUMBER",
        id3v24: "TXXX:CATALOGNUMBER",
        mp4: "----:com.apple.iTunes:CATALOGNUMBER",
        matroska: "T=30",
        asf: "WM/CatalogNo",
        riff: "",
    },
    FieldMapping {
        field: "COMMENT",
        id3v23: "COMM",
        id3v24: "COMM",
        mp4: "©cmt",
        matroska: "T=30",
        asf: "Description",
        riff: "ICMT",
    },
    FieldMapping {
        field: "COMPILATION",
        id3v23: "TCMP",
        id3v24: "TCMP",
        mp4: "cpil",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "COMPOSER",
        id3v23: "TCOM",
        id3v24: "TCOM",
        mp4: "©wrt",
        matroska: "T=30",
        asf: "WM/Composer",
        riff: "",
    },
    FieldMapping {
        field: "COMPOSERSORT",
        id3v23: "TSOC",
        id3v24: "TSOC",
        mp4: "soco",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "CONDUCTOR",
        id3v23: "TPE3",
        id3v24: "TPE3",
        mp4: "©con",
        matroska: "T=30",
        asf: "WM/Conductor",
        riff: "",
    },
    FieldMapping {
        field: "CONTENTGROUP",
        id3v23: "TIT1",
        id3v24: "TIT1",
        mp4: "©grp",
        matroska: "T=30",
        asf: "WM/ContentGroupDescription",
        riff: "",
    },
    FieldMapping {
        field: "COPYRIGHT",
        id3v23: "TCOP",
        id3v24: "TCOP",
        mp4: "cprt",
        matroska: "T=30",
        asf: "Copyright",
        riff: "ICOP",
    },
    FieldMapping {
        field: "DATE",
        id3v23: "TDAT",
        id3v24: "TXXX:DATE",
        mp4: "",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "DESCRIPTION",
        id3v23: "",
        id3v24: "",
        mp4: "desc",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "DISCNUMBER",
        id3v23: "TPOS",
        id3v24: "TPOS",
        mp4: "disk",
        matroska: "T=50 PART_NUMBER",
        asf: "WM/PartOfSet",
        riff: "",
    },
    FieldMapping {
        field: "ENCODEDBY",
        id3v23: "TENC",
        id3v24: "TENC",
        mp4: "©enc",
        matroska: "MKA: T=30 ENCODED_BY MKV: T=50 ENCODED_BY",
        asf: "WM/EncodedBy",
        riff: "",
    },
    FieldMapping {
        field: "ENCODERSETTINGS",
        id3v23: "TSSE",
        id3v24: "TSSE",
        mp4: "",
        matroska: "MKA: T=30 MKV: T=50",
        asf: "WM/EncodingSettings",
        riff: "",
    },
    FieldMapping {
        field: "ENCODINGTIME",
        id3v23: "",
        id3v24: "TDEN",
        mp4: "",
        matroska: "T=30",
        asf: "WM/EncodingTime",
        riff: "",
    },
    FieldMapping {
        field: "FILEOWNER",
        id3v23: "TOWN",
        id3v24: "TOWN",
        mp4: "",
        matroska: "MKA: T=30 PURCHASE_OWNER MKV: T=50 PURCHASE_OWNER",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "FILETYPE",
        id3v23: "TFLT",
        id3v24: "TFLT",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "GENRE",
        id3v23: "TCON",
        id3v24: "TCON",
        mp4: "©gen | gnre",
        matroska: "T=30",
        asf: "WM/Genre",
        riff: "IGNR",
    },
    FieldMapping {
        field: "GROUPING",
        id3v23: "GRP1",
        id3v24: "GRP1",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "INITIALKEY",
        id3v23: "TKEY",
        id3v24: "TKEY",
        mp4: "",
        matroska: "T=30 INITIAL_KEY",
        asf: "WM/InitialKey",
        riff: "",
    },
    FieldMapping {
        field: "INVOLVEDPEOPLE",
        id3v23: "IPLS",
        id3v24: "TIPL",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ISRC",
        id3v23: "TSRC",
        id3v24: "TSRC",
        mp4: "",
        matroska: "T=30",
        asf: "WM/ISRC",
        riff: "",
    },
    FieldMapping {
        field: "LANGUAGE",
        id3v23: "TLAN",
        id3v24: "TLAN",
        mp4: "",
        matroska: "T=30",
        asf: "WM/Language",
        riff: "ILNG",
    },
    FieldMapping {
        field: "LENGTH",
        id3v23: "TLEN",
        id3v24: "TLEN",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "LYRICIST",
        id3v23: "TEXT",
        id3v24: "TEXT",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MEDIATYPE",
        id3v23: "TMED",
        id3v24: "TMED",
        mp4: "",
        matroska: "T=50 ORIGINAL_MEDIA_TYPE",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MIXARTIST",
        id3v23: "TPE4",
        id3v24: "TPE4",
        mp4: "",
        matroska: "T=30 REMIXED_BY",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MOOD",
        id3v23: "",
        id3v24: "TMOO",
        mp4: "",
        matroska: "T=30",
        asf: "WM/Mood",
        riff: "",
    },
    FieldMapping {
        field: "MOVEMENTNAME",
        id3v23: "MVNM",
        id3v24: "MVNM",
        mp4: "©mvn",
        matroska: "T=20 TITLE",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MOVEMENT",
        id3v23: "MVIN",
        id3v24: "MVIN",
        mp4: "©mvi",
        matroska: "T=20 PART_NUMBER",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MOVEMENTTOTAL",
        id3v23: "MVIN",
        id3v24: "MVIN",
        mp4: "©mvc",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ALBUMARTISTID",
        id3v23: "TXXX:MusicBrainz Album Artist Id",
        id3v24: "TXXX:MusicBrainz Album Artist Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Album Artist Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ALBUMID",
        id3v23: "TXXX:MusicBrainz Album Id",
        id3v24: "TXXX:MusicBrainz Album Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Album Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ALBUMRELEASECOUNTRY",
        id3v23: "TXXX:MusicBrainz Album Release Country",
        id3v24: "TXXX:MusicBrainz Album Release Country",
        mp4: "----:com.apple.iTunes:MusicBrainz Album Release Country",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ALBUMSTATUS",
        id3v23: "TXXX:MusicBrainz Album Status",
        id3v24: "TXXX:MusicBrainz Album Status",
        mp4: "----:com.apple.iTunes:MusicBrainz Album Status",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ALBUMTYPE",
        id3v23: "TXXX:MusicBrainz Album Type",
        id3v24: "TXXX:MusicBrainz Album Type",
        mp4: "----:com.apple.iTunes:MusicBrainz Album Type",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ARTISTID",
        id3v23: "TXXX:MusicBrainz Artist Id",
        id3v24: "TXXX:MusicBrainz Artist Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Artist Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_DISCID",
        id3v23: "TXXX:MusicBrainz Disc Id",
        id3v24: "TXXX:MusicBrainz Disc Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Disc Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ORIGINALALBUMID",
        id3v23: "TXXX:MusicBrainz Original Album Id",
        id3v24: "TXXX:MusicBrainz Original Album Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Original Album Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_ORIGINALARTISTID",
        id3v23: "TXXX:MusicBrainz Original Artist Id",
        id3v24: "TXXX:MusicBrainz Original Artist Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Original Artist Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_RELEASEGROUPID",
        id3v23: "TXXX:MusicBrainz Release Group Id",
        id3v24: "TXXX:MusicBrainz Release Group Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Release Group Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_RELEASETRACKID",
        id3v23: "TXXX:MusicBrainz Release Track Id",
        id3v24: "TXXX:MusicBrainz Release Track Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Release Track Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_TRACKID",
        id3v23: "UFID:http://musicbrainz.org",
        id3v24: "UFID:http://musicbrainz.org",
        mp4: "----:com.apple.iTunes:MusicBrainz Track Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_TRMID",
        id3v23: "TXXX:MusicBrainz TRM Id",
        id3v24: "TXXX:MusicBrainz TRM Id",
        mp4: "----:com.apple.iTunes:MusicBrainz TRM Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICBRAINZ_WORKID",
        id3v23: "TXXX:MusicBrainz Work Id",
        id3v24: "TXXX:MusicBrainz Work Id",
        mp4: "----:com.apple.iTunes:MusicBrainz Work Id",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "MUSICIANCREDITS",
        id3v23: "",
        id3v24: "TMCL",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "NARRATOR",
        id3v23: "",
        id3v24: "",
        mp4: "©nrt",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "NETRADIOOWNER",
        id3v23: "TRSO",
        id3v24: "TRSO",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "NETRADIOSTATION",
        id3v23: "TRSN",
        id3v24: "TRSN",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ORIGALBUM",
        id3v23: "TOAL",
        id3v24: "TOAL",
        mp4: "",
        matroska: "T=30",
        asf: "WM/OriginalAlbumTitle",
        riff: "",
    },
    FieldMapping {
        field: "ORIGARTIST",
        id3v23: "TOPE",
        id3v24: "TOPE",
        mp4: "",
        matroska: "T=30",
        asf: "WM/OriginalArtist",
        riff: "",
    },
    FieldMapping {
        field: "ORIGFILENAME",
        id3v23: "TOFN",
        id3v24: "TOFN",
        mp4: "",
        matroska: "T=30",
        asf: "WM/OriginalFilename",
        riff: "",
    },
    FieldMapping {
        field: "ORIGLYRICIST",
        id3v23: "TOLY",
        id3v24: "TOLY",
        mp4: "",
        matroska: "T=30",
        asf: "WM/OriginalLyricist",
        riff: "",
    },
    FieldMapping {
        field: "ORIGYEAR",
        id3v23: "TORY",
        id3v24: "TDOR",
        mp4: "",
        matroska: "T=30",
        asf: "WM/OriginalReleaseYear",
        riff: "",
    },
    FieldMapping {
        field: "PODCAST",
        id3v23: "PCST",
        id3v24: "PCST",
        mp4: "pcst",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "PODCASTCATEGORY",
        id3v23: "TCAT",
        id3v24: "TCAT",
        mp4: "catg",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "PODCASTDESC",
        id3v23: "TDES",
        id3v24: "TDES",
        mp4: "ldes",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "PODCASTID",
        id3v23: "TGID",
        id3v24: "TGID",
        mp4: "egid",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "PODCASTKEYWORDS",
        id3v23: "TKWD",
        id3v24: "TKWD",
        mp4: "keyw",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "PODCASTURL",
        id3v23: "WFED",
        id3v24: "WFED",
        mp4: "purl",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "POPULARIMETER",
        id3v23: "POPM",
        id3v24: "POPM",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "PUBLISHER",
        id3v23: "TPUB",
        id3v24: "TPUB",
        mp4: "©pub",
        matroska: "T=30",
        asf: "WM/Publisher",
        riff: "",
    },
    FieldMapping {
        field: "RATING MM",
        id3v23: "POPM",
        id3v24: "POPM",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "RATING WMP",
        id3v23: "POPM",
        id3v24: "POPM",
        mp4: "",
        matroska: "T=30",
        asf: "WM/SharedUserRating",
        riff: "",
    },
    FieldMapping {
        field: "RELEASETIME",
        id3v23: "TDRL",
        id3v24: "TDRL",
        mp4: "",
        matroska: "T=50 DATE_RELEASED",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "SETSUBTITLE",
        id3v23: "",
        id3v24: "TSST",
        mp4: "",
        matroska: "MKA: T=50 SUBTITLE MKV: T=60 SUBTITLE",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "SUBTITLE",
        id3v23: "TIT3",
        id3v24: "TIT3",
        mp4: "",
        matroska: "MKA: T=30 MKV: T=50",
        asf: "WM/SubTitle",
        riff: "",
    },
    FieldMapping {
        field: "TAGGINGTIME",
        id3v23: "",
        id3v24: "TDTG",
        mp4: "",
        matroska: "T=30 DATE_TAGGED",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TITLE",
        id3v23: "TIT2",
        id3v24: "TIT2",
        mp4: "©nam",
        matroska: "MKA: T=30 MKV: T=50",
        asf: "Title",
        riff: "INAM",
    },
    FieldMapping {
        field: "TITLESORT",
        id3v23: "TSOT",
        id3v24: "TSOT",
        mp4: "sonm",
        matroska: "MKA: T=30 SORT_WITH MKV: T=50 SORT_WITH",
        asf: "WM/TitleSortOrder",
        riff: "",
    },
    FieldMapping {
        field: "TRACK",
        id3v23: "TRCK",
        id3v24: "TRCK",
        mp4: "trkn",
        matroska: "T=30 PART_NUMBER",
        asf: "WM/TrackNumber",
        riff: "ITRK",
    },
    FieldMapping {
        field: "UNIQUEFILEID",
        id3v23: "UFID",
        id3v24: "UFID",
        mp4: "",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "UNSYNCEDLYRICS",
        id3v23: "USLT",
        id3v24: "USLT",
        mp4: "©lyr",
        matroska: "T=30 LYRICS",
        asf: "WM/Lyrics",
        riff: "",
    },
    FieldMapping {
        field: "WWW",
        id3v23: "WXXX",
        id3v24: "WXXX",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "WWWARTIST",
        id3v23: "WOAR",
        id3v24: "WOAR",
        mp4: "",
        matroska: "T=30",
        asf: "WM/AuthorURL",
        riff: "",
    },
    FieldMapping {
        field: "WWWAUDIOFILE",
        id3v23: "WOAF",
        id3v24: "WOAF",
        mp4: "",
        matroska: "T=30",
        asf: "WM/AudioFileURL",
        riff: "",
    },
    FieldMapping {
        field: "WWWAUDIOSOURCE",
        id3v23: "WOAS",
        id3v24: "WOAS",
        mp4: "",
        matroska: "T=30",
        asf: "WM/AudioSourceURL",
        riff: "",
    },
    FieldMapping {
        field: "WWWCOMMERCIALINFO",
        id3v23: "WCOM",
        id3v24: "WCOM",
        mp4: "",
        matroska: "MKA: T=30 PURCHASE_INFO MKV: T=50 PURCHASE_INFO",
        asf: "WM/PromotionURL",
        riff: "",
    },
    FieldMapping {
        field: "WWWCOPYRIGHT",
        id3v23: "WCOP",
        id3v24: "WCOP",
        mp4: "",
        matroska: "T=30",
        asf: "CopyrightURL",
        riff: "",
    },
    FieldMapping {
        field: "WWWPAYMENT",
        id3v23: "WPAY",
        id3v24: "WPAY",
        mp4: "",
        matroska: "MKA: T=30 PURCHASE_ITEM MKV: T=50 PURCHASE_ITEM",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "WWWPUBLISHER",
        id3v23: "WPUB",
        id3v24: "WPUB",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "WWWRADIOPAGE",
        id3v23: "WORS",
        id3v24: "WORS",
        mp4: "",
        matroska: "T=30",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "YEAR",
        id3v23: "TYER",
        id3v24: "TDRC",
        mp4: "©day",
        matroska: "T=50 DATE_RECORDED",
        asf: "WM/Year",
        riff: "ICRD",
    },
    FieldMapping {
        field: "DIRECTOR",
        id3v23: "",
        id3v24: "",
        mp4: "©dir",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESACCOUNT",
        id3v23: "",
        id3v24: "",
        mp4: "apID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESADVISORY",
        id3v23: "",
        id3v24: "",
        mp4: "rtng",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESALBUMID",
        id3v23: "",
        id3v24: "",
        mp4: "plID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESARTISTID",
        id3v23: "",
        id3v24: "",
        mp4: "atID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESCATALOGID",
        id3v23: "",
        id3v24: "",
        mp4: "cnID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESCOMPOSERID",
        id3v23: "",
        id3v24: "",
        mp4: "cmID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESCOUNTRYID",
        id3v23: "",
        id3v24: "",
        mp4: "sfID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESGAPLESS",
        id3v23: "",
        id3v24: "",
        mp4: "pgap",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESGENREID",
        id3v23: "",
        id3v24: "",
        mp4: "geID",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESHDVIDEO",
        id3v23: "",
        id3v24: "",
        mp4: "hdvd",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESMEDIATYPE",
        id3v23: "",
        id3v24: "",
        mp4: "stik",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESOWNER",
        id3v23: "",
        id3v24: "",
        mp4: "ownr",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "ITUNESPURCHASEDATE",
        id3v23: "",
        id3v24: "",
        mp4: "purd",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "RATE",
        id3v23: "",
        id3v24: "",
        mp4: "rate",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "SHOWMOVEMENT",
        id3v23: "",
        id3v24: "",
        mp4: "shwm",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "STOREDESCRIPTION",
        id3v23: "",
        id3v24: "",
        mp4: "sdes",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TOTALDISCS",
        id3v23: "",
        id3v24: "",
        mp4: "disk",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TOTALTRACKS",
        id3v23: "",
        id3v24: "",
        mp4: "trkn",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TVEPISODE",
        id3v23: "",
        id3v24: "",
        mp4: "tves",
        matroska: "T=50 PART_NUMBER",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TVEPISODEID",
        id3v23: "",
        id3v24: "",
        mp4: "tven",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TVNETWORK",
        id3v23: "",
        id3v24: "",
        mp4: "tvnn",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TVSEASON",
        id3v23: "",
        id3v24: "",
        mp4: "tvsn",
        matroska: "T=60 PART_NUMBER",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TVSHOW",
        id3v23: "",
        id3v24: "",
        mp4: "tvsh",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "TVSHOWSORT",
        id3v23: "",
        id3v24: "",
        mp4: "sosn",
        matroska: "",
        asf: "",
        riff: "",
    },
    FieldMapping {
        field: "WORK",
        id3v23: "",
        id3v24: "",
        mp4: "©wrk",
        matroska: "",
        asf: "",
        riff: "",
    },
];
pub fn field_mapping(name: &str) -> Option<&'static FieldMapping> {
    FIELD_MAPPINGS
        .iter()
        .find(|r| r.field.eq_ignore_ascii_case(name))
}
fn id3_name(id: &str, version: Id3v2Version) -> Option<&'static str> {
    FIELD_MAPPINGS
        .iter()
        .find(|r| {
            let native = if version == Id3v2Version::V3 {
                r.id3v23
            } else {
                r.id3v24
            };
            !native.is_empty() && native.eq_ignore_ascii_case(id)
        })
        .map(|r| r.field)
}
fn mp4_name(ident: &AtomIdent<'_>) -> Option<&'static str> {
    let name = match ident {
        AtomIdent::Fourcc(v) => v.iter().map(|v| char::from(*v)).collect::<String>(),
        AtomIdent::Freeform { mean, name } => format!("----:{mean}:{name}"),
    };
    FIELD_MAPPINGS
        .iter()
        .find(|r| {
            r.mp4
                .split(" | ")
                .any(|n| !n.is_empty() && n.eq_ignore_ascii_case(&name))
        })
        .map(|r| r.field)
}
fn append(fields: &mut Fields, key: String, value: String) {
    fields.entry(key).or_default().push(value);
}

pub(super) fn id3_fields(tag: &Id3v2Tag) -> Fields {
    id3_fields_with(tag, tag.original_version(), false)
}
/// `popularimeter` shows every POPM frame as POPULARIMETER (Options → Tags →
/// Advanced) instead of the RATING MM/RATING WMP convenience fields.
pub(super) fn id3_fields_with(
    tag: &Id3v2Tag,
    version: Id3v2Version,
    popularimeter: bool,
) -> Fields {
    let mut fields = Fields::new();
    for frame in tag {
        let id = frame.id().as_str().to_owned();
        let ordinary = id3_name(&id, version)
            .or_else(|| id3_name(&id, Id3v2Version::V4))
            .unwrap_or(&id)
            .to_owned();
        match frame {
            Frame::Binary(binary) if id == "IPLS" => {
                // Keep implicit conversions disabled so TYER/TDAT remain distinct.
                // Under that setting Lofty leaves the old IPLS identifier opaque.
                if let Ok(Some(credits)) = KeyValueFrame::parse(
                    &mut binary.data.as_ref(),
                    FrameId::new("TIPL").expect("fixed frame identifier"),
                    binary.flags(),
                    version,
                ) {
                    append(
                        &mut fields,
                        "INVOLVEDPEOPLE".into(),
                        credits
                            .key_value_pairs
                            .iter()
                            .map(|(role, person)| format!("{role}:{person};"))
                            .collect(),
                    );
                }
            }
            Frame::Text(_) if id == "TCON" => {
                if let Some(genres) = tag.genres() {
                    for genre in genres {
                        append(&mut fields, "GENRE".into(), genre.into());
                    }
                }
            }
            Frame::Text(text) if id == "MVIN" => {
                let (number, total) = text
                    .value
                    .split_once('/')
                    .map(|(n, t)| (n, Some(t)))
                    .unwrap_or((&text.value, None));
                append(&mut fields, "MOVEMENT".into(), number.into());
                if let Some(total) = total {
                    append(&mut fields, "MOVEMENTTOTAL".into(), total.into());
                }
            }
            Frame::Text(text) => {
                for value in text.value.split('\0') {
                    append(&mut fields, ordinary.clone(), value.into());
                }
            }
            Frame::Timestamp(text) => append(&mut fields, ordinary, text.timestamp.to_string()),
            Frame::UserText(text) => {
                let id = format!("TXXX:{}", text.description);
                let name = id3_name(&id, version)
                    .unwrap_or(&text.description)
                    .to_owned();
                for value in text.content.split('\0') {
                    append(&mut fields, name.clone(), value.into());
                }
            }
            Frame::Url(url) => append(&mut fields, ordinary, url.url().into()),
            Frame::UserUrl(url) => append(&mut fields, "WWW".into(), url.content.to_string()),
            Frame::Comment(text) => append(&mut fields, "COMMENT".into(), text.content.to_string()),
            Frame::UnsynchronizedText(text) => append(
                &mut fields,
                "UNSYNCEDLYRICS".into(),
                format!(
                    "{}||{}",
                    String::from_utf8_lossy(&text.language),
                    text.content
                ),
            ),
            Frame::KeyValue(text) => append(
                &mut fields,
                ordinary,
                text.key_value_pairs
                    .iter()
                    .map(|(role, name)| format!("{role}:{name};"))
                    .collect(),
            ),
            Frame::UniqueFileIdentifier(uid) if uid.owner == "http://musicbrainz.org" => {
                if let Ok(value) = std::str::from_utf8(&uid.identifier) {
                    append(&mut fields, "MUSICBRAINZ_TRACKID".into(), value.into());
                }
            }
            Frame::Binary(value) if id == "PCST" && value.data.len() <= 8 => {
                append(
                    &mut fields,
                    "PODCAST".into(),
                    value
                        .data
                        .iter()
                        .fold(0u64, |v, b| (v << 8) | u64::from(*b))
                        .to_string(),
                );
            }
            Frame::Popularimeter(popm) if popularimeter => append(
                &mut fields,
                "POPULARIMETER".into(),
                format!("{}|{}|{}", popm.email, popm.rating, popm.counter),
            ),
            Frame::Popularimeter(popm) => {
                if popm.email == "no@email" {
                    if let Some(rating) = mm_rating(popm.rating) {
                        append(&mut fields, "RATING MM".into(), rating);
                    } else {
                        append(
                            &mut fields,
                            "POPULARIMETER".into(),
                            format!("{}|{}|{}", popm.email, popm.rating, popm.counter),
                        );
                    }
                } else if popm.email == "Windows Media Player 9 Series" {
                    append(
                        &mut fields,
                        "RATING WMP".into(),
                        wmp_rating(popm.rating).to_string(),
                    );
                } else {
                    append(
                        &mut fields,
                        "POPULARIMETER".into(),
                        format!("{}|{}|{}", popm.email, popm.rating, popm.counter),
                    );
                }
            }
            _ => {}
        }
    }
    fields
}

fn atom_name(id: &AtomIdent<'_>) -> String {
    match id {
        AtomIdent::Fourcc(id) => id.iter().map(|c| char::from(*c)).collect(),
        AtomIdent::Freeform { mean, name } if mean.as_ref() == "com.apple.iTunes" => {
            name.to_string()
        }
        AtomIdent::Freeform { mean, name } => format!("----:{mean}:{name}"),
    }
}
fn numeric_atom(field: &str) -> bool {
    matches!(
        field,
        "BPM"
            | "MOVEMENT"
            | "MOVEMENTTOTAL"
            | "ITUNESADVISORY"
            | "ITUNESALBUMID"
            | "ITUNESARTISTID"
            | "ITUNESCATALOGID"
            | "ITUNESCOMPOSERID"
            | "ITUNESCOUNTRYID"
            | "ITUNESGENREID"
            | "ITUNESHDVIDEO"
            | "ITUNESMEDIATYPE"
            | "RATE"
            | "TVEPISODE"
            | "TVSEASON"
    )
}
pub(super) fn mp4_fields(tag: &Ilst) -> Fields {
    let mut fields = Fields::new();
    for atom in tag {
        let name = mp4_name(atom.ident())
            .map(str::to_owned)
            .unwrap_or_else(|| atom_name(atom.ident()));
        for data in atom.data() {
            if matches!(atom.ident(),AtomIdent::Fourcc(v) if v==b"trkn"||v==b"disk") {
                if let AtomData::Unknown { data, .. } = data {
                    if data.len() >= 6 {
                        let number = u16::from_be_bytes([data[2], data[3]]);
                        let total = u16::from_be_bytes([data[4], data[5]]);
                        append(
                            &mut fields,
                            if name == "TRACK" {
                                "TRACK"
                            } else {
                                "DISCNUMBER"
                            }
                            .into(),
                            if total > 0 {
                                format!("{number}/{total}")
                            } else {
                                number.to_string()
                            },
                        );
                        if total > 0 {
                            append(
                                &mut fields,
                                if name == "TRACK" {
                                    "TOTALTRACKS"
                                } else {
                                    "TOTALDISCS"
                                }
                                .into(),
                                total.to_string(),
                            );
                        }
                    }
                }
                continue;
            }
            let value = match data {
                AtomData::UTF8(v) | AtomData::UTF16(v) => Some(v.clone()),
                AtomData::Bool(v) => Some(u8::from(*v).to_string()),
                AtomData::SignedInteger(v) => Some(v.to_string()),
                AtomData::UnsignedInteger(v) => Some(v.to_string()),
                AtomData::Unknown { data, .. }
                    if numeric_atom(&name) && !data.is_empty() && data.len() <= 8 =>
                {
                    Some(
                        data.iter()
                            .fold(0u64, |v, b| (v << 8) | u64::from(*b))
                            .to_string(),
                    )
                }
                _ => None,
            };
            if let Some(value) = value {
                let value = if name == "ITUNESMEDIATYPE" {
                    value
                        .parse::<u8>()
                        .ok()
                        .and_then(|code| {
                            MEDIA_TYPES
                                .iter()
                                .find(|(_, v)| *v == code)
                                .map(|(name, _)| name.to_string())
                        })
                        .unwrap_or(value)
                } else {
                    value
                };
                append(&mut fields, name.clone(), value);
            }
        }
    }
    fields
}

/// ID3v2 revision, text encoding and comment language used for a write.
#[derive(Debug, Clone, Copy)]
pub(super) struct Id3Target {
    pub version: Id3v2Version,
    pub latin1: bool,
    pub language: [u8; 3],
}
impl Id3Target {
    /// Keep the revision of an existing tag (v2.2 tags are upgraded to v2.4).
    pub(super) fn preserving(tag: &Id3v2Tag, language: [u8; 3]) -> Self {
        Self {
            version: if tag.original_version() == Id3v2Version::V3 {
                Id3v2Version::V3
            } else {
                Id3v2Version::V4
            },
            latin1: false,
            language,
        }
    }
    fn encoding(&self) -> TextEncoding {
        if self.latin1 {
            TextEncoding::Latin1
        } else if self.version == Id3v2Version::V3 {
            TextEncoding::UTF16
        } else {
            TextEncoding::UTF8
        }
    }
}

pub(super) fn save_id3(
    file: &mut std::fs::File,
    kind: lofty::file::FileType,
    tag: &Id3v2Tag,
    target: &Id3Target,
) -> Result<(), String> {
    if target.version == Id3v2Version::V3 {
        id3_v23::save(file, kind, tag, target.latin1)
    } else {
        tag.save_to(file, WriteOptions::default().use_id3v23(false))
            .map_err(|e| e.to_string())
    }
}

/// Serialize a complete ID3v2 tag for containers that embed it as an opaque
/// block (DSF, DSDIFF). An empty result means the tag has no frames.
pub(super) fn render_id3(tag: &Id3v2Tag, target: &Id3Target) -> Result<Vec<u8>, String> {
    if target.version == Id3v2Version::V3 {
        return id3_v23::serialize(tag, target.latin1);
    }
    let mut bytes = Vec::new();
    if !tag.is_empty() {
        tag.dump_to(&mut bytes, WriteOptions::default().use_id3v23(false))
            .map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}

/// Characters an ISO-8859-1 tag cannot hold are written as '?'.
pub(super) fn latin1_lossy(value: &str) -> String {
    value
        .chars()
        .map(|c| if u32::from(c) <= 0xff { c } else { '?' })
        .collect()
}

fn frame_key(frame: &Frame<'_>) -> String {
    match frame {
        Frame::UserText(text) => format!("TXXX:{}", text.description),
        _ => frame.id().as_str().to_owned(),
    }
}
fn version_native(field: &str, version: Id3v2Version) -> String {
    field_mapping(field)
        .map(|r| {
            if version == Id3v2Version::V3 {
                r.id3v23
            } else {
                r.id3v24
            }
        })
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("TXXX:{field}"))
}

/// Re-express documented frames whose identifier differs between ID3v2.3 and
/// ID3v2.4 (for example TYER/TDAT/TIME and TDRC). Other frames are retained.
pub(super) fn convert_id3_version(tag: &mut Id3v2Tag, target: &Id3Target) -> Result<(), String> {
    let from = if tag.original_version() == Id3v2Version::V3 {
        Id3v2Version::V3
    } else {
        Id3v2Version::V4
    };
    let to = target.version;
    if from == to {
        return Ok(());
    }
    let text = |id: &str| {
        tag.get_text(&FrameId::new(id).expect("fixed frame identifier"))
            .map(str::to_owned)
    };
    let mut moved = Fields::new();
    let mut extra = Vec::new();
    let mut consumed: Vec<&str> = Vec::new();
    if from == Id3v2Version::V3 {
        // TYER + TDAT (DDMM) + TIME (HHMM) combine into one TDRC timestamp.
        if let Some(year) =
            text("TYER").filter(|y| y.len() == 4 && y.bytes().all(|b| b.is_ascii_digit()))
        {
            let mut stamp = year;
            consumed.push("TYER");
            if let Some(date) =
                text("TDAT").filter(|d| d.len() == 4 && d.bytes().all(|b| b.is_ascii_digit()))
            {
                stamp = format!("{stamp}-{}-{}", &date[2..], &date[..2]);
                consumed.push("TDAT");
                if let Some(time) =
                    text("TIME").filter(|t| t.len() == 4 && t.bytes().all(|b| b.is_ascii_digit()))
                {
                    stamp = format!("{stamp}T{}:{}", &time[..2], &time[2..]);
                    consumed.push("TIME");
                }
            }
            moved.insert("YEAR".into(), vec![stamp]);
        }
    } else if let Some(Frame::Timestamp(frame)) =
        tag.get(&FrameId::new("TDRC").expect("fixed frame identifier"))
    {
        // ID3v2.3 stores the recording time as separate year, date and time frames.
        let stamp = frame.timestamp;
        consumed.push("TDRC");
        moved.insert("YEAR".into(), vec![format!("{:04}", stamp.year)]);
        if let (Some(month), Some(day)) = (stamp.month, stamp.day) {
            extra.push(("TDAT", format!("{day:02}{month:02}")));
            if let (Some(hour), Some(minute)) = (stamp.hour, stamp.minute) {
                extra.push(("TIME", format!("{hour:02}{minute:02}")));
            }
        }
    }
    let mut remove = Vec::new();
    for frame in &*tag {
        let key = frame_key(frame);
        if consumed.iter().any(|id| *id == key) {
            remove.push(true);
            continue;
        }
        let convertible = matches!(
            frame,
            Frame::Text(_) | Frame::Timestamp(_) | Frame::UserText(_) | Frame::KeyValue(_)
        ) || (matches!(frame, Frame::Binary(_)) && key == "IPLS");
        // Documented fields without a native frame in one revision are stored
        // as TXXX:<FIELD> there (for example MOOD in ID3v2.3).
        let field = id3_name(&key, from)
            .or_else(|| match frame {
                Frame::UserText(text)
                    if version_native(&text.description, from).eq_ignore_ascii_case(&key) =>
                {
                    field_mapping(&text.description).map(|r| r.field)
                }
                _ => None,
            })
            .filter(|field| convertible && !version_native(field, to).eq_ignore_ascii_case(&key));
        if field.is_none() {
            remove.push(false);
            continue;
        }
        let mut single = Id3v2Tag::new();
        single.insert(frame.clone());
        for (name, values) in id3_fields_with(&single, from, true) {
            moved.entry(name).or_default().extend(values);
        }
        remove.push(true);
    }
    let mut index = 0;
    tag.retain(|_| {
        index += 1;
        !remove[index - 1]
    });
    patch_id3(tag, &moved, target)?;
    for (id, value) in extra {
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new(id).map_err(|e| e.to_string())?,
            target.encoding(),
            value,
        )));
    }
    // Data length indicators only describe compressed or encrypted v2.4 frames.
    let mut failure = None;
    tag.retain_mut(|frame| {
        let mut flags = frame.flags();
        if flags.data_length_indicator.is_some() {
            if flags.compression || flags.encryption.is_some() {
                failure = Some("Compressed or encrypted ID3v2.4 frames cannot be converted; original file preserved".to_owned());
            } else {
                flags.data_length_indicator = None;
                frame.set_flags(flags);
            }
        }
        true
    });
    failure.map_or(Ok(()), Err)
}

pub(super) fn patch_id3(
    tag: &mut Id3v2Tag,
    updates: &Fields,
    target: &Id3Target,
) -> Result<(), String> {
    let version = target.version;
    let encoding = target.encoding();
    if updates.contains_key("MOVEMENT") || updates.contains_key("MOVEMENTTOTAL") {
        let existing = id3_fields(tag);
        let n = updates
            .get("MOVEMENT")
            .or_else(|| existing.get("MOVEMENT"))
            .and_then(|v| v.first());
        let t = updates
            .get("MOVEMENTTOTAL")
            .or_else(|| existing.get("MOVEMENTTOTAL"))
            .and_then(|v| v.first());
        tag.retain(|f| f.id().as_str() != "MVIN");
        let text = match (n, t) {
            (Some(n), Some(t)) => Some(format!("{n}/{t}")),
            (Some(n), None) => Some(n.clone()),
            (None, Some(t)) => Some(format!("0/{t}")),
            _ => None,
        };
        if let Some(text) = text {
            tag.insert(Frame::Text(TextInformationFrame::new(
                FrameId::new("MVIN").map_err(|e| e.to_string())?,
                encoding,
                text,
            )));
        }
    }
    for (name, values) in updates {
        if matches!(name.as_str(), "MOVEMENT" | "MOVEMENTTOTAL") {
            continue;
        }
        let mapped = field_mapping(name)
            .map(|r| {
                if version == Id3v2Version::V3 {
                    r.id3v23
                } else {
                    r.id3v24
                }
            })
            .filter(|s| !s.is_empty());
        let mapping = mapped
            .map(str::to_owned)
            .unwrap_or_else(|| format!("TXXX:{name}"));
        if let Some(desc) = mapping.strip_prefix("TXXX:") {
            tag.retain(
                |f| !matches!(f,Frame::UserText(t) if t.description.eq_ignore_ascii_case(desc)),
            );
            if !values.is_empty() {
                tag.insert_user_text(desc.to_owned(), values.join("\0"));
            }
            continue;
        }
        // The reader also exposes TXXX descriptions as logical field names
        // (for example FFmpeg's TXXX:comment). Replace those aliases only when
        // their field is edited, along with its standard mapped frame.
        tag.retain(|frame| {
            !matches!(frame, Frame::UserText(text) if text.description.eq_ignore_ascii_case(name))
        });
        if name == "RATING MM" || name == "RATING WMP" {
            let email = if name == "RATING MM" {
                "no@email"
            } else {
                "Windows Media Player 9 Series"
            };
            let counter = (&*tag)
                .into_iter()
                .find_map(|f| {
                    if let Frame::Popularimeter(p) = f {
                        (p.email == email).then_some(p.counter)
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            tag.retain(|f| !matches!(f,Frame::Popularimeter(p) if p.email==email));
            if values.len() > 1 {
                return Err(format!("{name} supports one rating per owner"));
            }
            if let Some(value) = values.first() {
                let half = parse_stars(value, name == "RATING MM")?;
                let rating = if name == "RATING MM" {
                    MM_VALUES[usize::from(half)]
                } else {
                    WMP_VALUES[usize::from(half / 2)]
                };
                tag.insert(Frame::Popularimeter(PopularimeterFrame::new(
                    email, rating, counter,
                )));
            }
            continue;
        }
        if mapping.starts_with("UFID:") {
            let owner = mapping.trim_start_matches("UFID:");
            tag.retain(|f| !matches!(f,Frame::UniqueFileIdentifier(v) if v.owner==owner));
            if values.len() > 1 {
                return Err(format!("{name} supports one identifier per owner"));
            }
            if let Some(v) = values.first() {
                tag.insert(Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame::new(
                    owner.to_owned(),
                    v.as_bytes().to_vec(),
                )));
            }
            continue;
        }
        if mapping == "UFID" {
            return Err("UNIQUEFILEID serialization is not yet verified; existing UFID frames are preserved".into());
        }
        let internal_id = if mapping == "IPLS" {
            "TIPL"
        } else {
            mapping.as_str()
        };
        tag.retain(|f| f.id().as_str() != mapping && f.id().as_str() != internal_id);
        if values.is_empty() {
            continue;
        }
        let id = FrameId::new(internal_id.to_owned()).map_err(|e| e.to_string())?;
        match mapping.as_str() {
            "PCST" => {
                if values != &["1".to_owned()] {
                    return Err("PODCAST requires 1 or deletion".into());
                }
                tag.insert(Frame::Binary(BinaryFrame::new(id, vec![0, 0, 0, 1])));
            }
            "COMM" => {
                for (i, v) in values.iter().enumerate() {
                    tag.insert(Frame::Comment(CommentFrame::new(
                        encoding,
                        target.language,
                        if i == 0 {
                            String::new()
                        } else {
                            format!("Comment {i}")
                        },
                        v.clone(),
                    )));
                }
            }
            "USLT" => {
                for v in values {
                    let (lang, lyrics) = v
                        .split_once("||")
                        .ok_or("UNSYNCEDLYRICS requires language||lyrics")?;
                    let language: [u8; 3] = lang
                        .as_bytes()
                        .try_into()
                        .map_err(|_| "Lyrics language requires three ASCII letters")?;
                    tag.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
                        encoding,
                        language,
                        "",
                        lyrics.to_owned(),
                    )));
                }
            }
            "WXXX" => {
                if values.len() > 1 {
                    return Err("WWW needs one URL per description".into());
                }
                tag.insert(Frame::UserUrl(ExtendedUrlFrame::new(
                    TextEncoding::Latin1,
                    "",
                    values[0].clone(),
                )));
            }
            "POPM" => {
                for v in values {
                    let mut parts = v.split('|');
                    let email = parts
                        .next()
                        .ok_or("POPULARIMETER requires email|rating|counter")?;
                    let rating = parts
                        .next()
                        .ok_or("POPULARIMETER requires a rating")?
                        .parse::<u8>()
                        .map_err(|_| "POPM rating requires 0..255")?;
                    let counter = parts
                        .next()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("0")
                        .parse::<u64>()
                        .map_err(|_| "POPM counter requires an unsigned integer")?;
                    if parts.next().is_some() {
                        return Err("POPULARIMETER has too many components".into());
                    }
                    tag.insert(Frame::Popularimeter(PopularimeterFrame::new(
                        email.to_owned(),
                        rating,
                        counter,
                    )));
                }
            }
            "IPLS" | "TIPL" | "TMCL" => {
                let mut pairs = Vec::new();
                for v in values {
                    for pair in v.split(';').filter(|s| !s.is_empty()) {
                        let (role, person) = pair
                            .split_once(':')
                            .ok_or("Credits require role:person; pairs")?;
                        pairs.push((role.to_owned().into(), person.to_owned().into()));
                    }
                }
                tag.insert(Frame::KeyValue(KeyValueFrame::new(id, encoding, pairs)));
            }
            "TDEN" | "TDOR" | "TDRC" | "TDRL" | "TDTG" => {
                if values.len() != 1 {
                    return Err(format!("{name} requires one timestamp"));
                }
                let timestamp = Timestamp::parse(&mut values[0].as_bytes(), ParsingMode::Strict)
                    .map_err(|e| e.to_string())?
                    .ok_or("Empty timestamp")?;
                tag.insert(Frame::Timestamp(TimestampFrame::new(
                    id, encoding, timestamp,
                )));
            }
            field if field.starts_with('W') => {
                if values.len() > 1 {
                    return Err(format!("{name} supports one URL"));
                }
                tag.insert(Frame::Url(UrlLinkFrame::new(id, values[0].clone())));
            }
            _ => {
                if matches!(mapping.as_str(), "TCMP" | "PCST") && values != &["1".to_owned()] {
                    return Err(format!("{name} requires 1 or deletion"));
                }
                tag.insert(Frame::Text(TextInformationFrame::new(
                    id,
                    encoding,
                    values.join("\0"),
                )));
            }
        }
    }
    Ok(())
}

const MM_VALUES: [u8; 11] = [0, 13, 1, 54, 64, 118, 128, 186, 196, 242, 255];
const WMP_VALUES: [u8; 6] = [0, 1, 64, 128, 196, 255];
fn mm_rating(value: u8) -> Option<String> {
    MM_VALUES.iter().position(|v| *v == value).map(|i| {
        if i % 2 == 0 {
            (i / 2).to_string()
        } else {
            format!("{}.5", i / 2)
        }
    })
}
fn wmp_rating(value: u8) -> u8 {
    match value {
        0 => 0,
        1..=31 => 1,
        32..=95 => 2,
        96..=159 => 3,
        160..=223 => 4,
        _ => 5,
    }
}
fn parse_stars(value: &str, half_allowed: bool) -> Result<u8, String> {
    let half = if value.chars().all(|c| c == '*' || c == '-') {
        (value.chars().filter(|c| *c == '*').count() * 2
            + value.chars().filter(|c| *c == '-').count()) as f64
    } else {
        value
            .parse::<f64>()
            .map_err(|_| "Rating requires a number or stars")?
            * 2.0
    };
    if !half.is_finite()
        || !(0.0..=10.0).contains(&half)
        || half.fract() != 0.0
        || (!half_allowed && half % 2.0 != 0.0)
    {
        return Err("Rating is outside the documented range".into());
    }
    Ok(half as u8)
}

pub(super) fn expected_id3_readback(
    name: &str,
    values: &[String],
) -> Result<(String, Vec<String>), String> {
    if name == "GENRE" {
        let mut tag = Id3v2Tag::new();
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new("TCON").map_err(|e| e.to_string())?,
            TextEncoding::UTF8,
            values.join("\0"),
        )));
        return Ok((
            name.into(),
            tag.genres()
                .map(|genres| genres.map(str::to_owned).collect())
                .unwrap_or_default(),
        ));
    }
    if matches!(name, "RATING MM" | "RATING WMP") {
        let output = values
            .iter()
            .map(|value| {
                parse_stars(value, name == "RATING MM").map(|n| {
                    if n % 2 == 0 {
                        (n / 2).to_string()
                    } else {
                        format!("{}.5", n / 2)
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok((name.into(), output));
    }
    if name == "POPULARIMETER" && values.len() == 1 {
        let mut parts = values[0].split('|');
        let email = parts.next().unwrap_or_default();
        let rating = parts
            .next()
            .unwrap_or_default()
            .parse::<u8>()
            .map_err(|_| "POPM rating requires 0..255")?;
        let counter = parts
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|_| "POPM counter requires an unsigned integer")?;
        if email == "no@email" {
            if let Some(rating) = mm_rating(rating) {
                return Ok(("RATING MM".into(), vec![rating]));
            }
        }
        if email == "Windows Media Player 9 Series" {
            return Ok(("RATING WMP".into(), vec![wmp_rating(rating).to_string()]));
        }
        return Ok((name.into(), vec![format!("{email}|{rating}|{counter}")]));
    }
    Ok((name.into(), values.to_vec()))
}

fn mp4_ident(name: &str) -> Result<AtomIdent<'static>, String> {
    let mapped = field_mapping(name)
        .map(|r| r.mp4)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let mapped = if name == "ARTIST" {
        "©ART"
    } else {
        mapped.split(" | ").next().unwrap_or("")
    };
    let id = if !mapped.is_empty() {
        mapped.to_owned()
    } else if name.starts_with("----:") {
        name.to_owned()
    } else {
        format!("----:com.apple.iTunes:{name}")
    };
    if let Some(rest) = id.strip_prefix("----:") {
        let (mean, name) = rest
            .split_once(':')
            .ok_or("MP4 freeform requires mean:name")?;
        Ok(AtomIdent::Freeform {
            mean: mean.to_owned().into(),
            name: name.to_owned().into(),
        })
    } else {
        let chars = id
            .chars()
            .map(|c| u8::try_from(u32::from(c)).map_err(|_| "MP4 fourcc must be Latin-1"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AtomIdent::Fourcc(
            chars
                .try_into()
                .map_err(|_| "MP4 fourcc requires four characters")?,
        ))
    }
}
fn one<'a>(field: &str, values: &'a [String]) -> Result<&'a str, String> {
    if values.len() != 1 {
        return Err(format!("{field} requires one value"));
    }
    Ok(&values[0])
}
pub(super) fn patch_mp4(tag: &mut Ilst, updates: &Fields) -> Result<(), String> {
    let existing = mp4_fields(tag);
    for (number, total, id) in [
        ("TRACK", "TOTALTRACKS", *b"trkn"),
        ("DISCNUMBER", "TOTALDISCS", *b"disk"),
    ] {
        if !updates.contains_key(number) && !updates.contains_key(total) {
            continue;
        }
        let mut n = existing
            .get(number)
            .and_then(|v| v.first())
            .map(|v| v.split('/').next().unwrap_or_default().to_owned());
        let mut t = existing.get(total).and_then(|v| v.first()).cloned();
        if let Some(values) = updates.get(number) {
            n = values.first().cloned();
            if let Some(value) = n.as_ref() {
                if let Some((first, last)) = value.split_once('/') {
                    let first = first.to_owned();
                    let last = last.to_owned();
                    if let Some(explicit) = updates.get(total).and_then(|v| v.first()) {
                        if explicit.parse::<u16>().ok() != last.parse::<u16>().ok() {
                            return Err(format!("Conflicting {total}"));
                        }
                    }
                    n = Some(first);
                    t = Some(last);
                }
            }
        }
        if let Some(values) = updates.get(total) {
            t = values.first().cloned();
        }
        tag.remove(&AtomIdent::Fourcc(id)).for_each(drop);
        if n.is_some() || t.is_some() {
            let n = n
                .map(|n| {
                    n.parse::<u16>()
                        .map_err(|_| format!("{number} requires a 16-bit unsigned number"))
                })
                .transpose()?
                .unwrap_or(0);
            let t = t
                .map(|t| {
                    t.parse::<u16>()
                        .map_err(|_| format!("{total} requires a 16-bit unsigned number"))
                })
                .transpose()?
                .unwrap_or(0);
            let n = n.to_be_bytes();
            let t = t.to_be_bytes();
            tag.insert(Atom::new(
                AtomIdent::Fourcc(id),
                AtomData::Unknown {
                    code: DataType::Reserved,
                    data: vec![0, 0, n[0], n[1], t[0], t[1], 0, 0],
                },
            ));
        }
    }
    for (name, values) in updates {
        if matches!(
            name.as_str(),
            "TRACK" | "TOTALTRACKS" | "DISCNUMBER" | "TOTALDISCS"
        ) {
            continue;
        }
        let id = mp4_ident(name)?;
        tag.remove(&id).for_each(drop);
        if values.is_empty() {
            continue;
        }
        let data = if matches!(
            name.as_str(),
            "COMPILATION" | "PODCAST" | "ITUNESGAPLESS" | "SHOWMOVEMENT"
        ) {
            if one(name, values)? != "1" {
                return Err(format!("{name} requires 1 or deletion"));
            }
            vec![AtomData::Bool(true)]
        } else if name == "ITUNESHDVIDEO" {
            let v = one(name, values)?;
            if !matches!(v, "0" | "1") {
                return Err("ITUNESHDVIDEO requires 0 or 1".into());
            }
            vec![AtomData::Bool(v == "1")]
        } else if name == "ITUNESMEDIATYPE" {
            let value = one(name, values)?;
            let code = MEDIA_TYPES
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(value))
                .map(|(_, code)| *code)
                .ok_or("ITUNESMEDIATYPE requires a documented media type name")?;
            vec![AtomData::SignedInteger(i32::from(code))]
        } else if name == "ITUNESALBUMID" {
            let v = one(name, values)?
                .parse::<u64>()
                .map_err(|_| "ITUNESALBUMID requires an unsigned 64-bit integer")?;
            vec![AtomData::Unknown {
                code: DataType::Be64BitUnsignedInteger,
                data: v.to_be_bytes().to_vec(),
            }]
        } else if numeric_atom(name) {
            let v = one(name, values)?
                .parse::<u32>()
                .map_err(|_| format!("{name} requires an unsigned integer"))?;
            if name == "ITUNESADVISORY" && v > 2 {
                return Err("ITUNESADVISORY requires 0, 1, or 2".into());
            }
            vec![AtomData::UnsignedInteger(v)]
        } else {
            values.iter().cloned().map(AtomData::UTF8).collect()
        };
        if let Some(atom) = Atom::from_collection(id, data) {
            tag.insert(atom);
        }
    }
    Ok(())
}

// Numeric kinds are independently documented by ExifTool's native QuickTime
// registry; display names follow Mp3tag's published ITUNESMEDIATYPE vocabulary.
const MEDIA_TYPES: [(&str, u8); 10] = [
    ("Normal", 1),
    ("Audiobook", 2),
    ("Bookmark", 5),
    ("Music Video", 6),
    ("Movie", 9),
    ("TV Show", 10),
    ("Booklet", 11),
    ("Ringtone", 14),
    ("Podcast", 21),
    ("iTunes U", 23),
];
