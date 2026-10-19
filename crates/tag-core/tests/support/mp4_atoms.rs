//! Independent structural observer for test fixtures; no Lofty/tag-core parser.
#![allow(dead_code)]
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Identity {
    Fourcc([u8; 4]),
    Freeform(Vec<u8>, Vec<u8>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Data {
    pub version: u8,
    pub code: u32,
    pub locale: u32,
    pub payload: Vec<u8>,
    pub raw: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub identity: Identity,
    pub values: Vec<Data>,
    pub raw: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct Box {
    pub kind: [u8; 4],
    pub start: usize,
    pub body: usize,
    pub end: usize,
}

pub fn boxes(bytes: &[u8], range: Range<usize>) -> Vec<Box> {
    let mut at = range.start;
    let mut out = Vec::new();
    while at < range.end {
        assert!(at + 8 <= range.end, "truncated box header");
        let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
        let (size, header) = match size {
            0 => (range.end - at, 8),
            1 => {
                assert!(at + 16 <= range.end);
                (
                    usize::try_from(u64::from_be_bytes(
                        bytes[at + 8..at + 16].try_into().unwrap(),
                    ))
                    .unwrap(),
                    16,
                )
            }
            n => (n as usize, 8),
        };
        assert!(size >= header && size <= range.end - at, "invalid box size");
        out.push(Box {
            kind: bytes[at + 4..at + 8].try_into().unwrap(),
            start: at,
            body: at + header,
            end: at + size,
        });
        at += size;
    }
    out
}
pub fn path(bytes: &[u8], ids: &[[u8; 4]]) -> Vec<Box> {
    let mut range = 0..bytes.len();
    let mut out = Vec::new();
    for id in ids {
        let found: Vec<_> = boxes(bytes, range)
            .into_iter()
            .filter(|b| b.kind == *id)
            .collect();
        assert_eq!(found.len(), 1, "nonunique/missing path");
        let b = found.into_iter().next().unwrap();
        let mut body = b.body;
        if *id == *b"meta" && bytes.get(body..body + 4) == Some(&[0; 4]) {
            body += 4;
        }
        range = body..b.end;
        out.push(b);
    }
    out
}
pub fn items(bytes: &[u8]) -> Vec<Item> {
    let chain = path(bytes, &[*b"moov", *b"udta", *b"meta", *b"ilst"]);
    let ilst = chain.last().unwrap();
    boxes(bytes, ilst.body..ilst.end)
        .into_iter()
        .map(|item| {
            let children = if matches!(&item.kind, b"free" | b"skip") {
                Vec::new()
            } else {
                boxes(bytes, item.body..item.end)
            };
            let identity = if item.kind == *b"----" {
                let text = |kind| {
                    let b = children.iter().find(|b| b.kind == kind).unwrap();
                    assert!(b.body + 4 <= b.end);
                    bytes[b.body + 4..b.end].to_vec()
                };
                Identity::Freeform(text(*b"mean"), text(*b"name"))
            } else {
                Identity::Fourcc(item.kind)
            };
            let values = children
                .iter()
                .filter(|b| b.kind == *b"data")
                .map(|b| {
                    assert!(b.body + 8 <= b.end);
                    Data {
                        version: bytes[b.body],
                        code: u32::from_be_bytes([
                            0,
                            bytes[b.body + 1],
                            bytes[b.body + 2],
                            bytes[b.body + 3],
                        ]),
                        locale: u32::from_be_bytes(
                            bytes[b.body + 4..b.body + 8].try_into().unwrap(),
                        ),
                        payload: bytes[b.body + 8..b.end].to_vec(),
                        raw: bytes[b.start..b.end].to_vec(),
                    }
                })
                .collect();
            Item {
                identity,
                values,
                raw: bytes[item.start..item.end].to_vec(),
            }
        })
        .collect()
}
pub fn media(bytes: &[u8]) -> Vec<Vec<u8>> {
    boxes(bytes, 0..bytes.len())
        .into_iter()
        .filter(|b| b.kind == *b"mdat")
        .map(|b| bytes[b.body..b.end].to_vec())
        .collect()
}
/// Chunk offsets independently decoded from the retained one-track carrier.
pub fn chunk_offsets(bytes: &[u8]) -> Vec<u64> {
    let chain = path(bytes, &[*b"moov", *b"trak", *b"mdia", *b"minf", *b"stbl"]);
    let table = chain.last().unwrap();
    let mut offsets = Vec::new();
    for atom in boxes(bytes, table.body..table.end)
        .into_iter()
        .filter(|a| matches!(&a.kind, b"stco" | b"co64"))
    {
        assert!(atom.body + 8 <= atom.end);
        let count =
            u32::from_be_bytes(bytes[atom.body + 4..atom.body + 8].try_into().unwrap()) as usize;
        let width = if atom.kind == *b"stco" { 4 } else { 8 };
        assert_eq!(atom.end - atom.body, 8 + count * width);
        for value in bytes[atom.body + 8..atom.end].chunks_exact(width) {
            offsets.push(if width == 4 {
                u64::from(u32::from_be_bytes(value.try_into().unwrap()))
            } else {
                u64::from_be_bytes(value.try_into().unwrap())
            });
        }
    }
    let media = boxes(bytes, 0..bytes.len())
        .into_iter()
        .filter(|a| a.kind == *b"mdat")
        .collect::<Vec<_>>();
    assert!(!offsets.is_empty());
    assert!(offsets.iter().all(|&offset| media
        .iter()
        .any(|m| offset >= m.body as u64 && offset < m.end as u64)));
    offsets
}
pub fn boxed(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(body.len() + 8)
        .unwrap()
        .to_be_bytes()
        .to_vec();
    out.extend(kind);
    out.extend(body);
    out
}
pub fn data(code: u32, locale: u32, payload: &[u8]) -> Vec<u8> {
    boxed(
        *b"data",
        &[
            code.to_be_bytes().as_slice(),
            locale.to_be_bytes().as_slice(),
            payload,
        ]
        .concat(),
    )
}
pub fn item(kind: [u8; 4], values: &[Vec<u8>]) -> Vec<u8> {
    boxed(kind, &values.concat())
}
pub fn freeform(mean: &[u8], name: &[u8], values: &[Vec<u8>]) -> Vec<u8> {
    boxed(
        *b"----",
        &[
            boxed(*b"mean", &[&[0; 4][..], mean].concat()),
            boxed(*b"name", &[&[0; 4][..], name].concat()),
            values.concat(),
        ]
        .concat(),
    )
}
/// Retained carrier has tail moov; metadata growth cannot move its media offsets.
pub fn with_items(carrier: &[u8], items: &[Vec<u8>], padding: usize) -> Vec<u8> {
    let chain = path(carrier, &[*b"moov", *b"udta", *b"meta", *b"ilst"]);
    let old = chain.last().unwrap();
    assert!(boxes(carrier, 0..carrier.len())
        .into_iter()
        .filter(|b| b.kind == *b"mdat")
        .all(|b| b.end <= chain[0].start));
    let mut replacement = boxed(*b"ilst", &items.concat());
    if padding > 0 {
        assert!(padding >= 8);
        replacement.extend(boxed(*b"free", &vec![0; padding - 8]));
    }
    let delta = replacement.len() as i64 - (old.end - old.start) as i64;
    let mut out = [&carrier[..old.start], &replacement, &carrier[old.end..]].concat();
    for b in &chain[..chain.len() - 1] {
        let size = (b.end - b.start) as i64 + delta;
        assert_eq!(b.body - b.start, 8);
        out[b.start..b.start + 4].copy_from_slice(&u32::try_from(size).unwrap().to_be_bytes());
    }
    out
}
