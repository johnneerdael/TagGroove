//! Read-only format placeholders. Missing evidence stays absent; raw stored
//! metadata is never populated with these values.
use super::{CoverInfo, Fields, NativeTags};
use chrono::{DateTime, Local, Locale, Utc};
use lofty::{
    file::{AudioFile, FileType, TaggedFile, TaggedFileExt},
    id3::v2::{Frame, Id3v2Tag, Id3v2Version},
    iff::{
        aiff::{AiffCompressionType, AiffProperties},
        wav::{WavFormat, WavProperties},
    },
    mp4::Mp4Properties,
    mpeg::{ChannelMode, Layer, MpegProperties, MpegVersion},
    picture::{Picture, PictureInformation},
    tag::TagType,
    TextEncoding,
};
use std::{
    fs::Metadata,
    path::{Path, MAIN_SEPARATOR},
    sync::OnceLock,
    time::{Duration, SystemTime},
};

fn put(fields: &mut Fields, name: &str, value: impl ToString) {
    fields.insert(name.into(), vec![value.to_string()]);
}

pub(super) fn picture_information(picture: &Picture) -> Option<PictureInformation> {
    let bytes = picture.data();
    if bytes.starts_with(&[255, 216, 255]) && !bounded_jpeg_header(bytes) {
        return None;
    }
    PictureInformation::from_picture(picture).ok()
}

fn bounded_jpeg_header(bytes: &[u8]) -> bool {
    // Lofty 0.25.4 subtracts two from unchecked JPEG segment lengths and
    // multiplies precision/components in u8. Validate those inputs before its
    // otherwise allocation-free header inspection; malformed art stays opaque.
    let mut position = 2usize;
    while position < bytes.len() {
        if bytes.get(position) != Some(&255) {
            return false;
        }
        while bytes.get(position) == Some(&255) {
            position += 1;
        }
        let Some(&marker) = bytes.get(position) else {
            return false;
        };
        position += 1;
        if marker == 217 {
            return true;
        }
        if matches!(marker, 1 | 208..=216) {
            continue;
        }
        let Some(size) = bytes.get(position..position + 2) else {
            return false;
        };
        let length = usize::from(u16::from_be_bytes([size[0], size[1]]));
        if length < 2 {
            return false;
        }
        let Some(end) = position
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
        else {
            return false;
        };
        if matches!(marker, 192 | 194)
            && (length < 8
                || bytes[position + 2]
                    .checked_mul(bytes[position + 7])
                    .is_none())
        {
            return false;
        }
        if marker == 218 {
            return true;
        }
        position = end;
    }
    true
}

pub(super) fn mpeg(properties: &MpegProperties) -> Fields {
    let mut fields = Fields::new();
    let version = match properties.version() {
        MpegVersion::V1 => "1",
        MpegVersion::V2 => "2",
        MpegVersion::V2_5 => "2.5",
        MpegVersion::V4 => "4",
    };
    let layer = match properties.layer() {
        Layer::Layer1 => "I",
        Layer::Layer2 => "II",
        Layer::Layer3 => "III",
    };
    put(
        &mut fields,
        "_CODEC",
        format!("MPEG-{version} Layer {layer}"),
    );
    put(
        &mut fields,
        "_MODE",
        match properties.channel_mode() {
            ChannelMode::Stereo => "Stereo",
            ChannelMode::JointStereo => "Joint Stereo",
            ChannelMode::DualChannel => "Dual Channel",
            ChannelMode::SingleChannel => "Mono",
        },
    );
    fields
}
pub(super) fn mp4(properties: &Mp4Properties) -> Fields {
    let mut fields = Fields::new();
    if let Some(codec) = properties.codec() {
        put(&mut fields, "_CODEC", format!("{codec:?}"));
    }
    fields
}
pub(super) fn wav(properties: &WavProperties) -> Fields {
    let mut fields = Fields::new();
    match properties.format() {
        WavFormat::PCM => put(&mut fields, "_CODEC", "PCM"),
        WavFormat::IEEE_FLOAT => put(&mut fields, "_CODEC", "IEEE Float"),
        WavFormat::Other(code) => put(&mut fields, "_CODEC", format!("WAVE format 0x{code:04X}")),
    }
    fields
}
pub(super) fn aiff(properties: &AiffProperties) -> Fields {
    let mut fields = Fields::new();
    let codec = match properties.compression_type() {
        None | Some(AiffCompressionType::None | AiffCompressionType::sowt) => "PCM".into(),
        Some(kind) => kind.compression_name().into_owned(),
    };
    if !codec.is_empty() {
        put(&mut fields, "_CODEC", codec);
    }
    fields
}

fn locale() -> Locale {
    static LOCALE: OnceLock<Locale> = OnceLock::new();
    *LOCALE.get_or_init(|| {
        sys_locale::get_locale()
            .and_then(|name| {
                let name = name.split('.').next().unwrap_or(&name).replace('-', "_");
                Locale::try_from(name.as_str()).ok()
            })
            .unwrap_or(Locale::en_US)
    })
}
fn timestamp(fields: &mut Fields, prefix: &str, time: SystemTime) {
    // Provider timestamps may exceed Chrono's representable calendar range.
    // Unlike From<SystemTime>, this checked conversion leaves unknown fields absent.
    let (seconds, nanos) = match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => {
            let Ok(seconds) = i64::try_from(duration.as_secs()) else {
                return;
            };
            (seconds, duration.subsec_nanos())
        }
        Err(before_epoch) => {
            let duration = before_epoch.duration();
            let Ok(seconds) = i64::try_from(duration.as_secs()) else {
                return;
            };
            if duration.subsec_nanos() == 0 {
                (-seconds, 0)
            } else {
                (-seconds - 1, 1_000_000_000 - duration.subsec_nanos())
            }
        }
    };
    let Some(utc) = DateTime::<Utc>::from_timestamp(seconds, nanos) else {
        return;
    };
    let local: DateTime<Local> = utc.with_timezone(&Local);
    put(fields, &format!("{prefix}_DATETIME_RAW"), utc.timestamp());
    put(
        fields,
        &format!("{prefix}_DATE"),
        local.format_localized("%x", locale()),
    );
    put(
        fields,
        &format!("{prefix}_DATETIME"),
        local.format_localized("%x %X", locale()),
    );
}

pub(super) fn file_time_fields(
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
) -> Fields {
    let mut fields = Fields::new();
    if let Some(time) = modified {
        timestamp(&mut fields, "_FILE_MOD", time);
    }
    if let Some(time) = created {
        timestamp(&mut fields, "_FILE_CREATE", time);
    }
    fields
}

/// Also refresh this after staging is persisted, so temporary filenames never
/// escape in mutation results. No additional audio parsing is needed.
pub(super) fn path_fields(fields: &mut Fields, path: &Path) {
    let parent = path.parent().unwrap_or(Path::new(""));
    for (name, value) in [
        ("_PATH", path.to_string_lossy().into_owned()),
        (
            "_FILENAME",
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "_FILENAME_EXT",
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "_EXTENSION",
            path.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "_FOLDERPATH",
            format!(
                "{}{}",
                parent.display(),
                if parent.as_os_str().is_empty()
                    || parent
                        .as_os_str()
                        .to_string_lossy()
                        .ends_with(MAIN_SEPARATOR)
                {
                    ""
                } else {
                    std::path::MAIN_SEPARATOR_STR
                }
            ),
        ),
        (
            "_DIRECTORY",
            parent
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "_PARENT_DIRECTORY",
            parent
                .parent()
                .and_then(Path::file_name)
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
    ] {
        put(fields, name, value);
    }
}
/// Display name of an ID3v2 tag, including its revision.
pub(super) fn id3_name(tag: &Id3v2Tag) -> &'static str {
    match tag.original_version() {
        Id3v2Version::V2 => "ID3v2.2",
        Id3v2Version::V3 => "ID3v2.3",
        Id3v2Version::V4 => "ID3v2.4",
    }
}
fn tag_name(kind: TagType, native: &NativeTags) -> String {
    match kind {
        TagType::Id3v2 => native.id3.as_ref().map(id3_name).unwrap_or("ID3v2").into(),
        TagType::Id3v1 => "ID3v1".into(),
        TagType::Ape => "APE".into(),
        TagType::VorbisComments => "VorbisComment".into(),
        TagType::Mp4Ilst => "MP4".into(),
        TagType::RiffInfo => "RIFF INFO".into(),
        TagType::AiffText => "AIFF Text".into(),
        other => format!("{other:?}"),
    }
}
fn picture_type(kind: &str) -> &str {
    match kind {
        "CoverFront" => "Front Cover",
        "CoverBack" => "Back Cover",
        "Other" => "Other",
        "Icon" => "File Icon",
        "OtherIcon" => "Other File Icon",
        "Leaflet" => "Leaflet Page",
        "Media" => "Media",
        "LeadArtist" => "Lead Artist",
        "Artist" => "Artist",
        "Conductor" => "Conductor",
        "Band" => "Band/Orchestra",
        "Composer" => "Composer",
        "Lyricist" => "Lyricist",
        "RecordingLocation" => "Recording Location",
        "DuringRecording" => "During Recording",
        "DuringPerformance" => "During Performance",
        "ScreenCapture" => "Screen Capture",
        "BrightFish" => "A Bright Coloured Fish",
        "Illustration" => "Illustration",
        "BandLogo" => "Band/Artist Logo",
        "PublisherLogo" => "Publisher/Studio Logo",
        unknown => unknown,
    }
}
pub(super) fn collect(
    path: &Path,
    file: &TaggedFile,
    native: &NativeTags,
    covers: &[CoverInfo],
    metadata: &Metadata,
    read: Option<TagType>,
) -> Fields {
    let mut fields = native.technical.clone();
    let properties = file.properties();
    let codec = match file.file_type() {
        FileType::Flac => Some("FLAC"),
        FileType::Vorbis => Some("Vorbis"),
        FileType::Opus => Some("Opus"),
        FileType::Speex => Some("Speex"),
        FileType::Ape => Some("Monkey's Audio"),
        FileType::WavPack => Some("WavPack"),
        FileType::Mpc => Some("Musepack"),
        FileType::Aac => Some("AAC"),
        _ => None,
    };
    if let Some(codec) = codec {
        put(&mut fields, "_CODEC", codec);
    }
    if let Some(vendor) = native
        .vorbis
        .as_ref()
        .map(|t| t.vendor())
        .filter(|v| !v.is_empty())
    {
        put(&mut fields, "_TOOL", vendor);
    }
    stream_fields(
        &mut fields,
        properties.bit_depth(),
        properties.audio_bitrate(),
        properties.sample_rate(),
        properties.channels(),
        properties.duration(),
    );
    let tags = super::ordered_tags(file)
        .iter()
        .map(|tag| tag_name(tag.tag_type(), native))
        .collect::<Vec<_>>();
    if !tags.is_empty() {
        put(&mut fields, "_TAG", tags.join(" "));
    }
    if let Some(kind) = read {
        put(&mut fields, "_TAG_READ", tag_name(kind, native));
    }
    if let Some(tag) = native.id3.as_ref() {
        id3_details(&mut fields, tag);
    }
    file_fields(&mut fields, covers, metadata, path);
    fields
}

/// Cover, size, timestamp and path fields shared by every container.
pub(super) fn file_fields(
    fields: &mut Fields,
    covers: &[CoverInfo],
    metadata: &Metadata,
    path: &Path,
) {
    put(fields, "_COVERS", covers.len());
    if let Some(cover) = covers.first() {
        put(fields, "_COVER_DESCRIPTION", &cover.description);
        put(fields, "_COVER_MIMETYPE", &cover.mime);
        put(fields, "_COVER_TYPE", picture_type(&cover.picture_type));
        put(fields, "_COVER_SIZE", cover.size);
        if let Some(width) = cover.width {
            put(fields, "_COVER_WIDTH", width);
        }
        if let Some(height) = cover.height {
            put(fields, "_COVER_HEIGHT", height);
        }
    }
    let size = metadata.len();
    put(fields, "_FILE_SIZE_BYTES", size);
    put(fields, "_FILE_SIZE_KB", size / 1024);
    put(
        fields,
        "_FILE_SIZE_MB",
        format!("{:.2}", size as f64 / (1024. * 1024.)),
    );
    put(
        fields,
        "_FILE_SIZE",
        if size < 1024 {
            format!("{size} B")
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.)
        } else {
            format!("{:.1} MB", size as f64 / (1024. * 1024.))
        },
    );
    fields.extend(file_time_fields(
        metadata.created().ok(),
        metadata.modified().ok(),
    ));
    path_fields(fields, path);
}

/// Audio stream properties shared by every container.
pub(super) fn stream_fields(
    fields: &mut Fields,
    bits: Option<u8>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    channels: Option<u8>,
    duration: Duration,
) {
    if let Some(bits) = bits.filter(|n| *n > 0) {
        put(fields, "_BITSPERSAMPLE", bits);
    }
    if let Some(rate) = bitrate.filter(|n| *n > 0) {
        put(fields, "_BITRATE", rate);
    }
    if let Some(rate) = sample_rate.filter(|n| *n > 0) {
        put(fields, "_SAMPLERATE", rate);
    }
    if !fields.contains_key("_MODE") {
        if let Some(channels) = channels.filter(|c| *c > 0) {
            put(
                fields,
                "_MODE",
                match channels {
                    1 => "Mono".into(),
                    2 => "Stereo".into(),
                    n => format!("{n} channels"),
                },
            );
        }
    }
    let seconds = duration.as_secs();
    put(
        fields,
        "_LENGTH_SECONDS",
        duration.as_secs_f64().round() as u64,
    );
    put(
        fields,
        "_LENGTH",
        format!("{}:{:02}", seconds / 60, seconds % 60),
    );
}

/// Text encodings and opaque frame identifiers found in an ID3v2 tag.
pub(super) fn id3_details(fields: &mut Fields, tag: &Id3v2Tag) {
    let mut encodings = Vec::new();
    let mut opaque = Vec::new();
    for frame in tag {
        let encoding = match frame {
            Frame::Text(t) => Some(t.encoding),
            Frame::UserText(t) => Some(t.encoding),
            Frame::UserUrl(t) => Some(t.encoding),
            Frame::Comment(t) => Some(t.encoding),
            Frame::UnsynchronizedText(t) => Some(t.encoding),
            Frame::KeyValue(t) => Some(t.encoding),
            Frame::Timestamp(t) => Some(t.encoding),
            Frame::Ownership(t) => Some(t.encoding),
            Frame::Binary(binary) if binary.id().as_str() == "IPLS" => match binary.data.first() {
                Some(0) => Some(TextEncoding::Latin1),
                Some(1) => Some(TextEncoding::UTF16),
                Some(2) => Some(TextEncoding::UTF16BE),
                Some(3) => Some(TextEncoding::UTF8),
                _ => None,
            },
            _ => None,
        };
        if let Some(encoding) = encoding {
            let label = match encoding {
                TextEncoding::Latin1 => "ISO-8859-1",
                TextEncoding::UTF16 => "UTF-16",
                TextEncoding::UTF16BE => "UTF-16BE",
                TextEncoding::UTF8 => "UTF-8",
            };
            if !encodings.contains(&label) {
                encodings.push(label);
            }
        }
        if let Frame::Binary(binary) = frame {
            let id = binary.id().as_str();
            if id != "IPLS" && id != "PCST" && !opaque.contains(&id) {
                opaque.push(id);
            }
        }
    }
    encodings.sort_unstable();
    opaque.sort_unstable();
    if !encodings.is_empty() {
        put(fields, "_ID3V2_CHARACTER_ENCODING", encodings.join(", "));
    }
    if !opaque.is_empty() {
        put(fields, "_ID3V2_UNKNOWN_FRAMES", opaque.join(", "));
    }
}
