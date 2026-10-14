//! FLAC Vorbis comment writes that reuse the file's padding.
//!
//! lofty rewrites FLAC metadata by shifting every audio byte whenever the
//! metadata size changes, even when a PADDING block could absorb the
//! difference. For a typical 40 MB track that turns a tag edit into a full
//! file rewrite. Here lofty encodes the new metadata against a copy of the
//! metadata region only; when the result fits the original region (by
//! resizing, adding or removing padding), it is written over that region in
//! place and the audio frames are never touched. Otherwise the caller falls
//! back to lofty's full write.
use lofty::{config::WriteOptions, ogg::tag::VorbisComments, prelude::*};
use std::{
    fs,
    io::{Cursor, Read, Seek, SeekFrom, Write},
};

const MAGIC: &[u8; 4] = b"fLaC";
const HEADER: usize = 4;
const PADDING: u8 = 1;
const MAX_BLOCK: usize = (1 << 24) - 1;
const TAIL: &[u8] = b"\xff\xf8OpenTag audio stand-in";

struct Block {
    kind: u8,
    start: usize,
    len: usize,
}

/// Blocks of a metadata region (bytes after `fLaC`), or `None` if malformed.
fn blocks(region: &[u8]) -> Option<Vec<Block>> {
    let mut blocks = Vec::new();
    let mut at = 0;
    loop {
        let header = region.get(at..at + HEADER)?;
        let len =
            usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]);
        region.get(at + HEADER..at + HEADER + len)?;
        blocks.push(Block {
            kind: header[0] & 0x7f,
            start: at + HEADER,
            len,
        });
        at += HEADER + len;
        if header[0] & 0x80 != 0 {
            return (at == region.len()).then_some(blocks);
        }
    }
}

/// Re-pack `region` to exactly `size` bytes by replacing its padding, or `None`.
fn fit(region: &[u8], size: usize) -> Option<Vec<u8>> {
    let kept: Vec<Block> = blocks(region)?
        .into_iter()
        .filter(|b| b.kind != PADDING)
        .collect();
    let used: usize = kept.iter().map(|b| HEADER + b.len).sum();
    let padding = match size.checked_sub(used)? {
        0 => None,
        gap if gap >= HEADER && gap - HEADER <= MAX_BLOCK => Some(gap - HEADER),
        _ => return None,
    };
    let mut out = Vec::with_capacity(size);
    for (index, block) in kept.iter().enumerate() {
        let last = padding.is_none() && index + 1 == kept.len();
        out.push(block.kind | if last { 0x80 } else { 0 });
        out.extend_from_slice(&(block.len as u32).to_be_bytes()[1..]);
        out.extend_from_slice(&region[block.start..block.start + block.len]);
    }
    if let Some(len) = padding {
        out.push(PADDING | 0x80);
        out.extend_from_slice(&(len as u32).to_be_bytes()[1..]);
        out.resize(out.len() + len, 0);
    }
    (out.len() == size).then_some(out)
}

/// Length of `fLaC` plus all metadata blocks, or `None` when the file does not
/// start with a native FLAC stream (for example an ID3v2-prefixed file).
fn metadata_end(file: &mut fs::File) -> std::io::Result<Option<u64>> {
    file.seek(SeekFrom::Start(0))?;
    let mut magic = [0; 4];
    if file.read_exact(&mut magic).is_err() || &magic != MAGIC {
        return Ok(None);
    }
    let mut at = MAGIC.len() as u64;
    loop {
        let mut header = [0; HEADER];
        if file.read_exact(&mut header).is_err() {
            return Ok(None);
        }
        let len = u64::from(header[1]) << 16 | u64::from(header[2]) << 8 | u64::from(header[3]);
        at += HEADER as u64 + len;
        if header[0] & 0x80 != 0 {
            return Ok(Some(at));
        }
        file.seek(SeekFrom::Start(at))?;
    }
}

/// Write `tag` into the FLAC file, in place when the padding allows it.
pub(crate) fn save(file: &mut fs::File, tag: &VorbisComments) -> Result<(), String> {
    if let Some(updated) = in_place(file, tag).map_err(|e| e.to_string())? {
        file.seek(SeekFrom::Start(MAGIC.len() as u64))
            .and_then(|_| file.write_all(&updated))
            .map_err(|e| e.to_string())?;
        crate::write_cost::rewrote_region(updated.len() as u64);
        return Ok(());
    }
    crate::write_cost::rewrote_all();
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    tag.save_to(file, WriteOptions::default())
        .map_err(|e| crate::tags::error_message(&e))
}

/// The new metadata region, sized to the old one, if it fits.
fn in_place(file: &mut fs::File, tag: &VorbisComments) -> std::io::Result<Option<Vec<u8>>> {
    let Some(end) = metadata_end(file)? else {
        return Ok(None);
    };
    let Ok(end) = usize::try_from(end) else {
        return Ok(None);
    };
    // lofty edits the metadata of an in-memory copy; a stand-in tail marks
    // where the audio would continue, so the new region can be measured.
    let mut head = vec![0; end + TAIL.len()];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut head[..end])?;
    head[end..].copy_from_slice(TAIL);
    let mut encoded = Cursor::new(head);
    if tag.save_to(&mut encoded, WriteOptions::default()).is_err() {
        return Ok(None);
    }
    let encoded = encoded.into_inner();
    let Some(region) = encoded
        .strip_prefix(MAGIC.as_slice())
        .and_then(|rest| rest.strip_suffix(TAIL))
    else {
        return Ok(None);
    };
    Ok(fit(region, end - MAGIC.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(kind: u8, last: bool, content: &[u8]) -> Vec<u8> {
        let mut out = vec![kind | if last { 0x80 } else { 0 }];
        out.extend_from_slice(&(content.len() as u32).to_be_bytes()[1..]);
        out.extend_from_slice(content);
        out
    }

    #[test]
    fn grows_and_shrinks_into_padding() {
        let region = [
            block(0, false, &[7; 34]),
            block(4, false, b"abc"),
            block(1, true, &[0; 20]),
        ]
        .concat();
        let size = region.len();
        let grown = [
            block(0, false, &[7; 34]),
            block(4, false, b"abcdefgh"),
            block(1, true, &[0; 20]),
        ]
        .concat();
        let fitted = fit(&grown, size).unwrap();
        assert_eq!(fitted.len(), size);
        let parsed = blocks(&fitted).unwrap();
        assert_eq!(
            parsed.iter().map(|b| (b.kind, b.len)).collect::<Vec<_>>(),
            [(0, 34), (4, 8), (1, 15)]
        );
        let shrunk = [block(0, false, &[7; 34]), block(4, true, b"a")].concat();
        let fitted = fit(&shrunk, size).unwrap();
        assert_eq!(
            blocks(&fitted).unwrap().last().map(|b| (b.kind, b.len)),
            Some((1, 22))
        );
    }

    #[test]
    fn exact_fit_drops_padding_and_overflow_declines() {
        let streaminfo = block(0, false, &[7; 34]);
        let comments = block(4, true, b"abcd");
        let exact = [streaminfo.clone(), comments.clone()].concat();
        let fitted = fit(&exact, exact.len()).unwrap();
        assert_eq!(fitted, exact);
        assert!(fit(&exact, exact.len() - 1).is_none());
        // A gap smaller than a block header cannot be filled.
        assert!(fit(&exact, exact.len() + 2).is_none());
    }
}
