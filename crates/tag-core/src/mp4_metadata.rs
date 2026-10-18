//! MP4 `ilst` writes that reuse the `free` atoms next to the tag.
//!
//! Plan `ilst` once, retaining raw data envelopes for native identities whose
//! values are unchanged. This preserves locale and encoding bytes absent from
//! Lofty's model. Explicit edits/removals use normal native encoding.
//!
//! Reuse adjacent padding or grow `moov` and adjust chunk offsets when possible.
//! Other layouts use Lofty's full layout writer with a structurally verified,
//! exactly sized surrogate `ilst`, replaced on staging before publication.
use crate::id3_padding::PADDING;
use lofty::{
    config::WriteOptions,
    mp4::{AtomData, AtomIdent, Ilst},
    tag::TagExt,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
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
#[cfg(test)]
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

/// An atom's kind and the byte range of its content.
type ChildRange = ([u8; 4], Range<u64>);

/// The children of the atom whose content is `range`, read header by
/// header; `None` when they do not tile it.
fn child_ranges(
    file: &mut (impl Read + Seek),
    range: Range<u64>,
) -> std::io::Result<Option<Vec<ChildRange>>> {
    let mut children = Vec::new();
    let mut at = range.start;
    while at + 8 <= range.end {
        file.seek(SeekFrom::Start(at))?;
        let mut head = [0u8; 16];
        file.read_exact(&mut head[..8])?;
        let (size, header) = match u32::from_be_bytes([head[0], head[1], head[2], head[3]]) {
            0 => (range.end - at, 8),
            1 => {
                file.read_exact(&mut head[8..])?;
                (
                    u64::from_be_bytes(head[8..16].try_into().expect("8 bytes")),
                    16,
                )
            }
            size => (u64::from(size), 8),
        };
        if size < header || at + size > range.end {
            return Ok(None);
        }
        children.push((
            head[4..8].try_into().expect("4 bytes"),
            at + header..at + size,
        ));
        at += size;
    }
    Ok((at == range.end).then_some(children))
}

/// lofty keeps only the 8-byte values of a `plID` (ITUNESALBUMID) item and
/// drops the item when none is 8 bytes long, even when reading strictly.
/// Collect every repeated `plID` item when any value has an unusual width,
/// keeping every value verbatim so a save does not drop its siblings.
/// Only atom headers and the `plID` items themselves are read.
pub(crate) fn odd_album_id(
    reader: &mut (impl Read + Seek),
) -> std::io::Result<Option<lofty::mp4::Atom<'static>>> {
    use lofty::mp4::{Atom as Item, AtomData, AtomIdent, DataType};
    let Some(moov) = top_level(reader)?.and_then(|top| {
        top.into_iter()
            .find(|(kind, _)| kind == b"moov")
            .map(|(_, range)| range)
    }) else {
        return Ok(None);
    };
    let mut size = [0u8; 4];
    reader.seek(SeekFrom::Start(moov.start))?;
    reader.read_exact(&mut size)?;
    let header = if size == 1u32.to_be_bytes() { 16 } else { 8 };
    let mut range = moov.start + header..moov.end;
    for kind in [b"udta", b"meta", b"ilst"] {
        if kind == b"ilst" {
            // ISO `meta` is a full atom: version and flags come first.
            let mut version = [0u8; 4];
            reader.seek(SeekFrom::Start(range.start))?;
            if reader.read_exact(&mut version).is_ok() && version == [0; 4] {
                range.start += 4;
            }
        }
        let Some(children) = child_ranges(reader, range.clone())? else {
            return Ok(None);
        };
        let Some((_, found)) = children.into_iter().find(|(k, _)| k == kind) else {
            return Ok(None);
        };
        range = found;
    }
    let Some(items) = child_ranges(reader, range)? else {
        return Ok(None);
    };
    let mut values = Vec::new();
    for (_, range) in items.into_iter().filter(|(kind, _)| kind == b"plID") {
        if range.end - range.start > 1 << 16 {
            return Ok(None);
        }
        let item = read_range(reader, &range)?;
        let Some(children) = atoms(&item, 0..item.len()) else {
            return Ok(None);
        };
        for data in children.iter().filter(|atom| &atom.kind == b"data") {
            let content = &item[data.content()];
            if content.len() < 8 || content[0] != 0 {
                return Ok(None);
            }
            values.push(AtomData::Unknown {
                code: DataType::from(u32::from_be_bytes([0, content[1], content[2], content[3]])),
                data: content[8..].to_vec(),
            });
        }
    }
    if values
        .iter()
        .all(|value| matches!(value, AtomData::Unknown { data, .. } if data.len() == 8))
    {
        return Ok(None);
    }
    Ok(Item::from_collection(AtomIdent::Fourcc(*b"plID"), values))
}

/// Write `tag` into the MP4 file, in place when the `free` atoms allow it.
/// Unless `single_atom`, a field with several values is stored as one atom
/// per value (Options → Tags → Advanced).
pub(crate) fn save(
    file: &mut File,
    before: &Ilst,
    tag: &Ilst,
    touched: &[AtomIdent<'_>],
    single_atom: bool,
) -> Result<(), crate::batch_errors::ErrorCause> {
    let plan = preserved_ilst(file, before, tag, touched, single_atom).map_err(SaveError::cause)?;
    match padded(file, &plan).map_err(crate::batch_errors::ErrorCause::external)? {
        Some(true) => return Ok(()),
        Some(false) => crate::write_cost::rewrote_all(),
        None => {
            crate::write_cost::rewrote_all();
            write_full_plan(file, &plan).map_err(SaveError::cause)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ItemKey {
    Fourcc([u8; 4]),
    Freeform(String, String),
}
fn item_key(id: &AtomIdent<'_>) -> ItemKey {
    match id {
        AtomIdent::Fourcc(id) => ItemKey::Fourcc(*id),
        AtomIdent::Freeform { mean, name } => ItemKey::Freeform(mean.to_string(), name.to_string()),
    }
}
#[derive(Clone)]
struct RawItem {
    key: ItemKey,
    bytes: Vec<u8>,
    kind: [u8; 4],
    prefix: Vec<u8>,
    data: Vec<Vec<u8>>,
    wide: bool,
}
#[derive(Debug)]
enum SaveError {
    Structure,
    External(String),
}
const STRUCTURE_GUARD: SaveError = SaveError::Structure;
impl From<String> for SaveError {
    fn from(value: String) -> Self {
        Self::External(value)
    }
}
impl SaveError {
    fn cause(self) -> crate::batch_errors::ErrorCause {
        match self {
            Self::Structure => crate::batch_errors::ErrorCause::owned(
                "MP4 metadata structure cannot be preserved safely; original file was not changed",
            ),
            Self::External(message) => crate::batch_errors::ErrorCause::external(message),
        }
    }
}

fn resolved_bytes(bytes: &[u8], atom: Atom) -> Result<Vec<u8>, SaveError> {
    let mut raw = bytes[atom.start..atom.end].to_vec();
    if raw[..4] == [0; 4] {
        let length = u32::try_from(raw.len()).map_err(|_| STRUCTURE_GUARD)?;
        raw[..4].copy_from_slice(&length.to_be_bytes());
    }
    Ok(raw)
}
fn raw_items(bytes: &[u8]) -> Result<Vec<RawItem>, SaveError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let root = atoms(bytes, 0..bytes.len())
        .filter(|v| v.len() == 1 && v[0].kind == *b"ilst")
        .ok_or(STRUCTURE_GUARD)?[0];
    atoms(bytes, root.content())
        .ok_or(STRUCTURE_GUARD)?
        .into_iter()
        .map(|item| {
            let children = if matches!(&item.kind, b"free" | b"skip") {
                Vec::new()
            } else {
                atoms(bytes, item.content()).ok_or(STRUCTURE_GUARD)?
            };
            let key = if item.kind == *b"----" {
                let text = |kind| -> Result<String, SaveError> {
                    let found: Vec<_> = children.iter().filter(|c| c.kind == kind).collect();
                    if found.len() != 1 {
                        return Err(STRUCTURE_GUARD);
                    }
                    let c = found[0];
                    String::from_utf8(
                        bytes
                            .get(c.content().start + 4..c.end)
                            .ok_or(STRUCTURE_GUARD)?
                            .to_vec(),
                    )
                    .map_err(|_| STRUCTURE_GUARD)
                };
                ItemKey::Freeform(text(*b"mean")?, text(*b"name")?)
            } else {
                ItemKey::Fourcc(item.kind)
            };
            let prefix = children
                .iter()
                .filter(|c| c.kind != *b"data")
                .map(|c| resolved_bytes(bytes, *c))
                .collect::<Result<Vec<_>, _>>()?
                .concat();
            let data = children
                .iter()
                .filter(|c| c.kind == *b"data")
                .map(|c| resolved_bytes(bytes, *c))
                .collect::<Result<Vec<_>, _>>()?;
            let mut raw = resolved_bytes(bytes, item)?;
            for child in &children {
                if bytes[child.start..child.start + 4] == [0; 4] {
                    let start = child.start - item.start;
                    raw[start..start + 4].copy_from_slice(
                        &u32::try_from(child.end - child.start)
                            .map_err(|_| STRUCTURE_GUARD)?
                            .to_be_bytes(),
                    );
                }
            }
            Ok(RawItem {
                key,
                bytes: raw,
                kind: item.kind,
                prefix,
                data,
                wide: item.header == 16,
            })
        })
        .collect()
}
fn make_item(item: &RawItem, data: &[Vec<u8>]) -> Result<Vec<u8>, SaveError> {
    let length = (if item.wide { 16usize } else { 8 })
        .checked_add(item.prefix.len())
        .and_then(|n| data.iter().try_fold(n, |n, v| n.checked_add(v.len())))
        .ok_or(STRUCTURE_GUARD)?;
    let mut bytes = if item.wide {
        let mut b = 1u32.to_be_bytes().to_vec();
        b.extend(item.kind);
        b.extend((length as u64).to_be_bytes());
        b
    } else {
        let mut b = u32::try_from(length)
            .map_err(|_| STRUCTURE_GUARD)?
            .to_be_bytes()
            .to_vec();
        b.extend(item.kind);
        b
    };
    bytes.extend(&item.prefix);
    for value in data {
        bytes.extend(value);
    }
    Ok(bytes)
}
fn locate_path(bytes: &[u8], path: &[[u8; 4]]) -> Result<Option<Range<usize>>, SaveError> {
    let top = atoms(bytes, 0..bytes.len()).ok_or(STRUCTURE_GUARD)?;
    let movies: Vec<_> = top.into_iter().filter(|a| a.kind == *b"moov").collect();
    if movies.len() != 1 {
        return Err(STRUCTURE_GUARD);
    }
    let mut parent = movies[0];
    for &kind in path {
        let mut range = parent.content();
        if parent.kind == *b"meta" && bytes.get(range.start..range.start + 4) == Some(&[0; 4]) {
            range.start += 4;
        }
        let found: Vec<_> = atoms(bytes, range)
            .ok_or(STRUCTURE_GUARD)?
            .into_iter()
            .filter(|a| a.kind == kind)
            .collect();
        if found.is_empty() {
            return Ok(None);
        }
        if found.len() != 1 {
            return Err(STRUCTURE_GUARD);
        }
        parent = found[0];
    }
    Ok(Some(parent.start..parent.end))
}
fn locate_ilst(bytes: &[u8]) -> Result<Option<Range<usize>>, SaveError> {
    locate_path(bytes, &[*b"udta", *b"meta", *b"ilst"])
}
fn original_ilst(file: &mut File) -> Result<Vec<u8>, SaveError> {
    let top = top_level(file)
        .map_err(|e| e.to_string())?
        .ok_or(STRUCTURE_GUARD)?;
    let movies: Vec<_> = top
        .into_iter()
        .filter(|(kind, _)| *kind == *b"moov")
        .collect();
    if movies.len() != 1 {
        return Err(STRUCTURE_GUARD);
    }
    let moov = read_range(file, &movies[0].1).map_err(|e| e.to_string())?;
    Ok(locate_ilst(&moov)?
        .map(|range| moov[range].to_vec())
        .unwrap_or_default())
}
fn native_groups(tag: &Ilst) -> BTreeMap<ItemKey, Vec<lofty::mp4::Atom<'static>>> {
    let mut out: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for atom in tag {
        out.entry(item_key(atom.ident()))
            .or_default()
            .push(atom.clone());
    }
    out
}
fn preserved_ilst(
    file: &mut File,
    before: &Ilst,
    after: &Ilst,
    touched: &[AtomIdent<'_>],
    single: bool,
) -> Result<Vec<u8>, SaveError> {
    let old = raw_items(&original_ilst(file)?)?;
    let mut encoded = Vec::new();
    after
        .dump_to(&mut encoded, WriteOptions::default())
        .map_err(|e| e.to_string())?;
    let new = raw_items(&encoded)?;
    let old_native = native_groups(before);
    let new_native = native_groups(after);
    let dirty: BTreeSet<_> = touched.iter().map(item_key).collect();
    let mut originals: BTreeMap<ItemKey, Vec<&RawItem>> = BTreeMap::new();
    for item in &old {
        originals.entry(item.key.clone()).or_default().push(item);
    }
    let mut templates: BTreeMap<ItemKey, Vec<&RawItem>> = BTreeMap::new();
    for item in &new {
        templates.entry(item.key.clone()).or_default().push(item);
    }
    let mut reused: BTreeMap<ItemKey, VecDeque<Vec<u8>>> = BTreeMap::new();
    for (key, items) in &originals {
        if dirty.contains(key) || old_native.get(key) != new_native.get(key) {
            continue;
        }
        let flat: Vec<_> = items
            .iter()
            .flat_map(|item| item.data.iter().map(move |data| (*item, data)))
            .collect();
        let shape = templates.get(key);
        let count = shape
            .map(|items| items.iter().map(|i| i.data.len()).sum::<usize>())
            .unwrap_or(0);
        let mut output = VecDeque::new();
        if count == flat.len() && count > 0 {
            let mut at = 0;
            for template in shape.unwrap() {
                let end = at + template.data.len();
                if end == at {
                    return Err(STRUCTURE_GUARD);
                }
                let source = flat[at].0;
                let data: Vec<_> = flat[at..end]
                    .iter()
                    .map(|(_, data)| (*data).clone())
                    .collect();
                output.push_back(if source.data == data {
                    source.bytes.clone()
                } else {
                    // Regrouping follows the established native atom shape;
                    // preserve each data envelope, not obsolete item wrappers.
                    make_item(source, &data)?
                });
                at = end;
            }
        } else {
            output.extend(items.iter().map(|item| item.bytes.clone()));
        }
        reused.insert(key.clone(), output);
    }
    let mut remaining: BTreeMap<ItemKey, usize> = BTreeMap::new();
    for item in &new {
        *remaining.entry(item.key.clone()).or_default() += 1;
    }
    let mut body = Vec::new();
    for item in &new {
        let count = remaining.get_mut(&item.key).unwrap();
        *count -= 1;
        if let Some(queue) = reused.get_mut(&item.key) {
            if let Some(bytes) = queue.pop_front() {
                body.extend(bytes);
            }
            if *count == 0 {
                for bytes in queue.drain(..) {
                    body.extend(bytes);
                }
            }
        } else {
            body.extend(&item.bytes);
        }
    }
    // Items the native reader cannot model are still retained unless this exact
    // physical identity was explicitly edited/removed.
    for item in &old {
        if let Some(queue) = reused.get_mut(&item.key) {
            for bytes in queue.drain(..) {
                body.extend(bytes);
            }
        }
    }
    if body.is_empty() {
        return Ok(Vec::new());
    }
    let mut result = u32::try_from(body.len() + 8)
        .map_err(|_| STRUCTURE_GUARD)?
        .to_be_bytes()
        .to_vec();
    result.extend(b"ilst");
    result.extend(body);
    if !single {
        result = split_items(&result).ok_or(STRUCTURE_GUARD)?;
    }
    Ok(result)
}
/// Keep all byte offsets while adapting small extended containers to Lofty's
/// 32-bit size updates. The extended-size word becomes a legal padding child.
fn normalize_extended_containers(bytes: &mut [u8]) -> Result<bool, SaveError> {
    let mut quicktime_adapter = false;
    let mut range = 0..bytes.len();
    for kind in [*b"moov", *b"udta", *b"meta"] {
        let found: Vec<_> = atoms(bytes, range)
            .ok_or(STRUCTURE_GUARD)?
            .into_iter()
            .filter(|a| a.kind == kind)
            .collect();
        if found.is_empty() {
            return Ok(quicktime_adapter);
        }
        if found.len() != 1 {
            return Err(STRUCTURE_GUARD);
        }
        let atom = found[0];
        let mut start = atom.start + atom.header;
        if atom.header == 16 {
            if let Ok(length) = u32::try_from(atom.end - atom.start) {
                if kind == *b"meta" {
                    // Lofty accepts ISO full and QuickTime non-full meta.
                    if bytes.get(start..start + 4) == Some(&[0; 4]) {
                        bytes.copy_within(start..start + 4, atom.start + 8);
                        bytes[atom.start + 12..atom.start + 20].copy_from_slice(&free_atom(8));
                        start += 4;
                    } else {
                        // `meta_is_full` recognizes `mhdr`, but not padding,
                        // as a first child of a QuickTime (non-full) meta.
                        bytes[atom.start + 8..atom.start + 16]
                            .copy_from_slice(&[0, 0, 0, 8, b'm', b'h', b'd', b'r']);
                        quicktime_adapter = true;
                    }
                } else {
                    bytes[atom.start + 8..atom.start + 16].copy_from_slice(&free_atom(8));
                }
                bytes[atom.start..atom.start + 4].copy_from_slice(&length.to_be_bytes());
            }
        } else if kind == *b"meta" && bytes.get(start..start + 4) == Some(&[0; 4]) {
            start += 4;
        }
        range = start..atom.end;
    }
    Ok(quicktime_adapter)
}
fn restore_quicktime_header(bytes: &mut [u8]) -> Result<(), SaveError> {
    let meta = locate_path(bytes, &[*b"udta", *b"meta"])?.ok_or(STRUCTURE_GUARD)?;
    let start = meta.start;
    let header = bytes.get(start..start + 16).ok_or(STRUCTURE_GUARD)?;
    if header[..4]
        != u32::try_from(meta.len())
            .map_err(|_| STRUCTURE_GUARD)?
            .to_be_bytes()
        || header[8..16] != [0, 0, 0, 8, b'm', b'h', b'd', b'r']
    {
        return Err(STRUCTURE_GUARD);
    }
    // Reuse the adapter slot as the original extended-size word. No child
    // moves, and no synthetic metadata atom reaches the published file.
    bytes[start..start + 4].copy_from_slice(&1u32.to_be_bytes());
    bytes[start + 8..start + 16].copy_from_slice(&(meta.len() as u64).to_be_bytes());
    Ok(())
}
fn write_full_plan(file: &mut File, plan: &[u8]) -> Result<(), SaveError> {
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut source = Vec::new();
    file.read_to_end(&mut source).map_err(|e| e.to_string())?;
    let quicktime_adapter = normalize_extended_containers(&mut source)?;
    let mut candidate = Cursor::new(source);
    if plan.is_empty() {
        Ilst::default()
            .save_to(&mut candidate, WriteOptions::default())
            .map_err(|e| crate::tags::error_message(&e))?;
    } else {
        let length = if plan.len() < 32 {
            if 32 - plan.len() >= 8 {
                32
            } else {
                plan.len() + 8
            }
        } else {
            plan.len()
        };
        let mut surrogate = Ilst::default();
        surrogate.insert(lofty::mp4::Atom::new(
            AtomIdent::Fourcc(*b"tGRv"),
            AtomData::Unknown {
                code: lofty::mp4::DataType::Reserved,
                data: vec![0; length - 32],
            },
        ));
        let mut expected = Vec::new();
        surrogate
            .dump_to(&mut expected, WriteOptions::default())
            .map_err(|e| e.to_string())?;
        if expected.len() != length {
            return Err(STRUCTURE_GUARD);
        }
        surrogate
            .save_to(&mut candidate, WriteOptions::default())
            .map_err(|e| crate::tags::error_message(&e))?;
        let range = locate_ilst(candidate.get_ref())?.ok_or(STRUCTURE_GUARD)?;
        if candidate.get_ref().get(range.clone()) != Some(expected.as_slice()) {
            return Err(STRUCTURE_GUARD);
        }
        let mut replacement = plan.to_vec();
        if length > plan.len() {
            replacement.extend(free_atom(length - plan.len()));
        }
        if replacement.len() != range.len() {
            return Err(STRUCTURE_GUARD);
        }
        candidate.get_mut()[range].copy_from_slice(&replacement);
    }
    let mut bytes = candidate.into_inner();
    if quicktime_adapter {
        restore_quicktime_header(&mut bytes)?;
    }
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.write_all(&bytes))
        .and_then(|_| file.set_len(bytes.len() as u64))
        .map_err(|e| SaveError::External(e.to_string()))
}

/// One item atom per `data` atom: an `ilst` item with several values becomes
/// several items with the same identifier (and `mean`/`name`). Cover art
/// keeps all its images in one `covr` atom.
fn split_items(ilst: &[u8]) -> Option<Vec<u8>> {
    let root = atoms(ilst, 0..ilst.len())?.into_iter().next()?;
    let mut body = Vec::new();
    for item in atoms(ilst, root.content())? {
        if matches!(&item.kind, b"free" | b"skip") {
            body.extend_from_slice(&ilst[item.start..item.end]);
            continue;
        }
        let children = atoms(ilst, item.content())?;
        let data: Vec<_> = children.iter().filter(|a| &a.kind == b"data").collect();
        if data.len() < 2 || &item.kind == b"covr" {
            body.extend_from_slice(&ilst[item.start..item.end]);
            continue;
        }
        let names: Vec<u8> = children
            .iter()
            .filter(|a| &a.kind != b"data")
            .flat_map(|a| ilst[a.start..a.end].iter().copied())
            .collect();
        for value in data {
            let size = u32::try_from(8 + names.len() + (value.end - value.start)).ok()?;
            body.extend(size.to_be_bytes());
            body.extend(item.kind);
            body.extend(&names);
            body.extend_from_slice(&ilst[value.start..value.end]);
        }
    }
    let mut output = u32::try_from(8 + body.len()).ok()?.to_be_bytes().to_vec();
    output.extend(b"ilst");
    output.extend(body);
    Some(output)
}

/// `Some(true)`: written in place; `Some(false)`: `moov` was rewritten with
/// padding; `None`: nothing was written and lofty has to write the tag.
fn padded(file: &mut File, new: &[u8]) -> std::io::Result<Option<bool>> {
    if new.is_empty() {
        return Ok(None);
    }
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
    let _ = ftyp;
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
    fn full_writer_tiny_and_empty_plans_have_legal_adjacent_padding() {
        let carrier = include_bytes!("../tests/fixtures/save-corpus/tone.m4a");
        for length in [16, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33] {
            let plan = atom(b"ilst", &atom(b"free", &vec![0; length - 16]));
            let mut file = tempfile::tempfile().unwrap();
            file.write_all(carrier).unwrap();
            write_full_plan(&mut file, &plan).unwrap();
            file.seek(SeekFrom::Start(0)).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            let range = locate_ilst(&bytes).unwrap().unwrap();
            assert_eq!(&bytes[range], plan);
            let moov = top_level(&mut file)
                .unwrap()
                .unwrap()
                .into_iter()
                .find(|(k, _)| k == b"moov")
                .unwrap()
                .1;
            assert!(meta(&read_range(&mut file, &moov).unwrap()).is_some());
        }
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(carrier).unwrap();
        write_full_plan(&mut file, &[]).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert!(locate_ilst(&bytes).unwrap().is_none());
    }
    #[test]
    fn structural_guard_has_owned_provenance_and_io_remains_external() {
        use crate::batch_errors::ErrorCause;
        assert!(matches!(
            raw_items(b"not an ilst").err().unwrap().cause(),
            ErrorCause::Owned { .. }
        ));
        let mut file = File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/save-corpus/tone.m4a"
        ))
        .unwrap();
        assert!(matches!(
            write_full_plan(&mut file, &[]).unwrap_err().cause(),
            ErrorCause::External { .. }
        ));
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
