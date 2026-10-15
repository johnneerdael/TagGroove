//! File → Remove tag following Options → Tags, user field mappings carried in
//! `TagOptions`, and WAV RIFF INFO creation.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    id3::{v1::Id3v1Tag, v2::Id3v2Tag},
    iff::wav::WavFile,
    prelude::TagExt,
    tag::ItemValue,
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    add_cover_with_options, read_entry_summary_with_options, read_entry_with_options, remove_tags,
    remove_tags_with_options, write_fields, write_fields_with_options, Fields, TagOptions,
    UserFieldMapping,
};

fn values(pairs: &[(&str, &[&str])]) -> Fields {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

fn rule(tag_type: &str, source: &str, target: &str) -> UserFieldMapping {
    UserFieldMapping {
        tag_type: tag_type.into(),
        source: source.into(),
        target: target.into(),
    }
}

/// Twenty MPEG-1 Layer III frames without frame sync or tag markers inside.
fn mp3(path: &Path) {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xFF, 0xFB, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
}

/// An MP3 with ID3v2, APEv2 and ID3v1 tags.
fn mp3_with_all_kinds(path: &Path) {
    mp3(path);
    let mut v2 = Id3v2Tag::new();
    lofty::tag::Accessor::set_title(&mut v2, "Two".into());
    v2.save_to_path(path, WriteOptions::default()).unwrap();
    let mut ape = ApeTag::new();
    ape.insert(ApeItem::new("Title".into(), ItemValue::Text("Ape".into())).unwrap());
    ape.save_to_path(path, WriteOptions::default()).unwrap();
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("One".into());
    v1.save_to_path(path, WriteOptions::default()).unwrap();
}

fn tag_kinds(path: &Path) -> String {
    read_entry_summary_with_options(path, &[], &TagOptions::default())
        .unwrap()
        .technical
        .get("_TAG")
        .map(|v| v.join(" "))
        .unwrap_or_default()
}

fn wav(path: &Path) {
    let mut b = Vec::new();
    b.extend(b"RIFF");
    b.extend(16036u32.to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(8000u32.to_le_bytes());
    b.extend(16000u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(16000u32.to_le_bytes());
    b.extend(vec![0u8; 16000]);
    fs::write(path, b).unwrap();
}

fn riff(path: &Path) -> Vec<(String, String)> {
    WavFile::read_from(&mut fs::File::open(path).unwrap(), ParseOptions::new())
        .unwrap()
        .riff_info()
        .map(|list| {
            list.into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

fn flac(path: &Path) -> bool {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("ffmpeg absent: skipping the FLAC fixture");
        return false;
    }
    let source = path.with_extension("wav");
    wav(&source);
    let r = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&source)
        .args(["-c:a", "flac"])
        .arg(path)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    true
}

#[test]
fn remove_tag_strips_only_the_kinds_selected_for_mpeg() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("all.mp3");
    mp3_with_all_kinds(&path);
    assert_eq!(tag_kinds(&path), "ID3v2.4 ID3v1 APEv2");

    let mut options = TagOptions::default();
    options.mpeg.remove_id3v2 = false;
    options.mpeg.remove_ape = false;
    let entry = remove_tags_with_options(&path, false, &options).unwrap();
    assert_eq!(entry.technical["_TAG"], ["ID3v2.4 APEv2"]);

    // Nothing selected is present any more: the file is left alone.
    let before = fs::read(&path).unwrap();
    remove_tags_with_options(&path, false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);

    // No kind selected at all is reported instead of silently doing nothing.
    options.mpeg.remove_id3v1 = false;
    let error = remove_tags_with_options(&path, false, &options).unwrap_err();
    assert!(error.contains("Options → Tags → Mpeg"), "{error}");

    // The defaults remove every MPEG tag kind.
    let entry = remove_tags_with_options(&path, false, &TagOptions::default()).unwrap();
    assert!(
        !entry.technical.contains_key("_TAG"),
        "{:?}",
        entry.technical
    );
    assert!(entry.fields.is_empty());
}

#[test]
fn remove_tag_strips_every_kind_of_formats_without_remove_options() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("song.wav");
    wav(&path);
    write_fields(&path, &values(&[("TITLE", &["Both"]), ("MOOD", &["Calm"])])).unwrap();
    assert_eq!(tag_kinds(&path), "ID3v2.4 RIFF INFO");
    let mut options = TagOptions::default();
    options.mpeg.remove_id3v2 = false;
    let entry = remove_tags_with_options(&path, false, &options).unwrap();
    assert!(entry.fields.is_empty(), "{:?}", entry.fields);
    assert!(riff(&path).is_empty());
}

#[test]
fn new_wav_files_get_riff_info_for_fields_with_a_riff_id() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("new.wav");
    wav(&path);

    // Only fields without a RIFF id: no empty INFO chunk is created.
    write_fields(&path, &values(&[("MOOD", &["Calm"])])).unwrap();
    assert!(riff(&path).is_empty());

    // A RIFF field creates INFO from every displayed value with a RIFF id;
    // the rest stays in ID3v2.
    let entry = write_fields(
        &path,
        &values(&[("TITLE", &["Song"]), ("ARTIST", &["A", "B"])]),
    )
    .unwrap();
    assert_eq!(entry.fields["MOOD"], ["Calm"]);
    assert_eq!(entry.fields["ARTIST"], ["A", "B"]);
    let mut info = riff(&path);
    info.sort();
    assert_eq!(
        info,
        [
            ("IART".to_string(), "A; B".to_string()),
            ("INAM".to_string(), "Song".to_string())
        ]
    );

    // Existing INFO is updated and cleared field by field.
    write_fields(&path, &values(&[("TITLE", &[]), ("COMMENT", &["Note"])])).unwrap();
    let mut info = riff(&path);
    info.sort();
    assert_eq!(
        info,
        [
            ("IART".to_string(), "A; B".to_string()),
            ("ICMT".to_string(), "Note".to_string())
        ]
    );
    remove_tags(&path).unwrap();
}

#[test]
fn riff_ids_without_a_mapping_round_trip_and_other_names_stay_in_id3v2() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("info.wav");
    wav(&path);
    let mut file =
        WavFile::read_from(&mut fs::File::open(&path).unwrap(), ParseOptions::new()).unwrap();
    let mut list = lofty::iff::wav::RiffInfoList::default();
    list.insert("IENG".into(), "Engineer".into());
    list.insert("INAM".into(), "Info title".into());
    file.set_riff_info(list);
    file.save_to_path(&path, WriteOptions::default()).unwrap();
    let entry = write_fields(
        &path,
        &values(&[("IENG", &["Other engineer"]), ("SPOT", &["x"])]),
    )
    .unwrap();
    assert_eq!(entry.fields["IENG"], ["Other engineer"]);
    assert_eq!(entry.fields["SPOT"], ["x"]);
    assert_eq!(entry.fields["TITLE"], ["Info title"]);
    let info = riff(&path);
    assert!(
        info.contains(&("IENG".into(), "Other engineer".into())),
        "{info:?}"
    );
    assert!(!info.iter().any(|(k, _)| k == "SPOT"), "{info:?}");
}

#[test]
fn id3v2_mappings_apply_through_the_mpeg_tag_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("mapped.mp3");
    mp3(&path);
    let options = TagOptions {
        mappings: vec![rule("ID3v2", "COMPOSER", "WRITER")],
        ..Default::default()
    };
    let saved = write_fields_with_options(
        &path,
        &values(&[("WRITER", &["Bach"]), ("TITLE", &["Fugue"])]),
        false,
        &options.mappings,
        &options,
    )
    .unwrap();
    assert_eq!(saved.fields["WRITER"], ["Bach"]);
    assert!(!saved.fields.contains_key("COMPOSER"));
    let raw = read_entry_with_options(&path, &[], &TagOptions::default()).unwrap();
    assert_eq!(raw.fields["COMPOSER"], ["Bach"]);
    assert!(!raw.fields.contains_key("WRITER"));
}

#[test]
fn operations_without_a_rules_parameter_return_mapped_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cover.flac");
    if !flac(&path) {
        return;
    }
    write_fields(&path, &values(&[("DATE", &["2024"])])).unwrap();
    let image = tmp.path().join("cover.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([9, 8, 7]))
        .save(&image)
        .unwrap();
    let options = TagOptions {
        mappings: vec![rule("VorbisComment", "DATE", "YEAR")],
        ..Default::default()
    };
    let entry = add_cover_with_options(&path, &image, false, &options).unwrap();
    assert_eq!(entry.covers.len(), 1);
    assert_eq!(entry.fields["YEAR"], ["2024"]);
    assert!(!entry.fields.contains_key("DATE"));
    let scanned = tag_core::scan_paths_with_options(
        &[path.to_string_lossy().into_owned()],
        false,
        &options,
        None,
    );
    assert_eq!(scanned.entries[0].fields["YEAR"], ["2024"]);
    let removed = remove_tags_with_options(&path, false, &options).unwrap();
    assert!(removed.fields.is_empty());
}

#[test]
fn tag_options_reject_invalid_mappings() {
    let mut options = TagOptions::default();
    assert!(options.validate().is_ok());
    options.mappings = vec![rule("RIFF", "TITLE", "NAME")];
    assert!(options.validate().unwrap_err().contains("tag type"));
    options.mappings = vec![
        rule("APEv2", "DATE", "YEAR"),
        rule("apev2", "date", "OTHER"),
    ];
    assert!(options.validate().unwrap_err().contains("multiple targets"));
    options.mappings = vec![rule("MP4", " ", "YEAR")];
    assert!(options.validate().is_err());
    let json = serde_json::to_value(TagOptions::default()).unwrap();
    assert_eq!(json["mappings"], serde_json::json!([]));
    let parsed: TagOptions = serde_json::from_str(
        r#"{"mappings":[{"tag_type":"WMA","source":"WM/Mood","target":"MOOD"}]}"#,
    )
    .unwrap();
    assert_eq!(parsed.mappings, [rule("WMA", "WM/Mood", "MOOD")]);
}
