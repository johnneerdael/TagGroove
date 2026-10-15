//! Format and mapping edge cases: AAC (ADTS) and WavPack tag kinds, and the
//! byte-level guarantee that every write leaves the audio stream (and every
//! tag it was not asked to change) untouched.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame},
    prelude::TagExt,
    tag::ItemValue,
    TextEncoding,
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    read_entry, read_entry_with_options, remove_tag_kinds_with_options, remove_tags,
    remove_tags_with_options, write_fields, write_fields_with_options, AudioEntry, Fields,
    RemovableTag, TagOptions,
};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}
fn value(entry: &AudioEntry, key: &str) -> Option<String> {
    entry
        .fields
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.join("|"))
}
fn technical(entry: &AudioEntry, key: &str) -> String {
    entry
        .technical
        .get(key)
        .map(|v| v.join("|"))
        .unwrap_or_default()
}
fn contains(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
/// Payload bytes below 0xFF: no frame sync and no tag markers.
fn payload(length: usize) -> Vec<u8> {
    (0..length).map(|i| (i * 37 % 89 + 1) as u8).collect()
}

/// `count` ADTS frames (MPEG-4 AAC LC, 44.1 kHz, stereo, no CRC).
fn adts(count: usize, body: usize) -> Vec<u8> {
    let length = 7 + body;
    let mut stream = Vec::new();
    for index in 0..count {
        stream.extend([
            0xFF,
            0xF1,
            (1 << 6) | (4 << 2),
            (2 << 6) | ((length >> 11) & 3) as u8,
            ((length >> 3) & 0xFF) as u8,
            (((length & 7) << 5) | 0x1F) as u8,
            0xFC,
        ]);
        stream.extend(payload(body + index % 3).into_iter().take(body));
    }
    stream
}

/// One WavPack block: 16-bit stereo at 44.1 kHz holding `samples` samples.
fn wavpack(samples: u32, body: usize) -> Vec<u8> {
    let mut block = b"wvpk".to_vec();
    block.extend(((24 + body) as u32).to_le_bytes());
    block.extend(0x410u16.to_le_bytes());
    block.extend([0, 0]);
    block.extend(samples.to_le_bytes());
    block.extend(0u32.to_le_bytes());
    block.extend(samples.to_le_bytes());
    // 2 bytes per sample, initial and final block, sample rate index 9.
    let flags: u32 = 1 | 0x800 | 0x1000 | (9 << 23);
    block.extend(flags.to_le_bytes());
    block.extend(0u32.to_le_bytes());
    block.extend(payload(body));
    block
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
        FrameId::new("TIT2").unwrap(),
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

#[test]
fn aac_reads_every_tag_kind_and_follows_the_mpeg_policy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("track.aac");
    let stream = adts(40, 120);
    let id3 = id3_bytes("From ID3v2");
    let mut bytes = id3.clone();
    bytes.extend(&stream);
    bytes.extend(ape_bytes(&[("Title", "From APE"), ("Artist", "Band")]));
    bytes.extend(id3v1("From ID3v1"));
    fs::write(&path, &bytes).unwrap();

    // APE is displayed first, then ID3v2, then ID3v1 (Options → Tags → Mpeg).
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "AAC");
    assert_eq!(technical(&entry, "_CODEC"), "AAC");
    assert_eq!((entry.sample_rate, entry.channels), (44_100, 2));
    assert!(entry.duration > 0.0 && entry.bitrate > 0);
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("From APE"));
    assert_eq!(technical(&entry, "_TAG_READ"), "APEv2");
    let tags = technical(&entry, "_TAG");
    assert!(
        tags.contains("ID3v1") && tags.contains("ID3v2") && tags.contains("APE"),
        "{tags}"
    );
    let mut options = TagOptions::default();
    options.mpeg.read_ape = false;
    let id3_only = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(value(&id3_only, "TITLE").as_deref(), Some("From ID3v2"));
    options.mpeg.read_id3v2 = false;
    let v1_only = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(value(&v1_only, "TITLE").as_deref(), Some("From ID3v1"));

    // Default Mpeg options: ID3v2.3 and ID3v1 are written, APE is removed and
    // its values carried over. The ADTS stream is not touched.
    let written = write_fields(&path, &fields(&[("TITLE", &["Ünïcode"])])).unwrap();
    assert_eq!(value(&written, "TITLE").as_deref(), Some("Ünïcode"));
    assert_eq!(value(&written, "ARTIST").as_deref(), Some("Band"));
    assert_eq!(technical(&written, "_TAG_READ"), "ID3v2.3");
    assert_eq!(written.format, "AAC");
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(b"ID3\x03"));
    let start = contains(&after, &stream[..64]).unwrap();
    assert_eq!(&after[start..start + stream.len()], stream.as_slice());
    assert_eq!(start + stream.len() + 128, after.len(), "APE removed");
    assert_eq!(&after[after.len() - 128..after.len() - 125], b"TAG");

    // APEv2 writing, without ID3v1 and ID3v2.
    let mut options = TagOptions::default();
    options.mpeg.write_ape = true;
    options.mpeg.write_id3v1 = false;
    options.mpeg.write_id3v2 = false;
    options.mpeg.remove_ape = false;
    let ape = write_fields_with_options(
        &path,
        &fields(&[("ALBUM", &["Ape album"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(technical(&ape, "_TAG_READ"), "APEv2");
    assert_eq!(value(&ape, "ALBUM").as_deref(), Some("Ape album"));
    assert_eq!(value(&ape, "TITLE").as_deref(), Some("Ünïcode"));
    let after = fs::read(&path).unwrap();
    assert!(!after.starts_with(b"ID3"), "ID3v2 removed");
    assert!(after.starts_with(&stream));
    assert_eq!(&after[stream.len()..stream.len() + 8], b"APETAGEX");

    // Utils → Remove tag types accepts AAC and keeps the stream.
    remove_tag_kinds_with_options(
        &path,
        &[RemovableTag::Ape, RemovableTag::Id3v1, RemovableTag::Id3v2],
        false,
        &options,
    )
    .unwrap();
    assert_eq!(fs::read(&path).unwrap(), stream);
}

#[test]
fn aac_remove_tag_and_id3v1_only_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.aac");
    let stream = adts(20, 90);
    fs::write(&path, &stream).unwrap();
    let mut options = TagOptions::default();
    options.mpeg.write_id3v2 = false;
    let written = write_fields_with_options(
        &path,
        &fields(&[("TITLE", &["Short"]), ("YEAR", &["2001"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(technical(&written, "_TAG_READ"), "ID3v1");
    let after = fs::read(&path).unwrap();
    assert_eq!(after.len(), stream.len() + 128);
    assert!(after.starts_with(&stream));
    remove_tags(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), stream);
}

#[test]
fn wavpack_leading_id3v2_is_parsed_kept_and_removable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("track.wv");
    let stream = wavpack(88_200, 3000);
    let id3 = id3_bytes("Hidden ID3v2");
    let mut bytes = id3.clone();
    bytes.extend(&stream);
    bytes.extend(ape_bytes(&[("Title", "From APE")]));
    fs::write(&path, &bytes).unwrap();

    // The ID3v2 tag is listed but never displayed (Options → Tags → Ape, Mpc).
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.format, "WavPack");
    assert_eq!(technical(&entry, "_CODEC"), "WavPack");
    assert_eq!((entry.sample_rate, entry.channels), (44_100, 2));
    assert_eq!(technical(&entry, "_BITSPERSAMPLE"), "16");
    assert!((entry.duration - 2.0).abs() < 0.01, "{}", entry.duration);
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("From APE"));
    assert_eq!(technical(&entry, "_TAG_READ"), "APEv2");
    assert!(technical(&entry, "_TAG").contains("ID3v2"));

    // Saving keeps the ID3v2 tag byte for byte unless Remove ID3v2 is set.
    write_fields(&path, &fields(&[("TITLE", &["New"])])).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(&id3));
    assert_eq!(
        &after[id3.len()..id3.len() + stream.len()],
        stream.as_slice()
    );
    assert_eq!(
        value(&read_entry(&path).unwrap(), "TITLE").as_deref(),
        Some("New")
    );

    // File → Remove tag with the defaults leaves the ID3v2 tag.
    remove_tags(&path).unwrap();
    let after = fs::read(&path).unwrap();
    assert_eq!(after, [id3.as_slice(), stream.as_slice()].concat());

    // Utils → Remove tag types removes it.
    let options = TagOptions::default();
    remove_tag_kinds_with_options(&path, &[RemovableTag::Id3v2], false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), stream);

    // So do saving and File → Remove tag with Remove ID3v2 enabled.
    let mut bytes = id3.clone();
    bytes.extend(&stream);
    fs::write(&path, &bytes).unwrap();
    let mut options = TagOptions::default();
    options.ape.remove_id3v2 = true;
    write_fields_with_options(&path, &fields(&[("ALBUM", &["A"])]), false, &[], &options).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(&stream));
    assert_eq!(&after[stream.len()..stream.len() + 8], b"APETAGEX");
    let mut bytes = id3.clone();
    bytes.extend(&stream);
    fs::write(&path, &bytes).unwrap();
    remove_tags_with_options(&path, false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), stream);
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
fn ffmpeg_aac_and_wavpack_decode_identically_after_tagging() {
    if !tool("ffmpeg") {
        eprintln!("Skipping: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, codec) in [("encoded.aac", "aac"), ("encoded.wv", "wavpack")] {
        let path = dir.path().join(name);
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
            ])
            .args(["-ac", "2", "-c:a", codec])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let before = decoded_md5(&path);
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.sample_rate, 44_100, "{name}");
        assert!(
            (entry.duration - 1.0).abs() < 0.1,
            "{name}: {}",
            entry.duration
        );
        // Prepend an ID3v2 tag, as some taggers do, then save twice.
        let mut bytes = id3_bytes("Prepended");
        bytes.extend(fs::read(&path).unwrap());
        fs::write(&path, &bytes).unwrap();
        let mut options = TagOptions::default();
        options.mpeg.write_ape = true;
        options.mpeg.remove_ape = false;
        // ffmpeg reads no WavPack stream behind an ID3v2 tag: saving removes it.
        options.ape.remove_id3v2 = true;
        write_fields_with_options(
            &path,
            &fields(&[("TITLE", &["Tagged"]), ("ARTIST", &["A", "B"])]),
            false,
            &[],
            &options,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        write_fields_with_options(
            &path,
            &fields(&[("ALBUM", &["Disc"])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let entry = read_entry_with_options(&path, &[], &options).unwrap();
        assert_eq!(value(&entry, "TITLE").as_deref(), Some("Tagged"), "{name}");
        assert_eq!(value(&entry, "ARTIST").as_deref(), Some("A|B"), "{name}");
        assert_eq!(value(&entry, "ALBUM").as_deref(), Some("Disc"), "{name}");
        assert_eq!(decoded_md5(&path), before, "{name}");
        assert_eq!(
            fs::read(&path).unwrap().starts_with(b"ID3"),
            codec == "aac",
            "{name}"
        );
    }
}
