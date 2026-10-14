//! Options → Tags → Mpeg / Ape, Mpc / Advanced and Files preferences against
//! real files. MPEG fixtures are independently generated frames; the encoded
//! matrix additionally checks decoder packets when ffmpeg is installed.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    id3::{
        v1::Id3v1Tag,
        v2::{Frame, FrameId, Id3v2Tag, Id3v2Version, TextInformationFrame, TimestampFrame},
    },
    mpeg::MpegFile,
    prelude::TagExt,
    tag::{items::Timestamp, ItemValue},
    TextEncoding,
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    add_cover_with_options, read_entry, read_entry_with_options, remove_cover_with_options,
    scan_paths_with_options, write_fields, write_fields_with_options, Fields, Id3v2Format,
    TagOptions,
};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

/// Twenty MPEG-1 Layer III frames (128 kbit/s, 44.1 kHz) with a recognisable
/// payload that contains no frame sync or tag markers.
fn mp3(path: &Path) -> Vec<u8> {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xFF, 0xFB, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
    audio
}

#[derive(Debug, PartialEq)]
struct Kinds {
    id3v2: Option<u8>,
    id3v1: bool,
    ape: Option<u32>,
}

fn ape_footer(bytes: &[u8]) -> Option<(usize, u32, usize)> {
    let end = if bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG" {
        bytes.len() - 128
    } else {
        bytes.len()
    };
    let footer = end.checked_sub(32)?;
    if &bytes[footer..footer + 8] != b"APETAGEX" {
        return None;
    }
    let version = u32::from_le_bytes(bytes[footer + 8..footer + 12].try_into().unwrap());
    let size = u32::from_le_bytes(bytes[footer + 12..footer + 16].try_into().unwrap()) as usize;
    let flags = u32::from_le_bytes(bytes[footer + 20..footer + 24].try_into().unwrap());
    let header = if flags & 0x8000_0000 != 0 { 32 } else { 0 };
    Some((end - size - header, version, end))
}

fn kinds(path: &Path) -> Kinds {
    let bytes = fs::read(path).unwrap();
    Kinds {
        id3v2: (&bytes[..3] == b"ID3").then(|| bytes[3]),
        id3v1: bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG",
        ape: ape_footer(&bytes).map(|(_, version, _)| version),
    }
}

/// The MPEG frames between the leading ID3v2 tag and trailing APE/ID3v1 tags.
fn audio(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    let start = if &bytes[..3] == b"ID3" {
        10 + bytes[6..10]
            .iter()
            .fold(0usize, |v, b| (v << 7) | usize::from(*b))
    } else {
        0
    };
    let end = match ape_footer(&bytes) {
        Some((start, _, _)) => start,
        None if kinds(path).id3v1 => bytes.len() - 128,
        None => bytes.len(),
    };
    bytes[start..end].to_vec()
}

fn id3(path: &Path) -> Id3v2Tag {
    MpegFile::read_from(
        &mut fs::File::open(path).unwrap(),
        ParseOptions::new().implicit_conversions(false),
    )
    .unwrap()
    .id3v2()
    .cloned()
    .unwrap()
}

fn text(tag: &Id3v2Tag, id: &str) -> Option<String> {
    tag.get_text(&FrameId::new(id).unwrap()).map(str::to_owned)
}

fn seed_all_kinds(path: &Path) -> Vec<u8> {
    let audio = mp3(path);
    let mut v2 = Id3v2Tag::new();
    lofty::tag::Accessor::set_title(&mut v2, "Two".into());
    v2.save_to_path(path, WriteOptions::default()).unwrap();
    let mut ape = ApeTag::new();
    ape.insert(ApeItem::new("Title".into(), ItemValue::Text("Ape".into())).unwrap());
    ape.save_to_path(path, WriteOptions::default()).unwrap();
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("One".into());
    v1.year = Some(1999);
    v1.save_to_path(path, WriteOptions::default()).unwrap();
    audio
}

fn reading(kind: &str) -> TagOptions {
    let mut options = TagOptions::default();
    options.mpeg.read_id3v1 = kind == "id3v1";
    options.mpeg.read_id3v2 = kind == "id3v2";
    options.mpeg.read_ape = kind == "ape";
    options
}

#[test]
fn mpeg_reads_display_one_enabled_tag_kind_by_priority() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("all.mp3");
    seed_all_kinds(&path);
    assert_eq!(
        kinds(&path),
        Kinds {
            id3v2: Some(4),
            id3v1: true,
            ape: Some(2000)
        }
    );
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["Title"], ["Ape"]);
    assert!(
        !entry.fields.contains_key("YEAR"),
        "ID3v1 values are not merged"
    );
    assert_eq!(entry.technical["_TAG_READ"], ["APE"]);
    let mut options = TagOptions::default();
    options.mpeg.read_ape = false;
    let entry = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(entry.fields["TITLE"], ["Two"]);
    assert!(!entry.fields.contains_key("YEAR"));
    assert_eq!(entry.technical["_TAG_READ"], ["ID3v2.4"]);
    options.mpeg.read_id3v2 = false;
    let entry = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(entry.fields["TITLE"], ["One"]);
    assert_eq!(entry.fields["YEAR"], ["1999"]);
    options.mpeg.read_id3v1 = false;
    let entry = read_entry_with_options(&path, &[], &options).unwrap();
    assert!(entry.fields.is_empty());
    assert!(!entry.technical.contains_key("_TAG_READ"));
    assert_eq!(
        entry.technical["_TAG"].len(),
        1,
        "all present kinds stay listed"
    );
}

#[test]
fn default_mpeg_write_creates_id3v1_and_utf16_id3v23_and_strips_ape() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("default.mp3");
    let original_audio = mp3(&path);
    let mut ape = ApeTag::new();
    ape.insert(ApeItem::new("Title".into(), ItemValue::Text("Ape title".into())).unwrap());
    ape.insert(ApeItem::new("Album".into(), ItemValue::Text("Ape album".into())).unwrap());
    ape.save_to_path(&path, WriteOptions::default()).unwrap();
    let entry = write_fields(&path, &fields(&[("ARTIST", &["Artist é"])])).unwrap();
    assert_eq!(
        kinds(&path),
        Kinds {
            id3v2: Some(3),
            id3v1: true,
            ape: None
        }
    );
    assert_eq!(audio(&path), original_audio);
    assert_eq!(entry.fields["TITLE"], ["Ape title"]);
    assert_eq!(entry.fields["ALBUM"], ["Ape album"]);
    assert_eq!(entry.fields["ARTIST"], ["Artist é"]);
    assert_eq!(entry.technical["_TAG_READ"], ["ID3v2.3"]);
    let tag = id3(&path);
    assert_eq!(tag.original_version(), Id3v2Version::V3);
    for frame in &tag {
        if let Frame::Text(frame) = frame {
            assert_eq!(frame.encoding, TextEncoding::UTF16, "{}", frame.value);
        }
    }
    let v1 = read_entry_with_options(&path, &[], &reading("id3v1")).unwrap();
    assert_eq!(v1.fields["TITLE"], ["Ape title"]);
    assert_eq!(v1.fields["ARTIST"], ["Artist é"]);
    assert_eq!(v1.fields["ALBUM"], ["Ape album"]);
}

#[test]
fn id3v2_revision_conversion_keeps_dates_and_version_specific_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("convert.mp3");
    let original_audio = mp3(&path);
    let mut tag = Id3v2Tag::new();
    tag.insert(Frame::Timestamp(TimestampFrame::new(
        FrameId::new("TDRC").unwrap(),
        TextEncoding::UTF8,
        Timestamp::parse(
            &mut "2024-03-25T10:11".as_bytes(),
            lofty::config::ParsingMode::Strict,
        )
        .unwrap()
        .unwrap(),
    )));
    for (id, value) in [("TMOO", "Calm"), ("TDOR", "1999"), ("TIT2", "Old")] {
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new(id).unwrap(),
            TextEncoding::UTF8,
            value.to_owned(),
        )));
    }
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    write_fields(&path, &fields(&[("TITLE", &["New"])])).unwrap();
    let tag = id3(&path);
    assert_eq!(tag.original_version(), Id3v2Version::V3);
    assert_eq!(text(&tag, "TYER").as_deref(), Some("2024"));
    assert_eq!(text(&tag, "TDAT").as_deref(), Some("2503"));
    assert_eq!(text(&tag, "TIME").as_deref(), Some("1011"));
    assert_eq!(text(&tag, "TORY").as_deref(), Some("1999"));
    assert!((&tag).into_iter().any(
        |f| matches!(f, Frame::UserText(t) if t.description == "MOOD" && t.content == "Calm")
    ));
    assert!(text(&tag, "TDRC").is_none() && text(&tag, "TMOO").is_none());
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["YEAR"], ["2024"]);
    assert_eq!(entry.fields["DATE"], ["2503"]);
    assert_eq!(entry.fields["MOOD"], ["Calm"]);
    assert_eq!(entry.fields["ORIGYEAR"], ["1999"]);

    let mut options = TagOptions::default();
    options.mpeg.id3v2_format = Id3v2Format::V24Utf8;
    let entry = write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Again"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(kinds(&path).id3v2, Some(4));
    assert_eq!(audio(&path), original_audio);
    let tag = id3(&path);
    assert!(matches!(
        tag.get(&FrameId::new("TDRC").unwrap()),
        Some(Frame::Timestamp(frame)) if frame.timestamp.to_string() == "2024-03-25T10:11"
    ));
    assert_eq!(text(&tag, "TMOO").as_deref(), Some("Calm"));
    assert!(matches!(
        tag.get(&FrameId::new("TDOR").unwrap()),
        Some(Frame::Timestamp(frame)) if frame.timestamp.to_string() == "1999"
    ));
    assert!(matches!(
        tag.get(&FrameId::new("TIT2").unwrap()),
        Some(Frame::Text(frame)) if frame.encoding == TextEncoding::UTF8
    ));
    assert_eq!(entry.fields["MOOD"], ["Calm"]);
    assert_eq!(entry.technical["_TAG_READ"], ["ID3v2.4"]);
}

#[test]
fn latin1_mode_writes_iso_8859_1_frames() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("latin1.mp3");
    mp3(&path);
    let mut options = TagOptions::default();
    options.mpeg.id3v2_format = Id3v2Format::V23Latin1;
    let entry = write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Café 日本"]), ("COMMENT", &["Plain"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(entry.fields["TITLE"], ["Café ??"]);
    let tag = id3(&path);
    assert_eq!(tag.original_version(), Id3v2Version::V3);
    for frame in &tag {
        match frame {
            Frame::Text(frame) => assert_eq!(frame.encoding, TextEncoding::Latin1),
            Frame::Comment(frame) => assert_eq!(frame.encoding, TextEncoding::Latin1),
            _ => {}
        }
    }
}

#[test]
fn write_and_remove_sets_produce_exactly_the_selected_tag_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    for write in [
        [true, false, false],
        [false, true, false],
        [false, false, true],
        [true, true, false],
        [false, true, true],
        [true, true, true],
    ] {
        for remove in [false, true] {
            let path = tmp.path().join("matrix.mp3");
            let original_audio = seed_all_kinds(&path);
            let mut options = TagOptions::default();
            let mpeg = &mut options.mpeg;
            [mpeg.write_id3v1, mpeg.write_id3v2, mpeg.write_ape] = write;
            (mpeg.remove_id3v1, mpeg.remove_id3v2, mpeg.remove_ape) = (remove, remove, remove);
            write_fields_with_options(
                &path,
                &fields(&[("TITLE", &["Matrix"])]),
                false,
                &[],
                &options,
            )
            .unwrap_or_else(|e| panic!("{write:?}/{remove}: {e}"));
            let found = kinds(&path);
            // Writing APEv2 also updates an existing ID3v1 tag.
            let v1 = write[0] || write[2];
            assert_eq!(found.id3v1, v1 || !remove, "{write:?}/{remove}");
            assert_eq!(
                found.id3v2.is_some(),
                write[1] || !remove,
                "{write:?}/{remove}"
            );
            assert_eq!(
                found.ape.is_some(),
                write[2] || !remove,
                "{write:?}/{remove}"
            );
            if write[1] {
                assert_eq!(found.id3v2, Some(3));
            }
            assert_eq!(audio(&path), original_audio, "{write:?}/{remove}");
            for (enabled, kind) in [(v1, "id3v1"), (write[1], "id3v2"), (write[2], "ape")] {
                if enabled {
                    let entry = read_entry_with_options(&path, &[], &reading(kind)).unwrap();
                    let title = entry
                        .fields
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("TITLE"))
                        .map(|(_, v)| v.clone());
                    assert_eq!(title, Some(vec!["Matrix".to_owned()]), "{kind} {write:?}");
                }
            }
        }
    }
}

#[test]
fn id3v2_is_written_only_when_id3v1_cannot_hold_the_values() {
    let tmp = tempfile::tempdir().unwrap();
    let mut options = TagOptions::default();
    options.mpeg.id3v2_only_if_needed = true;
    let path = tmp.path().join("small.mp3");
    mp3(&path);
    let short = fields(&[
        ("TITLE", &["Short"]),
        ("GENRE", &["Rock"]),
        ("TRACK", &["3"]),
        ("YEAR", &["2024"]),
    ]);
    let entry = write_fields_with_options(&path, &short, false, &[], &options).unwrap();
    assert_eq!(
        kinds(&path),
        Kinds {
            id3v2: None,
            id3v1: true,
            ape: None
        }
    );
    assert_eq!(entry.fields, short);
    let long = "A title that is longer than thirty characters";
    let entry =
        write_fields_with_options(&path, &fields(&[("TITLE", &[long])]), false, &[], &options)
            .unwrap();
    assert_eq!(kinds(&path).id3v2, Some(3));
    assert_eq!(entry.fields["TITLE"], [long]);
    assert_eq!(
        entry.fields["GENRE"],
        ["Rock"],
        "new ID3v2 starts from all values"
    );
    assert_eq!(entry.fields["TRACK"], ["3"]);
    // An existing ID3v2 tag is updated even when ID3v1 would suffice.
    let entry = write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Short"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(kinds(&path).id3v2, Some(3));
    assert_eq!(entry.technical["_TAG_READ"], ["ID3v2.3"]);

    let custom = tmp.path().join("genre.mp3");
    mp3(&custom);
    write_fields_with_options(
        &custom,
        &fields(&[("GENRE", &["My own genre"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(
        kinds(&custom).id3v2,
        Some(3),
        "user-defined genres need ID3v2"
    );
}

#[test]
fn apev1_tags_are_upgraded_to_apev2_and_existing_id3v1_is_updated() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("apev1.mp3");
    let original_audio = mp3(&path);
    let mut item = Vec::new();
    item.extend(3u32.to_le_bytes());
    item.extend(0u32.to_le_bytes());
    item.extend(b"Title\0One");
    let mut footer = b"APETAGEX".to_vec();
    footer.extend(1000u32.to_le_bytes());
    footer.extend((item.len() as u32 + 32).to_le_bytes());
    footer.extend(1u32.to_le_bytes());
    footer.extend(0u32.to_le_bytes());
    footer.extend([0; 8]);
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend(item);
    bytes.extend(footer);
    fs::write(&path, bytes).unwrap();
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("Old v1".into());
    v1.save_to_path(&path, WriteOptions::default()).unwrap();
    assert_eq!(kinds(&path).ape, Some(1000));
    assert_eq!(read_entry(&path).unwrap().fields["Title"], ["One"]);

    let mut options = TagOptions::default();
    options.mpeg.write_id3v1 = false;
    options.mpeg.write_id3v2 = false;
    options.mpeg.write_ape = true;
    let entry = write_fields_with_options(
        &path,
        &fields(&[("Album", &["Upgraded"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(
        kinds(&path),
        Kinds {
            id3v2: None,
            id3v1: true,
            ape: Some(2000)
        }
    );
    assert_eq!(entry.fields["Title"], ["One"]);
    assert_eq!(entry.fields["Album"], ["Upgraded"]);
    let v1 = read_entry_with_options(&path, &[], &reading("id3v1")).unwrap();
    assert_eq!(v1.fields["TITLE"], ["One"]);
    assert_eq!(v1.fields["ALBUM"], ["Upgraded"]);
    assert_eq!(audio(&path), original_audio);
}

#[test]
fn covers_follow_the_write_and_remove_sets() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cover.mp3");
    let original_audio = mp3(&path);
    let image = tmp.path().join("cover.png");
    fs::write(&image, base64_png()).unwrap();
    let mut options = TagOptions::default();
    options.mpeg.write_ape = true;
    let entry = add_cover_with_options(&path, &image, false, &options).unwrap();
    assert_eq!(entry.covers.len(), 1);
    for kind in ["id3v2", "ape"] {
        let entry = read_entry_with_options(&path, &[], &reading(kind)).unwrap();
        assert_eq!(entry.covers.len(), 1, "{kind}");
    }
    assert!(
        add_cover_with_options(&path, &image, false, &options).is_err(),
        "APE cannot store a second front cover"
    );
    let entry = remove_cover_with_options(&path, 0, false, &options).unwrap();
    assert!(entry.covers.is_empty());
    for kind in ["id3v2", "ape"] {
        assert!(read_entry_with_options(&path, &[], &reading(kind))
            .unwrap()
            .covers
            .is_empty());
    }
    let mut v1_only = TagOptions::default();
    v1_only.mpeg.write_id3v2 = false;
    v1_only.mpeg.remove_id3v2 = true;
    assert!(add_cover_with_options(&path, &image, false, &v1_only).is_err());
    assert_eq!(audio(&path), original_audio);
}

fn base64_png() -> Vec<u8> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=")
        .unwrap()
}

#[test]
fn advanced_comment_language_and_popularimeter_display() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("advanced.mp3");
    mp3(&path);
    let mut options = TagOptions::default();
    options.advanced.id3v2_comment_language = "deu".into();
    let entry = write_fields_with_options(
        &path,
        &fields(&[("COMMENT", &["Hallo"]), ("RATING MM", &["4"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(entry.fields["RATING MM"], ["4"]);
    let tag = id3(&path);
    assert!((&tag)
        .into_iter()
        .any(|f| matches!(f, Frame::Comment(c) if &c.language == b"deu" && c.content == "Hallo")));
    options.advanced.popularimeter_ratings = true;
    let entry = read_entry_with_options(&path, &[], &options).unwrap();
    assert!(!entry.fields.contains_key("RATING MM"));
    assert_eq!(entry.fields["POPULARIMETER"], ["no@email|196|0"]);
    options.advanced.id3v2_comment_language = "de".into();
    assert!(
        write_fields_with_options(&path, &fields(&[("TITLE", &["x"])]), false, &[], &options)
            .is_err()
    );
}

#[test]
fn preserve_timestamps_keeps_the_modification_time() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("time.mp3");
    mp3(&path);
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(old)
        .unwrap();
    let mut options = TagOptions {
        preserve_modified_time: true,
        ..Default::default()
    };
    write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Kept"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), old);
    options.preserve_modified_time = false;
    write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Moved"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_ne!(fs::metadata(&path).unwrap().modified().unwrap(), old);
}

#[test]
fn empty_id3v1_years_and_restricted_extensions_are_read() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("year.mp3");
    mp3(&path);
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("No year".into());
    v1.save_to_path(&path, WriteOptions::default()).unwrap();
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["No year"]);
    mp3(&tmp.path().join("other.mp2"));
    let all = scan_paths_with_options(
        &[tmp.path().to_string_lossy().into_owned()],
        false,
        &TagOptions::default(),
        None,
    );
    assert_eq!(all.entries.len(), 2);
    let restricted = scan_paths_with_options(
        &[tmp.path().to_string_lossy().into_owned()],
        false,
        &TagOptions::default(),
        Some(&[".MP2".to_owned()]),
    );
    assert_eq!(restricted.entries.len(), 1);
    assert_eq!(restricted.entries[0].filename, "other.mp2");
}

fn packet_hashes(path: &Path) -> Vec<String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_packets",
            "-show_data_hash",
        ])
        .args([
            "sha256",
            "-show_entries",
            "packet=data_hash",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn encoded_mp3_and_wavpack_policies_keep_audio_packets() {
    if Command::new("ffmpeg").arg("-version").output().is_err()
        || Command::new("ffprobe").arg("-version").output().is_err()
    {
        eprintln!("Skipping encoded tag-policy matrix: ffmpeg and ffprobe are required");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let encode = |name: &str, codec: &str| {
        let path = tmp.path().join(name);
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
            ])
            .args(["-c:a", codec])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        path
    };
    for format in [
        Id3v2Format::V23Utf16,
        Id3v2Format::V23Latin1,
        Id3v2Format::V24Utf8,
    ] {
        let path = encode("encoded.mp3", "libmp3lame");
        let before = packet_hashes(&path);
        let mut options = TagOptions::default();
        options.mpeg.id3v2_format = format;
        options.mpeg.write_ape = true;
        write_fields_with_options(
            &path,
            &fields(&[("TITLE", &["Encoded"])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let found = kinds(&path);
        assert_eq!(
            found.id3v2,
            Some(if format == Id3v2Format::V24Utf8 { 4 } else { 3 })
        );
        assert!(found.id3v1 && found.ape == Some(2000));
        assert_eq!(packet_hashes(&path), before, "{format:?}");
    }
    let path = encode("encoded.wv", "wavpack");
    let before = packet_hashes(&path);
    let entry = write_fields(&path, &fields(&[("Title", &["Pack"])])).unwrap();
    assert_eq!(entry.technical["_TAG_READ"], ["APE"]);
    let bytes = fs::read(&path).unwrap();
    assert!(
        &bytes[bytes.len() - 128..bytes.len() - 125] != b"TAG",
        "APE family default writes APEv2 only"
    );
    let mut options = TagOptions::default();
    options.ape.write_id3v1 = true;
    write_fields_with_options(
        &path,
        &fields(&[("Title", &["Both"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[bytes.len() - 128..bytes.len() - 125], b"TAG");
    options.ape.write_ape = false;
    options.ape.remove_ape = true;
    let entry = write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Only v1"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(entry.technical["_TAG_READ"], ["ID3v1"]);
    assert_eq!(entry.fields["TITLE"], ["Only v1"]);
    assert!(!entry.technical["_TAG"].iter().any(|t| t.contains("APE")));
    assert_eq!(packet_hashes(&path), before);
}

fn ape_with_gain(path: &Path, title: &str, track_gain: &str) {
    let mut ape = ApeTag::new();
    for (key, value) in [
        ("Title", title),
        ("REPLAYGAIN_TRACK_GAIN", track_gain),
        ("REPLAYGAIN_TRACK_PEAK", "0.988525"),
        ("MP3GAIN_UNDO", "+004,+004,N"),
    ] {
        ape.insert(ApeItem::new(key.into(), ItemValue::Text(value.into())).unwrap());
    }
    ape.save_to_path(path, WriteOptions::default()).unwrap();
}

#[test]
fn removing_a_displayed_ape_tag_keeps_gain_fields_in_id3v2() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("gain.mp3");
    mp3(&path);
    ape_with_gain(&path, "Ape title", "-6.48 dB");
    write_fields(&path, &fields(&[("ARTIST", &["Artist"])])).unwrap();
    assert_eq!(kinds(&path).ape, None);
    let v2 = read_entry_with_options(&path, &[], &reading("id3v2")).unwrap();
    assert_eq!(v2.fields["REPLAYGAIN_TRACK_GAIN"], ["-6.48 dB"]);
    assert_eq!(v2.fields["REPLAYGAIN_TRACK_PEAK"], ["0.988525"]);
    assert_eq!(v2.fields["MP3GAIN_UNDO"], ["+004,+004,N"]);
    assert_eq!(v2.fields["TITLE"], ["Ape title"]);
}

#[test]
fn removing_an_unread_ape_tag_migrates_only_gain_fields_into_id3v2() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("hidden-ape.mp3");
    mp3(&path);
    let mut v2 = Id3v2Tag::new();
    lofty::tag::Accessor::set_title(&mut v2, "Two".into());
    v2.save_to_path(&path, WriteOptions::default()).unwrap();
    ape_with_gain(&path, "Ape title", "-6.48 dB");
    let mut options = TagOptions::default();
    options.mpeg.read_ape = false;
    write_fields_with_options(
        &path,
        &fields(&[("ARTIST", &["Artist"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(kinds(&path).ape, None);
    let v2 = read_entry_with_options(&path, &[], &reading("id3v2")).unwrap();
    assert_eq!(
        v2.fields["TITLE"],
        ["Two"],
        "non-gain APE fields are not migrated"
    );
    assert_eq!(v2.fields["ARTIST"], ["Artist"]);
    assert_eq!(v2.fields["REPLAYGAIN_TRACK_GAIN"], ["-6.48 dB"]);
    assert_eq!(v2.fields["MP3GAIN_UNDO"], ["+004,+004,N"]);
}

#[test]
fn existing_id3v2_gain_values_win_over_a_removed_ape_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("both-gain.mp3");
    mp3(&path);
    let mut options = TagOptions::default();
    options.mpeg.read_ape = false;
    write_fields_with_options(
        &path,
        &fields(&[
            ("TITLE", &["Two"]),
            ("REPLAYGAIN_TRACK_GAIN", &["-1.00 dB"]),
        ]),
        false,
        &[],
        &options,
    )
    .unwrap();
    ape_with_gain(&path, "Ape title", "-6.48 dB");
    write_fields_with_options(
        &path,
        &fields(&[("ARTIST", &["Artist"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    let v2 = read_entry_with_options(&path, &[], &reading("id3v2")).unwrap();
    assert_eq!(v2.fields["REPLAYGAIN_TRACK_GAIN"], ["-1.00 dB"]);
    assert_eq!(v2.fields["MP3GAIN_UNDO"], ["+004,+004,N"]);
}
