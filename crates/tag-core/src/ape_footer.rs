//! An APEv2 tag of exactly 131 bytes (header, items and footer) at the end of
//! a file puts the "TAG" of its "APETAGEX" header 128 bytes before the end,
//! where readers look for an ID3v1 tag. lofty then reads an ID3v1 tag made of
//! the APE tag's bytes and misses the APE tag. Reading masks that "TAG" (see
//! `id3_frames::read_with_fallback`). Writing drops the header, which only
//! repeats the footer, so the tag no longer has that size; the footer then
//! says APEv1, because lofty takes every APEv2 footer to have a header. The
//! items are unchanged, and the next APE write stores APEv2 again.
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

const PREAMBLE: &[u8; 8] = b"APETAGEX";
/// Items and footer of a 131-byte tag with a 32-byte header.
const COLLIDING: u32 = 131 - 32;

/// Where the header of such a tag starts.
fn colliding_header(reader: &mut (impl Read + Seek)) -> io::Result<Option<u64>> {
    let length = reader.seek(SeekFrom::End(0))?;
    if length < 131 {
        return Ok(None);
    }
    let start = length - 131;
    reader.seek(SeekFrom::Start(start))?;
    let mut tail = [0u8; 131];
    reader.read_exact(&mut tail)?;
    let footer = &tail[99..];
    let size = u32::from_le_bytes(footer[12..16].try_into().expect("4 bytes"));
    let flags = u32::from_le_bytes(footer[20..24].try_into().expect("4 bytes"));
    Ok((&tail[..8] == PREAMBLE
        && &footer[..8] == PREAMBLE
        && size == COLLIDING
        && flags & (1 << 31) != 0)
        .then_some(start))
}

/// The offset of the "TAG" an ID3v1 reader would take for a tag.
pub(crate) fn id3v1_lookalike(reader: &mut (impl Read + Seek)) -> io::Result<Option<u64>> {
    Ok(colliding_header(reader)?.map(|start| start + 3))
}

/// Remove the header of such a tag, clear the footer's header flag and mark
/// it APEv1.
pub(crate) fn strip_colliding_header(file: &mut File) -> Result<(), String> {
    let Some(start) = colliding_header(file).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let mut rest = vec![0u8; COLLIDING as usize];
    file.seek(SeekFrom::Start(start + 32))
        .and_then(|_| file.read_exact(&mut rest))
        .map_err(|e| e.to_string())?;
    let footer = rest.len() - 32;
    let flags = u32::from_le_bytes(rest[footer + 20..footer + 24].try_into().expect("4 bytes"));
    rest[footer + 8..footer + 12].copy_from_slice(&1000u32.to_le_bytes());
    rest[footer + 20..footer + 24].copy_from_slice(&(flags & !(1 << 31)).to_le_bytes());
    file.seek(SeekFrom::Start(start))
        .and_then(|_| file.write_all(&rest))
        .and_then(|_| file.set_len(start + rest.len() as u64))
        .map_err(|e| e.to_string())?;
    crate::write_cost::rewrote_all();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn tag(items: &[u8], header: bool) -> Vec<u8> {
        let size = (items.len() + 32) as u32;
        let block = |flags: u32| {
            let mut b = PREAMBLE.to_vec();
            for v in [2000, size, 1, flags] {
                b.extend(u32::to_le_bytes(v));
            }
            b.extend([0; 8]);
            b
        };
        let mut bytes = if header {
            block(0xA000_0000)
        } else {
            Vec::new()
        };
        bytes.extend(items);
        bytes.extend(block(0x8000_0000));
        bytes
    }

    #[test]
    fn only_a_131_byte_tag_with_a_header_collides_and_loses_its_header() {
        let items = vec![7u8; 67];
        let mut bytes = vec![1u8; 300];
        bytes.extend(tag(&items, true));
        assert_eq!(
            id3v1_lookalike(&mut Cursor::new(&bytes)).unwrap(),
            Some(303)
        );
        assert_eq!(&bytes[bytes.len() - 128..bytes.len() - 125], b"TAG");
        let mut other = vec![1u8; 300];
        other.extend(tag(&[7u8; 68], true));
        assert_eq!(id3v1_lookalike(&mut Cursor::new(&other)).unwrap(), None);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        std::fs::write(&path, &bytes).unwrap();
        let mut file = File::options().read(true).write(true).open(&path).unwrap();
        strip_colliding_header(&mut file).unwrap();
        drop(file);
        let mut expected = vec![1u8; 300];
        let mut headerless = tag(&items, false);
        let footer = headerless.len() - 32;
        headerless.splice(footer + 8..footer + 12, 1000u32.to_le_bytes());
        headerless.splice(footer + 20..footer + 24, 0u32.to_le_bytes());
        expected.extend(headerless);
        assert_eq!(std::fs::read(&path).unwrap(), expected);
    }
}
