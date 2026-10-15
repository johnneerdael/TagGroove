//! MP4 box layout rewrites for Utils → Optimize: free space removal and `moov`
//! placement before the media data. Every chunk offset is relocated, and the
//! result is checked against the original bytes before the staged copy may
//! replace the file.
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
};

type Kind = [u8; 4];

/// Boxes whose payload is a list of boxes. `meta` may carry a version prefix.
const CONTAINERS: [&Kind; 10] = [
    b"moov", b"trak", b"mdia", b"minf", b"stbl", b"udta", b"edts", b"dinf", b"tref", b"meta",
];
/// Unused space. `wide` only reserves room for a 64-bit `mdat` header.
const FREE: [&Kind; 3] = [b"free", b"skip", b"wide"];
/// Fragmented files address media through offsets this rewrite does not know.
const FRAGMENTED: [&Kind; 5] = [b"moof", b"mfra", b"sidx", b"ssix", b"styp"];
const COMPARE: u64 = 16;

fn invalid(what: &str) -> String {
    format!("The MP4 structure cannot be rewritten safely: {what}")
}
fn io_error(error: io::Error) -> String {
    error.to_string()
}
fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes[..4].try_into().expect("four bytes"))
}
fn be64(bytes: &[u8]) -> u64 {
    u64::from_be_bytes(bytes[..8].try_into().expect("eight bytes"))
}

/// One top-level box as found in the file.
#[derive(Clone, Debug, PartialEq)]
struct TopBox {
    kind: Kind,
    start: u64,
    header: u64,
    end: u64,
    /// The size field was 0 ("extends to the end of the file").
    open_ended: bool,
}
impl TopBox {
    fn payload(&self) -> u64 {
        self.start + self.header
    }
}

fn top_level(file: &mut fs::File) -> Result<Vec<TopBox>, String> {
    let len = file.metadata().map_err(io_error)?.len();
    let mut boxes = Vec::new();
    let mut at = 0;
    while at < len {
        if len - at < 8 {
            return Err(invalid("the file ends inside a box header"));
        }
        let mut head = [0; 16];
        file.seek(SeekFrom::Start(at)).map_err(io_error)?;
        file.read_exact(&mut head[..8]).map_err(io_error)?;
        let kind: Kind = head[4..8].try_into().expect("four bytes");
        let (header, size, open_ended) = match be32(&head) {
            0 => (8, len - at, true),
            1 => {
                file.read_exact(&mut head[8..]).map_err(io_error)?;
                (16, be64(&head[8..]), false)
            }
            size => (8, u64::from(size), false),
        };
        if size < header || size > len - at {
            return Err(invalid("a box size does not match the file"));
        }
        boxes.push(TopBox {
            kind,
            start: at,
            header,
            end: at + size,
            open_ended,
        });
        at += size;
    }
    Ok(boxes)
}

#[derive(Clone, Debug, PartialEq)]
struct Node {
    kind: Kind,
    body: Body,
}
#[derive(Clone, Debug, PartialEq)]
enum Body {
    Leaf(Vec<u8>),
    /// `prefix` is a full-box version and flags; `suffix` the zero terminator
    /// QuickTime writes after `udta` children.
    Container {
        prefix: Vec<u8>,
        children: Vec<Node>,
        suffix: Vec<u8>,
    },
}

fn parse_children(bytes: &[u8]) -> Option<(Vec<Node>, Vec<u8>)> {
    let mut children = Vec::new();
    let mut at = 0;
    while bytes.len() - at >= 8 {
        let kind: Kind = bytes[at + 4..at + 8].try_into().ok()?;
        let (header, size) = match be32(&bytes[at..]) {
            0 => (8, bytes.len() - at),
            1 if bytes.len() - at >= 16 => (16, usize::try_from(be64(&bytes[at + 8..])).ok()?),
            1 => return None,
            size => (8, size as usize),
        };
        if size < header || size > bytes.len() - at {
            return None;
        }
        children.push(parse_node(kind, &bytes[at + header..at + size]));
        at += size;
    }
    let suffix = bytes[at..].to_vec();
    suffix.iter().all(|b| *b == 0).then_some((children, suffix))
}

fn parse_node(kind: Kind, payload: &[u8]) -> Node {
    if CONTAINERS.contains(&&kind) {
        // An ISO `meta` is a full box; QuickTime's starts with its `hdlr` child.
        let prefix = if &kind == b"meta" && payload.get(4..8) != Some(b"hdlr") {
            4
        } else {
            0
        };
        if let Some((children, suffix)) = payload.get(prefix..).and_then(parse_children) {
            return Node {
                kind,
                body: Body::Container {
                    prefix: payload[..prefix].to_vec(),
                    children,
                    suffix,
                },
            };
        }
    }
    Node {
        kind,
        body: Body::Leaf(payload.to_vec()),
    }
}

impl Node {
    fn body_len(&self) -> u64 {
        match &self.body {
            Body::Leaf(bytes) => bytes.len() as u64,
            Body::Container {
                prefix,
                children,
                suffix,
            } => {
                (prefix.len() + suffix.len()) as u64 + children.iter().map(Node::size).sum::<u64>()
            }
        }
    }
    fn size(&self) -> u64 {
        let body = self.body_len();
        body + if body + 8 > u64::from(u32::MAX) {
            16
        } else {
            8
        }
    }
    fn write(&self, out: &mut Vec<u8>) {
        let size = self.size();
        if size > u64::from(u32::MAX) {
            out.extend(1u32.to_be_bytes());
            out.extend(self.kind);
            out.extend(size.to_be_bytes());
        } else {
            out.extend((size as u32).to_be_bytes());
            out.extend(self.kind);
        }
        match &self.body {
            Body::Leaf(bytes) => out.extend(bytes),
            Body::Container {
                prefix,
                children,
                suffix,
            } => {
                out.extend(prefix);
                children.iter().for_each(|child| child.write(out));
                out.extend(suffix);
            }
        }
    }
    fn children(&self) -> &[Node] {
        match &self.body {
            Body::Container { children, .. } => children,
            Body::Leaf(_) => &[],
        }
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match &mut self.body {
            Body::Container { children, .. } => Some(children),
            Body::Leaf(_) => None,
        }
    }
    fn any(&self, test: &impl Fn(&Node) -> bool) -> bool {
        test(self) || self.children().iter().any(|c| c.any(test))
    }
    /// Remove free-space boxes below this one; true if any were found.
    fn drop_free(&mut self) -> bool {
        let Some(children) = self.children_mut() else {
            return false;
        };
        let before = children.len();
        children.retain(|c| !FREE.contains(&&c.kind));
        let mut removed = children.len() != before;
        for child in children {
            removed |= child.drop_free();
        }
        removed
    }
    /// Visit every `stco`/`co64` table in document order.
    fn chunk_tables(
        &mut self,
        visit: &mut impl FnMut(&mut Vec<u8>, bool) -> Result<(), String>,
    ) -> Result<(), String> {
        match &mut self.body {
            Body::Leaf(bytes) if &self.kind == b"stco" || &self.kind == b"co64" => {
                visit(bytes, &self.kind == b"co64")
            }
            Body::Leaf(_) => Ok(()),
            Body::Container { children, .. } => {
                for child in children {
                    child.chunk_tables(visit)?;
                }
                Ok(())
            }
        }
    }
}

/// Every chunk offset of a `stco` (32-bit) or `co64` (64-bit) payload.
fn table_offsets(bytes: &[u8], wide: bool) -> Result<Vec<u64>, String> {
    let width = if wide { 8 } else { 4 };
    let count = bytes
        .get(4..8)
        .map(be32)
        .ok_or_else(|| invalid("a chunk offset table is truncated"))? as usize;
    if bytes.len() != 8 + count * width {
        return Err(invalid("a chunk offset table has the wrong length"));
    }
    Ok(bytes[8..]
        .chunks_exact(width)
        .map(|v| if wide { be64(v) } else { u64::from(be32(v)) })
        .collect())
}

fn all_offsets(moov: &Node) -> Result<Vec<u64>, String> {
    let mut copy = moov.clone();
    let mut offsets = Vec::new();
    copy.chunk_tables(&mut |bytes, wide| {
        offsets.extend(table_offsets(bytes, wide)?);
        Ok(())
    })?;
    Ok(offsets)
}

fn read_moov(file: &mut fs::File, moov: &TopBox) -> Result<Node, String> {
    let mut payload = vec![0; (moov.end - moov.payload()) as usize];
    file.seek(SeekFrom::Start(moov.payload()))
        .map_err(io_error)?;
    file.read_exact(&mut payload).map_err(io_error)?;
    let node = parse_node(*b"moov", &payload);
    if matches!(node.body, Body::Leaf(_)) {
        return Err(invalid("the movie box could not be read"));
    }
    Ok(node)
}

/// A file's top-level boxes and its parsed `moov`.
#[derive(Debug)]
pub(crate) struct Layout {
    top: Vec<TopBox>,
    moov: usize,
    movie: Node,
}

pub(crate) fn read_layout(file: &mut fs::File) -> Result<Layout, String> {
    let top = top_level(file)?;
    if top.iter().any(|b| FRAGMENTED.contains(&&b.kind)) {
        return Err("Fragmented MP4 files cannot be rewritten".into());
    }
    if top.iter().any(|b| &b.kind == b"meta") {
        return Err(invalid("a top-level meta box may hold item offsets"));
    }
    let mut movies = top.iter().enumerate().filter(|(_, b)| &b.kind == b"moov");
    let moov = match (movies.next(), movies.next()) {
        (Some((index, _)), None) => index,
        _ => return Err(invalid("the file needs exactly one movie box")),
    };
    let movie = read_moov(file, &top[moov])?;
    if movie.any(&|n| &n.kind == b"iloc") {
        return Err(invalid("item location boxes are not supported"));
    }
    Ok(Layout { top, moov, movie })
}

impl Layout {
    fn first_media(&self) -> Option<usize> {
        self.top.iter().position(|b| &b.kind == b"mdat")
    }
    /// Whether `moov` precedes the media and no free space is left anywhere.
    pub(crate) fn optimized(&self) -> bool {
        let mut movie = self.movie.clone();
        self.first_media().is_none_or(|media| self.moov < media)
            && !self.top.iter().any(|b| FREE.contains(&&b.kind))
            && !movie.drop_free()
    }
}

/// Remove all free space and put `moov` before the media data.
pub(crate) fn optimize(layout: &mut Layout) {
    layout.movie.drop_free();
}

enum Part {
    Movie,
    Copy {
        source: TopBox,
        header: Vec<u8>,
    },
    /// New boxes placed directly after the movie box.
    Data(Vec<u8>),
}

/// Boxes added after `moov` and top-level boxes left out by [`write_with`].
/// Chunk offsets from `source length + n` address byte `n` of `boxes`.
struct Extra {
    boxes: Vec<u8>,
    drop: Vec<usize>,
}

/// Write `layout` into `target`. With `reorder`, free space is dropped and
/// `moov` moves before the first `mdat`. Chunk offsets are relocated and the
/// output is verified against `source`.
pub(crate) fn write(
    source: &mut fs::File,
    layout: Layout,
    reorder: bool,
    target: &mut fs::File,
) -> Result<(), String> {
    write_with(source, layout, reorder, target, None)
}

fn write_with(
    source: &mut fs::File,
    mut layout: Layout,
    reorder: bool,
    target: &mut fs::File,
    extra: Option<Extra>,
) -> Result<(), String> {
    let source_len = source.metadata().map_err(io_error)?.len();
    let (data, drop) = extra.map_or((None, Vec::new()), |e| (Some(e.boxes), e.drop));
    let mut parts: Vec<Part> = Vec::new();
    for (index, top) in layout.top.iter().enumerate() {
        if index == layout.moov {
            parts.push(Part::Movie);
            if let Some(data) = &data {
                parts.push(Part::Data(data.clone()));
            }
        } else if drop.contains(&index) {
            continue;
        } else if !(reorder && FREE.contains(&&top.kind)) {
            let header = if top.open_ended {
                let size = top.end - top.payload();
                let mut header = Vec::new();
                if size + 8 > u64::from(u32::MAX) {
                    header.extend(1u32.to_be_bytes());
                    header.extend(top.kind);
                    header.extend((size + 16).to_be_bytes());
                } else {
                    header.extend(((size + 8) as u32).to_be_bytes());
                    header.extend(top.kind);
                }
                header
            } else {
                let mut header = vec![0; top.header as usize];
                source.seek(SeekFrom::Start(top.start)).map_err(io_error)?;
                source.read_exact(&mut header).map_err(io_error)?;
                header
            };
            parts.push(Part::Copy {
                source: top.clone(),
                header,
            });
        }
    }
    if reorder {
        let movie = parts.iter().position(|p| matches!(p, Part::Movie));
        let media = parts
            .iter()
            .position(|p| matches!(p, Part::Copy { source, .. } if &source.kind == b"mdat"));
        if let (Some(movie), Some(media)) = (movie, media) {
            if movie > media {
                let part = parts.remove(movie);
                parts.insert(media, part);
            }
        }
    }
    // Box sizes are fixed now: chunk offsets keep their width.
    let mut at = 0;
    let mut moved: Vec<(u64, u64, u64)> = Vec::new();
    for part in &parts {
        match part {
            Part::Movie => at += layout.movie.size(),
            Part::Copy { source, header } => {
                let payload = source.end - source.payload();
                moved.push((source.payload(), source.end, at + header.len() as u64));
                at += header.len() as u64 + payload;
            }
            Part::Data(bytes) => {
                moved.push((source_len, source_len + bytes.len() as u64, at));
                at += bytes.len() as u64;
            }
        }
    }
    let relocate = |offset: u64| {
        moved
            .iter()
            .find(|(start, end, _)| (*start..*end).contains(&offset))
            .map(|(start, _, new)| new + (offset - start))
            .ok_or_else(|| invalid("a chunk offset points outside the media data"))
    };
    let mut pairs = Vec::new();
    layout.movie.chunk_tables(&mut |bytes, wide| {
        let old = table_offsets(bytes, wide)?;
        let width = if wide { 8 } else { 4 };
        for (index, offset) in old.into_iter().enumerate() {
            let new = relocate(offset)?;
            let slot = &mut bytes[8 + index * width..8 + (index + 1) * width];
            if wide {
                slot.copy_from_slice(&new.to_be_bytes());
            } else {
                let new32 = u32::try_from(new)
                    .map_err(|_| invalid("a 32-bit chunk offset would overflow"))?;
                slot.copy_from_slice(&new32.to_be_bytes());
            }
            pairs.push((offset, new));
        }
        Ok(())
    })?;
    let mut movie = Vec::new();
    layout.movie.write(&mut movie);
    target.set_len(0).map_err(io_error)?;
    target.seek(SeekFrom::Start(0)).map_err(io_error)?;
    {
        let mut out = io::BufWriter::new(&mut *target);
        for part in &parts {
            match part {
                Part::Movie => out.write_all(&movie).map_err(io_error)?,
                Part::Data(bytes) => out.write_all(bytes).map_err(io_error)?,
                Part::Copy {
                    source: top,
                    header,
                } => {
                    out.write_all(header).map_err(io_error)?;
                    source
                        .seek(SeekFrom::Start(top.payload()))
                        .map_err(io_error)?;
                    let length = top.end - top.payload();
                    let copied =
                        io::copy(&mut (&mut *source).take(length), &mut out).map_err(io_error)?;
                    if copied != length {
                        return Err(invalid("the media data ended early"));
                    }
                }
            }
        }
        out.flush().map_err(io_error)?;
    }
    verify(source, target, &parts, &pairs, at, reorder, data.as_deref())
}

fn same_bytes(
    a: &mut fs::File,
    a_at: u64,
    b: &mut fs::File,
    b_at: u64,
    length: u64,
) -> Result<bool, String> {
    let (mut left, mut right) = (vec![0; 1 << 16], vec![0; 1 << 16]);
    a.seek(SeekFrom::Start(a_at)).map_err(io_error)?;
    b.seek(SeekFrom::Start(b_at)).map_err(io_error)?;
    let mut remaining = length;
    while remaining > 0 {
        let n = remaining.min(left.len() as u64) as usize;
        a.read_exact(&mut left[..n]).map_err(io_error)?;
        b.read_exact(&mut right[..n]).map_err(io_error)?;
        if left[..n] != right[..n] {
            return Ok(false);
        }
        remaining -= n as u64;
    }
    Ok(true)
}

/// Re-read the output: the expected box order, identical media payloads and
/// chunk offsets that address the same bytes as before.
fn verify(
    source: &mut fs::File,
    target: &mut fs::File,
    parts: &[Part],
    pairs: &[(u64, u64)],
    length: u64,
    reorder: bool,
    data: Option<&[u8]>,
) -> Result<(), String> {
    let failed = || "The rewritten MP4 file did not verify; the original file was kept".to_string();
    let source_len = source.metadata().map_err(io_error)?.len();
    if target.metadata().map_err(io_error)?.len() != length {
        return Err(failed());
    }
    let written = read_layout(target).map_err(|_| failed())?;
    let boxes = |bytes: &[u8]| parse_children(bytes).map_or(usize::MAX, |(c, _)| c.len());
    let expected: usize = parts
        .iter()
        .map(|part| match part {
            Part::Data(bytes) => boxes(bytes),
            _ => 1,
        })
        .sum();
    if written.top.len() != expected {
        return Err(failed());
    }
    if reorder && !written.optimized() {
        return Err(failed());
    }
    let mut tops = written.top.iter();
    for part in parts {
        let top = match part {
            Part::Data(bytes) => {
                for _ in 0..boxes(bytes) {
                    tops.next();
                }
                continue;
            }
            _ => tops.next().ok_or_else(failed)?,
        };
        if let Part::Copy {
            source: original, ..
        } = part
        {
            let length = original.end - original.payload();
            if top.kind != original.kind
                || top.end - top.payload() != length
                || !same_bytes(source, original.payload(), target, top.payload(), length)?
            {
                return Err(failed());
            }
        }
    }
    let offsets = all_offsets(&written.movie)?;
    if offsets != pairs.iter().map(|(_, new)| *new).collect::<Vec<_>>() {
        return Err(failed());
    }
    for (old, new) in pairs {
        if let Some(at) = old.checked_sub(source_len) {
            // An offset into the added boxes.
            let data = data.ok_or_else(failed)?;
            let expected = data.get(at as usize..).ok_or_else(failed)?;
            let span = expected.len().min(COMPARE as usize);
            let mut actual = vec![0; span];
            target.seek(SeekFrom::Start(*new)).map_err(io_error)?;
            target.read_exact(&mut actual).map_err(io_error)?;
            if actual != expected[..span] {
                return Err(failed());
            }
            continue;
        }
        let span = COMPARE.min(source_len - old).min(length - new);
        if !same_bytes(source, *old, target, *new, span)? {
            return Err(failed());
        }
    }
    Ok(())
}

// Chapter writing: a Nero `chpl` box in `moov/udta` and a QuickTime chapter
// text track referenced by `tref/chap` from the audio and video tracks, laid
// out as ffmpeg writes them. Earlier chapter boxes and tracks are replaced;
// text samples of a replaced track stay unreferenced inside its media data.

/// QuickTime text sample description (display flags, justification, colors,
/// text box and font table) as ffmpeg writes it for chapter tracks.
const TEXT_SAMPLE_ENTRY: [u8; 59] = [
    0, 0, 0, 0x3b, b't', b'e', b'x', b't', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xd, b'f', b't', b'a', b'b',
    0, 1, 0, 1, 0,
];
/// QuickTime base media header (`gmhd` with `gmin` and the text matrix).
const TEXT_MEDIA_HEADER: [u8; 76] = [
    0, 0, 0, 0x4c, b'g', b'm', b'h', b'd', 0, 0, 0, 0x18, b'g', b'm', b'i', b'n', 0, 0, 0, 0, 0,
    0x40, 0x80, 0, 0x80, 0, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0x2c, b't', b'e', b'x', b't', 0, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0,
    0,
];
/// Text encoding atom marking each sample as UTF-8.
const ENCD: [u8; 12] = [0, 0, 0, 12, b'e', b'n', b'c', b'd', 0, 0, 1, 0];
const MATRIX: [u8; 36] = [
    0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x40, 0, 0, 0,
];
/// Free space left after chapter data that had to move the media.
const CHAPTER_PADDING: u64 = 1024;
/// Nero chapter starts are in 100 ns units.
const NERO_UNITS: u128 = 10_000_000;

fn boxed(kind: &Kind, body: &[u8]) -> Vec<u8> {
    let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend(kind);
    out.extend(body);
    out
}
fn find<'a>(node: &'a Node, kind: &Kind) -> Option<&'a Node> {
    node.children().iter().find(|c| &c.kind == kind)
}
fn leaf(node: &Node) -> Option<&[u8]> {
    match &node.body {
        Body::Leaf(bytes) => Some(bytes),
        Body::Container { .. } => None,
    }
}
fn track_id(trak: &Node) -> Option<u32> {
    let tkhd = leaf(find(trak, b"tkhd")?)?;
    let at = if tkhd.first() == Some(&1) { 20 } else { 12 };
    tkhd.get(at..at + 4).map(be32)
}
fn handler(trak: &Node) -> Option<Kind> {
    let hdlr = leaf(find(find(trak, b"mdia")?, b"hdlr")?)?;
    hdlr.get(8..12)?.try_into().ok()
}
fn chapter_refs(trak: &Node) -> Vec<u32> {
    find(trak, b"tref")
        .map(|tref| {
            tref.children()
                .iter()
                .filter(|c| &c.kind == b"chap")
                .filter_map(leaf)
                .flat_map(|ids| ids.as_chunks::<4>().0.iter().map(|id| be32(id)))
                .collect()
        })
        .unwrap_or_default()
}
fn node_offsets(node: &Node) -> Result<Vec<u64>, String> {
    all_offsets(node)
}

/// A full box (version 0 or 1) with `fields` after its version and flags.
fn full_box(kind: &Kind, version: u8, flags: u32, fields: &[u8]) -> Vec<u8> {
    let mut body = vec![version];
    body.extend(&flags.to_be_bytes()[1..]);
    body.extend(fields);
    boxed(kind, &body)
}

/// The chapter text track. `samples` are (duration, size) in the movie time
/// scale; their bytes form one chunk at `offset`.
fn chapter_trak(
    id: u32,
    timescale: u32,
    duration: u64,
    samples: &[(u64, u32)],
    offset: u64,
    wide: bool,
) -> Node {
    let long = duration > u64::from(u32::MAX);
    let version = u8::from(long);
    let time = |value: u64| {
        if long {
            value.to_be_bytes().to_vec()
        } else {
            (value as u32).to_be_bytes().to_vec()
        }
    };
    // Creation and modification times are left at zero, as ffmpeg does.
    let mut tkhd = time(0);
    tkhd.extend(time(0));
    tkhd.extend(id.to_be_bytes());
    tkhd.extend([0; 4]);
    tkhd.extend(time(duration));
    tkhd.extend([0; 16]);
    tkhd.extend(MATRIX);
    tkhd.extend([0; 8]);
    let mut elst = 1u32.to_be_bytes().to_vec();
    elst.extend(time(duration));
    elst.extend(time(0));
    elst.extend(0x0001_0000u32.to_be_bytes());
    let mut mdhd = time(0);
    mdhd.extend(time(0));
    mdhd.extend(timescale.to_be_bytes());
    mdhd.extend(time(duration));
    mdhd.extend([0x55, 0xC4, 0, 0]);
    let mut hdlr = vec![0; 4];
    hdlr.extend(b"text");
    hdlr.extend([0; 12]);
    hdlr.extend(b"Chapters\0");
    let mut stsd = 1u32.to_be_bytes().to_vec();
    stsd.extend(TEXT_SAMPLE_ENTRY);
    let mut stts = Vec::new();
    let mut runs: Vec<(u32, u64)> = Vec::new();
    for (delta, _) in samples {
        match runs.last_mut() {
            Some((count, last)) if *last == *delta => *count += 1,
            _ => runs.push((1, *delta)),
        }
    }
    stts.extend((runs.len() as u32).to_be_bytes());
    for (count, delta) in runs {
        stts.extend(count.to_be_bytes());
        stts.extend((delta.min(u64::from(u32::MAX)) as u32).to_be_bytes());
    }
    let mut stsc = 1u32.to_be_bytes().to_vec();
    stsc.extend(1u32.to_be_bytes());
    stsc.extend((samples.len() as u32).to_be_bytes());
    stsc.extend(1u32.to_be_bytes());
    let mut stsz = 0u32.to_be_bytes().to_vec();
    stsz.extend((samples.len() as u32).to_be_bytes());
    samples
        .iter()
        .for_each(|(_, size)| stsz.extend(size.to_be_bytes()));
    let mut chunks = 1u32.to_be_bytes().to_vec();
    if wide {
        chunks.extend(offset.to_be_bytes());
    } else {
        chunks.extend((offset as u32).to_be_bytes());
    }
    let stbl = [
        full_box(b"stsd", 0, 0, &stsd),
        full_box(b"stts", 0, 0, &stts),
        full_box(b"stsc", 0, 0, &stsc),
        full_box(b"stsz", 0, 0, &stsz),
        full_box(if wide { b"co64" } else { b"stco" }, 0, 0, &chunks),
    ]
    .concat();
    let dref = full_box(
        b"dref",
        0,
        0,
        &[&1u32.to_be_bytes()[..], &full_box(b"url ", 0, 1, &[])].concat(),
    );
    let minf = [
        TEXT_MEDIA_HEADER.to_vec(),
        boxed(b"dinf", &dref),
        boxed(b"stbl", &stbl),
    ]
    .concat();
    let mdia = [
        full_box(b"mdhd", version, 0, &mdhd),
        full_box(b"hdlr", 0, 0, &hdlr),
        boxed(b"minf", &minf),
    ]
    .concat();
    let trak = [
        full_box(b"tkhd", version, 2, &tkhd),
        boxed(b"edts", &full_box(b"elst", version, 0, &elst)),
        boxed(b"mdia", &mdia),
    ]
    .concat();
    parse_node(*b"trak", &trak)
}

/// Nero `chpl`: version 1, four reserved bytes, a count and per chapter a
/// 64-bit start in 100 ns units and a length-prefixed UTF-8 title.
fn chpl(chapters: &[(u64, String)]) -> Node {
    let mut body = vec![1, 0, 0, 0, 0, 0, 0, 0, chapters.len() as u8];
    for (start, title) in chapters {
        body.extend(start.to_be_bytes());
        body.push(title.len() as u8);
        body.extend(title.as_bytes());
    }
    Node {
        kind: *b"chpl",
        body: Body::Leaf(body),
    }
}

/// Text samples: a 16-bit length, the UTF-8 title and an `encd` atom.
fn text_samples(chapters: &[(u64, String)]) -> (Vec<u8>, Vec<u32>) {
    let mut data = Vec::new();
    let mut sizes = Vec::new();
    for (_, title) in chapters {
        let before = data.len();
        data.extend((title.len() as u16).to_be_bytes());
        data.extend(title.as_bytes());
        data.extend(ENCD);
        sizes.push((data.len() - before) as u32);
    }
    (data, sizes)
}

/// Replace the chapters of the MP4 file open as `file` (a staged copy at
/// `staged_path`) by `chapters` (start in 100 ns units, title), at least
/// one. The movie box and the chapter data are rewritten in place when they
/// end the file or fit the free space after `moov`; otherwise the file is
/// rewritten with every chunk offset relocated and verified.
pub(crate) fn write_chapters(
    file: &mut fs::File,
    staged_path: &std::path::Path,
    chapters: &[(u64, String)],
) -> Result<(), String> {
    if chapters.is_empty() || chapters.len() > 255 {
        return Err("MP4 files hold between 1 and 255 Nero chapters".into());
    }
    if chapters.iter().any(|(_, title)| title.len() > 255) {
        return Err("MP4 chapter titles are limited to 255 bytes".into());
    }
    if chapters.windows(2).any(|w| w[1].0 <= w[0].0) {
        return Err("MP4 chapters must start at different, increasing times".into());
    }
    let layout = read_layout(file)?;
    let file_len = file.metadata().map_err(io_error)?.len();
    let header = leaf(find(&layout.movie, b"mvhd").ok_or_else(|| invalid("no movie header"))?)
        .ok_or_else(|| invalid("no movie header"))?
        .to_vec();
    let (timescale, duration) = match header.first() {
        Some(1) if header.len() >= 32 => (be32(&header[20..]), be64(&header[24..])),
        Some(0) if header.len() >= 20 => (be32(&header[12..]), u64::from(be32(&header[16..]))),
        _ => return Err(invalid("the movie header is malformed")),
    };
    if timescale == 0 || header.len() < 4 {
        return Err(invalid("the movie time scale is zero"));
    }
    let traks: Vec<&Node> = layout
        .movie
        .children()
        .iter()
        .filter(|c| &c.kind == b"trak")
        .collect();
    let referenced: Vec<u32> = traks.iter().flat_map(|t| chapter_refs(t)).collect();
    let mut kept = Vec::new();
    let mut removed = Vec::new();
    let mut max_id = 0;
    for trak in &traks {
        let id = track_id(trak).ok_or_else(|| invalid("a track has no header"))?;
        max_id = max_id.max(id);
        if referenced.contains(&id) {
            removed.extend(node_offsets(trak)?);
        } else {
            kept.extend(node_offsets(trak)?);
        }
    }
    let next_id = be32(&header[header.len() - 4..]).max(max_id + 1);
    if next_id == u32::MAX {
        return Err(invalid("no track ID is left for the chapter track"));
    }

    // The movie without the old chapters, with the new Nero box and references.
    let mut base = layout.movie.clone();
    let children = base
        .children_mut()
        .ok_or_else(|| invalid("the movie box could not be read"))?;
    children.retain(|c| {
        !(&c.kind == b"trak" && track_id(c).is_some_and(|id| referenced.contains(&id)))
    });
    let mut referencing = 0;
    for trak in children.iter_mut().filter(|c| &c.kind == b"trak") {
        let media = matches!(handler(trak).as_ref(), Some(b"soun") | Some(b"vide"));
        let Some(parts) = trak.children_mut() else {
            continue;
        };
        for tref in parts.iter_mut().filter(|c| &c.kind == b"tref") {
            if let Some(refs) = tref.children_mut() {
                refs.retain(|c| &c.kind != b"chap");
            }
        }
        parts.retain(|c| {
            !(&c.kind == b"tref"
                && matches!(&c.body, Body::Container { children, .. } if children.is_empty()))
        });
        if !media {
            continue;
        }
        referencing += 1;
        let chap = Node {
            kind: *b"chap",
            body: Body::Leaf(next_id.to_be_bytes().to_vec()),
        };
        match parts.iter_mut().find(|c| &c.kind == b"tref") {
            Some(tref) => tref
                .children_mut()
                .ok_or_else(|| invalid("a track reference box is malformed"))?
                .push(chap),
            None => {
                let at = parts
                    .iter()
                    .position(|c| &c.kind == b"mdia")
                    .unwrap_or(parts.len());
                parts.insert(
                    at,
                    Node {
                        kind: *b"tref",
                        body: Body::Container {
                            prefix: Vec::new(),
                            children: vec![chap],
                            suffix: Vec::new(),
                        },
                    },
                );
            }
        }
    }
    if referencing == 0 {
        return Err("The MP4 file has no audio or video track for chapters".into());
    }
    for node in children.iter_mut() {
        if &node.kind == b"mvhd" {
            if let Body::Leaf(bytes) = &mut node.body {
                let at = bytes.len() - 4;
                bytes[at..].copy_from_slice(&(next_id + 1).to_be_bytes());
            }
        }
        if &node.kind == b"udta" {
            if let Some(items) = node.children_mut() {
                items.retain(|c| &c.kind != b"chpl");
            }
        }
    }
    let nero = chpl(chapters);
    match children.iter_mut().find(|c| &c.kind == b"udta") {
        Some(udta) => udta
            .children_mut()
            .ok_or_else(|| invalid("the user data box is malformed"))?
            .push(nero),
        None => children.push(Node {
            kind: *b"udta",
            body: Body::Container {
                prefix: Vec::new(),
                children: vec![nero],
                suffix: Vec::new(),
            },
        }),
    }
    let trak_at = children
        .iter()
        .rposition(|c| &c.kind == b"trak")
        .map_or(children.len(), |i| i + 1);

    // Samples in the movie time scale; each title lasts until the next one.
    let ticks = |units: u64| (u128::from(units) * u128::from(timescale) / NERO_UNITS) as u64;
    let last = ticks(chapters[chapters.len() - 1].0);
    let total = duration.max(last + u64::from(timescale));
    let (data, sizes) = text_samples(chapters);
    let samples: Vec<(u64, u32)> = (0..chapters.len())
        .map(|i| {
            let from = if i == 0 { 0 } else { ticks(chapters[i].0) };
            let to = chapters.get(i + 1).map_or(total, |next| ticks(next.0));
            (to.saturating_sub(from).max(1), sizes[i])
        })
        .collect();
    let movie = |offset: u64, wide: bool| {
        let mut movie = base.clone();
        let trak = chapter_trak(next_id, timescale, total, &samples, offset, wide);
        if let Some(children) = movie.children_mut() {
            children.insert(trak_at, trak);
        }
        movie
    };
    let mdat = |padding: u64| {
        let mut out = boxed(b"mdat", &data);
        let size = out.len() as u64 + padding;
        out[..4].copy_from_slice(&(size as u32).to_be_bytes());
        out.resize(size as usize, 0);
        out
    };

    // In place: `moov` and everything after it is free space or old chapter
    // data, or the free space after `moov` holds the new boxes.
    let moov = layout.top[layout.moov].clone();
    let chapter_only = |top: &TopBox| {
        &top.kind == b"mdat"
            && removed.iter().any(|o| (top.payload()..top.end).contains(o))
            && !kept.iter().any(|o| (top.payload()..top.end).contains(o))
    };
    let droppable = |top: &TopBox| FREE.contains(&&top.kind) || chapter_only(top);
    let after = &layout.top[layout.moov + 1..];
    let blocker = after.iter().find(|top| !droppable(top));
    let region_end = blocker.map_or(file_len, |top| top.start);
    let tail = blocker.is_none() && kept.iter().all(|o| *o < moov.start);
    let in_region = kept.iter().any(|o| (moov.start..region_end).contains(o));
    let sized = |wide: bool| movie(0, wide).size();
    let place = |wide: bool| moov.start + sized(wide) + 8;
    let wide = place(false) > u64::from(u32::MAX);
    let need = sized(wide) + 8 + data.len() as u64;
    if !in_region && (tail || need <= region_end - moov.start) {
        let offset = place(wide);
        let mut bytes = Vec::new();
        movie(offset, wide).write(&mut bytes);
        let spare = if tail {
            0
        } else {
            region_end - moov.start - need
        };
        if spare >= 8 {
            bytes.extend(mdat(0));
            bytes.extend(boxed(b"free", &vec![0; (spare - 8) as usize]));
        } else {
            bytes.extend(mdat(spare));
        }
        file.seek(SeekFrom::Start(moov.start)).map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        if tail {
            file.set_len(moov.start + bytes.len() as u64)
                .map_err(io_error)?;
        }
        crate::write_cost::rewrote_region(bytes.len() as u64);
        file.sync_all().map_err(io_error)?;
        // Every other chunk offset is unchanged and the samples are in place.
        let written = read_layout(file).map_err(|_| invalid("the rewritten movie box"))?;
        let mut expected = kept.clone();
        expected.push(offset);
        let mut sample = vec![0; data.len()];
        file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        file.read_exact(&mut sample).map_err(io_error)?;
        if all_offsets(&written.movie)? != expected || sample != data {
            return Err("The MP4 chapters did not verify; the original file was kept".into());
        }
        return Ok(());
    }

    // Rewrite: the chapter data follows `moov`, with room for later edits.
    crate::write_cost::rewrote_all();
    let directory = staged_path
        .parent()
        .ok_or_else(|| invalid("the staged file has no folder"))?;
    let copy = crate::tags::staging_copy(staged_path, directory, ".opentag-chapters-", "")?;
    let mut source = copy.reopen().map_err(io_error)?;
    let wide = file_len + need + 2 * CHAPTER_PADDING > u64::from(u32::MAX);
    let mut boxes = mdat(0);
    boxes.extend(boxed(b"free", &vec![0; CHAPTER_PADDING as usize]));
    let drop = layout
        .top
        .iter()
        .enumerate()
        .filter(|(_, top)| chapter_only(top))
        .map(|(index, _)| index)
        .collect();
    let rewritten = Layout {
        top: layout.top.clone(),
        moov: layout.moov,
        movie: movie(file_len + 8, wide),
    };
    write_with(
        &mut source,
        rewritten,
        false,
        file,
        Some(Extra { boxes, drop }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend(kind);
        out.extend(body);
        out
    }
    fn stco(offsets: &[u32]) -> Vec<u8> {
        let mut body = vec![0, 0, 0, 0];
        body.extend((offsets.len() as u32).to_be_bytes());
        offsets.iter().for_each(|o| body.extend(o.to_be_bytes()));
        boxed(b"stco", &body)
    }
    fn tkhd(id: u32) -> Vec<u8> {
        let mut body = vec![0; 84];
        body[12..16].copy_from_slice(&id.to_be_bytes());
        boxed(b"tkhd", &body)
    }
    fn hdlr(kind: &[u8; 4]) -> Vec<u8> {
        let mut body = vec![0; 24];
        body[8..12].copy_from_slice(kind);
        boxed(b"hdlr", &body)
    }
    fn trak(id: u32, handler: &[u8; 4], offsets: &[u32], tref: Option<u32>) -> Vec<u8> {
        let stbl = boxed(b"stbl", &[stco(offsets), boxed(b"free", &[0; 5])].concat());
        let mdia = boxed(b"mdia", &[hdlr(handler), boxed(b"minf", &stbl)].concat());
        let mut body = tkhd(id);
        if let Some(chapters) = tref {
            body.extend(boxed(b"tref", &boxed(b"chap", &chapters.to_be_bytes())));
        }
        body.extend(mdia);
        boxed(b"trak", &body)
    }
    /// ftyp, free, mdat (two chunks per track), moov at the end.
    fn sample(dir: &Path) -> (std::path::PathBuf, Vec<u8>) {
        let ftyp = boxed(b"ftyp", b"M4A \0\0\0\0M4A mp42isom");
        let free = boxed(b"free", &[0; 40]);
        let media: Vec<u8> = (0..400u32).map(|i| (i * 7 % 251) as u8).collect();
        let base = (ftyp.len() + free.len() + 8) as u32;
        let udta = boxed(
            b"udta",
            &[
                boxed(b"chpl", &[1, 0, 0, 0, 0]),
                boxed(
                    b"meta",
                    &[
                        vec![0; 4],
                        hdlr(b"mdir"),
                        boxed(b"ilst", &[]),
                        boxed(b"free", &[0; 9]),
                    ]
                    .concat(),
                ),
            ]
            .concat(),
        );
        let moov = boxed(
            b"moov",
            &[
                boxed(b"mvhd", &[0; 100]),
                trak(1, b"soun", &[base, base + 100], Some(2)),
                trak(2, b"text", &[base + 300], None),
                udta,
            ]
            .concat(),
        );
        let file = [ftyp, free, boxed(b"mdat", &media), moov].concat();
        let path = dir.join("sample.m4a");
        fs::write(&path, &file).unwrap();
        (path, media)
    }
    use std::path::Path;

    fn rewrite(path: &Path, change: impl FnOnce(&mut Layout) -> bool, reorder: bool) -> Vec<u8> {
        let mut source = fs::File::open(path).unwrap();
        let mut layout = read_layout(&mut source).unwrap();
        assert!(change(&mut layout));
        let out = path.with_extension("out");
        let mut target = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&out)
            .unwrap();
        write(&mut source, layout, reorder, &mut target).unwrap();
        fs::read(out).unwrap()
    }

    #[test]
    fn optimize_moves_moov_drops_free_space_and_relocates_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let (path, media) = sample(dir.path());
        let before = fs::read(&path).unwrap();
        let mut source = fs::File::open(&path).unwrap();
        assert!(!read_layout(&mut source).unwrap().optimized());
        let after = rewrite(
            &path,
            |layout| {
                optimize(layout);
                true
            },
            true,
        );
        let mut file = fs::File::open(path.with_extension("out")).unwrap();
        let layout = read_layout(&mut file).unwrap();
        let kinds: Vec<_> = layout.top.iter().map(|b| b.kind).collect();
        assert_eq!(kinds, [*b"ftyp", *b"moov", *b"mdat"]);
        assert!(layout.optimized());
        assert!(
            !layout.movie.any(&|n| &n.kind == b"free"),
            "nested free boxes are gone"
        );
        let offsets = all_offsets(&layout.movie).unwrap();
        let mdat = layout.top[2].payload() as usize;
        assert_eq!(offsets, [mdat as u64, mdat as u64 + 100, mdat as u64 + 300]);
        assert_eq!(&after[mdat..mdat + media.len()], media.as_slice());
        let old_base = before.windows(4).position(|w| w == b"mdat").unwrap() + 4;
        assert_eq!(after[offsets[2] as usize], before[old_base + 300]);
        assert!(after.len() < before.len());
    }

    #[test]
    fn fragmented_and_malformed_files_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.mp4");
        fs::write(
            &path,
            [
                boxed(b"ftyp", b"iso5"),
                boxed(b"moov", &[]),
                boxed(b"moof", &[]),
            ]
            .concat(),
        )
        .unwrap();
        assert!(read_layout(&mut fs::File::open(&path).unwrap())
            .unwrap_err()
            .contains("Fragmented"));
        let mut bad = boxed(b"ftyp", b"iso5");
        bad.extend([0, 0, 1, 0, b'm', b'd', b'a', b't']);
        fs::write(&path, bad).unwrap();
        assert!(read_layout(&mut fs::File::open(&path).unwrap()).is_err());
        // A chunk offset into free space cannot be relocated.
        let ftyp = boxed(b"ftyp", b"M4A ");
        let moov_len = boxed(b"moov", &trak(1, b"soun", &[0], None)).len();
        let into_free = (ftyp.len() + moov_len + 8 + 2) as u32;
        let file = [
            ftyp,
            boxed(b"moov", &trak(1, b"soun", &[into_free], None)),
            boxed(b"free", &[0; 8]),
            boxed(b"mdat", &[1; 8]),
        ]
        .concat();
        fs::write(&path, &file).unwrap();
        let mut source = fs::File::open(&path).unwrap();
        let mut layout = read_layout(&mut source).unwrap();
        optimize(&mut layout);
        let mut target = tempfile::tempfile().unwrap();
        assert!(write(&mut source, layout, true, &mut target)
            .unwrap_err()
            .contains("outside the media data"));
    }

    /// ftyp and a one-track movie (1000 ticks per second, two seconds) with
    /// its media before (`tail`) or after the movie box.
    fn movie_file(dir: &Path, tail: bool) -> (std::path::PathBuf, Vec<u8>) {
        let ftyp = boxed(b"ftyp", b"M4A \0\0\0\0M4A mp42isom");
        let media: Vec<u8> = (0..300u32).map(|i| (i * 13 % 251) as u8).collect();
        let mut mvhd = vec![0; 100];
        mvhd[12..16].copy_from_slice(&1000u32.to_be_bytes());
        mvhd[16..20].copy_from_slice(&2000u32.to_be_bytes());
        mvhd[96..100].copy_from_slice(&2u32.to_be_bytes());
        let moov = |base: u32| {
            boxed(
                b"moov",
                &[
                    boxed(b"mvhd", &mvhd),
                    trak(1, b"soun", &[base, base + 100], None),
                ]
                .concat(),
            )
        };
        let size = moov(0).len() as u32;
        let file = if tail {
            let base = (ftyp.len() + 8) as u32;
            [ftyp, boxed(b"mdat", &media), moov(base)].concat()
        } else {
            let base = ftyp.len() as u32 + size + 8;
            [ftyp, moov(base), boxed(b"mdat", &media)].concat()
        };
        let path = dir.join(if tail { "tail.m4a" } else { "fast.m4a" });
        fs::write(&path, &file).unwrap();
        (path, media)
    }

    fn chapters(list: &[(u64, &str)]) -> Vec<(u64, String)> {
        list.iter().map(|(s, t)| (*s, t.to_string())).collect()
    }

    fn rewrite_chapters(path: &Path, list: &[(u64, &str)]) -> Layout {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        write_chapters(&mut file, path, &chapters(list)).unwrap();
        read_layout(&mut file).unwrap()
    }

    #[test]
    fn chapters_are_written_in_place_at_the_tail_and_after_a_fast_start_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        for tail in [true, false] {
            let (path, media) = movie_file(dir.path(), tail);
            let before = fs::read(&path).unwrap();
            let audio = |layout: &Layout| {
                let trak = layout
                    .movie
                    .children()
                    .iter()
                    .find(|c| track_id(c) == Some(1))
                    .unwrap();
                node_offsets(trak).unwrap()
            };
            let original = audio(&read_layout(&mut fs::File::open(&path).unwrap()).unwrap());
            let layout = rewrite_chapters(&path, &[(0, "One"), (5_000_000, "Two – é")]);
            let after = fs::read(&path).unwrap();
            let kinds: Vec<_> = layout.top.iter().map(|b| b.kind).collect();
            let moved = audio(&layout);
            if tail {
                assert_eq!(kinds, [*b"ftyp", *b"mdat", *b"moov", *b"mdat"]);
                assert_eq!(moved, original, "nothing before moov moves");
                assert_eq!(
                    after[..layout.top[2].start as usize],
                    before[..layout.top[2].start as usize]
                );
            } else {
                assert_eq!(kinds, [*b"ftyp", *b"moov", *b"mdat", *b"free", *b"mdat"]);
            }
            let at = moved[0] as usize;
            assert_eq!(&after[at..at + 300], media.as_slice());
            let found = crate::tags::mp4_chapters::scan(&path).unwrap();
            assert_eq!(
                found.list,
                [(0, "One".to_string()), (500, "Two – é".to_string())]
            );
            assert_eq!(found.track, found.list);
            let id = |layout: &Layout| {
                let mvhd = leaf(find(&layout.movie, b"mvhd").unwrap()).unwrap();
                be32(&mvhd[96..])
            };
            assert_eq!(id(&layout), 3, "next track ID follows the chapter track");

            // A second edit replaces the chapter track and reuses the space.
            let length = after.len();
            let layout = rewrite_chapters(&path, &[(0, "A"), (2_000_000, "B"), (9_000_000, "C")]);
            let again = fs::read(&path).unwrap();
            if !tail {
                assert_eq!(again.len(), length, "fits the space after moov");
            }
            assert_eq!(
                &again[audio(&layout)[0] as usize..][..300],
                media.as_slice()
            );
            let found = crate::tags::mp4_chapters::scan(&path).unwrap();
            assert_eq!(found.list.len(), 3);
            assert_eq!(found.track[2], (900, "C".to_string()));
            let traks = layout
                .movie
                .children()
                .iter()
                .filter(|c| &c.kind == b"trak")
                .count();
            assert_eq!(traks, 2, "the old chapter track is gone");
        }
    }

    #[test]
    fn invalid_chapter_lists_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (path, _) = movie_file(dir.path(), true);
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let before = fs::read(&path).unwrap();
        for list in [
            chapters(&[]),
            chapters(&[(0, "A"), (0, "B")]),
            vec![(0, "x".repeat(256))],
            (0..256).map(|i| (i * 10, String::new())).collect(),
        ] {
            assert!(write_chapters(&mut file, &path, &list).is_err());
        }
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}
