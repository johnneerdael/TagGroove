use super::header::{utf16z, Asf, Attribute, Location, PictureValue, Value, DATA, HEADER};
use super::*;

const FILE_PROPERTIES: [u8; 16] = [
    0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const CONTENT: [u8; 16] = [
    0x33, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
];
const EXTENDED: [u8; 16] = [
    0x40, 0xA4, 0xD0, 0xD2, 0x07, 0xE3, 0xD2, 0x11, 0x97, 0xF0, 0x00, 0xA0, 0xC9, 0x5E, 0xA8, 0x50,
];
const LIBRARY: [u8; 16] = [
    0x94, 0x1C, 0x23, 0x44, 0x98, 0x94, 0xD1, 0x49, 0xA1, 0x41, 0x1D, 0x13, 0x4E, 0x45, 0x70, 0x54,
];
const METADATA: [u8; 16] = [
    0xEA, 0xCB, 0xF8, 0xC5, 0xAF, 0x5B, 0x77, 0x48, 0x84, 0x67, 0xAA, 0x8C, 0x44, 0xFA, 0x4C, 0xCA,
];
const EXTENSION: [u8; 16] = [
    0xB5, 0x03, 0xBF, 0x5F, 0x2E, 0xA9, 0xCF, 0x11, 0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const EXTENSION_RESERVED: [u8; 16] = [
    0x11, 0xD2, 0xD3, 0xAB, 0xBA, 0xA9, 0xCF, 0x11, 0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const UNKNOWN: [u8; 16] = *b"opentag-unknown!";

fn object(id: [u8; 16], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend(((body.len() + 24) as u64).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn file_properties(file_size: u64) -> Vec<u8> {
    let mut body = vec![7u8; 16];
    body.extend(file_size.to_le_bytes());
    body.extend(0u64.to_le_bytes());
    body.extend(1u64.to_le_bytes());
    body.extend(25_000_000u64.to_le_bytes());
    body.extend(0u64.to_le_bytes());
    body.extend(500u64.to_le_bytes());
    body.extend(2u32.to_le_bytes());
    body.extend([0u8; 8]);
    body.extend(96_000u32.to_le_bytes());
    object(FILE_PROPERTIES, &body)
}
fn extended(items: &[(&str, u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = (items.len() as u16).to_le_bytes().to_vec();
    for (name, kind, data) in items {
        let name = utf16z(name);
        body.extend((name.len() as u16).to_le_bytes());
        body.extend(name);
        body.extend(kind.to_le_bytes());
        body.extend((data.len() as u16).to_le_bytes());
        body.extend(data);
    }
    object(EXTENDED, &body)
}
fn library(id: [u8; 16], items: &[(u16, u16, &str, u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = (items.len() as u16).to_le_bytes().to_vec();
    for (language, stream, name, kind, data) in items {
        let name = utf16z(name);
        body.extend(language.to_le_bytes());
        body.extend(stream.to_le_bytes());
        body.extend((name.len() as u16).to_le_bytes());
        body.extend(kind.to_le_bytes());
        body.extend((data.len() as u32).to_le_bytes());
        body.extend(name);
        body.extend(data);
    }
    object(id, &body)
}
fn content(values: [&str; 5]) -> Vec<u8> {
    let values: Vec<_> = values
        .iter()
        .map(|v| if v.is_empty() { Vec::new() } else { utf16z(v) })
        .collect();
    let mut body = Vec::new();
    for value in &values {
        body.extend((value.len() as u16).to_le_bytes());
    }
    for value in values {
        body.extend(value);
    }
    object(CONTENT, &body)
}
fn extension(children: &[Vec<u8>]) -> Vec<u8> {
    let data = children.concat();
    let mut body = EXTENSION_RESERVED.to_vec();
    body.extend(6u16.to_le_bytes());
    body.extend((data.len() as u32).to_le_bytes());
    body.extend(data);
    object(EXTENSION, &body)
}
/// A file: header with the given children, then a small Data Object.
fn file(children: &[Vec<u8>]) -> Vec<u8> {
    let data = object(DATA, &[0x55; 40]);
    let body = children.concat();
    let header_size = body.len() + 30;
    let mut bytes = HEADER.to_vec();
    bytes.extend((header_size as u64).to_le_bytes());
    bytes.extend((children.len() as u32).to_le_bytes());
    bytes.extend([1, 2]);
    bytes.extend(body);
    let total = (bytes.len() + data.len()) as u64;
    // Patch the File Properties file size wherever the object is.
    if let Some(at) = bytes.windows(16).position(|w| w == FILE_PROPERTIES) {
        bytes[at + 40..at + 48].copy_from_slice(&total.to_le_bytes());
    }
    bytes.extend(data);
    bytes
}
fn parse(bytes: &[u8]) -> Asf {
    Asf::read(&mut &bytes[..], bytes.len() as u64).unwrap()
}
fn tail(bytes: &[u8], asf: &Asf) -> u64 {
    bytes.len() as u64 - asf.header_size
}
fn rich() -> Vec<u8> {
    file(&[
        file_properties(0),
        object(UNKNOWN, b"opaque bytes stay"),
        content(["Title", "Artist", "", "Comment", ""]),
        extended(&[
            ("WM/AlbumTitle", 0, utf16z("Album")),
            ("IsVBR", 2, 1u32.to_le_bytes().to_vec()),
            ("WM/TrackNumber", 3, 7u32.to_le_bytes().to_vec()),
            ("Odd", 9, vec![1, 2, 3]),
        ]),
        extension(&[
            object(UNKNOWN, b"nested"),
            library(
                METADATA,
                &[(0, 2, "StreamThing", 5, 9u16.to_le_bytes().to_vec())],
            ),
            library(
                LIBRARY,
                &[
                    (0, 0, "WM/Genre", 0, utf16z("Rock")),
                    (0, 0, "WM/Genre", 0, utf16z("Pop")),
                    (0, 0, "Id", 6, (1..=16).collect()),
                    (3, 0, "Localized", 0, utf16z("Hallo")),
                    (0, 0, "Flag", 2, 1u16.to_le_bytes().to_vec()),
                ],
            ),
        ]),
    ])
}

#[test]
fn unchanged_headers_render_byte_for_byte() {
    let bytes = rich();
    let asf = parse(&bytes);
    assert_eq!(
        asf.render(tail(&bytes, &asf)).unwrap(),
        bytes[..asf.header_size as usize]
    );
}

#[test]
fn parses_typed_attributes_from_every_object() {
    let asf = parse(&rich());
    let get = |name: &str| {
        asf.attributes
            .iter()
            .filter(|a| a.name == name)
            .map(|a| (a.value.clone(), a.location, a.stream, a.language))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        get("Title"),
        [(
            Value::Unicode("Title".into()),
            Some(Location::Content),
            0,
            0
        )]
    );
    assert_eq!(get("IsVBR")[0].0, Value::Bool(true));
    assert_eq!(get("WM/TrackNumber")[0].0, Value::Dword(7));
    assert_eq!(
        get("Odd")[0].0,
        Value::Other {
            kind: 9,
            data: vec![1, 2, 3]
        }
    );
    assert_eq!(
        get("StreamThing")[0],
        (Value::Word(9), Some(Location::Metadata), 2, 0)
    );
    assert_eq!(get("WM/Genre").len(), 2);
    assert!(matches!(get("Id")[0].0, Value::Guid(_)));
    assert_eq!(get("Localized")[0].3, 3);
    assert_eq!(get("Flag")[0].0, Value::Bool(true));
    // Empty Content Description fields are absent.
    assert!(get("Copyright").is_empty());
    assert_eq!(asf.properties.duration, 2.0);
    assert_eq!(asf.properties.bitrate, 96);

    let fields = local_fields(&asf);
    assert_eq!(fields["TITLE"], ["Title"]);
    assert_eq!(fields["ARTIST"], ["Artist"]);
    assert_eq!(fields["COMMENT"], ["Comment"]);
    assert_eq!(fields["ALBUM"], ["Album"]);
    assert_eq!(fields["TRACK"], ["7"]);
    assert_eq!(fields["GENRE"], ["Rock", "Pop"]);
    assert_eq!(fields["IsVBR"], ["1"]);
    assert_eq!(fields["Id"], ["{04030201-0605-0807-090A-0B0C0D0E0F10}"]);
    assert!(!fields.contains_key("Odd"));
}

#[test]
fn edits_keep_untouched_objects_and_place_new_values_by_rule() {
    let bytes = rich();
    let mut asf = parse(&bytes);
    let large = "x".repeat(40_000);
    let expected = apply_fields(
        &mut asf,
        &Fields::from([
            ("ALBUM".into(), vec!["New album".into()]),
            ("TRACK".into(), vec!["8".into()]),
            ("ARTIST".into(), vec!["A".into(), "B".into()]),
            ("LYRICS".into(), vec![large.clone()]),
            ("COMMENT".into(), vec![]),
        ]),
    )
    .unwrap();
    assert_eq!(expected["TRACK"], ["8"]);
    let header = asf.render(tail(&bytes, &asf)).unwrap();
    let mut file = header.clone();
    file.extend_from_slice(&bytes[asf.header_size as usize..]);
    let written = parse(&file);
    let find = |name: &str| {
        written
            .attributes
            .iter()
            .filter(|a| a.name == name)
            .map(|a| (a.location, a.value.clone()))
            .collect::<Vec<_>>()
    };
    // The track number keeps its DWORD type; repeated values and values over
    // 64 KiB of UTF-16 go to the Metadata Library.
    assert_eq!(
        find("WM/TrackNumber"),
        [(Some(Location::Extended), Value::Dword(8))]
    );
    assert_eq!(
        find("Author"),
        [
            (Some(Location::Library), Value::Unicode("A".into())),
            (Some(Location::Library), Value::Unicode("B".into()))
        ]
    );
    assert_eq!(
        find("LYRICS"),
        [(Some(Location::Library), Value::Unicode(large))]
    );
    assert!(find("Description").is_empty());
    assert_eq!(find("Title")[0].0, Some(Location::Content));
    // Unknown objects (top level and nested) are kept verbatim, and so is the
    // untouched Metadata Object.
    let contains = |needle: &[u8]| header.windows(needle.len()).any(|w| w == needle);
    assert!(contains(&object(UNKNOWN, b"opaque bytes stay")));
    assert!(contains(&object(UNKNOWN, b"nested")));
    assert!(contains(&library(
        METADATA,
        &[(0, 2, "StreamThing", 5, 9u16.to_le_bytes().to_vec())]
    )));
    let fields = local_fields(&written);
    assert_eq!(fields["ARTIST"], ["A", "B"]);
    assert!(!fields.contains_key("COMMENT"));
    assert_eq!(fields["GENRE"], ["Rock", "Pop"]);
    assert_eq!(fields["Localized"], ["Hallo"]);
    // File Properties carries the new file size.
    let at = file.windows(16).position(|w| w == FILE_PROPERTIES).unwrap();
    assert_eq!(
        u64::from_le_bytes(file[at + 40..at + 48].try_into().unwrap()),
        file.len() as u64
    );
}

#[test]
fn creates_a_header_extension_and_converts_boolean_sizes() {
    let bytes = file(&[file_properties(0), content(["T", "", "", "", ""])]);
    let mut asf = parse(&bytes);
    asf.attributes.push(Attribute {
        language: 2,
        ..Attribute::new("Flag", Value::Bool(true))
    });
    asf.attributes
        .push(Attribute::new("Small", Value::Bool(true)));
    let header = asf.render(tail(&bytes, &asf)).unwrap();
    let mut file = header.clone();
    file.extend_from_slice(&bytes[asf.header_size as usize..]);
    let written = parse(&file);
    let flag = written
        .attributes
        .iter()
        .find(|a| a.name == "Flag")
        .unwrap();
    assert_eq!(
        (flag.location, &flag.value),
        (Some(Location::Library), &Value::Bool(true))
    );
    let small = written
        .attributes
        .iter()
        .find(|a| a.name == "Small")
        .unwrap();
    assert_eq!(small.location, Some(Location::Extended));
    assert!(header.windows(16).any(|w| w == EXTENSION));
    // Header object count covers the new extension and ECD objects.
    assert_eq!(u32::from_le_bytes(header[24..28].try_into().unwrap()), 4);
}

#[test]
fn values_keep_their_type_only_when_they_read_back_identically() {
    let rating = |text: &str| parse_value(text, None, true).unwrap();
    assert_eq!(rating("3"), Value::Dword(50));
    assert_eq!(rating("*****"), Value::Dword(99));
    assert!(parse_value("2.5", None, true).is_err());
    let dword = Some(&Value::Dword(1));
    assert_eq!(parse_value("12", dword, false).unwrap(), Value::Dword(12));
    assert_eq!(
        parse_value("012", dword, false).unwrap(),
        Value::Unicode("012".into())
    );
    assert_eq!(
        parse_value("3/12", dword, false).unwrap(),
        Value::Unicode("3/12".into())
    );
    let id = "{8CABDCA1-A947-11CF-8EE4-00C00C205365}";
    let guid = parse_value(id, Some(&Value::Guid([0; 16])), false).unwrap();
    assert_eq!(guid, Value::Guid(FILE_PROPERTIES));
    assert_eq!(display(&Attribute::new("x", guid)).unwrap(), id);
    for (stored, shown) in [
        (0, "0"),
        (1, "1"),
        (25, "2"),
        (50, "3"),
        (75, "4"),
        (99, "5"),
    ] {
        let attribute = Attribute::new(RATING, Value::Dword(stored));
        assert_eq!(display(&attribute).unwrap(), shown);
    }
}

#[test]
fn pictures_roundtrip_and_reject_malformed_payloads() {
    let picture = PictureValue {
        kind: 3,
        mime: "image/png".into(),
        description: "Front ✓".into(),
        data: vec![1, 2, 3, 4],
    };
    let encoded = picture.encode().unwrap();
    assert_eq!(PictureValue::parse(&encoded).unwrap(), picture);
    assert!(PictureValue::parse(&encoded[..encoded.len() - 1]).is_none());
    let mut extra = encoded.clone();
    extra.push(0);
    assert!(PictureValue::parse(&extra).is_none());
}

#[test]
fn malformed_headers_are_rejected() {
    let bytes = rich();
    let mut truncated = bytes.clone();
    truncated.truncate(100);
    assert!(Asf::read(&mut &truncated[..], truncated.len() as u64).is_err());
    let mut oversized = bytes.clone();
    oversized[16..24].copy_from_slice(&(bytes.len() as u64 + 1).to_le_bytes());
    assert!(Asf::read(&mut &oversized[..], oversized.len() as u64).is_err());
    let no_properties = file(&[content(["T", "", "", "", ""])]);
    assert!(Asf::read(&mut &no_properties[..], no_properties.len() as u64).is_err());
}
