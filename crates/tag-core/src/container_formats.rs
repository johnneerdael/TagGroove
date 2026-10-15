//! Containers that lofty does not parse: DSF and DSDIFF (an ID3v2 tag plus
//! DSDIFF edited-master text) and Matroska/WebM (EBML Tags, chapters and
//! image attachments). Every mutation uses the shared sibling staging, then
//! checks the read-back metadata and that the media payload is byte-identical.
//! TTA, TAK, OptimFROG, AAC (ADTS) and WavPack streams are detected here too;
//! their MP3-style tags go through the MPEG and APE tag policies (see
//! `raw_streams`).
use super::*;
use std::{
    io::{Read, Write},
    ops::Range,
    time::Duration,
};
#[path = "dsd.rs"]
mod dsd;
#[path = "matroska.rs"]
mod matroska;
#[path = "raw_streams.rs"]
pub(super) mod raw_streams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Dsf,
    Dff,
    Matroska,
}

/// Identify a container by its magic bytes; extensions are not trusted.
pub(super) fn detect(path: &Path) -> Option<Kind> {
    let mut header = Vec::with_capacity(16);
    fs::File::open(path)
        .ok()?
        .take(16)
        .read_to_end(&mut header)
        .ok()?;
    if header.starts_with(b"DSD ") {
        Some(Kind::Dsf)
    } else if header.starts_with(b"FRM8") && header.get(12..16) == Some(b"DSD ") {
        Some(Kind::Dff)
    } else if header.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        Some(Kind::Matroska)
    } else {
        None
    }
}

/// Size of the ID3v2 tag whose 10-byte header is `header`, footer included.
pub(super) fn id3v2_size(header: &[u8]) -> Option<u64> {
    let [b'I', b'D', b'3', major, minor, flags, size @ ..] = header.get(..10)? else {
        return None;
    };
    if *major == 0xFF || *minor == 0xFF || size.iter().any(|b| *b >= 0x80) {
        return None;
    }
    let body = size.iter().fold(0u64, |v, b| (v << 7) | u64::from(*b));
    Some(10 + body + if flags & 0x10 != 0 { 10 } else { 0 })
}

/// Identify a TTA, TAK, OptimFROG, AAC (ADTS) or WavPack stream by the
/// magic after any leading ID3v2 tags.
pub(super) fn detect_stream(path: &Path) -> Option<raw_streams::Stream> {
    let mut file = fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let start = raw_streams::leading_tags(&mut file, length).ok()?;
    let magic = read_at(&mut file, start, 4).ok()?;
    raw_streams::Stream::from_magic(&magic)
}

/// Metadata and stream properties of one container, with fields already
/// expressed as Mp3tag field names.
pub(super) struct Description {
    pub format: &'static str,
    pub fields: Fields,
    pub pictures: Vec<Picture>,
    pub duration: f64,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub bits: Option<u8>,
    pub codec: String,
    /// Tag kinds present, in display priority order (`_TAG`).
    pub tags: Vec<String>,
    pub id3: Option<Id3v2Tag>,
    /// Further information fields, such as `_SUBSONG_COUNT`.
    pub technical: Fields,
    pub chapters: Vec<ChapterInfo>,
}

/// One requested change. Field names are canonical Mp3tag names.
pub(super) enum Edit {
    Fields(Fields),
    Pictures(Vec<Picture>),
    Remove,
}

fn describe(
    path: &Path,
    kind: Kind,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<Description, String> {
    match kind {
        Kind::Dsf | Kind::Dff => dsd::describe(path, kind, rules, options),
        Kind::Matroska => matroska::describe(path, options),
    }
}

pub(super) fn read(
    path: &Path,
    kind: Kind,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let description = describe(path, kind, rules, options)?;
    let covers: Vec<_> = description
        .pictures
        .iter()
        .enumerate()
        .map(|(index, picture)| cover_info(index, picture, include_cover_data))
        .collect();
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut technical = Fields::new();
    if !description.codec.is_empty() {
        technical.insert("_CODEC".into(), vec![description.codec.clone()]);
    }
    technical::stream_fields(
        &mut technical,
        description.bits,
        Some(description.bitrate),
        Some(description.sample_rate),
        Some(description.channels),
        Duration::try_from_secs_f64(description.duration).unwrap_or_default(),
    );
    if let Some(first) = description.tags.first() {
        technical.insert("_TAG".into(), vec![description.tags.join(" ")]);
        technical.insert("_TAG_READ".into(), vec![first.clone()]);
    }
    if let Some(tag) = description.id3.as_ref() {
        technical::id3_details(&mut technical, tag);
    }
    technical.extend(description.technical.clone());
    technical::file_fields(&mut technical, &covers, &metadata, path);
    let mut entry = AudioEntry {
        path: path.to_string_lossy().into_owned(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        format: description.format.into(),
        duration: description.duration,
        bitrate: description.bitrate,
        sample_rate: description.sample_rate,
        channels: description.channels,
        size: metadata.len(),
        fields: description.fields,
        covers,
        technical,
        chapters: Vec::new(),
    };
    subsongs::attach(&mut entry, description.chapters);
    Ok(entry)
}

/// Stage, apply and verify one edit chosen from the current description.
fn mutate(
    path: &Path,
    kind: Kind,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    plan: impl FnOnce(&Description) -> Result<Edit, String>,
    message: &str,
) -> Result<AudioEntry, String> {
    staged_write(
        path,
        include_cover_data,
        rules,
        options,
        |staging, original, staged_path| {
            let current = describe(original, kind, rules, options)?;
            let edit = plan(&current)?;
            let latin1 = match kind {
                Kind::Dsf | Kind::Dff => {
                    dsd::write(staging, original, kind, &edit, rules, options)?
                }
                Kind::Matroska => {
                    matroska::write(staging, original, &edit, options)?;
                    false
                }
            };
            staging.sync_all().map_err(|e| e.to_string())?;
            verify(staged_path, kind, &edit, rules, options, latin1, message)?;
            match kind {
                Kind::Dsf | Kind::Dff => dsd::verify_media(original, staged_path, kind),
                Kind::Matroska => matroska::verify_media(original, staged_path, &edit),
            }
        },
    )
}

fn verify(
    staged_path: &Path,
    kind: Kind,
    edit: &Edit,
    rules: &[UserFieldMapping],
    options: &TagOptions,
    latin1: bool,
    message: &str,
) -> Result<(), String> {
    let actual = read(staged_path, kind, false, &[], options)?;
    match edit {
        Edit::Fields(updates) => {
            if kind == Kind::Matroska {
                // Matroska strings round-trip verbatim, like APE items.
                verify_readback(&actual.fields, updates, TagType::Ape, false)
            } else {
                let native = normalized_updates(
                    &map_write_fields(updates, TagType::Id3v2, rules)?,
                    TagType::Id3v2,
                )?;
                verify_readback(&actual.fields, &native, TagType::Id3v2, latin1)
            }
        }
        Edit::Pictures(expected) => {
            if actual.covers.len() != expected.len()
                || actual.covers.iter().zip(expected).any(|(cover, picture)| {
                    cover.picture_type != format!("{:?}", picture.pic_type())
                        || cover.description != picture.description().unwrap_or_default()
                })
            {
                return Err(message.into());
            }
            Ok(())
        }
        Edit::Remove => {
            if actual.fields.is_empty() && actual.covers.is_empty() {
                Ok(())
            } else {
                Err("The tags could not be removed".into())
            }
        }
    }
}

pub(super) fn write_fields(
    path: &Path,
    kind: Kind,
    updates: &Fields,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let mut updates = normalized_updates(updates, TagType::Id3v2)?;
    if kind == Kind::Matroska {
        matroska::normalize_chapters(&mut updates)?;
    }
    mutate(
        path,
        kind,
        include_cover_data,
        rules,
        options,
        |_| Ok(Edit::Fields(updates)),
        "",
    )
}

pub(super) fn remove_tags(
    path: &Path,
    kind: Kind,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    mutate(
        path,
        kind,
        include_cover_data,
        &options.mappings,
        options,
        |_| Ok(Edit::Remove),
        "",
    )
}

pub(super) fn add_cover(
    path: &Path,
    kind: Kind,
    mut picture: Picture,
    label: &str,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    mutate(
        path,
        kind,
        include_cover_data,
        &options.mappings,
        options,
        |current| {
            let mut pictures = current.pictures.clone();
            if kind != Kind::Matroska {
                describe_repeated_cover(&pictures, &mut picture, label);
            }
            pictures.push(picture);
            Ok(Edit::Pictures(pictures))
        },
        "This tag format cannot append another cover with the same picture type",
    )
}

pub(super) fn remove_cover(
    path: &Path,
    kind: Kind,
    index: usize,
    include_cover_data: bool,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    mutate(
        path,
        kind,
        include_cover_data,
        &options.mappings,
        options,
        |current| {
            if index >= current.pictures.len() {
                return Err(format!("Cover index {index} does not exist"));
            }
            let mut pictures = current.pictures.clone();
            pictures.remove(index);
            Ok(Edit::Pictures(pictures))
        },
        "The cover could not be removed",
    )
}

/// Replace the whole cover list (Manage covers).
pub(super) fn set_covers(
    path: &Path,
    kind: Kind,
    options: &TagOptions,
    pictures: Vec<Picture>,
    failure: &str,
) -> Result<AudioEntry, String> {
    mutate(
        path,
        kind,
        false,
        &options.mappings,
        options,
        |_| Ok(Edit::Pictures(pictures)),
        failure,
    )
}

pub(super) fn transform_covers(
    path: &Path,
    kind: Kind,
    options: &TagOptions,
    mut transform: impl FnMut(&Picture) -> Result<Option<Picture>, String>,
) -> Result<AudioEntry, String> {
    mutate(
        path,
        kind,
        false,
        &options.mappings,
        options,
        |current| {
            let mut pictures = Vec::new();
            for picture in &current.pictures {
                if let Some(picture) = transform(picture)? {
                    pictures.push(picture);
                }
            }
            Ok(Edit::Pictures(pictures))
        },
        "The tag format cannot preserve all edited covers (duplicate type/description)",
    )
}

/// A planned output file: byte ranges copied from the original file and new
/// bytes, in order.
pub(super) enum Piece {
    Copy(Range<u64>),
    Bytes(Vec<u8>),
}
impl Piece {
    fn len(&self) -> u64 {
        match self {
            Piece::Copy(range) => range.end - range.start,
            Piece::Bytes(bytes) => bytes.len() as u64,
        }
    }
}

const BUFFER: usize = 1 << 20;

/// Lay `pieces` out in `staging`, a copy of `original`. Copies that already
/// sit at their source offset are skipped, so unchanged media is not
/// rewritten; every other piece is read from the untouched original file.
pub(super) fn assemble(
    staging: &mut fs::File,
    original: &Path,
    pieces: &[Piece],
) -> Result<u64, String> {
    let mut source = fs::File::open(original).map_err(|e| e.to_string())?;
    let mut offset = 0u64;
    let mut buffer = vec![0u8; BUFFER];
    for piece in pieces {
        match piece {
            Piece::Copy(range) if range.start == offset => {}
            Piece::Copy(range) => {
                source
                    .seek(SeekFrom::Start(range.start))
                    .map_err(|e| e.to_string())?;
                staging
                    .seek(SeekFrom::Start(offset))
                    .map_err(|e| e.to_string())?;
                let mut remaining = range.end - range.start;
                while remaining > 0 {
                    let take = remaining.min(BUFFER as u64) as usize;
                    source
                        .read_exact(&mut buffer[..take])
                        .map_err(|e| e.to_string())?;
                    staging
                        .write_all(&buffer[..take])
                        .map_err(|e| e.to_string())?;
                    remaining -= take as u64;
                }
            }
            Piece::Bytes(bytes) => {
                staging
                    .seek(SeekFrom::Start(offset))
                    .map_err(|e| e.to_string())?;
                staging.write_all(bytes).map_err(|e| e.to_string())?;
            }
        }
        offset += piece.len();
    }
    staging.set_len(offset).map_err(|e| e.to_string())?;
    Ok(offset)
}

pub(super) fn write_at(file: &mut fs::File, offset: u64, bytes: &[u8]) -> Result<(), String> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())
}

pub(super) fn read_at(file: &mut fs::File, offset: u64, length: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0u8; length];
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// Compare `range` of `left` with the same number of bytes at `right_start`
/// of `right`.
pub(super) fn same_bytes(
    left: &Path,
    range: Range<u64>,
    right: &Path,
    right_start: u64,
) -> Result<bool, String> {
    let mut a = fs::File::open(left).map_err(|e| e.to_string())?;
    let mut b = fs::File::open(right).map_err(|e| e.to_string())?;
    let length = b.metadata().map_err(|e| e.to_string())?.len();
    if right_start + (range.end - range.start) > length {
        return Ok(false);
    }
    a.seek(SeekFrom::Start(range.start))
        .map_err(|e| e.to_string())?;
    b.seek(SeekFrom::Start(right_start))
        .map_err(|e| e.to_string())?;
    let mut x = vec![0u8; BUFFER];
    let mut y = vec![0u8; BUFFER];
    let mut remaining = range.end - range.start;
    while remaining > 0 {
        let take = remaining.min(BUFFER as u64) as usize;
        a.read_exact(&mut x[..take]).map_err(|e| e.to_string())?;
        b.read_exact(&mut y[..take]).map_err(|e| e.to_string())?;
        if x[..take] != y[..take] {
            return Ok(false);
        }
        remaining -= take as u64;
    }
    Ok(true)
}

/// Text stored by older DSD and Matroska writers is usually UTF-8; fall back
/// to ISO-8859-1 so that no byte sequence is rejected.
pub(super) fn decode_text(bytes: &[u8]) -> String {
    let end = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    let bytes = &bytes[..end];
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes.iter().map(|b| char::from(*b)).collect(),
    }
}
