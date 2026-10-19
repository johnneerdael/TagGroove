//! TTA (True Audio), TAK, OptimFROG, AAC (ADTS) and WavPack: raw streams
//! that, like MP3, carry a leading ID3v2 tag and trailing APE and ID3v1 tags.
//! Mp3tag treats TTA and AAC like MP3 (Options → Tags → Mpeg) and the others
//! like Monkey's Audio (Ape, Mpc). lofty cannot parse TTA, TAK and OptimFROG,
//! reads no APE tag in AAC and no ID3v2 tag in WavPack, so the tags of all
//! five are read and written on a stand-in: the file's own tag bytes around a
//! minimal MPEG (TTA, AAC) or Monkey's Audio (TAK, OptimFROG, WavPack)
//! stream. The shared tag policies run unchanged on the stand-in; its tags
//! are then spliced around the untouched audio bytes, which are verified to
//! be identical. lofty still reads the AAC and WavPack stream properties,
//! from the audio bytes alone.
use super::*;
use super::{
    assemble_diagnostic as assemble, read_at_diagnostic as read_at,
    same_bytes_diagnostic as same_bytes,
};
use crate::batch_errors::ErrorCause;
use lofty::{aac::AacFile, ape::ApeFile, mpeg::MpegFile};
use std::io::{Cursor, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tags) enum Stream {
    Tta,
    Tak,
    OptimFrog,
    Aac,
    WavPack,
}

impl Stream {
    pub(in crate::tags) fn from_magic(magic: &[u8]) -> Option<Self> {
        match magic {
            b"TTA1" => Some(Self::Tta),
            b"tBaK" => Some(Self::Tak),
            b"OFR " => Some(Self::OptimFrog),
            b"wvpk" => Some(Self::WavPack),
            // ADTS sync: twelve set bits, then the MPEG version and a layer
            // of 0 (MPEG audio never uses layer 0).
            [0xFF, second, ..] if second & 0xF6 == 0xF0 => Some(Self::Aac),
            _ => None,
        }
    }
    fn format(self) -> &'static str {
        match self {
            Self::Tta => "TTA",
            Self::Tak => "TAK",
            Self::OptimFrog => "OptimFROG",
            Self::Aac => "AAC",
            Self::WavPack => "WavPack",
        }
    }
    fn codec(self) -> &'static str {
        match self {
            Self::Tta => "True Audio",
            Self::Tak => "TAK",
            Self::OptimFrog => "OptimFROG",
            Self::Aac => "AAC",
            Self::WavPack => "WavPack",
        }
    }
    /// Whether the stream follows Options → Tags → Mpeg (otherwise Ape, Mpc).
    fn mpeg_policy(self) -> bool {
        matches!(self, Self::Tta | Self::Aac)
    }
    /// A minimal stream lofty parses as the format whose tag policy applies.
    fn stand_in(self) -> Vec<u8> {
        match self {
            // Two MPEG-1 Layer III frames (32 kbit/s, 32 kHz, mono): lofty
            // confirms a frame sync by finding the next frame.
            Self::Tta | Self::Aac => {
                let mut frame = [0u8; 144];
                frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x18, 0xC0]);
                frame.repeat(2)
            }
            // A Monkey's Audio 3.99 descriptor and header with one frame.
            Self::Tak | Self::OptimFrog | Self::WavPack => {
                let mut bytes = b"MAC ".to_vec();
                bytes.extend(3990u16.to_le_bytes());
                bytes.extend(0u16.to_le_bytes());
                bytes.extend(52u32.to_le_bytes());
                bytes.extend(24u32.to_le_bytes());
                bytes.extend([0u8; 36]);
                bytes.extend(2000u16.to_le_bytes());
                bytes.extend(0u16.to_le_bytes());
                for value in [1152u32, 1152, 1] {
                    bytes.extend(value.to_le_bytes());
                }
                bytes.extend(16u16.to_le_bytes());
                bytes.extend(2u16.to_le_bytes());
                bytes.extend(44_100u32.to_le_bytes());
                bytes
            }
        }
    }
}

/// Bytes of junk or padding searched after the leading ID3v2 tags for the
/// first ADTS frame.
const ADTS_SEARCH: usize = 64 << 10;
/// ADTS frames that must follow each other for a frame sync found after junk
/// to count.
const ADTS_CONFIRM: usize = 3;

/// Sampling frequency index, channel configuration and length of the ADTS
/// frame header at the start of `bytes`.
fn adts_header(bytes: &[u8]) -> Option<(u8, u8, usize)> {
    let [0xFF, second, third, fourth, fifth, sixth, ..] = *bytes else {
        return None;
    };
    if second & 0xF6 != 0xF0 {
        return None;
    }
    let frequency = (third >> 2) & 0xF;
    let channels = ((third & 1) << 2) | (fourth >> 6);
    let length =
        (usize::from(fourth & 3) << 11) | (usize::from(fifth) << 3) | usize::from(sixth >> 5);
    let header = if second & 1 == 0 { 9 } else { 7 };
    (frequency < 13 && length > header).then_some((frequency, channels, length))
}

/// Whether `bytes` start with an MPEG audio frame header (layer I–III).
fn mpeg_header(bytes: &[u8]) -> bool {
    matches!(*bytes, [0xFF, second, third, ..]
        if second & 0xE0 == 0xE0 && second & 0x06 != 0 && third >> 4 != 15 && (third >> 2) & 3 != 3)
}

/// The offset in `bytes` of the first of [`ADTS_CONFIRM`] consecutive ADTS
/// frames of the same format, within the first [`ADTS_SEARCH`] bytes. The
/// search stops at an MPEG audio frame: that is an MPEG stream.
fn adts_after_junk(bytes: &[u8]) -> Option<usize> {
    for start in 0..bytes.len().min(ADTS_SEARCH) {
        if mpeg_header(&bytes[start..]) {
            return None;
        }
        let Some((frequency, channels, _)) = adts_header(&bytes[start..]) else {
            continue;
        };
        let mut at = start;
        let confirmed = (0..ADTS_CONFIRM).all(|_| match bytes.get(at..).and_then(adts_header) {
            Some((f, c, length)) if (f, c) == (frequency, channels) => {
                at += length;
                true
            }
            _ => false,
        });
        if confirmed {
            return Some(start);
        }
    }
    None
}

/// Where the ADTS stream of an `.aac` file starts when junk or padding sits
/// between its leading ID3v2 tags (ending at `prefix`) and the first frame.
/// Only `.aac` files are searched, as Mp3tag tells formats by extension and
/// other streams (FLAC frames, for one) can look like ADTS frames.
pub(in crate::tags) fn adts_start(
    file: &mut fs::File,
    path: &Path,
    prefix: u64,
    length: u64,
) -> Result<Option<u64>, String> {
    adts_start_diagnostic(file, path, prefix, length).map_err(|error| error.to_string())
}

pub(in crate::tags) fn adts_start_diagnostic(
    file: &mut fs::File,
    path: &Path,
    prefix: u64,
    length: u64,
) -> Result<Option<u64>, ErrorCause> {
    let aac = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("aac"));
    if !aac || prefix == 0 || prefix >= length {
        return Ok(None);
    }
    let window = (length - prefix).min((ADTS_SEARCH + ADTS_CONFIRM * 0x2000) as u64);
    let bytes = read_at(file, prefix, window as usize)?;
    Ok(adts_after_junk(&bytes).map(|at| prefix + at as u64))
}

fn invalid(stream: Stream, what: &str) -> ErrorCause {
    ErrorCause::wrap(
        "Invalid {value} file: {what}",
        [("value", stream.format().into())],
        "what",
        ErrorCause::owned(what),
    )
}

/// Bytes taken by consecutive ID3v2 tags at the start of the file.
pub(in crate::tags) fn leading_tags(file: &mut fs::File, length: u64) -> Result<u64, String> {
    leading_tags_diagnostic(file, length).map_err(|error| error.to_string())
}

pub(in crate::tags) fn leading_tags_diagnostic(
    file: &mut fs::File,
    length: u64,
) -> Result<u64, ErrorCause> {
    let mut position = 0u64;
    while position + 10 <= length {
        let header = read_at(file, position, 10)?;
        let Some(size) = id3v2_size(&header) else {
            break;
        };
        position += size;
    }
    if position > length {
        return Err(ErrorCause::owned("The ID3v2 tag exceeds the file"));
    }
    Ok(position)
}

/// Where the trailing tags begin: an APE tag (with its optional header),
/// then a Lyrics3v2 block and an ID3v1 tag, the order lofty reads them in.
fn trailing_tags(file: &mut fs::File, floor: u64, length: u64) -> Result<u64, ErrorCause> {
    let mut end = length;
    let lookalike = crate::ape_footer::id3v1_lookalike(file).map_err(ErrorCause::external)?;
    if end >= floor + 128 && read_at(file, end - 128, 3)? == b"TAG" && lookalike != Some(end - 128)
    {
        end -= 128;
    }
    if let Some(range) = crate::lyrics3::detect(file).map_err(ErrorCause::external)? {
        if range.start >= floor {
            end = range.start;
        }
    }
    if end >= floor + 32 {
        let footer = read_at(file, end - 32, 32)?;
        if footer.starts_with(b"APETAGEX") {
            let size = u64::from(u32::from_le_bytes(footer[12..16].try_into().expect("4")));
            let flags = u32::from_le_bytes(footer[20..24].try_into().expect("4"));
            let total = size + if flags & (1 << 31) != 0 { 32 } else { 0 };
            if size < 32 || end < floor + total {
                return Err(ErrorCause::owned("The APE tag exceeds the file"));
            }
            end -= total;
        }
    }
    Ok(end)
}

struct Layout {
    stream: Stream,
    /// Leading ID3v2 bytes; the audio bytes start here.
    prefix: u64,
    /// The stream header or first frame: `prefix`, or later after junk.
    start: u64,
    audio: Range<u64>,
    length: u64,
}

fn layout(file: &mut fs::File, path: &Path, stream: Stream) -> Result<Layout, ErrorCause> {
    let length = file.metadata().map_err(ErrorCause::external)?.len();
    let prefix = leading_tags_diagnostic(file, length)?;
    let start =
        if prefix + 4 <= length && Stream::from_magic(&read_at(file, prefix, 4)?) == Some(stream) {
            Some(prefix)
        } else if stream == Stream::Aac {
            adts_start_diagnostic(file, path, prefix, length)?
        } else {
            None
        }
        .ok_or_else(|| invalid(stream, "missing stream header"))?;
    let end = trailing_tags(file, start + 4, length)?;
    Ok(Layout {
        stream,
        prefix,
        start,
        audio: prefix..end,
        length,
    })
}

/// The file's tags around the stand-in stream.
fn stand_in_bytes(file: &mut fs::File, layout: &Layout) -> Result<Vec<u8>, ErrorCause> {
    let mut bytes = read_at(file, 0, layout.prefix as usize)?;
    bytes.extend(layout.stream.stand_in());
    bytes.extend(read_at(
        file,
        layout.audio.end,
        (layout.length - layout.audio.end) as usize,
    )?);
    Ok(bytes)
}

/// Split a stand-in into its leading and trailing tag bytes.
fn split_stand_in(bytes: &[u8], stream: Stream) -> Result<(&[u8], &[u8]), ErrorCause> {
    let mut position = 0usize;
    while let Some(size) = bytes.get(position..).and_then(id3v2_size) {
        position = usize::try_from(size)
            .ok()
            .and_then(|size| position.checked_add(size))
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| ErrorCause::owned("The written ID3v2 tag is damaged"))?;
    }
    let stand_in = stream.stand_in();
    if bytes.get(position..position + stand_in.len()) != Some(stand_in.as_slice()) {
        return Err(ErrorCause::owned(
            "The tag writer changed the audio stream; original file preserved",
        ));
    }
    Ok((&bytes[..position], &bytes[position + stand_in.len()..]))
}

fn parse_stand_in(bytes: &[u8], stream: Stream) -> Result<(TaggedFile, NativeTags), ErrorCause> {
    let (mut file, native) = strict_or_lenient_id3v1_diagnostic(|mode| {
        let options = ParseOptions::new()
            .implicit_conversions(false)
            .read_properties(false)
            .parsing_mode(mode);
        let read = |mut reader: &mut dyn ReadSeek| {
            reader
                .seek(SeekFrom::Start(0))
                .map_err(ErrorCause::external)?;
            Ok(if stream.mpeg_policy() {
                let file = MpegFile::read_from(&mut reader, options)
                    .map_err(|e| ErrorCause::external(error_message(&e)))?;
                let native = NativeTags {
                    id3: file.id3v2().cloned(),
                    ape: file.ape().cloned(),
                    ..Default::default()
                };
                (TaggedFile::from(file), native)
            } else {
                let file = ApeFile::read_from(&mut reader, options)
                    .map_err(|e| ErrorCause::external(error_message(&e)))?;
                let native = NativeTags {
                    id3: file.id3v2().cloned(),
                    ape: file.ape().cloned(),
                    ..Default::default()
                };
                (TaggedFile::from(file), native)
            })
        };
        crate::id3_frames::read_with_fallback_diagnostic(&mut Cursor::new(bytes), read)
            .map(|((file, native), _, phantom)| (without_phantom(file, phantom), native))
    })?;
    ape_pictures_diagnostic(&native, &mut file)?;
    Ok((file, native))
}

/// The tags of `path` as lofty sees them on the stand-in.
#[cfg(test)]
pub(in crate::tags) fn load(path: &Path, stream: Stream) -> Result<TaggedFile, String> {
    load_diagnostic(path, stream).map_err(|error| error.to_string())
}

pub(in crate::tags) fn load_diagnostic(
    path: &Path,
    stream: Stream,
) -> Result<TaggedFile, ErrorCause> {
    let mut file = fs::File::open(path).map_err(ErrorCause::external)?;
    let layout = layout(&mut file, path, stream)?;
    Ok(parse_stand_in(&stand_in_bytes(&mut file, &layout)?, stream)?.0)
}

struct Properties {
    sample_rate: u32,
    channels: u8,
    bits: Option<u8>,
    /// Samples per channel.
    samples: u64,
}

/// The audio bytes of a file as a stream of their own, so lofty reads the
/// stream properties without seeing the tags around them.
struct Window<'a> {
    file: &'a mut fs::File,
    range: Range<u64>,
    position: u64,
}
impl Read for Window<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let left = (self.range.end - self.range.start).saturating_sub(self.position);
        let take = buffer
            .len()
            .min(usize::try_from(left).unwrap_or(usize::MAX));
        if take == 0 {
            return Ok(0);
        }
        self.file
            .seek(SeekFrom::Start(self.range.start + self.position))?;
        let read = self.file.read(&mut buffer[..take])?;
        self.position += read as u64;
        Ok(read)
    }
}
impl Seek for Window<'_> {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let length = (self.range.end - self.range.start) as i128;
        let target = match to {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(delta) => i128::from(self.position) + i128::from(delta),
            SeekFrom::End(delta) => length + i128::from(delta),
        };
        self.position = u64::try_from(target).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek before the stream")
        })?;
        Ok(self.position)
    }
}

/// Stream properties lofty reads from the audio bytes (AAC, WavPack).
fn lofty_properties(
    file: &mut fs::File,
    layout: &Layout,
) -> Result<lofty::properties::FileProperties, ErrorCause> {
    let mut window = Window {
        file,
        range: layout.start..layout.audio.end,
        position: 0,
    };
    let options = ParseOptions::new()
        .read_tags(false)
        .parsing_mode(ParsingMode::Strict);
    Ok(match layout.stream {
        Stream::Aac => AacFile::read_from(&mut window, options)
            .map_err(|e| ErrorCause::external(error_message(&e)))?
            .properties()
            .to_owned()
            .into(),
        _ => WavPackFile::read_from(&mut window, options)
            .map_err(|e| ErrorCause::external(error_message(&e)))?
            .properties()
            .to_owned()
            .into(),
    })
}

/// Little-endian bit reader (TAK stores its stream info LSB first).
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}
impl Bits<'_> {
    fn take(&mut self, count: u32) -> Option<u64> {
        let mut value = 0u64;
        for index in 0..count {
            let byte = *self.data.get(self.at / 8)?;
            value |= u64::from((byte >> (self.at % 8)) & 1) << index;
            self.at += 1;
        }
        Some(value)
    }
}

/// TAK STREAMINFO: codec, profile, frame size, sample count, data type,
/// sample rate, bits and channels (as in FFmpeg's TAK parser).
fn tak_stream_info(data: &[u8]) -> Option<Properties> {
    let mut bits = Bits { data, at: 0 };
    bits.take(6 + 4 + 4)?;
    let samples = bits.take(35)?;
    bits.take(3)?;
    let sample_rate = u32::try_from(bits.take(18)? + 6000).ok()?;
    let bits_per_sample = u8::try_from(bits.take(5)? + 8).ok()?;
    let channels = u8::try_from(bits.take(4)? + 1).ok()?;
    Some(Properties {
        sample_rate,
        channels,
        bits: Some(bits_per_sample),
        // All ones marks an unknown length.
        samples: if samples == (1 << 35) - 1 { 0 } else { samples },
    })
}

fn properties(file: &mut fs::File, layout: &Layout) -> Result<Properties, ErrorCause> {
    let stream = layout.stream;
    let start = layout.start + 4;
    let available = layout.audio.end.saturating_sub(start);
    let header = |file: &mut fs::File, length: u64| {
        if available < length {
            return Err(invalid(stream, "truncated stream header"));
        }
        read_at(file, start, length as usize)
    };
    let le16 = |b: &[u8], at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let le32 = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("4"));
    match stream {
        // TTA1: format, channels, bits, sample rate, samples per channel, CRC.
        Stream::Tta => {
            let h = header(file, 18)?;
            Ok(Properties {
                channels: u8::try_from(le16(&h, 2)).unwrap_or(0),
                bits: u8::try_from(le16(&h, 4)).ok(),
                sample_rate: le32(&h, 6),
                samples: u64::from(le32(&h, 10)),
            })
        }
        // OFR: header size, total samples (48 bits, all channels), sample
        // type, channel configuration, sample rate.
        Stream::OptimFrog => {
            let h = header(file, 16)?;
            if !matches!(le32(&h, 0), 12 | 15) {
                return Err(invalid(stream, "unknown header size"));
            }
            let total = u64::from(le32(&h, 4)) | u64::from(le16(&h, 8)) << 32;
            let channels = h[11].saturating_add(1);
            Ok(Properties {
                channels,
                bits: match h[10] {
                    0 | 1 => Some(8),
                    2 | 3 => Some(16),
                    4 | 5 => Some(24),
                    6 | 7 => Some(32),
                    _ => None,
                },
                sample_rate: le32(&h, 12),
                samples: total / u64::from(channels),
            })
        }
        // tBaK, then metadata blocks: a type byte, a 24-bit size and data
        // (ending in a CRC-24) until the END block.
        Stream::Tak => {
            let mut position = start;
            while position + 4 <= layout.audio.end {
                let block = read_at(file, position, 4)?;
                let kind = block[0] & 0x7F;
                let size = u64::from(u32::from_le_bytes([block[1], block[2], block[3], 0]));
                if kind == 0 || position + 4 + size > layout.audio.end {
                    break;
                }
                if kind == 1 {
                    let data = read_at(file, position + 4, size.min(64) as usize)?;
                    return tak_stream_info(&data)
                        .ok_or_else(|| invalid(stream, "truncated stream information"));
                }
                position += 4 + size;
            }
            Err(invalid(stream, "missing stream information"))
        }
        // lofty reads these (`lofty_properties`).
        Stream::Aac | Stream::WavPack => Err(invalid(stream, "unexpected stream header")),
    }
}

pub(in crate::tags) fn read_diagnostic(
    path: &Path,
    stream: Stream,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, ErrorCause> {
    let mut source = fs::File::open(path).map_err(ErrorCause::external)?;
    let layout = layout(&mut source, path, stream)?;
    let (file, native) = parse_stand_in(&stand_in_bytes(&mut source, &layout)?, stream)?;
    let mut entry =
        describe_with_native_diagnostic(path, &file, &native, include_cover_data, rules, options)?;
    let audio = layout.audio.end - layout.audio.start;
    let bitrate = |duration: f64| {
        if duration > 0.0 {
            (audio as f64 * 8.0 / duration / 1000.0).round() as u32
        } else {
            0
        }
    };
    let (duration, bits) = match stream {
        Stream::Aac | Stream::WavPack => {
            let properties = lofty_properties(&mut source, &layout)?;
            let duration = properties.duration().as_secs_f64();
            entry.bitrate = properties
                .audio_bitrate()
                .or(properties.overall_bitrate())
                .unwrap_or_else(|| bitrate(duration));
            entry.sample_rate = properties.sample_rate().unwrap_or(0);
            entry.channels = properties.channels().unwrap_or(0);
            (duration, properties.bit_depth())
        }
        _ => {
            let properties = properties(&mut source, &layout)?;
            let duration = if properties.sample_rate > 0 {
                properties.samples as f64 / f64::from(properties.sample_rate)
            } else {
                0.0
            };
            entry.bitrate = bitrate(duration);
            entry.sample_rate = properties.sample_rate;
            entry.channels = properties.channels;
            (duration, properties.bits)
        }
    };
    entry.format = stream.format().into();
    entry.duration = duration;
    // Replace the stand-in's stream fields, and the bitrate type and encoder
    // that were looked up as if the real stream were the stand-in's format.
    for name in [
        "_BITRATE",
        "_BITSPERSAMPLE",
        "_LENGTH",
        "_LENGTH_SECONDS",
        "_MODE",
        "_SAMPLERATE",
        "_TOOL",
        "_VBR",
    ] {
        entry.technical.remove(name);
    }
    entry
        .technical
        .insert("_CODEC".into(), vec![stream.codec().into()]);
    technical::stream_fields(
        &mut entry.technical,
        bits,
        Some(entry.bitrate),
        Some(entry.sample_rate),
        Some(entry.channels),
        Duration::try_from_secs_f64(duration).unwrap_or_default(),
    );
    Ok(entry)
}

/// Run a lofty-based `mutation` on a stand-in for `original`, then lay the
/// stand-in's tags around the original audio bytes in `staging`.
pub(in crate::tags) fn stage_diagnostic(
    staging: &mut fs::File,
    original: &Path,
    staged_path: &Path,
    stream: Stream,
    mutation: impl FnOnce(&mut fs::File, &Path, &Path) -> Result<(), ErrorCause>,
) -> Result<(), ErrorCause> {
    let mut source = fs::File::open(original).map_err(ErrorCause::external)?;
    let layout = layout(&mut source, original, stream)?;
    let bytes = stand_in_bytes(&mut source, &layout)?;
    // Beside the staging file: that directory is known to be writable.
    let dir = tempfile::Builder::new()
        .prefix(".opentag-stand-in-")
        .tempdir_in(
            staged_path
                .parent()
                .ok_or_else(|| ErrorCause::owned("The staging file has no folder"))?,
        )
        .map_err(ErrorCause::external)?;
    let before = dir.path().join("before");
    let after = dir.path().join("after");
    fs::write(&before, &bytes).map_err(ErrorCause::external)?;
    fs::write(&after, &bytes).map_err(ErrorCause::external)?;
    let mut edited = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&after)
        .map_err(ErrorCause::external)?;
    // The stand-in's own rewrite cost says nothing about the real file.
    let scope = crate::write_cost::Scope::begin();
    mutation(&mut edited, &before, &after)?;
    scope.finish();
    drop(edited);
    let edited = fs::read(&after).map_err(ErrorCause::external)?;
    let (prefix, trailer) = split_stand_in(&edited, stream)?;
    let (old_prefix, old_trailer) = split_stand_in(&bytes, stream)?;
    let prefix_changed = prefix != old_prefix;
    let trailer_changed = trailer != old_trailer;
    let pieces = [
        if prefix_changed {
            Piece::Bytes(prefix.to_vec())
        } else {
            Piece::Copy(0..layout.prefix)
        },
        Piece::Copy(layout.audio.clone()),
        if trailer_changed {
            Piece::Bytes(trailer.to_vec())
        } else {
            Piece::Copy(layout.audio.end..layout.length)
        },
    ];
    assemble(staging, original, &pieces)?;
    if prefix.len() as u64 == layout.prefix {
        let mut region = 0;
        if prefix_changed {
            region += prefix.len() as u64;
        }
        if trailer_changed {
            region += trailer.len() as u64;
        }
        crate::write_cost::rewrote_region(region);
    } else {
        crate::write_cost::rewrote_all();
    }
    staging.sync_all().map_err(ErrorCause::external)?;
    if !same_bytes(original, layout.audio, staged_path, prefix.len() as u64)? {
        return Err(ErrorCause::owned(
            "The audio stream changed while writing; original file preserved",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_stream_diagnostics_keep_owned_reason_and_real_io_distinct() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bad.tta");
        let bytes = b"not a TTA stream";
        fs::write(&path, bytes).unwrap();
        let cause = load_diagnostic(&path, Stream::Tta)
            .err()
            .expect("invalid stream must return a diagnostic");
        assert_eq!(cause.to_string(), "Invalid TTA file: missing stream header");
        assert_eq!(
            load(&path, Stream::Tta)
                .err()
                .expect("invalid legacy stream"),
            cause.to_string()
        );
        let value = serde_json::to_value(cause).unwrap();
        assert_eq!(value["kind"], "owned");
        assert_eq!(value["params"]["value"], "TTA");
        assert_eq!(value["children"]["what"]["key"], "missing stream header");
        assert!(matches!(
            load_diagnostic(&directory.path().join("missing.tta"), Stream::Tta)
                .err()
                .expect("missing file must fail"),
            ErrorCause::External { .. }
        ));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn raw_stream_stage_keeps_typed_callback_errors_and_original_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("practice.tta");
        let bytes = [b"TTA1".as_slice(), &[0u8; 24]].concat();
        fs::write(&original, &bytes).unwrap();
        for external in [false, true] {
            let staged_path = directory
                .path()
                .join(if external { "io.tmp" } else { "owned.tmp" });
            let before = b"unchanged staged bytes";
            fs::write(&staged_path, before).unwrap();
            let mut staging = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&staged_path)
                .unwrap();
            let cause = stage_diagnostic(
                &mut staging,
                &original,
                &staged_path,
                Stream::Tta,
                |_, _, _| {
                    Err(if external {
                        ErrorCause::external(std::io::Error::other("Select files first."))
                    } else {
                        ErrorCause::owned("Select files first.")
                    })
                },
            )
            .unwrap_err();
            assert_eq!(cause.to_string(), "Select files first.");
            assert_eq!(matches!(cause, ErrorCause::External { .. }), external);
            assert_eq!(fs::read(&staged_path).unwrap(), before);
            assert_eq!(fs::read(&original).unwrap(), bytes);
        }
    }

    #[test]
    fn stand_ins_parse_as_their_tag_policy_formats() {
        for (stream, kind) in [
            (Stream::Tta, FileType::Mpeg),
            (Stream::Tak, FileType::Ape),
            (Stream::OptimFrog, FileType::Ape),
        ] {
            let bytes = stream.stand_in();
            let probe = Probe::new(Cursor::new(&bytes)).guess_file_type().unwrap();
            assert_eq!(probe.file_type(), Some(kind));
            // Full property parsing succeeds, so verification can read it back.
            probe.read().unwrap();
            assert_eq!(split_stand_in(&bytes, stream).unwrap(), (&[][..], &[][..]));
        }
    }

    fn adts_frame(length: usize) -> Vec<u8> {
        let mut frame = vec![
            0xFF,
            0xF1,
            (1 << 6) | (4 << 2),
            (2 << 6) | ((length >> 11) & 3) as u8,
            ((length >> 3) & 0xFF) as u8,
            (((length & 7) << 5) | 0x1F) as u8,
            0xFC,
        ];
        frame.resize(length, 0x11);
        frame
    }

    #[test]
    fn adts_after_junk_needs_consecutive_frames_and_no_mpeg_frame() {
        let frames = [adts_frame(100), adts_frame(90), adts_frame(120)].concat();
        let mut bytes = vec![0u8; 50];
        bytes.extend(b"junk");
        bytes.extend(&frames);
        assert_eq!(adts_after_junk(&bytes), Some(54));
        // A lone sync in the junk is passed over.
        let mut lone = adts_frame(40)[..7].to_vec();
        lone.extend(&bytes);
        assert_eq!(adts_after_junk(&lone), Some(61));
        // Two frames are not enough.
        assert_eq!(adts_after_junk(&bytes[..bytes.len() - 120]), None);
        // An MPEG frame first: an MPEG stream, whatever follows.
        let mut mpeg = vec![0xFF, 0xFB, 0x90, 0x00, 0, 0];
        mpeg.extend(&bytes);
        assert_eq!(adts_after_junk(&mpeg), None);
        // Junk longer than the search window.
        let mut long = vec![0u8; ADTS_SEARCH];
        long.extend(&frames);
        assert_eq!(adts_after_junk(&long), None);
    }

    #[test]
    fn only_aac_files_are_searched_after_junk() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = b"ID3\x04\0\0\0\0\0\0".to_vec();
        bytes.extend([0u8; 20]);
        bytes.extend([adts_frame(100), adts_frame(90), adts_frame(120)].concat());
        for (name, found) in [("a.aac", Some(30)), ("a.AAC", Some(30)), ("a.flac", None)] {
            let path = dir.path().join(name);
            fs::write(&path, &bytes).unwrap();
            let mut file = fs::File::open(&path).unwrap();
            let length = bytes.len() as u64;
            assert_eq!(
                adts_start(&mut file, &path, 10, length).unwrap(),
                found,
                "{name}"
            );
        }
        // Without a leading ID3v2 tag nothing is searched.
        let path = dir.path().join("a.aac");
        let mut file = fs::File::open(&path).unwrap();
        assert_eq!(adts_start(&mut file, &path, 0, 10).unwrap(), None);
    }

    #[test]
    fn tak_stream_info_is_read_lsb_first() {
        // codec 2, profile 0, frame type 3, 88200 samples, data type 0,
        // 44100 Hz, 16 bits, 2 channels.
        let mut value: u128 = 0;
        let mut at = 0;
        for (field, width) in [
            (2u128, 6),
            (0, 4),
            (3, 4),
            (88_200, 35),
            (0, 3),
            (44_100 - 6000, 18),
            (16 - 8, 5),
            (1, 4),
            (0, 1),
        ] {
            value |= field << at;
            at += width;
        }
        let info = tak_stream_info(&value.to_le_bytes()[..10]).unwrap();
        assert_eq!(info.samples, 88_200);
        assert_eq!(info.sample_rate, 44_100);
        assert_eq!(info.bits, Some(16));
        assert_eq!(info.channels, 2);
    }
}
