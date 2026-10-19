//! Independent bounded EBML observer for retained Matroska filesystem controls.
use std::ops::Range;
pub const SEGMENT: u32 = 0x18538067;
pub const SEEK_HEAD: u32 = 0x114d9b74;
pub const INFO: u32 = 0x1549a966;
pub const TRACKS: u32 = 0x1654ae6b;
pub const TAGS: u32 = 0x1254c367;
pub const CLUSTER: u32 = 0x1f43b675;
pub const CUES: u32 = 0x1c53bb6b;
pub const VOID: u32 = 0xec;
#[derive(Clone, Debug)]
pub struct Element {
    pub id: u32,
    pub start: usize,
    pub data: usize,
    pub end: usize,
}
fn vint(bytes: &[u8], at: usize, size: bool) -> (u64, usize) {
    let first = *bytes.get(at).expect("truncated EBML integer");
    assert_ne!(first, 0);
    let width = first.leading_zeros() as usize + 1;
    assert!(width <= if size { 8 } else { 4 });
    let raw = bytes.get(at..at + width).expect("truncated EBML integer");
    let value = raw.iter().fold(0u64, |n, &b| (n << 8) | u64::from(b));
    if size {
        let mask = (1u64 << (7 * width)) - 1;
        let value = value & mask;
        assert_ne!(value, mask, "unknown-sized fixture element");
        (value, width)
    } else {
        (value, width)
    }
}
pub fn elements(bytes: &[u8], range: Range<usize>) -> Vec<Element> {
    assert!(bytes.len() <= 4 * 1024 * 1024 && range.start <= range.end && range.end <= bytes.len());
    let mut at = range.start;
    let mut result = Vec::new();
    while at < range.end {
        let (id, iw) = vint(bytes, at, false);
        let (size, sw) = vint(bytes, at + iw, true);
        let data = at + iw + sw;
        let end = data.checked_add(usize::try_from(size).unwrap()).unwrap();
        assert!(end <= range.end);
        result.push(Element {
            id: u32::try_from(id).unwrap(),
            start: at,
            data,
            end,
        });
        at = end;
    }
    assert_eq!(at, range.end);
    result
}
pub fn unique(bytes: &[u8], range: Range<usize>, id: u32) -> Element {
    let mut found = elements(bytes, range).into_iter().filter(|e| e.id == id);
    let result = found.next().expect("missing EBML element");
    assert!(found.next().is_none(), "ambiguous fixture EBML path");
    result
}
pub fn segment(bytes: &[u8]) -> Element {
    let segment = unique(bytes, 0..bytes.len(), SEGMENT);
    assert_eq!(segment.end, bytes.len());
    segment
}
pub fn level_one(bytes: &[u8]) -> Vec<Element> {
    let s = segment(bytes);
    elements(bytes, s.data..s.end)
}
pub fn top(bytes: &[u8], id: u32) -> Element {
    let s = segment(bytes);
    unique(bytes, s.data..s.end, id)
}
pub fn raw(bytes: &[u8], e: &Element) -> Vec<u8> {
    bytes[e.start..e.end].to_vec()
}
pub fn uint(bytes: &[u8]) -> u64 {
    assert!(!bytes.is_empty() && bytes.len() <= 8);
    bytes.iter().fold(0u64, |n, &b| (n << 8) | u64::from(b))
}
pub fn seeks(bytes: &[u8]) -> Vec<(u32, usize)> {
    let base = segment(bytes).data;
    let mut result = Vec::new();
    for head in level_one(bytes).into_iter().filter(|e| e.id == SEEK_HEAD) {
        for seek in elements(bytes, head.data..head.end)
            .into_iter()
            .filter(|e| e.id == 0x4dbb)
        {
            let id = unique(bytes, seek.data..seek.end, 0x53ab);
            let pos = unique(bytes, seek.data..seek.end, 0x53ac);
            let id = u32::try_from(uint(&bytes[id.data..id.end])).unwrap();
            let at = base + usize::try_from(uint(&bytes[pos.data..pos.end])).unwrap();
            assert!(
                level_one(bytes).iter().any(|e| e.id == id && e.start == at),
                "Seek entry does not address its element"
            );
            result.push((id, at));
        }
    }
    result
}
/// CueClusterPosition is Segment-relative; CueRelativePosition addresses the
/// actual SimpleBlock element inside that Cluster, before its block header.
pub fn cue_targets(bytes: &[u8]) -> Vec<(usize, usize)> {
    let base = segment(bytes).data;
    let cues = top(bytes, CUES);
    let mut result = Vec::new();
    for cue in elements(bytes, cues.data..cues.end)
        .into_iter()
        .filter(|e| e.id == 0xbb)
    {
        for pos in elements(bytes, cue.data..cue.end)
            .into_iter()
            .filter(|e| e.id == 0xb7)
        {
            let cp = unique(bytes, pos.data..pos.end, 0xf1);
            let rp = unique(bytes, pos.data..pos.end, 0xf0);
            let track = unique(bytes, pos.data..pos.end, 0xf7);
            assert_eq!(uint(&bytes[track.data..track.end]), 1);
            let cluster_at = base + usize::try_from(uint(&bytes[cp.data..cp.end])).unwrap();
            let cluster = level_one(bytes)
                .into_iter()
                .find(|e| e.id == CLUSTER && e.start == cluster_at)
                .expect("cue does not address Cluster");
            let block_at = cluster.data + usize::try_from(uint(&bytes[rp.data..rp.end])).unwrap();
            assert!(
                elements(bytes, cluster.data..cluster.end)
                    .iter()
                    .any(|e| e.id == 0xa3 && e.start == block_at),
                "cue does not address SimpleBlock"
            );
            result.push((cluster_at, block_at));
        }
    }
    assert!(!result.is_empty());
    result
}
pub fn packet_ranges(bytes: &[u8]) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    for cluster in level_one(bytes).into_iter().filter(|e| e.id == CLUSTER) {
        for block in elements(bytes, cluster.data..cluster.end)
            .into_iter()
            .filter(|e| e.id == 0xa3)
        {
            let (track, width) = vint(bytes, block.data, true);
            assert_eq!(track, 1);
            let data = block.data + width + 3;
            assert!(data < block.end);
            assert_eq!(
                bytes[block.data + width + 2] & 6,
                0,
                "fixture uses no lacing"
            );
            result.push(data..block.end);
        }
    }
    assert!(!result.is_empty());
    result
}
pub fn packets(bytes: &[u8]) -> Vec<Vec<u8>> {
    packet_ranges(bytes)
        .into_iter()
        .map(|r| bytes[r].to_vec())
        .collect()
}
pub fn info_title(bytes: &[u8]) -> String {
    let info = top(bytes, INFO);
    let title = unique(bytes, info.data..info.end, 0x7ba9);
    String::from_utf8(bytes[title.data..title.end].to_vec()).unwrap()
}
/// Physical level/name/value records, without production field mappings.
pub fn tag_values(bytes: &[u8]) -> Vec<(u64, String, String)> {
    let mut result = Vec::new();
    for tags in level_one(bytes).into_iter().filter(|e| e.id == TAGS) {
        for tag in elements(bytes, tags.data..tags.end)
            .into_iter()
            .filter(|e| e.id == 0x7373)
        {
            let targets = unique(bytes, tag.data..tag.end, 0x63c0);
            let levels = elements(bytes, targets.data..targets.end);
            let level = levels
                .iter()
                .find(|e| e.id == 0x68ca)
                .map_or(50, |e| uint(&bytes[e.data..e.end]));
            for simple in elements(bytes, tag.data..tag.end)
                .into_iter()
                .filter(|e| e.id == 0x67c8)
            {
                let name = unique(bytes, simple.data..simple.end, 0x45a3);
                let value = unique(bytes, simple.data..simple.end, 0x4487);
                result.push((
                    level,
                    String::from_utf8(bytes[name.data..name.end].to_vec()).unwrap(),
                    String::from_utf8(bytes[value.data..value.end].to_vec()).unwrap(),
                ));
            }
        }
    }
    result
}
