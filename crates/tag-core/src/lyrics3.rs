//! Bounded physical Lyrics3v2 envelopes. The contents remain opaque.
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::ops::Range;

fn at(reader: &mut (impl Read + Seek), offset: u64, bytes: &mut [u8]) -> io::Result<()> {
    reader.seek(SeekFrom::Start(offset))?;
    reader.read_exact(bytes)
}

pub(crate) fn real_v1(reader: &mut (impl Read + Seek)) -> io::Result<Option<[u8; 128]>> {
    let length = reader.seek(SeekFrom::End(0))?;
    if length < 128 {
        return Ok(None);
    }
    let mut bytes = [0; 128];
    at(reader, length - 128, &mut bytes)?;
    if !bytes.starts_with(b"TAG")
        || bytes == crate::ape_footer::PHANTOM
        || crate::ape_footer::id3v1_lookalike(reader)?.is_some()
    {
        return Ok(None);
    }
    Ok(Some(bytes))
}

/// Inspect only physical EOF, then the boundary before a genuine V1 suffix.
/// Six decimal size bytes and the calculated BEGIN are required; no field parser.
pub(crate) fn detect(reader: &mut (impl Read + Seek)) -> io::Result<Option<Range<u64>>> {
    let position = reader.stream_position()?;
    let result = (|| {
        let length = reader.seek(SeekFrom::End(0))?;
        let mut floor = 0;
        if length >= 10 {
            let mut header = [0; 10];
            at(reader, 0, &mut header)?;
            if header.starts_with(b"ID3") {
                if header[6..10].iter().any(|b| b & 128 != 0) {
                    return Ok(None);
                }
                floor = 10
                    + header[6..10]
                        .iter()
                        .fold(0u64, |n, b| (n << 7) | u64::from(*b));
                if header[3] == 4 && header[5] & 16 != 0 {
                    floor += 10;
                }
            }
        }
        let ends = [Some(length), real_v1(reader)?.map(|_| length - 128)];
        for end in ends.into_iter().flatten() {
            if end < 26 {
                continue;
            }
            let mut footer = [0; 15];
            at(reader, end - 15, &mut footer)?;
            if &footer[6..] != b"LYRICS200" || !footer[..6].iter().all(u8::is_ascii_digit) {
                continue;
            }
            let size = footer[..6]
                .iter()
                .fold(0u64, |n, b| n * 10 + u64::from(*b - b'0'));
            let Some(start) = end.checked_sub(15 + size).filter(|start| *start >= floor) else {
                continue;
            };
            if size < 11 {
                continue;
            }
            let mut begin = [0; 11];
            at(reader, start, &mut begin)?;
            if &begin == b"LYRICSBEGIN" {
                return Ok(Some(start..end));
            }
        }
        Ok(None)
    })();
    reader.seek(SeekFrom::Start(position))?;
    result
}

/// Only stream families that own these physical tails get a logical view.
pub(crate) fn read_range(reader: &mut (impl Read + Seek)) -> io::Result<Option<Range<u64>>> {
    let position = reader.stream_position()?;
    reader.seek(SeekFrom::Start(0))?;
    let kind = lofty::probe::Probe::new(BufReader::new(&mut *reader))
        .guess_file_type()
        .map(|p| p.file_type());
    reader.seek(SeekFrom::Start(position))?;
    if matches!(
        kind?,
        Some(
            lofty::file::FileType::Mpeg
                | lofty::file::FileType::Ape
                | lofty::file::FileType::Mpc
                | lofty::file::FileType::WavPack
        )
    ) {
        detect(reader)
    } else {
        Ok(None)
    }
}

/// A logical read-only view outside physical ID3 patches and any appended tail.
pub(crate) struct Skip<R> {
    inner: R,
    range: Range<u64>,
    length: u64,
    position: u64,
}
impl<R: Read + Seek> Skip<R> {
    pub(crate) fn new(mut inner: R, range: Option<Range<u64>>) -> io::Result<Self> {
        let physical = inner.stream_position()?;
        let length = inner.seek(SeekFrom::End(0))?;
        let range = range.unwrap_or(length..length);
        if range.start > range.end || range.end > length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid hidden range",
            ));
        }
        let position = if physical < range.start {
            physical
        } else if physical < range.end {
            range.start
        } else {
            physical - (range.end - range.start)
        };
        let mut view = Self {
            inner,
            length: length - (range.end - range.start),
            range,
            position,
        };
        view.seek(SeekFrom::Start(position))?;
        Ok(view)
    }
    fn physical(&self, logical: u64) -> io::Result<u64> {
        if logical < self.range.start {
            Ok(logical)
        } else {
            logical
                .checked_add(self.range.end - self.range.start)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek overflow"))
        }
    }
}
impl<R: Read + Seek> Read for Skip<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() || self.position >= self.length {
            return Ok(0);
        }
        let boundary = if self.position < self.range.start {
            self.range.start
        } else {
            self.length
        };
        let take = bytes
            .len()
            .min(usize::try_from(boundary - self.position).unwrap_or(usize::MAX));
        self.inner
            .seek(SeekFrom::Start(self.physical(self.position)?))?;
        let read = self.inner.read(&mut bytes[..take])?;
        self.position += read as u64;
        Ok(read)
    }
}
impl<R: Read + Seek> Seek for Skip<R> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let target = match to {
            SeekFrom::Start(at) => Some(at),
            SeekFrom::Current(by) => self.position.checked_add_signed(by),
            SeekFrom::End(by) => self.length.checked_add_signed(by),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid logical seek"))?;
        self.inner.seek(SeekFrom::Start(self.physical(target)?))?;
        self.position = target;
        Ok(target)
    }
}

/// Private operation intent, carried through one staged mutation only.
#[derive(Default)]
pub(crate) struct Effects {
    pub removed_v1: bool,
    pub removed_ape: bool,
    pub wrote_v1: bool,
    /// This mutation's guard appended a physical placeholder. The blank V1
    /// serializer may replace precisely that owned slot, despite no V1 inventory.
    pub owned_phantom: bool,
}
impl Effects {
    pub(crate) fn removed(&mut self, kind: lofty::tag::TagType) {
        self.removed_v1 |= kind == lofty::tag::TagType::Id3v1;
        self.removed_ape |= kind == lofty::tag::TagType::Ape;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn logical_seeks_and_small_reads_cross_the_hidden_span_and_appended_tail() {
        let mut source = Cursor::new(b"abHIDDENcdTAIL".to_vec());
        source.set_position(1);
        let mut view = Skip::new(source, Some(2..8)).unwrap();
        assert_eq!(view.stream_position().unwrap(), 1);
        let mut result = Vec::new();
        let mut byte = [0];
        while view.read(&mut byte).unwrap() != 0 {
            result.extend(byte);
        }
        assert_eq!(result, b"bcdTAIL");
        assert_eq!(view.seek(SeekFrom::End(-4)).unwrap(), 4);
        assert_eq!(view.seek(SeekFrom::Current(-2)).unwrap(), 2);
        let mut crossing = [0; 6];
        view.read_exact(&mut crossing).unwrap();
        assert_eq!(&crossing, b"cdTAIL");
        assert!(view.seek(SeekFrom::Start(u64::MAX)).is_err());
        assert!(view.seek(SeekFrom::End(-9)).is_err());
        view.seek(SeekFrom::End(10)).unwrap();
        assert_eq!(view.read(&mut byte).unwrap(), 0);
        assert_eq!(view.read(&mut []).unwrap(), 0);
    }

    #[test]
    fn envelope_bounds_begin_marker_and_phantom_are_conservative() {
        for size in ["000000", "000010", "999999", "00001x"] {
            let bytes = [
                b"audioLYRICSBEGIN".as_slice(),
                size.as_bytes(),
                b"LYRICS200",
            ]
            .concat();
            assert_eq!(detect(&mut Cursor::new(bytes)).unwrap(), None);
        }
        let span = b"LYRICSBEGIN\xffopaque000018LYRICS200";
        let mut bytes = b"audio".to_vec();
        bytes.extend(span);
        let mut cursor = Cursor::new(bytes.clone());
        cursor.set_position(3);
        assert_eq!(detect(&mut cursor).unwrap(), Some(5..bytes.len() as u64));
        assert_eq!(cursor.position(), 3);
        bytes.extend(crate::ape_footer::PHANTOM);
        assert_eq!(detect(&mut Cursor::new(bytes)).unwrap(), None);
    }

    #[test]
    fn inspection_and_views_preserve_external_read_and_seek_errors() {
        struct Broken(Cursor<Vec<u8>>);
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "read refused",
                ))
            }
        }
        impl Seek for Broken {
            fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
                self.0.seek(to)
            }
        }
        let mut broken = Broken(Cursor::new(vec![0; 40]));
        assert_eq!(
            detect(&mut broken).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        let mut view = Skip::new(broken, Some(2..4)).unwrap();
        assert_eq!(
            view.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        struct NoSeek;
        impl Read for NoSeek {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Ok(0)
            }
        }
        impl Seek for NoSeek {
            fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
                Err(io::Error::other("seek refused"))
            }
        }
        assert_eq!(detect(&mut NoSeek).unwrap_err().to_string(), "seek refused");
    }
}
