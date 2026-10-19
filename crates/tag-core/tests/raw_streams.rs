//! TTA, TAK and OptimFROG streams built from their public header layouts,
//! tagged through the MPEG (TTA) and Ape, Mpc (TAK, OptimFROG) policies. The
//! stream bytes between the tags must never change. ffmpeg-encoded TTA and
//! independent readers (ffprobe, mutagen) are exercised when installed.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    id3::v2::{Frame, Id3v2Tag, TextInformationFrame},
    prelude::TagExt,
    tag::ItemValue,
    TextEncoding,
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    add_cover_bytes_with_options, read_entry, read_entry_with_options,
    remove_tag_kinds_with_options, remove_tags, scan_paths, write_fields,
    write_fields_with_options, Fields, RemovableTag, TagOptions,
};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}
/// APE item names keep their stored case ("Title"), like Mp3tag shows them.
fn value(entry: &tag_core::AudioEntry, key: &str) -> Option<String> {
    entry
        .fields
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.join("|"))
}
fn technical(entry: &tag_core::AudioEntry, key: &str) -> String {
    entry
        .technical
        .get(key)
        .map(|v| v.join("|"))
        .unwrap_or_default()
}
/// Payload bytes without tag markers ("ID3", "TAG", "APETAGEX").
fn payload(length: usize) -> Vec<u8> {
    (0..length).map(|i| (i * 37 % 89 + 1) as u8).collect()
}

/// TTA1 header: format 1 (PCM), channels, bits, sample rate, samples, CRC.
fn tta(channels: u16, bits: u16, rate: u32, samples: u32, audio: &[u8]) -> Vec<u8> {
    let mut bytes = b"TTA1".to_vec();
    for value in [1u16, channels, bits] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(rate.to_le_bytes());
    bytes.extend(samples.to_le_bytes());
    let crc = crc32(&bytes);
    bytes.extend(crc.to_le_bytes());
    bytes.extend(audio);
    bytes
}
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
/// CRC-24/OpenPGP (polynomial 0x864CFB, seed 0xB704CE), which TAK stores
/// little-endian after each metadata block.
fn crc24(bytes: &[u8]) -> u32 {
    let mut crc = 0xB704CEu32;
    for byte in bytes {
        crc ^= u32::from(*byte) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x100_0000 != 0 {
                crc ^= 0x864CFB;
            }
        }
    }
    crc & 0xFF_FFFF
}

/// tBaK, a STREAMINFO block (fields packed LSB first) and the END block.
fn tak(channels: u64, bits: u64, rate: u64, samples: u64, audio: &[u8]) -> Vec<u8> {
    let mut value: u128 = 0;
    let mut at = 0;
    for (field, width) in [
        (2u64, 6),
        (0, 4),
        (0, 4),
        (samples, 35),
        (0, 3),
        (rate - 6000, 18),
        (bits - 8, 5),
        (channels - 1, 4),
        (0, 1),
    ] {
        value |= u128::from(field) << at;
        at += width;
    }
    let info = value.to_le_bytes()[..10].to_vec();
    let mut bytes = b"tBaK".to_vec();
    bytes.push(1);
    bytes.extend(&((info.len() + 3) as u32).to_le_bytes()[..3]);
    bytes.extend(&info);
    bytes.extend(&crc24(&info).to_le_bytes()[..3]);
    bytes.extend([0, 0, 0, 0]);
    bytes.extend(audio);
    bytes
}

/// OFR header (15 bytes): total samples over all channels, sample type,
/// channel configuration, sample rate, encoder and compression.
fn ofr(channels: u8, sample_type: u8, rate: u32, samples: u64, audio: &[u8]) -> Vec<u8> {
    let mut bytes = b"OFR ".to_vec();
    bytes.extend(15u32.to_le_bytes());
    let total = samples * u64::from(channels);
    bytes.extend((total as u32).to_le_bytes());
    bytes.extend(((total >> 32) as u16).to_le_bytes());
    bytes.push(sample_type);
    bytes.push(channels - 1);
    bytes.extend(rate.to_le_bytes());
    bytes.extend(0x0510u16.to_le_bytes());
    bytes.push(0);
    bytes.extend(audio);
    bytes
}

fn ape_bytes(items: &[(&str, &str)]) -> Vec<u8> {
    let mut tag = ApeTag::new();
    for (key, value) in items {
        tag.insert(ApeItem::new(key.to_string(), ItemValue::Text(value.to_string())).unwrap());
    }
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
    bytes
}
fn id3_bytes(title: &str) -> Vec<u8> {
    let mut tag = Id3v2Tag::new();
    tag.insert(Frame::Text(TextInformationFrame::new(
        lofty::id3::v2::FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF8,
        title.to_owned(),
    )));
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
    bytes
}
fn id3v1(title: &str) -> Vec<u8> {
    let mut bytes = b"TAG".to_vec();
    let mut text = title.as_bytes().to_vec();
    text.resize(30, 0);
    bytes.extend(&text);
    bytes.resize(127, 0);
    bytes.push(255);
    bytes
}

fn contains(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[test]
fn stream_headers_give_technical_information() {
    let dir = tempfile::tempdir().unwrap();
    let audio = payload(4000);
    let cases = [
        (
            "a.tta",
            tta(2, 16, 44_100, 88_200, &audio),
            "TTA",
            "True Audio",
            2u8,
            "16",
            44_100u32,
            2.0,
        ),
        (
            "a.tak",
            tak(1, 24, 48_000, 24_000, &audio),
            "TAK",
            "TAK",
            1,
            "24",
            48_000,
            0.5,
        ),
        (
            "a.ofr",
            ofr(2, 4, 96_000, 192_000, &audio),
            "OptimFROG",
            "OptimFROG",
            2,
            "24",
            96_000,
            2.0,
        ),
        (
            "a.ofs",
            ofr(6, 2, 44_100, 44_100, &audio),
            "OptimFROG",
            "OptimFROG",
            6,
            "16",
            44_100,
            1.0,
        ),
    ];
    for (name, bytes, format, codec, channels, bits, rate, seconds) in cases {
        let path = dir.path().join(name);
        fs::write(&path, &bytes).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.format, format, "{name}");
        assert_eq!(technical(&entry, "_CODEC"), codec, "{name}");
        assert_eq!(entry.channels, channels, "{name}");
        assert_eq!(entry.sample_rate, rate, "{name}");
        assert_eq!(technical(&entry, "_BITSPERSAMPLE"), bits, "{name}");
        assert!((entry.duration - seconds).abs() < 1e-9, "{name}");
        assert!(entry.fields.is_empty(), "{name}");
        assert_eq!(technical(&entry, "_TAG"), "", "{name}");
        // The bitrate covers the stream, without tags.
        let expected = (bytes.len() as f64 * 8.0 / seconds / 1000.0).round() as u32;
        assert_eq!(entry.bitrate, expected, "{name}");
    }
    let scanned = scan_paths(&[dir.path().to_string_lossy().into_owned()], false);
    assert!(scanned.errors.is_empty(), "{:?}", scanned.errors);
    assert_eq!(scanned.entries.len(), 4);
}

#[test]
fn tta_follows_the_mpeg_policy_and_keeps_the_stream() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("track.tta");
    let stream = tta(2, 16, 44_100, 44_100, &payload(3000));
    let mut bytes = stream.clone();
    bytes.extend(ape_bytes(&[("Title", "From APE"), ("Artist", "Band")]));
    fs::write(&path, &bytes).unwrap();
    let mut ape_read = TagOptions::default();
    ape_read.mpeg.read_ape = true;
    let entry = read_entry_with_options(&path, &[], &ape_read).unwrap();
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("From APE"));
    assert_eq!(technical(&entry, "_TAG"), "APEv2");

    // Explicit APE read sources the ID3 write; the unwritten APE remains.
    write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Ünïcode"]), ("COMMENT", &["note"])]),
        false,
        &[],
        &ape_read,
    )
    .unwrap();
    let written = read_entry(&path).unwrap();
    assert_eq!(value(&written, "TITLE").as_deref(), Some("Ünïcode"));
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("Band"));
    assert_eq!(technical(&written, "_TAG_READ"), "ID3v2.3");
    assert_eq!(technical(&written, "_CODEC"), "True Audio");
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(b"ID3\x03"));
    let start = contains(&after, b"TTA1").unwrap();
    assert_eq!(&after[start..start + stream.len()], stream.as_slice());
    assert_eq!(&after[after.len() - 128..after.len() - 125], b"TAG");
    let original_ape = ape_bytes(&[("Title", "From APE"), ("Artist", "Band")]);
    assert_eq!(
        &after[start + stream.len()..after.len() - 128],
        original_ape
    );

    // Remove Tag strips every kind, leaving exactly the stream.
    let removed = remove_tags(&path).unwrap();
    assert!(removed.fields.is_empty());
    assert_eq!(fs::read(&path).unwrap(), stream);
}

#[test]
fn tak_and_optimfrog_follow_the_ape_policy() {
    let dir = tempfile::tempdir().unwrap();
    for (name, stream) in [
        ("track.tak", tak(2, 16, 44_100, 44_100, &payload(5000))),
        ("track.ofr", ofr(2, 2, 44_100, 44_100, &payload(5000))),
    ] {
        let path = dir.path().join(name);
        // A prepended ID3v2 tag is neither read nor written, and an ID3v1
        // tag is displayed only while no APE tag exists.
        let id3 = id3_bytes("Hidden ID3v2");
        let mut bytes = id3.clone();
        bytes.extend(&stream);
        bytes.extend(id3v1("Legacy"));
        fs::write(&path, &bytes).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(value(&entry, "TITLE").as_deref(), Some("Legacy"), "{name}");
        assert_eq!(technical(&entry, "_TAG_READ"), "ID3v1");

        let written = write_fields(
            &path,
            &fields(&[
                ("TITLE", &["New"]),
                ("ARTIST", &["A", "B"]),
                ("CUSTOM", &["x"]),
            ]),
        )
        .unwrap();
        assert_eq!(value(&written, "TITLE").as_deref(), Some("New"), "{name}");
        assert_eq!(value(&written, "ARTIST").as_deref(), Some("A|B"), "{name}");
        assert_eq!(value(&written, "CUSTOM").as_deref(), Some("x"), "{name}");
        assert_eq!(technical(&written, "_TAG_READ"), "APEv2", "{name}");
        let after = fs::read(&path).unwrap();
        // The ID3v2 tag and the stream are untouched; APEv2 sits before the
        // updated ID3v1 tag.
        assert!(after.starts_with(&id3));
        assert_eq!(
            &after[id3.len()..id3.len() + stream.len()],
            stream.as_slice()
        );
        let ape = id3.len() + stream.len();
        assert_eq!(&after[ape..ape + 8], b"APETAGEX", "APEv2 header");
        assert_eq!(&after[after.len() - 128..after.len() - 125], b"TAG");
        assert_eq!(&after[after.len() - 125..after.len() - 122], b"New");

        // A cover goes into the APE tag.
        let png = {
            let mut data = Vec::new();
            image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]))
                .write_to(
                    &mut std::io::Cursor::new(&mut data),
                    image::ImageFormat::Png,
                )
                .unwrap();
            data
        };
        let covered =
            add_cover_bytes_with_options(&path, png, "", None, &TagOptions::default()).unwrap();
        assert_eq!(covered.covers.len(), 1, "{name}");

        // Removing ID3v2 on save is an option; it shifts the stream.
        let mut options = TagOptions::default();
        options.ape.remove_id3v2 = true;
        write_fields_with_options(
            &path,
            &fields(&[("TITLE", &["Third"])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let after = fs::read(&path).unwrap();
        assert_eq!(&after[..stream.len()], stream.as_slice(), "{name}");
        let reread = read_entry_with_options(&path, &[], &options).unwrap();
        assert_eq!(value(&reread, "TITLE").as_deref(), Some("Third"));
        assert_eq!(value(&reread, "ARTIST").as_deref(), Some("A|B"));

        // Utils → Remove tag types and Remove Tag leave the bare stream.
        remove_tag_kinds_with_options(&path, &[RemovableTag::Id3v1], false, &options).unwrap();
        assert!(fs::read(&path).unwrap()[stream.len()..].starts_with(b"APETAGEX"));
        remove_tags(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), stream, "{name}");
    }
}

#[test]
fn damaged_streams_are_refused_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("short.tak");
    fs::write(&path, b"tBaK\x01\x20\x00\x00").unwrap();
    assert!(read_entry(&path).is_err());
    assert!(write_fields(&path, &fields(&[("TITLE", &["x"])])).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"tBaK\x01\x20\x00\x00");
}

fn tool(name: &str) -> bool {
    Command::new(name).arg("-version").output().is_ok()
}

fn decoded_md5(path: &Path) -> String {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-f", "md5", "-"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn ffmpeg_tta_decodes_identically_after_tagging() {
    if !tool("ffmpeg") || !tool("ffprobe") {
        eprintln!("Skipping: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("encoded.tta");
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
        ])
        .args(["-ac", "2", "-c:a", "tta", "-metadata", "title=ffmpeg title"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let before = decoded_md5(&path);
    let mut ape = TagOptions::default();
    ape.mpeg.read_ape = true;
    ape.mpeg.write_ape = true;
    let entry = read_entry_with_options(&path, &[], &ape).unwrap();
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("ffmpeg title"));
    assert_eq!(entry.sample_rate, 44_100);
    assert!((entry.duration - 1.0).abs() < 0.01);

    write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Tagged"]), ("ALBUM", &["Disc"])]),
        false,
        &[],
        &ape,
    )
    .unwrap();
    assert_eq!(decoded_md5(&path), before);
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format_tags=title,album",
            "-of",
            "compact",
        ])
        .arg(&path)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&probe.stdout);
    assert!(
        text.contains("title=Tagged") && text.contains("album=Disc"),
        "{text}"
    );
}

#[test]
fn ffprobe_agrees_with_the_tak_stream_information() {
    if !tool("ffprobe") {
        eprintln!("Skipping: ffprobe is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("header.tak");
    fs::write(&path, tak(2, 24, 96_000, 192_000, &payload(64))).unwrap();
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=sample_rate,channels,bits_per_raw_sample,duration_ts",
            "-of",
            "compact",
        ])
        .arg(&path)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("sample_rate=96000"), "{text}");
    assert!(text.contains("channels=2"), "{text}");
    assert!(text.contains("duration_ts=192000"), "{text}");
    let entry = read_entry(&path).unwrap();
    assert_eq!((entry.sample_rate, entry.channels), (96_000, 2));
    assert!((entry.duration - 2.0).abs() < 1e-9);
}

#[test]
fn mutagen_reads_what_was_written() {
    let probe = Command::new("python3")
        .args(["-c", "import mutagen"])
        .output();
    if !probe.is_ok_and(|o| o.status.success()) {
        eprintln!("Skipping: python3 with mutagen is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let ofr_path = dir.path().join("check.ofr");
    fs::write(&ofr_path, ofr(2, 2, 44_100, 88_200, &payload(4000))).unwrap();
    write_fields(
        &ofr_path,
        &fields(&[("TITLE", &["Ofr title"]), ("ARTIST", &["X", "Y"])]),
    )
    .unwrap();
    let tta_path = dir.path().join("check.tta");
    fs::write(&tta_path, tta(2, 16, 44_100, 88_200, &payload(4000))).unwrap();
    write_fields(&tta_path, &fields(&[("TITLE", &["Tta title"])])).unwrap();
    let script = r#"
import sys
from mutagen.optimfrog import OptimFROG
from mutagen.trueaudio import TrueAudio
o = OptimFROG(sys.argv[1])
print(o.info.sample_rate, o.info.channels, round(o.info.length, 3))
print(str(o.tags["Title"]), "|".join(o.tags["Artist"]))
t = TrueAudio(sys.argv[2])
print(t.info.sample_rate, round(t.info.length, 3), str(t.tags["TIT2"]))
"#;
    let output = Command::new("python3")
        .args(["-I", "-c", script])
        .arg(&ofr_path)
        .arg(&tta_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "44100 2 2.0\nOfr title X|Y\n44100 2.0 Tta title\n"
    );
}

#[test]
fn same_size_tag_edits_rewrite_only_the_tags_and_undo_exactly() {
    use tag_core::history::{Journal, Limits};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.tak");
    let mut original = tak(2, 16, 44_100, 44_100 * 60, &payload(4 << 20));
    original.extend(ape_bytes(&[("Title", "Aaaa")]));
    fs::write(&path, &original).unwrap();
    let mut history = Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
    let key = path.to_string_lossy().into_owned();
    let capture = history.begin("Save tags", None, std::slice::from_ref(&key));
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let options = TagOptions {
        preserve_modified_time: true,
        ..Default::default()
    };
    write_fields_with_options(
        &path,
        &fields(&[("Title", &["Bbbb"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    history.commit(capture, &[], &[]);
    let edited = fs::read(&path).unwrap();
    assert_eq!(edited.len(), original.len());
    assert_eq!(
        edited[..original.len() - 64],
        original[..original.len() - 64]
    );
    if tag_core::clones_in(dir.path()) {
        assert!(
            history.retained_bytes() <= 64 * 1024,
            "{} bytes kept for a tag-only edit",
            history.retained_bytes()
        );
    }
    history.undo().unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
}
