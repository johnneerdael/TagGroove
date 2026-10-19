use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    id3::v2::{BinaryFrame, Frame, FrameId, Id3v2Tag},
    prelude::TagExt,
    tag::{ItemKey, ItemValue, Tag, TagItem, TagType},
};
use std::{collections::BTreeMap, fs, path::Path};
use tag_core::{
    add_cover, add_cover_summary, read_entry, read_entry_summary, remove_cover,
    remove_cover_summary, remove_tags, remove_tags_summary, scan_paths, write_fields,
    write_fields_summary, Fields,
};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(key, values)| {
            (
                key.to_string(),
                values.iter().map(|v| v.to_string()).collect(),
            )
        })
        .collect()
}

// Independently generated one-second, mono PCM audio. No vendor media is used.
fn wav(path: &Path) -> Vec<u8> {
    let samples: Vec<u8> = (0..8000)
        .flat_map(|i| ((i % 251) as i16 * 31).to_le_bytes())
        .collect();
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend((36 + samples.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32).to_le_bytes());
    bytes.extend(&samples);
    fs::write(path, bytes).unwrap();
    samples
}

fn wav_payload(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    let mut pos = 12;
    while pos + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if &bytes[pos..pos + 4] == b"data" {
            return bytes[pos + 8..pos + 8 + size].to_vec();
        }
        pos += 8 + size + size % 2;
    }
    panic!("WAV data chunk missing")
}

#[test]
fn real_wav_patch_roundtrip_preserves_unedited_fields_and_audio() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("sample.wav");
    let payload = wav(&path);
    let original = fields(&[
        ("TITLE", &["First"]),
        ("ARTIST", &["Alice", "Bob"]),
        ("ALBUM", &["Album"]),
        ("TRACK", &["3"]),
        ("TOTALTRACKS", &["12"]),
        ("DISCNUMBER", &["2"]),
        ("TOTALDISCS", &["4"]),
        ("YEAR", &["2024-08-21"]),
        ("CUSTOM FIELD", &["first", "second"]),
    ]);
    let entry = write_fields(&path, &original).unwrap();
    assert_eq!(entry.fields, original);
    assert_eq!(entry.sample_rate, 8000);
    assert_eq!(entry.channels, 1);
    assert!((entry.duration - 1.0).abs() < 0.01);
    let entry = write_fields(&path, &fields(&[("title", &["Changed"]), ("ALBUM", &[])])).unwrap();
    let mut expected = original;
    expected.insert("TITLE".into(), vec!["Changed".into()]);
    expected.remove("ALBUM");
    assert_eq!(entry.fields, expected);
    assert_eq!(wav_payload(&path), payload);
}

#[test]
fn untouched_opaque_id3_frame_survives_field_edit() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("opaque.wav");
    wav(&path);
    let mut tag = Id3v2Tag::new();
    let id = FrameId::new("XABC").unwrap();
    tag.insert(Frame::Binary(BinaryFrame::new(
        id.clone(),
        vec![0, 1, 2, 255, 13, 0],
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    write_fields(&path, &fields(&[("TITLE", &["Changed"])])).unwrap();
    let tagged = lofty::read_from_path(&path).unwrap();
    let tag: Id3v2Tag = tagged.tag(TagType::Id3v2).unwrap().clone().into();
    match tag.get(&id).unwrap() {
        Frame::Binary(f) => assert_eq!(f.data.as_ref(), &[0, 1, 2, 255, 13, 0]),
        f => panic!("wrong frame: {f:?}"),
    }
}

#[test]
fn editing_mapped_id3_fields_replaces_case_insensitive_user_text_aliases() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("comment-alias.wav");
    let payload = wav(&path);
    let mut tag = Id3v2Tag::new();
    // FFmpeg writes -metadata comment as TXXX:comment in MP3 files. It is
    // exposed as COMMENT by the reader, just like a standard COMM frame.
    tag.insert_user_text("comment".into(), "Original comment".into());
    tag.insert_user_text("TiTlE".into(), "Shadow title".into());
    tag.insert_user_text("Unmanaged_Field".into(), "Keep Café 猫".into());
    tag.insert(Frame::Text(lofty::id3::v2::TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        lofty::TextEncoding::UTF8,
        "Original title",
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    let before = read_entry(&path).unwrap().fields;
    assert!(before["TITLE"].iter().any(|value| value == "Shadow title"));
    let entry = write_fields(
        &path,
        &fields(&[
            ("COMMENT", &["UAT common — Café 猫"]),
            ("ALBUM", &["New album"]),
        ]),
    )
    .unwrap();
    assert_eq!(entry.fields["COMMENT"], ["UAT common — Café 猫"]);
    assert!(!entry.fields.contains_key("comment"));
    assert_eq!(entry.fields["ALBUM"], ["New album"]);
    assert_eq!(entry.fields["TITLE"], before["TITLE"]);
    assert_eq!(entry.fields["Unmanaged_Field"], ["Keep Café 猫"]);
    let entry = write_fields(
        &path,
        &fields(&[("TITLE", &["Only title"]), ("COMMENT", &[])]),
    )
    .unwrap();
    assert_eq!(entry.fields["TITLE"], ["Only title"]);
    assert!(!entry.fields.contains_key("COMMENT"));
    assert_eq!(entry.fields["Unmanaged_Field"], ["Keep Café 猫"]);
    assert_eq!(wav_payload(&path), payload);
}

#[test]
fn unedited_padded_id3_numbers_and_frame_details_survive_other_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("padded.wav");
    wav(&path);
    let mut tag = Id3v2Tag::new();
    let id = FrameId::new("TRCK").unwrap();
    tag.insert(Frame::Text(lofty::id3::v2::TextInformationFrame::new(
        id.clone(),
        lofty::TextEncoding::UTF8,
        "01/012",
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    write_fields(&path, &fields(&[("TITLE", &["Changed"])])).unwrap();
    let tagged = lofty::read_from_path(&path).unwrap();
    let tag: Id3v2Tag = tagged.tag(TagType::Id3v2).unwrap().clone().into();
    // Read the actual frame; generic Tag conversions normalize number pairs.
    let concrete = lofty::iff::wav::WavFile::read_from(
        &mut fs::File::open(&path).unwrap(),
        lofty::config::ParseOptions::new(),
    )
    .unwrap();
    assert_eq!(concrete.id3v2().unwrap().get_text(&id), Some("01/012"));
    assert!(tag.get(&id).is_some());
}

#[test]
fn field_patch_updates_all_existing_writable_tag_sets() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("multi.wav");
    wav(&path);
    for kind in [TagType::Id3v2, TagType::RiffInfo] {
        let mut tag = Tag::new(kind);
        tag.push(TagItem::new(
            ItemKey::TrackTitle,
            ItemValue::Text("old".into()),
        ));
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
    }
    write_fields(&path, &fields(&[("TITLE", &["new"])])).unwrap();
    let tagged = lofty::read_from_path(&path).unwrap();
    for tag in tagged.tags() {
        assert_eq!(tag.get_string(ItemKey::TrackTitle), Some("new"));
    }
    remove_tags(&path).unwrap();
    assert!(read_entry(&path).unwrap().fields.is_empty());
    assert!(!lofty::read_from_path(&path).unwrap().contains_tag());
}

#[test]
fn covers_roundtrip_remove_by_index_and_do_not_destroy_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cover.wav");
    let payload = wav(&path);
    let png = STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap();
    let image = tmp.path().join("cover.png");
    fs::write(&image, &png).unwrap();
    write_fields(&path, &fields(&[("TITLE", &["Music"])])).unwrap();
    for _ in 0..2 {
        add_cover(&path, &image).unwrap();
    }
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.covers.len(), 2);
    assert_eq!(entry.covers[1].index, 1);
    assert_eq!(entry.covers[0].mime, "image/png");
    assert_eq!(STANDARD.decode(&entry.covers[0].data_base64).unwrap(), png);
    let before = fs::read(&path).unwrap();
    assert!(remove_cover(&path, 5).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(remove_cover(&path, 0).unwrap().covers.len(), 1);
    assert!(remove_cover(&path, 0).unwrap().covers.is_empty());
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["Music"]);
    assert_eq!(wav_payload(&path), payload);
}

fn assert_lightweight_summary(full: &tag_core::AudioEntry, summary: &tag_core::AudioEntry) {
    let mut expected = full.clone();
    for cover in &mut expected.covers {
        cover.data_base64.clear();
    }
    assert_eq!(
        serde_json::to_value(&expected).unwrap(),
        serde_json::to_value(summary).unwrap()
    );
    assert!(summary.covers.iter().all(|c| c.data_base64.is_empty()));
}

#[test]
fn list_scan_summaries_preserve_cover_descriptors_and_custom_multivalue_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("summary.wav");
    wav(&path);
    let png = STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap();
    let image = tmp.path().join("cover.png");
    fs::write(&image, &png).unwrap();
    write_fields(
        &path,
        &fields(&[
            ("ARTIST", &["First", "Second"]),
            ("CUSTOM", &["One", "Two"]),
        ]),
    )
    .unwrap();
    add_cover(&path, &image).unwrap();
    add_cover(&path, &image).unwrap();
    let bytes = fs::read(&path).unwrap();
    let full = read_entry(&path).unwrap();
    let summary = read_entry_summary(&path).unwrap();
    assert_eq!(full.covers.len(), 2);
    assert!(!full.covers[0].data_base64.is_empty());
    assert_lightweight_summary(&full, &summary);
    let scanned = scan_paths(&[path.to_string_lossy().into_owned()], false);
    assert!(scanned.errors.is_empty());
    assert_lightweight_summary(&full, &scanned.entries[0]);
    assert_eq!(
        fs::read(&path).unwrap(),
        bytes,
        "read-only summary mutated source"
    );
}

#[test]
fn mutation_summaries_match_full_reads_without_returning_artwork_payloads() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bulk.wav");
    let payload = wav(&path);
    let image = tmp.path().join("cover.png");
    fs::write(&image, STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap()).unwrap();
    let cover = add_cover_summary(&path, &image).unwrap();
    assert_eq!(cover.covers.len(), 1);
    assert_lightweight_summary(&read_entry(&path).unwrap(), &cover);
    let entry = write_fields_summary(
        &path,
        &fields(&[
            ("TITLE", &["Bulk save"]),
            ("ARTIST", &["A", "B"]),
            ("CUSTOM", &["One", "Two"]),
        ]),
    )
    .unwrap();
    assert_lightweight_summary(&read_entry(&path).unwrap(), &entry);
    assert_eq!(entry.fields["ARTIST"], ["A", "B"]);
    let entry = write_fields_summary(&path, &Fields::new()).unwrap();
    assert_lightweight_summary(&read_entry(&path).unwrap(), &entry);
    let entry = remove_cover_summary(&path, 0).unwrap();
    assert!(entry.covers.is_empty());
    assert_lightweight_summary(&read_entry(&path).unwrap(), &entry);
    add_cover_summary(&path, &image).unwrap();
    let entry = remove_tags_summary(&path).unwrap();
    assert!(entry.covers.is_empty());
    assert!(entry.fields.is_empty());
    assert_lightweight_summary(&read_entry(&path).unwrap(), &entry);
    assert_eq!(wav_payload(&path), payload);
}

#[test]
fn malformed_inputs_and_invalid_patches_leave_source_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let bad = tmp.path().join("bad.wav");
    fs::write(&bad, b"RIFFbroken").unwrap();
    let original = fs::read(&bad).unwrap();
    assert!(write_fields(&bad, &fields(&[("TITLE", &["x"])])).is_err());
    assert!(remove_tags(&bad).is_err());
    assert_eq!(fs::read(&bad).unwrap(), original);
    let path = tmp.path().join("good.wav");
    wav(&path);
    let original = fs::read(&path).unwrap();
    assert!(write_fields(&path, &fields(&[("TITLE", &["invalid\0value"])])).is_err());
    assert!(write_fields(&path, &fields(&[("", &["value"])])).is_err());
    let image = tmp.path().join("bad.png");
    fs::write(&image, b"not an image").unwrap();
    assert!(add_cover(&path, &image).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(tmp.path().read_dir().unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".opentag-")));
}

#[test]
fn scan_recursion_canonical_dedup_and_errors_are_visible() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let one = tmp.path().join("one.wav");
    let two = nested.join("two.WAV");
    wav(&one);
    wav(&two);
    fs::write(tmp.path().join("notes.txt"), "not audio").unwrap();
    let bad = tmp.path().join("bad.mp3");
    fs::write(&bad, "bad mp3").unwrap();
    let paths = vec![
        tmp.path().to_string_lossy().into_owned(),
        one.to_string_lossy().into_owned(),
        tmp.path()
            .join("missing.wav")
            .to_string_lossy()
            .into_owned(),
    ];
    let shallow = scan_paths(&paths, false);
    assert_eq!(shallow.entries.len(), 1);
    assert_eq!(shallow.errors.len(), 2);
    let deep = scan_paths(&paths, true);
    assert_eq!(deep.entries.len(), 2);
    assert_eq!(deep.errors.len(), 2);
    assert!(deep.errors.iter().any(|e| e.path.ends_with("bad.mp3")));
    assert!(deep.errors.iter().any(|e| e.path.ends_with("missing.wav")));
}

#[cfg(unix)]
#[test]
fn successful_atomic_write_preserves_permissions_and_symlink_target() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("song.wav");
    wav(&path);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let alias = tmp.path().join("alias.wav");
    symlink(&path, &alias).unwrap();
    write_fields(&alias, &fields(&[("TITLE", &["Saved"])])).unwrap();
    assert!(fs::symlink_metadata(&alias)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["Saved"]);
}

#[test]
fn empty_patch_does_not_reencode_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("song.wav");
    wav(&path);
    let original = fs::read(&path).unwrap();
    write_fields(&path, &BTreeMap::new()).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn compound_id3_track_and_disc_values_remain_single_native_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("number.wav");
    wav(&path);
    let entry = write_fields(
        &path,
        &fields(&[("TRACK", &["01/12"]), ("DISCNUMBER", &["02/04"])]),
    )
    .unwrap();
    assert_eq!(entry.fields["TRACK"], ["01/12"]);
    assert_eq!(entry.fields["DISCNUMBER"], ["02/04"]);
    assert!(!entry.fields.contains_key("TOTALTRACKS"));
    assert!(!entry.fields.contains_key("TOTALDISCS"));
    let entry = write_fields(&path, &fields(&[("TRACK", &["03"])])).unwrap();
    assert_eq!(entry.fields["TRACK"], ["03"]);
    assert_eq!(entry.fields["DISCNUMBER"], ["02/04"]);
    let entry = write_fields(
        &path,
        &fields(&[("TRACK", &["A1"]), ("TOTALTRACKS", &["Custom text total"])]),
    )
    .unwrap();
    assert_eq!(entry.fields["TRACK"], ["A1"]);
    assert_eq!(entry.fields["TOTALTRACKS"], ["Custom text total"]);
}

fn audio_packet_hashes(path: &Path) -> Vec<String> {
    let result = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_packets",
            "-show_data_hash",
            "sha256",
            "-show_entries",
            "packet=data_hash",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "ffprobe: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let packets: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let hashes: Vec<_> = packets["packets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["data_hash"].as_str().unwrap().to_owned())
        .collect();
    assert!(!hashes.is_empty());
    hashes
}

#[test]
fn ffmpeg_generated_format_matrix_roundtrips_without_changing_audio_packets() {
    use std::process::Command;
    if Command::new("ffmpeg").arg("-version").output().is_err()
        || Command::new("ffprobe").arg("-version").output().is_err()
    {
        eprintln!("Skipping optional encoded-format matrix: ffmpeg and ffprobe are required");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.wav");
    wav(&source);
    let image = tmp.path().join("cover.png");
    fs::write(&image, STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap()).unwrap();
    for (ext, codec) in [
        ("mp3", "libmp3lame"),
        ("flac", "flac"),
        ("m4a", "aac"),
        ("ogg", "vorbis"),
        ("opus", "libopus"),
        ("aiff", "pcm_s16be"),
        ("wv", "wavpack"),
        ("aac", "aac"),
    ] {
        let path = tmp.path().join(format!("sample.{ext}"));
        let encoded = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&source)
            .args([
                "-map_metadata",
                "-1",
                "-ac",
                "2",
                "-c:a",
                codec,
                "-strict",
                "-2",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            encoded.status.success(),
            "{ext}: {}",
            String::from_utf8_lossy(&encoded.stderr)
        );
        let before = audio_packet_hashes(&path);
        let original_vendor = match ext {
            "ogg" => Some(
                lofty::ogg::VorbisFile::read_from(
                    &mut fs::File::open(&path).unwrap(),
                    lofty::config::ParseOptions::new(),
                )
                .unwrap()
                .vorbis_comments()
                .vendor()
                .to_owned(),
            ),
            "opus" => Some(
                lofty::ogg::OpusFile::read_from(
                    &mut fs::File::open(&path).unwrap(),
                    lofty::config::ParseOptions::new(),
                )
                .unwrap()
                .vorbis_comments()
                .vendor()
                .to_owned(),
            ),
            _ => None,
        };
        let cover_entry = add_cover(&path, &image).unwrap_or_else(|e| panic!("cover {ext}: {e}"));
        assert_eq!(cover_entry.covers.len(), 1, "cover {ext}");
        if ext == "wv" {
            let before_duplicate = fs::read(&path).unwrap();
            assert!(
                add_cover(&path, &image).is_err(),
                "APE cannot append another front cover without replacing the existing one"
            );
            assert_eq!(fs::read(&path).unwrap(), before_duplicate);
        }
        let mut updates = fields(&[
            ("TITLE", &["Unicode café 日本"]),
            ("ARTIST", &["First", "Second"]),
            ("TRACK", &["3"]),
            ("TOTALTRACKS", &["12"]),
            ("DISCNUMBER", &["2"]),
            ("TOTALDISCS", &["4"]),
            ("YEAR", &["2024"]),
            ("CUSTOM FIELD", &["first", "second"]),
        ]);
        if matches!(ext, "flac" | "ogg" | "opus" | "m4a") {
            updates.insert("ENCODER".into(), vec!["TagGroove test".into()]);
        }
        let entry = write_fields(&path, &updates).unwrap_or_else(|e| panic!("{ext}: {e}"));
        assert_lightweight_summary(&entry, &read_entry_summary(&path).unwrap());
        assert_eq!(entry.covers.len(), 1, "{ext}: field update removed cover");
        for (key, values) in &updates {
            let expected = if ext == "m4a" && key == "TRACK" {
                vec!["3/12".to_owned()]
            } else if ext == "m4a" && key == "DISCNUMBER" {
                vec!["2/4".to_owned()]
            } else {
                values.clone()
            };
            assert_eq!(entry.fields.get(key), Some(&expected), "{ext}: {key}");
        }
        assert_eq!(
            audio_packet_hashes(&path),
            before,
            "{ext} audio packets changed"
        );
        let entry = write_fields(
            &path,
            &fields(&[("TITLE", &["Edited"]), ("CUSTOM FIELD", &[])]),
        )
        .unwrap_or_else(|e| panic!("{ext}: {e}"));
        assert_eq!(
            entry.fields["ARTIST"],
            ["First", "Second"],
            "{ext}: untouched artists reordered"
        );
        assert!(!entry.fields.contains_key("CUSTOM FIELD"));
        let numbered = write_fields(
            &path,
            &fields(&[("TRACK", &["01/012"]), ("DISCNUMBER", &["02/04"])]),
        )
        .unwrap_or_else(|e| panic!("number {ext}: {e}"));
        assert_eq!(
            numbered.fields["TRACK"][0]
                .split('/')
                .next()
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            1
        );
        assert_eq!(
            numbered.fields["TOTALTRACKS"][0].parse::<u32>().unwrap(),
            12
        );
        assert_eq!(
            numbered.fields["TRACK"],
            [if ext == "m4a" { "1/12" } else { "01/012" }],
            "{ext}: counter representation"
        );
        assert!(remove_cover(&path, 0)
            .unwrap_or_else(|e| panic!("remove cover {ext}: {e}"))
            .covers
            .is_empty());
        remove_tags(&path).unwrap_or_else(|e| panic!("remove {ext}: {e}"));
        let cleared = read_entry(&path).unwrap();
        assert!(
            cleared.fields.is_empty(),
            "{ext}: only actual tag fields belong in Fields"
        );
        if let Some(vendor) = original_vendor.as_ref() {
            let current = if ext == "ogg" {
                lofty::ogg::VorbisFile::read_from(
                    &mut fs::File::open(&path).unwrap(),
                    lofty::config::ParseOptions::new(),
                )
                .unwrap()
                .vorbis_comments()
                .vendor()
                .to_owned()
            } else {
                lofty::ogg::OpusFile::read_from(
                    &mut fs::File::open(&path).unwrap(),
                    lofty::config::ParseOptions::new(),
                )
                .unwrap()
                .vorbis_comments()
                .vendor()
                .to_owned()
            };
            assert_eq!(&current, vendor, "required Ogg vendor changed");
        }
        assert_eq!(
            audio_packet_hashes(&path),
            before,
            "{ext} audio packets changed after removal"
        );
    }
}

// Observe one physical APE item from its bounded tail/footer, independently
// of the production reader and Lofty's decoded tag view. ID3v1 may follow it.
fn raw_ape_item(bytes: &[u8], wanted: &[u8]) -> (Vec<u8>, u32, Vec<u8>) {
    let end = if bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG" {
        bytes.len() - 128
    } else {
        bytes.len()
    };
    assert!(end >= 32);
    let footer = end - 32;
    assert_eq!(&bytes[footer..footer + 8], b"APETAGEX");
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(word(footer + 8), 2000);
    let size = word(footer + 12) as usize;
    assert!(size >= 32 && size <= end);
    let mut at = end - size;
    let count = word(footer + 16) as usize;
    assert!(count <= (footer - at) / 9);
    let mut found = Vec::new();
    for _ in 0..count {
        assert!(footer - at >= 9);
        let length = word(at) as usize;
        let flags = word(at + 4);
        at += 8;
        let name_length = bytes[at..footer].iter().position(|b| *b == 0).unwrap();
        let name = bytes[at..at + name_length].to_vec();
        at += name_length + 1;
        assert!(length <= footer - at);
        let data = bytes[at..at + length].to_vec();
        at += length;
        if name == wanted {
            found.push((name, flags, data));
        }
    }
    assert_eq!(at, footer);
    assert_eq!(found.len(), 1);
    found.pop().unwrap()
}

#[test]
fn native_vorbis_custom_text_and_ape_binary_survive_unrelated_changes() {
    use std::process::Command;
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("Skipping optional native-tag preservation test: ffmpeg required");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source.wav");
    wav(&source);
    let flac = tmp.path().join("custom.flac");
    let mp3 = tmp.path().join("custom.mp3");
    for (path, codec) in [(&flac, "flac"), (&mp3, "libmp3lame")] {
        let output = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&source)
            .args(["-c:a", codec])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
    }
    let mut comments = lofty::ogg::tag::VorbisComments::new();
    comments.set_vendor("Original vendor".into());
    comments.push("Unmanaged_Case_Field".into(), "first".into());
    comments.push("Unmanaged_Case_Field".into(), "second".into());
    comments
        .save_to_path(&flac, WriteOptions::default())
        .unwrap();
    let original_vendor = lofty::flac::FlacFile::read_from(
        &mut fs::File::open(&flac).unwrap(),
        lofty::config::ParseOptions::new(),
    )
    .unwrap()
    .vorbis_comments()
    .unwrap()
    .vendor()
    .to_owned();
    assert_eq!(
        read_entry(&flac).unwrap().fields["Unmanaged_Case_Field"],
        ["first", "second"]
    );
    write_fields(&flac, &fields(&[("TITLE", &["Changed"])])).unwrap();
    let concrete = lofty::flac::FlacFile::read_from(
        &mut fs::File::open(&flac).unwrap(),
        lofty::config::ParseOptions::new(),
    )
    .unwrap();
    let tag = concrete.vorbis_comments().unwrap();
    assert_eq!(tag.vendor(), original_vendor);
    assert_eq!(
        tag.get_all("Unmanaged_Case_Field").collect::<Vec<_>>(),
        ["first", "second"]
    );
    assert!(tag.items().any(|(k, _)| k == "Unmanaged_Case_Field"));
    let mut ape = lofty::ape::ApeTag::new();
    ape.insert(
        lofty::ape::ApeItem::new(
            "Unmanaged Binary".into(),
            ItemValue::Binary(vec![0, 1, 2, 255]),
        )
        .unwrap(),
    );
    ape.insert(
        lofty::ape::ApeItem::new(
            "Unmanaged_Case_Field".into(),
            ItemValue::Text("first\0second".into()),
        )
        .unwrap(),
    );
    ape.save_to_path(&mp3, WriteOptions::default()).unwrap();
    // This case edits the displayed native APE tag. Unwritten APE retention
    // under the default MPEG read/write policy has separate policy coverage.
    let mut options = tag_core::TagOptions::default();
    options.mpeg.read_ape = true;
    options.mpeg.write_ape = true;
    assert_eq!(
        tag_core::read_entry_with_options(&mp3, &[], &options)
            .unwrap()
            .fields["Unmanaged_Case_Field"],
        ["first", "second"]
    );
    let original_binary = raw_ape_item(&fs::read(&mp3).unwrap(), b"Unmanaged Binary");
    assert_eq!(original_binary.0, b"Unmanaged Binary");
    assert_eq!(
        (original_binary.1 >> 1) & 3,
        1,
        "fixture must contain a physical binary item"
    );
    assert_eq!(original_binary.2, [0, 1, 2, 255]);
    tag_core::write_fields_with_options(
        &mp3,
        &fields(&[("TITLE", &["Changed"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    let concrete = lofty::mpeg::MpegFile::read_from(
        &mut fs::File::open(&mp3).unwrap(),
        lofty::config::ParseOptions::new(),
    )
    .unwrap();
    let tag = concrete.ape().unwrap();
    assert_eq!(
        tag.get("Unmanaged Binary").unwrap().value(),
        &ItemValue::Binary(vec![0, 1, 2, 255])
    );
    assert_eq!(
        tag.get("Unmanaged_Case_Field").unwrap().value(),
        &ItemValue::Text("first\0second".into())
    );
    assert_eq!(
        raw_ape_item(&fs::read(&mp3).unwrap(), b"Unmanaged Binary"),
        original_binary,
        "unrelated native APE edit changed the physical binary name, flags or data"
    );
    assert_eq!(
        tag_core::read_entry_with_options(&mp3, &[], &options)
            .unwrap()
            .fields["Unmanaged_Case_Field"],
        ["first", "second"]
    );
}
