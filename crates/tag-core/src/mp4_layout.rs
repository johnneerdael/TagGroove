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
    Copy { source: TopBox, header: Vec<u8> },
}

/// Write `layout` into `target`. With `reorder`, free space is dropped and
/// `moov` moves before the first `mdat`. Chunk offsets are relocated and the
/// output is verified against `source`.
pub(crate) fn write(
    source: &mut fs::File,
    mut layout: Layout,
    reorder: bool,
    target: &mut fs::File,
) -> Result<(), String> {
    let mut parts: Vec<Part> = Vec::new();
    for (index, top) in layout.top.iter().enumerate() {
        if index == layout.moov {
            parts.push(Part::Movie);
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
    verify(source, target, &parts, &pairs, at, reorder)
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
) -> Result<(), String> {
    let failed = || "The optimized MP4 file did not verify; the original file was kept".to_string();
    let source_len = source.metadata().map_err(io_error)?.len();
    if target.metadata().map_err(io_error)?.len() != length {
        return Err(failed());
    }
    let written = read_layout(target).map_err(|_| failed())?;
    if written.top.len() != parts.len() {
        return Err(failed());
    }
    if reorder && !written.optimized() {
        return Err(failed());
    }
    for (part, top) in parts.iter().zip(&written.top) {
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
        let span = COMPARE.min(source_len - old).min(length - new);
        if !same_bytes(source, *old, target, *new, span)? {
            return Err(failed());
        }
    }
    Ok(())
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
}
