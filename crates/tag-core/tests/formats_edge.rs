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

fn encode(path: &Path, codec: &str) -> bool {
    if !tool("ffmpeg") {
        eprintln!("Skipping: ffmpeg is required");
        return false;
    }
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
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

/// The item atom `ident` with one `data` atom of `code` holding `payload`.
fn item(ident: &[u8; 4], code: u8, payload: &[u8]) -> Vec<u8> {
    let mut data = ((16 + payload.len()) as u32).to_be_bytes().to_vec();
    data.extend(b"data");
    data.extend([0, 0, 0, code, 0, 0, 0, 0]);
    data.extend(payload);
    let mut item = ((8 + data.len()) as u32).to_be_bytes().to_vec();
    item.extend(ident);
    item.extend(data);
    item
}

#[test]
fn legacy_eight_byte_integer_atoms_are_read_kept_and_editable() {
    use lofty::mp4::{Atom, AtomData, AtomIdent, DataType, Mp4File};
    use lofty::prelude::*;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.m4a");
    if !encode(&path, "aac") {
        return;
    }
    let album: u64 = 1_234_567_890_123;
    let catalog: u64 = 9_876_543_210;
    let mut file = Mp4File::read_from(
        &mut fs::File::open(&path).unwrap(),
        lofty::config::ParseOptions::new(),
    )
    .unwrap();
    let mut ilst = file.ilst().cloned().unwrap_or_default();
    ilst.set_title("Legacy".into());
    // Older iTunes versions label 8-byte IDs as 32-bit signed integers.
    for (ident, value) in [(*b"plID", album), (*b"cnID", catalog)] {
        ilst.insert(Atom::new(
            AtomIdent::Fourcc(ident),
            AtomData::Unknown {
                code: DataType::BeSignedInteger,
                data: value.to_be_bytes().to_vec(),
            },
        ));
    }
    file.set_ilst(ilst);
    file.save_to_path(&path, lofty::config::WriteOptions::default())
        .unwrap();
    let plid = item(b"plID", 21, &album.to_be_bytes());
    let cnid = item(b"cnID", 21, &catalog.to_be_bytes());
    let original = fs::read(&path).unwrap();
    assert!(contains(&original, &plid).is_some() && contains(&original, &cnid).is_some());
    // lofty's strict reader refuses the 8-byte cnID (it special-cases plID),
    // and its lenient reader drops it.
    let strict = |path: &Path| {
        lofty::probe::Probe::open(path)
            .unwrap()
            .options(
                lofty::config::ParseOptions::new().parsing_mode(lofty::config::ParsingMode::Strict),
            )
            .read()
    };
    assert!(strict(&path).is_err());

    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "ITUNESALBUMID"), Some(album.to_string()));
    assert_eq!(value(&entry, "ITUNESCATALOGID"), Some(catalog.to_string()));
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("Legacy"));

    // Other edits keep both atoms byte for byte.
    let written = write_fields(&path, &fields(&[("TITLE", &["Edited"])])).unwrap();
    assert_eq!(value(&written, "ITUNESALBUMID"), Some(album.to_string()));
    assert_eq!(
        value(&written, "ITUNESCATALOGID"),
        Some(catalog.to_string())
    );
    let after = fs::read(&path).unwrap();
    assert!(contains(&after, &plid).is_some() && contains(&after, &cnid).is_some());

    // Editing the fields writes their documented types.
    let written = write_fields(
        &path,
        &fields(&[("ITUNESALBUMID", &["77"]), ("ITUNESCATALOGID", &["78"])]),
    )
    .unwrap();
    assert_eq!(value(&written, "ITUNESALBUMID").as_deref(), Some("77"));
    assert_eq!(value(&written, "ITUNESCATALOGID").as_deref(), Some("78"));
    let after = fs::read(&path).unwrap();
    assert!(contains(&after, &plid).is_none());
    assert!(contains(&after, &item(b"plID", 78, &77u64.to_be_bytes())).is_some());
    assert!(contains(&after, &cnid).is_none());
    assert!(strict(&path).is_ok());
}

/// Twenty MPEG-1 Layer III frames (128 kbit/s, 44.1 kHz).
fn mpeg_frames() -> Vec<u8> {
    let mut frame = vec![0u8; 417];
    frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
    frame.repeat(20)
}
fn syncsafe(value: usize) -> [u8; 4] {
    [
        ((value >> 21) & 127) as u8,
        ((value >> 14) & 127) as u8,
        ((value >> 7) & 127) as u8,
        (value & 127) as u8,
    ]
}
/// zlib data holding `data` in one stored (uncompressed) deflate block.
fn zlib(data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x78, 0x01, 0x01];
    let length = data.len() as u16;
    bytes.extend(length.to_le_bytes());
    bytes.extend((!length).to_le_bytes());
    bytes.extend(data);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    bytes.extend(((b << 16) | a).to_be_bytes());
    bytes
}
fn raw_frame(major: u8, id: &[u8; 4], flags: [u8; 2], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    if major == 3 {
        bytes.extend((body.len() as u32).to_be_bytes());
    } else {
        bytes.extend(syncsafe(body.len()));
    }
    bytes.extend(flags);
    bytes.extend(body);
    bytes
}
fn raw_tag(major: u8, frames: &[Vec<u8>], padding: usize) -> Vec<u8> {
    let body = frames.concat();
    let mut bytes = b"ID3".to_vec();
    bytes.extend([major, 0, 0]);
    bytes.extend(syncsafe(body.len() + padding));
    bytes.extend(body);
    bytes.resize(bytes.len() + padding, 0);
    bytes
}

#[test]
fn compressed_and_encrypted_id3v23_frames_are_read_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flags.mp3");
    let album = b"\x00Compressed album";
    let mut compressed_body = (album.len() as u32).to_be_bytes().to_vec();
    compressed_body.extend(zlib(album));
    let compressed = raw_frame(3, b"TALB", [0, 0x80], &compressed_body);
    let encrypted = raw_frame(3, b"TXXX", [0, 0x40], b"\x81\x13\x37opaque cipher bytes");
    let title = raw_frame(3, b"TIT2", [0, 0], b"\x00Plain title");
    let tag = raw_tag(3, &[title, compressed.clone(), encrypted.clone()], 256);
    let audio = mpeg_frames();
    fs::write(&path, [tag.as_slice(), audio.as_slice()].concat()).unwrap();

    // lofty alone fails on the encrypted ID3v2.3 frame.
    assert!(lofty::probe::Probe::open(&path)
        .unwrap()
        .options(
            lofty::config::ParseOptions::new().parsing_mode(lofty::config::ParsingMode::Strict)
        )
        .read()
        .is_err());
    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "TITLE").as_deref(), Some("Plain title"));
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("Compressed album"));
    assert_eq!(technical(&entry, "_ID3V2_UNKNOWN_FRAMES"), "TXXX");

    // Editing another field copies both frames byte for byte; the tag keeps
    // its size (it is rewritten in place) and the audio is untouched.
    write_fields(&path, &fields(&[("ARTIST", &["Someone"])])).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(contains(&after, &compressed).is_some());
    assert!(contains(&after, &encrypted).is_some());
    let reread = read_entry(&path).unwrap();
    assert_eq!(value(&reread, "ARTIST").as_deref(), Some("Someone"));
    assert_eq!(value(&reread, "ALBUM").as_deref(), Some("Compressed album"));
    assert_eq!(value(&reread, "TITLE").as_deref(), Some("Plain title"));
    let start = contains(&after, &audio[..417 * 2]).unwrap();
    assert_eq!(&after[start..start + audio.len()], audio.as_slice());
    assert_eq!(start, tag.len(), "the tag was rewritten in place");

    // Editing the compressed field writes it plain; the encrypted frame stays.
    write_fields(&path, &fields(&[("ALBUM", &["New album"])])).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(contains(&after, &compressed).is_none());
    assert!(contains(&after, &encrypted).is_some());
    assert_eq!(
        value(&read_entry(&path).unwrap(), "ALBUM").as_deref(),
        Some("New album")
    );

    // Utils → Remove unsupported fields drops the encrypted frame.
    tag_core::remove_unsupported_fields_with_options(&path, false, &TagOptions::default()).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(contains(&after, &encrypted).is_none());
    assert!(technical(&read_entry(&path).unwrap(), "_ID3V2_UNKNOWN_FRAMES").is_empty());
}

#[test]
fn id3v24_flagged_frames_are_kept_in_v24_and_block_a_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("flags24.mp3");
    let album = b"\x03Album \xc3\xa9";
    let mut compressed_body = syncsafe(album.len()).to_vec();
    compressed_body.extend(zlib(album));
    let compressed = raw_frame(4, b"TALB", [0, 0x09], &compressed_body);
    let mut encrypted_body = vec![0x82];
    encrypted_body.extend(syncsafe(12));
    encrypted_body.extend(b"cipher-bytes");
    let encrypted = raw_frame(4, b"TPE2", [0, 0x05], &encrypted_body);
    let tag = raw_tag(4, &[compressed.clone(), encrypted.clone()], 64);
    let audio = mpeg_frames();
    let original = [tag.as_slice(), audio.as_slice()].concat();
    fs::write(&path, &original).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(value(&entry, "ALBUM").as_deref(), Some("Album é"));
    assert_eq!(technical(&entry, "_ID3V2_UNKNOWN_FRAMES"), "TPE2");

    // The default ID3v2.3 target cannot carry the encrypted frame over.
    let error = write_fields(&path, &fields(&[("TITLE", &["T"])])).unwrap_err();
    assert!(error.contains("Encrypted"), "{error}");
    assert_eq!(fs::read(&path).unwrap(), original);

    // Writing ID3v2.4 keeps both frames byte for byte.
    let mut options = TagOptions::default();
    options.mpeg.id3v2_format = tag_core::Id3v2Format::V24Utf8;
    options.mpeg.write_id3v1 = false;
    write_fields_with_options(&path, &fields(&[("TITLE", &["T"])]), false, &[], &options).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(contains(&after, &compressed).is_some());
    assert!(contains(&after, &encrypted).is_some());
    assert!(after.ends_with(&audio));
    let reread = read_entry_with_options(&path, &[], &options).unwrap();
    assert_eq!(value(&reread, "TITLE").as_deref(), Some("T"));
    assert_eq!(value(&reread, "ALBUM").as_deref(), Some("Album é"));

    // Removing the encrypted frame lets the conversion through; the
    // compressed album is converted and written plain.
    tag_core::remove_unsupported_fields_with_options(&path, false, &options).unwrap();
    write_fields(&path, &fields(&[("TITLE", &["T3"])])).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(b"ID3\x03"));
    let reread = read_entry(&path).unwrap();
    assert_eq!(value(&reread, "ALBUM").as_deref(), Some("Album é"));
    assert_eq!(value(&reread, "TITLE").as_deref(), Some("T3"));
}

#[test]
fn described_comments_lyrics_and_unique_file_ids_roundtrip() {
    use lofty::id3::v2::{CommentFrame, UniqueFileIdentifierFrame, UnsynchronizedTextFrame};
    for format in [
        tag_core::Id3v2Format::V24Utf8,
        tag_core::Id3v2Format::V23Utf16,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("described.mp3");
        let mut tag = Id3v2Tag::new();
        for (description, text) in [
            ("", "Main"),
            ("iTunNORM", " 00000A1B 00000B2C"),
            ("Songs-DB_Custom1", "Mood"),
        ] {
            tag.insert(Frame::Comment(CommentFrame::new(
                TextEncoding::UTF8,
                *b"eng",
                description.to_owned(),
                text.to_owned(),
            )));
        }
        for (language, description, text) in
            [(*b"eng", "", "Main lyrics"), (*b"deu", "Verse", "Strophe")]
        {
            tag.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
                TextEncoding::UTF8,
                language,
                description.to_owned(),
                text.to_owned(),
            )));
        }
        for (owner, id) in [
            ("http://musicbrainz.org", "mbid"),
            ("http://example.com", "abc"),
        ] {
            tag.insert(Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame::new(
                owner.to_owned(),
                id.as_bytes().to_vec(),
            )));
        }
        let mut bytes = Vec::new();
        tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
        let audio = mpeg_frames();
        bytes.extend(&audio);
        fs::write(&path, &bytes).unwrap();
        let mut options = TagOptions::default();
        options.mpeg.id3v2_format = format;
        options.mpeg.write_id3v1 = false;
        let read = |path: &Path| read_entry_with_options(path, &[], &options).unwrap();

        let entry = read(&path);
        assert_eq!(value(&entry, "COMMENT").as_deref(), Some("Main"));
        assert_eq!(
            value(&entry, "COMMENT iTunNORM").as_deref(),
            Some(" 00000A1B 00000B2C")
        );
        assert_eq!(
            value(&entry, "COMMENT Songs-DB_Custom1").as_deref(),
            Some("Mood")
        );
        assert_eq!(
            value(&entry, "UNSYNCEDLYRICS").as_deref(),
            Some("eng||Main lyrics")
        );
        assert_eq!(
            value(&entry, "UNSYNCEDLYRICS Verse").as_deref(),
            Some("deu||Strophe")
        );
        assert_eq!(
            value(&entry, "UNIQUEFILEID").as_deref(),
            Some("http://example.com|abc")
        );
        assert_eq!(
            value(&entry, "MUSICBRAINZ_TRACKID").as_deref(),
            Some("mbid")
        );

        // Editing COMMENT leaves the described comments alone.
        let write =
            |updates: Fields| write_fields_with_options(&path, &updates, false, &[], &options);
        write(fields(&[("COMMENT", &["One"])])).unwrap();
        let entry = read(&path);
        assert_eq!(
            value(&entry, "COMMENT").as_deref(),
            Some("One"),
            "{format:?}"
        );
        assert_eq!(
            value(&entry, "COMMENT iTunNORM").as_deref(),
            Some(" 00000A1B 00000B2C")
        );
        // Described fields are edited, added and removed by name.
        write(fields(&[
            ("COMMENT Songs-DB_Custom1", &["Calm"]),
            ("COMMENT iTunNORM", &[]),
            ("COMMENT New note", &["Added"]),
            ("UNSYNCEDLYRICS Verse", &["eng||Verse", "deu||Strophe 2"]),
            ("UNSYNCEDLYRICS", &[]),
            (
                "UNIQUEFILEID",
                &["http://example.com|xyz", "http://other.org|42"],
            ),
        ]))
        .unwrap();
        let entry = read(&path);
        assert_eq!(
            value(&entry, "COMMENT Songs-DB_Custom1").as_deref(),
            Some("Calm")
        );
        assert_eq!(value(&entry, "COMMENT iTunNORM"), None);
        assert_eq!(value(&entry, "COMMENT New note").as_deref(), Some("Added"));
        assert_eq!(value(&entry, "COMMENT").as_deref(), Some("One"));
        assert_eq!(
            value(&entry, "UNSYNCEDLYRICS Verse").as_deref(),
            Some("eng||Verse|deu||Strophe 2")
        );
        assert_eq!(value(&entry, "UNSYNCEDLYRICS"), None);
        assert_eq!(
            value(&entry, "UNIQUEFILEID").as_deref(),
            Some("http://example.com|xyz|http://other.org|42")
        );
        assert_eq!(
            value(&entry, "MUSICBRAINZ_TRACKID").as_deref(),
            Some("mbid")
        );
        assert!(fs::read(&path).unwrap().ends_with(&audio));

        // Invalid values are refused and leave the file untouched.
        let before = fs::read(&path).unwrap();
        for (name, invalid) in [
            ("UNIQUEFILEID", "no owner"),
            ("UNIQUEFILEID", "http://musicbrainz.org|x"),
            ("UNSYNCEDLYRICS Verse", "no language"),
        ] {
            assert!(write(fields(&[(name, &[invalid])])).is_err(), "{invalid}");
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert!(write(fields(&[("UNIQUEFILEID", &["a|1", "a|2"])])).is_err());
        // One comment per description: further ones need their own description.
        assert!(write(fields(&[("COMMENT", &["One", "Two"])])).is_err());
        assert!(write(fields(&[("UNSYNCEDLYRICS", &["eng||a", "eng||b"])])).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn reuse_unmodified_id3v2_frames_toggle() {
    use lofty::prelude::AudioFile;
    for reuse in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("remap.mp3");
        let mut tag = Id3v2Tag::new();
        tag.insert_user_text("MOOD".into(), "Calm".into());
        tag.insert_user_text("MUSICBRAINZ_ALBUMID".into(), "album-id".into());
        tag.insert_user_text("Unmapped".into(), "kept".into());
        let mut bytes = Vec::new();
        tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
        bytes.extend(mpeg_frames());
        fs::write(&path, &bytes).unwrap();
        let mut options = TagOptions::default();
        options.mpeg.id3v2_format = tag_core::Id3v2Format::V24Utf8;
        options.mpeg.write_id3v1 = false;
        options.advanced.reuse_id3v2_frames = reuse;
        write_fields_with_options(&path, &fields(&[("TITLE", &["T"])]), false, &[], &options)
            .unwrap();
        let written = lofty::mpeg::MpegFile::read_from(
            &mut fs::File::open(&path).unwrap(),
            lofty::config::ParseOptions::new().implicit_conversions(false),
        )
        .unwrap()
        .id3v2()
        .cloned()
        .unwrap();
        let user = |description: &str| {
            (&written)
                .into_iter()
                .any(|f| matches!(f, Frame::UserText(t) if t.description == description))
        };
        let tmoo = written.get_text(&FrameId::new("TMOO").unwrap());
        // Off (the default): moved to their documented frames on any save.
        assert_eq!(tmoo.is_some(), !reuse, "reuse {reuse}");
        assert_eq!(user("MOOD"), reuse);
        assert_eq!(user("MusicBrainz Album Id"), !reuse);
        assert_eq!(user("MUSICBRAINZ_ALBUMID"), reuse);
        assert!(user("Unmapped"));
        let entry = read_entry_with_options(&path, &[], &options).unwrap();
        assert_eq!(value(&entry, "MOOD").as_deref(), Some("Calm"));
        assert_eq!(
            value(&entry, "MUSICBRAINZ_ALBUMID").as_deref(),
            Some("album-id")
        );
    }
}

#[test]
fn mp4_reuse_and_single_atom_toggles() {
    use lofty::mp4::{Atom, AtomData, AtomIdent, Mp4File};
    use lofty::prelude::*;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.m4a");
    if !encode(&source, "aac") {
        return;
    }
    let mut file = Mp4File::read_from(
        &mut fs::File::open(&source).unwrap(),
        lofty::config::ParseOptions::new(),
    )
    .unwrap();
    let mut ilst = file.ilst().cloned().unwrap_or_default();
    ilst.insert(Atom::new(
        AtomIdent::Freeform {
            mean: "com.apple.iTunes".into(),
            name: "COMPOSER".into(),
        },
        AtomData::UTF8("Bach".into()),
    ));
    file.set_ilst(ilst);
    file.save_to_path(&source, WriteOptions::default()).unwrap();
    let freeform = b"\x00\x00\x00\x14name\x00\x00\x00\x00COMPOSER";
    let composer = b"\xa9wrt";
    // ©ART holding `values` as one atom with several `data` atoms.
    let artist_atom = |values: &[&str]| {
        let mut body = Vec::new();
        for value in values {
            body.extend(((16 + value.len()) as u32).to_be_bytes());
            body.extend(b"data\x00\x00\x00\x01\x00\x00\x00\x00");
            body.extend(value.as_bytes());
        }
        let mut atom = ((8 + body.len()) as u32).to_be_bytes().to_vec();
        atom.extend(b"\xa9ART");
        atom.extend(body);
        atom
    };
    for (reuse, single) in [(false, true), (true, false)] {
        let path = dir.path().join(format!("{reuse}-{single}.m4a"));
        fs::copy(&source, &path).unwrap();
        let mut options = TagOptions::default();
        options.advanced.reuse_mp4_atoms = reuse;
        options.advanced.mp4_single_atom = single;
        write_fields_with_options(
            &path,
            &fields(&[("ARTIST", &["A", "B"])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(contains(&bytes, freeform).is_some(), reuse, "reuse {reuse}");
        assert_eq!(
            contains(&bytes, composer).is_some(),
            !reuse,
            "reuse {reuse}"
        );
        if single {
            assert!(contains(&bytes, &artist_atom(&["A", "B"])).is_some());
        } else {
            // Two ©ART atoms, one per value, side by side.
            let (first, second) = (artist_atom(&["A"]), artist_atom(&["B"]));
            let a = contains(&bytes, &first).unwrap();
            let b = a + first.len();
            assert_eq!(&bytes[b..b + second.len()], second.as_slice());
        }
        let entry = read_entry_with_options(&path, &[], &options).unwrap();
        assert_eq!(value(&entry, "ARTIST").as_deref(), Some("A|B"));
        assert_eq!(value(&entry, "COMPOSER").as_deref(), Some("Bach"));
    }
}

#[test]
fn an_ape_tag_whose_header_looks_like_id3v1_is_read_and_written() {
    // A 131-byte APE tag (header, one 67-byte item, footer) puts the "TAG" of
    // its header exactly where an ID3v1 tag would start.
    let ape = ape_bytes(&[("encoder", &"L".repeat(51))]);
    assert_eq!(ape.len(), 131);
    assert_eq!(&ape[3..6], b"TAG");
    let dir = tempfile::tempdir().unwrap();
    for (name, stream) in [
        ("track.wv", wavpack(44_100, 2000)),
        ("track.mp3", mpeg_frames()),
    ] {
        let path = dir.path().join(name);
        let mut bytes = id3_bytes("Prepended");
        bytes.extend(&stream);
        bytes.extend(&ape);
        fs::write(&path, &bytes).unwrap();
        let mut options = TagOptions::default();
        options.mpeg.write_ape = true;
        options.mpeg.remove_ape = false;
        options.mpeg.write_id3v1 = false;
        options.mpeg.read_id3v2 = false;
        let entry = read_entry_with_options(&path, &[], &options).unwrap();
        assert_eq!(technical(&entry, "_TAG_READ"), "APEv2", "{name}");
        assert_eq!(value(&entry, "ENCODER"), Some("L".repeat(51)), "{name}");
        assert!(!technical(&entry, "_TAG").contains("ID3v1"), "{name}");
        write_fields_with_options(
            &path,
            &fields(&[("ARTIST", &["A", "B"])]),
            false,
            &[],
            &options,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let entry = read_entry_with_options(&path, &[], &options).unwrap();
        assert_eq!(value(&entry, "ARTIST").as_deref(), Some("A|B"), "{name}");
        assert_eq!(value(&entry, "ENCODER"), Some("L".repeat(51)), "{name}");
        let after = fs::read(&path).unwrap();
        let start = contains(&after, &stream[..64]).unwrap();
        assert_eq!(
            &after[start..start + stream.len()],
            stream.as_slice(),
            "{name}"
        );
    }
}
