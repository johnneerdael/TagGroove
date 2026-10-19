//! Windows Media (ASF) through the public API. A hand-built file covers every
//! attribute object and value type without external tools; files encoded by
//! ffmpeg, when it is installed, are also checked with ffprobe and decoded.
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{fs, path::Path, process::Command};
use tag_core::*;

const fn guid(a: u32, b: u16, c: u16, d: [u8; 8]) -> [u8; 16] {
    let (a, b, c) = (a.to_le_bytes(), b.to_le_bytes(), c.to_le_bytes());
    [
        a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d[0], d[1], d[2], d[3], d[4], d[5], d[6],
        d[7],
    ]
}
const A: [u8; 8] = [0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C];
const B: [u8; 8] = [0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65];
const C: [u8; 8] = [0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B];
const HEADER: [u8; 16] = guid(0x75B2_2630, 0x668E, 0x11CF, A);
const DATA: [u8; 16] = guid(0x75B2_2636, 0x668E, 0x11CF, A);
const CONTENT: [u8; 16] = guid(0x75B2_2633, 0x668E, 0x11CF, A);
const FILE_PROPERTIES: [u8; 16] = guid(
    0x8CAB_DCA1,
    0xA947,
    0x11CF,
    [0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65],
);
const STREAM_PROPERTIES: [u8; 16] = guid(0xB7DC_0791, 0xA9B7, 0x11CF, B);
const EXTENSION: [u8; 16] = guid(
    0x5FBF_03B5,
    0xA92E,
    0x11CF,
    [0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65],
);
const EXTENSION_RESERVED: [u8; 16] = guid(0xABD3_D211, 0xA9BA, 0x11CF, B);
const EXTENDED: [u8; 16] = guid(
    0xD2D0_A440,
    0xE307,
    0x11D2,
    [0x97, 0xF0, 0x00, 0xA0, 0xC9, 0x5E, 0xA8, 0x50],
);
const METADATA: [u8; 16] = guid(
    0xC5F8_CBEA,
    0x5BAF,
    0x4877,
    [0x84, 0x67, 0xAA, 0x8C, 0x44, 0xFA, 0x4C, 0xCA],
);
const LIBRARY: [u8; 16] = guid(
    0x4423_1C94,
    0x9498,
    0x49D1,
    [0xA1, 0x41, 0x1D, 0x13, 0x4E, 0x45, 0x70, 0x54],
);
const AUDIO: [u8; 16] = guid(0xF869_9E40, 0x5B4D, 0x11CF, C);
const NO_ERROR_CORRECTION: [u8; 16] = guid(0x20FB_5700, 0x5B55, 0x11CF, C);
const SIMPLE_INDEX: [u8; 16] = guid(
    0x3300_0890,
    0xE5B1,
    0x11CF,
    [0x89, 0xF4, 0x00, 0xA0, 0xC9, 0x03, 0x49, 0xCB],
);
const UNKNOWN: [u8; 16] = *b"unknown-object!!";
const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=";

fn w(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}
fn object(id: [u8; 16], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend(((body.len() + 24) as u64).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn picture(kind: u8, mime: &str, description: &str, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![kind];
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(w(mime));
    bytes.extend(w(description));
    bytes.extend_from_slice(data);
    bytes
}

/// A complete ASF file built from the specification: every attribute object,
/// every value type, an unknown object, a Data Object and a Simple Index.
fn hand_built() -> Vec<u8> {
    let png = STANDARD.decode(PNG).unwrap();
    let mut properties = vec![9u8; 16];
    properties.extend(0u64.to_le_bytes()); // file size, patched below
    properties.extend(0u64.to_le_bytes());
    properties.extend(1u64.to_le_bytes());
    properties.extend(40_000_000u64.to_le_bytes()); // 4 s including preroll
    properties.extend(0u64.to_le_bytes());
    properties.extend(1000u64.to_le_bytes()); // 1 s preroll
    properties.extend(2u32.to_le_bytes());
    properties.extend(64u32.to_le_bytes());
    properties.extend(64u32.to_le_bytes());
    properties.extend(128_000u32.to_le_bytes());

    let mut stream = AUDIO.to_vec();
    stream.extend(NO_ERROR_CORRECTION);
    stream.extend(0u64.to_le_bytes());
    stream.extend(18u32.to_le_bytes());
    stream.extend(0u32.to_le_bytes());
    stream.extend(1u16.to_le_bytes());
    stream.extend(0u32.to_le_bytes());
    for value in [0x0161u16, 2] {
        stream.extend(value.to_le_bytes());
    }
    stream.extend(44_100u32.to_le_bytes());
    stream.extend(16_000u32.to_le_bytes());
    for value in [4096u16, 16, 0] {
        stream.extend(value.to_le_bytes());
    }

    let fields = [
        w("Hand Title"),
        w("Hand Artist"),
        vec![],
        w("Hand comment"),
        vec![],
    ];
    let mut content = Vec::new();
    for field in &fields {
        content.extend((field.len() as u16).to_le_bytes());
    }
    content.extend(fields.concat());

    let items: Vec<(&str, u16, Vec<u8>)> = vec![
        ("WM/AlbumTitle", 0, w("Hand Album")),
        ("WM/TrackNumber", 3, 3u32.to_le_bytes().to_vec()),
        ("IsVBR", 2, 1u32.to_le_bytes().to_vec()),
        ("WM/Year", 0, w("2001")),
        ("WM/Picture", 1, picture(3, "image/png", "front", &png)),
        ("Blob", 1, vec![1, 2, 3]),
        ("Big", 4, (1u64 << 40).to_le_bytes().to_vec()),
        ("Small", 5, 7u16.to_le_bytes().to_vec()),
        ("WM/SharedUserRating", 3, 75u32.to_le_bytes().to_vec()),
    ];
    let mut extended = (items.len() as u16).to_le_bytes().to_vec();
    for (name, kind, data) in items {
        extended.extend((w(name).len() as u16).to_le_bytes());
        extended.extend(w(name));
        extended.extend(kind.to_le_bytes());
        extended.extend((data.len() as u16).to_le_bytes());
        extended.extend(data);
    }

    let records = |items: &[(u16, u16, &str, u16, Vec<u8>)]| {
        let mut body = (items.len() as u16).to_le_bytes().to_vec();
        for (language, stream, name, kind, data) in items {
            body.extend(language.to_le_bytes());
            body.extend(stream.to_le_bytes());
            body.extend((w(name).len() as u16).to_le_bytes());
            body.extend(kind.to_le_bytes());
            body.extend((data.len() as u32).to_le_bytes());
            body.extend(w(name));
            body.extend(data);
        }
        body
    };
    let metadata = object(
        METADATA,
        &records(&[(0, 1, "StreamAttr", 3, 5u32.to_le_bytes().to_vec())]),
    );
    let library = object(
        LIBRARY,
        &records(&[
            (0, 0, "WM/Genre", 0, w("Rock")),
            (0, 0, "WM/Genre", 0, w("Pop")),
            (0, 0, "WM/MediaClassPrimaryID", 6, (0..16).collect()),
            (1, 0, "Localized", 0, w("Grüße")),
        ]),
    );
    let mut extension = EXTENSION_RESERVED.to_vec();
    extension.extend(6u16.to_le_bytes());
    extension.extend(((metadata.len() + library.len()) as u32).to_le_bytes());
    extension.extend(metadata);
    extension.extend(library);

    let children = [
        object(FILE_PROPERTIES, &properties),
        object(STREAM_PROPERTIES, &stream),
        object(CONTENT, &content),
        object(EXTENDED, &extended),
        object(UNKNOWN, b"opaque payload"),
        object(EXTENSION, &extension),
    ];
    let mut data = vec![7u8; 16];
    data.extend(1u64.to_le_bytes());
    data.extend([1, 1]);
    data.extend((0..64).map(|i| i as u8));
    let mut index = vec![7u8; 16];
    index.extend(10_000_000u64.to_le_bytes());
    index.extend(1u32.to_le_bytes());
    index.extend(1u32.to_le_bytes());
    index.extend(0u32.to_le_bytes());
    index.extend(1u16.to_le_bytes());

    let body = children.concat();
    let mut file = HEADER.to_vec();
    file.extend(((body.len() + 30) as u64).to_le_bytes());
    file.extend((children.len() as u32).to_le_bytes());
    file.extend([1, 2]);
    file.extend(body);
    file.extend(object(DATA, &data));
    file.extend(object(SIMPLE_INDEX, &index));
    let size = file.len() as u64;
    let at = file.windows(16).position(|w| w == FILE_PROPERTIES).unwrap();
    file[at + 40..at + 48].copy_from_slice(&size.to_le_bytes());
    file
}

fn header_size(bytes: &[u8]) -> usize {
    u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize
}
/// Everything after the Header Object: the Data and Index objects.
fn media(bytes: &[u8]) -> &[u8] {
    &bytes[header_size(bytes)..]
}
fn header_objects(bytes: &[u8]) -> Vec<[u8; 16]> {
    let (mut at, end, mut ids) = (30, header_size(bytes), Vec::new());
    while at < end {
        ids.push(bytes[at..at + 16].try_into().unwrap());
        at += u64::from_le_bytes(bytes[at + 16..at + 24].try_into().unwrap()) as usize;
    }
    assert_eq!(at, end, "header objects fill the header exactly");
    ids
}
fn assert_consistent(bytes: &[u8]) {
    let ids = header_objects(bytes);
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize,
        ids.len()
    );
    let at = bytes
        .windows(16)
        .position(|w| w == FILE_PROPERTIES)
        .unwrap();
    assert_eq!(
        u64::from_le_bytes(bytes[at + 40..at + 48].try_into().unwrap()),
        bytes.len() as u64,
        "File Properties file size"
    );
    assert_eq!(&bytes[header_size(bytes)..header_size(bytes) + 16], DATA);
}
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
fn fields(items: &[(&str, &[&str])]) -> Fields {
    items
        .iter()
        .map(|(k, v)| ((*k).into(), v.iter().map(|s| (*s).into()).collect()))
        .collect()
}
fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn hand_built_asf_reads_fields_properties_and_covers() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write(tmp.path(), "hand.wma", &hand_built());
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "WMA");
    let get = |name: &str| entry.fields.get(name).cloned().unwrap_or_default();
    assert_eq!(get("TITLE"), ["Hand Title"]);
    assert_eq!(get("ARTIST"), ["Hand Artist"]);
    assert_eq!(get("COMMENT"), ["Hand comment"]);
    assert_eq!(get("ALBUM"), ["Hand Album"]);
    assert_eq!(get("TRACK"), ["3"]);
    assert_eq!(get("YEAR"), ["2001"]);
    assert_eq!(get("GENRE"), ["Rock", "Pop"]);
    assert_eq!(get("RATING WMP"), ["4"]);
    assert_eq!(get("IsVBR"), ["1"]);
    assert_eq!(get("Big"), ["1099511627776"]);
    assert_eq!(get("Small"), ["7"]);
    assert_eq!(get("StreamAttr"), ["5"]);
    assert_eq!(get("Localized"), ["Grüße"]);
    assert_eq!(
        get("WM/MediaClassPrimaryID"),
        ["{03020100-0504-0706-0809-0A0B0C0D0E0F}"]
    );
    assert!(!entry.fields.contains_key("Blob"));
    assert!(!entry.fields.contains_key("WM/Picture"));
    assert_eq!(entry.duration, 3.0);
    assert_eq!(
        (entry.bitrate, entry.sample_rate, entry.channels),
        (128, 44_100, 2)
    );
    assert_eq!(entry.technical["_CODEC"], ["Windows Media Audio"]);
    assert_eq!(entry.technical["_LENGTH"], ["0:03"]);
    assert_eq!(entry.technical["_MODE"], ["Stereo"]);
    assert_eq!(entry.technical["_TAG"], ["WMA"]);
    assert_eq!(entry.covers.len(), 1);
    let cover = &entry.covers[0];
    assert_eq!(
        (
            cover.mime.as_str(),
            cover.picture_type.as_str(),
            cover.description.as_str()
        ),
        ("image/png", "CoverFront", "front")
    );
    assert_eq!((cover.width, cover.height), (Some(1), Some(1)));
    assert_eq!(
        STANDARD.decode(&cover.data_base64).unwrap(),
        STANDARD.decode(PNG).unwrap()
    );
    assert_eq!(entry.technical["_COVERS"], ["1"]);
}

#[test]
fn field_writes_rebuild_only_the_header() {
    let tmp = tempfile::tempdir().unwrap();
    let original = hand_built();
    let path = write(tmp.path(), "hand.wma", &original);
    let large = "長".repeat(40_000);
    let updates = fields(&[
        ("TITLE", &["Ünïcode 猫 title"]),
        ("ALBUM", &["New album"]),
        ("TRACK", &["4"]),
        ("ARTIST", &["First", "Second"]),
        ("COMMENT", &[]),
        ("RATING WMP", &["**"]),
        ("UNSYNCEDLYRICS", &[large.as_str()]),
        ("MY CUSTOM", &["custom ✓"]),
        ("IsVBR", &["0"]),
    ]);
    let entry = write_fields(&path, &updates).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original), "data and index bytes");
    assert_consistent(&bytes);
    assert!(contains(&bytes, &object(UNKNOWN, b"opaque payload")));
    for (key, values) in &updates {
        let expected: Vec<String> = match key.as_str() {
            "RATING WMP" => vec!["2".into()],
            _ => values.clone(),
        };
        if expected.is_empty() {
            assert!(!entry.fields.contains_key(key), "{key} removed");
        } else {
            assert_eq!(entry.fields[key], expected, "{key}");
        }
    }
    // Untouched values of every kind survive.
    for (key, value) in [
        ("YEAR", "2001"),
        ("Big", "1099511627776"),
        ("Small", "7"),
        ("StreamAttr", "5"),
        ("Localized", "Grüße"),
    ] {
        assert_eq!(entry.fields[key], [value], "{key}");
    }
    assert_eq!(entry.fields["GENRE"], ["Rock", "Pop"]);
    assert_eq!(entry.covers.len(), 1);
    // The value over 64 KiB is in the Metadata Library, not the ECD.
    let library_at = bytes.windows(16).position(|w| w == LIBRARY).unwrap();
    let extended_at = bytes.windows(16).position(|w| w == EXTENDED).unwrap();
    let lyrics = w("WM/Lyrics");
    let lyrics_at = bytes
        .windows(lyrics.len())
        .position(|w| w == lyrics)
        .unwrap();
    assert!(lyrics_at > library_at && lyrics_at > extended_at);
    assert_eq!(read_entry(&path).unwrap().fields, entry.fields);

    // Restoring the original values gives the original metadata back.
    let restore = fields(&[
        ("TITLE", &["Hand Title"]),
        ("ALBUM", &["Hand Album"]),
        ("TRACK", &["3"]),
        ("ARTIST", &["Hand Artist"]),
        ("COMMENT", &["Hand comment"]),
        ("RATING WMP", &["4"]),
        ("UNSYNCEDLYRICS", &[]),
        ("MY CUSTOM", &[]),
        ("IsVBR", &["1"]),
    ]);
    let restored = write_fields(&path, &restore).unwrap();
    assert_eq!(
        restored.fields,
        read_entry(&write(tmp.path(), "o.wma", &original))
            .unwrap()
            .fields
    );
    assert_eq!(media(&fs::read(&path).unwrap()), media(&original));
}

#[test]
fn covers_add_replace_and_remove_including_large_pictures() {
    let tmp = tempfile::tempdir().unwrap();
    let original = hand_built();
    let path = write(tmp.path(), "hand.wma", &original);
    // Noise does not compress, so the PNG exceeds the 64 KiB ECD value limit.
    let mut seed = 0x1234_5678u32;
    let noise = image::RgbImage::from_fn(200, 200, |_, _| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        image::Rgb([seed as u8, (seed >> 8) as u8, (seed >> 16) as u8])
    });
    let image_path = tmp.path().join("large.png");
    noise.save(&image_path).unwrap();
    let large = fs::read(&image_path).unwrap();
    assert!(large.len() > 65_535);

    let entry = add_cover(&path, &image_path).unwrap();
    assert_eq!(entry.covers.len(), 2);
    assert_eq!(
        STANDARD.decode(&entry.covers[1].data_base64).unwrap(),
        large
    );
    assert_eq!(
        (entry.covers[1].width, entry.covers[1].height),
        (Some(200), Some(200))
    );
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original));
    assert_consistent(&bytes);
    assert_eq!(entry.fields["TITLE"], ["Hand Title"]);

    let entry =
        set_cover_properties_summary(&path, Some("CoverBack"), Some("back ✓"), Some("CoverFront"))
            .unwrap();
    assert!(entry
        .covers
        .iter()
        .all(|c| c.picture_type == "CoverBack" && c.description == "back ✓"));
    let entry = remove_cover(&path, 0).unwrap();
    assert_eq!(entry.covers.len(), 1);
    assert_eq!(
        STANDARD.decode(&entry.covers[0].data_base64).unwrap(),
        large
    );
    assert!(remove_cover(&path, 5).is_err());
    let entry = remove_all_covers_summary(&path).unwrap();
    assert!(entry.covers.is_empty());
    assert_eq!(entry.fields["ALBUM"], ["Hand Album"]);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original));
    assert_consistent(&bytes);
}

#[test]
fn removing_the_tag_keeps_stream_attributes_and_media() {
    let tmp = tempfile::tempdir().unwrap();
    let original = hand_built();
    let path = write(tmp.path(), "hand.wma", &original);
    let entry = remove_tags(&path).unwrap();
    assert_eq!(
        entry.fields,
        Fields::from([("StreamAttr".into(), vec!["5".into()])])
    );
    assert!(entry.covers.is_empty());
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original));
    assert_consistent(&bytes);
    assert!(contains(&bytes, &object(UNKNOWN, b"opaque payload")));
    assert!(!header_objects(&bytes).contains(&CONTENT));
    assert!(!header_objects(&bytes).contains(&EXTENDED));
}

#[test]
fn asf_files_are_scanned_and_honour_wma_user_mappings() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a.wma", &hand_built());
    write(tmp.path(), "b.asf", &hand_built());
    write(tmp.path(), "not-audio.txt", b"text");
    let scan = scan_paths(&[tmp.path().to_string_lossy().into_owned()], false);
    assert!(scan.errors.is_empty(), "{:?}", scan.errors);
    assert_eq!(scan.entries.len(), 2);
    assert!(scan
        .entries
        .iter()
        .all(|e| e.format == "WMA" && e.covers[0].data_base64.is_empty()));

    let path = tmp.path().join("a.wma");
    let rules = [UserFieldMapping {
        tag_type: "WMA".into(),
        source: "Small".into(),
        target: "MOVEMENT".into(),
    }];
    let entry = read_entry_with_mappings(&path, &rules).unwrap();
    assert_eq!(entry.fields["MOVEMENT"], ["7"]);
    let entry =
        write_fields_with_mappings(&path, &fields(&[("MOVEMENT", &["9"])]), &rules).unwrap();
    assert_eq!(entry.fields["MOVEMENT"], ["9"]);
    assert_eq!(read_entry(&path).unwrap().fields["Small"], ["9"]);
}

#[test]
fn invalid_writes_leave_the_file_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let original = hand_built();
    let path = write(tmp.path(), "hand.wma", &original);
    assert_eq!(
        write_fields(&path, &fields(&[("Author", &["Conflict"])])).unwrap_err(),
        "Author has no separate ASF attribute; Author holds ARTIST"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    for updates in [
        fields(&[("WM/Picture", &["x"])]),
        fields(&[("DESCRIPTION", &["clashes with COMMENT"])]),
        fields(&[("RATING WMP", &["7"])]),
    ] {
        assert!(write_fields(&path, &updates).is_err(), "{updates:?}");
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    // Removing fields that cannot exist is a no-op.
    let entry = write_fields(&path, &fields(&[("DESCRIPTION", &[]), ("WM/Picture", &[])])).unwrap();
    assert_eq!(entry.covers.len(), 1);
    assert_eq!(entry.fields["COMMENT"], ["Hand comment"]);
    let mut truncated = original.clone();
    truncated.truncate(200);
    let broken = write(tmp.path(), "broken.wma", &truncated);
    assert!(read_entry(&broken).is_err());
    assert!(write_fields(&broken, &fields(&[("TITLE", &["x"])])).is_err());
    assert_eq!(fs::read(&broken).unwrap(), truncated);
}

fn tools() -> bool {
    let ok = Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok();
    if !ok {
        eprintln!("Skipping ffmpeg-encoded WMA checks: ffmpeg and ffprobe are required");
    }
    ok
}
fn probe_tags(path: &Path) -> serde_json::Value {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
/// The libavformat major version reported by `ffprobe -version`.
fn libavformat_major() -> Option<u32> {
    let output = Command::new("ffprobe").arg("-version").output().ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("libavformat"))?
        .trim()
        .split('.')
        .next()?
        .trim()
        .parse()
        .ok()
}
fn decodes(path: &Path) {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-f", "null", "-"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ffmpeg_encoded_wma_roundtrips_and_stays_decodable() {
    if !tools() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("encoded.wma");
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
        ])
        .args(["-ac", "2", "-c:a", "wmav2", "-b:a", "96k"])
        .args([
            "-metadata",
            "title=Encoded title",
            "-metadata",
            "artist=Encoded artist",
        ])
        .args(["-metadata", "album=Encoded album", "-metadata", "track=5"])
        .args([
            "-metadata",
            "genre=Jazz",
            "-metadata",
            "WM/Composer=Composer",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let original = fs::read(&path).unwrap();

    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "WMA");
    assert_eq!(entry.fields["TITLE"], ["Encoded title"]);
    assert_eq!(entry.fields["ARTIST"], ["Encoded artist"]);
    assert_eq!(entry.fields["ALBUM"], ["Encoded album"]);
    assert_eq!(entry.fields["TRACK"], ["5"]);
    assert_eq!(entry.fields["GENRE"], ["Jazz"]);
    assert_eq!(entry.fields["COMPOSER"], ["Composer"]);
    assert_eq!((entry.sample_rate, entry.channels), (44_100, 2));
    assert!((entry.duration - 2.0).abs() < 0.2, "{}", entry.duration);
    assert!(entry.bitrate > 0);
    assert!(entry.technical["_CODEC"][0].starts_with("Windows Media Audio"));

    let updates = fields(&[
        ("TITLE", &["Nouveau titre 猫"]),
        ("ARTIST", &["Solo"]),
        ("ALBUMARTIST", &["Album artist"]),
        ("YEAR", &["1999"]),
        ("GENRE", &[]),
        ("COMPOSER", &["Ünïcode composer"]),
    ]);
    write_fields(&path, &updates).unwrap();
    let png = tmp.path().join("cover.png");
    fs::write(&png, STANDARD.decode(PNG).unwrap()).unwrap();
    let entry = add_cover(&path, &png).unwrap();
    assert_eq!(entry.covers.len(), 1);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original));
    assert_consistent(&bytes);

    decodes(&path);
    let probed = probe_tags(&path);
    let tags = &probed["format"]["tags"];
    assert_eq!(tags["title"], "Nouveau titre 猫");
    assert_eq!(tags["artist"], "Solo");
    assert_eq!(tags["album_artist"], "Album artist");
    assert_eq!(tags["WM/Year"], "1999", "{tags}");
    assert_eq!(tags["composer"], "Ünïcode composer");
    assert_eq!(tags["album"], "Encoded album");
    assert!(tags.get("genre").is_none());
    let streams = probed["streams"].as_array().unwrap();
    assert!(
        streams
            .iter()
            .any(|s| s["disposition"]["attached_pic"] == 1),
        "ffprobe sees the WM/Picture cover: {streams:?}"
    );
    let audio = streams.iter().find(|s| s["codec_type"] == "audio").unwrap();
    assert_eq!(audio["codec_name"], "wmav2");

    remove_tags(&path).unwrap();
    decodes(&path);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original));
    assert!(probe_tags(&path)["format"]["tags"].get("title").is_none());
}

#[test]
fn ffmpeg_reads_values_moved_to_the_metadata_library() {
    if !tools() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("library.wma");
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "sine=duration=1"])
        .args(["-c:a", "wmav2", "-metadata", "title=Short"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let original = fs::read(&path).unwrap();
    let long = "L".repeat(70_000);
    let entry = write_fields(&path, &fields(&[("MY LONG", &[long.as_str()])])).unwrap();
    assert_eq!(entry.fields["MY LONG"], [long.as_str()]);
    assert_eq!(media(&fs::read(&path).unwrap()), media(&original));
    decodes(&path);
    let tags = &probe_tags(&path)["format"]["tags"];
    assert_eq!(tags["title"], "Short");
    // libavformat before 62 (ffmpeg 8.0) rejects Metadata Library values over
    // 65535 bytes and skips the rest of that object; newer versions read it.
    match libavformat_major() {
        Some(major) if major >= 62 => assert_eq!(tags["MY LONG"], long.as_str()),
        major => eprintln!("libavformat {major:?} cannot read values over 64 KiB"),
    }
    // Repeated values also live in the Metadata Library; ffmpeg keeps one.
    // They are written before the oversized value, so every ffmpeg version
    // sees them, even one that stops at the oversized value.
    let entry = write_fields(&path, &fields(&[("ARTIST", &["One", "Two"])])).unwrap();
    assert_eq!(entry.fields["ARTIST"], ["One", "Two"]);
    assert_eq!(entry.fields["MY LONG"], [long.as_str()]);
    assert_eq!(media(&fs::read(&path).unwrap()), media(&original));
    decodes(&path);
    assert_eq!(probe_tags(&path)["format"]["tags"]["artist"], "Two");
}

// These records are deliberately admitted noncanonical text, not valid-Unicode
// or original-editor fixtures. Observe raw descriptor boundaries independently
// of the production ASF parser, which exposes a lossy text view of their data.
#[derive(Debug, PartialEq)]
struct RawExtendedDescriptor {
    name: String,
    kind: u16,
    value: Vec<u8>,
    raw: Vec<u8>,
    value_start: usize,
}

fn bounded_asf_children(bytes: &[u8]) -> Vec<std::ops::Range<usize>> {
    assert!(bytes.starts_with(&HEADER));
    let end = header_size(bytes);
    assert!((30..=bytes.len()).contains(&end));
    let count = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
    let mut at = 30;
    let mut children = Vec::new();
    while at < end {
        assert!(end - at >= 24);
        let size = usize::try_from(u64::from_le_bytes(
            bytes[at + 16..at + 24].try_into().unwrap(),
        ))
        .unwrap();
        assert!(size >= 24 && size <= end - at);
        children.push(at..at + size);
        at += size;
    }
    assert_eq!(at, end);
    assert_eq!(children.len(), count);
    children
}

fn extended_descriptor_observation(bytes: &[u8]) -> Vec<RawExtendedDescriptor> {
    let mut found = Vec::new();
    for child in bounded_asf_children(bytes) {
        if bytes[child.start..child.start + 16] != EXTENDED {
            continue;
        }
        let mut at = child.start + 24;
        let word = |at: usize| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap());
        let count = word(at);
        at += 2;
        for _ in 0..count {
            let start = at;
            assert!(child.end - at >= 2);
            let name_len = usize::from(word(at));
            at += 2;
            assert!(name_len >= 2 && name_len % 2 == 0 && name_len <= child.end - at);
            let name = bytes[at..at + name_len]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes(*b))
                .collect::<Vec<_>>();
            assert_eq!(name.last(), Some(&0));
            let name = String::from_utf16(&name[..name.len() - 1]).unwrap();
            at += name_len;
            assert!(child.end - at >= 4);
            let kind = word(at);
            let length = usize::from(word(at + 2));
            at += 4;
            assert!(length <= child.end - at);
            let value_start = at;
            at += length;
            found.push(RawExtendedDescriptor {
                name,
                kind,
                value: bytes[value_start..at].to_vec(),
                raw: bytes[start..at].to_vec(),
                value_start,
            });
        }
        assert_eq!(at, child.end);
    }
    found
}

fn noncanonical_descriptor(name: &str, value: &[u8]) -> Vec<u8> {
    let name = w(name);
    let mut descriptor = (name.len() as u16).to_le_bytes().to_vec();
    descriptor.extend(name);
    descriptor.extend(0u16.to_le_bytes());
    descriptor.extend((value.len() as u16).to_le_bytes());
    descriptor.extend(value);
    descriptor
}

fn inject_noncanonical_extended_text(original: &[u8]) -> Vec<u8> {
    let mut children = Vec::new();
    let mut injected = 0;
    for child in bounded_asf_children(original) {
        let bytes = &original[child.clone()];
        if bytes[..16] == EXTENDED {
            let count = u16::from_le_bytes(bytes[24..26].try_into().unwrap());
            let mut body = count.checked_add(2).unwrap().to_le_bytes().to_vec();
            body.extend(&bytes[26..]);
            body.extend(noncanonical_descriptor(
                "KeepSurrogate",
                &[0x00, 0xd8, 0, 0],
            ));
            body.extend(noncanonical_descriptor(
                "KeepOddByte",
                &[b'A', 0, 0, 0, 0x55],
            ));
            children.extend(object(EXTENDED, &body));
            injected += 1;
        } else {
            children.extend(bytes);
        }
    }
    assert_eq!(injected, 1, "encoded fixture needs one Extended object");
    let mut bytes = original[..30].to_vec();
    bytes[16..24].copy_from_slice(&((children.len() + 30) as u64).to_le_bytes());
    bytes.extend(children);
    bytes.extend(media(original));
    let size = bytes.len() as u64;
    for child in bounded_asf_children(&bytes) {
        if bytes[child.start..child.start + 16] == FILE_PROPERTIES {
            assert!(child.len() >= 104);
            bytes[child.start + 40..child.start + 48].copy_from_slice(&size.to_le_bytes());
        }
    }
    bytes
}

fn encoded_noncanonical_asf(path: &Path) {
    let output = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=523:duration=0.25",
            "-ac",
            "1",
            "-c:a",
            "wmav2",
            "-b:a",
            "64k",
            "-metadata",
            "title=Original title",
            "-metadata",
            "album=Original album",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let original = fs::read(path).unwrap();
    let injected = inject_noncanonical_extended_text(&original);
    assert_eq!(media(&injected), media(&original));
    fs::write(path, injected).unwrap();
}

fn watched_descriptors(bytes: &[u8]) -> Vec<RawExtendedDescriptor> {
    let watched = extended_descriptor_observation(bytes)
        .into_iter()
        .filter(|descriptor| matches!(descriptor.name.as_str(), "KeepSurrogate" | "KeepOddByte"))
        .collect::<Vec<_>>();
    assert_eq!(watched.len(), 2);
    watched
}

fn admitted_noncanonical_asf_edit(field: &str, explicit: bool) {
    if !tools() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("admitted.wma");
    encoded_noncanonical_asf(&path);
    let before = fs::read(&path).unwrap();
    let observed = watched_descriptors(&before);
    assert_eq!(observed[0].kind, 0);
    assert_eq!(observed[0].value, [0x00, 0xd8, 0, 0]);
    assert_eq!(observed[1].kind, 0);
    assert_eq!(observed[1].value, [b'A', 0, 0, 0, 0x55]);
    let admitted = read_entry(&path).expect("current reader must admit the noncanonical records");
    assert_eq!(admitted.fields["KeepSurrogate"], ["\u{fffd}"]);
    assert_eq!(admitted.fields["KeepOddByte"], ["A"]);

    let mut altered = before.clone();
    altered[observed[1].value_start + 4] ^= 1;
    let altered = watched_descriptors(&altered);
    assert_ne!(
        altered[1].raw, observed[1].raw,
        "observer missed a changed discarded byte"
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "observer control touched disk"
    );

    let mut journal =
        history::Journal::open(&dir.path().join("journal"), history::Limits::default()).unwrap();
    let capture = journal.begin(
        "Admitted ASF edit",
        None,
        &[path.to_string_lossy().into_owned()],
    );
    let updates = if explicit {
        fields(&[
            ("KeepSurrogate", &["Replacement ✓"]),
            ("KeepOddByte", &["Second replacement"]),
        ])
    } else {
        fields(&[(field, &["Unrelated edit"])])
    };
    write_fields(&path, &updates).expect("admitted fixture edit must reach the production writer");
    journal.commit(capture, &[], &[]);
    let state = journal.state();
    assert!(
        state.notice.is_none(),
        "current ASF edit did not receive Undo admission"
    );
    assert_eq!(state.undo.as_ref().unwrap().label, "Admitted ASF edit");
    assert_eq!(state.undo.unwrap().files, 1);
    let saved = fs::read(&path).unwrap();
    assert_eq!(media(&saved), media(&before));
    let after = watched_descriptors(&saved);
    println!(
        "ASF_ADMITTED_DESCRIPTOR_TRACE {}",
        serde_json::json!({"field":field,"explicit":explicit,"write_accepted":true,"before":observed.iter().map(|d|&d.value).collect::<Vec<_>>(),"saved":after.iter().map(|d|&d.value).collect::<Vec<_>>(),"media_preserved":true})
    );
    if explicit {
        assert_eq!(after[0].kind, 0);
        assert_eq!(after[0].value, w("Replacement ✓"));
        assert_eq!(after[1].kind, 0);
        assert_eq!(after[1].value, w("Second replacement"));
    } else {
        assert_eq!(
            after.iter().map(|d| &d.raw).collect::<Vec<_>>(),
            observed.iter().map(|d| &d.raw).collect::<Vec<_>>(),
            "unrelated edit changed an admitted untouched raw descriptor"
        );
    }
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), saved);
}

#[test]
fn admitted_noncanonical_asf_text_survives_another_object_edit() {
    // ffmpeg duplicates TITLE in Extended as well as Content Description.
    // COMMENT instead targets its absent Content Description slot exclusively.
    admitted_noncanonical_asf_edit("COMMENT", false);
}

#[test]
fn admitted_noncanonical_asf_text_survives_same_object_edit() {
    admitted_noncanonical_asf_edit("ALBUM", false);
}

#[test]
fn explicitly_edited_noncanonical_asf_text_uses_normal_unicode_serialization() {
    admitted_noncanonical_asf_edit("KeepSurrogate", true);
}

// Independent bounded observers for all shared text codecs. They deliberately
// retain names/values as bytes rather than asking the ASF implementation to
// decode its own output.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawTextRecord {
    name: Vec<u8>,
    value: Vec<u8>,
    kind: u16,
    language: u16,
    stream: u16,
}
fn record_body(records: &[RawTextRecord], extended: bool) -> Vec<u8> {
    let mut body = (records.len() as u16).to_le_bytes().to_vec();
    for r in records {
        if !extended {
            body.extend(r.language.to_le_bytes());
            body.extend(r.stream.to_le_bytes());
        }
        body.extend((r.name.len() as u16).to_le_bytes());
        if extended {
            body.extend(&r.name);
        }
        body.extend(r.kind.to_le_bytes());
        if extended {
            body.extend((r.value.len() as u16).to_le_bytes());
        } else {
            body.extend((r.value.len() as u32).to_le_bytes());
            body.extend(&r.name);
        }
        body.extend(&r.value);
    }
    body
}
fn observe_record_body(body: &[u8], extended: bool) -> Vec<RawTextRecord> {
    let mut at = 0;
    let take = |at: &mut usize, n: usize| {
        let end = at.checked_add(n).unwrap();
        assert!(end <= body.len());
        let value = &body[*at..end];
        *at = end;
        value
    };
    let word = |at: &mut usize| u16::from_le_bytes(take(at, 2).try_into().unwrap());
    let count = word(&mut at);
    let mut out = Vec::new();
    for _ in 0..count {
        let language = if extended { 0 } else { word(&mut at) };
        let stream = if extended { 0 } else { word(&mut at) };
        let name_len = usize::from(word(&mut at));
        let mut name = if extended {
            take(&mut at, name_len).to_vec()
        } else {
            vec![]
        };
        let kind = word(&mut at);
        let size = if extended {
            usize::from(word(&mut at))
        } else {
            u32::from_le_bytes(take(&mut at, 4).try_into().unwrap()) as usize
        };
        if !extended {
            name = take(&mut at, name_len).to_vec();
        }
        let value = take(&mut at, size).to_vec();
        out.push(RawTextRecord {
            name,
            value,
            kind,
            language,
            stream,
        });
    }
    assert_eq!(at, body.len());
    out
}
fn objects_in(bytes: &[u8]) -> Vec<&[u8]> {
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        assert!(bytes.len() - at >= 24);
        let n = usize::try_from(u64::from_le_bytes(
            bytes[at + 16..at + 24].try_into().unwrap(),
        ))
        .unwrap();
        assert!(n >= 24 && n <= bytes.len() - at);
        out.push(&bytes[at..at + n]);
        at += n;
    }
    out
}
fn observe_records(bytes: &[u8], location: [u8; 16]) -> Vec<RawTextRecord> {
    let mut out = Vec::new();
    for child in bounded_asf_children(bytes) {
        let b = &bytes[child];
        if b[..16] == location {
            out.extend(observe_record_body(&b[24..], true));
        }
        if b[..16] == EXTENSION {
            assert_eq!(
                u32::from_le_bytes(b[42..46].try_into().unwrap()) as usize,
                b.len() - 46
            );
            for nested in objects_in(&b[46..]) {
                if nested[..16] == location {
                    out.extend(observe_record_body(&nested[24..], false));
                }
            }
        }
    }
    out
}
fn observe_content(bytes: &[u8]) -> Vec<Vec<u8>> {
    let contents = bounded_asf_children(bytes)
        .into_iter()
        .filter(|r| bytes[r.start..r.start + 16] == CONTENT)
        .collect::<Vec<_>>();
    assert_eq!(contents.len(), 1);
    let b = &bytes[contents[0].clone()];
    assert!(b.len() >= 34);
    let mut at = 34;
    let mut out = Vec::new();
    for i in 0..5 {
        let size = usize::from(u16::from_le_bytes(
            b[24 + i * 2..26 + i * 2].try_into().unwrap(),
        ));
        assert!(size <= b.len() - at);
        out.push(b[at..at + size].to_vec());
        at += size;
    }
    assert_eq!(at, b.len());
    out
}
fn rebuild_fixture(original: &[u8], replacements: &[([u8; 16], Vec<u8>)]) -> Vec<u8> {
    let mut children = Vec::new();
    for r in bounded_asf_children(original) {
        let b = &original[r];
        if b[..16] == EXTENSION {
            continue;
        }
        if replacements.iter().any(|(id, _)| b[..16] == *id) {
            continue;
        }
        children.push(b.to_vec());
    }
    let mut nested = Vec::new();
    for (id, body) in replacements {
        if *id == METADATA || *id == LIBRARY {
            nested.extend(object(*id, body));
        } else {
            children.push(object(*id, body));
        }
    }
    if !nested.is_empty() {
        let mut body = EXTENSION_RESERVED.to_vec();
        body.extend(6u16.to_le_bytes());
        body.extend((nested.len() as u32).to_le_bytes());
        body.extend(nested);
        children.push(object(EXTENSION, &body));
    }
    let mut bytes = original[..30].to_vec();
    bytes[24..28].copy_from_slice(&(children.len() as u32).to_le_bytes());
    bytes[16..24].copy_from_slice(
        &((30 + children.iter().map(Vec::len).sum::<usize>()) as u64).to_le_bytes(),
    );
    for b in children {
        bytes.extend(b);
    }
    bytes.extend(media(original));
    let size = bytes.len() as u64;
    for r in bounded_asf_children(&bytes) {
        if bytes[r.start..r.start + 16] == FILE_PROPERTIES {
            bytes[r.start + 40..r.start + 48].copy_from_slice(&size.to_le_bytes());
        }
    }
    bytes
}
fn raw_text_records(location: [u8; 16]) -> Vec<RawTextRecord> {
    let (language, stream) = if location == LIBRARY {
        (7, 0)
    } else if location == METADATA {
        (0, 1)
    } else {
        (0, 0)
    };
    let mut name1 = w("RawName");
    name1.splice(name1.len() - 2..name1.len() - 2, [0x00, 0xd8]);
    let mut name2 = w("RawName");
    name2.splice(name2.len() - 2..name2.len() - 2, [0x01, 0xd8]);
    vec![
        RawTextRecord {
            name: name1,
            value: vec![0x00, 0xd8, 0, 0],
            kind: 0,
            language,
            stream,
        },
        RawTextRecord {
            name: name2,
            value: vec![0x01, 0xd8, 0, 0],
            kind: 0,
            language: if location == LIBRARY { 8 } else { language },
            stream: if location == METADATA { 2 } else { stream },
        },
        RawTextRecord {
            name: {
                let mut b = w("OddName");
                b.push(0x99);
                b
            },
            value: vec![b'A', 0, 0, 0, 0x55],
            kind: 0,
            language,
            stream,
        },
        RawTextRecord {
            name: w("TouchRaw"),
            value: w("before"),
            kind: 0,
            language,
            stream,
        },
    ]
}
fn journaled_asf_step(
    path: &Path,
    updates: &Fields,
    rules: &[UserFieldMapping],
    journal: &mut history::Journal,
) {
    let before = fs::read(path).unwrap();
    let capture = journal.begin("ASF raw text", None, &[path.to_string_lossy().into_owned()]);
    write_fields_with_mappings(path, updates, rules).unwrap();
    journal.commit(capture, &[], &[]);
    let state = journal.state();
    assert!(state.notice.is_none(), "{:?}", state.notice);
    assert_eq!(state.undo.unwrap().files, 1);
    let saved = fs::read(path).unwrap();
    assert_eq!(media(&before), media(&saved));
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(path).unwrap(), before);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(path).unwrap(), saved);
}
fn noncanonical_asf_shared_location(location: [u8; 16], edit: bool) {
    if !tools() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.wma");
    encoded_noncanonical_asf(&path);
    let records = raw_text_records(location);
    let before = rebuild_fixture(
        &fs::read(&path).unwrap(),
        &[(location, record_body(&records, location == EXTENDED))],
    );
    fs::write(&path, &before).unwrap();
    let entry = read_entry(&path).expect("admitted shared text fixture");
    assert_eq!(entry.fields["RawName�"], ["�", "�"]);
    assert_eq!(entry.fields["OddName"], ["A"]);
    assert_eq!(observe_records(&before, location), records);
    for mutation in 0..if location == EXTENDED { 2 } else { 4 } {
        let mut changed = records.clone();
        match mutation {
            0 => changed[0].name[0] ^= 1,
            1 => changed[1].value[0] ^= 1,
            2 => changed[2].stream ^= 1,
            _ => changed[2].language ^= 1,
        }
        assert_ne!(
            records,
            observe_record_body(
                &record_body(&changed, location == EXTENDED),
                location == EXTENDED
            ),
            "observer missed independent mutation {mutation}"
        );
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut journal =
        history::Journal::open(&dir.path().join("journal"), history::Limits::default()).unwrap();
    if !edit {
        journaled_asf_step(
            &path,
            &fields(&[("TouchRaw", &["after"])]),
            &[],
            &mut journal,
        );
        let saved = observe_records(&fs::read(&path).unwrap(), location);
        assert_eq!(
            &saved[..3],
            &records[..3],
            "same-location edit changed raw name/value occurrences"
        );
    } else {
        let rules = [UserFieldMapping {
            tag_type: "WMA".into(),
            source: "RawName�".into(),
            target: "MAPPEDRAW".into(),
        }];
        journaled_asf_step(
            &path,
            &fields(&[("MAPPEDRAW", &["�"]), ("OddName", &[])]),
            &rules,
            &mut journal,
        );
        let all = [EXTENDED, METADATA, LIBRARY]
            .into_iter()
            .flat_map(|id| observe_records(&fs::read(&path).unwrap(), id))
            .collect::<Vec<_>>();
        let replaced = all
            .iter()
            .filter(|r| r.name == w("RawName�"))
            .collect::<Vec<_>>();
        assert_eq!(replaced.len(), 1);
        assert_eq!(replaced[0].value, w("�"));
        assert!(!all.iter().any(|r| r.name.starts_with(&w("OddName")[..14])));
        journaled_asf_step(
            &path,
            &fields(&[("TouchRaw", &["second"])]),
            &[],
            &mut journal,
        );
        let all = [EXTENDED, METADATA, LIBRARY]
            .into_iter()
            .flat_map(|id| observe_records(&fs::read(&path).unwrap(), id))
            .collect::<Vec<_>>();
        assert!(
            !all.iter().any(|r| records[..3].contains(r)),
            "removed raw occurrence resurrected"
        );
    }
}
#[test]
fn noncanonical_asf_extended_name_and_value_occurrences_survive() {
    noncanonical_asf_shared_location(EXTENDED, false);
}
#[test]
fn noncanonical_asf_metadata_name_and_value_occurrences_survive() {
    noncanonical_asf_shared_location(METADATA, false);
}
#[test]
fn noncanonical_asf_library_name_and_value_occurrences_survive() {
    noncanonical_asf_shared_location(LIBRARY, false);
}
#[test]
fn noncanonical_asf_mapped_equal_display_edits_and_removals_are_authored() {
    for location in [EXTENDED, METADATA, LIBRARY] {
        noncanonical_asf_shared_location(location, true);
    }
}
fn noncanonical_asf_content(edit: bool) {
    if !tools() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("content.wma");
    encoded_noncanonical_asf(&path);
    let slots = vec![
        vec![0x00, 0xd8, 0, 0],
        {
            let mut raw = w("Artist");
            raw.extend([0, 0]);
            raw
        },
        vec![0, 0, 0x99],
        w("before"),
        vec![],
    ];
    let mut body = Vec::new();
    for slot in &slots {
        body.extend((slot.len() as u16).to_le_bytes());
    }
    body.extend(slots.concat());
    // Remove ffmpeg's duplicate Extended title, retaining a separate album.
    let album = RawTextRecord {
        name: w("WM/AlbumTitle"),
        value: w("album"),
        kind: 0,
        language: 0,
        stream: 0,
    };
    let before = rebuild_fixture(
        &fs::read(&path).unwrap(),
        &[(CONTENT, body), (EXTENDED, record_body(&[album], true))],
    );
    fs::write(&path, &before).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["TITLE"], ["�"]);
    assert!(!entry.fields.contains_key("COPYRIGHT"));
    assert!(!entry.fields.contains_key("RATING"));
    assert_eq!(observe_content(&before), slots);
    let mut altered = before.clone();
    let r = bounded_asf_children(&altered)
        .into_iter()
        .find(|r| altered[r.start..r.start + 16] == CONTENT)
        .unwrap();
    altered[r.start + 34 + slots[0].len() + slots[1].len() + 2] ^= 1;
    assert_ne!(observe_content(&altered), slots);
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut journal =
        history::Journal::open(&dir.path().join("journal"), history::Limits::default()).unwrap();
    if !edit {
        journaled_asf_step(
            &path,
            &fields(&[("COMMENT", &["after"])]),
            &[],
            &mut journal,
        );
        let saved = observe_content(&fs::read(&path).unwrap());
        for i in [0, 1, 2, 4] {
            assert_eq!(saved[i], slots[i], "untouched Content slot {i} changed");
        }
    } else {
        journaled_asf_step(
            &path,
            &fields(&[("TITLE", &["�"]), ("COPYRIGHT", &[])]),
            &[],
            &mut journal,
        );
        let saved = observe_content(&fs::read(&path).unwrap());
        assert_eq!(saved[0], w("�"));
        assert!(saved[2].is_empty());
        journaled_asf_step(
            &path,
            &fields(&[("COMMENT", &["second"])]),
            &[],
            &mut journal,
        );
        assert!(
            observe_content(&fs::read(&path).unwrap())[2].is_empty(),
            "removed empty raw Content slot resurrected"
        );
    }
}
#[test]
fn noncanonical_asf_content_slots_survive_sibling_edit() {
    noncanonical_asf_content(false);
}
#[test]
fn noncanonical_asf_content_equal_display_edit_and_empty_slot_removal() {
    noncanonical_asf_content(true);
}

#[test]
fn noncanonical_asf_empty_content_slots_stay_hidden_and_remove_without_resurrection() {
    if !tools() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty-slots.wma");
    encoded_noncanonical_asf(&path);
    let slots = vec![vec![0, 0], vec![], vec![0, 0, 0x99], vec![], vec![]];
    let mut body = Vec::new();
    for slot in &slots {
        body.extend((slot.len() as u16).to_le_bytes());
    }
    body.extend(slots.concat());
    let before = rebuild_fixture(
        &fs::read(&path).unwrap(),
        &[(CONTENT, body), (EXTENDED, 0u16.to_le_bytes().to_vec())],
    );
    fs::write(&path, &before).unwrap();
    let entry = read_entry(&path).unwrap();
    assert!(entry.fields.is_empty());
    assert!(!entry.technical.contains_key("_TAG"));
    let mut journal =
        history::Journal::open(&dir.path().join("journal"), history::Limits::default()).unwrap();
    journaled_asf_step(
        &path,
        &fields(&[("COMMENT", &["sibling"])]),
        &[],
        &mut journal,
    );
    let saved = observe_content(&fs::read(&path).unwrap());
    for i in [0, 1, 2, 4] {
        assert_eq!(saved[i], slots[i]);
    }
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields, fields(&[("COMMENT", &["sibling"])]));
    journaled_asf_step(
        &path,
        &fields(&[("TITLE", &[]), ("COPYRIGHT", &[])]),
        &[],
        &mut journal,
    );
    journaled_asf_step(
        &path,
        &fields(&[("COMMENT", &["later"])]),
        &[],
        &mut journal,
    );
    let saved = observe_content(&fs::read(&path).unwrap());
    assert!(saved[0].is_empty() && saved[2].is_empty());
}

#[test]
fn cover_plans_reorder_retype_and_add_on_asf() {
    let tmp = tempfile::tempdir().unwrap();
    let original = hand_built();
    let path = write(tmp.path(), "hand.wma", &original);
    let options = TagOptions::default();
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::new(3, 2))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let entry = add_cover_bytes_with_options(
        &path,
        png.into_inner(),
        "new.png",
        Some("CoverBack"),
        &options,
    )
    .unwrap();
    assert_eq!(entry.covers.len(), 2);
    let keep = |cover: &CoverInfo, picture_type: &str| CoverChange {
        original: Some(CoverRef::of(cover)),
        picture_type: picture_type.into(),
        description: cover.description.clone(),
        ..Default::default()
    };
    let front = &entry.covers[0];
    let data = find_cover_data(&path, &CoverRef::of(front), &options)
        .unwrap()
        .unwrap();
    assert_eq!(cover_digest(&data), front.digest);
    let entry = apply_cover_changes_with_options(
        &path,
        &[keep(&entry.covers[1], "CoverBack"), keep(front, "Media")],
        &options,
    )
    .unwrap();
    assert_eq!(
        entry
            .covers
            .iter()
            .map(|c| c.picture_type.as_str())
            .collect::<Vec<_>>(),
        ["CoverBack", "Media"]
    );
    assert_eq!(entry.covers[1].digest, front.digest);
    assert_eq!(entry.fields["TITLE"], ["Hand Title"]);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(media(&bytes), media(&original));
    assert_consistent(&bytes);
}
