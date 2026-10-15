//! ID3v2 writes that reuse the existing tag's padding.
//!
//! Replacing the ID3v2 tag at the start of an MPEG stream moves every audio
//! byte whenever the tag size changes, so a tag edit becomes a full file
//! rewrite. When the new frames fit the existing tag (its frames plus
//! padding), the tag is rendered with exactly the padding that keeps its size
//! and is written over the old one: the audio is never touched. The same
//! applies to the single `ID3 ` chunk of WAV and AIFF files, whose chunk size
//! then stays the same. Writes that do not fit fall back to a full rewrite that
//! reserves [`PADDING`] bytes, so later edits fit.
use lofty::file::FileType;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
};

/// Padding reserved when a tag has to be rewritten, matching the 4 KB that
/// Mp3tag's Utils → Optimize FLAC reserves for FLAC metadata.
pub(crate) const PADDING: u32 = 4096;

fn syncsafe(bytes: &[u8]) -> Option<u64> {
    bytes.iter().try_fold(0u64, |v, b| {
        (b & 0x80 == 0).then_some((v << 7) | u64::from(*b))
    })
}

/// Byte range of the ID3v2 tag an in-place write may replace: the tag at the
/// start of an MPEG or AAC stream (including a v2.4 footer), or the payload of
/// the only `ID3 ` chunk of a WAV or AIFF file.
fn existing(file: &mut File, kind: FileType) -> std::io::Result<Option<Range<u64>>> {
    let length = file.metadata()?.len();
    file.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; 12];
    if file.read_exact(&mut header).is_err() {
        return Ok(None);
    }
    let range = match kind {
        FileType::Mpeg | FileType::Aac => {
            if &header[..3] != b"ID3" || !(2..=4).contains(&header[3]) {
                return Ok(None);
            }
            let Some(size) = syncsafe(&header[6..10]) else {
                return Ok(None);
            };
            let footer = if header[3] == 4 && header[5] & 0x10 != 0 {
                10
            } else {
                0
            };
            0..10 + size + footer
        }
        FileType::Wav | FileType::Aiff => {
            let little = kind == FileType::Wav;
            let form: &[u8; 4] = if little { b"RIFF" } else { b"FORM" };
            if &header[..4] != form {
                return Ok(None);
            }
            let mut position = 12u64;
            let mut found = None;
            while position + 8 <= length {
                file.seek(SeekFrom::Start(position))?;
                let mut chunk = [0u8; 8];
                file.read_exact(&mut chunk)?;
                let size = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                let size = u64::from(if little { size.swap_bytes() } else { size });
                if &chunk[..4] == b"ID3 " || &chunk[..4] == b"id3 " {
                    if found.is_some() {
                        return Ok(None);
                    }
                    found = Some(position + 8..position + 8 + size);
                }
                position += 8 + size + size % 2;
            }
            match found {
                Some(range) => range,
                None => return Ok(None),
            }
        }
        _ => return Ok(None),
    };
    Ok((range.end <= length).then_some(range))
}

/// Write the tag that `render(padding)` produces over the existing tag when
/// it can be padded to exactly the existing size. Returns whether it did;
/// `false` leaves the file untouched for a full rewrite. A tag without frames
/// (an empty rendering) is a removal, which never happens in place.
pub(crate) fn save_in_place(
    file: &mut File,
    kind: FileType,
    render: impl Fn(u32) -> Result<Vec<u8>, String>,
) -> Result<bool, String> {
    let Some(range) = existing(file, kind).map_err(|e| e.to_string())? else {
        return Ok(false);
    };
    let unpadded = render(0)?.len() as u64;
    let available = range.end - range.start;
    if unpadded == 0 || unpadded > available {
        return Ok(false);
    }
    let Ok(padding) = u32::try_from(available - unpadded) else {
        return Ok(false);
    };
    let bytes = render(padding)?;
    // A footer or another flag can make a writer ignore the requested padding.
    if bytes.len() as u64 != available {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(range.start))
        .and_then(|_| file.write_all(&bytes))
        .map_err(|e| e.to_string())?;
    crate::write_cost::rewrote_region(available);
    Ok(true)
}
