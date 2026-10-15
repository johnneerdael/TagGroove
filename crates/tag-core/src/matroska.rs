//! Matroska and WebM: EBML Tags (SimpleTags grouped by target type value),
//! chapters and image attachments.
//!
//! Chapters of the default edition are exposed as the multi-value field
//! CHAPTERS, one "HH:MM:SS.mmm Title" value per chapter, and as
//! `_SUBSONG_COUNT`. Writing CHAPTERS rebuilds that edition.
//!
//! Writing never moves clusters or cues. Old Tags/Chapters/Attachments are
//! overwritten by EBML Void elements and their replacements are placed in a
//! free (Void) region that fits or appended at the end of the Segment. The
//! SeekHead is rebuilt inside its own space (including following Void
//! padding); if it cannot fit, the write is refused with the original kept.
use super::*;
use lofty::picture::MimeType;

const EBML: u32 = 0x1A45_DFA3;
const DOC_TYPE: u32 = 0x4282;
const SEGMENT: u32 = 0x1853_8067;
const SEEK_HEAD: u32 = 0x114D_9B74;
const SEEK: u32 = 0x4DBB;
const SEEK_ID: u32 = 0x53AB;
const SEEK_POSITION: u32 = 0x53AC;
const INFO: u32 = 0x1549_A966;
const TIMESTAMP_SCALE: u32 = 0x2A_D7B1;
const DURATION: u32 = 0x4489;
const SEGMENT_TITLE: u32 = 0x7BA9;
const TRACKS: u32 = 0x1654_AE6B;
const TRACK_ENTRY: u32 = 0xAE;
const TRACK_TYPE: u32 = 0x83;
const CODEC_ID: u32 = 0x86;
const AUDIO: u32 = 0xE1;
const TRACK_UID: u32 = 0x73C5;
const VIDEO: u32 = 0xE0;
const PIXEL_WIDTH: u32 = 0xB0;
const PIXEL_HEIGHT: u32 = 0xBA;
const TAG_TRACK_UID: u32 = 0x63C5;
const SAMPLING_FREQUENCY: u32 = 0xB5;
const CHANNELS: u32 = 0x9F;
const BIT_DEPTH: u32 = 0x6264;
const TAGS: u32 = 0x1254_C367;
const TAG: u32 = 0x7373;
const TARGETS: u32 = 0x63C0;
const TARGET_TYPE_VALUE: u32 = 0x68CA;
const TARGET_UIDS: [u32; 4] = [0x63C5, 0x63C9, 0x63C4, 0x63C6];
const SIMPLE_TAG: u32 = 0x67C8;
const TAG_NAME: u32 = 0x45A3;
const TAG_STRING: u32 = 0x4487;
const TAG_BINARY: u32 = 0x4485;
const ATTACHMENTS: u32 = 0x1941_A469;
const ATTACHED_FILE: u32 = 0x61A7;
const FILE_DESCRIPTION: u32 = 0x467E;
const FILE_NAME: u32 = 0x466E;
const FILE_MIME_TYPE: u32 = 0x4660;
const FILE_DATA: u32 = 0x465C;
const FILE_UID: u32 = 0x46AE;
const CHAPTERS: u32 = 0x1043_A770;
const EDITION_ENTRY: u32 = 0x45B9;
const EDITION_UID: u32 = 0x45BC;
const EDITION_FLAG_DEFAULT: u32 = 0x45DB;
const CHAPTER_ATOM: u32 = 0xB6;
const CHAPTER_UID: u32 = 0x73C4;
const CHAPTER_TIME_START: u32 = 0x91;
const CHAPTER_TIME_END: u32 = 0x92;
const CHAPTER_DISPLAY: u32 = 0x80;
const CHAP_STRING: u32 = 0x85;
const CHAP_LANGUAGE: u32 = 0x437C;
const CLUSTER: u32 = 0x1F43_B675;
const VOID: u32 = 0xEC;

/// Item-level target type value for fields without a documented mapping.
const ITEM_LEVEL: u64 = 30;
/// Default TargetTypeValue when Targets omits it.
const DEFAULT_LEVEL: u64 = 50;

fn invalid(what: &str) -> String {
    format!("Invalid Matroska file: {what}")
}

/// Element ID (with its length marker) and its encoded length.
fn parse_id(bytes: &[u8]) -> Result<(u32, usize), String> {
    let first = *bytes.first().ok_or_else(|| invalid("truncated element"))?;
    let length = first.leading_zeros() as usize + 1;
    if length > 4 || bytes.len() < length {
        return Err(invalid("bad element ID"));
    }
    Ok((
        bytes[..length]
            .iter()
            .fold(0u32, |id, b| (id << 8) | u32::from(*b)),
        length,
    ))
}

/// Element data size (`None` for the reserved unknown size) and its length.
fn parse_size(bytes: &[u8]) -> Result<(Option<u64>, usize), String> {
    let first = *bytes.first().ok_or_else(|| invalid("truncated element"))?;
    let length = first.leading_zeros() as usize + 1;
    if length > 8 || bytes.len() < length {
        return Err(invalid("bad element size"));
    }
    let mut value = u64::from(first) & (0xFF >> length);
    for b in &bytes[1..length] {
        value = (value << 8) | u64::from(*b);
    }
    let unknown = value == (1u64 << (7 * length)) - 1;
    Ok(((!unknown).then_some(value), length))
}

fn id_bytes(id: u32) -> Vec<u8> {
    let bytes = id.to_be_bytes();
    let skip = (id.leading_zeros() / 8) as usize;
    bytes[skip.min(3)..].to_vec()
}

/// Encode `value` as an EBML size of exactly `width` bytes, or the minimal
/// width when `width` is `None`.
fn size_bytes(value: u64, width: Option<usize>) -> Result<Vec<u8>, String> {
    let fits =
        |w: usize| w < 8 && value < (1u64 << (7 * w)) - 1 || w == 8 && value < (1u64 << 56) - 1;
    let width = match width {
        Some(w) if (1..=8).contains(&w) && fits(w) => w,
        Some(_) => return Err("The Matroska element size does not fit its size field".into()),
        None => (1..=8)
            .find(|w| fits(*w))
            .ok_or("Matroska element is too large")?,
    };
    let marked = value | (1u64 << (7 * width));
    Ok(marked.to_be_bytes()[8 - width..].to_vec())
}

fn element(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = id_bytes(id);
    bytes.extend(size_bytes(payload.len() as u64, None).expect("in-memory element size"));
    bytes.extend(payload);
    bytes
}

fn uint_element(id: u32, value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let skip = (value.leading_zeros() / 8).min(7) as usize;
    element(id, &bytes[skip..])
}

/// A Void element of exactly `length` (>= 2) bytes.
fn void_header(length: u64) -> Result<Vec<u8>, String> {
    if length < 2 {
        return Err("Cannot fill a one-byte gap in a Matroska file".into());
    }
    let mut bytes = id_bytes(VOID);
    if length - 2 < 127 {
        bytes.extend(size_bytes(length - 2, Some(1))?);
    } else {
        bytes.extend(size_bytes(length - 9, Some(8))?);
    }
    Ok(bytes)
}

fn uint(data: &[u8]) -> u64 {
    data.iter().take(8).fold(0, |v, b| (v << 8) | u64::from(*b))
}
fn float(data: &[u8]) -> f64 {
    match data.len() {
        4 => f64::from(f32::from_be_bytes(data.try_into().expect("four bytes"))),
        8 => f64::from_be_bytes(data.try_into().expect("eight bytes")),
        _ => 0.0,
    }
}
fn text(data: &[u8]) -> String {
    decode_text(data)
}

/// One child element inside an in-memory parent.
struct Raw<'a> {
    id: u32,
    data: &'a [u8],
    whole: &'a [u8],
}

fn children(data: &[u8]) -> Result<Vec<Raw<'_>>, String> {
    let mut output = Vec::new();
    let mut position = 0;
    while position < data.len() {
        let (id, id_length) = parse_id(&data[position..])?;
        let (size, size_length) = parse_size(&data[position + id_length..])?;
        let start = position + id_length + size_length;
        let size = size.ok_or_else(|| invalid("unknown size inside a metadata element"))?;
        let end = usize::try_from(size)
            .ok()
            .and_then(|s| start.checked_add(s))
            .filter(|end| *end <= data.len())
            .ok_or_else(|| invalid("element exceeds its parent"))?;
        output.push(Raw {
            id,
            data: &data[start..end],
            whole: &data[position..end],
        });
        position = end;
    }
    Ok(output)
}

#[derive(Debug, Clone)]
struct Element {
    id: u32,
    start: u64,
    data: u64,
    end: u64,
}

struct Layout {
    length: u64,
    segment_size_at: u64,
    segment_size_width: usize,
    segment_unknown: bool,
    data_start: u64,
    data_end: u64,
    elements: Vec<Element>,
    /// False when an unknown-sized cluster stopped the sequential scan.
    complete: bool,
    doc_type: String,
}

struct Header {
    id: u32,
    size: Option<u64>,
    size_at: u64,
    size_width: usize,
    data: u64,
}

fn read_header(file: &mut fs::File, at: u64, length: u64) -> Result<Header, String> {
    let available = (length.saturating_sub(at)).min(12) as usize;
    let bytes = read_at(file, at, available)?;
    let (id, id_length) = parse_id(&bytes)?;
    let (size, size_width) = parse_size(&bytes[id_length..])?;
    Ok(Header {
        id,
        size,
        size_at: at + id_length as u64,
        size_width,
        data: at + (id_length + size_width) as u64,
    })
}

fn layout(file: &mut fs::File) -> Result<Layout, String> {
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let header = read_header(file, 0, length)?;
    if header.id != EBML {
        return Err(invalid("missing EBML header"));
    }
    let size = header.size.ok_or_else(|| invalid("EBML header size"))?;
    if header.data + size > length || size > 4096 {
        return Err(invalid("EBML header size"));
    }
    let ebml = read_at(file, header.data, size as usize)?;
    let doc_type = children(&ebml)?
        .iter()
        .find(|c| c.id == DOC_TYPE)
        .map(|c| text(c.data))
        .unwrap_or_default();
    if !matches!(doc_type.as_str(), "matroska" | "webm") {
        return Err(invalid("unsupported EBML document type"));
    }
    let mut position = header.data + size;
    let segment = loop {
        if position >= length {
            return Err(invalid("missing Segment"));
        }
        let header = read_header(file, position, length)?;
        if header.id == SEGMENT {
            break header;
        }
        position = header
            .size
            .map(|s| header.data + s)
            .ok_or_else(|| invalid("unknown-sized element before the Segment"))?;
    };
    let data_start = segment.data;
    let data_end = match segment.size {
        Some(size) => data_start
            .checked_add(size)
            .filter(|end| *end <= length)
            .ok_or_else(|| invalid("the Segment is truncated"))?,
        None => length,
    };
    let mut elements = Vec::new();
    let mut complete = true;
    let mut position = data_start;
    while position < data_end {
        let header = read_header(file, position, data_end)?;
        let Some(size) = header.size else {
            if header.id == CLUSTER {
                complete = false;
                break;
            }
            return Err(invalid("unknown-sized top-level element"));
        };
        let end = header
            .data
            .checked_add(size)
            .filter(|end| *end <= data_end)
            .ok_or_else(|| invalid("top-level element exceeds the Segment"))?;
        elements.push(Element {
            id: header.id,
            start: position,
            data: header.data,
            end,
        });
        position = end;
    }
    let mut layout = Layout {
        length,
        segment_size_at: segment.size_at,
        segment_size_width: segment.size_width,
        segment_unknown: segment.size.is_none(),
        data_start,
        data_end,
        elements,
        complete,
        doc_type,
    };
    if !complete {
        // Live recordings: find the metadata elements through the SeekHead.
        for (id, position) in seek_entries(file, &layout)? {
            let at = data_start + position;
            if matches!(id, INFO | TRACKS | TAGS | CHAPTERS | ATTACHMENTS)
                && !layout.elements.iter().any(|e| e.start == at)
                && at < data_end
            {
                let header = read_header(file, at, data_end)?;
                if let (true, Some(size)) = (header.id == id, header.size) {
                    if header.data + size <= data_end {
                        layout.elements.push(Element {
                            id,
                            start: at,
                            data: header.data,
                            end: header.data + size,
                        });
                    }
                }
            }
        }
    }
    Ok(layout)
}

/// Metadata elements are read into memory; larger ones are rejected.
const METADATA_LIMIT: u64 = 512 << 20;
/// Bound for nested SimpleTags.
const NESTING_LIMIT: usize = 32;

fn element_data(file: &mut fs::File, element: &Element) -> Result<Vec<u8>, String> {
    let size = usize::try_from(element.end - element.data)
        .ok()
        .filter(|size| *size as u64 <= METADATA_LIMIT)
        .ok_or_else(|| invalid("metadata element too large"))?;
    read_at(file, element.data, size)
}

fn seek_entries(file: &mut fs::File, layout: &Layout) -> Result<Vec<(u32, u64)>, String> {
    let mut entries = Vec::new();
    for head in layout.elements.iter().filter(|e| e.id == SEEK_HEAD) {
        let data = element_data(file, head)?;
        for seek in children(&data)?.iter().filter(|c| c.id == SEEK) {
            let fields = children(seek.data)?;
            let id = fields
                .iter()
                .find(|c| c.id == SEEK_ID)
                .map(|c| uint(c.data));
            let position = fields
                .iter()
                .find(|c| c.id == SEEK_POSITION)
                .map(|c| uint(c.data));
            if let (Some(id), Some(position)) = (id, position) {
                entries.push((id as u32, position));
            }
        }
    }
    Ok(entries)
}

#[derive(Debug, Clone)]
struct SimpleTag {
    name: String,
    value: Option<String>,
    /// Language, default flag, binary value and unknown children.
    extra: Vec<Vec<u8>>,
    nested: Vec<SimpleTag>,
}
impl SimpleTag {
    fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
            extra: Vec::new(),
            nested: Vec::new(),
        }
    }
    fn parse(data: &[u8], depth: usize) -> Result<Self, String> {
        if depth > NESTING_LIMIT {
            return Err(invalid("SimpleTags nested too deeply"));
        }
        let mut tag = Self {
            name: String::new(),
            value: None,
            extra: Vec::new(),
            nested: Vec::new(),
        };
        for child in children(data)? {
            match child.id {
                TAG_NAME => tag.name = text(child.data),
                TAG_STRING => tag.value = Some(text(child.data)),
                SIMPLE_TAG => tag.nested.push(Self::parse(child.data, depth + 1)?),
                _ => tag.extra.push(child.whole.to_vec()),
            }
        }
        Ok(tag)
    }
    fn bytes(&self) -> Vec<u8> {
        let mut payload = element(TAG_NAME, self.name.as_bytes());
        for extra in &self.extra {
            payload.extend(extra);
        }
        if let Some(value) = &self.value {
            payload.extend(element(TAG_STRING, value.as_bytes()));
        }
        for nested in &self.nested {
            payload.extend(nested.bytes());
        }
        element(SIMPLE_TAG, &payload)
    }
    fn is(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }
}

#[derive(Debug, Clone)]
struct MkvTag {
    targets: Option<Vec<u8>>,
    level: u64,
    /// Bound to a track, edition, chapter or attachment UID.
    scoped: bool,
    simple: Vec<SimpleTag>,
    extra: Vec<Vec<u8>>,
}
impl MkvTag {
    fn new(level: u64) -> Self {
        Self {
            targets: Some(element(TARGETS, &uint_element(TARGET_TYPE_VALUE, level))),
            level,
            scoped: false,
            simple: Vec::new(),
            extra: Vec::new(),
        }
    }
    fn parse(data: &[u8]) -> Result<Self, String> {
        let mut tag = Self {
            targets: None,
            level: DEFAULT_LEVEL,
            scoped: false,
            simple: Vec::new(),
            extra: Vec::new(),
        };
        for child in children(data)? {
            match child.id {
                TARGETS => {
                    for target in children(child.data)? {
                        if target.id == TARGET_TYPE_VALUE {
                            tag.level = uint(target.data);
                        } else if TARGET_UIDS.contains(&target.id) && uint(target.data) != 0 {
                            tag.scoped = true;
                        }
                    }
                    tag.targets = Some(child.whole.to_vec());
                }
                SIMPLE_TAG => tag.simple.push(SimpleTag::parse(child.data, 0)?),
                _ => tag.extra.push(child.whole.to_vec()),
            }
        }
        Ok(tag)
    }
    fn bytes(&self) -> Vec<u8> {
        let mut payload = self
            .targets
            .clone()
            .unwrap_or_else(|| element(TARGETS, &[]));
        for simple in &self.simple {
            payload.extend(simple.bytes());
        }
        for extra in &self.extra {
            payload.extend(extra);
        }
        element(TAG, &payload)
    }
    fn global(&self) -> bool {
        !self.scoped
    }
}

/// Where one Mp3tag field lives: a SimpleTag name at a target type value,
/// or a SORT_WITH nested in that level's TITLE.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Rule {
    field: &'static str,
    level: u64,
    name: String,
    sort: bool,
}

/// Documented Matroska mappings. `video` selects the recommended mapping for
/// Matroska video (MKV) files; audio files use the MKA mapping.
fn mapping_rules(video: bool) -> Vec<Rule> {
    FIELD_MAPPINGS
        .iter()
        .filter(|m| !m.matroska.is_empty())
        .filter(|m| match m.field {
            "TVEPISODE" | "TVSEASON" => video,
            // T=50 PART_NUMBER means the episode in video files.
            "DISCNUMBER" => !video,
            _ => true,
        })
        .filter_map(|m| {
            let spec = match m.matroska.find("MKV:") {
                Some(at) if video => &m.matroska[at + 4..],
                Some(at) => &m.matroska[..at],
                None => m.matroska,
            };
            let spec = spec.trim().trim_start_matches("MKA:").trim();
            let mut tokens = spec.split_whitespace();
            let level = tokens.next()?.strip_prefix("T=")?.parse().ok()?;
            let name: Vec<_> = tokens.collect();
            let name = if name.is_empty() {
                m.field.to_owned()
            } else {
                name.join(" ")
            };
            let sort = name == "SORT_WITH";
            Some(Rule {
                field: m.field,
                level,
                name: if sort { "TITLE".into() } else { name },
                sort,
            })
        })
        .collect()
}

struct Model {
    tags: Vec<MkvTag>,
    rules: Vec<Rule>,
}
impl Model {
    fn rule_for_field(&self, field: &str) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|r| r.field.eq_ignore_ascii_case(field))
    }
    fn rule_at(&self, level: u64, name: &str, sort: bool) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|r| r.level == level && r.sort == sort && r.name.eq_ignore_ascii_case(name))
    }
    /// Native names and Mp3tag fields that belong to documented mappings;
    /// they are never exposed or written under their raw name.
    fn reserved(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case("SORT_WITH")
            || self
                .rules
                .iter()
                .any(|r| r.name.eq_ignore_ascii_case(name) || r.field.eq_ignore_ascii_case(name))
    }
    /// A SimpleTag named after a mapped Mp3tag field that is not itself a
    /// Matroska name (for example ALBUM, which ffmpeg writes): an alias read
    /// only when the documented location is empty, and replaced on write.
    fn alias(&self, name: &str) -> Option<&'static str> {
        let rule = self.rule_for_field(name)?;
        (!self.rules.iter().any(|r| r.name.eq_ignore_ascii_case(name))).then_some(rule.field)
    }

    fn fields(&self) -> Fields {
        let mut fields = Fields::new();
        let mut push = |name: &str, value: &str| {
            let key = fields
                .keys()
                .find(|k| k.eq_ignore_ascii_case(name))
                .cloned()
                .unwrap_or_else(|| name.to_owned());
            fields.entry(key).or_default().push(value.to_owned());
        };
        let mut aliases: Vec<(&str, &str)> = Vec::new();
        for tag in self.tags.iter().filter(|t| t.global()) {
            for simple in &tag.simple {
                if let Some(value) = &simple.value {
                    if let Some(rule) = self.rule_at(tag.level, &simple.name, false) {
                        push(rule.field, value);
                    } else if let Some(field) = self.alias(&simple.name) {
                        aliases.push((field, value));
                    } else if !self.reserved(&simple.name) && !simple.name.is_empty() {
                        push(&simple.name, value);
                    }
                }
                if simple.is("TITLE") {
                    for nested in simple.nested.iter().filter(|n| n.is("SORT_WITH")) {
                        if let (Some(rule), Some(value)) =
                            (self.rule_at(tag.level, "TITLE", true), &nested.value)
                        {
                            push(rule.field, value);
                        }
                    }
                }
            }
        }
        let documented: Vec<String> = fields.keys().cloned().collect();
        for (field, value) in aliases {
            if !documented.iter().any(|k| k.eq_ignore_ascii_case(field)) {
                fields
                    .entry(field.to_owned())
                    .or_default()
                    .push(value.to_owned());
            }
        }
        fields
    }

    /// Replace the values of SimpleTags named `name` at `level`, reusing
    /// existing elements (and their language and nested tags) in order.
    fn set(&mut self, level: u64, name: &str, values: &[String]) {
        let mut remaining = values.iter();
        let mut home = None;
        for (index, tag) in self.tags.iter_mut().enumerate() {
            if !tag.global() || tag.level != level {
                continue;
            }
            home.get_or_insert(index);
            tag.simple.retain_mut(|simple| {
                if !simple.is(name) {
                    return true;
                }
                match remaining.next() {
                    Some(value) => {
                        simple.value = Some(value.clone());
                        simple
                            .extra
                            .retain(|raw| parse_id(raw).map(|(id, _)| id) != Ok(TAG_BINARY));
                        true
                    }
                    None => false,
                }
            });
        }
        let rest: Vec<_> = remaining.collect();
        if rest.is_empty() {
            return;
        }
        let index = match home {
            Some(index) => index,
            None => {
                self.tags.push(MkvTag::new(level));
                self.tags.len() - 1
            }
        };
        for value in rest {
            self.tags[index].simple.push(SimpleTag::new(name, value));
        }
    }

    fn set_sort(&mut self, level: u64, values: &[String]) -> Result<(), String> {
        let title = self
            .tags
            .iter_mut()
            .filter(|t| t.global() && t.level == level)
            .flat_map(|t| t.simple.iter_mut())
            .find(|s| s.is("TITLE"));
        match title {
            Some(title) => {
                title.nested.retain(|n| !n.is("SORT_WITH"));
                for value in values {
                    title.nested.push(SimpleTag::new("SORT_WITH", value));
                }
                Ok(())
            }
            None if values.is_empty() => Ok(()),
            None => Err(
                "Matroska stores sort titles inside the matching title; set the title first".into(),
            ),
        }
    }

    fn apply(&mut self, updates: &Fields) -> Result<(), String> {
        let mut sorts = Vec::new();
        for (field, values) in updates {
            match self.rule_for_field(field).cloned() {
                Some(rule) if rule.sort => sorts.push((rule.level, values)),
                Some(rule) => {
                    if self.alias(field).is_some() {
                        for tag in self.tags.iter_mut().filter(|t| t.global()) {
                            tag.simple.retain(|s| !s.is(field));
                        }
                    }
                    self.set(rule.level, &rule.name, values);
                }
                None if self.reserved(field) => {
                    return Err(format!("{field} is a reserved Matroska tag name"));
                }
                None => {
                    for tag in self.tags.iter_mut().filter(|t| t.global()) {
                        tag.simple.retain(|s| !s.is(field));
                    }
                    self.set(ITEM_LEVEL, field, values);
                }
            }
        }
        for (level, values) in sorts {
            self.set_sort(level, values)?;
        }
        self.tags
            .retain(|t| t.scoped || !t.simple.is_empty() || !t.extra.is_empty());
        Ok(())
    }
}

struct Attachment {
    raw: Vec<u8>,
    name: String,
    mime: String,
    description: String,
    data: Vec<u8>,
}
impl Attachment {
    fn parse(raw: &[u8], data: &[u8]) -> Result<Self, String> {
        let mut attachment = Self {
            raw: raw.to_vec(),
            name: String::new(),
            mime: String::new(),
            description: String::new(),
            data: Vec::new(),
        };
        for child in children(data)? {
            match child.id {
                FILE_NAME => attachment.name = text(child.data),
                FILE_MIME_TYPE => attachment.mime = text(child.data),
                FILE_DESCRIPTION => attachment.description = text(child.data),
                FILE_DATA => attachment.data = child.data.to_vec(),
                _ => {}
            }
        }
        Ok(attachment)
    }
    fn image(&self) -> bool {
        self.mime.to_ascii_lowercase().starts_with("image/")
    }
    fn picture(&self) -> Picture {
        let mut picture = Picture::unchecked(self.data.clone())
            .pic_type(picture_type(&self.name))
            .mime_type(MimeType::from_str(&self.mime));
        if !self.description.is_empty() {
            picture = picture.description(self.description.clone());
        }
        picture.build()
    }
}

/// Attachment file name stems for picture types. Matroska only standardises
/// front covers (cover, small_cover, cover_land, small_cover_land).
const PICTURE_NAMES: &[(PictureType, &str)] = &[
    (PictureType::CoverFront, "cover"),
    (PictureType::CoverBack, "back"),
    (PictureType::Icon, "icon"),
    (PictureType::OtherIcon, "other_icon"),
    (PictureType::Leaflet, "leaflet"),
    (PictureType::Media, "media"),
    (PictureType::LeadArtist, "lead_artist"),
    (PictureType::Artist, "artist"),
    (PictureType::Conductor, "conductor"),
    (PictureType::Band, "band"),
    (PictureType::Composer, "composer"),
    (PictureType::Lyricist, "lyricist"),
    (PictureType::RecordingLocation, "recording_location"),
    (PictureType::DuringRecording, "during_recording"),
    (PictureType::DuringPerformance, "during_performance"),
    (PictureType::ScreenCapture, "screen_capture"),
    (PictureType::BrightFish, "fish"),
    (PictureType::Illustration, "illustration"),
    (PictureType::BandLogo, "band_logo"),
    (PictureType::PublisherLogo, "publisher_logo"),
    (PictureType::Other, "other"),
];

fn picture_type(name: &str) -> PictureType {
    // ffmpeg on Windows stores the whole `-attach` path, since it only strips
    // directories up to a '/'; judge the type by the base name.
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = name
        .rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_ascii_lowercase();
    let stem = match stem.rsplit_once(" (") {
        Some((base, suffix))
            if suffix.ends_with(')')
                && suffix[..suffix.len() - 1]
                    .bytes()
                    .all(|b| b.is_ascii_digit()) =>
        {
            base.to_owned()
        }
        _ => stem,
    };
    if matches!(
        stem.as_str(),
        "small_cover" | "cover_land" | "small_cover_land"
    ) {
        return PictureType::CoverFront;
    }
    PICTURE_NAMES
        .iter()
        .find(|(_, name)| *name == stem)
        .map_or(PictureType::Other, |(kind, _)| *kind)
}

fn picture_name(picture: &Picture, taken: &[String]) -> String {
    let stem = PICTURE_NAMES
        .iter()
        .find(|(kind, _)| *kind == picture.pic_type())
        .map_or("other", |(_, name)| name);
    let mime = picture
        .mime_type()
        .map(MimeType::as_str)
        .unwrap_or_default();
    let extension = picture
        .mime_type()
        .and_then(MimeType::ext)
        .or_else(|| mime.strip_prefix("image/"))
        .filter(|e| !e.is_empty() && e.bytes().all(|b| b.is_ascii_alphanumeric()))
        .unwrap_or("bin");
    let mut name = format!("{stem}.{extension}");
    let mut suffix = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
        name = format!("{stem} ({suffix}).{extension}");
        suffix += 1;
    }
    name
}

fn attached_file(picture: &Picture, name: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    if let Some(description) = picture.description().filter(|d| !d.is_empty()) {
        payload.extend(element(FILE_DESCRIPTION, description.as_bytes()));
    }
    payload.extend(element(FILE_NAME, name.as_bytes()));
    let mime = picture
        .mime_type()
        .map(MimeType::as_str)
        .unwrap_or("application/octet-stream");
    payload.extend(element(FILE_MIME_TYPE, mime.as_bytes()));
    payload.extend(element(FILE_DATA, picture.data()));
    payload.extend(uint_element(FILE_UID, rand::random::<u64>().max(1)));
    element(ATTACHED_FILE, &payload)
}

use super::subsongs::{chapter_value, parse_chapter_value, CHAPTERS_FIELD};

#[derive(Debug, Clone)]
struct Chapter {
    /// Start and stored end in nanoseconds.
    start: u64,
    end: Option<u64>,
    title: String,
    uid: u64,
    /// Flags and other children kept when the chapter is rewritten.
    extra: Vec<Vec<u8>>,
    /// Language children of the first ChapterDisplay.
    display: Vec<Vec<u8>>,
    /// Nested chapters.
    children: Vec<Chapter>,
}

/// The editions of a Chapters element and the chapter tree of the default
/// (or else first) edition.
#[derive(Debug, Clone)]
struct Editions {
    raw: Vec<Vec<u8>>,
    current: Option<usize>,
    /// EditionUID, flags and other children of the current edition.
    edition_extra: Vec<Vec<u8>>,
    chapters: Vec<Chapter>,
}

impl Editions {
    /// Chapters in document order, nested ones after their parent: the
    /// order of CHAPTERS values.
    fn flat(&self) -> Vec<&Chapter> {
        fn walk<'a>(list: &'a [Chapter], out: &mut Vec<&'a Chapter>) {
            for chapter in list {
                out.push(chapter);
                walk(&chapter.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.chapters, &mut out);
        out
    }
}

fn parse_chapters(data: &[u8]) -> Result<Editions, String> {
    let editions: Vec<_> = children(data)?
        .into_iter()
        .filter(|c| c.id == EDITION_ENTRY)
        .collect();
    let mut current = None;
    for (index, edition) in editions.iter().enumerate() {
        if children(edition.data)?
            .iter()
            .any(|c| c.id == EDITION_FLAG_DEFAULT && uint(c.data) == 1)
        {
            current = Some(index);
            break;
        }
    }
    let current = current.or((!editions.is_empty()).then_some(0));
    let mut chapters = Vec::new();
    let mut edition_extra = Vec::new();
    if let Some(index) = current {
        for child in children(editions[index].data)? {
            if child.id == CHAPTER_ATOM {
                chapters.push(parse_chapter(child.data, 0)?);
            } else {
                edition_extra.push(child.whole.to_vec());
            }
        }
    }
    Ok(Editions {
        raw: editions.iter().map(|e| e.whole.to_vec()).collect(),
        current,
        edition_extra,
        chapters,
    })
}

fn parse_chapter(data: &[u8], depth: usize) -> Result<Chapter, String> {
    if depth > NESTING_LIMIT {
        return Err(invalid("chapters nested too deeply"));
    }
    let mut chapter = Chapter {
        start: 0,
        end: None,
        title: String::new(),
        uid: 0,
        extra: Vec::new(),
        display: Vec::new(),
        children: Vec::new(),
    };
    let mut titled = false;
    for child in children(data)? {
        match child.id {
            CHAPTER_UID => chapter.uid = uint(child.data),
            CHAPTER_TIME_START => chapter.start = uint(child.data),
            CHAPTER_TIME_END => chapter.end = Some(uint(child.data)),
            CHAPTER_DISPLAY if !titled => {
                titled = true;
                for part in children(child.data)? {
                    if part.id == CHAP_STRING {
                        chapter.title = text(part.data);
                    } else {
                        chapter.display.push(part.whole.to_vec());
                    }
                }
            }
            CHAPTER_ATOM => chapter.children.push(parse_chapter(child.data, depth + 1)?),
            _ => chapter.extra.push(child.whole.to_vec()),
        }
    }
    Ok(chapter)
}

/// Write CHAPTERS values in their canonical form, so they read back as written.
pub(super) fn normalize_chapters(updates: &mut Fields) -> Result<(), String> {
    super::subsongs::normalize_chapters(updates)
}

/// For each new value, the old chapter (document order) it replaces. Lists
/// of equal length pair by position; otherwise chapters with the same start
/// (in milliseconds) are kept in order and the rest are added or removed.
fn pair_chapters(old: &[u64], new: &[u64]) -> Vec<Option<usize>> {
    if old.len() == new.len() {
        return (0..new.len()).map(Some).collect();
    }
    let ms = |ns: u64| ns / 1_000_000;
    let mut table = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            table[i][j] = if ms(old[i]) == ms(new[j]) {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let mut pairs = vec![None; new.len()];
    let (mut i, mut j) = (0, 0);
    while i < old.len() && j < new.len() {
        if ms(old[i]) == ms(new[j]) {
            pairs[j] = Some(i);
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// A chapter of the rebuilt tree, with the times it had before.
struct Rebuilt<'a> {
    start: u64,
    title: String,
    old: Option<(&'a Chapter, Option<u64>)>,
    children: Vec<Rebuilt<'a>>,
}

/// Map the old tree onto the new values: paired chapters keep their place,
/// UID, flags, languages and nested chapters; removed chapters hand their
/// nested chapters to their parent; new chapters follow the chapter before
/// them in document order.
fn rebuild_tree<'a>(
    list: &'a [Chapter],
    index: &mut usize,
    old_of: &[Option<usize>],
    after: &[Vec<usize>],
    values: &[(u64, String)],
) -> Vec<Rebuilt<'a>> {
    let fresh = |j: usize| Rebuilt {
        start: values[j].0,
        title: values[j].1.clone(),
        old: None,
        children: Vec::new(),
    };
    let mut out = Vec::new();
    for (k, chapter) in list.iter().enumerate() {
        let i = *index;
        *index += 1;
        let mut children = rebuild_tree(&chapter.children, index, old_of, after, values);
        let next = list.get(k + 1).map(|n| n.start);
        match old_of.iter().position(|o| *o == Some(i)) {
            Some(j) => {
                let (start, title) = &values[j];
                // Keep the exact stored start while it still displays the same.
                let start = if chapter.start / 1_000_000 == start / 1_000_000 {
                    chapter.start
                } else {
                    *start
                };
                let added: Vec<_> = after[i].iter().map(|j| fresh(*j)).collect();
                let has_children = !children.is_empty();
                if has_children {
                    children.splice(0..0, added);
                }
                out.push(Rebuilt {
                    start,
                    title: title.clone(),
                    old: Some((chapter, next)),
                    children,
                });
                if !has_children {
                    out.extend(after[i].iter().map(|j| fresh(*j)));
                }
            }
            None => out.extend(children),
        }
    }
    out
}

/// Chapter atoms of a rebuilt sibling list. A chapter keeps its stored end
/// while its start and the next start are unchanged; otherwise it ends where
/// the next sibling starts, or with its parent or the file.
fn chapter_atoms(list: &[Rebuilt], parent_end: Option<u64>, duration: Option<u64>) -> Vec<u8> {
    let mut atoms = Vec::new();
    for (k, chapter) in list.iter().enumerate() {
        let next = list.get(k + 1).map(|n| n.start);
        let computed = next
            .or(parent_end)
            .or(duration)
            .filter(|end| *end > chapter.start);
        let end = match chapter.old {
            Some((old, old_next))
                if old.start == chapter.start && old_next == next && old.end.is_some() =>
            {
                old.end
            }
            _ => computed,
        };
        let before = chapter.old.map(|(old, _)| old);
        let uid = before
            .map(|c| c.uid)
            .filter(|uid| *uid != 0)
            .unwrap_or_else(|| rand::random::<u64>().max(1));
        let mut payload = uint_element(CHAPTER_UID, uid);
        payload.extend(uint_element(CHAPTER_TIME_START, chapter.start));
        if let Some(end) = end {
            payload.extend(uint_element(CHAPTER_TIME_END, end));
        }
        let mut display = element(CHAP_STRING, chapter.title.as_bytes());
        match before {
            Some(old) => {
                for extra in &old.extra {
                    payload.extend(extra);
                }
                for part in &old.display {
                    display.extend(part);
                }
            }
            None => display.extend(element(CHAP_LANGUAGE, b"und")),
        }
        payload.extend(element(CHAPTER_DISPLAY, &display));
        payload.extend(chapter_atoms(&chapter.children, end, duration));
        atoms.extend(element(CHAPTER_ATOM, &payload));
    }
    atoms
}

/// The Chapters element with the current edition's chapters replaced by
/// `values`, or `None` when no edition remains. Nested chapters stay nested.
fn rebuilt_chapters(
    old: Option<&Editions>,
    values: &[String],
    duration: Option<u64>,
) -> Result<Option<Vec<u8>>, String> {
    let parsed = values
        .iter()
        .map(|value| parse_chapter_value(value))
        .collect::<Result<Vec<_>, _>>()?;
    let tree = old.map_or(&[][..], |e| e.chapters.as_slice());
    let flat = old.map(Editions::flat).unwrap_or_default();
    let starts: Vec<u64> = flat.iter().map(|c| c.start).collect();
    let old_of = pair_chapters(&starts, &parsed.iter().map(|(s, _)| *s).collect::<Vec<_>>());
    // New chapters follow the paired chapter before them.
    let mut after = vec![Vec::new(); flat.len()];
    let mut leading = Vec::new();
    let mut previous = None;
    for (j, pair) in old_of.iter().enumerate() {
        match (pair, previous) {
            (Some(i), _) => previous = Some(*i),
            (None, Some(i)) => after[i].push(j),
            (None, None) => leading.push(j),
        }
    }
    let mut rebuilt: Vec<Rebuilt> = leading
        .iter()
        .map(|j| Rebuilt {
            start: parsed[*j].0,
            title: parsed[*j].1.clone(),
            old: None,
            children: Vec::new(),
        })
        .collect();
    rebuilt.extend(rebuild_tree(tree, &mut 0, &old_of, &after, &parsed));
    let atoms = chapter_atoms(&rebuilt, None, duration);
    let mut editions = old.map(|e| e.raw.clone()).unwrap_or_default();
    let current = old.and_then(|e| e.current);
    if atoms.is_empty() {
        if let Some(index) = current {
            editions.remove(index);
        }
    } else {
        let mut payload = match (old, current) {
            (Some(old), Some(_)) => old.edition_extra.concat(),
            _ => uint_element(EDITION_UID, rand::random::<u64>().max(1)),
        };
        payload.extend(atoms);
        let edition = element(EDITION_ENTRY, &payload);
        match current {
            Some(index) => editions[index] = edition,
            None => editions.push(edition),
        }
    }
    Ok((!editions.is_empty()).then(|| element(CHAPTERS, &editions.concat())))
}

struct Track {
    kind: u64,
    uid: u64,
    codec: String,
    sample_rate: f64,
    channels: u64,
    bits: u64,
    width: u64,
    height: u64,
}

struct Parsed {
    layout: Layout,
    tags: Vec<MkvTag>,
    chapters: Option<Editions>,
    attachments: Vec<Attachment>,
    tracks: Vec<Track>,
    duration: f64,
    /// The Segment Info title.
    title: Option<String>,
}

fn parse(path: &Path) -> Result<Parsed, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let layout = layout(&mut file)?;
    let mut tags = Vec::new();
    let mut chapters = None;
    let mut attachments = Vec::new();
    let mut tracks = Vec::new();
    let mut duration = 0.0;
    let mut title = None;
    let mut seen_info = false;
    let mut seen_tracks = false;
    for element in &layout.elements {
        match element.id {
            INFO if !seen_info => {
                seen_info = true;
                let data = element_data(&mut file, element)?;
                let fields = children(&data)?;
                let scale = fields
                    .iter()
                    .find(|c| c.id == TIMESTAMP_SCALE)
                    .map_or(1_000_000, |c| uint(c.data));
                if let Some(value) = fields.iter().find(|c| c.id == DURATION) {
                    duration = float(value.data) * scale as f64 / 1e9;
                }
                title = fields
                    .iter()
                    .find(|c| c.id == SEGMENT_TITLE)
                    .map(|c| text(c.data));
            }
            TRACKS if !seen_tracks => {
                seen_tracks = true;
                let data = element_data(&mut file, element)?;
                for entry in children(&data)?.iter().filter(|c| c.id == TRACK_ENTRY) {
                    let mut track = Track {
                        kind: 0,
                        uid: 0,
                        codec: String::new(),
                        sample_rate: 0.0,
                        channels: 0,
                        bits: 0,
                        width: 0,
                        height: 0,
                    };
                    for child in children(entry.data)? {
                        match child.id {
                            TRACK_TYPE => track.kind = uint(child.data),
                            TRACK_UID => track.uid = uint(child.data),
                            CODEC_ID => track.codec = text(child.data),
                            VIDEO => {
                                for video in children(child.data)? {
                                    match video.id {
                                        PIXEL_WIDTH => track.width = uint(video.data),
                                        PIXEL_HEIGHT => track.height = uint(video.data),
                                        _ => {}
                                    }
                                }
                            }
                            AUDIO => {
                                for audio in children(child.data)? {
                                    match audio.id {
                                        SAMPLING_FREQUENCY => track.sample_rate = float(audio.data),
                                        CHANNELS => track.channels = uint(audio.data),
                                        BIT_DEPTH => track.bits = uint(audio.data),
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    tracks.push(track);
                }
            }
            TAGS => {
                let data = element_data(&mut file, element)?;
                for tag in children(&data)?.iter().filter(|c| c.id == TAG) {
                    tags.push(MkvTag::parse(tag.data)?);
                }
            }
            CHAPTERS if chapters.is_none() => {
                chapters = Some(parse_chapters(&element_data(&mut file, element)?)?);
            }
            ATTACHMENTS => {
                let data = element_data(&mut file, element)?;
                for attached in children(&data)?.iter().filter(|c| c.id == ATTACHED_FILE) {
                    attachments.push(Attachment::parse(attached.whole, attached.data)?);
                }
            }
            _ => {}
        }
    }
    Ok(Parsed {
        layout,
        tags,
        chapters,
        attachments,
        tracks,
        duration,
        title,
    })
}

fn video(parsed: &Parsed, options: &TagOptions) -> bool {
    options.advanced.matroska_video_mapping && parsed.tracks.iter().any(|t| t.kind == 1)
}

fn codec_name(codec: &str) -> String {
    let name = match codec {
        "A_OPUS" => "Opus",
        "A_VORBIS" => "Vorbis",
        "A_FLAC" => "FLAC",
        "A_ALAC" => "ALAC",
        "A_AC3" => "AC-3",
        "A_EAC3" => "E-AC-3",
        "A_TRUEHD" => "TrueHD",
        "A_MPEG/L3" => "MPEG-1 Layer III",
        "A_MPEG/L2" => "MPEG-1 Layer II",
        "A_WAVPACK4" => "WavPack",
        "A_TTA1" => "TTA",
        c if c.starts_with("A_AAC") => "AAC",
        c if c.starts_with("A_DTS") => "DTS",
        c if c.starts_with("A_PCM") => "PCM",
        other => other,
    };
    name.to_owned()
}

fn video_codec_name(codec: &str) -> String {
    let name = match codec {
        "V_MPEG4/ISO/AVC" => "H.264",
        "V_MPEGH/ISO/HEVC" => "H.265",
        "V_AV1" => "AV1",
        "V_VP8" => "VP8",
        "V_VP9" => "VP9",
        "V_THEORA" => "Theora",
        "V_MPEG1" => "MPEG-1",
        "V_MPEG2" => "MPEG-2",
        "V_MJPEG" => "Motion JPEG",
        "V_PRORES" => "Apple ProRes",
        c if c.starts_with("V_MPEG4/ISO/") => "MPEG-4 Visual",
        other => other,
    };
    name.to_owned()
}

/// `_VIDEO_*` of the first video track. The bitrate is the `BPS` statistic
/// that muxers such as mkvmerge store for the track.
fn video_fields(parsed: &Parsed, technical: &mut Fields) {
    let Some(video) = parsed.tracks.iter().find(|t| t.kind == 1) else {
        return;
    };
    if !video.codec.is_empty() {
        technical.insert("_VIDEO_CODEC".into(), vec![video_codec_name(&video.codec)]);
    }
    for (name, value) in [
        ("_VIDEO_WIDTH", video.width),
        ("_VIDEO_HEIGHT", video.height),
    ] {
        if value > 0 {
            technical.insert(name.into(), vec![value.to_string()]);
        }
    }
    let targets_video = |tag: &MkvTag| {
        video.uid != 0
            && tag.targets.as_deref().is_some_and(|targets| {
                children(targets)
                    .ok()
                    .and_then(|outer| outer.into_iter().next())
                    .and_then(|outer| children(outer.data).ok())
                    .is_some_and(|inner| {
                        inner
                            .iter()
                            .any(|t| t.id == TAG_TRACK_UID && uint(t.data) == video.uid)
                    })
            })
    };
    let bps = parsed
        .tags
        .iter()
        .filter(|tag| targets_video(tag))
        .flat_map(|tag| &tag.simple)
        .find(|simple| simple.name.eq_ignore_ascii_case("BPS"))
        .and_then(|simple| simple.value.as_deref())
        .and_then(|value| value.trim().parse::<u64>().ok());
    if let Some(bps) = bps.filter(|b| *b > 0) {
        technical.insert(
            "_VIDEO_BITRATE".into(),
            vec![((bps as f64) / 1000.0).round().to_string()],
        );
    }
}

pub(super) fn describe(path: &Path, options: &TagOptions) -> Result<Description, String> {
    let parsed = parse(path)?;
    let model = Model {
        rules: mapping_rules(video(&parsed, options)),
        tags: parsed.tags.clone(),
    };
    let audio = parsed
        .tracks
        .iter()
        .find(|t| t.kind == 2)
        .or_else(|| parsed.tracks.first());
    let bitrate = if parsed.duration > 0.0 {
        (parsed.layout.length as f64 * 8.0 / parsed.duration / 1000.0).round() as u32
    } else {
        0
    };
    let mut fields = model.fields();
    let flat = parsed
        .chapters
        .as_ref()
        .map(Editions::flat)
        .unwrap_or_default();
    let mut technical = Fields::new();
    if !flat.is_empty() {
        fields.insert(
            CHAPTERS_FIELD.into(),
            flat.iter()
                .map(|c| chapter_value(c.start, &c.title))
                .collect(),
        );
        technical.insert("_SUBSONG_COUNT".into(), vec![flat.len().to_string()]);
    }
    if let Some(title) = parsed.title.as_ref().filter(|t| !t.is_empty()) {
        technical.insert("_SEGMENT_TITLE".into(), vec![title.clone()]);
    }
    let chapters = flat
        .iter()
        .map(|c| ChapterInfo {
            start: c.start / 1_000_000,
            end: c.end.map(|end| end / 1_000_000),
            title: c.title.clone(),
            fields: Fields::new(),
            source: Default::default(),
        })
        .collect();
    video_fields(&parsed, &mut technical);
    Ok(Description {
        format: if parsed.layout.doc_type == "webm" {
            "WebM"
        } else {
            "Matroska"
        },
        tags: if parsed.tags.is_empty() {
            Vec::new()
        } else {
            vec!["Matroska".into()]
        },
        fields,
        pictures: parsed
            .attachments
            .iter()
            .filter(|a| a.image())
            .map(Attachment::picture)
            .collect(),
        duration: parsed.duration,
        bitrate,
        sample_rate: audio.map_or(0, |t| t.sample_rate.round() as u32),
        channels: audio.map_or(0, |t| u8::try_from(t.channels).unwrap_or(0)),
        bits: audio.and_then(|t| u8::try_from(t.bits).ok()),
        codec: audio.map(|t| codec_name(&t.codec)).unwrap_or_default(),
        id3: None,
        technical,
        chapters,
    })
}

/// A free byte range in the Segment: Void elements and replaced elements.
#[derive(Debug, Clone)]
struct Region {
    start: u64,
    end: u64,
    /// Ends at the end of the file, so it may grow or shrink.
    open: bool,
    /// Reserved for rebuilding this SeekHead (element index).
    seek_head: Option<usize>,
    /// Data placed at the region start.
    placed: Vec<u8>,
}

pub(super) fn write(
    staging: &mut fs::File,
    original: &Path,
    edit: &Edit,
    options: &TagOptions,
) -> Result<(), String> {
    let parsed = parse(original)?;
    let layout = &parsed.layout;
    if !layout.complete {
        return Err("Writing Matroska files with unknown-sized clusters is not supported; original file preserved".into());
    }
    let mut model = Model {
        rules: mapping_rules(video(&parsed, options)),
        tags: parsed.tags.clone(),
    };
    let mut chapters = None;
    let (tags_changed, chapters_changed, attachments_changed) = match edit {
        Edit::Fields(updates) => {
            let mut updates = updates.clone();
            let key = updates
                .keys()
                .find(|k| k.eq_ignore_ascii_case(CHAPTERS_FIELD))
                .cloned();
            let values = key.and_then(|key| updates.remove(&key));
            if let Some(values) = &values {
                let duration = (parsed.duration > 0.0).then_some((parsed.duration * 1e9) as u64);
                chapters = rebuilt_chapters(parsed.chapters.as_ref(), values, duration)?;
            }
            model.apply(&updates)?;
            (!updates.is_empty(), values.is_some(), false)
        }
        Edit::Pictures(_) => (false, false, true),
        Edit::Remove => {
            model.tags.retain(|t| t.scoped);
            (true, parsed.chapters.is_some(), true)
        }
    };
    // New level-1 elements, in placement order.
    let mut new_elements: Vec<(u32, Vec<u8>)> = Vec::new();
    if let Some(chapters) = chapters {
        new_elements.push((CHAPTERS, chapters));
    }
    if tags_changed {
        let payload: Vec<u8> = model.tags.iter().flat_map(MkvTag::bytes).collect();
        if !payload.is_empty() {
            new_elements.push((TAGS, element(TAGS, &payload)));
        }
    }
    if attachments_changed {
        let pictures = match edit {
            Edit::Pictures(pictures) => pictures.as_slice(),
            _ => &[],
        };
        let mut payload = Vec::new();
        let mut taken: Vec<String> = Vec::new();
        for attachment in parsed.attachments.iter().filter(|a| !a.image()) {
            payload.extend(&attachment.raw);
            taken.push(attachment.name.clone());
        }
        let mut originals: Vec<_> = parsed.attachments.iter().filter(|a| a.image()).collect();
        for picture in pictures {
            // Unchanged pictures keep their element, UID and file name.
            if let Some(index) = originals.iter().position(|a| a.picture() == *picture) {
                let attachment = originals.remove(index);
                payload.extend(&attachment.raw);
                taken.push(attachment.name.clone());
                continue;
            }
            let name = picture_name(picture, &taken);
            payload.extend(attached_file(picture, &name));
            taken.push(name);
        }
        if !payload.is_empty() {
            new_elements.push((ATTACHMENTS, element(ATTACHMENTS, &payload)));
        }
    }
    let replaced = |id: u32| {
        (tags_changed && id == TAGS)
            || (chapters_changed && id == CHAPTERS)
            || (attachments_changed && id == ATTACHMENTS)
    };
    let at_eof = layout.data_end == layout.length;

    // Group level-1 elements into free regions; a SeekHead owns the free
    // space directly after it.
    let mut regions: Vec<Region> = Vec::new();
    for (index, element) in layout.elements.iter().enumerate() {
        let free = element.id == VOID || replaced(element.id);
        if element.id == SEEK_HEAD {
            regions.push(Region {
                start: element.start,
                end: element.end,
                open: false,
                seek_head: Some(index),
                placed: Vec::new(),
            });
        } else if free {
            match regions.last_mut() {
                Some(region) if region.end == element.start => region.end = element.end,
                _ => regions.push(Region {
                    start: element.start,
                    end: element.end,
                    open: false,
                    seek_head: None,
                    placed: Vec::new(),
                }),
            }
        } else {
            // A non-free element closes the current region.
            regions.push(Region {
                start: element.end,
                end: element.end,
                open: false,
                seek_head: None,
                placed: Vec::new(),
            });
        }
    }
    regions.retain(|r| r.seek_head.is_some() || r.end > r.start);
    match regions.last_mut() {
        Some(region) if region.end == layout.data_end && region.seek_head.is_none() && at_eof => {
            region.open = true;
        }
        _ if at_eof => regions.push(Region {
            start: layout.data_end,
            end: layout.data_end,
            open: true,
            seek_head: None,
            placed: Vec::new(),
        }),
        _ => {}
    }

    let mut positions: Vec<(u32, u64)> = Vec::new();
    for (id, bytes) in &new_elements {
        let need = bytes.len() as u64;
        let region = regions
            .iter_mut()
            .filter(|r| r.seek_head.is_none())
            .find(|r| {
                let free = r.end - r.start - r.placed.len() as u64;
                r.open || free == need || free >= need + 2
            })
            .ok_or("The Matroska Segment cannot be extended in place; original file preserved")?;
        positions.push((*id, region.start + region.placed.len() as u64));
        region.placed.extend(bytes);
        if region.open {
            region.end = region.end.max(region.start + region.placed.len() as u64);
        }
    }

    // Rebuild every SeekHead whose entries change.
    let mut file = fs::File::open(original).map_err(|e| e.to_string())?;
    let mut first_seek_head = true;
    for region in regions.iter_mut() {
        let Some(index) = region.seek_head else {
            continue;
        };
        let data = element_data(&mut file, &layout.elements[index])?;
        let mut entries = Vec::new();
        let mut changed = false;
        for child in children(&data)? {
            if child.id != SEEK {
                // A CRC-32 child is dropped when the SeekHead is rebuilt.
                continue;
            }
            let target = children(child.data)?
                .iter()
                .find(|c| c.id == SEEK_ID)
                .map(|c| uint(c.data) as u32);
            if target.is_some_and(replaced) {
                changed = true;
            } else {
                entries.push(child.whole.to_vec());
            }
        }
        if first_seek_head {
            first_seek_head = false;
            for (id, position) in &positions {
                let mut seek = element(SEEK_ID, &id_bytes(*id));
                seek.extend(uint_element(SEEK_POSITION, position - layout.data_start));
                entries.push(element(SEEK, &seek));
                changed = true;
            }
        }
        if !changed {
            // Keep the original SeekHead bytes; the rest stays free space.
            region.placed = read_at(
                &mut file,
                region.start,
                (layout.elements[index].end - region.start) as usize,
            )?;
            continue;
        }
        let bytes = element(SEEK_HEAD, &entries.concat());
        let free = region.end - region.start;
        if !(bytes.len() as u64 == free || bytes.len() as u64 + 2 <= free) {
            return Err("The Matroska SeekHead has no room for the new metadata index; original file preserved".into());
        }
        region.placed = bytes;
    }

    // Write each region: placed bytes, then one Void for the remainder.
    let mut data_end = layout.data_end;
    for region in &regions {
        let placed = region.placed.len() as u64;
        if region.open {
            write_at(staging, region.start, &region.placed)?;
            data_end = region.start + placed;
            staging.set_len(data_end).map_err(|e| e.to_string())?;
            continue;
        }
        if placed > 0 {
            write_at(staging, region.start, &region.placed)?;
        }
        let gap = region.end - region.start - placed;
        if gap > 0 {
            let at = region.start + placed;
            let header = void_header(gap)?;
            write_at(staging, at, &header)?;
            let zeros = vec![0u8; (gap - header.len() as u64).min(1 << 20) as usize];
            let mut written = header.len() as u64;
            while written < gap {
                let take = (gap - written).min(zeros.len() as u64) as usize;
                write_at(staging, at + written, &zeros[..take])?;
                written += take as u64;
            }
        }
    }
    if !layout.segment_unknown {
        let size = size_bytes(
            data_end - layout.data_start,
            Some(layout.segment_size_width),
        )?;
        write_at(staging, layout.segment_size_at, &size)?;
    }
    Ok(())
}

/// Every element outside Tags, Attachments, SeekHead and Void (clusters,
/// cues, tracks, info and, unless `edit` changes them, chapters) must be
/// byte-identical at the same offset.
pub(super) fn verify_media(original: &Path, staged: &Path, edit: &Edit) -> Result<(), String> {
    let chapters = match edit {
        Edit::Fields(updates) => updates
            .keys()
            .any(|k| k.eq_ignore_ascii_case(CHAPTERS_FIELD)),
        Edit::Pictures(_) => false,
        Edit::Remove => true,
    };
    let mut file = fs::File::open(original).map_err(|e| e.to_string())?;
    let before = layout(&mut file)?;
    let after = layout(&mut fs::File::open(staged).map_err(|e| e.to_string())?)?;
    let error =
        || Err("The Matroska media data changed while writing; original file preserved".into());
    if !same_bytes(original, 0..before.segment_size_at, staged, 0)? {
        return error();
    }
    for element in before
        .elements
        .iter()
        .filter(|e| !matches!(e.id, TAGS | ATTACHMENTS | SEEK_HEAD | VOID))
        .filter(|e| !(chapters && e.id == CHAPTERS))
    {
        if !after
            .elements
            .iter()
            .any(|e| e.start == element.start && e.end == element.end && e.id == element.id)
            || !same_bytes(original, element.start..element.end, staged, element.start)?
        {
            return error();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_rules_are_unambiguous_in_both_modes() {
        for video in [false, true] {
            let rules = mapping_rules(video);
            for (i, a) in rules.iter().enumerate() {
                for b in &rules[i + 1..] {
                    assert!(
                        !(a.level == b.level && a.sort == b.sort && a.name == b.name),
                        "{a:?} and {b:?} collide (video={video})"
                    );
                }
            }
            let find = |field: &str| rules.iter().find(|r| r.field == field).cloned().unwrap();
            assert_eq!(find("TRACK").level, 30);
            assert_eq!(find("TRACK").name, "PART_NUMBER");
            assert_eq!(find("YEAR").name, "DATE_RECORDED");
            assert_eq!(find("GENRE").name, "GENRE");
            assert!(find("TITLESORT").sort);
        }
        let audio = mapping_rules(false);
        let video = mapping_rules(true);
        let level =
            |rules: &[Rule], field: &str| rules.iter().find(|r| r.field == field).map(|r| r.level);
        assert_eq!(level(&audio, "TITLE"), Some(30));
        assert_eq!(level(&audio, "ALBUM"), Some(50));
        assert_eq!(level(&audio, "DISCNUMBER"), Some(50));
        assert_eq!(level(&audio, "TVEPISODE"), None);
        assert_eq!(level(&video, "TITLE"), Some(50));
        assert_eq!(level(&video, "ALBUM"), Some(60));
        assert_eq!(level(&video, "TVEPISODE"), Some(50));
        assert_eq!(level(&video, "DISCNUMBER"), None);
    }

    #[test]
    fn field_names_used_as_tag_names_are_aliases() {
        let mut tag = MkvTag::new(50);
        tag.simple.push(SimpleTag::new("ALBUM", "From ffmpeg"));
        tag.simple.push(SimpleTag::new("ENCODER", "Lavf"));
        let mut model = Model {
            tags: vec![tag],
            rules: mapping_rules(false),
        };
        let fields = model.fields();
        assert_eq!(fields["ALBUM"], ["From ffmpeg"]);
        assert_eq!(fields["ENCODER"], ["Lavf"]);
        let mut updates = Fields::new();
        updates.insert("ALBUM".into(), vec!["Documented".into()]);
        model.apply(&updates).unwrap();
        assert_eq!(model.fields()["ALBUM"], ["Documented"]);
        assert!(!model.tags[0].simple.iter().any(|s| s.is("ALBUM")));
        updates.insert("ALBUM".into(), Vec::new());
        model.apply(&updates).unwrap();
        assert!(!model.fields().contains_key("ALBUM"));
    }

    #[test]
    fn sizes_and_voids_have_exact_lengths() {
        assert_eq!(size_bytes(0, None).unwrap(), [0x80]);
        assert_eq!(size_bytes(126, None).unwrap(), [0xFE]);
        assert_eq!(size_bytes(127, None).unwrap(), [0x40, 0x7F]);
        assert_eq!(size_bytes(5, Some(8)).unwrap(), [1, 0, 0, 0, 0, 0, 0, 5]);
        assert!(size_bytes(200, Some(1)).is_err());
        assert_eq!(parse_size(&[0xFF]).unwrap(), (None, 1));
        assert_eq!(parse_size(&[0x40, 0x7F]).unwrap(), (Some(127), 2));
        for length in [2u64, 3, 128, 129, 130, 5000] {
            let header = void_header(length).unwrap();
            let (id, id_len) = parse_id(&header).unwrap();
            let (size, size_len) = parse_size(&header[id_len..]).unwrap();
            assert_eq!(id, VOID);
            assert_eq!(id_len as u64 + size_len as u64 + size.unwrap(), length);
        }
        assert!(void_header(1).is_err());
        assert_eq!(id_bytes(TAGS), [0x12, 0x54, 0xC3, 0x67]);
        assert_eq!(id_bytes(VOID), [0xEC]);
        assert_eq!(uint_element(TARGET_TYPE_VALUE, 50), [0x68, 0xCA, 0x81, 50]);
    }

    #[test]
    fn picture_names_round_trip_types() {
        let jpeg = Picture::unchecked(vec![1])
            .pic_type(PictureType::CoverBack)
            .mime_type(MimeType::Jpeg)
            .build();
        let name = picture_name(&jpeg, &["back.jpg".into()]);
        assert_eq!(name, "back (2).jpg");
        assert_eq!(picture_type(&name), PictureType::CoverBack);
        assert_eq!(picture_type("cover.png"), PictureType::CoverFront);
        assert_eq!(
            picture_type("small_cover_land.jpg"),
            PictureType::CoverFront
        );
        assert_eq!(picture_type("poster.jpg"), PictureType::Other);
        assert_eq!(
            picture_type(r"C:\Users\me\Temp\cover.png"),
            PictureType::CoverFront
        );
        assert_eq!(picture_type("/tmp/a/back (3).jpg"), PictureType::CoverBack);
    }
}
