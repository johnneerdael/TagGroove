//! Compressed and encrypted ID3v2 frames.
//!
//! lofty decompresses compressed frames on read but cannot write them back,
//! and it fails on encrypted ID3v2.3 frames and on frames that are both
//! compressed and encrypted. Reading such a tag falls back to a view of the
//! file ([`Overlay`]) in which every encrypted frame is renamed to an opaque
//! placeholder lofty keeps as binary data. Writing ([`Passthrough`]) copies
//! every compressed or encrypted frame the edit left alone from the original
//! tag byte for byte; frames converted to another ID3v2 revision are written
//! decompressed, and encrypted frames cannot be converted.
use crate::batch_errors::ErrorCause;
use lofty::id3::v2::{Frame, Id3v2Tag};
use std::io::{self, Read, Seek, SeekFrom};

/// The ID3v2.3 and ID3v2.4 frame flag bits for compression and encryption.
fn flagged(major: u8, flags: [u8; 2]) -> (bool, bool) {
    match major {
        3 => (flags[1] & 0x80 != 0, flags[1] & 0x40 != 0),
        4 => (flags[1] & 0x08 != 0, flags[1] & 0x04 != 0),
        _ => (false, false),
    }
}

fn syncsafe(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0u32, |v, b| {
        (b & 0x80 == 0).then_some((v << 7) | u32::from(*b))
    })
}
fn syncsafe_bytes(value: usize) -> Option<[u8; 4]> {
    let value = u32::try_from(value).ok().filter(|v| *v <= 0x0fff_ffff)?;
    Some([
        ((value >> 21) & 127) as u8,
        ((value >> 14) & 127) as u8,
        ((value >> 7) & 127) as u8,
        (value & 127) as u8,
    ])
}

/// One compressed or encrypted frame of a stored tag.
#[derive(Debug, Clone)]
pub(crate) struct RawFrame {
    /// Offset of the frame header in the file.
    pub offset: u64,
    pub id: [u8; 4],
    /// Header and body, exactly as stored.
    pub bytes: Vec<u8>,
    pub encrypted: bool,
}

/// The compressed and encrypted frames of a stored ID3v2.3 or v2.4 tag.
#[derive(Debug, Clone)]
pub(crate) struct RawTag {
    pub major: u8,
    /// Tag header flags.
    pub flags: u8,
    pub frames: Vec<RawFrame>,
}

/// Where the tag of the file starts: at the beginning, or in the `ID3 `
/// chunk of a WAV or AIFF file.
fn locate(reader: &mut (impl Read + Seek)) -> io::Result<Option<u64>> {
    let length = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;
    let mut head = [0u8; 12];
    if reader.read_exact(&mut head).is_err() {
        return Ok(None);
    }
    if &head[..3] == b"ID3" {
        return Ok(Some(0));
    }
    let little = match &head[..4] {
        b"RIFF" => true,
        b"FORM" => false,
        _ => return Ok(None),
    };
    let mut position = 12u64;
    while position + 8 <= length {
        reader.seek(SeekFrom::Start(position))?;
        let mut chunk = [0u8; 8];
        reader.read_exact(&mut chunk)?;
        let size = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
        let size = u64::from(if little { size.swap_bytes() } else { size });
        if &chunk[..4] == b"ID3 " || &chunk[..4] == b"id3 " {
            return Ok(Some(position + 8));
        }
        position += 8 + size + size % 2;
    }
    Ok(None)
}

/// Scan the frame headers of the file's ID3v2 tag. `None` when there is no
/// ID3v2.3 or v2.4 tag.
pub(crate) fn scan(reader: &mut (impl Read + Seek)) -> io::Result<Option<RawTag>> {
    let Some(start) = locate(reader)? else {
        return Ok(None);
    };
    reader.seek(SeekFrom::Start(start))?;
    let mut header = [0u8; 10];
    if reader.read_exact(&mut header).is_err() || &header[..3] != b"ID3" {
        return Ok(None);
    }
    let major = header[3];
    let Some(size) = syncsafe(&header[6..10]).filter(|_| matches!(major, 3 | 4)) else {
        return Ok(None);
    };
    let end = start + 10 + u64::from(size);
    let mut position = start + 10;
    if header[5] & 0x40 != 0 {
        let mut extended = [0u8; 4];
        reader.read_exact(&mut extended)?;
        position += if major == 3 {
            4 + u64::from(u32::from_be_bytes(extended))
        } else {
            u64::from(syncsafe(&extended).unwrap_or(0))
        };
    }
    let mut frames = Vec::new();
    while position + 10 <= end {
        reader.seek(SeekFrom::Start(position))?;
        let mut head = [0u8; 10];
        reader.read_exact(&mut head)?;
        if head[0] == 0 || !head[..4].iter().all(|b| b.is_ascii_alphanumeric()) {
            break;
        }
        let Some(size) = (if major == 3 {
            Some(u32::from_be_bytes([head[4], head[5], head[6], head[7]]))
        } else {
            syncsafe(&head[4..8])
        }) else {
            break;
        };
        let next = position + 10 + u64::from(size);
        if next > end {
            break;
        }
        let (compressed, encrypted) = flagged(major, [head[8], head[9]]);
        if compressed || encrypted {
            let mut bytes = head.to_vec();
            bytes.resize(10 + size as usize, 0);
            reader.read_exact(&mut bytes[10..])?;
            frames.push(RawFrame {
                offset: position,
                id: head[..4].try_into().expect("4 bytes"),
                bytes,
                encrypted,
            });
        }
        position = next;
    }
    Ok(Some(RawTag {
        major,
        flags: header[5],
        frames,
    }))
}

/// The identifier an encrypted frame is read under: `Y` (unused by the
/// standard) and its position among the encrypted frames in base 36.
fn placeholder(index: usize) -> Option<[u8; 4]> {
    const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if index >= 36 * 36 * 36 {
        return None;
    }
    Some([
        b'Y',
        DIGITS[index / 1296],
        DIGITS[index / 36 % 36],
        DIGITS[index % 36],
    ])
}
/// Whether `frame` stands in for an encrypted frame (see [`Overlay`]).
pub(crate) fn is_placeholder(frame: &Frame<'_>) -> bool {
    let id = frame.id().as_str().as_bytes();
    matches!(frame, Frame::Binary(_))
        && id.len() == 4
        && id[0] == b'Y'
        && id[1..]
            .iter()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}
/// The stored identifiers of the encrypted frames, by placeholder.
pub(crate) fn placeholder_names(tag: &RawTag) -> Vec<(String, String)> {
    tag.frames
        .iter()
        .filter(|frame| frame.encrypted)
        .enumerate()
        .filter_map(|(index, frame)| {
            Some((
                String::from_utf8_lossy(&placeholder(index)?).into_owned(),
                String::from_utf8_lossy(&frame.id).into_owned(),
            ))
        })
        .collect()
}

/// Placeholder headers for the encrypted frames of the file's ID3v2 tag
/// (see [`Overlay`]). `None` when there is none, so an overlay cannot help.
fn encrypted_patches(reader: &mut (impl Read + Seek)) -> io::Result<Option<(Patches, RawTag)>> {
    let Some(tag) = scan(reader)? else {
        return Ok(None);
    };
    let mut patches = Vec::new();
    for (index, frame) in tag.frames.iter().filter(|f| f.encrypted).enumerate() {
        let Some(id) = placeholder(index) else {
            return Ok(None);
        };
        let mut header = frame.bytes[..10].to_vec();
        header[..4].copy_from_slice(&id);
        header[8] = 0;
        header[9] = 0;
        patches.push((frame.offset, header));
    }
    Ok((!patches.is_empty()).then_some((patches, tag)))
}

/// Bytes shown instead of the stored ones, by offset.
type Patches = crate::ape_footer::Patches;

/// Lofty reads a v2.3 extended header using the v2.4 layout. Present its
/// validated frames followed by extra zero padding instead, keeping the tag,
/// container and media offsets unchanged. Only this read view is normalized;
/// scan/Passthrough still see the original flags and physical frame bytes.
fn v23_extended_view(reader: &mut (impl Read + Seek)) -> io::Result<(Patches, u64)> {
    let position = reader.stream_position()?;
    let result = (|| {
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid ID3v2.3 extended header",
            )
        };
        let Some(start) = locate(reader)? else {
            return Ok((Vec::new(), 0));
        };
        reader.seek(SeekFrom::Start(start))?;
        let mut header = [0; 10];
        reader.read_exact(&mut header)?;
        if header[3] != 3 || header[5] & 0x40 == 0 {
            return Ok((Vec::new(), 0));
        }
        // A global unsynchronisation changes physical offsets, including the
        // extended header. Do not apply this non-unsynchronised view to it.
        if header[5] & 0x80 != 0 {
            return Err(io::Error::new(io::ErrorKind::Unsupported,
                "Combined ID3v2.3 extended header and unsynchronisation needs further verification; original file preserved"));
        }
        if header[5] & 0x1f != 0 {
            return Err(invalid());
        }
        let size = u64::from(syncsafe(&header[6..10]).ok_or_else(invalid)?);
        let length = reader.seek(SeekFrom::End(0))?;
        let end = start
            .checked_add(10)
            .and_then(|at| at.checked_add(size))
            .ok_or_else(invalid)?;
        if end > length || size < 10 {
            return Err(invalid());
        }
        if start != 0 {
            // locate() found an embedded RIFF/AIFF ID3 chunk. Do not read into
            // the next chunk even if the tag claims more bytes than its owner.
            reader.seek(SeekFrom::Start(0))?;
            let mut form = [0; 12];
            reader.read_exact(&mut form)?;
            reader.seek(SeekFrom::Start(start - 4))?;
            let mut chunk_size = [0; 4];
            reader.read_exact(&mut chunk_size)?;
            let size = if &form[..4] == b"RIFF" {
                u32::from_le_bytes(chunk_size)
            } else {
                u32::from_be_bytes(chunk_size)
            };
            if end > start + u64::from(size) {
                return Err(invalid());
            }
        }
        reader.seek(SeekFrom::Start(start + 10))?;
        let mut extended = [0; 10];
        reader.read_exact(&mut extended)?;
        let declared = u32::from_be_bytes(extended[..4].try_into().expect("4 bytes"));
        let flags = u16::from_be_bytes(extended[4..6].try_into().expect("2 bytes"));
        let has_crc = flags == 0x8000;
        if flags != 0 && !has_crc || declared != if has_crc { 10 } else { 6 } {
            return Err(invalid());
        }
        let shift = u64::from(declared) + 4;
        if shift > size {
            return Err(invalid());
        }
        let padding = u64::from(u32::from_be_bytes(
            extended[6..10].try_into().expect("4 bytes"),
        ));
        if padding > size - shift {
            return Err(invalid());
        }
        // All allocation is limited to the actual, validated metadata range.
        reader.seek(SeekFrom::Start(start + 10))?;
        let size_usize = usize::try_from(size).map_err(|_| invalid())?;
        let mut body = Vec::new();
        body.try_reserve_exact(size_usize)
            .map_err(io::Error::other)?;
        body.resize(size_usize, 0);
        reader.read_exact(&mut body)?;
        let frames_end = body.len() - padding as usize;
        if body[frames_end..].iter().any(|byte| *byte != 0) {
            return Err(invalid());
        }
        if has_crc {
            let expected = u32::from_be_bytes(body[10..14].try_into().expect("4 bytes"));
            if crc32fast::hash(&body[shift as usize..frames_end]) != expected {
                return Err(invalid());
            }
        }
        body.copy_within(shift as usize.., 0);
        body[(size - shift) as usize..].fill(0);
        Ok((
            vec![(start + 5, vec![header[5] & !0x40]), (start + 10, body)],
            shift,
        ))
    })();
    reader.seek(SeekFrom::Start(position))?;
    result
}

/// A reader that shows the file with some bytes replaced and some appended:
/// each encrypted ID3v2 frame renamed to a placeholder without flags, so
/// lofty reads its stored bytes as binary data, and the view of a trailing
/// APE tag lofty would misread (see `ape_footer`).
pub(crate) struct Overlay<'a, R> {
    inner: R,
    patches: &'a [(u64, Vec<u8>)],
    /// Bytes after the end of `inner`.
    tail: &'a [u8],
    length: u64,
    position: u64,
}
impl<'a, R: Read + Seek> Overlay<'a, R> {
    fn new(mut inner: R, patches: &'a [(u64, Vec<u8>)], tail: &'a [u8]) -> io::Result<Self> {
        let position = inner.stream_position()?;
        let length = inner.seek(SeekFrom::End(0))?;
        inner.seek(SeekFrom::Start(position))?;
        Ok(Self {
            inner,
            patches,
            tail,
            length,
            position,
        })
    }
}
impl<R: Read> Read for Overlay<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.length {
            let tail = self
                .tail
                .get((self.position - self.length) as usize..)
                .unwrap_or_default();
            let read = tail.len().min(buffer.len());
            buffer[..read].copy_from_slice(&tail[..read]);
            self.position += read as u64;
            return Ok(read);
        }
        let take = buffer
            .len()
            .min(usize::try_from(self.length - self.position).unwrap_or(usize::MAX));
        let read = self.inner.read(&mut buffer[..take])?;
        let window = self.position..self.position + read as u64;
        for (offset, bytes) in self.patches {
            // Copy only the intersection: a normalized extended-header patch
            // may span the whole tag, while the reader requests small buffers.
            let start = window.start.max(*offset);
            let end = window.end.min(offset.saturating_add(bytes.len() as u64));
            if start < end {
                buffer[(start - window.start) as usize..(end - window.start) as usize]
                    .copy_from_slice(&bytes[(start - offset) as usize..(end - offset) as usize]);
            }
        }
        self.position = window.end;
        Ok(read)
    }
}
impl<R: Seek> Seek for Overlay<'_, R> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let end = self.length + self.tail.len() as u64;
        let target = match to {
            SeekFrom::Start(at) => Some(at),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => end.checked_add_signed(delta),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before the start"))?;
        self.inner.seek(SeekFrom::Start(target.min(self.length)))?;
        self.position = target;
        Ok(target)
    }
}

/// What [`read_with_fallback_diagnostic`] read, the placeholder names it used, and
/// whether lofty saw the phantom ID3v1 tag of `ape_footer` (an ID3v1 tag it
/// reports is then not in the file).
pub(crate) type Fallback<T> = (T, Vec<(String, String)>, bool);

/// Read with `read`, through an [`Overlay`] that fixes how lofty sees a
/// trailing APE tag; when that fails on a tag with encrypted frames, read
/// again with those frames renamed.
pub(crate) fn read_with_fallback_diagnostic<R: Read + Seek, T>(
    reader: &mut R,
    mut read: impl FnMut(&mut dyn ReadSeek) -> Result<T, ErrorCause>,
) -> Result<Fallback<T>, ErrorCause> {
    let crate::ape_footer::View {
        mut patches,
        append,
        phantom,
    } = crate::ape_footer::read_view(reader).map_err(ErrorCause::external)?;
    let (extended, shift) = v23_extended_view(reader).map_err(ErrorCause::external)?;
    patches.extend(extended);
    let tail = if append {
        crate::ape_footer::PHANTOM.to_vec()
    } else {
        Vec::new()
    };
    let first = if patches.is_empty() && tail.is_empty() {
        read(reader)
    } else {
        let mut overlay =
            Overlay::new(&mut *reader, &patches, &tail).map_err(ErrorCause::external)?;
        read(&mut overlay)
    };
    let error = match first {
        Ok(value) => return Ok((value, Vec::new(), phantom)),
        Err(error) => error,
    };
    match encrypted_patches(reader) {
        Ok(Some((mut encrypted, tag))) => {
            for (offset, _) in &mut encrypted {
                *offset -= shift;
            }
            patches.extend(encrypted);
            let mut overlay =
                Overlay::new(&mut *reader, &patches, &tail).map_err(ErrorCause::external)?;
            let value = read(&mut overlay)?;
            Ok((value, placeholder_names(&tag), phantom))
        }
        _ => Err(error),
    }
}

/// A seekable reader as a trait object.
pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

fn is_flagged(frame: &Frame<'_>) -> bool {
    let flags = frame.flags();
    flags.compression || flags.encryption.is_some() || is_placeholder(frame)
}
/// Whether `tag` holds a compressed or encrypted frame.
pub(crate) fn has_flagged(tag: &Id3v2Tag) -> bool {
    tag.into_iter().any(is_flagged)
}
/// Whether `tag` has to go through [`Passthrough::split_diagnostic`] before it is
/// rendered: it holds flagged frames or data length indicators.
pub(crate) fn needs_split(tag: &Id3v2Tag) -> bool {
    tag.into_iter()
        .any(|frame| is_flagged(frame) || frame.flags().data_length_indicator.is_some())
}

/// Compressed and encrypted frames to copy into a written tag.
pub(crate) struct Passthrough {
    major: u8,
    frames: Vec<Vec<u8>>,
}
impl Passthrough {
    /// Nothing to copy.
    pub(crate) const NONE: Self = Self {
        major: 0,
        frames: Vec::new(),
    };

    /// Split the frames of `tag` (the tag about to be written, as revision
    /// `major`) into those rendered normally, returned, and the stored frames
    /// of `stored` copied verbatim. `original` is the tag as read from the
    /// file that `stored` describes.
    pub(crate) fn split_diagnostic(
        tag: &Id3v2Tag,
        major: u8,
        original: Option<&Id3v2Tag>,
        stored: Option<&RawTag>,
    ) -> Result<(Id3v2Tag, Self), ErrorCause> {
        let mut rendered = tag.clone();
        let mut frames = Vec::new();
        // The k-th flagged frame lofty read is the k-th stored one.
        let read: Vec<_> = original
            .map(|tag| tag.into_iter().filter(|f| is_flagged(f)).collect())
            .unwrap_or_default();
        let stored = stored.map(|tag| (tag.major, tag.flags, &tag.frames[..]));
        let copyable = stored.is_some_and(|(stored_major, flags, list)| {
            stored_major == major && flags & 0x80 == 0 && list.len() == read.len()
        });
        let mut next = 0;
        let mut copied = Vec::new();
        for (index, frame) in tag.into_iter().enumerate() {
            if !is_flagged(frame) {
                continue;
            }
            let debug = format!("{frame:?}");
            let position = read[next.min(read.len())..]
                .iter()
                .position(|f| format!("{f:?}") == debug)
                .map(|p| next + p);
            match (position, stored) {
                (Some(position), Some((_, _, list))) if copyable => {
                    frames.push(list[position].bytes.clone());
                    copied.push(index);
                    next = position + 1;
                }
                _ => {
                    let flags = frame.flags();
                    if flags.encryption.is_some() || is_placeholder(frame) {
                        return Err(ErrorCause::message(
                            "The encrypted ID3v2 frame {field} cannot be written as ID3v2.{major}; original file preserved",
                            [("field", frame.id().as_str().to_owned()), ("major", major.to_string())],
                        ));
                    }
                }
            }
        }
        let mut index = 0;
        rendered.retain_mut(|frame| {
            index += 1;
            if copied.contains(&(index - 1)) {
                return false;
            }
            // A decompressed frame is written uncompressed, and a data length
            // indicator only describes a compressed or encrypted frame.
            let mut flags = frame.flags();
            if flags.encryption.is_none()
                && (flags.compression || flags.data_length_indicator.is_some())
            {
                flags.compression = false;
                flags.data_length_indicator = None;
                frame.set_flags(flags);
            }
            true
        });
        Ok((rendered, Self { major, frames }))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Insert the copied frames after the frames of the rendered tag `bytes`
    /// (which may be empty), before its padding of `padding` bytes.
    #[cfg(test)]
    pub(crate) fn splice(&self, bytes: Vec<u8>, padding: u32) -> Result<Vec<u8>, String> {
        self.splice_diagnostic(bytes, padding)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn splice_diagnostic(
        &self,
        bytes: Vec<u8>,
        padding: u32,
    ) -> Result<Vec<u8>, ErrorCause> {
        if self.frames.is_empty() {
            return Ok(bytes);
        }
        let mut output = if bytes.is_empty() {
            let mut header = b"ID3".to_vec();
            header.extend([self.major, 0, 0]);
            header.extend(
                syncsafe_bytes(padding as usize)
                    .ok_or_else(|| ErrorCause::owned("The ID3v2 padding is too large"))?,
            );
            header.resize(10 + padding as usize, 0);
            header
        } else {
            bytes
        };
        if output.len() < 10 || &output[..3] != b"ID3" || output[3] != self.major {
            return Err(ErrorCause::owned("Invalid ID3v2 serialization"));
        }
        if output[5] & 0xD0 != 0 {
            return Err(
                ErrorCause::owned("Compressed or encrypted ID3v2 frames cannot be kept in an unsynchronised tag or one with an extended header or footer; original file preserved"),
            );
        }
        let size = syncsafe(&output[6..10])
            .ok_or_else(|| ErrorCause::owned("Invalid ID3v2 size"))? as usize;
        let end = (10 + size).min(output.len());
        let mut at = 10;
        while at + 10 <= end && output[at] != 0 {
            let frame = if self.major == 3 {
                u32::from_be_bytes(output[at + 4..at + 8].try_into().expect("4 bytes"))
            } else {
                syncsafe(&output[at + 4..at + 8])
                    .ok_or_else(|| ErrorCause::owned("Invalid ID3v2 frame size"))?
            } as usize;
            at += 10 + frame;
        }
        let at = at.min(end);
        let inserted: Vec<u8> = self.frames.concat();
        output.splice(at..at, inserted.iter().copied());
        let size = syncsafe_bytes(size + inserted.len())
            .ok_or_else(|| ErrorCause::owned("The ID3v2 tag is too large"))?;
        output[6..10].copy_from_slice(&size);
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn frame(id: &[u8; 4], flags: [u8; 2], body: &[u8], major: u8) -> Vec<u8> {
        let mut bytes = id.to_vec();
        if major == 3 {
            bytes.extend((body.len() as u32).to_be_bytes());
        } else {
            bytes.extend(syncsafe_bytes(body.len()).unwrap());
        }
        bytes.extend(flags);
        bytes.extend(body);
        bytes
    }
    fn tag(major: u8, frames: &[Vec<u8>], padding: usize) -> Vec<u8> {
        let body = frames.concat();
        let mut bytes = b"ID3".to_vec();
        bytes.extend([major, 0, 0]);
        bytes.extend(syncsafe_bytes(body.len() + padding).unwrap());
        bytes.extend(body);
        bytes.resize(bytes.len() + padding, 0);
        bytes
    }

    #[test]
    fn extended_only_read_view_does_not_move_media_or_reject_an_empty_body() {
        // The ID3v2.3 spec requires frames, but the adapter need not reject a
        // tolerated empty tag before the underlying reader makes that decision.
        let mut bytes = b"ID3\x03\0\x40\0\0\0\x0a".to_vec();
        bytes.extend(6u32.to_be_bytes());
        bytes.extend([0; 6]);
        bytes.extend(b"media payload");
        let mut source = Cursor::new(bytes.clone());
        source.set_position(7);
        let (patches, shift) = v23_extended_view(&mut source).unwrap();
        assert_eq!(
            source.position(),
            7,
            "inspection preserves the reader position"
        );
        assert_eq!(shift, 10);
        source.set_position(0);
        let mut view = Overlay::new(source, &patches, &[]).unwrap();
        let mut normalized = Vec::new();
        view.read_to_end(&mut normalized).unwrap();
        assert_eq!(normalized.len(), bytes.len());
        assert_eq!(normalized[5], 0);
        assert_eq!(&normalized[6..10], &bytes[6..10]);
        assert_eq!(&normalized[10..20], [0; 10]);
        assert_eq!(&normalized[20..], b"media payload");
    }

    #[test]
    fn detailed_fallback_retains_callback_provenance_and_splice_owns_its_guard() {
        let text = "Invalid ID3v2 serialization";
        let owned = read_with_fallback_diagnostic(
            &mut Cursor::new(vec![0; 32]),
            |_| -> Result<(), ErrorCause> { Err(ErrorCause::owned(text)) },
        )
        .unwrap_err();
        let external = read_with_fallback_diagnostic(
            &mut Cursor::new(vec![0; 32]),
            |_| -> Result<(), ErrorCause> { Err(ErrorCause::external(io::Error::other(text))) },
        )
        .unwrap_err();
        assert_eq!(owned.to_string(), external.to_string());
        assert!(matches!(owned, ErrorCause::Owned { .. }));
        assert!(matches!(external, ErrorCause::External { .. }));
        let copied = Passthrough {
            major: 4,
            frames: vec![vec![1]],
        };
        let error = copied.splice_diagnostic(b"bad".to_vec(), 0).unwrap_err();
        assert!(matches!(&error, ErrorCause::Owned { key, .. } if key == text));
        assert_eq!(
            copied.splice(b"bad".to_vec(), 0).unwrap_err(),
            error.to_string()
        );
    }

    #[test]
    fn scan_finds_flagged_frames_and_the_overlay_renames_encrypted_ones() {
        let plain = frame(b"TIT2", [0, 0], b"\x00Title", 3);
        let encrypted = frame(b"TALB", [0, 0x40], b"\x81secret", 3);
        let compressed = frame(b"TPE1", [0, 0x80], b"\x00\x00\x00\x05zlib", 3);
        let bytes = tag(3, &[plain.clone(), encrypted.clone(), compressed], 16);
        let raw = scan(&mut Cursor::new(&bytes)).unwrap().unwrap();
        assert_eq!(raw.major, 3);
        assert_eq!(raw.frames.len(), 2);
        assert!(raw.frames[0].encrypted && !raw.frames[1].encrypted);
        assert_eq!(raw.frames[0].offset, 10 + plain.len() as u64);
        assert_eq!(raw.frames[0].bytes, encrypted);

        let mut cursor = Cursor::new(bytes.clone());
        let (patches, _) = encrypted_patches(&mut cursor).unwrap().unwrap();
        cursor.set_position(0);
        let mut overlay = Overlay::new(cursor, &patches, b"tail").unwrap();
        let mut seen = Vec::new();
        // Read in small pieces so patches straddle buffer boundaries.
        let mut buffer = [0u8; 7];
        loop {
            let n = overlay.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            seen.extend_from_slice(&buffer[..n]);
        }
        let at = 10 + plain.len();
        assert_eq!(&seen[at..at + 4], b"Y000");
        assert_eq!(&seen[at + 8..at + 10], [0, 0]);
        assert_eq!(&seen[..at], &bytes[..at]);
        assert_eq!(&seen[at + 10..bytes.len()], &bytes[at + 10..]);
        // The tail follows the file, also after seeking from the end.
        assert_eq!(&seen[bytes.len()..], b"tail");
        assert_eq!(
            overlay.seek(SeekFrom::End(-6)).unwrap(),
            bytes.len() as u64 - 2
        );
        let mut end = Vec::new();
        overlay.read_to_end(&mut end).unwrap();
        assert_eq!(end, [&bytes[bytes.len() - 2..], b"tail"].concat());
    }

    #[test]
    fn splice_appends_frames_before_the_padding() {
        let first = frame(b"TIT2", [0, 0], b"\x03A", 4);
        let rendered = tag(4, std::slice::from_ref(&first), 20);
        let kept = frame(b"TALB", [0, 0x0D], b"\x00\x00\x00\x05\x81data", 4);
        let passthrough = Passthrough {
            major: 4,
            frames: vec![kept.clone()],
        };
        let output = passthrough.splice(rendered, 20).unwrap();
        assert_eq!(output, tag(4, &[first, kept.clone()], 20));
        // A tag of copied frames alone gets its own header.
        let alone = passthrough.splice(Vec::new(), 4).unwrap();
        assert_eq!(alone, tag(4, &[kept], 4));
    }
}
