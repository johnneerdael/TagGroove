//! Read-only placeholders measured from the file bytes rather than from a
//! parsed tag: the sizes of the tags around the audio (`_TAG_SIZE`,
//! `_TAG_SIZE_PREPENDED`, `_TAG_SIZE_APPENDED`), the MPEG bitrate type and
//! encoder string from the Xing/Info, VBRI and LAME headers (`_VBR`,
//! `_TOOL`), the Monkey's Audio compression level (`_VBR`), the volume label
//! (`_VOLUME`) and where the audio-only MD5 (`_MD5AUDIO`) comes from.
//! Anything that cannot be established stays absent.
use super::Fields;
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    ops::Range,
    path::Path,
};

fn put(fields: &mut Fields, name: &str, value: impl ToString) {
    fields.insert(name.into(), vec![value.to_string()]);
}

fn read_at(file: &mut File, at: u64, length: usize) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(at))?;
    let mut bytes = Vec::with_capacity(length);
    file.take(length as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn syncsafe(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0, |size, byte| (size << 7) | u64::from(byte & 0x7F))
}

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Tag bytes before and after the audio of an ID3/APE-tagged stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TagSpan {
    pub prepended: u64,
    pub appended: u64,
    pub length: u64,
}

impl TagSpan {
    pub fn audio(&self) -> Range<u64> {
        self.prepended..self.length.saturating_sub(self.appended)
    }
}

/// ID3v2 tags at the start; APEv1/v2, Lyrics3v2, ID3v1 and appended ID3v2
/// tags (with a footer) at the end, in any order.
pub(super) fn tag_span(file: &mut File) -> io::Result<TagSpan> {
    let length = file.metadata()?.len();
    let mut prepended = 0u64;
    loop {
        let header = read_at(file, prepended, 10)?;
        if header.len() < 10 || &header[..3] != b"ID3" || header[3] == 0xFF {
            break;
        }
        let footer = if header[3] >= 4 && header[5] & 0x10 != 0 {
            10
        } else {
            0
        };
        let next = prepended + 10 + syncsafe(&header[6..10]) + footer;
        if next > length {
            break;
        }
        prepended = next;
    }
    let mut end = length;
    while end > prepended {
        let available = end - prepended;
        if available >= 128 && read_at(file, end - 128, 3)? == b"TAG" {
            end -= 128;
            continue;
        }
        if available >= 32 {
            let footer = read_at(file, end - 32, 32)?;
            if &footer[..8] == b"APETAGEX" {
                let size = u64::from(le32(&footer, 12).unwrap_or(0));
                let header = if le32(&footer, 20).unwrap_or(0) & 0x8000_0000 != 0 {
                    32
                } else {
                    0
                };
                let whole = size + header;
                if size >= 32 && whole <= available {
                    end -= whole;
                    continue;
                }
            }
        }
        if available >= 15 {
            let trailer = read_at(file, end - 15, 15)?;
            if &trailer[6..] == b"LYRICS200" {
                let size = std::str::from_utf8(&trailer[..6])
                    .ok()
                    .and_then(|digits| digits.parse::<u64>().ok());
                if let Some(whole) = size.map(|size| size + 15).filter(|w| *w <= available) {
                    end -= whole;
                    continue;
                }
            }
        }
        if available >= 20 {
            let footer = read_at(file, end - 10, 10)?;
            if &footer[..3] == b"3DI" {
                let whole = syncsafe(&footer[6..10]) + 20;
                if whole <= available {
                    end -= whole;
                    continue;
                }
            }
        }
        break;
    }
    Ok(TagSpan {
        prepended,
        appended: length - end,
        length,
    })
}

/// Sizes of the FLAC metadata blocks holding tags: Vorbis comments and
/// pictures, with their block headers.
fn flac_tag_bytes(file: &mut File, start: u64) -> io::Result<u64> {
    if read_at(file, start, 4)? != b"fLaC" {
        return Ok(0);
    }
    let mut at = start + 4;
    let mut total = 0;
    loop {
        let header = read_at(file, at, 4)?;
        if header.len() < 4 {
            return Ok(total);
        }
        let size = u64::from(u32::from_be_bytes([0, header[1], header[2], header[3]]));
        if matches!(header[0] & 0x7F, 4 | 6) {
            total += 4 + size;
        }
        if header[0] & 0x80 != 0 {
            return Ok(total);
        }
        at += 4 + size;
    }
}

/// Streams whose tags sit before and after the audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stream {
    Mpeg,
    MonkeysAudio,
    Flac,
    /// AAC/ADTS, Musepack, WavPack and the like: tag sizes only.
    Other,
}

/// `_TAG_SIZE*`, and `_VBR`/`_TOOL` where the stream header names them,
/// from one open of the file.
pub(super) fn stream_fields(fields: &mut Fields, path: &Path, stream: Stream) {
    let Ok(mut file) = File::open(path) else {
        return;
    };
    let Ok(span) = tag_span(&mut file) else {
        return;
    };
    let mut prepended = span.prepended;
    if stream == Stream::Flac {
        let Ok(blocks) = flac_tag_bytes(&mut file, span.prepended) else {
            return;
        };
        prepended += blocks;
    }
    put(fields, "_TAG_SIZE", prepended + span.appended);
    put(fields, "_TAG_SIZE_PREPENDED", prepended);
    put(fields, "_TAG_SIZE_APPENDED", span.appended);
    match stream {
        Stream::Mpeg => mpeg_fields(fields, &mut file, &span),
        Stream::MonkeysAudio => ape_fields(fields, &mut file, &span),
        Stream::Flac | Stream::Other => {}
    }
}

/// An MPEG audio frame header: whether it is MPEG-1 and mono decide where
/// the Xing/Info header sits.
fn mpeg_header(bytes: &[u8]) -> Option<(bool, bool)> {
    let [a, b, c, d] = *bytes.get(..4)? else {
        return None;
    };
    let version = (b >> 3) & 3;
    let layer = (b >> 1) & 3;
    if a != 0xFF || b & 0xE0 != 0xE0 || version == 1 || layer == 0 {
        return None;
    }
    if c >> 4 == 15 || (c >> 2) & 3 == 3 {
        return None;
    }
    Some((version == 3, d >> 6 == 3))
}

/// The bitrate type of an MPEG stream and the encoder named by its LAME tag.
fn mpeg_fields(fields: &mut Fields, file: &mut File, span: &TagSpan) {
    let Ok(bytes) = read_at(file, span.prepended, 4096) else {
        return;
    };
    // The first frame follows the tag, perhaps after a little padding.
    let Some((at, (mpeg1, mono))) = (0..bytes.len().min(2048))
        .find_map(|at| mpeg_header(&bytes[at..]).map(|header| (at, header)))
    else {
        return;
    };
    let frame = &bytes[at..];
    let side_info = match (mpeg1, mono) {
        (true, false) => 32,
        (true, true) | (false, false) => 17,
        (false, true) => 9,
    };
    let xing = frame.get(4 + side_info..);
    let kind = xing.and_then(|x| x.get(..4));
    let mut vbr = match kind {
        Some(b"Xing") => "VBR",
        Some(b"Info") => "CBR",
        _ if frame.get(36..40) == Some(b"VBRI") => "VBR",
        _ => "CBR",
    };
    if let Some(xing) = xing.filter(|_| matches!(kind, Some(b"Xing" | b"Info"))) {
        let flags = xing
            .get(4..8)
            .map_or(0, |f| u32::from_be_bytes([f[0], f[1], f[2], f[3]]));
        let mut encoder = 8;
        for (flag, length) in [(1, 4), (2, 4), (4, 100), (8, 4)] {
            if flags & flag != 0 {
                encoder += length;
            }
        }
        if let Some(name) = xing.get(encoder..encoder + 9) {
            let text: String = name
                .iter()
                .take_while(|byte| **byte != 0)
                .map(|byte| char::from(*byte))
                .collect();
            let text = text.trim();
            if text.len() >= 4
                && text.chars().all(|c| c.is_ascii_graphic() || c == ' ')
                && text.starts_with(|c: char| c.is_ascii_alphabetic())
            {
                put(fields, "_TOOL", text);
                // LAME's VBR method: 1 and 8 constant, 2 and 9 average.
                if text.starts_with("LAME") {
                    match xing.get(encoder + 9).map(|byte| byte & 0x0F) {
                        Some(1 | 8) => vbr = "CBR",
                        Some(2 | 9) => vbr = "ABR",
                        Some(3..=6) => vbr = "VBR",
                        _ => {}
                    }
                }
            }
        }
    }
    put(fields, "_VBR", vbr);
}

/// Monkey's Audio compression level as `_VBR`.
fn ape_fields(fields: &mut Fields, file: &mut File, span: &TagSpan) {
    let Ok(bytes) = read_at(file, span.prepended, 64) else {
        return;
    };
    if bytes.get(..4) != Some(b"MAC ") || bytes.len() < 8 {
        return;
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    let level_at = if version >= 3980 {
        // The descriptor names its own length; the header starts with the level.
        le32(&bytes, 8).map(|length| length as usize)
    } else {
        Some(6)
    };
    let Some(level) = level_at
        .and_then(|at| bytes.get(at..at + 2))
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
    else {
        return;
    };
    let name = match level {
        1000 => "Fast",
        2000 => "Normal",
        3000 => "High",
        4000 => "Extra High",
        5000 => "Insane",
        _ => return,
    };
    put(fields, "_VBR", name);
}

/// Where the MD5 of a file's audio comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioDigest {
    /// Stored by the encoder (FLAC STREAMINFO, WavPack MD5 block).
    Stored(String),
    /// Hash these bytes: the audio between the ID3/APE tags.
    Range(Range<u64>),
    /// Not available for this format.
    Unavailable,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `%_md5audio%`: native for FLAC and WavPack, otherwise the bytes between
/// the tags of streams that carry only ID3 and APE tags (MPEG, AAC/ADTS,
/// Monkey's Audio, Musepack, TTA, TAK, OptimFROG). A FLAC file without a
/// stored checksum, or a WavPack file without the MD5 block, has none.
pub fn audio_digest(path: &Path) -> Result<AudioDigest, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let span = tag_span(&mut file).map_err(|e| e.to_string())?;
    let magic = read_at(&mut file, span.prepended, 4).map_err(|e| e.to_string())?;
    match magic.get(..4) {
        Some(b"fLaC") => {
            let block = read_at(&mut file, span.prepended + 4, 38).map_err(|e| e.to_string())?;
            Ok(match block.get(22..38) {
                Some(md5) if block[0] & 0x7F == 0 && md5.iter().any(|b| *b != 0) => {
                    AudioDigest::Stored(hex(md5))
                }
                _ => AudioDigest::Unavailable,
            })
        }
        Some(b"wvpk") => {
            let header = read_at(&mut file, span.prepended, 32).map_err(|e| e.to_string())?;
            let size = le32(&header, 4).unwrap_or(0) as usize;
            let block = read_at(
                &mut file,
                span.prepended + 32,
                size.saturating_sub(24).min(1 << 20),
            )
            .map_err(|e| e.to_string())?;
            Ok(wavpack_md5(&block).map_or(AudioDigest::Unavailable, AudioDigest::Stored))
        }
        Some(b"MAC " | b"MPCK" | b"TTA1" | b"tBaK" | b"OFR " | [b'M', b'P', b'+', _]) => {
            Ok(AudioDigest::Range(span.audio()))
        }
        Some(header) if mpeg_header(header).is_some() || adts(header) => {
            Ok(AudioDigest::Range(span.audio()))
        }
        _ => Ok(AudioDigest::Unavailable),
    }
}

fn adts(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] & 0xF6 == 0xF0
}

/// The ID_MD5_CHECKSUM sub-block of a WavPack block's metadata.
fn wavpack_md5(mut metadata: &[u8]) -> Option<String> {
    while metadata.len() >= 2 {
        let id = metadata[0];
        let (words, header) = if id & 0x80 != 0 {
            let size = metadata.get(1..4)?;
            (
                usize::from(size[0]) | (usize::from(size[1]) << 8) | (usize::from(size[2]) << 16),
                4,
            )
        } else {
            (usize::from(metadata[1]), 2)
        };
        let length = words * 2;
        let data = metadata.get(header..header + length)?;
        if id & 0x3F == 0x26 && data.len() >= 16 {
            return Some(hex(&data[..16]));
        }
        metadata = &metadata[header + length..];
    }
    None
}

/// `%_volume%`: the label of the volume holding `path`.
pub(super) fn volume_label(path: &Path) -> Option<String> {
    platform::volume_label(path).filter(|label| !label.is_empty())
}

#[cfg(target_os = "macos")]
mod platform {
    use std::{
        path::{Component, Path},
        sync::Mutex,
        time::{Duration, Instant},
    };

    /// External volumes mount as /Volumes/<label>; the startup volume is
    /// listed there as a link to `/`.
    pub fn volume_label(path: &Path) -> Option<String> {
        let mut components = path.components();
        if components.next() == Some(Component::RootDir) {
            if let (Some(Component::Normal(volumes)), Some(Component::Normal(label))) =
                (components.next(), components.next())
            {
                if volumes == "Volumes" {
                    return Some(label.to_string_lossy().into_owned());
                }
            }
        }
        static ROOT: Mutex<Option<(Instant, Option<String>)>> = Mutex::new(None);
        let mut cached = ROOT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((at, label)) = cached.as_ref() {
            if at.elapsed() < Duration::from_secs(30) {
                return label.clone();
            }
        }
        let label = std::fs::read_dir("/Volumes").ok().and_then(|entries| {
            entries.flatten().find_map(|entry| {
                let target = std::fs::read_link(entry.path()).ok()?;
                (target == Path::new("/")).then(|| entry.file_name().to_string_lossy().into_owned())
            })
        });
        *cached = Some((Instant::now(), label.clone()));
        label
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use std::{
        path::{Path, PathBuf},
        sync::Mutex,
        time::{Duration, Instant},
    };

    /// Undo `\ooo` (mount table) or `\xhh` (udev link name) byte escapes.
    fn unescape(text: &str, hex: bool) -> String {
        let (marker, digits, radix) = if hex { ("\\x", 2, 16) } else { ("\\", 3, 8) };
        let mut bytes = Vec::new();
        let mut rest = text;
        while let Some(at) = rest.find(marker) {
            bytes.extend(&rest.as_bytes()[..at]);
            let start = at + marker.len();
            match rest
                .get(start..start + digits)
                .and_then(|code| u8::from_str_radix(code, radix).ok())
            {
                Some(byte) => {
                    bytes.push(byte);
                    rest = &rest[start + digits..];
                }
                None => {
                    bytes.extend(marker.as_bytes());
                    rest = &rest[start..];
                }
            }
        }
        bytes.extend(rest.as_bytes());
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Mount points and their volume labels.
    type Mounts = Vec<(PathBuf, String)>;

    /// Mount points with the label of their device, from /proc/self/mounts
    /// and /dev/disk/by-label.
    fn mounts() -> Mounts {
        let labels: Vec<(PathBuf, String)> = std::fs::read_dir("/dev/disk/by-label")
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|entry| {
                        let device = std::fs::canonicalize(entry.path()).ok()?;
                        let name = entry.file_name().to_string_lossy().into_owned();
                        Some((device, unescape(&name, true)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let Ok(table) = std::fs::read_to_string("/proc/self/mounts") else {
            return Vec::new();
        };
        table
            .lines()
            .filter_map(|line| {
                let mut parts = line.split(' ');
                let device = std::fs::canonicalize(unescape(parts.next()?, false)).ok()?;
                let mount = PathBuf::from(unescape(parts.next()?, false));
                let label = labels.iter().find(|(d, _)| *d == device)?.1.clone();
                Some((mount, label))
            })
            .collect()
    }

    pub fn volume_label(path: &Path) -> Option<String> {
        static CACHE: Mutex<Option<(Instant, Mounts)>> = Mutex::new(None);
        let mut cached = CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if cached
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() >= Duration::from_secs(30))
        {
            *cached = Some((Instant::now(), mounts()));
        }
        let (_, table) = cached.as_ref()?;
        table
            .iter()
            .filter(|(mount, _)| path.starts_with(mount))
            .max_by_key(|(mount, _)| mount.components().count())
            .map(|(_, label)| label.clone())
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn mount_table_and_udev_escapes_are_decoded() {
            assert_eq!(super::unescape("My\\x20Disk\\x2", true), "My Disk\\x2");
            assert_eq!(super::unescape("/media/a\\040b", false), "/media/a b");
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        os::windows::ffi::OsStrExt,
        path::{Component, Path},
    };

    #[link(name = "kernel32")]
    extern "system" {
        fn GetVolumeInformationW(
            root: *const u16,
            name: *mut u16,
            name_size: u32,
            serial: *mut u32,
            component_length: *mut u32,
            flags: *mut u32,
            file_system: *mut u16,
            file_system_size: u32,
        ) -> i32;
    }

    pub fn volume_label(path: &Path) -> Option<String> {
        let Some(Component::Prefix(prefix)) = path.components().next() else {
            return None;
        };
        let mut root: Vec<u16> = prefix.as_os_str().encode_wide().collect();
        root.extend("\\".encode_utf16());
        root.push(0);
        let mut name = [0u16; 261];
        // SAFETY: `root` is NUL-terminated and `name` holds `name.len()` units;
        // the optional outputs are null, which the API accepts.
        let ok = unsafe {
            GetVolumeInformationW(
                root.as_ptr(),
                name.as_mut_ptr(),
                name.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        };
        if ok == 0 {
            return None;
        }
        let length = name
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(name.len());
        Some(String::from_utf16_lossy(&name[..length]))
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    pub fn volume_label(_: &std::path::Path) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn mounted_volumes_are_named_by_their_mount_folder() {
        assert_eq!(
            volume_label(Path::new("/Volumes/Music Drive/a/b.mp3")).as_deref(),
            Some("Music Drive")
        );
    }

    fn id3v2(body: usize) -> Vec<u8> {
        let mut tag = b"ID3\x04\x00\x00".to_vec();
        let size = body as u32;
        tag.extend([
            ((size >> 21) & 0x7F) as u8,
            ((size >> 14) & 0x7F) as u8,
            ((size >> 7) & 0x7F) as u8,
            (size & 0x7F) as u8,
        ]);
        tag.extend(vec![0; body]);
        tag
    }

    fn ape(items: usize, header: bool) -> Vec<u8> {
        let size = (items + 32) as u32;
        let block = |flags: u32| {
            let mut b = b"APETAGEX".to_vec();
            b.extend(2000u32.to_le_bytes());
            b.extend(size.to_le_bytes());
            b.extend(1u32.to_le_bytes());
            b.extend(flags.to_le_bytes());
            b.extend([0; 8]);
            b
        };
        let mut tag = Vec::new();
        if header {
            tag.extend(block(0xA000_0000));
        }
        tag.extend(vec![0; items]);
        tag.extend(block(if header { 0x8000_0000 } else { 0 }));
        tag
    }

    /// One MPEG-1 Layer III stereo frame with an Info or Xing header and a
    /// LAME tag naming `method` as its VBR method.
    fn mpeg_frame(kind: &[u8; 4], encoder: &[u8; 9], method: u8) -> Vec<u8> {
        let mut frame = vec![0xFF, 0xFB, 0x90, 0x00];
        frame.extend([0; 32]);
        frame.extend(kind);
        frame.extend(15u32.to_be_bytes());
        frame.extend(10u32.to_be_bytes());
        frame.extend(4000u32.to_be_bytes());
        frame.extend([0; 100]);
        frame.extend(0u32.to_be_bytes());
        frame.extend(encoder);
        frame.push(0x10 | method);
        frame.resize(417, 0);
        frame
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn tag_span_counts_id3v2_ape_lyrics3_and_id3v1() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = id3v2(100);
        bytes.extend(mpeg_frame(b"Info", b"LAME3.100", 1));
        bytes.extend(ape(40, true));
        let mut lyrics = b"LYRICSBEGIN".to_vec();
        lyrics.extend(b"IND00002");
        lyrics.extend(b"10");
        let size = format!("{:06}", lyrics.len());
        lyrics.extend(size.as_bytes());
        lyrics.extend(b"LYRICS200");
        bytes.extend(&lyrics);
        let mut id3v1 = b"TAG".to_vec();
        id3v1.resize(128, 0);
        bytes.extend(id3v1);
        let path = write(dir.path(), "a.mp3", &bytes);
        let span = tag_span(&mut File::open(&path).unwrap()).unwrap();
        assert_eq!(span.prepended, 110);
        assert_eq!(span.appended, (40 + 64 + lyrics.len() + 128) as u64);
        assert_eq!(span.audio(), 110..110 + 417);
        let mut fields = Fields::new();
        stream_fields(&mut fields, &path, Stream::Other);
        assert_eq!(fields["_TAG_SIZE_PREPENDED"], ["110"]);
        assert_eq!(
            fields["_TAG_SIZE_APPENDED"],
            [(40 + 64 + lyrics.len() + 128).to_string()]
        );
        assert_eq!(
            fields["_TAG_SIZE"],
            [(110 + 40 + 64 + lyrics.len() + 128).to_string()]
        );
        // Untagged audio has no tag bytes.
        let bare = write(
            dir.path(),
            "bare.mp3",
            &mpeg_frame(b"Info", b"LAME3.100", 1),
        );
        let mut fields = Fields::new();
        stream_fields(&mut fields, &bare, Stream::Other);
        assert_eq!(fields["_TAG_SIZE"], ["0"]);
    }

    #[test]
    fn mpeg_bitrate_type_and_encoder_come_from_the_info_and_lame_headers() {
        let dir = tempfile::tempdir().unwrap();
        for (kind, method, expected) in [
            (b"Info", 1, "CBR"),
            (b"Xing", 4, "VBR"),
            (b"Xing", 2, "ABR"),
            (b"Info", 0, "CBR"),
        ] {
            let mut bytes = id3v2(20);
            bytes.extend(mpeg_frame(kind, b"LAME3.100", method));
            let path = write(dir.path(), "x.mp3", &bytes);
            let mut fields = Fields::new();
            stream_fields(&mut fields, &path, Stream::Mpeg);
            assert_eq!(fields["_VBR"], [expected], "{kind:?} {method}");
            assert_eq!(fields["_TOOL"], ["LAME3.100"]);
        }
        let mut plain = vec![0xFF, 0xFB, 0x90, 0x00];
        plain.resize(417, 0);
        let path = write(dir.path(), "plain.mp3", &plain);
        let mut fields = Fields::new();
        stream_fields(&mut fields, &path, Stream::Mpeg);
        assert_eq!(fields["_VBR"], ["CBR"]);
        assert!(!fields.contains_key("_TOOL"));
        let mut vbri = vec![0xFF, 0xFB, 0x90, 0x00];
        vbri.extend([0; 32]);
        vbri.extend(b"VBRI");
        vbri.resize(417, 0);
        let path = write(dir.path(), "vbri.mp3", &vbri);
        let mut fields = Fields::new();
        stream_fields(&mut fields, &path, Stream::Mpeg);
        assert_eq!(fields["_VBR"], ["VBR"]);
        // A stream that is not MPEG gets no bitrate type.
        let path = write(dir.path(), "tta.tta", b"TTA1\x01\x00\x02\x00");
        let mut fields = Fields::new();
        stream_fields(&mut fields, &path, Stream::Mpeg);
        assert!(!fields.contains_key("_VBR"));
        assert_eq!(fields["_TAG_SIZE"], ["0"]);
    }

    #[test]
    fn monkeys_audio_compression_level_is_the_bitrate_type() {
        let dir = tempfile::tempdir().unwrap();
        let mut new = b"MAC ".to_vec();
        new.extend(3990u16.to_le_bytes());
        new.extend([0, 0]);
        new.extend(52u32.to_le_bytes());
        new.resize(52, 0);
        new.extend(4000u16.to_le_bytes());
        new.resize(100, 0);
        let mut fields = Fields::new();
        stream_fields(
            &mut fields,
            &write(dir.path(), "new.ape", &new),
            Stream::MonkeysAudio,
        );
        assert_eq!(fields["_VBR"], ["Extra High"]);
        let mut old = b"MAC ".to_vec();
        old.extend(3970u16.to_le_bytes());
        old.extend(2000u16.to_le_bytes());
        old.resize(64, 0);
        let mut fields = Fields::new();
        stream_fields(
            &mut fields,
            &write(dir.path(), "old.ape", &old),
            Stream::MonkeysAudio,
        );
        assert_eq!(fields["_VBR"], ["Normal"]);
    }

    #[test]
    fn audio_digest_is_native_for_flac_and_wavpack_and_a_range_for_id3_streams() {
        let dir = tempfile::tempdir().unwrap();
        let mut flac = b"fLaC".to_vec();
        flac.extend([0x80, 0, 0, 34]);
        let mut info = vec![0u8; 34];
        info[18..34].copy_from_slice(&[0xAB; 16]);
        flac.extend(info);
        let path = write(dir.path(), "a.flac", &flac);
        assert_eq!(
            audio_digest(&path).unwrap(),
            AudioDigest::Stored("ab".repeat(16))
        );
        flac[26..42].fill(0);
        let path = write(dir.path(), "zero.flac", &flac);
        assert_eq!(audio_digest(&path).unwrap(), AudioDigest::Unavailable);

        let mut wavpack = b"wvpk".to_vec();
        let metadata = {
            let mut m = vec![0x21, 1, 0, 0];
            m.extend([0x26, 8]);
            m.extend([0x5A; 16]);
            m
        };
        wavpack.extend(((24 + metadata.len()) as u32).to_le_bytes());
        wavpack.resize(32, 0);
        wavpack.extend(&metadata);
        let path = write(dir.path(), "a.wv", &wavpack);
        assert_eq!(
            audio_digest(&path).unwrap(),
            AudioDigest::Stored("5a".repeat(16))
        );

        let mut mp3 = id3v2(30);
        mp3.extend(mpeg_frame(b"Info", b"LAME3.100", 1));
        mp3.extend(ape(10, false));
        let path = write(dir.path(), "a.mp3", &mp3);
        assert_eq!(
            audio_digest(&path).unwrap(),
            AudioDigest::Range(40..40 + 417)
        );
        let path = write(dir.path(), "a.ogg", b"OggS\0\x02");
        assert_eq!(audio_digest(&path).unwrap(), AudioDigest::Unavailable);
    }

    #[test]
    fn flac_tag_size_counts_comment_and_picture_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let mut flac = b"fLaC".to_vec();
        flac.extend([0, 0, 0, 34]);
        flac.extend([0; 34]);
        flac.extend([4, 0, 0, 20]);
        flac.extend([0; 20]);
        flac.extend([1, 0, 0, 8]);
        flac.extend([0; 8]);
        flac.extend([0x86, 0, 0, 30]);
        flac.extend([0; 30]);
        flac.extend([0xFF, 0xF8]);
        let path = write(dir.path(), "a.flac", &flac);
        let mut fields = Fields::new();
        stream_fields(&mut fields, &path, Stream::Flac);
        assert_eq!(fields["_TAG_SIZE"], ["58"]);
        assert_eq!(fields["_TAG_SIZE_PREPENDED"], ["58"]);
        assert_eq!(fields["_TAG_SIZE_APPENDED"], ["0"]);
    }
}
