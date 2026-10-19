//! Trailing APE tags that lofty misreads.
//!
//! - **Header-less APEv2.** lofty takes every APEv2 footer to follow a
//!   header, so it places such a tag 32 bytes before its real start: reads
//!   cut the stream short and writes delete the 32 bytes in front of the tag.
//!   Readers see the footer marked APEv1, which lofty measures correctly; the
//!   items read the same either way.
//! - **"TAG" where an ID3v1 tag would start.** An APE tag that ends the file
//!   and reaches back at least 128 bytes can hold "TAG" exactly there: in the
//!   "APETAGEX" of its header (a 131-byte tag) or in its items. lofty then
//!   reads an ID3v1 tag made of APE bytes and misses the APE tag. Readers see
//!   the file with an extra, empty ID3v1 tag after it (the [`PHANTOM`]), so
//!   lofty finds the APE footer where it looks behind an ID3v1 tag, every
//!   offset stays the same and no APE byte is altered. Callers drop the
//!   ID3v1 tag lofty then reports.
//!
//! Writes go through [`Guard`]: the staged file gets the same footer mark and
//! phantom before lofty touches it, and both are undone afterwards, so an APE
//! tag that was not rewritten keeps its bytes.
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

const PREAMBLE: &[u8; 8] = b"APETAGEX";
const HAS_HEADER: u32 = 1 << 31;

/// An empty ID3v1 tag no ID3v1 writer produces: its title has a byte after
/// the terminating NUL. The year is "0000" because lofty's strict reader
/// refuses an empty one.
pub(crate) const PHANTOM: [u8; 128] = {
    let mut tag = [0u8; 128];
    tag[0] = b'T';
    tag[1] = b'A';
    tag[2] = b'G';
    tag[32] = 0xFF;
    tag[93] = b'0';
    tag[94] = b'0';
    tag[95] = b'0';
    tag[96] = b'0';
    tag[127] = 0xFF;
    tag
};

/// A plausible APE footer: where it is, its version, the size it gives (items
/// and footer) and its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Footer {
    pub at: u64,
    pub version: u32,
    pub size: u32,
    pub flags: u32,
}
impl Footer {
    /// Where the tag starts, its header included.
    pub(crate) fn start(&self) -> u64 {
        let header = if self.flags & HAS_HEADER != 0 { 32 } else { 0 };
        self.at + 32 - u64::from(self.size) - header
    }
    fn headerless_v2(&self) -> bool {
        self.version == 2000 && self.flags & HAS_HEADER == 0
    }
}

fn read_at(reader: &mut (impl Read + Seek), at: u64, buffer: &mut [u8]) -> io::Result<()> {
    reader.seek(SeekFrom::Start(at))?;
    reader.read_exact(buffer)
}

fn footer_at(reader: &mut (impl Read + Seek), at: u64) -> io::Result<Option<Footer>> {
    let mut bytes = [0u8; 32];
    read_at(reader, at, &mut bytes)?;
    let word = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
    let footer = Footer {
        at,
        version: word(8),
        size: word(12),
        flags: word(20),
    };
    let header = if footer.flags & HAS_HEADER != 0 {
        32
    } else {
        0
    };
    let valid = &bytes[..8] == PREAMBLE
        && matches!(footer.version, 1000 | 2000)
        && footer.size >= 32
        && u64::from(footer.size) + header <= at + 32;
    Ok(valid.then_some(footer))
}

/// The footer of an APE tag that ends the file and has "TAG" where an ID3v1
/// tag would start.
fn colliding(reader: &mut (impl Read + Seek), length: u64) -> io::Result<Option<Footer>> {
    if length < 128 {
        return Ok(None);
    }
    let Some(footer) = footer_at(reader, length - 32)? else {
        return Ok(None);
    };
    let mut marker = [0u8; 3];
    read_at(reader, length - 128, &mut marker)?;
    Ok((footer.start() <= length - 128 && &marker == b"TAG").then_some(footer))
}

/// The offset of the "TAG" an ID3v1 reader would take from an APE tag.
pub(crate) fn id3v1_lookalike(reader: &mut (impl Read + Seek)) -> io::Result<Option<u64>> {
    let length = reader.seek(SeekFrom::End(0))?;
    Ok(colliding(reader, length)?.map(|_| length - 128))
}

/// The footer of the APE tag at the end of the file, behind which lofty
/// looks for an ID3v1 tag and a Lyrics3v2 block.
pub(crate) fn trailing(reader: &mut (impl Read + Seek)) -> io::Result<Option<Footer>> {
    let length = reader.seek(SeekFrom::End(0))?;
    if let Some(footer) = colliding(reader, length)? {
        return Ok(Some(footer));
    }
    let mut end = length;
    let mut marker = [0u8; 3];
    if end >= 128 {
        read_at(reader, end - 128, &mut marker)?;
        if &marker == b"TAG" {
            end -= 128;
        }
    }
    if let Some(range) = crate::lyrics3::detect(reader)? {
        end = range.start;
    }
    if end < 32 {
        return Ok(None);
    }
    footer_at(reader, end - 32)
}

/// Bytes shown instead of the stored ones, by offset.
pub(crate) type Patches = Vec<(u64, Vec<u8>)>;

/// How readers see the file (see the module documentation).
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct View {
    pub patches: Patches,
    /// The [`PHANTOM`] follows the file.
    pub append: bool,
    /// The ID3v1 tag lofty reports is the phantom: appended, or already
    /// there in a file staged by [`Guard`].
    pub phantom: bool,
}

pub(crate) fn read_view(reader: &mut (impl Read + Seek)) -> io::Result<View> {
    let length = reader.seek(SeekFrom::End(0))?;
    let mut tail = [0u8; 128];
    let staged = length >= 128 && {
        read_at(reader, length - 128, &mut tail)?;
        tail == PHANTOM
    };
    let append = !staged && colliding(reader, length)?.is_some();
    let patches = match trailing(reader)? {
        Some(footer) if footer.headerless_v2() => {
            vec![(footer.at + 8, 1000u32.to_le_bytes().to_vec())]
        }
        _ => Vec::new(),
    };
    Ok(View {
        patches,
        append,
        phantom: staged || append,
    })
}

/// Gives lofty's writers a staged file they handle correctly and undoes that
/// afterwards; see the module documentation.
#[must_use]
pub(crate) struct Guard {
    phantom: bool,
    /// Offset of a footer version changed to APEv1 within the tag, and the
    /// tag's bytes as staged.
    marked: Option<(u64, Vec<u8>)>,
    lyrics: Option<Vec<u8>>,
    v1: Option<[u8; 128]>,
}
impl Guard {
    pub(crate) fn prepare(file: &mut File, owns_tail: bool) -> Result<Self, String> {
        let error = |e: io::Error| e.to_string();
        let mut lyrics = None;
        let mut v1 = None;
        if owns_tail {
            if let Some(range) = crate::lyrics3::detect(file).map_err(error)? {
                let mut bytes = vec![0; (range.end - range.start) as usize];
                read_at(file, range.start, &mut bytes).map_err(error)?;
                let length = file.seek(SeekFrom::End(0)).map_err(error)?;
                v1 = if range.end.checked_add(128) == Some(length) {
                    crate::lyrics3::real_v1(file).map_err(error)?
                } else {
                    None
                };
                file.set_len(range.start).map_err(error)?;
                file.seek(SeekFrom::End(0)).map_err(error)?;
                if let Some(tag) = v1 {
                    file.write_all(&tag).map_err(error)?;
                }
                lyrics = Some(bytes);
                crate::write_cost::rewrote_all();
            }
        }
        let length = file.seek(SeekFrom::End(0)).map_err(error)?;
        let phantom = colliding(file, length).map_err(error)?.is_some();
        let mut marked = None;
        if let Some(footer) = trailing(file).map_err(error)?.filter(Footer::headerless_v2) {
            let start = footer.start();
            let mut tag = vec![0u8; (footer.at + 32 - start) as usize];
            read_at(file, start, &mut tag).map_err(error)?;
            let version = (footer.at + 8 - start) as usize;
            tag[version..version + 4].copy_from_slice(&1000u32.to_le_bytes());
            file.seek(SeekFrom::Start(footer.at + 8))
                .and_then(|_| file.write_all(&1000u32.to_le_bytes()))
                .map_err(error)?;
            marked = Some((version as u64, tag));
            crate::write_cost::rewrote_region(4);
        }
        if phantom {
            file.seek(SeekFrom::End(0))
                .and_then(|_| file.write_all(&PHANTOM))
                .map_err(error)?;
            crate::write_cost::rewrote_all();
        }
        Ok(Self {
            phantom,
            marked,
            lyrics,
            v1,
        })
    }

    pub(crate) fn effects(&self) -> crate::lyrics3::Effects {
        crate::lyrics3::Effects {
            owned_phantom: self.phantom,
            ..Default::default()
        }
    }

    /// Remove a phantom ID3v1 tag that is still there, and mark an APE tag
    /// that was not rewritten APEv2 again.
    pub(crate) fn finish(
        self,
        file: &mut File,
        effects: &crate::lyrics3::Effects,
    ) -> Result<(), String> {
        let error = |e: io::Error| e.to_string();
        let length = file.seek(SeekFrom::End(0)).map_err(error)?;
        if self.phantom && length >= 128 {
            let mut tail = [0u8; 128];
            read_at(file, length - 128, &mut tail).map_err(error)?;
            if tail == PHANTOM {
                file.set_len(length - 128).map_err(error)?;
                crate::write_cost::rewrote_all();
            }
        }
        if let Some((version, staged)) = self.marked {
            if let Some(footer) = trailing(file).map_err(error)? {
                let start = footer.start();
                let mut tag = vec![0u8; (footer.at + 32 - start) as usize];
                read_at(file, start, &mut tag).map_err(error)?;
                if tag == staged {
                    file.seek(SeekFrom::Start(start + version))
                        .and_then(|_| file.write_all(&2000u32.to_le_bytes()))
                        .map_err(error)?;
                    crate::write_cost::rewrote_region(4);
                }
            }
        }
        if let Some(lyrics) = self.lyrics {
            if effects.removed_v1 {
                return Ok(());
            }
            if effects.removed_ape {
                if effects.wrote_v1 {
                    // The Save writer already serialized the updated model.
                    // Restoring the captured pre-edit V1 would undo that Save.
                    if crate::lyrics3::real_v1(file).map_err(error)?.is_none() {
                        return Err("The written ID3v1 tag is missing".into());
                    }
                    return Ok(());
                }
                // Native APE-only Remove drops Lyrics3v2 and serializes a
                // real retained V1; ordinary APE writes preserve its raw bytes.
                if let Some(raw) = self.v1 {
                    use lofty::tag::TagExt;
                    let tag =
                        lofty::id3::v1::Id3v1Tag::parse(raw, lofty::config::ParsingMode::Relaxed)
                            .map_err(|e| e.to_string())?;
                    let mut bytes = Vec::new();
                    tag.dump_to(&mut bytes, lofty::config::WriteOptions::default())
                        .map_err(|e| e.to_string())?;
                    let end = file.seek(SeekFrom::End(0)).map_err(error)?;
                    if crate::lyrics3::real_v1(file).map_err(error)?.is_some() {
                        file.set_len(end - 128).map_err(error)?;
                    }
                    file.seek(SeekFrom::End(0))
                        .and_then(|_| file.write_all(&bytes))
                        .map_err(error)?;
                    crate::write_cost::rewrote_all();
                }
                return Ok(());
            }
            let current = crate::lyrics3::real_v1(file).map_err(error)?;
            let end = file.seek(SeekFrom::End(0)).map_err(error)?;
            if current.is_some() {
                file.set_len(end - 128).map_err(error)?;
            }
            file.seek(SeekFrom::End(0))
                .and_then(|_| file.write_all(&lyrics))
                .map_err(error)?;
            let v1 = if effects.wrote_v1 {
                current
            } else {
                self.v1.or(current)
            };
            if let Some(tag) = v1 {
                file.write_all(&tag).map_err(error)?;
            }
            crate::write_cost::rewrote_all();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn tag(items: &[u8], header: bool, version: u32) -> Vec<u8> {
        let size = (items.len() + 32) as u32;
        let block = |flags: u32| {
            let mut b = PREAMBLE.to_vec();
            for v in [version, size, 1, flags] {
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
        bytes.extend(block(if header { 0x8000_0000 } else { 0 }));
        bytes
    }

    #[test]
    fn the_phantom_parses_as_an_empty_id3v1_tag() {
        let tag =
            lofty::id3::v1::Id3v1Tag::parse(PHANTOM, lofty::config::ParsingMode::Strict).unwrap();
        assert_eq!(tag.title, None);
        assert_eq!(tag.year, Some(0));
        assert_eq!(tag.genre, None);
    }

    #[test]
    fn colliding_tags_are_found_in_headers_and_items() {
        // 131 bytes with a header: its "TAG" is 128 bytes before the end.
        let mut bytes = vec![1u8; 300];
        bytes.extend(tag(&[7u8; 67], true, 2000));
        assert_eq!(
            id3v1_lookalike(&mut Cursor::new(&bytes)).unwrap(),
            Some(303)
        );
        // One byte more and the header no longer collides.
        let mut other = vec![1u8; 300];
        other.extend(tag(&[7u8; 68], true, 2000));
        assert_eq!(id3v1_lookalike(&mut Cursor::new(&other)).unwrap(), None);
        // "TAG" inside the items of a header-less tag.
        let mut items = vec![7u8; 200];
        items[200 - 96..200 - 93].copy_from_slice(b"TAG");
        let mut inside = vec![1u8; 300];
        inside.extend(tag(&items, false, 2000));
        assert_eq!(&inside[inside.len() - 128..inside.len() - 125], b"TAG");
        let mut cursor = Cursor::new(&inside);
        assert_eq!(id3v1_lookalike(&mut cursor).unwrap(), Some(404));
        let footer = trailing(&mut cursor).unwrap().unwrap();
        assert_eq!((footer.at, footer.start()), (500, 300));
        let patches = vec![(508, 1000u32.to_le_bytes().to_vec())];
        assert_eq!(
            read_view(&mut cursor).unwrap(),
            View {
                patches: patches.clone(),
                append: true,
                phantom: true
            }
        );
        // An ID3v1 tag after the APE tag is not a collision; the phantom
        // itself (in a staged file) is still reported as one.
        let mut staged = inside.clone();
        staged.extend(PHANTOM);
        let mut cursor = Cursor::new(&staged);
        assert_eq!(id3v1_lookalike(&mut cursor).unwrap(), None);
        assert_eq!(trailing(&mut cursor).unwrap().unwrap().at, 500);
        let view = read_view(&mut cursor).unwrap();
        assert_eq!((view.append, view.phantom), (false, true));
        let mut real = inside.clone();
        real.extend(b"TAG");
        real.resize(real.len() + 125, 0);
        let view = read_view(&mut Cursor::new(&real)).unwrap();
        assert_eq!((view.append, view.phantom), (false, false));
    }

    #[test]
    fn the_guard_restores_untouched_tags_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        let mut items = vec![7u8; 200];
        items[200 - 96..200 - 93].copy_from_slice(b"TAG");
        let mut bytes = vec![1u8; 300];
        bytes.extend(tag(&items, false, 2000));
        std::fs::write(&path, &bytes).unwrap();
        let mut file = File::options().read(true).write(true).open(&path).unwrap();
        let guard = Guard::prepare(&mut file, true).unwrap();
        let staged = std::fs::read(&path).unwrap();
        assert_eq!(staged.len(), bytes.len() + 128);
        assert_eq!(&staged[508..512], 1000u32.to_le_bytes());
        assert_eq!(&staged[bytes.len()..], PHANTOM);
        guard
            .finish(&mut file, &crate::lyrics3::Effects::default())
            .unwrap();
        drop(file);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);

        // A rewritten tag is left as written.
        let mut file = File::options().read(true).write(true).open(&path).unwrap();
        let guard = Guard::prepare(&mut file, true).unwrap();
        let new = tag(b"other", true, 2000);
        file.set_len(300).unwrap();
        file.seek(SeekFrom::End(0)).unwrap();
        file.write_all(&new).unwrap();
        file.write_all(&PHANTOM).unwrap();
        guard
            .finish(&mut file, &crate::lyrics3::Effects::default())
            .unwrap();
        drop(file);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            [&[1u8; 300][..], &new].concat()
        );
    }
}
