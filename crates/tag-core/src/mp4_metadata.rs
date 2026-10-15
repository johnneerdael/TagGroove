//! MP4 `ilst` writes that reuse the `free` atoms next to the tag.
//!
//! lofty reads a whole MP4 file into memory and writes all of it back for
//! every tag change, even when the new `ilst` fits the `free` atoms beside the
//! old one. Here lofty encodes the new tag against an in-memory copy of only
//! the `ftyp` and `moov` atoms. When the encoded `ilst` fits the old `ilst`
//! plus its neighbouring `free` atoms inside `moov.udta.meta`, it is written
//! over that region and the rest becomes a `free` atom: `moov` keeps its size,
//! so no chunk offset changes and the media data is never touched.
//!
//! A tag that outgrows the region is spliced into `moov` with a [`PADDING`]
//! `free` atom after it, so later edits fit; chunk offsets that point past
//! `moov` are shifted. Files without an `ilst`, fragmented files and 64-bit
//! container sizes fall back to lofty's full write.
use crate::id3_padding::PADDING;
use lofty::{config::WriteOptions, mp4::Ilst, tag::TagExt};
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    ops::Range,
};

#[derive(Clone, Copy)]
struct Atom {
    kind: [u8; 4],
    start: usize,
    header: usize,
    end: usize,
}
impl Atom {
    fn content(&self) -> Range<usize> {
        self.start + self.header..self.end
    }
}

/// The atoms in `data[range]`, or `None` when they do not tile it exactly.
fn atoms(data: &[u8], range: Range<usize>) -> Option<Vec<Atom>> {
    let mut atoms = Vec::new();
    let mut at = range.start;
    while at < range.end {
        let head = data.get(at..at + 8)?;
        let (size, header) = match u32::from_be_bytes(head[..4].try_into().ok()?) {
            0 => (range.end - at, 8),
            1 => {
                let size = u64::from_be_bytes(data.get(at + 8..at + 16)?.try_into().ok()?);
                (usize::try_from(size).ok()?, 16)
            }
            size => (size as usize, 8),
        };
        if size < header || at + size > range.end {
            return None;
        }
        atoms.push(Atom {
            kind: head[4..8].try_into().ok()?,
            start: at,
            header,
            end: at + size,
        });
        at += size;
    }
    Some(atoms)
}

fn child(data: &[u8], parent: &Atom, kind: &[u8; 4]) -> Option<Atom> {
    atoms(data, parent.content())?
        .into_iter()
        .find(|a| &a.kind == kind)
}

/// `moov.udta` and `moov.udta.meta`, and the children of `meta`. ISO `meta`
/// is a full atom (version and flags precede its children); QuickTime's is not.
fn meta(moov: &[u8]) -> Option<(Atom, Atom, Vec<Atom>)> {
    let root = atoms(moov, 0..moov.len())?.into_iter().next()?;
    let udta = child(moov, &root, b"udta")?;
    let meta = child(moov, &udta, b"meta")?;
    let mut content = meta.content();
    if moov.get(content.start..content.start + 4)? == [0; 4] {
        content.start += 4;
    }
    Some((udta, meta, atoms(moov, content)?))
}

/// The `ilst` atom of an encoded `moov`, if it has one.
fn ilst(moov: &[u8]) -> Option<&[u8]> {
    let (_, _, children) = meta(moov)?;
    let ilst = children.iter().find(|a| &a.kind == b"ilst")?;
    Some(&moov[ilst.start..ilst.end])
}

/// The existing `ilst` plus the `free` atoms directly around it.
fn region(children: &[Atom]) -> Option<Range<usize>> {
    let index = children.iter().position(|a| &a.kind == b"ilst")?;
    let free = |a: &Atom| &a.kind == b"free" || &a.kind == b"skip";
    let first = children[..index]
        .iter()
        .rposition(|a| !free(a))
        .map_or(0, |i| i + 1);
    let last = children[index + 1..]
        .iter()
        .position(|a| !free(a))
        .map_or(children.len(), |i| index + 1 + i);
    Some(children[first].start..children[last - 1].end)
}

fn free_atom(size: usize) -> Vec<u8> {
    let mut atom = (size as u32).to_be_bytes().to_vec();
    atom.extend_from_slice(b"free");
    atom.resize(size, 0);
    atom
}

/// A top-level atom: its kind and byte range in the file.
type TopAtom = ([u8; 4], Range<u64>);

/// Top-level atoms of the file, or `None` when they do not tile it.
fn top_level(file: &mut (impl Read + Seek)) -> std::io::Result<Option<Vec<TopAtom>>> {
    let length = file.seek(SeekFrom::End(0))?;
    let mut atoms = Vec::new();
    let mut at = 0u64;
    while at + 8 <= length {
        file.seek(SeekFrom::Start(at))?;
        let mut head = [0u8; 16];
        file.read_exact(&mut head[..8])?;
        let size = match u32::from_be_bytes([head[0], head[1], head[2], head[3]]) {
            0 => length - at,
            1 => {
                file.read_exact(&mut head[8..])?;
                u64::from_be_bytes(head[8..16].try_into().expect("8 bytes"))
            }
            size => u64::from(size),
        };
        if size < 8 || at + size > length {
            return Ok(None);
        }
        atoms.push((head[4..8].try_into().expect("4 bytes"), at..at + size));
        at += size;
    }
    Ok(Some(atoms))
}

fn read_range(file: &mut (impl Read + Seek), range: &Range<u64>) -> std::io::Result<Vec<u8>> {
    let mut bytes = vec![0; usize::try_from(range.end - range.start).unwrap_or(usize::MAX)];
    file.seek(SeekFrom::Start(range.start))?;
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// lofty's strict reader rejects an `ilst` item whose integer `data` is not
/// one to four bytes long, such as the 8-byte `plID` (ITUNESALBUMID) older
/// iTunes versions stored as a signed integer; its relaxed reader drops the
/// item. These are such items, with their data kept verbatim so a later save
/// writes them back unchanged, and the number of items in the `ilst`.
pub(crate) fn legacy_integer_items(
    reader: &mut (impl Read + Seek),
) -> std::io::Result<(Vec<lofty::mp4::Atom<'static>>, usize)> {
    use lofty::mp4::{Atom as Item, AtomData, AtomIdent, DataType};
    let none = (Vec::new(), 0);
    let Some(moov) = top_level(reader)?.and_then(|top| {
        top.into_iter()
            .find(|(kind, _)| kind == b"moov")
            .map(|(_, range)| range)
    }) else {
        return Ok(none);
    };
    let moov = read_range(reader, &moov)?;
    let Some(ilst) = meta(&moov)
        .and_then(|(_, _, children)| children.into_iter().find(|atom| &atom.kind == b"ilst"))
    else {
        return Ok(none);
    };
    let Some(items) = atoms(&moov, ilst.content()) else {
        return Ok(none);
    };
    let text = |atom: &Atom| {
        // `mean` and `name` are full atoms: version and flags come first.
        moov.get(atom.content().start + 4..atom.end)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    };
    let mut legacy = Vec::new();
    for item in &items {
        let Some(children) = atoms(&moov, item.content()) else {
            continue;
        };
        let mut values = Vec::new();
        let mut odd = false;
        for data in children.iter().filter(|atom| &atom.kind == b"data") {
            let Some(content) = moov.get(data.content()) else {
                continue;
            };
            if content.len() < 8 || content[0] != 0 {
                continue;
            }
            let code = u32::from_be_bytes([0, content[1], content[2], content[3]]);
            let payload = &content[8..];
            if matches!(code, 21 | 22) && !(1..=4).contains(&payload.len()) {
                odd = true;
            }
            values.push(AtomData::Unknown {
                code: DataType::from(code),
                data: payload.to_vec(),
            });
        }
        if !odd {
            continue;
        }
        let ident = if &item.kind == b"----" {
            let find = |kind: &[u8; 4]| children.iter().find(|a| &a.kind == kind).and_then(text);
            let (Some(mean), Some(name)) = (find(b"mean"), find(b"name")) else {
                continue;
            };
            AtomIdent::Freeform {
                mean: mean.into(),
                name: name.into(),
            }
        } else {
            AtomIdent::Fourcc(item.kind)
        };
        if let Some(atom) = Item::from_collection(ident, values) {
            legacy.push(atom);
        }
    }
    Ok((legacy, items.len()))
}

/// Write `tag` into the MP4 file, in place when the `free` atoms allow it.
pub(crate) fn save(file: &mut File, tag: &Ilst) -> Result<(), String> {
    match padded(file, tag).map_err(|e| e.to_string())? {
        Some(true) => return Ok(()),
        Some(false) => crate::write_cost::rewrote_all(),
        None => {
            crate::write_cost::rewrote_all();
            file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            tag.save_to(file, WriteOptions::default())
                .map_err(|e| crate::tags::error_message(&e))?;
        }
    }
    Ok(())
}

/// `Some(true)`: written in place; `Some(false)`: `moov` was rewritten with
/// padding; `None`: nothing was written and lofty has to write the tag.
fn padded(file: &mut File, tag: &Ilst) -> std::io::Result<Option<bool>> {
    let Some(top) = top_level(file)? else {
        return Ok(None);
    };
    let (Some(ftyp), Some(moov)) = (
        top.first().filter(|(kind, _)| kind == b"ftyp"),
        top.iter().find(|(kind, _)| kind == b"moov"),
    ) else {
        return Ok(None);
    };
    if top.iter().filter(|(kind, _)| kind == b"moov").count() != 1
        || top.iter().any(|(kind, _)| kind == b"moof")
    {
        return Ok(None);
    }
    let moov_range = moov.1.clone();
    let original = read_range(file, &moov_range)?;
    let Some((udta, meta, children)) = meta(&original) else {
        return Ok(None);
    };
    let Some(region) = region(&children) else {
        return Ok(None);
    };
    // lofty encodes the tag into a copy of `ftyp` and `moov` alone.
    let mut copy = read_range(file, &ftyp.1)?;
    let moov_at = copy.len();
    copy.extend_from_slice(&original);
    let mut encoded = Cursor::new(copy);
    if tag.save_to(&mut encoded, WriteOptions::default()).is_err() {
        return Ok(None);
    }
    let encoded = encoded.into_inner();
    let Some(new) = encoded.get(moov_at..).and_then(ilst) else {
        // Removing the tag is left to lofty.
        return Ok(None);
    };
    let available = region.len();
    if new.len() == available || new.len() + 8 <= available {
        let mut bytes = new.to_vec();
        if bytes.len() < available {
            bytes.extend(free_atom(available - new.len()));
        }
        file.seek(SeekFrom::Start(moov_range.start + region.start as u64))?;
        file.write_all(&bytes)?;
        crate::write_cost::rewrote_region(available as u64);
        return Ok(Some(true));
    }
    // Grow: splice the tag and fresh padding into `moov`.
    let mut moov = original[..region.start].to_vec();
    moov.extend_from_slice(new);
    moov.extend(free_atom(PADDING as usize));
    moov.extend_from_slice(&original[region.end..]);
    let delta = moov.len() - original.len();
    for start in [0, udta.start, meta.start] {
        let size = u32::from_be_bytes(moov[start..start + 4].try_into().expect("4 bytes"));
        let Some(grown) = (size > 1)
            .then(|| u32::try_from(size as usize + delta).ok())
            .flatten()
        else {
            return Ok(None);
        };
        moov[start..start + 4].copy_from_slice(&grown.to_be_bytes());
    }
    if !shift_chunk_offsets(&mut moov, moov_range.end, delta as u64) {
        return Ok(None);
    }
    // Everything after `moov` moves: write it once after the new `moov`.
    let length = file.metadata()?.len();
    let tail = read_range(file, &(moov_range.end..length))?;
    file.seek(SeekFrom::Start(moov_range.start))?;
    file.write_all(&moov)?;
    file.write_all(&tail)?;
    Ok(Some(false))
}

/// Add `delta` to every `stco`/`co64` chunk offset at or after `from` (data
/// stored after `moov`, which moves when `moov` grows). `false` if malformed.
fn shift_chunk_offsets(moov: &mut [u8], from: u64, delta: u64) -> bool {
    let Some(root) = atoms(moov, 0..moov.len()).and_then(|a| a.into_iter().next()) else {
        return false;
    };
    let mut pending = vec![root];
    let mut tables = Vec::new();
    while let Some(atom) = pending.pop() {
        match &atom.kind {
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" => {
                let Some(children) = atoms(moov, atom.content()) else {
                    return false;
                };
                pending.extend(children);
            }
            b"stco" | b"co64" => tables.push(atom),
            _ => {}
        }
    }
    for table in tables {
        let wide = &table.kind == b"co64";
        let width = if wide { 8 } else { 4 };
        let content = table.content();
        let Some(count) = moov
            .get(content.start + 4..content.start + 8)
            .map(|b| u32::from_be_bytes(b.try_into().expect("4 bytes")) as usize)
        else {
            return false;
        };
        let entries = content.start + 8;
        if entries + count * width > content.end {
            return false;
        }
        for entry in (0..count).map(|i| entries + i * width) {
            let slot = &mut moov[entry..entry + width];
            if wide {
                let offset = u64::from_be_bytes(slot.try_into().expect("8 bytes"));
                if offset >= from {
                    slot.copy_from_slice(&(offset + delta).to_be_bytes());
                }
            } else {
                let offset = u64::from(u32::from_be_bytes(slot.try_into().expect("4 bytes")));
                if offset >= from {
                    let Ok(moved) = u32::try_from(offset + delta) else {
                        return false;
                    };
                    slot.copy_from_slice(&moved.to_be_bytes());
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(kind: &[u8; 4], content: &[u8]) -> Vec<u8> {
        let mut out = ((content.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(content);
        out
    }

    #[test]
    fn region_spans_free_atoms_around_ilst_only() {
        let hdlr = atom(b"hdlr", &[1; 25]);
        let before = atom(b"free", &[0; 4]);
        let ilst = atom(b"ilst", &[2; 10]);
        let after = atom(b"free", &[0; 20]);
        let other = atom(b"xtra", &[3; 2]);
        let mut content = vec![0; 4];
        for part in [&hdlr, &before, &ilst, &after, &other] {
            content.extend_from_slice(part);
        }
        let moov = atom(b"moov", &atom(b"udta", &atom(b"meta", &content)));
        let (_, _, children) = meta(&moov).unwrap();
        let region = region(&children).unwrap();
        assert_eq!(region.len(), before.len() + ilst.len() + after.len());
        assert_eq!(&moov[region.start..region.start + 12], &before[..12]);
        assert_eq!(ilst_bytes(&moov), ilst);
    }

    fn ilst_bytes(moov: &[u8]) -> Vec<u8> {
        ilst(moov).unwrap().to_vec()
    }

    #[test]
    fn chunk_offsets_after_moov_shift() {
        let mut stco = vec![0; 4];
        stco.extend(2u32.to_be_bytes());
        stco.extend(10u32.to_be_bytes());
        stco.extend(500u32.to_be_bytes());
        let mut co64 = vec![0; 4];
        co64.extend(1u32.to_be_bytes());
        co64.extend(600u64.to_be_bytes());
        let stbl = [atom(b"stco", &stco), atom(b"co64", &co64)].concat();
        let trak = atom(
            b"trak",
            &atom(b"mdia", &atom(b"minf", &atom(b"stbl", &stbl))),
        );
        let mut moov = atom(b"moov", &trak);
        assert!(shift_chunk_offsets(&mut moov, 100, 7));
        let at = moov.len() - 8 - 16 - 8;
        let entries: Vec<u32> = (0..2)
            .map(|i| u32::from_be_bytes(moov[at + i * 4..at + i * 4 + 4].try_into().unwrap()))
            .collect();
        assert_eq!(entries, [10, 507]);
        let last = u64::from_be_bytes(moov[moov.len() - 8..].try_into().unwrap());
        assert_eq!(last, 607);
    }
}
