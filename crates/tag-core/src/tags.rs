//! Portable metadata I/O. Every mutation is written and verified in a sibling
//! temporary file before the original is atomically replaced.
#[path = "asf/mod.rs"]
mod asf;
#[path = "container_formats.rs"]
mod container_formats;
#[path = "cover_ops.rs"]
mod cover_ops;
#[path = "mapping.rs"]
mod mapping;
#[path = "tag_policy.rs"]
mod tag_policy;
#[path = "technical.rs"]
mod technical;
use crate::preferences::TagOptions;
use base64::{engine::general_purpose::STANDARD, Engine};
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
    field_mapping, map_read_fields, map_write_fields, validate_user_mappings, FieldMapping,
    UserFieldMapping, FIELD_MAPPINGS,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};
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
fn strict_or_lenient_id3v1<T>(
    mut read: impl FnMut(ParsingMode) -> Result<T, String>,
) -> Result<T, String> {
    match read(ParsingMode::Strict) {
        Err(error) if empty_id3v1_year(&error) => read(ParsingMode::Relaxed),
        result => result,
    }
}

fn load(path: &Path) -> Result<TaggedFile, String> {
    let mut file = strict_or_lenient_id3v1(|mode| {
        Probe::open(path)
            .map_err(|e| e.to_string())?
            .guess_file_type()
            .map_err(|e| e.to_string())?
            .options(
                ParseOptions::new()
                    .implicit_conversions(false)
                    .parsing_mode(mode),
            )
            .read()
            .map_err(|e| error_message(&e))
    })?;
    if file.tag(TagType::Ape).is_some() {
        let native = native_tags(
            &mut fs::File::open(path).map_err(|e| e.to_string())?,
            file.file_type(),
        )?;
        if let (Some(ape), Some(tag)) = (native.ape, file.tag_mut(TagType::Ape)) {
            for item in &ape {
                if item.key().starts_with("Cover Art (") {
                    if let ItemValue::Binary(data) = item.value() {
                        tag.push_picture(
                            Picture::from_ape_bytes(item.key(), data).map_err(|e| e.to_string())?,
                        );
                    }
                }
            }
        }
    }
    Ok(file)
}

fn ordered_tags(file: &TaggedFile) -> Vec<&Tag> {
    let primary = file.primary_tag_type();
    file.tags()
        .iter()
        .filter(|t| t.tag_type() == primary)
        .chain(file.tags().iter().filter(|t| t.tag_type() != primary))
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
    ilst: Option<Ilst>,
    riff: Option<RiffInfoList>,
    technical: Fields,
}

// Lofty 0.25 generic conversions discard unknown APE and Vorbis items. Keep
// their concrete tags, including flags, vendor and binary items, separately.
fn native_tags(reader: &mut fs::File, kind: FileType) -> Result<NativeTags, String> {
    strict_or_lenient_id3v1(|mode| native_tags_with(reader, kind, mode))
}
fn native_tags_with(
    reader: &mut fs::File,
    kind: FileType,
    mode: ParsingMode,
) -> Result<NativeTags, String> {
    reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let options = ParseOptions::new()
        .implicit_conversions(false)
        .read_properties(false)
        .parsing_mode(mode);
    let mut native = NativeTags::default();
    match kind {
        FileType::Flac => {
            let file = FlacFile::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.id3 = file.id3v2().cloned();
            let mut tag = file.vorbis_comments().cloned().unwrap_or_default();
            for (picture, info) in file.pictures() {
                tag.insert_picture(picture.clone(), Some(*info))
                    .map_err(|e| e.to_string())?;
            }
            native.vorbis = Some(tag);
        }
        FileType::Vorbis => {
            native.vorbis = Some(
                VorbisFile::read_from(reader, options)
                    .map_err(|e| error_message(&e))?
                    .vorbis_comments()
                    .clone(),
            )
        }
        FileType::Opus => {
            native.vorbis = Some(
                OpusFile::read_from(reader, options)
                    .map_err(|e| error_message(&e))?
                    .vorbis_comments()
                    .clone(),
            )
        }
        FileType::Speex => {
            native.vorbis = Some(
                SpeexFile::read_from(reader, options)
                    .map_err(|e| error_message(&e))?
                    .vorbis_comments()
                    .clone(),
            )
        }
        FileType::Mpeg => {
            let file = MpegFile::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.ape = file.ape().cloned();
            native.id3 = file.id3v2().cloned();
        }
        FileType::Ape => {
            let file = ApeFile::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.ape = file.ape().cloned();
            native.id3 = file.id3v2().cloned();
        }
        FileType::Mpc => {
            let file = MpcFile::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.ape = file.ape().cloned();
            native.id3 = file.id3v2().cloned();
        }
        FileType::WavPack => {
            native.ape = WavPackFile::read_from(reader, options)
                .map_err(|e| error_message(&e))?
                .ape()
                .cloned()
        }
        FileType::Aac => {
            native.id3 = AacFile::read_from(reader, options)
                .map_err(|e| error_message(&e))?
                .id3v2()
                .cloned()
        }
        FileType::Wav => {
            let file = WavFile::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.id3 = file.id3v2().cloned();
            native.riff = file.riff_info().cloned();
        }
        FileType::Aiff => {
            let file = AiffFile::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.id3 = file.id3v2().cloned();
        }
        FileType::Mp4 => {
            let file = Mp4File::read_from(reader, options).map_err(|e| error_message(&e))?;
            native.ilst = file.ilst().cloned();
        }
        _ => {}
    }
    Ok(native)
}

/// Logical fields of one tag, using its concrete native representation.
fn tag_fields(
    tag: &Tag,
    native: &NativeTags,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<Fields, String> {
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
    map_read_fields(&local, tag.tag_type(), rules)
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
    }
}

fn describe_with_native(
    path: &Path,
    file: &TaggedFile,
    native: &NativeTags,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let mut fields = Fields::new();
    let mut covers = Vec::new();
    let displayed = displayed_tags(file, options);
    for tag in &displayed {
        let local = tag_fields(tag, native, rules, options)?;
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
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let read = displayed.first().map(|tag| tag.tag_type());
    let technical = technical::collect(path, file, native, &covers, &metadata, read);
    Ok(AudioEntry {
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
    })
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
    read_description(path, true, rules, options)
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
    read_description(path, false, rules, options)
}
fn read_description(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    validate_user_mappings(rules)?;
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    // Before lofty: its MPEG sync search can match bytes inside ASF artwork.
    if asf::sniff(&path) {
        return asf::describe(&path, include_cover_data, rules);
    }
    if let Some(kind) = container_formats::detect(&path) {
        return container_formats::read(&path, kind, include_cover_data, rules, options);
    }
    let probe = Probe::open(&path)
        .map_err(|e| e.to_string())?
        .guess_file_type()
        .map_err(|e| e.to_string())?;
    let kind = probe.file_type().ok_or("Unrecognized audio file type")?;
    let parse_options = ParseOptions::new()
        .implicit_conversions(false)
        .parsing_mode(ParsingMode::Strict);
    let mut reader = probe.into_inner();
    macro_rules! read {
        ($ty:ty,$snapshot:expr) => {{
            let media = strict_or_lenient_id3v1(|mode| {
                reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
                let mut options = parse_options;
                <$ty>::read_from(&mut reader, options.parsing_mode(mode))
                    .map_err(|e| error_message(&e))
            })?;
            let native: NativeTags = ($snapshot)(&media);
            (TaggedFile::from(media), native)
        }};
    }
    let (mut file, native) = match kind {
        FileType::Flac => read!(FlacFile, |f: &FlacFile| NativeTags {
            vorbis: f.vorbis_comments().cloned(),
            id3: f.id3v2().cloned(),
            ..Default::default()
        }),
        FileType::Mpeg => read!(MpegFile, |f: &MpegFile| NativeTags {
            id3: f.id3v2().cloned(),
            ape: f.ape().cloned(),
            technical: technical::mpeg(f.properties()),
            ..Default::default()
        }),
        FileType::Mp4 => read!(Mp4File, |f: &Mp4File| NativeTags {
            ilst: f.ilst().cloned(),
            technical: technical::mp4(f.properties()),
            ..Default::default()
        }),
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
            let file = load(&path)?;
            let native = native_tags(reader.get_mut(), kind)?;
            (file, native)
        }
    };
    // Generic APE conversion intentionally omits cover binary items. Enrich
    // from the already parsed native tag, without a second file read.
    if let (Some(ape), Some(tag)) = (native.ape.as_ref(), file.tag_mut(TagType::Ape)) {
        for item in ape {
            if item.key().starts_with("Cover Art (") {
                if let ItemValue::Binary(data) = item.value() {
                    tag.push_picture(
                        Picture::from_ape_bytes(item.key(), data).map_err(|e| e.to_string())?,
                    );
                }
            }
        }
    }
    describe_with_native(&path, &file, &native, include_cover_data, rules, options)
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
            | "ogg"
            | "oga"
            | "opus"
            | "spx"
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
/// Like [`scan_paths_with_options`], reading files in parallel while reporting
/// progress. After a cancellation the files read so far are returned.
pub fn scan_paths_with_progress(
    paths: &[String],
    recursive: bool,
    options: &TagOptions,
    extensions: Option<&[String]>,
    progress: &dyn crate::parallel::Progress,
) -> ScanResult {
    let mut result = ScanResult::default();
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for raw in paths {
        let path = PathBuf::from(raw);
        match fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => {
                let walk = WalkDir::new(&path).follow_links(false).sort_by_file_name();
                let walk = if recursive { walk } else { walk.max_depth(1) };
                for item in walk {
                    match item {
                        Ok(item) if item.file_type().is_file() || item.file_type().is_symlink() => {
                            if audio_extension(item.path(), extensions) {
                                candidates.push(item.into_path());
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
            Ok(_) => candidates.push(path),
            Err(e) => result.errors.push(FileError {
                path: raw.clone(),
                error: e.to_string(),
            }),
        }
    }
    let mut unique = Vec::new();
    for path in candidates {
        match path.canonicalize() {
            Ok(canonical) => {
                if seen.insert(canonical.clone()) {
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
    let read = crate::parallel::map_ordered(&unique, progress, |canonical| {
        read_entry_summary_with_options(canonical, &[], options)
    });
    for (canonical, outcome) in unique.iter().zip(read) {
        match outcome {
            Some(Ok(entry)) => result.entries.push(entry),
            Some(Err(error)) => result.errors.push(FileError {
                path: canonical.to_string_lossy().into_owned(),
                error,
            }),
            None => {}
        }
    }
    result
}

/// Copy `source` to a new temporary file in `dir`. The copy is created at a
/// fresh path rather than into an already opened temporary file, so
/// copy-on-write filesystems clone it (APFS `clonefile`, reflinks through
/// `copy_file_range`) instead of duplicating every audio byte; elsewhere it is
/// an ordinary copy. The file is removed when the handle is dropped unpersisted.
pub fn staging_copy(
    source: &Path,
    dir: &Path,
    prefix: &str,
    suffix: &str,
) -> Result<tempfile::NamedTempFile, String> {
    tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .rand_bytes(12)
        .make_in(dir, |path| {
            if let Err(error) = fs::copy(source, path) {
                let _ = fs::remove_file(path);
                return Err(error);
            }
            fs::OpenOptions::new().read(true).write(true).open(path)
        })
        .map_err(|e| e.to_string())
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
    staged_mutation_with_mappings(path, include_cover_data, &[], options, mutation)
}

fn staged_mutation_with_mappings<F>(
    path: &Path,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    mutation: F,
) -> Result<AudioEntry, String>
where
    F: FnOnce(&mut fs::File, TaggedFile, &Path) -> Result<(), String>,
{
    staged_write(
        path,
        include_cover_data,
        rules,
        options,
        |staging, original, staged_path| mutation(staging, load(original)?, staged_path),
    )
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
    validate_user_mappings(rules)?;
    options.validate()?;
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let original = fs::metadata(&path).map_err(|e| e.to_string())?;
    if !original.is_file() {
        return Err("The audio path must be a regular file".into());
    }
    if original.permissions().readonly() {
        return Err("The audio file is read-only".into());
    }
    let mut staging = staging_copy(
        &path,
        path.parent().ok_or("Audio path has no parent directory")?,
        ".opentag-",
        "",
    )?;
    let staged_path = staging.path().to_path_buf();
    mutation(staging.as_file_mut(), &path, &staged_path)?;
    if options.preserve_modified_time {
        if let Ok(modified) = original.modified() {
            staging
                .as_file()
                .set_modified(modified)
                .map_err(|e| e.to_string())?;
        }
    }
    staging.as_file().sync_all().map_err(|e| e.to_string())?;
    let mut entry = read_description(staging.path(), include_cover_data, rules, options)?;
    fs::set_permissions(staging.path(), original.permissions()).map_err(|e| e.to_string())?;
    let current = fs::metadata(&path).map_err(|e| e.to_string())?;
    if current.len() != original.len() || current.modified().ok() != original.modified().ok() {
        return Err("The file changed during editing; reload it before saving".into());
    }
    staging.persist(&path).map_err(|e| e.error.to_string())?;
    entry.path = path.to_string_lossy().into_owned();
    entry.filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    technical::path_fields(&mut entry.technical, &path);
    Ok(entry)
}

fn normalized_updates(updates: &Fields, kind: TagType) -> Result<Fields, String> {
    let mut output = Fields::new();
    let mut seen = HashSet::new();
    for (name, values) in updates {
        if name.trim().is_empty()
            || name.contains(['\0', '\n', '\r'])
            || (kind == TagType::VorbisComments && name.contains('='))
        {
            return Err(format!("Invalid metadata field name: {name:?}"));
        }
        if values.iter().any(|v| v.contains('\0')) {
            return Err(format!("{name}: values cannot contain NUL characters"));
        }
        let name = if matches!(kind, TagType::VorbisComments | TagType::Ape) {
            name.clone()
        } else {
            field_mapping(name)
                .map(|m| m.field.to_owned())
                .unwrap_or_else(|| name.clone())
        };
        if !seen.insert(name.to_lowercase()) {
            return Err(format!("Duplicate metadata field: {name}"));
        }
        output.insert(name, values.clone());
    }
    Ok(output)
}

/// How a write treats an ID3v2 tag: keep its revision (non-MPEG formats) or
/// convert it to the configured MPEG revision and encoding.
#[derive(Clone, Copy)]
enum Id3Write {
    Preserve([u8; 3]),
    Convert(mapping::Id3Target),
}
impl Id3Write {
    fn preserving(options: &TagOptions) -> Self {
        Self::Preserve(options.comment_language())
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
    let originals = native_tags(staging, file_type)?;
    match tag.tag_type() {
        TagType::Id3v2 => {
            let mut native = originals.id3.unwrap_or_default();
            let target = match id3 {
                Id3Write::Preserve(language) => mapping::Id3Target::preserving(&native, language),
                Id3Write::Convert(target) => {
                    mapping::convert_id3_version(&mut native, &target)?;
                    target
                }
            };
            mapping::patch_id3(&mut native, updates, &target)?;
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
                .map_err(|e| e.to_string())?;
            mapping::save_id3(staging, file_type, &native, &target)
        }
        TagType::Mp4Ilst => {
            let mut native = originals.ilst.unwrap_or_default();
            mapping::patch_mp4(&mut native, updates)?;
            if covers_changed {
                native.remove_pictures();
                for picture in tag.pictures() {
                    native.insert_picture(picture.clone());
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))
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
                        .map_err(|e| e.to_string())?;
                }
            }
            if file_type == FileType::Flac {
                return crate::flac_metadata::save(staging, &native);
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))
        }
        TagType::Ape => {
            let mut native = originals.ape.unwrap_or_default();
            for (name, values) in updates {
                native.remove(name);
                if !values.is_empty() {
                    native.insert(
                        ApeItem::new(name.clone(), ItemValue::Text(values.join("\0")))
                            .map_err(|e| e.to_string())?,
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
                        .ok_or("APE does not support this picture type")?
                        .to_owned();
                    native.insert(
                        ApeItem::new(key, ItemValue::Binary(picture.as_ape_bytes()))
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))
        }
        TagType::RiffInfo => {
            let mut native = originals.riff.unwrap_or_default();
            for (name, values) in updates {
                let key = field_mapping(name)
                    .map(|r| r.riff)
                    .filter(|s| !s.is_empty())
                    .or_else(|| (name.len() == 4).then_some(name.as_str()));
                let Some(key) = key else {
                    continue;
                };
                if values.len() > 1 {
                    return Err(format!("RIFF {name} supports one text value"));
                }
                native.remove(key);
                if let Some(value) = values.first() {
                    native.insert(key.to_owned(), value.clone());
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            native
                .save_to(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))
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
                .map_err(|e| e.to_string())?;
            tag.save_to(staging, WriteOptions::default())
                .map_err(|e| error_message(&e))
        }
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
    write_fields_result(path, updates, include_cover_data, rules, options)
}

/// Compare a written tag's read-back fields with the requested native values.
fn verify_readback(
    actual: &Fields,
    native_updates: &Fields,
    kind: TagType,
    latin1: bool,
) -> Result<(), String> {
    for (key, values) in native_updates {
        let (read_key, mut expected) = if kind == TagType::Id3v2 {
            mapping::expected_id3_readback(key, values)?
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
                return Err(format!("The format could not remove field {key}"));
            }
        } else if kind == TagType::Mp4Ilst
            && matches!(
                key.as_str(),
                "TRACK" | "DISCNUMBER" | "TOTALTRACKS" | "TOTALDISCS"
            )
        {
            let requested = values.first().ok_or("Missing number")?;
            let found = found
                .and_then(|v| v.first())
                .ok_or_else(|| format!("The format lost field {key}"))?;
            let expected_number = requested
                .split('/')
                .next()
                .unwrap_or_default()
                .parse::<u16>()
                .map_err(|_| format!("{key} requires an unsigned number"))?;
            let actual_number = found
                .split('/')
                .next()
                .unwrap_or_default()
                .parse::<u16>()
                .map_err(|_| format!("The format returned an invalid {key}"))?;
            if expected_number != actual_number {
                return Err(format!("The format changed field {key}"));
            }
            if let Some((_, total)) = requested.split_once('/') {
                if found.split_once('/').map(|(_, t)| t.parse::<u16>().ok())
                    != Some(total.parse::<u16>().ok())
                {
                    return Err(format!("The format changed the total in {key}"));
                }
            }
        } else if found != Some(&expected) {
            return Err(format!(
                "The format could not preserve all values for field {key}"
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
    if updates.is_empty() {
        return read_description(path, include_cover_data, rules, options);
    }
    if asf::sniff(path) {
        return asf::write_fields(path, updates, include_cover_data, rules, options);
    }
    if let Some(kind) = container_formats::detect(path) {
        return container_formats::write_fields(
            path,
            kind,
            updates,
            include_cover_data,
            rules,
            options,
        );
    }
    staged_mutation_with_mappings(
        path,
        include_cover_data,
        rules,
        options,
        |staging, mut file, staged_path| {
            if tag_policy::policy(file.file_type()).is_some() {
                let updates = normalized_updates(updates, TagType::Ape)?;
                let written = tag_policy::write(
                    staging,
                    &file,
                    rules,
                    options,
                    tag_policy::Change::Fields(&updates),
                )?;
                return tag_policy::verify_fields(staged_path, rules, options, written, &updates);
            }
            let primary = file.primary_tag_type();
            let updates = normalized_updates(updates, primary)?;
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
                let tag = file.remove(kind).ok_or("Tag disappeared during editing")?;
                let native_updates =
                    normalized_updates(&map_write_fields(&updates, kind, rules)?, kind)?;
                save_preserving_native(
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
                .map_err(|e| e.to_string())?;
            let actual = read_entry_summary_with_options(staged_path, &[], options)?.fields;
            let native_updates =
                normalized_updates(&map_write_fields(&updates, primary, rules)?, primary)?;
            verify_readback(&actual, &native_updates, primary, false)
        },
    )
}

pub fn remove_tags(path: &Path) -> Result<AudioEntry, String> {
    remove_tags_result(path, true, &TagOptions::default())
}

pub fn remove_tags_summary(path: &Path) -> Result<AudioEntry, String> {
    remove_tags_result(path, false, &TagOptions::default())
}

/// Remove every tag kind. The options control the returned description and
/// whether the modification time is preserved.
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
    staged_mutation(path, include_cover_data, options, |staging, file, _| {
        for tag in file.tags() {
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            tag.tag_type()
                .remove_from(staging, WriteOptions::default())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
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
    let kind = picture_type
        .map(picture_type_from_name)
        .transpose()?
        .unwrap_or(PictureType::CoverFront);
    if delete_existing {
        let mut reader = fs::File::open(image).map_err(|e| e.to_string())?;
        Picture::from_reader(&mut reader).map_err(|e| e.to_string())?;
        remove_all_covers_with_options(path, options)?;
    }
    add_cover_result(path, image, kind, false, options)
}

/// ID3 requires picture descriptions to identify repeated cover types.
fn describe_repeated_cover(pictures: &[Picture], picture: &mut Picture, image: &Path) {
    if !pictures.iter().any(|p| {
        p.pic_type() == picture.pic_type() && p.description().unwrap_or_default().is_empty()
    }) {
        return;
    }
    let basename = image.file_name().unwrap_or_default().to_string_lossy();
    let mut description = basename.to_string();
    let mut suffix = 2;
    while pictures
        .iter()
        .any(|p| p.description() == Some(description.as_str()))
    {
        description = format!("{basename} ({suffix})");
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
    let mut image_file = fs::File::open(image).map_err(|e| e.to_string())?;
    let mut picture = Picture::from_reader(&mut image_file).map_err(|e| e.to_string())?;
    picture.set_pic_type(kind);
    if asf::sniff(path) {
        return asf::change_pictures(
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
        return container_formats::add_cover(
            path,
            container,
            picture,
            image,
            include_cover_data,
            options,
        );
    }
    staged_mutation(
        path,
        include_cover_data,
        options,
        |staging, mut file, staged_path| {
            if tag_policy::policy(file.file_type()).is_some() {
                let mut pictures = tag_policy::displayed_pictures(&file, options);
                describe_repeated_cover(&pictures, &mut picture, image);
                pictures.push(picture);
                let written = tag_policy::write(
                    staging,
                    &file,
                    &[],
                    options,
                    tag_policy::Change::Pictures(pictures.clone()),
                )?;
                return tag_policy::verify_pictures(
                    staged_path,
                    options,
                    written,
                    &pictures,
                    "This tag format cannot append another cover with the same picture type",
                );
            }
            let primary = file.primary_tag_type();
            let previous_covers: usize = file.tags().iter().map(|tag| tag.pictures().len()).sum();
            if file.primary_tag().is_none() {
                file.insert_tag(Tag::new(primary));
            }
            let file_type = file.file_type();
            let tag = file
                .primary_tag_mut()
                .ok_or("The audio format has no writable tag")?;
            if primary == TagType::Id3v2 {
                describe_repeated_cover(tag.pictures(), &mut picture, image);
            }
            tag.push_picture(picture);
            save_preserving_native(
                staging,
                file_type,
                tag,
                &Fields::new(),
                true,
                Id3Write::preserving(options),
            )?;
            if read_entry_summary_with_options(staged_path, &[], options)?
                .covers
                .len()
                != previous_covers + 1
            {
                return Err(
                    "This tag format cannot append another cover with the same picture type".into(),
                );
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
                    &[],
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
