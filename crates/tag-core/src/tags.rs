//! Portable metadata I/O. Every mutation is written and verified in a sibling
//! temporary file before the original is atomically replaced.
#[path = "asf/mod.rs"]
mod asf;
#[path = "container_formats.rs"]
mod container_formats;
#[path = "cover_manage.rs"]
mod cover_manage;
#[path = "cover_ops.rs"]
mod cover_ops;
#[path = "mapping.rs"]
mod mapping;
#[path = "media_info.rs"]
mod media_info;
#[path = "mp4_chapters.rs"]
mod mp4_chapters;
#[path = "mp4_layout.rs"]
mod mp4_layout;
#[path = "nero_tags.rs"]
mod nero_tags;
#[path = "optimize.rs"]
mod optimize;
#[path = "subsongs.rs"]
mod subsongs;
#[path = "tag_policy.rs"]
mod tag_policy;
#[path = "tag_types.rs"]
mod tag_types;
#[path = "technical.rs"]
mod technical;
use crate::batch_errors::ErrorCause;
use crate::{id3_frames::ReadSeek, preferences::TagOptions};
use base64::{engine::general_purpose::STANDARD, Engine};
pub use cover_manage::*;
pub use cover_ops::*;
use lofty::{
    aac::AacFile,
    ape::{ApeFile, ApeItem, ApeTag},
    config::{ParseOptions, ParsingMode, WriteOptions},
    file::{AudioFile, FileType, TaggedFile, TaggedFileExt},
    flac::FlacFile,
    id3::v2::{Frame, Id3v2Tag},
    iff::{
        aiff::AiffFile,
        wav::{RiffInfoList, WavFile},
    },
    mp4::{Ilst, Mp4File},
    mpeg::MpegFile,
    musepack::MpcFile,
    ogg::{tag::VorbisComments, OggPictureStorage, OpusFile, SpeexFile, VorbisFile},
    picture::{Picture, PictureType},
    probe::Probe,
    tag::{ItemKey, ItemValue, Tag, TagExt, TagItem, TagType},
    wavpack::WavPackFile,
};
pub use mapping::{
    field_mapping, map_read_fields, map_read_fields_diagnostic, map_write_fields,
    map_write_fields_diagnostic, validate_user_mappings, validate_user_mappings_diagnostic,
    FieldMapping, UserFieldMapping, FIELD_MAPPINGS, LYRICS_LANGUAGE_REQUIRED,
};
pub use media_info::{audio_digest, AudioDigest};
pub use optimize::*;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
    fs,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};
pub use subsongs::{
    chapter_rows, expand_chapter_rows, subsong_file, subsong_parts, subsong_path, ChapterInfo,
    SUBSONG_SEPARATOR,
};
pub use tag_types::*;

/// `%_length%`: `m:ss`, or `h:mm:ss` from one hour on, of whole seconds
/// rounded like `%_length_seconds%`.
pub fn format_length(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}
use walkdir::WalkDir;

pub(crate) fn error_message(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}
pub type Fields = BTreeMap<String, Vec<String>>;

/// `%_tag%` and `%_tag_read%` name of an APE tag, as Mp3tag spells it.
pub const APE_TAG_NAME: &str = "APEv2";
/// `%_tag%` and `%_tag_read%` name of a Vorbis comment block.
pub const VORBIS_TAG_NAME: &str = "VorbisComment";

/// Whether a tag kind, named as `%_tag_read%` shows it, stores field names
/// verbatim, so `_FIELDNAME` actions can change them (APEv2, VorbisComment).
pub fn keeps_field_names(tag: &str) -> bool {
    let tag = tag.trim();
    [
        APE_TAG_NAME,
        VORBIS_TAG_NAME,
        "APE",
        "APEv1",
        "VorbisComments",
    ]
    .iter()
    .any(|name| tag.eq_ignore_ascii_case(name))
}
/// Derive available file timestamps using the same locale format as audio reads.
/// Provider-backed callers can omit unavailable creation times.
pub fn file_time_fields(
    created: Option<std::time::SystemTime>,
    modified: Option<std::time::SystemTime>,
) -> Fields {
    technical::file_time_fields(created, modified)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverInfo {
    pub index: usize,
    pub mime: String,
    pub description: String,
    pub picture_type: String,
    pub data_base64: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    /// Content identity of the image bytes, used to aggregate covers.
    #[serde(default)]
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEntry {
    pub path: String,
    pub filename: String,
    pub format: String,
    pub duration: f64,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub size: u64,
    pub fields: Fields,
    pub covers: Vec<CoverInfo>,
    #[serde(default)]
    pub technical: Fields,
    /// Chapters (MP4, Matroska, ID3v2, embedded cue sheet), listed as rows
    /// of their own with Options → Tags → Advanced → List chapters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<ChapterInfo>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileError {
    pub path: String,
    pub error: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanResult {
    pub entries: Vec<AudioEntry>,
    pub errors: Vec<FileError>,
}

/// Lofty's strict mode rejects the empty ID3v1 year ("\0\0\0\0") that common
/// taggers, lofty included, write. Only that failure is retried leniently.
fn empty_id3v1_year(message: &str) -> bool {
    message.contains("Id3v1") && message.contains("4 digit year")
}
fn strict_or_lenient_id3v1_diagnostic<T>(
    mut read: impl FnMut(ParsingMode) -> Result<T, ErrorCause>,
) -> Result<T, ErrorCause> {
    match read(ParsingMode::Strict) {
        Err(error) if empty_id3v1_year(&error.to_string()) => read(ParsingMode::Relaxed),
        result => result,
    }
}

/// Read an MP4 file. Items whose integer data lofty's strict reader rejects
/// (see [`crate::mp4_metadata::legacy_integer_items`]) are read leniently and
/// kept verbatim; any other malformed item still fails the read. A `plID`
/// lofty drops is restored (see [`crate::mp4_metadata::odd_album_id`]).
fn read_mp4_diagnostic<R: std::io::Read + Seek>(
    reader: &mut R,
    mut options: ParseOptions,
) -> Result<Mp4File, ErrorCause> {
    let start = reader.stream_position().map_err(ErrorCause::external)?;
    let album = crate::mp4_metadata::odd_album_id(reader).map_err(ErrorCause::external)?;
    let album_ident = lofty::mp4::AtomIdent::Fourcc(*b"plID");
    reader
        .seek(SeekFrom::Start(start))
        .map_err(ErrorCause::external)?;
    let mut file = match Mp4File::read_from(reader, options) {
        Ok(file) => file,
        Err(error) => {
            let error = error_message(&error);
            if !error.contains("unexpected atom size for type \"BE") {
                return Err(ErrorCause::external(error));
            }
            reader
                .seek(SeekFrom::Start(start))
                .map_err(ErrorCause::external)?;
            let (legacy, items) =
                crate::mp4_metadata::legacy_integer_items(reader).map_err(ErrorCause::external)?;
            if legacy.is_empty() {
                return Err(ErrorCause::external(error));
            }
            reader
                .seek(SeekFrom::Start(start))
                .map_err(ErrorCause::external)?;
            let mut file = Mp4File::read_from(reader, options.parsing_mode(ParsingMode::Relaxed))
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            let mut ilst = file.ilst().cloned().unwrap_or_default();
            // Only the items the lenient reader dropped are restored; lofty
            // itself keeps an 8-byte `plID`, and an odd one is restored below.
            let dropped: Vec<_> = legacy
                .into_iter()
                .filter(|atom| ilst.get(atom.ident()).is_none())
                .filter(|atom| album.is_none() || atom.ident() != &album_ident)
                .collect();
            let album_dropped = album.is_some() && ilst.get(&album_ident).is_none();
            if ilst.len() + dropped.len() + usize::from(album_dropped) != items {
                return Err(ErrorCause::external(error));
            }
            for atom in dropped {
                ilst.insert(atom);
            }
            file.set_ilst(ilst);
            file
        }
    };
    if let Some(album) = album {
        let mut ilst = file.ilst().cloned().unwrap_or_default();
        ilst.remove(&album_ident).for_each(drop);
        ilst.insert(album);
        file.set_ilst(ilst);
    }
    Ok(file)
}

fn load(path: &Path) -> Result<TaggedFile, String> {
    load_diagnostic(path).map_err(|error| error.to_string())
}

fn load_diagnostic(path: &Path) -> Result<TaggedFile, ErrorCause> {
    if subsongs::split_path(path).is_some() {
        return Err(ErrorCause::owned(subsongs::ROW_OPERATION));
    }
    if let Some(stream) = container_formats::detect_stream(path) {
        return container_formats::raw_streams::load_diagnostic(path, stream);
    }
    let mut source = fs::File::open(path).map_err(ErrorCause::external)?;
    let mut file = strict_or_lenient_id3v1_diagnostic(|mode| {
        crate::id3_frames::read_with_fallback_diagnostic(&mut source, |reader| {
            reader
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            let probe = Probe::new(std::io::BufReader::new(reader))
                .guess_file_type()
                .map_err(ErrorCause::external)?;
            let options = ParseOptions::new()
                .implicit_conversions(false)
                .parsing_mode(mode);
            if probe.file_type() == Some(FileType::Mp4) {
                return read_mp4_diagnostic(&mut probe.into_inner(), options).map(TaggedFile::from);
            }
            probe
                .options(options)
                .read()
                .map_err(|e| ErrorCause::external(error_message(&e)))
        })
        .map(|(file, _, phantom)| without_phantom(file, phantom))
    })?;
    if file.tag(TagType::Ape).is_some() {
        let native = native_tags_diagnostic(
            &mut fs::File::open(path).map_err(ErrorCause::external)?,
            file.file_type(),
        )?;
        ape_pictures_diagnostic(&native, &mut file)?;
    }
    mp4_chapters::add_nero_covers(&mut file, path);
    Ok(file)
}

/// Drop the ID3v1 tag lofty read from the phantom of `ape_footer`.
pub(super) fn without_phantom(mut file: TaggedFile, phantom: bool) -> TaggedFile {
    if phantom {
        file.remove(TagType::Id3v1);
    }
    file
}

/// Generic APE conversion intentionally omits cover binary items. Add them
/// from the already parsed native tag.
fn ape_pictures_diagnostic(native: &NativeTags, file: &mut TaggedFile) -> Result<(), ErrorCause> {
    if let (Some(ape), Some(tag)) = (native.ape.as_ref(), file.tag_mut(TagType::Ape)) {
        for item in ape {
            if item.key().starts_with("Cover Art (") {
                if let ItemValue::Binary(data) = item.value() {
                    tag.push_picture(
                        Picture::from_ape_bytes(item.key(), data).map_err(ErrorCause::external)?,
                    );
                }
            }
        }
    }
    Ok(())
}

/// Tags in display order, primary first. FLAC reads Vorbis comments only: a
/// prepended ID3v2 tag is never shown and is stripped by the next save.
fn ordered_tags(file: &TaggedFile) -> Vec<&Tag> {
    let primary = file.primary_tag_type();
    let read = |t: &&Tag| !(file.file_type() == FileType::Flac && t.tag_type() == TagType::Id3v2);
    file.tags()
        .iter()
        .filter(read)
        .filter(|t| t.tag_type() == primary)
        .chain(
            file.tags()
                .iter()
                .filter(read)
                .filter(|t| t.tag_type() != primary),
        )
        .collect()
}

fn field_name(key: ItemKey) -> String {
    match key {
        ItemKey::TrackNumber => "TRACK".into(),
        ItemKey::TrackTotal => "TOTALTRACKS".into(),
        ItemKey::DiscNumber => "DISCNUMBER".into(),
        ItemKey::DiscTotal => "TOTALDISCS".into(),
        ItemKey::RecordingDate | ItemKey::Year => "YEAR".into(),
        ItemKey::OriginalReleaseDate => "ORIGYEAR".into(),
        ItemKey::ContentGroup => "GROUPING".into(),
        ItemKey::UnsyncLyrics => "UNSYNCEDLYRICS".into(),
        ItemKey::CopyrightMessage => "COPYRIGHT".into(),
        _ => key
            .map_key(TagType::VorbisComments)
            .or_else(|| key.map_key(TagType::Ape))
            .map(str::to_uppercase)
            .unwrap_or_else(|| format!("{key:?}").to_uppercase()),
    }
}

fn text_value(value: &ItemValue) -> Option<&str> {
    match value {
        ItemValue::Text(v) | ItemValue::Locator(v) => Some(v),
        _ => None,
    }
}

#[derive(Default)]
struct NativeTags {
    vorbis: Option<VorbisComments>,
    ape: Option<ApeTag>,
    id3: Option<Id3v2Tag>,
    id3_placeholders: crate::id3_frames::Placeholders,
    ilst: Option<Ilst>,
    riff: Option<RiffInfoList>,
    technical: Fields,
}

// Lofty 0.25 generic conversions discard unknown APE and Vorbis items. Keep
// their concrete tags, including flags, vendor and binary items, separately.
fn native_tags(reader: &mut fs::File, kind: FileType) -> Result<NativeTags, String> {
    native_tags_diagnostic(reader, kind).map_err(|error| error.to_string())
}

fn native_tags_diagnostic(reader: &mut fs::File, kind: FileType) -> Result<NativeTags, ErrorCause> {
    strict_or_lenient_id3v1_diagnostic(|mode| {
        crate::id3_frames::read_with_fallback_diagnostic(reader, |reader| {
            native_tags_with_diagnostic(reader, kind, mode)
        })
        .map(|(mut native, names, _)| {
            native.id3_placeholders = names;
            native
        })
    })
}
fn native_tags_with_diagnostic(
    mut reader: &mut dyn ReadSeek,
    kind: FileType,
    mode: ParsingMode,
) -> Result<NativeTags, ErrorCause> {
    reader
        .seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    let options = ParseOptions::new()
        .implicit_conversions(false)
        .read_properties(false)
        .parsing_mode(mode);
    let mut native = NativeTags::default();
    match kind {
        FileType::Flac => {
            let file = FlacFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            native.id3 = file.id3v2().cloned();
            let mut tag = file.vorbis_comments().cloned().unwrap_or_default();
            for (picture, info) in file.pictures() {
                tag.insert_picture(picture.clone(), Some(*info))
                    .map_err(ErrorCause::external)?;
            }
            native.vorbis = Some(tag);
        }
        FileType::Vorbis => {
            native.vorbis = Some(
                VorbisFile::read_from(&mut reader, options)
                    .map_err(|e| ErrorCause::external(error_message(&e)))?
                    .vorbis_comments()
                    .clone(),
            )
        }
        FileType::Opus => {
            native.vorbis = Some(
                OpusFile::read_from(&mut reader, options)
                    .map_err(|e| ErrorCause::external(error_message(&e)))?
                    .vorbis_comments()
                    .clone(),
            )
        }
        FileType::Speex => {
            native.vorbis = Some(
                SpeexFile::read_from(&mut reader, options)
                    .map_err(|e| ErrorCause::external(error_message(&e)))?
                    .vorbis_comments()
                    .clone(),
            )
        }
        FileType::Mpeg => {
            let file = MpegFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            native.ape = file.ape().cloned();
            native.id3 = file.id3v2().cloned();
        }
        FileType::Ape => {
            let file = ApeFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            native.ape = file.ape().cloned();
            native.id3 = file.id3v2().cloned();
        }
        FileType::Mpc => {
            let file = MpcFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            native.ape = file.ape().cloned();
            native.id3 = file.id3v2().cloned();
        }
        FileType::WavPack => {
            native.ape = WavPackFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?
                .ape()
                .cloned()
        }
        FileType::Aac => {
            native.id3 = AacFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?
                .id3v2()
                .cloned()
        }
        FileType::Wav => {
            let file = WavFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            native.id3 = file.id3v2().cloned();
            native.riff = file.riff_info().cloned();
        }
        FileType::Aiff => {
            let file = AiffFile::read_from(&mut reader, options)
                .map_err(|e| ErrorCause::external(error_message(&e)))?;
            native.id3 = file.id3v2().cloned();
        }
        FileType::Mp4 => {
            native.ilst = read_mp4_diagnostic(&mut reader, options)?.ilst().cloned();
        }
        _ => {}
    }
    Ok(native)
}

/// Logical fields of one tag, using its concrete native representation.
fn tag_fields_diagnostic(
    tag: &Tag,
    native: &NativeTags,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<Fields, ErrorCause> {
    let mut local = Fields::new();
    match tag.tag_type() {
        TagType::Id3v2 => {
            if let Some(tag) = native.id3.as_ref() {
                local = mapping::id3_fields_with(
                    tag,
                    tag.original_version(),
                    options.advanced.popularimeter_ratings,
                );
            }
        }
        TagType::Mp4Ilst => {
            if let Some(tag) = native.ilst.as_ref() {
                local = mapping::mp4_fields(tag);
            }
        }
        TagType::VorbisComments => {
            if let Some(tag) = native.vorbis.as_ref() {
                for (key, value) in tag.items() {
                    local
                        .entry(key.to_owned())
                        .or_default()
                        .push(value.to_owned());
                }
            }
        }
        TagType::Ape => {
            if let Some(tag) = native.ape.as_ref() {
                for item in tag {
                    if let Some(value) = text_value(item.value()) {
                        local
                            .entry(item.key().to_owned())
                            .or_default()
                            .extend(value.split('\0').map(str::to_owned));
                    }
                }
            }
        }
        TagType::RiffInfo => {
            if let Some(tag) = native.riff.as_ref() {
                for (key, value) in tag {
                    let name = FIELD_MAPPINGS
                        .iter()
                        .find(|m| !m.riff.is_empty() && m.riff.eq_ignore_ascii_case(key))
                        .map(|m| m.field.to_owned())
                        .unwrap_or_else(|| key.to_owned());
                    local.entry(name).or_default().push(value.to_owned());
                }
            }
        }
        _ => {
            for item in tag.items() {
                if let Some(value) = text_value(item.value()) {
                    local
                        .entry(field_name(item.key()))
                        .or_default()
                        .push(value.to_owned());
                }
            }
        }
    }
    map_read_fields_diagnostic(&local, tag.tag_type(), rules)
}

/// MPEG and APE-family files display one enabled tag kind by priority. Other
/// formats keep primary values and add fields only present in secondary tags.
fn displayed_tags<'a>(file: &'a TaggedFile, options: &TagOptions) -> Vec<&'a Tag> {
    match tag_policy::read_order(file.file_type(), options) {
        Some(order) => order
            .iter()
            .find_map(|kind| file.tag(*kind))
            .into_iter()
            .collect(),
        None => ordered_tags(file),
    }
}

fn cover_info(index: usize, picture: &Picture, include_cover_data: bool) -> CoverInfo {
    let info = technical::picture_information(picture);
    CoverInfo {
        index,
        mime: picture
            .mime_type()
            .map(|m| m.as_str())
            .unwrap_or("application/octet-stream")
            .into(),
        description: picture.description().unwrap_or_default().into(),
        picture_type: format!("{:?}", picture.pic_type()),
        data_base64: if include_cover_data {
            STANDARD.encode(picture.data())
        } else {
            String::new()
        },
        size: picture.data().len() as u64,
        width: info.as_ref().map(|i| i.width).filter(|n| *n > 0),
        height: info.as_ref().map(|i| i.height).filter(|n| *n > 0),
        digest: cover_digest(picture.data()),
    }
}

fn describe_with_native_diagnostic(
    path: &Path,
    file: &TaggedFile,
    native: &NativeTags,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    let mut fields = Fields::new();
    let mut covers = Vec::new();
    let displayed = displayed_tags(file, options);
    for tag in &displayed {
        let local = tag_fields_diagnostic(tag, native, rules, options)?;
        // Primary values take precedence over legacy representations. Secondary
        // tags still contribute fields that the primary does not contain.
        for (key, values) in local {
            fields.entry(key).or_insert(values);
        }
        for picture in tag.pictures() {
            covers.push(cover_info(covers.len(), picture, include_cover_data));
        }
    }
    let properties = file.properties();
    let metadata = fs::metadata(path).map_err(ErrorCause::external)?;
    let read = displayed.first().map(|tag| tag.tag_type());
    let technical = technical::collect(path, file, native, &covers, &metadata, read);
    let chapters = match native.id3.as_ref().map(subsongs::id3_chapters) {
        Some(list) if !list.is_empty() => list.into_iter().map(|(_, c)| c).collect(),
        _ => subsongs::embedded_cue_chapters(&fields),
    };
    let mut entry = AudioEntry {
        path: path.to_string_lossy().into_owned(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        format: match file.file_type() {
            FileType::Mpeg => "MP3",
            FileType::Aac => "AAC",
            FileType::Aiff => "AIFF",
            FileType::Ape => "APE",
            FileType::Flac => "FLAC",
            FileType::Mp4 => "MP4",
            FileType::Mpc => "MPC",
            FileType::Opus => "Opus",
            FileType::Vorbis => "Ogg Vorbis",
            FileType::Speex => "Speex",
            FileType::Wav => "WAV",
            FileType::WavPack => "WavPack",
            _ => "Unknown",
        }
        .into(),
        duration: properties.duration().as_secs_f64(),
        bitrate: properties
            .audio_bitrate()
            .or(properties.overall_bitrate())
            .unwrap_or(0),
        sample_rate: properties.sample_rate().unwrap_or(0),
        channels: properties.channels().unwrap_or(0),
        size: metadata.len(),
        fields,
        covers,
        technical,
        chapters: Vec::new(),
    };
    subsongs::attach(&mut entry, chapters);
    Ok(entry)
}

pub fn read_entry(path: &Path) -> Result<AudioEntry, String> {
    read_entry_with_mappings(path, &[])
}
pub fn read_entry_with_mappings(
    path: &Path,
    rules: &[UserFieldMapping],
) -> Result<AudioEntry, String> {
    read_description(path, true, rules, &TagOptions::default())
}
/// Read with explicit tag preferences (which tag kind is displayed, ratings).
pub fn read_entry_with_options(
    path: &Path,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    read_entry_with_options_diagnostic(path, rules, options).map_err(|error| error.to_string())
}

pub fn read_entry_with_options_diagnostic(
    path: &Path,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    read_description_diagnostic(path, true, rules, options)
}
/// Read list fields and image descriptors without serializing artwork.
/// Each supported format is parsed once, keeping concrete tags and properties.
pub fn read_entry_summary(path: &Path) -> Result<AudioEntry, String> {
    read_entry_summary_with_mappings(path, &[])
}
pub fn read_entry_summary_with_mappings(
    path: &Path,
    rules: &[UserFieldMapping],
) -> Result<AudioEntry, String> {
    read_description(path, false, rules, &TagOptions::default())
}
pub fn read_entry_summary_with_options(
    path: &Path,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    read_entry_summary_with_options_diagnostic(path, rules, options)
        .map_err(|error| error.to_string())
}

pub fn read_entry_summary_with_options_diagnostic(
    path: &Path,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    read_description_diagnostic(path, false, rules, options)
}
fn read_description(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    read_description_diagnostic(path, include_cover_data, rules, options)
        .map_err(|error| error.to_string())
}

fn read_description_diagnostic(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    validate_user_mappings_diagnostic(rules)?;
    if let Some((file, number)) = subsongs::split_path(path) {
        return subsongs::read_row_diagnostic(&file, number, include_cover_data, rules, options);
    }
    let path = dunce::canonicalize(path).map_err(ErrorCause::external)?;
    // Before lofty: its MPEG sync search can match bytes inside ASF artwork.
    if asf::sniff(&path) {
        return asf::describe_diagnostic(&path, include_cover_data, rules);
    }
    if let Some(kind) = container_formats::detect(&path) {
        return container_formats::read_diagnostic(&path, kind, include_cover_data, rules, options);
    }
    if let Some(stream) = container_formats::detect_stream(&path) {
        return container_formats::raw_streams::read_diagnostic(
            &path,
            stream,
            include_cover_data,
            rules,
            options,
        );
    }
    let probe = Probe::open(&path)
        .map_err(ErrorCause::external)?
        .guess_file_type()
        .map_err(ErrorCause::external)?;
    let kind = probe
        .file_type()
        .ok_or_else(|| ErrorCause::owned("Unrecognized audio file type"))?;
    let parse_options = ParseOptions::new()
        .implicit_conversions(false)
        .parsing_mode(ParsingMode::Strict);
    let mut reader = probe.into_inner();
    macro_rules! read {
        ($ty:ty,$snapshot:expr) => {{
            let (media, names, phantom) = strict_or_lenient_id3v1_diagnostic(|mode| {
                crate::id3_frames::read_with_fallback_diagnostic(&mut reader, |mut reader| {
                    reader
                        .seek(SeekFrom::Start(0))
                        .map_err(ErrorCause::external)?;
                    let mut options = parse_options;
                    <$ty>::read_from(&mut reader, options.parsing_mode(mode))
                        .map_err(|e| ErrorCause::external(error_message(&e)))
                })
            })?;
            let mut native: NativeTags = ($snapshot)(&media);
            native.id3_placeholders = names;
            (without_phantom(TaggedFile::from(media), phantom), native)
        }};
    }
    let (mut file, mut native) = match kind {
        FileType::Flac => read!(FlacFile, |f: &FlacFile| NativeTags {
            vorbis: f.vorbis_comments().cloned(),
            ..Default::default()
        }),
        FileType::Mpeg => read!(MpegFile, |f: &MpegFile| NativeTags {
            id3: f.id3v2().cloned(),
            ape: f.ape().cloned(),
            technical: technical::mpeg(f.properties()),
            ..Default::default()
        }),
        FileType::Mp4 => {
            let media = read_mp4_diagnostic(&mut reader, parse_options)?;
            let native = NativeTags {
                ilst: media.ilst().cloned(),
                technical: technical::mp4(media.properties()),
                ..Default::default()
            };
            let mut file = TaggedFile::from(media);
            mp4_chapters::add_nero_covers(&mut file, &path);
            (file, native)
        }
        FileType::Wav => read!(WavFile, |f: &WavFile| NativeTags {
            id3: f.id3v2().cloned(),
            riff: f.riff_info().cloned(),
            technical: technical::wav(f.properties()),
            ..Default::default()
        }),
        FileType::Aiff => read!(AiffFile, |f: &AiffFile| NativeTags {
            id3: f.id3v2().cloned(),
            technical: technical::aiff(f.properties()),
            ..Default::default()
        }),
        FileType::Vorbis => read!(VorbisFile, |f: &VorbisFile| NativeTags {
            vorbis: Some(f.vorbis_comments().clone()),
            ..Default::default()
        }),
        FileType::Opus => read!(OpusFile, |f: &OpusFile| NativeTags {
            vorbis: Some(f.vorbis_comments().clone()),
            ..Default::default()
        }),
        FileType::Speex => read!(SpeexFile, |f: &SpeexFile| NativeTags {
            vorbis: Some(f.vorbis_comments().clone()),
            ..Default::default()
        }),
        FileType::Ape => read!(ApeFile, |f: &ApeFile| NativeTags {
            ape: f.ape().cloned(),
            id3: f.id3v2().cloned(),
            ..Default::default()
        }),
        FileType::WavPack => read!(WavPackFile, |f: &WavPackFile| NativeTags {
            ape: f.ape().cloned(),
            ..Default::default()
        }),
        FileType::Mpc => read!(MpcFile, |f: &MpcFile| NativeTags {
            ape: f.ape().cloned(),
            id3: f.id3v2().cloned(),
            ..Default::default()
        }),
        FileType::Aac => read!(AacFile, |f: &AacFile| NativeTags {
            id3: f.id3v2().cloned(),
            ..Default::default()
        }),
        _ => {
            let file = load_diagnostic(&path)?;
            let native = native_tags_diagnostic(reader.get_mut(), kind)?;
            (file, native)
        }
    };
    if kind == FileType::Mp4 {
        native
            .technical
            .extend(mp4_chapters::header_technical(&path));
    }
    // Enrich APE covers from the already parsed native tag, without a second
    // file read.
    ape_pictures_diagnostic(&native, &mut file)?;
    let mut entry =
        describe_with_native_diagnostic(&path, &file, &native, include_cover_data, rules, options)?;
    if kind == FileType::Mp4 {
        mp4_chapters::describe(&path, &mut entry, rules);
    }
    name_placeholders(&mut entry.technical, &native.id3_placeholders);
    Ok(entry)
}

/// List encrypted ID3v2 frames read as placeholders under their stored
/// identifiers in `%_id3v2_unknown_frames%`.
fn name_placeholders(technical: &mut Fields, names: &[(String, String)]) {
    if names.is_empty() {
        return;
    }
    if let Some(values) = technical.get_mut("_ID3V2_UNKNOWN_FRAMES") {
        let mut ids: Vec<String> = values
            .join(", ")
            .split(", ")
            .map(|id| {
                names
                    .iter()
                    .find(|(placeholder, _)| placeholder == id)
                    .map_or_else(|| id.to_owned(), |(_, stored)| stored.clone())
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        *values = vec![ids.join(", ")];
    }
}

fn audio_extension(path: &Path, allowed: Option<&[String]>) -> bool {
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if let Some(allowed) = allowed {
        return allowed
            .iter()
            .any(|a| a.trim_start_matches('.').eq_ignore_ascii_case(&extension));
    }
    matches!(
        extension.as_str(),
        "mp3"
            | "mp2"
            | "mp1"
            | "aac"
            | "aif"
            | "aiff"
            | "aifc"
            | "ape"
            | "asf"
            | "dff"
            | "dsf"
            | "flac"
            | "m4a"
            | "m4b"
            | "m4p"
            | "mp4"
            | "m4v"
            | "mka"
            | "mkv"
            | "mpc"
            | "mp+"
            | "mpp"
            | "ofr"
            | "ofs"
            | "ogg"
            | "oga"
            | "opus"
            | "spx"
            | "tak"
            | "tta"
            | "wav"
            | "wave"
            | "webm"
            | "wma"
            | "wv"
    )
}

pub fn scan_paths(paths: &[String], recursive: bool) -> ScanResult {
    scan_paths_with_options(paths, recursive, &TagOptions::default(), None)
}
/// `extensions` restricts files found in directories (Options → Files → Restrict
/// incoming files to); explicitly listed files are always read.
pub fn scan_paths_with_options(
    paths: &[String],
    recursive: bool,
    options: &TagOptions,
    extensions: Option<&[String]>,
) -> ScanResult {
    scan_paths_with_progress(
        paths,
        recursive,
        options,
        extensions,
        &crate::parallel::Silent,
    )
}
/// Entries kept between sessions (Options → Library) that stand in for reading
/// unchanged files. Paths are canonical.
pub trait EntryCache {
    /// The stored entry of each path, or `None` where the file must be read.
    fn lookup(&self, paths: &[PathBuf]) -> Vec<Option<AudioEntry>>;
    /// Keep entries that were just read from their files.
    fn store(&self, entries: &[&AudioEntry]);
}

/// Like [`scan_paths_with_options`], reading files in parallel while reporting
/// progress. After a cancellation the files read so far are returned.
pub fn scan_paths_with_progress(
    paths: &[String],
    recursive: bool,
    options: &TagOptions,
    extensions: Option<&[String]>,
    progress: &dyn crate::parallel::Progress,
) -> ScanResult {
    scan_paths_cached(paths, recursive, options, extensions, progress, None)
}
/// [`scan_paths_with_progress`] that takes unchanged files from `cache` and
/// stores the files it had to read there.
pub fn scan_paths_cached(
    paths: &[String],
    recursive: bool,
    options: &TagOptions,
    extensions: Option<&[String]>,
    progress: &dyn crate::parallel::Progress,
    cache: Option<&dyn EntryCache>,
) -> ScanResult {
    let mut result = ScanResult::default();
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    let mut rows = Vec::new();
    let mut input_position = 0usize;
    let mut input_ranks = std::collections::HashMap::new();
    for raw in paths {
        // Chapter and cue sheet rows are read as they are.
        if subsong_parts(raw).is_some() {
            rows.push((input_position, raw));
            input_position += 1;
            continue;
        }
        let path = PathBuf::from(raw);
        match fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => {
                let walk = WalkDir::new(&path).follow_links(false).sort_by_file_name();
                let walk = if recursive { walk } else { walk.max_depth(1) };
                for item in walk {
                    match item {
                        Ok(item) if item.file_type().is_file() || item.file_type().is_symlink() => {
                            if audio_extension(item.path(), extensions) {
                                candidates.push((input_position, item.into_path()));
                                input_position += 1;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => result.errors.push(FileError {
                            path: e.path().unwrap_or(&path).to_string_lossy().into_owned(),
                            error: e.to_string(),
                        }),
                    }
                }
            }
            Ok(_) => {
                candidates.push((input_position, path));
                input_position += 1;
            }
            Err(e) => result.errors.push(FileError {
                path: raw.clone(),
                error: e.to_string(),
            }),
        }
    }
    let has_explicit_rows = !rows.is_empty();
    let mut unique = Vec::new();
    for (position, path) in candidates {
        match dunce::canonicalize(&path) {
            Ok(canonical) => {
                if seen.insert(canonical.clone()) {
                    if has_explicit_rows {
                        input_ranks.insert(canonical.clone(), position);
                    }
                    unique.push(canonical);
                }
            }
            Err(e) => result.errors.push(FileError {
                path: path.to_string_lossy().into_owned(),
                error: e.to_string(),
            }),
        }
    }
    progress.start(unique.len());
    let mut stored = match cache {
        Some(cache) => cache.lookup(&unique),
        None => vec![None; unique.len()],
    };
    let missing: Vec<&PathBuf> = unique
        .iter()
        .zip(&stored)
        .filter(|(_, entry)| entry.is_none())
        .map(|(path, _)| path)
        .collect();
    for _ in missing.len()..unique.len() {
        progress.advance();
    }
    let read = crate::parallel::map_ordered(&missing, progress, |canonical| {
        read_entry_summary_with_options(canonical, &options.mappings, options)
    });
    if let Some(cache) = cache {
        let fresh: Vec<&AudioEntry> = read.iter().flatten().flatten().collect();
        if !fresh.is_empty() {
            cache.store(&fresh);
        }
    }
    let mut read = read.into_iter();
    for (canonical, entry) in unique.iter().zip(stored.iter_mut()) {
        let outcome = match entry.take() {
            Some(entry) => Some(Ok(entry)),
            None => read.next().flatten(),
        };
        match outcome {
            Some(Ok(entry)) => result.entries.push(entry),
            Some(Err(error)) => result.errors.push(FileError {
                path: canonical.to_string_lossy().into_owned(),
                error,
            }),
            None => {}
        }
    }
    if options.advanced.list_chapters {
        result.entries = expand_chapter_rows(std::mem::take(&mut result.entries));
    }
    for (position, raw) in rows {
        if !seen.insert(PathBuf::from(raw)) {
            continue;
        }
        input_ranks.insert(PathBuf::from(raw), position);
        match read_entry_summary_with_options(Path::new(raw), &options.mappings, options) {
            Ok(entry) => result.entries.push(entry),
            Err(error) => result.errors.push(FileError {
                path: raw.clone(),
                error,
            }),
        }
    }
    // Ordinary audio is read in parallel and explicit CUE/chapter rows are
    // read afterwards, but the combined result must retain their input order.
    // Rows expanded from one ordinary file keep their existing relative order.
    if has_explicit_rows {
        result.entries.sort_by_key(|entry| {
            input_ranks
                .get(Path::new(&entry.path))
                .or_else(|| {
                    subsong_parts(&entry.path)
                        .and_then(|(path, _)| input_ranks.get(Path::new(path)))
                })
                .copied()
                .unwrap_or(usize::MAX)
        });
    }
    result
}

/// Write the edits of several rows of one file with chapters (or of one
/// loaded cue sheet) together; `rows` pairs 1-based subsong numbers with
/// their field changes. Fields other than the chapter's own are file fields
/// and must agree between the rows. Returns the rows as read back.
pub fn write_chapter_rows(
    file: &Path,
    rows: &[(usize, Fields)],
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<Vec<AudioEntry>, String> {
    validate_user_mappings(&options.mappings)?;
    options.validate()?;
    subsongs::write_rows(file, rows, include_cover_data, &options.mappings, options)
}

/// Copy `source` to a new temporary file in `dir`. The copy is created at a
/// fresh path rather than into an already opened temporary file, so it can be a
/// copy-on-write clone (APFS `clonefile`) that shares every block the write
/// does not change; elsewhere it is an ordinary copy (`copy_file_range`, which
/// may reflink, but TagGroove cannot tell). Either way it is a new inode: writes
/// never reach `source`, and persisting it renames it over the original. The
/// file is removed when the handle is dropped unpersisted. [`clones_in`]
/// reports whether copies in `dir` are clones.
pub fn staging_copy(
    source: &Path,
    dir: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<tempfile::NamedTempFile, String> {
    staging_clone(source, dir, prefix, suffix).map(|(file, _)| file)
}

/// [`staging_copy`], also reporting whether the copy is a true clone.
pub fn staging_clone(
    source: &Path,
    dir: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<(tempfile::NamedTempFile, bool), String> {
    let mut cloned = false;
    let file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .rand_bytes(12)
        .make_in(dir, |path| {
            let copied = match clone_file(source, path) {
                Ok(true) => {
                    cloned = true;
                    Ok(())
                }
                Ok(false) => fs::copy(source, path).map(|_| ()),
                Err(error) => Err(error),
            };
            if let Err(error) = copied {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
            fs::OpenOptions::new().read(true).write(true).open(path)
        })
        .map_err(|e| e.to_string())?;
    Ok((file, cloned))
}

/// Whether [`staging_copy`] makes copy-on-write clones in `dir`: true only
/// when a clone was verified (APFS on Apple platforms). An undo snapshot that
/// hard-links the original then retains only the blocks a write changed,
/// rather than the whole file. Probes with two empty temporary files.
pub fn clones_in(dir: &Path) -> bool {
    let Ok(probe) = tempfile::Builder::new()
        .prefix(".opentag-probe-")
        .tempfile_in(dir)
    else {
        return false;
    };
    let target = probe.path().with_extension("clone");
    let cloned = matches!(clone_file(probe.path(), &target), Ok(true));
    if cloned {
        let _ = fs::remove_file(&target);
    }
    cloned
}

/// Clone `source` to the new path `target`. `Ok(false)` means the filesystem
/// cannot clone here and nothing was created; an existing `target` is an error.
#[cfg(target_vendor = "apple")]
fn clone_file(source: &Path, target: &Path) -> std::io::Result<bool> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let from = CString::new(source.as_os_str().as_bytes())?;
    let to = CString::new(target.as_os_str().as_bytes())?;
    const CLONE_NOFOLLOW: u32 = 0x0001; // <sys/clonefile.h>
                                        // SAFETY: both arguments are valid NUL-terminated paths for the duration of the call.
    if unsafe { libc::clonefile(from.as_ptr(), to.as_ptr(), CLONE_NOFOLLOW) } == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::EEXIST) => Err(error),
        _ => Ok(false),
    }
}
#[cfg(not(target_vendor = "apple"))]
fn clone_file(_source: &Path, _target: &Path) -> std::io::Result<bool> {
    Ok(false)
}

/// The closure only touches the temporary copy. Failed writes or failed
/// validation remove the copy through NamedTempFile's destructor.
fn staged_mutation<F>(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
    mutation: F,
) -> Result<AudioEntry, String>
where
    F: FnOnce(&mut fs::File, TaggedFile, &Path) -> Result<(), String>,
{
    staged_mutation_diagnostic(
        path,
        include_cover_data,
        options,
        |staging, file, staged_path| {
            mutation(staging, file, staged_path).map_err(ErrorCause::external)
        },
    )
    .map_err(|error| error.to_string())
}

fn staged_mutation_diagnostic<F>(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
    mutation: F,
) -> Result<AudioEntry, ErrorCause>
where
    F: FnOnce(&mut fs::File, TaggedFile, &Path) -> Result<(), ErrorCause>,
{
    staged_mutation_with_mappings_diagnostic(
        path,
        include_cover_data,
        &options.mappings,
        options,
        mutation,
    )
}

fn staged_mutation_with_mappings_diagnostic<F>(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    mutation: F,
) -> Result<AudioEntry, ErrorCause>
where
    F: FnOnce(&mut fs::File, TaggedFile, &Path) -> Result<(), ErrorCause>,
{
    staged_write_diagnostic(
        path,
        include_cover_data,
        rules,
        options,
        |staging, original, staged_path| match container_formats::detect_stream(original) {
            // Tags of raw streams are edited on a stand-in file lofty can parse.
            Some(stream) => container_formats::raw_streams::stage_diagnostic(
                staging,
                original,
                staged_path,
                stream,
                |staging, original, staged_path| {
                    let guard =
                        crate::ape_footer::Guard::prepare(staging).map_err(ErrorCause::external)?;
                    mutation(staging, load_diagnostic(original)?, staged_path)?;
                    guard.finish(staging).map_err(ErrorCause::external)
                },
            ),
            None => {
                // Trailing APE tags lofty's writers would misplace.
                let guard =
                    crate::ape_footer::Guard::prepare(staging).map_err(ErrorCause::external)?;
                let file = load_diagnostic(original)?;
                let flac_id3 =
                    file.file_type() == FileType::Flac && file.tag(TagType::Id3v2).is_some();
                mutation(staging, file, staged_path)?;
                guard.finish(staging).map_err(ErrorCause::external)?;
                if flac_id3 {
                    strip_flac_id3v2_diagnostic(staging)?;
                }
                Ok(())
            }
        },
    )
}

/// Mp3tag removes the ID3v2 tag some taggers prepend to FLAC files whenever
/// it saves them; the FLAC stream itself is kept byte for byte.
fn strip_flac_id3v2_diagnostic(staging: &mut fs::File) -> Result<(), ErrorCause> {
    staging
        .seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    let mut header = [0u8; 10];
    if std::io::Read::read_exact(staging, &mut header).is_err()
        || container_formats::id3v2_size(&header).is_none()
    {
        return Ok(());
    }
    staging
        .seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    crate::write_cost::rewrote_all();
    TagType::Id3v2
        .remove_from(staging, WriteOptions::default())
        .map_err(|e| ErrorCause::external(error_message(&e)))
}

/// Like `staged_mutation_with_mappings`, for formats that lofty cannot load.
/// The closure receives the staging file, the original path and the staged path.
fn staged_write<F>(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    mutation: F,
) -> Result<AudioEntry, String>
where
    F: FnOnce(&mut fs::File, &Path, &Path) -> Result<(), String>,
{
    staged_write_diagnostic(
        path,
        include_cover_data,
        rules,
        options,
        |staging, original, staged_path| {
            mutation(staging, original, staged_path).map_err(ErrorCause::external)
        },
    )
    .map_err(|error| error.to_string())
}

fn staged_write_diagnostic<F>(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    mutation: F,
) -> Result<AudioEntry, ErrorCause>
where
    F: FnOnce(&mut fs::File, &Path, &Path) -> Result<(), ErrorCause>,
{
    validate_user_mappings_diagnostic(rules)?;
    options.validate().map_err(ErrorCause::owned_text)?;
    if subsongs::split_path(path).is_some() {
        return Err(ErrorCause::owned(subsongs::ROW_OPERATION));
    }
    let path = dunce::canonicalize(path).map_err(ErrorCause::external)?;
    let original = fs::metadata(&path).map_err(ErrorCause::external)?;
    if !original.is_file() {
        return Err(ErrorCause::owned("The audio path must be a regular file"));
    }
    if original.permissions().readonly() {
        return Err(ErrorCause::owned("The audio file is read-only"));
    }
    let (mut staging, cloned) = staging_clone(
        &path,
        path.parent()
            .ok_or_else(|| ErrorCause::owned("Audio path has no parent directory"))?,
        ".opentag-",
        "",
    )
    .map_err(ErrorCause::external)?;
    let staged_path = staging.path().to_path_buf();
    let scope = crate::write_cost::Scope::begin();
    mutation(staging.as_file_mut(), &path, &staged_path)?;
    // An in-place rewrite never changes the length; anything else is a full rewrite.
    let rewritten = scope.finish().filter(|_| {
        staging
            .as_file()
            .metadata()
            .is_ok_and(|m| m.len() == original.len())
    });
    if options.preserve_modified_time {
        if let Ok(modified) = original.modified() {
            staging
                .as_file()
                .set_modified(modified)
                .map_err(ErrorCause::external)?;
        }
    }
    staging.as_file().sync_all().map_err(ErrorCause::external)?;
    let mut entry =
        read_description_diagnostic(staging.path(), include_cover_data, rules, options)?;
    fs::set_permissions(staging.path(), original.permissions()).map_err(ErrorCause::external)?;
    let current = fs::metadata(&path).map_err(ErrorCause::external)?;
    if current.len() != original.len() || current.modified().ok() != original.modified().ok() {
        return Err(ErrorCause::owned(
            "The file changed during editing; reload it before saving",
        ));
    }
    staging
        .persist(&path)
        .map_err(|e| ErrorCause::external(e.error))?;
    crate::history::note_write(&path, cloned, rewritten);
    entry.path = path.to_string_lossy().into_owned();
    entry.filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    technical::path_fields(&mut entry.technical, &path);
    Ok(entry)
}

#[cfg(test)]
fn normalized_updates(updates: &Fields, kind: TagType) -> Result<Fields, String> {
    normalized_updates_diagnostic(updates, kind).map_err(|error| error.to_string())
}

fn normalized_updates_diagnostic(updates: &Fields, kind: TagType) -> Result<Fields, ErrorCause> {
    let mut output = Fields::new();
    let mut seen = HashSet::new();
    for (name, values) in updates {
        if name.trim().is_empty()
            || name.contains(['\0', '\n', '\r'])
            || (kind == TagType::VorbisComments && name.contains('='))
        {
            return Err(ErrorCause::message(
                "Invalid metadata field name: {name}",
                [("name", format!("{name:?}"))],
            ));
        }
        if values.iter().any(|v| v.contains('\0')) {
            return Err(ErrorCause::message(
                "{name}: values cannot contain NUL characters",
                [("name", name.to_string())],
            ));
        }
        let name = if matches!(kind, TagType::VorbisComments | TagType::Ape) {
            name.clone()
        } else {
            field_mapping(name)
                .map(|m| m.field.to_owned())
                .unwrap_or_else(|| name.clone())
        };
        if !seen.insert(name.to_lowercase()) {
            return Err(ErrorCause::message(
                "Duplicate metadata field: {name}",
                [("name", name.to_string())],
            ));
        }
        output.insert(name, values.clone());
    }
    Ok(output)
}

/// Prepare only the raw lyrics changed by this source import for one target.
fn source_lyrics_updates_diagnostic(
    source: &Fields,
    changes: &Fields,
    kind: TagType,
    rules: &[UserFieldMapping],
) -> Result<Fields, ErrorCause> {
    let mut output = source.clone();
    if kind == TagType::Id3v2 {
        for name in changes.keys() {
            let native =
                map_write_fields_diagnostic(&[(name.clone(), Vec::new())].into(), kind, rules)?;
            if native.keys().any(|key| {
                key.eq_ignore_ascii_case("UNSYNCEDLYRICS")
                    || key.to_ascii_uppercase().starts_with("UNSYNCEDLYRICS ")
            }) {
                if let Some((_, values)) = output
                    .iter_mut()
                    .find(|(key, _)| key.eq_ignore_ascii_case(name))
                {
                    for value in values {
                        *value = format!("XXX||{value}");
                    }
                }
            }
        }
    }
    Ok(output)
}

/// How a write treats an ID3v2 tag: keep its revision (non-MPEG formats) or
/// convert it to the configured MPEG revision and encoding.
#[derive(Clone, Copy)]
enum Id3Revision {
    Preserve([u8; 3]),
    Convert(mapping::Id3Target),
}
/// How a write treats native tags: the ID3v2 revision, and the Options →
/// Tags → Advanced settings for MP4 atoms and ID3v2 frames.
#[derive(Clone, Copy)]
struct Id3Write {
    revision: Id3Revision,
    reuse_id3v2_frames: bool,
    reuse_mp4_atoms: bool,
    mp4_single_atom: bool,
}
impl Id3Write {
    fn preserving(options: &TagOptions) -> Self {
        Self::with(Id3Revision::Preserve(options.comment_language()), options)
    }
    fn converting(target: mapping::Id3Target, options: &TagOptions) -> Self {
        Self::with(Id3Revision::Convert(target), options)
    }
    fn with(revision: Id3Revision, options: &TagOptions) -> Self {
        let advanced = &options.advanced;
        Self {
            revision,
            reuse_id3v2_frames: advanced.reuse_id3v2_frames,
            reuse_mp4_atoms: advanced.reuse_mp4_atoms,
            mp4_single_atom: advanced.mp4_single_atom,
        }
    }
}

fn save_preserving_native(
    staging: &mut fs::File,
    file_type: FileType,
    tag: &Tag,
    updates: &Fields,
    covers_changed: bool,
    id3: Id3Write,
) -> Result<(), String> {
    save_preserving_native_diagnostic(staging, file_type, tag, updates, covers_changed, id3)
        .map_err(|error| error.to_string())
}

fn save_preserving_native_diagnostic(
    staging: &mut fs::File,
    file_type: FileType,
    tag: &Tag,
    updates: &Fields,
    covers_changed: bool,
    id3: Id3Write,
) -> Result<(), ErrorCause> {
    // FLAC Vorbis comments, ID3v2 and MP4 ilst can be rewritten in place; their
    // writers report how (flac_metadata, id3_padding, mp4_metadata).
    if !matches!(
        (file_type, tag.tag_type()),
        (FileType::Flac, TagType::VorbisComments) | (_, TagType::Id3v2) | (_, TagType::Mp4Ilst)
    ) {
        crate::write_cost::rewrote_all();
    }
    let originals = native_tags_diagnostic(staging, file_type)?;
    match tag.tag_type() {
        TagType::Id3v2 => {
            let placeholders = originals.id3_placeholders;
            let mut native = originals.id3.unwrap_or_default();
            let target = match id3.revision {
                Id3Revision::Preserve(language) => {
                    mapping::Id3Target::preserving(&native, language)
                }
                Id3Revision::Convert(target) => {
                    mapping::convert_id3_version_diagnostic(&mut native, &target, &placeholders)?;
                    target
                }
            };
            let mut updates = Cow::Borrowed(updates);
            if !id3.reuse_id3v2_frames {
                let moved = mapping::remap_id3(&mut native, &updates, &target);
                if !moved.is_empty() {
                    updates.to_mut().extend(moved);
                }
            }
            mapping::patch_id3_diagnostic(&mut native, &updates, &target)?;
            if covers_changed {
                native.retain(|frame| !matches!(frame, Frame::Picture(_)));
                let pictures: Id3v2Tag = tag.clone().into();
                for frame in &pictures {
                    if matches!(frame, Frame::Picture(_)) {
                        native.insert(frame.clone());
                    }
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            mapping::save_id3_diagnostic(staging, file_type, &native, &target, &placeholders)
        }
        TagType::Mp4Ilst => {
            let mut native = originals.ilst.unwrap_or_default();
            let before = native.clone();
            let mut updates = Cow::Borrowed(updates);
            if !id3.reuse_mp4_atoms {
                let moved = mapping::remap_mp4(&mut native, &updates);
                if !moved.is_empty() {
                    updates.to_mut().extend(moved);
                }
            }
            mapping::patch_mp4_diagnostic(&mut native, &updates)?;
            if covers_changed {
                native.remove_pictures();
                for picture in tag.pictures() {
                    native.insert_picture(picture.clone());
                }
            }
            let mut touched = updates
                .keys()
                .map(|name| match name.as_str() {
                    "TRACK" | "TOTALTRACKS" => Ok(lofty::mp4::AtomIdent::Fourcc(*b"trkn")),
                    "DISCNUMBER" | "TOTALDISCS" => Ok(lofty::mp4::AtomIdent::Fourcc(*b"disk")),
                    _ => mapping::mp4_ident_diagnostic(name),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if covers_changed {
                touched.push(lofty::mp4::AtomIdent::Fourcc(*b"covr"));
            }
            crate::mp4_metadata::save(staging, &before, &native, &touched, id3.mp4_single_atom)?;
            if covers_changed {
                // The covers shown, Nero ones included, are now the iTunes covers.
                mp4_chapters::drop_nero_covers_diagnostic(staging)?;
            }
            Ok(())
        }
        TagType::VorbisComments => {
            let mut native = originals.vorbis.unwrap_or_default();
            for (name, values) in updates {
                let items: Vec<_> = native.take_items().collect();
                for (key, value) in items {
                    if !key.eq_ignore_ascii_case(name) {
                        native.push(key, value);
                    }
                }
                for value in values {
                    native.push(name.clone(), value.clone());
                }
            }
            if covers_changed {
                native.remove_pictures();
                for picture in tag.pictures() {
                    native
                        .insert_picture(picture.clone(), None)
                        .map_err(ErrorCause::external)?;
                }
            }
            if file_type == FileType::Flac {
                return crate::flac_metadata::save(staging, &native).map_err(ErrorCause::external);
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))
        }
        TagType::Ape => {
            let mut native = originals.ape.unwrap_or_default();
            for (name, values) in updates {
                native.remove(name);
                if !values.is_empty() {
                    native.insert(
                        ApeItem::new(name.clone(), ItemValue::Text(values.join("\0")))
                            .map_err(ErrorCause::external)?,
                    );
                }
            }
            if covers_changed {
                let remove: Vec<_> = (&native)
                    .into_iter()
                    .filter(|i| i.key().starts_with("Cover Art ("))
                    .map(|i| i.key().to_owned())
                    .collect();
                for name in remove {
                    native.remove(&name);
                }
                for picture in tag.pictures() {
                    let key = picture
                        .pic_type()
                        .as_ape_key()
                        .ok_or_else(|| ErrorCause::owned("APE does not support this picture type"))?
                        .to_owned();
                    native.insert(
                        ApeItem::new(key, ItemValue::Binary(picture.as_ape_bytes()))
                            .map_err(ErrorCause::external)?,
                    );
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))
        }
        TagType::RiffInfo => {
            let mut native = originals.riff.unwrap_or_default();
            // Fields without a RIFF id live only in the ID3v2 chunk, which
            // every WAV write also updates.
            for (name, values) in updates {
                let Some(key) = riff_key(name) else {
                    continue;
                };
                native.remove(key);
                if !values.is_empty() {
                    native.insert(key.to_owned(), values.join(RIFF_SEPARATOR));
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))
        }
        _ => {
            let mut tag = tag.clone();
            for (name, values) in updates {
                let key = match name.to_ascii_uppercase().as_str() {
                    "TITLE" => Some(ItemKey::TrackTitle),
                    "ARTIST" => Some(ItemKey::TrackArtist),
                    "ALBUM" => Some(ItemKey::AlbumTitle),
                    "COMMENT" => Some(ItemKey::Comment),
                    "GENRE" => Some(ItemKey::Genre),
                    "YEAR" => Some(ItemKey::Year),
                    "TRACK" => Some(ItemKey::TrackNumber),
                    _ => None,
                };
                if let Some(key) = key {
                    tag.remove_key(key);
                    for value in values {
                        tag.push(TagItem::new(key, ItemValue::Text(value.clone())));
                    }
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            tag.save_to(staging, WriteOptions::default())
                .map_err(|e| ErrorCause::external(error_message(&e)))
        }
    }
}

/// RIFF INFO holds one string per id; multiple values are joined.
const RIFF_SEPARATOR: &str = "; ";

/// The RIFF INFO id of a field: its documented mapping, or a field read from
/// an INFO id that has no mapping (for example `IENG`).
fn riff_key(name: &str) -> Option<&str> {
    match field_mapping(name) {
        Some(mapping) => (!mapping.riff.is_empty()).then_some(mapping.riff),
        None => (name.len() == 4
            && name.starts_with('I')
            && name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
        .then_some(name),
    }
}

pub fn write_fields(path: &Path, updates: &Fields) -> Result<AudioEntry, String> {
    write_fields_result(path, updates, true, &[], &TagOptions::default())
}

pub fn write_fields_with_mappings(
    path: &Path,
    updates: &Fields,
    rules: &[UserFieldMapping],
) -> Result<AudioEntry, String> {
    write_fields_result(path, updates, true, rules, &TagOptions::default())
}

pub fn write_fields_summary_with_mappings(
    path: &Path,
    updates: &Fields,
    rules: &[UserFieldMapping],
) -> Result<AudioEntry, String> {
    write_fields_result(path, updates, false, rules, &TagOptions::default())
}

pub fn write_fields_summary(path: &Path, updates: &Fields) -> Result<AudioEntry, String> {
    write_fields_result(path, updates, false, &[], &TagOptions::default())
}

/// Write field changes honouring the tag read/write/remove preferences.
pub fn write_fields_with_options(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    write_fields_with_options_diagnostic(path, updates, include_cover_data, rules, options)
        .map_err(|error| error.to_string())
}

pub fn write_fields_with_options_diagnostic(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    write_fields_result_diagnostic(path, updates, include_cover_data, rules, options)
}

/// Compare a written tag's read-back fields with the requested native values.
fn verify_readback_diagnostic(
    actual: &Fields,
    native_updates: &Fields,
    kind: TagType,
    latin1: bool,
) -> Result<(), ErrorCause> {
    for (key, values) in native_updates {
        let (read_key, mut expected) = if kind == TagType::Id3v2 {
            mapping::expected_id3_readback_diagnostic(key, values)?
        } else {
            (key.clone(), values.clone())
        };
        if latin1 {
            expected = expected.iter().map(|v| mapping::latin1_lossy(v)).collect();
        }
        let found = actual
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(&read_key))
            .map(|(_, values)| values);
        if values.is_empty() {
            if found.is_some() {
                return Err(ErrorCause::message(
                    "The format could not remove field {key}",
                    [("key", key.to_string())],
                ));
            }
        } else if kind == TagType::Mp4Ilst
            && matches!(
                key.as_str(),
                "TRACK" | "DISCNUMBER" | "TOTALTRACKS" | "TOTALDISCS"
            )
        {
            let requested = values
                .first()
                .ok_or_else(|| ErrorCause::owned("Missing number"))?;
            let found = found.and_then(|v| v.first()).ok_or_else(|| {
                ErrorCause::message("The format lost field {key}", [("key", key.to_string())])
            })?;
            let expected_number = requested
                .split('/')
                .next()
                .unwrap_or_default()
                .parse::<u16>()
                .map_err(|_| {
                    ErrorCause::message(
                        "{key} requires an unsigned number",
                        [("key", key.to_string())],
                    )
                })?;
            let actual_number = found
                .split('/')
                .next()
                .unwrap_or_default()
                .parse::<u16>()
                .map_err(|_| {
                    ErrorCause::message(
                        "The format returned an invalid {key}",
                        [("key", key.to_string())],
                    )
                })?;
            if expected_number != actual_number {
                return Err(ErrorCause::message(
                    "The format changed field {key}",
                    [("key", key.to_string())],
                ));
            }
            if let Some((_, total)) = requested.split_once('/') {
                if found.split_once('/').map(|(_, t)| t.parse::<u16>().ok())
                    != Some(total.parse::<u16>().ok())
                {
                    return Err(ErrorCause::message(
                        "The format changed the total in {key}",
                        [("key", key.to_string())],
                    ));
                }
            }
        } else if found != Some(&expected) {
            return Err(ErrorCause::message(
                "The format could not preserve all values for field {key}",
                [("key", key.to_string())],
            ));
        }
    }
    Ok(())
}

fn write_fields_result(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    write_fields_result_diagnostic(path, updates, include_cover_data, rules, options)
        .map_err(|error| error.to_string())
}

/// Write source-import fields whose lyric values are raw text, even when they
/// contain `||`. Only an ID3v2 target receives the unknown-language prefix.
pub fn write_source_fields_with_options(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    write_fields_result_with_lyrics_diagnostic(
        path,
        updates,
        include_cover_data,
        rules,
        options,
        true,
    )
    .map_err(|error| error.to_string())
}

fn write_fields_result_diagnostic(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    write_fields_result_with_lyrics_diagnostic(
        path,
        updates,
        include_cover_data,
        rules,
        options,
        false,
    )
}

fn write_fields_result_with_lyrics_diagnostic(
    path: &Path,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    raw_lyrics: bool,
) -> Result<AudioEntry, ErrorCause> {
    if updates.is_empty() {
        return read_description_diagnostic(path, include_cover_data, rules, options);
    }
    if let Some((file, number)) = subsongs::split_path(path) {
        let updates = if raw_lyrics
            && load_diagnostic(&file).is_ok_and(|file| file.primary_tag_type() == TagType::Id3v2)
        {
            source_lyrics_updates_diagnostic(updates, updates, TagType::Id3v2, rules)?
        } else {
            updates.clone()
        };
        let mut rows = subsongs::write_rows_diagnostic(
            &file,
            &[(number, updates)],
            include_cover_data,
            rules,
            options,
        )?;
        return Ok(rows.remove(0));
    }
    if asf::sniff(path) {
        return asf::write_fields_diagnostic(path, updates, include_cover_data, rules, options);
    }
    if let Some(kind) = container_formats::detect(path) {
        let updates = if raw_lyrics
            && matches!(
                kind,
                container_formats::Kind::Dsf | container_formats::Kind::Dff
            ) {
            source_lyrics_updates_diagnostic(updates, updates, TagType::Id3v2, rules)?
        } else {
            updates.clone()
        };
        return container_formats::write_fields_diagnostic(
            path,
            kind,
            &updates,
            include_cover_data,
            rules,
            options,
        );
    }
    staged_mutation_with_mappings_diagnostic(
        path,
        include_cover_data,
        rules,
        options,
        |staging, file, staged_path| {
            // MP4 chapters are boxes of their own, written after the tag.
            let mut updates = updates.clone();
            let chapters = match file.file_type() {
                FileType::Mp4 => updates
                    .keys()
                    .find(|k| k.eq_ignore_ascii_case(subsongs::CHAPTERS_FIELD))
                    .cloned()
                    .and_then(|key| updates.remove(&key)),
                _ => None,
            };
            if file.file_type() == FileType::Mp4 {
                // First, so the read-back of the iTunes tag sees no stale
                // file-wide Nero value.
                let native = map_write_fields_diagnostic(&updates, TagType::Mp4Ilst, rules)?;
                mp4_chapters::sync_nero_fields_diagnostic(staging, staged_path, &native)?;
            }
            if let Some(values) = chapters {
                if !updates.is_empty() {
                    write_tag_fields_with_lyrics_diagnostic(
                        staging,
                        file,
                        staged_path,
                        &updates,
                        rules,
                        options,
                        raw_lyrics,
                    )?;
                }
                return mp4_chapters::write_diagnostic(staging, staged_path, &values);
            }
            write_tag_fields_with_lyrics_diagnostic(
                staging,
                file,
                staged_path,
                &updates,
                rules,
                options,
                raw_lyrics,
            )
        },
    )
}

/// Write field changes into the tags of a staged lofty-readable file.
fn write_tag_fields_diagnostic(
    staging: &mut fs::File,
    file: TaggedFile,
    staged_path: &Path,
    updates: &Fields,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<(), ErrorCause> {
    write_tag_fields_with_lyrics_diagnostic(
        staging,
        file,
        staged_path,
        updates,
        rules,
        options,
        false,
    )
}

fn write_tag_fields_with_lyrics_diagnostic(
    staging: &mut fs::File,
    mut file: TaggedFile,
    staged_path: &Path,
    updates: &Fields,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    raw_lyrics: bool,
) -> Result<(), ErrorCause> {
    if tag_policy::policy(file.file_type()).is_some() {
        let updates = normalized_updates_diagnostic(updates, TagType::Ape)?;
        let written = tag_policy::write_diagnostic(
            staging,
            &file,
            rules,
            options,
            if raw_lyrics {
                tag_policy::Change::RawLyricsFields(&updates)
            } else {
                tag_policy::Change::Fields(&updates)
            },
        )?;
        return tag_policy::verify_fields_diagnostic(
            staged_path,
            rules,
            options,
            written,
            &updates,
            raw_lyrics,
        );
    }
    let primary = file.primary_tag_type();
    let updates = normalized_updates_diagnostic(updates, primary)?;
    // WAV files carry RIFF INFO and ID3v2 (file-formats). A new INFO
    // chunk starts from every displayed value that has a RIFF id.
    let mut seeded = None;
    if file.file_type() == FileType::Wav && file.tag(TagType::RiffInfo).is_none() {
        let mut values = Fields::new();
        let native = native_tags_diagnostic(staging, FileType::Wav)?;
        for tag in ordered_tags(&file) {
            for (key, list) in tag_fields_diagnostic(tag, &native, rules, options)? {
                values.entry(key).or_insert(list);
            }
        }
        for (name, list) in &updates {
            values.retain(|key, _| !key.eq_ignore_ascii_case(name));
            values.insert(name.clone(), list.clone());
        }
        if map_write_fields_diagnostic(&values, TagType::RiffInfo, rules)?
            .iter()
            .any(|(name, list)| !list.is_empty() && riff_key(name).is_some())
        {
            file.insert_tag(Tag::new(TagType::RiffInfo));
            seeded = Some(values);
        }
    }
    if file.primary_tag().is_none() {
        file.insert_tag(Tag::new(primary));
    }
    let types: Vec<_> = file
        .tags()
        .iter()
        .map(Tag::tag_type)
        .filter(|t| file.tag_support(*t).is_writable())
        .collect();
    for kind in types {
        let tag = file
            .remove(kind)
            .ok_or_else(|| ErrorCause::owned("Tag disappeared during editing"))?;
        let source = match &seeded {
            Some(values) if kind == TagType::RiffInfo => values,
            _ => &updates,
        };
        let source = if raw_lyrics {
            source_lyrics_updates_diagnostic(source, &updates, kind, rules)?
        } else {
            source.clone()
        };
        let native_updates = normalized_updates_diagnostic(
            &map_write_fields_diagnostic(&source, kind, rules)?,
            kind,
        )?;
        save_preserving_native_diagnostic(
            staging,
            file.file_type(),
            &tag,
            &native_updates,
            false,
            Id3Write::preserving(options),
        )?;
    }
    staging
        .seek(SeekFrom::Start(0))
        .map_err(ErrorCause::external)?;
    let actual = read_entry_summary_with_options_diagnostic(staged_path, &[], options)?.fields;
    let expected = if raw_lyrics {
        source_lyrics_updates_diagnostic(&updates, &updates, primary, rules)?
    } else {
        updates.clone()
    };
    let native_updates = normalized_updates_diagnostic(
        &map_write_fields_diagnostic(&expected, primary, rules)?,
        primary,
    )?;
    verify_readback_diagnostic(&actual, &native_updates, primary, false)
}

/// Utils → Strip chapters MP4: remove Nero chapters and QuickTime chapter
/// tracks. Other files, and MP4 files without chapters, are left untouched.
pub fn strip_mp4_chapters_with_options(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    if subsongs::split_path(path).is_some() {
        return Err(subsongs::ROW_OPERATION.into());
    }
    let mp4 = !asf::sniff(path)
        && container_formats::detect(path).is_none()
        && container_formats::detect_stream(path).is_none()
        && Probe::open(path)
            .ok()
            .and_then(|p| p.guess_file_type().ok())
            .is_some_and(|p| p.file_type() == Some(FileType::Mp4));
    if !mp4 || mp4_chapters::scan(path)?.list.is_empty() {
        return read_description(path, include_cover_data, &options.mappings, options);
    }
    staged_write(
        path,
        include_cover_data,
        &options.mappings,
        options,
        mp4_chapters::strip,
    )
}

pub fn remove_tags(path: &Path) -> Result<AudioEntry, String> {
    remove_tags_result(path, true, &TagOptions::default())
}

pub fn remove_tags_summary(path: &Path) -> Result<AudioEntry, String> {
    remove_tags_result(path, false, &TagOptions::default())
}

/// File → Remove tag: strip the kinds chosen under Options → Tags (MPEG and
/// APE-family files) or every tag kind (other formats). Files without such a
/// tag are left untouched.
pub fn remove_tags_with_options(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    remove_tags_result(path, include_cover_data, options)
}

fn remove_tags_result(
    path: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    if asf::sniff(path) {
        return asf::remove_tags(path, include_cover_data, options);
    }
    if let Some(kind) = container_formats::detect(path) {
        return container_formats::remove_tags(path, kind, include_cover_data, options);
    }
    let loaded = load(path)?;
    // Nero tags (per-chapter fields of MP4 files) go with the iTunes tag.
    let nero = loaded.file_type() == FileType::Mp4
        && mp4_chapters::scan(path).is_ok_and(|c| c.nero_tags.is_some());
    if !nero && tag_policy::removed_kinds(&loaded, options)?.is_empty() {
        return read_description(path, include_cover_data, &options.mappings, options);
    }
    staged_mutation(
        path,
        include_cover_data,
        options,
        |staging, file, staged_path| {
            for kind in tag_policy::removed_kinds(&file, options)? {
                staging
                    .seek(SeekFrom::Start(0))
                    .map_err(|e| e.to_string())?;
                crate::write_cost::rewrote_all();
                kind.remove_from(staging, WriteOptions::default())
                    .map_err(|e| e.to_string())?;
            }
            if nero {
                staging.sync_all().map_err(|e| e.to_string())?;
                mp4_chapters::strip_nero_tags(staging, staged_path)?;
            }
            Ok(())
        },
    )
}

pub fn add_cover(path: &Path, image: &Path) -> Result<AudioEntry, String> {
    add_cover_result(
        path,
        image,
        PictureType::CoverFront,
        true,
        &TagOptions::default(),
    )
}

pub fn add_cover_summary(path: &Path, image: &Path) -> Result<AudioEntry, String> {
    add_cover_result(
        path,
        image,
        PictureType::CoverFront,
        false,
        &TagOptions::default(),
    )
}

pub fn add_cover_with_options(
    path: &Path,
    image: &Path,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    add_cover_result(
        path,
        image,
        PictureType::CoverFront,
        include_cover_data,
        options,
    )
}

/// Import Cover from File: embed `image` as `picture_type` (front cover when
/// `None`), optionally deleting existing covers first. The image is decoded
/// before any existing art is removed.
pub fn import_cover_with_options(
    path: &Path,
    image: &Path,
    picture_type: Option<&str>,
    delete_existing: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    import_cover_with_options_diagnostic(path, image, picture_type, delete_existing, options)
        .map_err(|error| error.to_string())
}

pub fn import_cover_with_options_diagnostic(
    path: &Path,
    image: &Path,
    picture_type: Option<&str>,
    delete_existing: bool,
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    let kind = picture_type
        .map(picture_type_from_name_diagnostic)
        .transpose()?
        .unwrap_or(PictureType::CoverFront);
    if delete_existing {
        picture_from_bytes_diagnostic(fs::read(image).map_err(ErrorCause::external)?)?;
        remove_all_covers_with_options_diagnostic(path, options)?;
    }
    add_cover_result_diagnostic(path, image, kind, false, options)
}

/// ID3 requires picture descriptions to identify repeated cover types.
fn describe_repeated_cover(pictures: &[Picture], picture: &mut Picture, label: &str) {
    if picture.description().is_some_and(|d| !d.is_empty())
        || !pictures.iter().any(|p| {
            p.pic_type() == picture.pic_type() && p.description().unwrap_or_default().is_empty()
        })
    {
        return;
    }
    let mut description = label.to_string();
    let mut suffix = 2;
    while pictures
        .iter()
        .any(|p| p.description() == Some(description.as_str()))
    {
        description = format!("{label} ({suffix})");
        suffix += 1;
    }
    picture.set_description(Some(description));
}

fn add_cover_result(
    path: &Path,
    image: &Path,
    kind: PictureType,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    add_cover_result_diagnostic(path, image, kind, include_cover_data, options)
        .map_err(|error| error.to_string())
}

fn add_cover_result_diagnostic(
    path: &Path,
    image: &Path,
    kind: PictureType,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    // The formats Manage Covers accepts, WebP included.
    let mut picture =
        picture_from_bytes_diagnostic(fs::read(image).map_err(ErrorCause::external)?)?;
    picture.set_pic_type(kind);
    let label = image.file_name().unwrap_or_default().to_string_lossy();
    add_picture_result_diagnostic(path, picture, &label, include_cover_data, options)
}

fn add_picture_result(
    path: &Path,
    picture: Picture,
    label: &str,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    add_picture_result_diagnostic(path, picture, label, include_cover_data, options)
        .map_err(|error| error.to_string())
}

fn add_picture_result_diagnostic(
    path: &Path,
    mut picture: Picture,
    label: &str,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    if asf::sniff(path) {
        return asf::change_pictures_diagnostic(
            path,
            include_cover_data,
            options,
            |pictures| {
                pictures.push(picture);
                Ok(())
            },
            "The cover could not be added",
        );
    }
    if let Some(container) = container_formats::detect(path) {
        return container_formats::add_cover_diagnostic(
            path,
            container,
            picture,
            label,
            include_cover_data,
            options,
        );
    }
    staged_mutation_diagnostic(
        path,
        include_cover_data,
        options,
        |staging, mut file, staged_path| {
            if tag_policy::policy(file.file_type()).is_some() {
                let mut pictures = tag_policy::displayed_pictures(&file, options);
                describe_repeated_cover(&pictures, &mut picture, label);
                pictures.push(picture);
                let written = tag_policy::write_diagnostic(
                    staging,
                    &file,
                    &options.mappings,
                    options,
                    tag_policy::Change::Pictures(pictures.clone()),
                )?;
                return tag_policy::verify_pictures_diagnostic(
                    staged_path,
                    options,
                    written,
                    &pictures,
                    "This tag format cannot append another cover with the same picture type",
                );
            }
            let primary = file.primary_tag_type();
            let previous_covers: usize = ordered_tags(&file)
                .iter()
                .map(|tag| tag.pictures().len())
                .sum();
            if file.primary_tag().is_none() {
                file.insert_tag(Tag::new(primary));
            }
            let file_type = file.file_type();
            let tag = file
                .primary_tag_mut()
                .ok_or_else(|| ErrorCause::owned("The audio format has no writable tag"))?;
            if primary == TagType::Id3v2 {
                describe_repeated_cover(tag.pictures(), &mut picture, label);
            }
            tag.push_picture(picture);
            save_preserving_native_diagnostic(
                staging,
                file_type,
                tag,
                &Fields::new(),
                true,
                Id3Write::preserving(options),
            )?;
            if read_entry_summary_with_options_diagnostic(staged_path, &[], options)?
                .covers
                .len()
                != previous_covers + 1
            {
                return Err(ErrorCause::owned(
                    "This tag format cannot append another cover with the same picture type",
                ));
            }
            Ok(())
        },
    )
}

pub fn remove_cover(path: &Path, index: usize) -> Result<AudioEntry, String> {
    remove_cover_result(path, index, true, &TagOptions::default())
}

pub fn remove_cover_summary(path: &Path, index: usize) -> Result<AudioEntry, String> {
    remove_cover_result(path, index, false, &TagOptions::default())
}

pub fn remove_cover_with_options(
    path: &Path,
    index: usize,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    remove_cover_result(path, index, include_cover_data, options)
}

fn remove_cover_result(
    path: &Path,
    index: usize,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    if asf::sniff(path) {
        return asf::change_pictures(
            path,
            include_cover_data,
            options,
            |pictures| {
                if index >= pictures.len() {
                    return Err(format!("Cover index {index} does not exist"));
                }
                pictures.remove(index);
                Ok(())
            },
            "The cover could not be removed",
        );
    }
    if let Some(kind) = container_formats::detect(path) {
        return container_formats::remove_cover(path, kind, index, include_cover_data, options);
    }
    staged_mutation(
        path,
        include_cover_data,
        options,
        |staging, mut file, staged_path| {
            if tag_policy::policy(file.file_type()).is_some() {
                let mut pictures = tag_policy::displayed_pictures(&file, options);
                if index >= pictures.len() {
                    return Err(format!("Cover index {index} does not exist"));
                }
                pictures.remove(index);
                let written = tag_policy::write(
                    staging,
                    &file,
                    &options.mappings,
                    options,
                    tag_policy::Change::Pictures(pictures.clone()),
                )?;
                return tag_policy::verify_pictures(
                    staged_path,
                    options,
                    written,
                    &pictures,
                    "The cover could not be removed",
                );
            }
            let mut remaining = index;
            let mut location = None;
            for tag in ordered_tags(&file) {
                if remaining < tag.pictures().len() {
                    location = Some((tag.tag_type(), remaining));
                    break;
                }
                remaining -= tag.pictures().len();
            }
            let (kind, local) =
                location.ok_or_else(|| format!("Cover index {index} does not exist"))?;
            if !file.tag_support(kind).is_writable() {
                return Err("This cover belongs to a read-only tag type".into());
            }
            let file_type = file.file_type();
            let tag = file.tag_mut(kind).ok_or("Cover tag disappeared")?;
            tag.remove_picture(local);
            save_preserving_native(
                staging,
                file_type,
                tag,
                &Fields::new(),
                true,
                Id3Write::preserving(options),
            )
        },
    )
}

#[cfg(test)]
mod diagnostic_producer_tests {
    use super::*;

    #[test]
    fn tag_reader_keeps_owned_unrecognized_guard_separate_from_real_io() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Help {error}; Title.bin");
        fs::write(&path, [0u8; 16]).unwrap();
        let options = TagOptions::default();
        let error = read_entry_summary_with_options_diagnostic(&path, &[], &options).unwrap_err();
        assert_eq!(
            error.to_string(),
            read_entry_summary_with_options(&path, &[], &options).unwrap_err()
        );
        let data = serde_json::to_value(error).unwrap();
        assert_eq!(data["kind"], "owned");
        assert_eq!(data["key"], "Unrecognized audio file type");
        fs::remove_file(&path).unwrap();
        let error = read_entry_with_options_diagnostic(&path, &[], &options).unwrap_err();
        assert_eq!(
            error.to_string(),
            read_entry_with_options(&path, &[], &options).unwrap_err()
        );
        assert_eq!(serde_json::to_value(error).unwrap()["kind"], "external");
    }

    #[test]
    fn tag_writer_owned_readonly_guard_preserves_legacy_bytes_and_message() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Help {error}.bin");
        fs::write(&path, b"original bytes").unwrap();
        let original_permissions = fs::metadata(&path).unwrap().permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        let updates = Fields::from([("TITLE".into(), vec!["new".into()])]);
        let options = TagOptions::default();
        let error = write_fields_with_options_diagnostic(&path, &updates, false, &[], &options)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            write_fields_with_options(&path, &updates, false, &[], &options).unwrap_err()
        );
        let data = serde_json::to_value(error).unwrap();
        assert_eq!(data["kind"], "owned");
        assert_eq!(data["key"], "The audio file is read-only");
        assert_eq!(fs::read(&path).unwrap(), b"original bytes");
        fs::set_permissions(&path, original_permissions).unwrap();
    }

    #[test]
    fn metadata_guard_preserves_authored_field_as_raw_param() {
        let name = "Help {error}\nTitle";
        let updates = Fields::from([(name.into(), vec!["value".into()])]);
        let error = normalized_updates_diagnostic(&updates, TagType::VorbisComments).unwrap_err();
        assert_eq!(
            error.to_string(),
            normalized_updates(&updates, TagType::VorbisComments).unwrap_err()
        );
        let data = serde_json::to_value(error).unwrap();
        assert_eq!(data["kind"], "owned");
        assert_eq!(data["params"]["name"], format!("{name:?}"));
    }
}
