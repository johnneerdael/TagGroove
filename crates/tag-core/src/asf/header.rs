//! ASF Header Object codec, following the Microsoft ASF specification. Only
//! the Header Object is decoded and rebuilt: Data and Index objects are never
//! parsed, and header objects without metadata are kept byte for byte.
//! Attribute objects are re-encoded only when their attribute list changes.

use crate::batch_errors::ErrorCause;

pub(super) type Guid = [u8; 16];

const fn guid(a: u32, b: u16, c: u16, d: [u8; 8]) -> Guid {
    let (a, b, c) = (a.to_le_bytes(), b.to_le_bytes(), c.to_le_bytes());
    [
        a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d[0], d[1], d[2], d[3], d[4], d[5], d[6],
        d[7],
    ]
}
const ASF: [u8; 8] = [0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C];
const ASF_2: [u8; 8] = [0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65];
pub(super) const HEADER: Guid = guid(0x75B2_2630, 0x668E, 0x11CF, ASF);
const CONTENT_DESCRIPTION: Guid = guid(0x75B2_2633, 0x668E, 0x11CF, ASF);
const FILE_PROPERTIES: Guid = guid(
    0x8CAB_DCA1,
    0xA947,
    0x11CF,
    [0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65],
);
const STREAM_PROPERTIES: Guid = guid(0xB7DC_0791, 0xA9B7, 0x11CF, ASF_2);
const HEADER_EXTENSION: Guid = guid(
    0x5FBF_03B5,
    0xA92E,
    0x11CF,
    [0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65],
);
const HEADER_EXTENSION_RESERVED: Guid = guid(0xABD3_D211, 0xA9BA, 0x11CF, ASF_2);
const CODEC_LIST: Guid = guid(
    0x86D1_5240,
    0x311D,
    0x11D0,
    [0xA3, 0xA4, 0x00, 0xA0, 0xC9, 0x03, 0x48, 0xF6],
);
const EXTENDED_CONTENT_DESCRIPTION: Guid = guid(
    0xD2D0_A440,
    0xE307,
    0x11D2,
    [0x97, 0xF0, 0x00, 0xA0, 0xC9, 0x5E, 0xA8, 0x50],
);
const METADATA: Guid = guid(
    0xC5F8_CBEA,
    0x5BAF,
    0x4877,
    [0x84, 0x67, 0xAA, 0x8C, 0x44, 0xFA, 0x4C, 0xCA],
);
const METADATA_LIBRARY: Guid = guid(
    0x4423_1C94,
    0x9498,
    0x49D1,
    [0xA1, 0x41, 0x1D, 0x13, 0x4E, 0x45, 0x70, 0x54],
);
const EXTENDED_STREAM_PROPERTIES: Guid = guid(
    0x14E6_A5CB,
    0xC672,
    0x4332,
    [0x83, 0x99, 0xA9, 0x69, 0x52, 0x06, 0x5B, 0x5A],
);
const AUDIO_MEDIA: Guid = guid(
    0xF869_9E40,
    0x5B4D,
    0x11CF,
    [0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B],
);
#[cfg(test)]
pub(super) const DATA: Guid = guid(0x75B2_2636, 0x668E, 0x11CF, ASF);

/// Content Description fields, in their on-disk order.
pub(super) const CONTENT_NAMES: [&str; 5] =
    ["Title", "Author", "Copyright", "Description", "Rating"];
const OBJECT_HEADER: usize = 24;
/// Headers hold artwork, but anything larger than this is not a tag.
const MAX_HEADER: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    Unicode(String),
    /// Admitted text whose original encoding is not reproduced by UTF-16LE
    /// encoding its display value. Explicit edits create `Unicode` instead.
    UnicodeOriginal {
        display: String,
        raw: Vec<u8>,
    },
    Bytes(Vec<u8>),
    Bool(bool),
    Dword(u32),
    Qword(u64),
    Word(u16),
    Guid(Guid),
    /// Unknown type codes, or sizes that do not match the type, kept verbatim.
    Other {
        kind: u16,
        data: Vec<u8>,
    },
}

/// The header object an attribute is stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Location {
    Content,
    Extended,
    Metadata,
    Library,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Attribute {
    pub(super) name: String,
    pub(super) value: Value,
    // Per-occurrence provenance; distinct names can have the same lossy view.
    pub(super) original_name: Option<Vec<u8>>,
    pub(super) stream: u16,
    pub(super) language: u16,
    /// Where the attribute was read from. New or edited attributes have none
    /// and are placed by their size, type, multiplicity, stream and language.
    pub(super) location: Option<Location>,
}
impl Attribute {
    pub(super) fn new(name: &str, value: Value) -> Self {
        Self {
            name: name.into(),
            value,
            original_name: None,
            stream: 0,
            language: 0,
            location: None,
        }
    }

    fn encoded_name(&self) -> Vec<u8> {
        self.original_name
            .clone()
            .unwrap_or_else(|| utf16z(&self.name))
    }

    // Content's empty slots are not visible fields, but their original bytes
    // must survive sibling edits. Keeping them as occurrences also makes an
    // explicit field removal discard provenance rather than resurrect it.
    pub(super) fn empty_content_slot(&self) -> bool {
        self.location == Some(Location::Content)
            && match &self.value {
                Value::Unicode(text) | Value::UnicodeOriginal { display: text, .. } => {
                    text.is_empty()
                }
                _ => false,
            }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct Properties {
    /// Seconds, without preroll.
    pub(super) duration: f64,
    /// Kilobits per second.
    pub(super) bitrate: u32,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) bits_per_sample: u16,
    pub(super) format_tag: Option<u16>,
    pub(super) codec_name: Option<String>,
}
impl Properties {
    pub(super) fn codec(&self) -> Option<String> {
        self.codec_name.clone().or_else(|| {
            Some(
                match self.format_tag? {
                    0x0160 => "Windows Media Audio V1",
                    0x0161 => "Windows Media Audio",
                    0x0162 => "Windows Media Audio Professional",
                    0x0163 => "Windows Media Audio Lossless",
                    0x000A => "Windows Media Audio Voice",
                    _ => return None,
                }
                .into(),
            )
        })
    }
}

#[derive(Debug, Clone)]
enum Child {
    Raw(Vec<u8>),
    FileProperties(Vec<u8>),
    Attributes(Location, Vec<u8>),
    Extension {
        prefix: Vec<u8>,
        children: Vec<Child>,
    },
}

#[derive(Debug, Clone)]
pub(super) struct Asf {
    reserved: [u8; 2],
    children: Vec<Child>,
    original: Vec<Attribute>,
    pub(super) attributes: Vec<Attribute>,
    pub(super) properties: Properties,
    /// Size of the Header Object on disk; the Data Object starts here.
    pub(super) header_size: u64,
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}
impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], ErrorCause> {
        let end = self
            .position
            .checked_add(length)
            .filter(|end| *end <= self.data.len())
            .ok_or_else(|| ErrorCause::owned("Malformed ASF header: truncated object"))?;
        let bytes = &self.data[self.position..end];
        self.position = end;
        Ok(bytes)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ErrorCause> {
        self.take(N)?
            .try_into()
            .map_err(|_| ErrorCause::owned("Malformed ASF header"))
    }
    fn u16(&mut self) -> Result<u16, ErrorCause> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, ErrorCause> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, ErrorCause> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn remaining(&self) -> usize {
        self.data.len() - self.position
    }
}

/// Split a byte range into complete objects (GUID, whole object bytes).
fn objects(data: &[u8]) -> Result<Vec<(Guid, &[u8])>, ErrorCause> {
    let mut reader = Reader::new(data);
    let mut objects = Vec::new();
    while reader.remaining() > 0 {
        let start = reader.position;
        let id: Guid = reader.array()?;
        let size = usize::try_from(reader.u64()?)
            .ok()
            .filter(|size| *size >= OBJECT_HEADER)
            .ok_or_else(|| ErrorCause::owned("Malformed ASF header: invalid object size"))?;
        reader.take(size - OBJECT_HEADER)?;
        objects.push((id, &data[start..start + size]));
    }
    Ok(objects)
}

pub(super) fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    let end = units
        .iter()
        .rposition(|unit| *unit != 0)
        .map_or(0, |i| i + 1);
    String::from_utf16_lossy(&units[..end])
}
/// UTF-16LE with the terminating NUL the specification requires.
pub(super) fn utf16z(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn decode_text(data: &[u8]) -> Value {
    let display = utf16(data);
    if utf16z(&display) == data {
        Value::Unicode(display)
    } else {
        Value::UnicodeOriginal {
            display,
            raw: data.to_vec(),
        }
    }
}

fn original_name(name: &str, raw: &[u8]) -> Option<Vec<u8>> {
    (utf16z(name) != raw).then(|| raw.to_vec())
}

fn decode_value(kind: u16, data: &[u8], location: Location) -> Value {
    let other = || Value::Other {
        kind,
        data: data.to_vec(),
    };
    let bool_size = if location == Location::Extended { 4 } else { 2 };
    match (kind, data.len()) {
        (0, _) => decode_text(data),
        (1, _) => Value::Bytes(data.to_vec()),
        (2, n) if n == bool_size && data[0] <= 1 && data[1..].iter().all(|b| *b == 0) => {
            Value::Bool(data[0] == 1)
        }
        (3, 4) => Value::Dword(u32::from_le_bytes([data[0], data[1], data[2], data[3]])),
        (4, 8) => Value::Qword(u64::from_le_bytes(data.try_into().unwrap_or_default())),
        (5, 2) => Value::Word(u16::from_le_bytes([data[0], data[1]])),
        (6, 16) if location == Location::Library => {
            Value::Guid(data.try_into().unwrap_or_default())
        }
        _ => other(),
    }
}
fn encode_value(value: &Value, location: Location) -> (u16, Vec<u8>) {
    match value {
        Value::Unicode(text) => (0, utf16z(text)),
        Value::UnicodeOriginal { raw, .. } => (0, raw.clone()),
        Value::Bytes(data) => (1, data.clone()),
        Value::Bool(flag) if location == Location::Extended => {
            (2, u32::from(*flag).to_le_bytes().to_vec())
        }
        Value::Bool(flag) => (2, u16::from(*flag).to_le_bytes().to_vec()),
        Value::Dword(n) => (3, n.to_le_bytes().to_vec()),
        Value::Qword(n) => (4, n.to_le_bytes().to_vec()),
        Value::Word(n) => (5, n.to_le_bytes().to_vec()),
        Value::Guid(id) => (6, id.to_vec()),
        Value::Other { kind, data } => (*kind, data.clone()),
    }
}

fn parse_content(body: &[u8], output: &mut Vec<Attribute>) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(body);
    let mut lengths = [0usize; 5];
    for length in &mut lengths {
        *length = reader.u16()?.into();
    }
    for (name, length) in CONTENT_NAMES.iter().zip(lengths) {
        let raw = reader.take(length)?;
        // Zero-length slots need no provenance: the encoder's absent slot is
        // already zero-length. Nonzero empty slots remain hidden occurrences.
        if !raw.is_empty() {
            output.push(Attribute {
                location: Some(Location::Content),
                ..Attribute::new(name, decode_text(raw))
            });
        }
    }
    Ok(())
}
fn parse_extended(body: &[u8], output: &mut Vec<Attribute>) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(body);
    for _ in 0..reader.u16()? {
        let name_length = reader.u16()?.into();
        let raw_name = reader.take(name_length)?;
        let name = utf16(raw_name);
        let kind = reader.u16()?;
        let length = reader.u16()?.into();
        let value = decode_value(kind, reader.take(length)?, Location::Extended);
        output.push(Attribute {
            location: Some(Location::Extended),
            original_name: original_name(&name, raw_name),
            ..Attribute::new(&name, value)
        });
    }
    Ok(())
}
fn parse_metadata(
    body: &[u8],
    location: Location,
    output: &mut Vec<Attribute>,
) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(body);
    for _ in 0..reader.u16()? {
        let language = reader.u16()?;
        let stream = reader.u16()?;
        let name_length = reader.u16()?.into();
        let kind = reader.u16()?;
        let length = usize::try_from(reader.u32()?).map_err(ErrorCause::external)?;
        let raw_name = reader.take(name_length)?;
        let name = utf16(raw_name);
        let value = decode_value(kind, reader.take(length)?, location);
        output.push(Attribute {
            original_name: original_name(&name, raw_name),
            name,
            value,
            stream,
            language,
            location: Some(location),
        });
    }
    Ok(())
}

fn parse_file_properties(object: &[u8], properties: &mut Properties) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(object);
    reader.take(OBJECT_HEADER + 16 + 8 + 8 + 8)?;
    let play_duration = reader.u64()?;
    let _send_duration = reader.u64()?;
    let preroll = reader.u64()?;
    let flags = reader.u32()?;
    reader.take(8)?;
    let max_bitrate = reader.u32()?;
    // Broadcast files (flag bit 0) have no valid duration.
    if flags & 1 == 0 {
        properties.duration =
            (play_duration as f64 / 10_000_000.0 - preroll as f64 / 1000.0).max(0.0);
    }
    if properties.bitrate == 0 {
        properties.bitrate = max_bitrate / 1000;
    }
    Ok(())
}
fn parse_stream_properties(object: &[u8], properties: &mut Properties) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(object);
    reader.take(OBJECT_HEADER)?;
    let kind: Guid = reader.array()?;
    reader.take(16 + 8)?;
    let specific = usize::try_from(reader.u32()?).map_err(ErrorCause::external)?;
    reader.take(4 + 2 + 4)?;
    if kind != AUDIO_MEDIA || properties.format_tag.is_some() {
        return Ok(());
    }
    // WAVEFORMATEX
    let mut format = Reader::new(reader.take(specific)?);
    properties.format_tag = Some(format.u16()?);
    properties.channels = format.u16()?;
    properties.sample_rate = format.u32()?;
    let bytes_per_second = format.u32()?;
    format.u16()?;
    properties.bits_per_sample = format.u16()?;
    if bytes_per_second > 0 {
        properties.bitrate = (u64::from(bytes_per_second) * 8 / 1000) as u32;
    }
    Ok(())
}
fn parse_extended_stream(object: &[u8], properties: &mut Properties) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(object);
    reader.take(OBJECT_HEADER + 60)?;
    let names = reader.u16()?;
    let systems = reader.u16()?;
    for _ in 0..names {
        reader.u16()?;
        let length = reader.u16()?.into();
        reader.take(length)?;
    }
    for _ in 0..systems {
        reader.take(18)?;
        let length = usize::try_from(reader.u32()?).map_err(ErrorCause::external)?;
        reader.take(length)?;
    }
    let rest = reader.take(reader.remaining())?;
    for (id, object) in objects(rest)? {
        if id == STREAM_PROPERTIES {
            parse_stream_properties(object, properties)?;
        }
    }
    Ok(())
}
fn parse_codec_list(object: &[u8], properties: &mut Properties) -> Result<(), ErrorCause> {
    let mut reader = Reader::new(object);
    reader.take(OBJECT_HEADER + 16)?;
    for _ in 0..reader.u32()? {
        let kind = reader.u16()?;
        let name_length = usize::from(reader.u16()?) * 2;
        let name = utf16(reader.take(name_length)?);
        let description_length = usize::from(reader.u16()?) * 2;
        reader.take(description_length)?;
        let information_length = reader.u16()?.into();
        reader.take(information_length)?;
        if kind == 2 && properties.codec_name.is_none() && !name.trim().is_empty() {
            properties.codec_name = Some(name.trim().to_owned());
        }
    }
    Ok(())
}

fn parse_children(
    data: &[u8],
    extension: bool,
    attributes: &mut Vec<Attribute>,
    properties: &mut Properties,
) -> Result<Vec<Child>, ErrorCause> {
    let mut children = Vec::new();
    for (id, object) in objects(data)? {
        let body = &object[OBJECT_HEADER..];
        let child = match id {
            FILE_PROPERTIES if !extension => {
                parse_file_properties(object, properties)?;
                Child::FileProperties(object.to_vec())
            }
            CONTENT_DESCRIPTION if !extension => {
                parse_content(body, attributes)?;
                Child::Attributes(Location::Content, object.to_vec())
            }
            EXTENDED_CONTENT_DESCRIPTION if !extension => {
                parse_extended(body, attributes)?;
                Child::Attributes(Location::Extended, object.to_vec())
            }
            METADATA if extension => {
                parse_metadata(body, Location::Metadata, attributes)?;
                Child::Attributes(Location::Metadata, object.to_vec())
            }
            METADATA_LIBRARY if extension => {
                parse_metadata(body, Location::Library, attributes)?;
                Child::Attributes(Location::Library, object.to_vec())
            }
            HEADER_EXTENSION if !extension => {
                let mut reader = Reader::new(body);
                reader.take(18)?;
                let size = usize::try_from(reader.u32()?).map_err(ErrorCause::external)?;
                if size != reader.remaining() {
                    return Err(ErrorCause::owned("Malformed ASF header extension size"));
                }
                Child::Extension {
                    prefix: body[..18].to_vec(),
                    children: parse_children(&body[22..], true, attributes, properties)?,
                }
            }
            _ => {
                match id {
                    STREAM_PROPERTIES => parse_stream_properties(object, properties)?,
                    EXTENDED_STREAM_PROPERTIES => parse_extended_stream(object, properties)?,
                    CODEC_LIST => parse_codec_list(object, properties)?,
                    _ => {}
                }
                Child::Raw(object.to_vec())
            }
        };
        children.push(child);
    }
    Ok(children)
}

pub(super) fn is_asf(start: &[u8]) -> bool {
    start.starts_with(&HEADER)
}

impl Asf {
    /// Parse a header from the start of `file`, whose total length is `file_size`.
    #[cfg(test)]
    pub(super) fn read(file: &mut impl std::io::Read, file_size: u64) -> Result<Self, String> {
        Self::read_diagnostic(file, file_size).map_err(|error| error.to_string())
    }

    pub(super) fn read_diagnostic(
        file: &mut impl std::io::Read,
        file_size: u64,
    ) -> Result<Self, ErrorCause> {
        let mut start = [0u8; 30];
        file.read_exact(&mut start)
            .map_err(|_| ErrorCause::owned("Not an ASF file: truncated header"))?;
        if !is_asf(&start) {
            return Err(ErrorCause::owned("Not an ASF file"));
        }
        let header_size = u64::from_le_bytes(start[16..24].try_into().unwrap_or_default());
        if header_size < 30 || header_size > file_size || header_size > MAX_HEADER {
            return Err(ErrorCause::owned(
                "Malformed ASF header: invalid header size",
            ));
        }
        let mut data = vec![0u8; (header_size - 30) as usize];
        file.read_exact(&mut data)
            .map_err(|_| ErrorCause::owned("Malformed ASF header: truncated header"))?;
        let mut attributes = Vec::new();
        let mut properties = Properties::default();
        let children = parse_children(&data, false, &mut attributes, &mut properties)?;
        if !children
            .iter()
            .any(|child| matches!(child, Child::FileProperties(_)))
        {
            return Err(ErrorCause::owned(
                "Malformed ASF header: no File Properties Object",
            ));
        }
        Ok(Self {
            reserved: [start[28], start[29]],
            children,
            original: attributes.clone(),
            attributes,
            properties,
            header_size,
        })
    }

    /// Resolve where every attribute is written. Untouched attributes stay in
    /// their object; new ones follow the specification's size and type limits
    /// and keep repeated names in the Metadata Library.
    fn placed(&self) -> Vec<(Location, &Attribute)> {
        let mut content_used = [false; 5];
        for attribute in &self.attributes {
            if attribute.location == Some(Location::Content) {
                if let Some(i) = CONTENT_NAMES.iter().position(|n| *n == attribute.name) {
                    content_used[i] = true;
                }
            }
        }
        self.attributes
            .iter()
            .map(|attribute| {
                if let Some(location) = attribute.location {
                    return (location, attribute);
                }
                let repeated = self
                    .attributes
                    .iter()
                    .filter(|other| other.name == attribute.name)
                    .count()
                    > 1;
                let (_, data) = encode_value(&attribute.value, Location::Extended);
                let small = data.len() <= usize::from(u16::MAX)
                    && utf16z(&attribute.name).len() <= usize::from(u16::MAX);
                let content = CONTENT_NAMES.iter().position(|n| *n == attribute.name);
                let location = if repeated
                    || !small
                    || attribute.language != 0
                    || matches!(attribute.value, Value::Guid(_))
                {
                    Location::Library
                } else if attribute.stream != 0 {
                    Location::Metadata
                } else if let Some(i) = content.filter(|i| {
                    !content_used[*i]
                        && matches!(&attribute.value, Value::Unicode(text) if !text.is_empty())
                }) {
                    content_used[i] = true;
                    Location::Content
                } else {
                    Location::Extended
                };
                (location, attribute)
            })
            .collect()
    }

    fn encode_attributes(
        location: Location,
        attributes: &[&Attribute],
    ) -> Result<Vec<u8>, ErrorCause> {
        let too_many = || {
            ErrorCause::message(
                "ASF {value} object limits exceeded",
                [("value", format!("{location:?}"))],
            )
        };
        let mut body = Vec::new();
        match location {
            Location::Content => {
                let mut values = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                for attribute in attributes {
                    let i = CONTENT_NAMES
                        .iter()
                        .position(|n| *n == attribute.name)
                        .ok_or_else(|| {
                            ErrorCause::owned("Invalid Content Description attribute")
                        })?;
                    if !matches!(
                        attribute.value,
                        Value::Unicode(_) | Value::UnicodeOriginal { .. }
                    ) {
                        return Err(ErrorCause::owned("Content Description values must be text"));
                    }
                    values[i] = encode_value(&attribute.value, location).1;
                }
                for value in &values {
                    let length = u16::try_from(value.len()).map_err(|_| too_many())?;
                    body.extend(length.to_le_bytes());
                }
                for value in values {
                    body.extend(value);
                }
            }
            Location::Extended => {
                let count = u16::try_from(attributes.len()).map_err(|_| too_many())?;
                body.extend(count.to_le_bytes());
                for attribute in attributes {
                    let name = attribute.encoded_name();
                    let (kind, data) = encode_value(&attribute.value, location);
                    body.extend(
                        u16::try_from(name.len())
                            .map_err(|_| too_many())?
                            .to_le_bytes(),
                    );
                    body.extend(name);
                    body.extend(kind.to_le_bytes());
                    body.extend(
                        u16::try_from(data.len())
                            .map_err(|_| too_many())?
                            .to_le_bytes(),
                    );
                    body.extend(data);
                }
            }
            Location::Metadata | Location::Library => {
                let count = u16::try_from(attributes.len()).map_err(|_| too_many())?;
                body.extend(count.to_le_bytes());
                // libavformat before 62 (ffmpeg 8.0) stops reading this object
                // at the first value over 64 KiB, so those values go last.
                let mut attributes = attributes.to_vec();
                attributes.sort_by_key(|attribute| {
                    encode_value(&attribute.value, location).1.len() > usize::from(u16::MAX)
                });
                for attribute in attributes {
                    let name = attribute.encoded_name();
                    let (kind, data) = encode_value(&attribute.value, location);
                    body.extend(attribute.language.to_le_bytes());
                    body.extend(attribute.stream.to_le_bytes());
                    body.extend(
                        u16::try_from(name.len())
                            .map_err(|_| too_many())?
                            .to_le_bytes(),
                    );
                    body.extend(kind.to_le_bytes());
                    body.extend(
                        u32::try_from(data.len())
                            .map_err(|_| too_many())?
                            .to_le_bytes(),
                    );
                    body.extend(name);
                    body.extend(data);
                }
            }
        }
        let id = match location {
            Location::Content => CONTENT_DESCRIPTION,
            Location::Extended => EXTENDED_CONTENT_DESCRIPTION,
            Location::Metadata => METADATA,
            Location::Library => METADATA_LIBRARY,
        };
        Ok(object(id, &body))
    }

    /// Render the complete Header Object for a file whose Data and Index
    /// objects (everything after the header) keep `tail_size` bytes.
    #[cfg(test)]
    pub(super) fn render(&self, tail_size: u64) -> Result<Vec<u8>, String> {
        self.render_diagnostic(tail_size)
            .map_err(|error| error.to_string())
    }

    pub(super) fn render_diagnostic(&self, tail_size: u64) -> Result<Vec<u8>, ErrorCause> {
        let placed = self.placed();
        let group = |location: Location| -> Vec<&Attribute> {
            placed
                .iter()
                .filter(|(l, _)| *l == location)
                .map(|(_, a)| *a)
                .collect()
        };
        let original = |location: Location| -> Vec<&Attribute> {
            self.original
                .iter()
                .filter(|a| a.location == Some(location))
                .collect()
        };
        let mut rendered = std::collections::HashMap::new();
        for location in [
            Location::Content,
            Location::Extended,
            Location::Metadata,
            Location::Library,
        ] {
            let attributes = group(location);
            // None keeps every original object of this kind byte for byte.
            let encoded = if attributes == original(location) {
                None
            } else if attributes.is_empty() {
                Some(Vec::new())
            } else {
                Some(Self::encode_attributes(location, &attributes)?)
            };
            rendered.insert(location, encoded);
        }
        let mut pending = rendered.clone();
        let mut children = Vec::new();
        let mut file_properties = None;
        let mut extension_present = false;
        for child in &self.children {
            match child {
                Child::Raw(bytes) => children.extend_from_slice(bytes),
                Child::FileProperties(bytes) => {
                    file_properties = Some(children.len());
                    children.extend_from_slice(bytes);
                }
                Child::Attributes(location, bytes) => {
                    emit(&rendered, &mut pending, *location, bytes, &mut children)
                }
                Child::Extension {
                    prefix,
                    children: nested,
                } => {
                    extension_present = true;
                    let mut data = Vec::new();
                    for child in nested {
                        match child {
                            Child::Attributes(location, bytes) => {
                                emit(&rendered, &mut pending, *location, bytes, &mut data)
                            }
                            Child::Raw(bytes) | Child::FileProperties(bytes) => {
                                data.extend_from_slice(bytes)
                            }
                            Child::Extension { .. } => {
                                return Err(ErrorCause::owned("Nested ASF header extension"))
                            }
                        }
                    }
                    for location in [Location::Metadata, Location::Library] {
                        if let Some(Some(bytes)) = pending.remove(&location) {
                            data.extend(bytes);
                        }
                    }
                    let mut body = prefix.clone();
                    body.extend(
                        u32::try_from(data.len())
                            .map_err(|_| ErrorCause::owned("ASF header extension is too large"))?
                            .to_le_bytes(),
                    );
                    body.extend(data);
                    children.extend(object(HEADER_EXTENSION, &body));
                }
            }
        }
        for location in [Location::Content, Location::Extended] {
            if let Some(Some(bytes)) = pending.remove(&location) {
                children.extend(bytes);
            }
        }
        if !extension_present {
            let mut data = Vec::new();
            for location in [Location::Metadata, Location::Library] {
                if let Some(Some(bytes)) = pending.remove(&location) {
                    data.extend(bytes);
                }
            }
            if !data.is_empty() {
                let mut body = HEADER_EXTENSION_RESERVED.to_vec();
                body.extend(6u16.to_le_bytes());
                body.extend(
                    u32::try_from(data.len())
                        .map_err(|_| ErrorCause::owned("ASF header extension is too large"))?
                        .to_le_bytes(),
                );
                body.extend(data);
                children.extend(object(HEADER_EXTENSION, &body));
            }
        }
        let count = objects(&children)?.len() as u32;
        let mut header = HEADER.to_vec();
        let size = (children.len() + 30) as u64;
        header.extend(size.to_le_bytes());
        header.extend(count.to_le_bytes());
        header.extend(self.reserved);
        let offset = file_properties
            .ok_or_else(|| ErrorCause::owned("ASF header has no File Properties Object"))?
            + 30;
        header.extend(children);
        // File Properties: the file size follows the 24-byte object header
        // and file ID. Broadcast files store no meaningful size.
        let flags = u32::from_le_bytes(
            header[offset + 88..offset + 92]
                .try_into()
                .map_err(|_| ErrorCause::owned("Malformed File Properties Object"))?,
        );
        if flags & 1 == 0 {
            header[offset + 40..offset + 48].copy_from_slice(&(size + tail_size).to_le_bytes());
        }
        Ok(header)
    }
}

/// Write one attribute object of a kind: the original bytes when the kind is
/// unchanged, otherwise its re-encoding once in place of the first original.
fn emit(
    rendered: &std::collections::HashMap<Location, Option<Vec<u8>>>,
    pending: &mut std::collections::HashMap<Location, Option<Vec<u8>>>,
    location: Location,
    original: &[u8],
    output: &mut Vec<u8>,
) {
    match rendered.get(&location) {
        Some(None) => output.extend_from_slice(original),
        _ => {
            if let Some(Some(bytes)) = pending.remove(&location) {
                output.extend(bytes);
            }
        }
    }
}

fn object(id: Guid, body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend(((body.len() + OBJECT_HEADER) as u64).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}

/// WM/Picture payload: type, data length, MIME and description (NUL-terminated
/// UTF-16LE) and the image bytes.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PictureValue {
    pub(super) kind: u8,
    pub(super) mime: String,
    pub(super) description: String,
    pub(super) data: Vec<u8>,
}
impl PictureValue {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let mut reader = Reader::new(bytes);
        let kind = reader.array::<1>().ok()?[0];
        let length = usize::try_from(reader.u32().ok()?).ok()?;
        let mut text = || -> Option<String> {
            let rest = &bytes[reader.position..];
            let end = rest
                .as_chunks::<2>()
                .0
                .iter()
                .position(|pair| *pair == [0, 0])?
                * 2;
            let value = utf16(&rest[..end]);
            reader.take(end + 2).ok()?;
            Some(value)
        };
        let mime = text()?;
        let description = text()?;
        let data = reader.take(length).ok()?.to_vec();
        (reader.remaining() == 0).then_some(Self {
            kind,
            mime,
            description,
            data,
        })
    }
    #[cfg(test)]
    pub(super) fn encode(&self) -> Result<Vec<u8>, String> {
        self.encode_diagnostic().map_err(|error| error.to_string())
    }

    pub(super) fn encode_diagnostic(&self) -> Result<Vec<u8>, ErrorCause> {
        let mut bytes = vec![self.kind];
        bytes.extend(
            u32::try_from(self.data.len())
                .map_err(|_| ErrorCause::owned("Cover is too large for ASF"))?
                .to_le_bytes(),
        );
        bytes.extend(utf16z(&self.mime));
        bytes.extend(utf16z(&self.description));
        bytes.extend_from_slice(&self.data);
        Ok(bytes)
    }
}
