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

fn invalid(stream: Stream, what: &str) -> String {
    format!("Invalid {} file: {what}", stream.format())
}

/// Bytes taken by consecutive ID3v2 tags at the start of the file.
pub(in crate::tags) fn leading_tags(file: &mut fs::File, length: u64) -> Result<u64, String> {
    let mut position = 0u64;
    while position + 10 <= length {
        let header = read_at(file, position, 10)?;
        let Some(size) = id3v2_size(&header) else {
            break;
        };
        position += size;
    }
    if position > length {
        return Err("The ID3v2 tag exceeds the file".into());
    }
    Ok(position)
}

/// Where the trailing tags begin: an APE tag (with its optional header),
/// then a Lyrics3v2 block and an ID3v1 tag, the order lofty reads them in.
fn trailing_tags(file: &mut fs::File, floor: u64, length: u64) -> Result<u64, String> {
    let mut end = length;
    if end >= floor + 128 && read_at(file, end - 128, 3)? == b"TAG" {
        end -= 128;
    }
    if end >= floor + 15 && read_at(file, end - 9, 9)? == b"LYRICS200" {
        let digits = read_at(file, end - 15, 6)?;
        if let Some(size) = std::str::from_utf8(&digits)
            .ok()
            .and_then(|d| d.parse::<u64>().ok())
            .filter(|size| end >= floor + 15 + size)
        {
            end -= 15 + size;
        }
    }
    if end >= floor + 32 {
        let footer = read_at(file, end - 32, 32)?;
        if footer.starts_with(b"APETAGEX") {
            let size = u64::from(u32::from_le_bytes(footer[12..16].try_into().expect("4")));
            let flags = u32::from_le_bytes(footer[20..24].try_into().expect("4"));
            let total = size + if flags & (1 << 31) != 0 { 32 } else { 0 };
            if size < 32 || end < floor + total {
                return Err("The APE tag exceeds the file".into());
            }
            end -= total;
        }
    }
    Ok(end)
}

struct Layout {
    stream: Stream,
    /// Leading ID3v2 bytes; the stream starts here.
    prefix: u64,
    audio: Range<u64>,
    length: u64,
}

fn layout(file: &mut fs::File, stream: Stream) -> Result<Layout, String> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let prefix = leading_tags(file, length)?;
    if prefix + 4 > length || Stream::from_magic(&read_at(file, prefix, 4)?) != Some(stream) {
        return Err(invalid(stream, "missing stream header"));
    }
    let end = trailing_tags(file, prefix + 4, length)?;
    Ok(Layout {
        stream,
        prefix,
        audio: prefix..end,
        length,
    })
}

/// The file's tags around the stand-in stream.
fn stand_in_bytes(file: &mut fs::File, layout: &Layout) -> Result<Vec<u8>, String> {
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
fn split_stand_in(bytes: &[u8], stream: Stream) -> Result<(&[u8], &[u8]), String> {
    let mut position = 0usize;
    while let Some(size) = bytes.get(position..).and_then(id3v2_size) {
        position = usize::try_from(size)
            .ok()
            .and_then(|size| position.checked_add(size))
            .filter(|end| *end <= bytes.len())
            .ok_or("The written ID3v2 tag is damaged")?;
    }
    let stand_in = stream.stand_in();
    if bytes.get(position..position + stand_in.len()) != Some(stand_in.as_slice()) {
        return Err("The tag writer changed the audio stream; original file preserved".into());
    }
    Ok((&bytes[..position], &bytes[position + stand_in.len()..]))
}

fn parse_stand_in(bytes: &[u8], stream: Stream) -> Result<(TaggedFile, NativeTags), String> {
    let (mut file, native) = strict_or_lenient_id3v1(|mode| {
        let options = ParseOptions::new()
            .implicit_conversions(false)
            .read_properties(false)
            .parsing_mode(mode);
        let read = |mut reader: &mut dyn ReadSeek| {
            reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            Ok(if stream.mpeg_policy() {
                let file =
                    MpegFile::read_from(&mut reader, options).map_err(|e| error_message(&e))?;
                let native = NativeTags {
                    id3: file.id3v2().cloned(),
                    ape: file.ape().cloned(),
                    ..Default::default()
                };
                (TaggedFile::from(file), native)
            } else {
                let file =
                    ApeFile::read_from(&mut reader, options).map_err(|e| error_message(&e))?;
                let native = NativeTags {
                    id3: file.id3v2().cloned(),
                    ape: file.ape().cloned(),
                    ..Default::default()
                };
                (TaggedFile::from(file), native)
            })
        };
        crate::id3_frames::read_with_fallback(&mut Cursor::new(bytes), read)
            .map(|(parsed, _)| parsed)
    })?;
    ape_pictures(&native, &mut file)?;
    Ok((file, native))
}

/// The tags of `path` as lofty sees them on the stand-in.
pub(in crate::tags) fn load(path: &Path, stream: Stream) -> Result<TaggedFile, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let layout = layout(&mut file, stream)?;
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
) -> Result<lofty::properties::FileProperties, String> {
    let mut window = Window {
        file,
        range: layout.audio.clone(),
        position: 0,
    };
    let options = ParseOptions::new()
        .read_tags(false)
        .parsing_mode(ParsingMode::Strict);
    Ok(match layout.stream {
        Stream::Aac => AacFile::read_from(&mut window, options)
            .map_err(|e| error_message(&e))?
            .properties()
            .to_owned()
            .into(),
        _ => WavPackFile::read_from(&mut window, options)
            .map_err(|e| error_message(&e))?
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

fn properties(file: &mut fs::File, layout: &Layout) -> Result<Properties, String> {
    let stream = layout.stream;
    let start = layout.prefix + 4;
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

pub(in crate::tags) fn read(
    path: &Path,
    stream: Stream,
    include_cover_data: bool,
    rules: &[UserFieldMapping],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let mut source = fs::File::open(path).map_err(|e| e.to_string())?;
    let layout = layout(&mut source, stream)?;
    let (file, native) = parse_stand_in(&stand_in_bytes(&mut source, &layout)?, stream)?;
    let mut entry = describe_with_native(path, &file, &native, include_cover_data, rules, options)?;
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
pub(in crate::tags) fn stage(
    staging: &mut fs::File,
    original: &Path,
    staged_path: &Path,
    stream: Stream,
    mutation: impl FnOnce(&mut fs::File, &Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let mut source = fs::File::open(original).map_err(|e| e.to_string())?;
    let layout = layout(&mut source, stream)?;
    let bytes = stand_in_bytes(&mut source, &layout)?;
    // Beside the staging file: that directory is known to be writable.
    let dir = tempfile::Builder::new()
        .prefix(".opentag-stand-in-")
        .tempdir_in(
            staged_path
                .parent()
                .ok_or("The staging file has no folder")?,
        )
        .map_err(|e| e.to_string())?;
    let before = dir.path().join("before");
    let after = dir.path().join("after");
    fs::write(&before, &bytes).map_err(|e| e.to_string())?;
    fs::write(&after, &bytes).map_err(|e| e.to_string())?;
    let mut edited = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&after)
        .map_err(|e| e.to_string())?;
    // The stand-in's own rewrite cost says nothing about the real file.
    let scope = crate::write_cost::Scope::begin();
    mutation(&mut edited, &before, &after)?;
    scope.finish();
    drop(edited);
    let edited = fs::read(&after).map_err(|e| e.to_string())?;
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
    staging.sync_all().map_err(|e| e.to_string())?;
    if !same_bytes(original, layout.audio, staged_path, prefix.len() as u64)? {
        return Err("The audio stream changed while writing; original file preserved".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
