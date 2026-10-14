//! DSF (Sony DSD Stream File 1.01) and DSDIFF 1.5 (Philips). Both store
//! metadata as a complete ID3v2 tag, read and written with the shared ID3v2
//! mapping. DSDIFF can also carry edited-master artist and title text (DIIN
//! DIAR/DITI), which is shown only where the ID3v2 tag has no value.
//!
//! Revision policy: an existing tag keeps its ID3v2 revision (as for WAV and
//! AIFF); a new tag uses the revision and encoding chosen for MPEG files,
//! because Mp3tag offers no separate DSD preference.
use super::*;
use lofty::{
    config::{ParseOptions, ParsingMode},
    id3::v2::{Frame, Id3v2Tag},
    iff::aiff::AiffFile,
};
use std::io::Cursor;

/// Parse a standalone ID3v2 tag. Lofty exposes no public parser for raw tag
/// bytes, so the tag is wrapped in a minimal in-memory AIFF form, which lofty
/// reads without audio properties.
fn parse_id3(bytes: &[u8]) -> Result<Id3v2Tag, String> {
    if !bytes.starts_with(b"ID3") {
        return Err("The metadata block is not an ID3v2 tag".into());
    }
    let size = u32::try_from(bytes.len()).map_err(|_| "The ID3v2 tag is too large")?;
    let padding = bytes.len() % 2;
    let mut form = Vec::with_capacity(bytes.len() + 21);
    form.extend(b"FORM");
    form.extend((size + 12 + padding as u32).to_be_bytes());
    form.extend(b"AIFFID3 ");
    form.extend(size.to_be_bytes());
    form.extend(bytes);
    form.resize(form.len() + padding, 0);
    let options = ParseOptions::new()
        .read_properties(false)
        .implicit_conversions(false)
        .parsing_mode(ParsingMode::Strict);
    AiffFile::read_from(&mut Cursor::new(form), options)
        .map_err(|e| error_message(&e))?
        .id3v2()
        .cloned()
        .ok_or_else(|| "The ID3v2 tag could not be read".into())
}

fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}
fn u64_le(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}
fn u64_be(bytes: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}

fn rate_label(prefix: &str, rate: u32) -> String {
    if rate > 0 && rate.is_multiple_of(44_100) {
        format!("{prefix}{}", rate / 44_100)
    } else {
        prefix.to_owned()
    }
}

struct Dsf {
    length: u64,
    /// Offset of the ID3v2 tag, 0 when absent.
    pointer: u64,
    /// Sample data, excluding the data chunk header.
    data: Range<u64>,
    sample_rate: u32,
    channels: u32,
    bits: u32,
    samples: u64,
}

fn parse_dsf(file: &mut fs::File) -> Result<Dsf, String> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let invalid = |what: &str| format!("Invalid DSF file: {what}");
    if length < 28 + 52 + 12 {
        return Err(invalid("the file is truncated"));
    }
    let header = read_at(file, 0, 28)?;
    if &header[..4] != b"DSD " || u64_le(&header, 4) != 28 {
        return Err(invalid("missing DSD chunk"));
    }
    let pointer = u64_le(&header, 20);
    let fmt = read_at(file, 28, 52)?;
    if &fmt[..4] != b"fmt " {
        return Err(invalid("missing fmt chunk"));
    }
    let fmt_size = u64_le(&fmt, 4);
    if fmt_size < 52 {
        return Err(invalid("short fmt chunk"));
    }
    if u32_le(&fmt, 16) != 0 {
        return Err(invalid("unsupported DSD format identifier"));
    }
    let data_at = 28u64
        .checked_add(fmt_size)
        .filter(|at| at + 12 <= length)
        .ok_or_else(|| invalid("missing data chunk"))?;
    let data = read_at(file, data_at, 12)?;
    let data_size = u64_le(&data, 4);
    if &data[..4] != b"data" || data_size < 12 {
        return Err(invalid("missing data chunk"));
    }
    let data_end = data_at
        .checked_add(data_size)
        .filter(|end| *end <= length)
        .ok_or_else(|| invalid("the sample data is truncated"))?;
    if pointer != 0 && (pointer < data_end || pointer >= length) {
        return Err(invalid("the metadata pointer is outside the metadata area"));
    }
    Ok(Dsf {
        length,
        pointer,
        data: data_at + 12..data_end,
        sample_rate: u32_le(&fmt, 28),
        channels: u32_le(&fmt, 24),
        bits: u32_le(&fmt, 32),
        samples: u64_le(&fmt, 36),
    })
}

#[derive(Debug, Clone)]
struct Chunk {
    id: [u8; 4],
    start: u64,
    data: u64,
    size: u64,
}
impl Chunk {
    fn data_end(&self) -> u64 {
        self.data + self.size
    }
}

/// DSDIFF chunks between `from` and `to`: 4-byte identifier, big-endian
/// 64-bit size, data and a pad byte after odd sizes.
fn dff_chunks(file: &mut fs::File, from: u64, to: u64) -> Result<Vec<Chunk>, String> {
    let mut chunks = Vec::new();
    let mut position = from;
    while position + 12 <= to {
        let header = read_at(file, position, 12)?;
        let size = u64_be(&header, 4);
        let data = position + 12;
        if data.checked_add(size).is_none_or(|end| end > to) {
            return Err(format!(
                "Invalid DSDIFF file: chunk {} exceeds its container",
                String::from_utf8_lossy(&header[..4])
            ));
        }
        chunks.push(Chunk {
            id: header[..4].try_into().expect("four bytes"),
            start: position,
            data,
            size,
        });
        position = data + size + size % 2;
    }
    Ok(chunks)
}

struct Dff {
    length: u64,
    form_end: u64,
    chunks: Vec<Chunk>,
    /// PROP sub-chunks (after the "SND " property type).
    prop: Vec<Chunk>,
    /// DIIN sub-chunks.
    diin: Vec<Chunk>,
    /// Root-level and PROP-level ID3 chunks, in file order.
    id3: Vec<Chunk>,
    sound: Chunk,
    sample_rate: u32,
    channels: u16,
    compressed: bool,
    dst_frames: Option<(u32, u16)>,
}

fn parse_dff(file: &mut fs::File) -> Result<Dff, String> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let header = read_at(file, 0, 16)?;
    if &header[..4] != b"FRM8" || &header[12..16] != b"DSD " {
        return Err("Invalid DSDIFF file: missing FRM8 DSD form".into());
    }
    let form_end = 12u64
        .checked_add(u64_be(&header, 4))
        .filter(|end| *end <= length)
        .ok_or("Invalid DSDIFF file: the form is truncated")?;
    let chunks = dff_chunks(file, 16, form_end)?;
    let mut prop = Vec::new();
    let mut diin = Vec::new();
    let mut id3 = Vec::new();
    let mut sound = None;
    for chunk in &chunks {
        match &chunk.id {
            b"PROP" => {
                if !prop.is_empty() || chunk.size < 4 || read_at(file, chunk.data, 4)? != b"SND " {
                    return Err("Invalid DSDIFF file: unexpected property chunk".into());
                }
                prop = dff_chunks(file, chunk.data + 4, chunk.data_end())?;
                id3.extend(prop.iter().filter(|c| &c.id == b"ID3 ").cloned());
            }
            b"DIIN" => diin = dff_chunks(file, chunk.data, chunk.data_end())?,
            b"DSD " | b"DST " if sound.is_some() => {
                return Err("Invalid DSDIFF file: more than one sound data chunk".into());
            }
            b"DSD " | b"DST " => sound = Some(chunk.clone()),
            b"ID3 " => id3.push(chunk.clone()),
            _ => {}
        }
    }
    id3.sort_by_key(|c| c.start);
    let sound = sound.ok_or("Invalid DSDIFF file: missing sound data")?;
    let mut sample_rate = 0;
    let mut channels = 0;
    let mut compressed = false;
    for chunk in &prop {
        match &chunk.id {
            b"FS  " if chunk.size >= 4 => {
                sample_rate = u32::from_be_bytes(
                    read_at(file, chunk.data, 4)?
                        .try_into()
                        .expect("four bytes"),
                )
            }
            b"CHNL" if chunk.size >= 2 => {
                channels =
                    u16::from_be_bytes(read_at(file, chunk.data, 2)?.try_into().expect("two bytes"))
            }
            b"CMPR" if chunk.size >= 4 => compressed = read_at(file, chunk.data, 4)? == b"DST ",
            _ => {}
        }
    }
    let mut dst_frames = None;
    if &sound.id == b"DST " {
        for chunk in dff_chunks(file, sound.data, sound.data_end())? {
            if &chunk.id == b"FRTE" && chunk.size >= 6 {
                let bytes = read_at(file, chunk.data, 6)?;
                dst_frames = Some((
                    u32::from_be_bytes(bytes[..4].try_into().expect("four bytes")),
                    u16::from_be_bytes(bytes[4..6].try_into().expect("two bytes")),
                ));
                break;
            }
        }
    }
    Ok(Dff {
        length,
        form_end,
        chunks,
        prop,
        diin,
        id3,
        sound,
        sample_rate,
        channels,
        compressed,
        dst_frames,
    })
}

/// DIAR/DITI text: a big-endian 32-bit count followed by the characters.
fn diin_text(file: &mut fs::File, chunk: &Chunk) -> Result<Option<String>, String> {
    if chunk.size < 4 {
        return Ok(None);
    }
    let bytes = read_at(file, chunk.data, chunk.size as usize)?;
    let count = u32::from_be_bytes(bytes[..4].try_into().expect("four bytes")) as usize;
    let text = decode_text(&bytes[4..(4 + count).min(bytes.len())]);
    Ok((!text.is_empty()).then_some(text))
}

fn chunk_bytes(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(data.len() + 13);
    bytes.extend(id);
    bytes.extend((data.len() as u64).to_be_bytes());
    bytes.extend(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

struct Snapshot {
    id3: Option<Id3v2Tag>,
    artist: Option<String>,
    title: Option<String>,
    description: Description,
}

fn snapshot(path: &Path, kind: Kind) -> Result<Snapshot, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let (id3, artist, title, description) = match kind {
        Kind::Dsf => {
            let dsf = parse_dsf(&mut file)?;
            let id3 = if dsf.pointer == 0 {
                None
            } else {
                Some(parse_id3(&read_at(
                    &mut file,
                    dsf.pointer,
                    (dsf.length - dsf.pointer) as usize,
                )?)?)
            };
            let duration = if dsf.sample_rate > 0 {
                dsf.samples as f64 / f64::from(dsf.sample_rate)
            } else {
                0.0
            };
            (
                id3,
                None,
                None,
                Description {
                    format: "DSF",
                    fields: Fields::new(),
                    pictures: Vec::new(),
                    duration,
                    bitrate: (u64::from(dsf.sample_rate) * u64::from(dsf.channels) / 1000) as u32,
                    sample_rate: dsf.sample_rate,
                    channels: u8::try_from(dsf.channels).unwrap_or(0),
                    bits: u8::try_from(dsf.bits).ok(),
                    codec: rate_label("DSD", dsf.sample_rate),
                    tags: Vec::new(),
                    id3: None,
                },
            )
        }
        _ => {
            let dff = parse_dff(&mut file)?;
            let id3 = match dff.id3.first() {
                Some(chunk) => Some(parse_id3(&read_at(
                    &mut file,
                    chunk.data,
                    chunk.size as usize,
                )?)?),
                None => None,
            };
            let mut artist = None;
            let mut title = None;
            for chunk in &dff.diin {
                match &chunk.id {
                    b"DIAR" => artist = diin_text(&mut file, chunk)?,
                    b"DITI" => title = diin_text(&mut file, chunk)?,
                    _ => {}
                }
            }
            let rate = u64::from(dff.sample_rate) * u64::from(dff.channels);
            let duration = match dff.dst_frames {
                Some((frames, per_second)) if per_second > 0 => {
                    f64::from(frames) / f64::from(per_second)
                }
                _ if rate > 0 && !dff.compressed => dff.sound.size as f64 * 8.0 / rate as f64,
                _ => 0.0,
            };
            let bitrate = if dff.compressed && duration > 0.0 {
                (dff.sound.size as f64 * 8.0 / duration / 1000.0) as u32
            } else {
                (rate / 1000) as u32
            };
            (
                id3,
                artist,
                title,
                Description {
                    format: "DFF",
                    fields: Fields::new(),
                    pictures: Vec::new(),
                    duration,
                    bitrate,
                    sample_rate: dff.sample_rate,
                    channels: u8::try_from(dff.channels).unwrap_or(0),
                    bits: Some(1),
                    codec: rate_label(if dff.compressed { "DST" } else { "DSD" }, dff.sample_rate),
                    tags: Vec::new(),
                    id3: None,
                },
            )
        }
    };
    Ok(Snapshot {
        id3,
        artist,
        title,
        description,
    })
}

pub(super) fn describe(
    path: &Path,
    kind: Kind,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<Description, String> {
    let Snapshot {
        id3,
        artist,
        title,
        mut description,
    } = snapshot(path, kind)?;
    let mut native = Fields::new();
    if let Some(tag) = id3.as_ref() {
        native = mapping::id3_fields_with(
            tag,
            tag.original_version(),
            options.advanced.popularimeter_ratings,
        );
        description.tags.push(technical::id3_name(tag).into());
        let generic: Tag = tag.clone().into();
        description.pictures = generic.pictures().to_vec();
    }
    let mut fields = map_read_fields(&native, TagType::Id3v2, rules)?;
    // ID3v2 takes precedence; DIIN text only fills fields the tag lacks.
    if artist.is_some() || title.is_some() {
        description.tags.push("DIIN".into());
    }
    for (name, value) in [("ARTIST", artist), ("TITLE", title)] {
        if let Some(value) = value {
            if !fields.keys().any(|key| key.eq_ignore_ascii_case(name)) {
                fields.insert(name.into(), vec![value]);
            }
        }
    }
    description.fields = fields;
    description.id3 = id3;
    Ok(description)
}

/// Apply `edit` to `staging` and return whether text was written as ISO-8859-1.
pub(super) fn write(
    staging: &mut fs::File,
    original: &Path,
    kind: Kind,
    edit: &Edit,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<bool, String> {
    let current = snapshot(original, kind)?;
    let target = match current.id3.as_ref() {
        Some(tag) => mapping::Id3Target::preserving(tag, options.comment_language()),
        None => tag_policy::id3_target(options),
    };
    let mut native = current.id3.clone().unwrap_or_default();
    let mut diin: Vec<(&[u8; 4], Option<String>)> = Vec::new();
    match edit {
        Edit::Fields(updates) => {
            let native_updates = normalized_updates(
                &map_write_fields(updates, TagType::Id3v2, rules)?,
                TagType::Id3v2,
            )?;
            mapping::patch_id3(&mut native, &native_updates, &target)?;
            for (field, id) in [("ARTIST", b"DIAR"), ("TITLE", b"DITI")] {
                if let Some(values) = updates.get(field) {
                    diin.push((id, (!values.is_empty()).then(|| values.join("; "))));
                }
            }
        }
        Edit::Pictures(pictures) => {
            native.retain(|frame| !matches!(frame, Frame::Picture(_)));
            let mut generic = Tag::new(TagType::Id3v2);
            for picture in pictures {
                generic.push_picture(picture.clone());
            }
            let converted: Id3v2Tag = generic.into();
            for frame in &converted {
                if matches!(frame, Frame::Picture(_)) {
                    native.insert(frame.clone());
                }
            }
        }
        Edit::Remove => {
            native = Id3v2Tag::default();
            diin = vec![(b"DIAR", None), (b"DITI", None)];
        }
    }
    let bytes = mapping::render_id3(&native, &target)?;
    match kind {
        Kind::Dsf => write_dsf(staging, original, &bytes)?,
        _ => write_dff(
            staging,
            original,
            &bytes,
            &diin,
            matches!(edit, Edit::Remove),
        )?,
    }
    Ok(target.latin1)
}

fn write_dsf(staging: &mut fs::File, original: &Path, id3: &[u8]) -> Result<(), String> {
    let dsf = parse_dsf(&mut fs::File::open(original).map_err(|e| e.to_string())?)?;
    // The metadata region runs from the pointer to the end of the file; bytes
    // between the sample data and a pointer are kept.
    let keep = if dsf.pointer == 0 {
        dsf.length
    } else {
        dsf.pointer
    };
    let mut pieces = vec![Piece::Copy(0..keep)];
    if !id3.is_empty() {
        pieces.push(Piece::Bytes(id3.to_vec()));
    }
    let length = assemble(staging, original, &pieces)?;
    write_at(staging, 12, &length.to_le_bytes())?;
    let pointer = if id3.is_empty() { 0 } else { keep };
    write_at(staging, 20, &pointer.to_le_bytes())
}

/// Rewrite DIIN with replaced (or removed) DIAR/DITI sub-chunks. Returns
/// `None` when DIIN is unchanged and an empty vector when it becomes empty.
fn rebuilt_diin(
    file: &mut fs::File,
    dff: &Dff,
    changes: &[(&[u8; 4], Option<String>)],
) -> Result<Option<Vec<u8>>, String> {
    let affected = |id: &[u8; 4]| changes.iter().find(|(c, _)| *c == id);
    if !dff.diin.iter().any(|c| affected(&c.id).is_some()) {
        return Ok(None);
    }
    let mut body = Vec::new();
    for chunk in &dff.diin {
        match affected(&chunk.id) {
            Some((_, None)) => {}
            Some((id, Some(text))) => {
                let mut data = (text.len() as u32).to_be_bytes().to_vec();
                data.extend(text.as_bytes());
                body.extend(chunk_bytes(id, &data));
            }
            None => {
                let data = read_at(file, chunk.data, chunk.size as usize)?;
                body.extend(chunk_bytes(&chunk.id, &data));
            }
        }
    }
    Ok(Some(if body.is_empty() {
        Vec::new()
    } else {
        chunk_bytes(b"DIIN", &body)
    }))
}

/// ID3v2 is written as the last root-level chunk; every other ID3 chunk,
/// including one inside PROP, is removed so that exactly one copy remains.
fn write_dff(
    staging: &mut fs::File,
    original: &Path,
    id3: &[u8],
    diin: &[(&[u8; 4], Option<String>)],
    remove: bool,
) -> Result<(), String> {
    let mut file = fs::File::open(original).map_err(|e| e.to_string())?;
    let dff = parse_dff(&mut file)?;
    if dff.id3.len() > 1 && !remove {
        return Err(
            "This DSDIFF file contains more than one ID3 chunk; remove the tag first so that no metadata is silently merged"
                .into(),
        );
    }
    let new_diin = rebuilt_diin(&mut file, &dff, diin)?;
    let mut pieces = vec![Piece::Copy(0..16)];
    let mut offset = 16u64;
    let mut sound_at = None;
    let push = |pieces: &mut Vec<Piece>, piece: Piece, offset: &mut u64| {
        *offset += piece.len();
        pieces.push(piece);
    };
    for chunk in &dff.chunks {
        let end = chunk.data_end();
        match &chunk.id {
            b"ID3 " => continue,
            b"PROP" if dff.prop.iter().any(|c| &c.id == b"ID3 ") => {
                let mut body = b"SND ".to_vec();
                for sub in dff.prop.iter().filter(|c| &c.id != b"ID3 ") {
                    let data = read_at(&mut file, sub.data, sub.size as usize)?;
                    body.extend(chunk_bytes(&sub.id, &data));
                }
                push(
                    &mut pieces,
                    Piece::Bytes(chunk_bytes(b"PROP", &body)),
                    &mut offset,
                );
            }
            b"DIIN" if new_diin.is_some() => {
                let bytes = new_diin.clone().unwrap_or_default();
                if !bytes.is_empty() {
                    push(&mut pieces, Piece::Bytes(bytes), &mut offset);
                }
            }
            _ => {
                if chunk.start == dff.sound.start {
                    sound_at = Some(offset + 12);
                }
                let padded = end + chunk.size % 2;
                if padded <= dff.form_end {
                    push(&mut pieces, Piece::Copy(chunk.start..padded), &mut offset);
                } else {
                    // A final odd chunk may lack its pad byte; restore it.
                    push(&mut pieces, Piece::Copy(chunk.start..end), &mut offset);
                    push(&mut pieces, Piece::Bytes(vec![0]), &mut offset);
                }
            }
        }
    }
    if sound_at != Some(dff.sound.data) && dff.chunks.iter().any(|c| &c.id == b"DSTI") {
        return Err(
            "Writing this DSDIFF layout would move DST sound data referenced by its index; original file preserved"
                .into(),
        );
    }
    if !id3.is_empty() {
        push(
            &mut pieces,
            Piece::Bytes(chunk_bytes(b"ID3 ", id3)),
            &mut offset,
        );
    }
    let form_end = offset;
    if dff.form_end < dff.length {
        pieces.push(Piece::Copy(dff.form_end..dff.length));
    }
    assemble(staging, original, &pieces)?;
    write_at(staging, 4, &(form_end - 12).to_be_bytes())
}

pub(super) fn verify_media(original: &Path, staged: &Path, kind: Kind) -> Result<(), String> {
    let error = || Err("The audio data changed while writing; original file preserved".into());
    match kind {
        Kind::Dsf => {
            let before = parse_dsf(&mut fs::File::open(original).map_err(|e| e.to_string())?)?;
            let after = parse_dsf(&mut fs::File::open(staged).map_err(|e| e.to_string())?)?;
            if before.data != after.data || !same_bytes(original, 28..before.data.end, staged, 28)?
            {
                return error();
            }
        }
        _ => {
            let before = parse_dff(&mut fs::File::open(original).map_err(|e| e.to_string())?)?;
            let after = parse_dff(&mut fs::File::open(staged).map_err(|e| e.to_string())?)?;
            if before.sound.size != after.sound.size
                || before.sound.id != after.sound.id
                || !same_bytes(
                    original,
                    before.sound.data..before.sound.data_end(),
                    staged,
                    after.sound.data,
                )?
            {
                return error();
            }
        }
    }
    Ok(())
}
