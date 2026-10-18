//! Exercise the production command, clipboard resolver and document transaction
//! against disposable WAV/PNG files. Only provider transport is substituted.
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use documents::command_fixture::Session;
use std::{fs, io::Cursor};

fn wav(path: &Path) {
    let samples: Vec<u8> = (0..8820).map(|index| (index % 251) as u8).collect();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + samples.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(44100u32.to_le_bytes());
    bytes.extend(88200u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32).to_le_bytes());
    bytes.extend(samples);
    fs::write(path, bytes).unwrap();
}
fn audio(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    let mut cursor = 12;
    while cursor + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        if &bytes[cursor..cursor + 4] == b"data" {
            return bytes[cursor + 8..cursor + 8 + len].to_vec();
        }
        cursor += 8 + len + len % 2;
    }
    panic!("missing PCM audio");
}
fn png(color: [u8; 3]) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb(color)))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}
fn cover(path: &Path, data: Vec<u8>, kind: &str, description: &str) {
    tag_core::add_cover_bytes_with_options(
        path,
        data,
        description,
        Some(kind),
        &tag_core::TagOptions::default(),
    )
    .unwrap();
    tag_core::set_cover_properties_with_options(
        path,
        None,
        Some(description),
        Some(kind),
        &tag_core::TagOptions::default(),
    )
    .unwrap();
}
fn fields(title: &str) -> Fields {
    Fields::from([
        ("TITLE".into(), vec![title.into()]),
        ("COMMENT".into(), vec![]),
    ])
}
fn pending(path: &Path) -> Vec<list_behaviour::PendingCover> {
    let copied = tauri::async_runtime::block_on(list_behaviour::copy_tag_covers(
        vec![path.to_string_lossy().into_owned()],
        vec![],
    ))
    .unwrap();
    let wire = serde_json::to_value(copied).unwrap();
    assert!(wire["errors"].as_array().unwrap().is_empty());
    serde_json::from_value(wire["covers"][0].clone()).unwrap()
}
fn stale_cover() -> Vec<list_behaviour::PendingCover> {
    serde_json::from_value(serde_json::json!([
        {"digest":"unavailable", "picture_type":"not-a-picture-type", "description":"stale draft"}
    ]))
    .unwrap()
}

// Inspect native tag boundaries/items independently of the tag-core reader.
fn mpeg_native(path: &Path) -> (Vec<u8>, std::collections::BTreeMap<String, Vec<u8>>) {
    let bytes = fs::read(path).unwrap();
    assert!(
        !bytes.starts_with(b"ID3"),
        "APE-only copy must not create ID3v2"
    );
    assert!(
        bytes.len() < 128 || &bytes[bytes.len() - 128..bytes.len() - 125] != b"TAG",
        "APE-only copy must not create ID3v1"
    );
    let footer = &bytes[bytes.len() - 32..];
    assert_eq!(&footer[..8], b"APETAGEX", "APE-only copy must retain APEv2");
    assert_eq!(u32::from_le_bytes(footer[8..12].try_into().unwrap()), 2000);
    let size = u32::from_le_bytes(footer[12..16].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(footer[16..20].try_into().unwrap());
    let flags = u32::from_le_bytes(footer[20..24].try_into().unwrap());
    let mut cursor = bytes.len() - size;
    let audio_end = cursor - if flags & 0x8000_0000 != 0 { 32 } else { 0 };
    let mut items = std::collections::BTreeMap::new();
    for _ in 0..count {
        let len = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 8;
        let end = cursor + bytes[cursor..].iter().position(|byte| *byte == 0).unwrap();
        let key = std::str::from_utf8(&bytes[cursor..end])
            .unwrap()
            .to_ascii_uppercase();
        cursor = end + 1;
        items.insert(key, bytes[cursor..cursor + len].to_vec());
        cursor += len;
    }
    assert_eq!(cursor, bytes.len() - 32);
    (bytes[..audio_end].to_vec(), items)
}

fn edited_copy_policy_case(with_fields: bool) {
    let _clipboard = list_behaviour::TEST_CLIPBOARD.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.mp3");
    fs::write(
        &source,
        include_bytes!("../../../crates/tag-core/tests/fixtures/save-corpus/tone.mp3"),
    )
    .unwrap();
    let mut options = tag_core::TagOptions::default();
    options.mpeg.write_id3v1 = false;
    options.mpeg.write_id3v2 = false;
    options.mpeg.write_ape = true;
    options.mpeg.read_id3v1 = false;
    options.mpeg.read_id3v2 = false;
    options.mappings = vec![tag_core::UserFieldMapping {
        tag_type: "APEv2".into(),
        source: "TG_NATIVE_NOTE".into(),
        target: "MIGRANT_NOTE".into(),
    }];
    tag_core::write_fields_with_options(
        &source,
        &Fields::from([
            ("TITLE".into(), vec!["Original".into()]),
            ("COMMENT".into(), vec!["Remove me".into()]),
            ("ARTIST".into(), vec!["Retained artist".into()]),
            ("MIGRANT_NOTE".into(), vec!["Original mapped note".into()]),
        ]),
        false,
        &options.mappings,
        &options,
    )
    .unwrap();
    tag_core::add_cover_bytes_with_options(
        &source,
        png([255, 0, 0]),
        "Original front",
        Some("CoverFront"),
        &options,
    )
    .unwrap();
    let original = fs::read(&source).unwrap();
    let (sound, _) = mpeg_native(&source);
    let donor = dir.path().join("clipboard.wav");
    wav(&donor);
    let image = png([0, 0, 255]);
    cover(&donor, image.clone(), "CoverFront", "Pending front");
    let covers = pending(&donor);
    let session = Session::import(&dir.path().join("private"), &source, "Source.mp3", false);
    let retained = fs::read(&session.path).unwrap();
    let target = dir.path().join("edited.mp3");
    session.copy_to(&target);
    let mut updates = if with_fields {
        fields("Pending title")
    } else {
        Fields::new()
    };
    if with_fields {
        updates.insert("MIGRANT_NOTE".into(), vec!["Pending mapped note".into()]);
    }
    let path = session.path.clone();
    let active = options.clone();
    let copied = tauri::async_runtime::block_on(io(move || {
        settings::with_test_tag_options(active, || {
            documents::save_copy(&path, &updates, Some(&covers), "edited")
        })
    }))
    .unwrap();
    assert!(copied.errors.is_empty());
    assert_eq!(copied.entries.len(), 1);
    let (copied_sound, native) = mpeg_native(&target);
    assert_eq!(copied_sound, sound);
    assert_eq!(
        native["TITLE"],
        if with_fields {
            b"Pending title".as_slice()
        } else {
            b"Original".as_slice()
        }
    );
    assert_eq!(native["ARTIST"], b"Retained artist");
    assert_eq!(
        native["TG_NATIVE_NOTE"],
        if with_fields {
            b"Pending mapped note".as_slice()
        } else {
            b"Original mapped note".as_slice()
        }
    );
    assert!(
        !native.contains_key("MIGRANT_NOTE"),
        "UI alias must not leak into native APE names"
    );
    assert_eq!(native.contains_key("COMMENT"), !with_fields);
    let artwork = &native["COVER ART (FRONT)"];
    let image_start = artwork.iter().position(|byte| *byte == 0).unwrap() + 1;
    assert_eq!(&artwork[image_start..], image);
    let read = tag_core::read_entry_with_options(&target, &options.mappings, &options).unwrap();
    assert_eq!(
        read.fields["MIGRANT_NOTE"],
        [if with_fields {
            "Pending mapped note"
        } else {
            "Original mapped note"
        }]
    );
    // Provider result projection still uses its existing default reader, so
    // mapped aliases are verified above, not promised by this return value.
    assert!(copied.entries[0].fields.contains_key("TG_NATIVE_NOTE"));
    assert!(!copied.entries[0].fields.contains_key("MIGRANT_NOTE"));
    assert_eq!(copied.entries[0].covers.len(), 1);
    let backup = dir.path().join("original.mp3");
    session.copy_to(&backup);
    let path = session.path.clone();
    tauri::async_runtime::block_on(io(move || {
        settings::with_test_tag_options(options, || {
            documents::save_copy(
                &path,
                &fields("Ignored draft"),
                Some(&stale_cover()),
                "original",
            )
        })
    }))
    .unwrap();
    assert_eq!(fs::read(&backup).unwrap(), original);
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&session.path).unwrap(), retained);
}

#[test]
fn edited_copy_artwork_uses_active_mpeg_policy() {
    edited_copy_policy_case(false);
}
#[test]
fn edited_copy_artwork_and_fields_share_active_mpeg_policy_and_mappings() {
    edited_copy_policy_case(true);
}

#[test]
fn edited_copy_command_writes_pending_fields_and_ordered_artwork_without_changing_read_only_source()
{
    let _clipboard = list_behaviour::TEST_CLIPBOARD.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.wav");
    wav(&source);
    tag_core::write_fields(
        &source,
        &Fields::from([
            ("TITLE".into(), vec!["Original".into()]),
            ("COMMENT".into(), vec!["Remove me".into()]),
        ]),
    )
    .unwrap();
    cover(&source, png([255, 0, 0]), "CoverFront", "Original front");
    let original = fs::read(&source).unwrap();
    let donor = dir.path().join("clipboard.wav");
    wav(&donor);
    let front = png([0, 0, 255]);
    let back = png([0, 255, 0]);
    cover(&donor, front.clone(), "CoverFront", "New front");
    cover(&donor, back.clone(), "CoverBack", "New back");
    let covers = pending(&donor);
    let session = Session::import(
        &dir.path().join("private"),
        &source,
        "Mémoire 猫.wav",
        false,
    );
    let retained = fs::read(&session.path).unwrap();
    let target = dir.path().join("edited.wav");
    session.copy_to(&target);
    let copied = tauri::async_runtime::block_on(document_save_copy(
        session.path.clone(),
        fields("Edited title"),
        Some(covers),
        "edited".into(),
    ))
    .unwrap();
    assert!(copied.errors.is_empty());
    assert_eq!(copied.entries.len(), 1);
    assert_eq!(copied.entries[0].fields["TITLE"], ["Edited title"]);
    assert!(!copied.entries[0].fields.contains_key("COMMENT"));
    let actual =
        tauri::async_runtime::block_on(get_covers(copied.entries[0].path.clone())).unwrap();
    assert_eq!(
        actual
            .iter()
            .map(|cover| (&*cover.picture_type, &*cover.description))
            .collect::<Vec<_>>(),
        [("CoverFront", "New front"), ("CoverBack", "New back")]
    );
    assert_eq!(
        actual
            .iter()
            .map(|cover| STANDARD.decode(&cover.data_base64).unwrap())
            .collect::<Vec<_>>(),
        [front, back]
    );
    assert_eq!(audio(&target), audio(&source));
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&session.path).unwrap(), retained);
    assert_eq!(
        tag_core::read_entry(&target).unwrap().fields["TITLE"],
        ["Edited title"]
    );
    assert_eq!(
        tauri::async_runtime::block_on(document_states()).unwrap()[&session.path].state,
        "ready"
    );
}

#[test]
fn copy_command_distinguishes_unchanged_artwork_remove_all_and_original_backup() {
    let _clipboard = list_behaviour::TEST_CLIPBOARD.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.wav");
    wav(&source);
    cover(&source, png([255, 0, 0]), "CoverBack", "Original back");
    let original = fs::read(&source).unwrap();
    let session = Session::import(&dir.path().join("private"), &source, "Source.wav", false);
    for (name, version, covers, count) in [
        ("unchanged", "edited", None, 1),
        ("removed", "edited", Some(vec![]), 0),
        ("original-removal", "original", Some(vec![]), 1),
        ("original-stale", "original", Some(stale_cover()), 1),
    ] {
        let target = dir.path().join(format!("{name}.wav"));
        session.copy_to(&target);
        let copied = tauri::async_runtime::block_on(document_save_copy(
            session.path.clone(),
            fields("Pending"),
            covers,
            version.into(),
        ))
        .unwrap();
        assert!(copied.errors.is_empty());
        assert_eq!(
            tag_core::read_entry(&target).unwrap().covers.len(),
            count,
            "{name}"
        );
        assert_eq!(audio(&target), audio(&source));
        if version == "original" {
            assert_eq!(fs::read(&target).unwrap(), original, "{name}");
        } else {
            assert_eq!(
                tag_core::read_entry(&target).unwrap().fields["TITLE"],
                ["Pending"]
            );
        }
    }
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&session.path).unwrap(), original);
}

#[test]
fn missing_pending_artwork_is_rejected_before_creating_destination_and_cancellation_retains_source()
{
    let _clipboard = list_behaviour::TEST_CLIPBOARD.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.wav");
    wav(&source);
    let original = fs::read(&source).unwrap();
    let session = Session::import(&dir.path().join("private"), &source, "Source.wav", false);
    let private = dunce::canonicalize(dir.path().join("private")).unwrap();
    let source_paths = || {
        tauri::async_runtime::block_on(document_states())
            .unwrap()
            .into_keys()
            .filter(|path| Path::new(path).starts_with(&private))
            .collect::<Vec<_>>()
    };
    let before = source_paths();
    // No destination selected models a cancelled CREATE_DOCUMENT picker.
    let cancelled = tauri::async_runtime::block_on(document_save_copy(
        session.path.clone(),
        fields("Pending"),
        Some(vec![]),
        "edited".into(),
    ))
    .unwrap();
    assert!(cancelled.entries.is_empty() && cancelled.errors.is_empty());
    let target = dir.path().join("not-created.wav");
    session.copy_to(&target);
    let error = tauri::async_runtime::block_on(document_save_copy(
        session.path.clone(),
        fields("Pending"),
        Some(stale_cover()),
        "edited".into(),
    ))
    .unwrap_err();
    assert!(error.contains("no longer available"), "{error}");
    assert!(!target.exists());
    assert_eq!(source_paths(), before);
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&session.path).unwrap(), original);
}

#[test]
fn failed_copy_publication_keeps_edited_artwork_and_allows_retry_without_source_publication() {
    let _clipboard = list_behaviour::TEST_CLIPBOARD.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.wav");
    wav(&source);
    let original = fs::read(&source).unwrap();
    let donor = dir.path().join("clipboard.wav");
    wav(&donor);
    let image = png([0, 255, 0]);
    cover(&donor, image.clone(), "CoverBack", "Pending back");
    let covers = pending(&donor);
    let session = Session::import(&dir.path().join("private"), &source, "Source.wav", true);
    session.fail_publish();
    let failed_target = dir.path().join("failed.wav");
    session.copy_to(&failed_target);
    let error = tauri::async_runtime::block_on(document_save_copy(
        session.path.clone(),
        fields("Pending"),
        Some(covers.clone()),
        "edited".into(),
    ))
    .unwrap_err();
    assert!(error.contains("publication failed"));
    let recovered = tauri::async_runtime::block_on(restore_document_recovery()).unwrap();
    let private = dunce::canonicalize(dir.path().join("private")).unwrap();
    let edited = recovered
        .entries
        .iter()
        .find(|entry| Path::new(&entry.path).starts_with(&private) && entry.path != session.path)
        .unwrap();
    assert_eq!(edited.fields["TITLE"], ["Pending"]);
    let retained_covers = tauri::async_runtime::block_on(get_covers(edited.path.clone())).unwrap();
    assert_eq!(
        STANDARD.decode(&retained_covers[0].data_base64).unwrap(),
        image
    );
    assert_eq!(retained_covers[0].description, "Pending back");
    session.resume_publish();
    let retry_target = dir.path().join("retry.wav");
    session.copy_to(&retry_target);
    let copied = tauri::async_runtime::block_on(document_save_copy(
        session.path.clone(),
        fields("Pending"),
        Some(covers),
        "edited".into(),
    ))
    .unwrap();
    assert_eq!(copied.entries[0].fields["TITLE"], ["Pending"]);
    assert_eq!(audio(&retry_target), audio(&source));
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&session.path).unwrap(), original);
    assert_eq!(
        tauri::async_runtime::block_on(document_states()).unwrap()[&edited.path].state,
        "recovery_required"
    );
}

#[test]
fn recovery_copy_overlays_pending_artwork_but_original_uses_verified_backup_only() {
    let _clipboard = list_behaviour::TEST_CLIPBOARD.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.wav");
    wav(&source);
    cover(&source, png([255, 0, 0]), "CoverFront", "Original front");
    let original = fs::read(&source).unwrap();
    let donor = dir.path().join("clipboard.wav");
    wav(&donor);
    let image = png([0, 255, 0]);
    cover(&donor, image.clone(), "CoverBack", "Pending back");
    let covers = pending(&donor);
    let session = Session::import(&dir.path().join("private"), &source, "Source.wav", true);
    session.fail_publish();
    let removed = tauri::async_runtime::block_on(remove_tags(vec![session.path.clone()])).unwrap();
    assert!(removed.entries.is_empty() && !removed.errors.is_empty());
    let retained = fs::read(&session.path).unwrap();
    assert!(tag_core::read_entry(Path::new(&session.path))
        .unwrap()
        .covers
        .is_empty());
    session.resume_publish();
    for version in ["edited", "original"] {
        let target = dir.path().join(format!("{version}.wav"));
        session.copy_to(&target);
        let copied = tauri::async_runtime::block_on(document_save_copy(
            session.path.clone(),
            fields("Pending"),
            Some(covers.clone()),
            version.into(),
        ))
        .unwrap();
        assert_eq!(copied.entries.len(), 1);
        if version == "original" {
            assert_eq!(fs::read(&target).unwrap(), original);
        } else {
            let written = tag_core::read_entry(&target).unwrap();
            assert_eq!(written.fields["TITLE"], ["Pending"]);
            assert_eq!(written.covers[0].picture_type, "CoverBack");
            assert_eq!(
                STANDARD.decode(&written.covers[0].data_base64).unwrap(),
                image
            );
            assert_eq!(audio(&target), audio(&source));
        }
        assert_eq!(fs::read(&session.path).unwrap(), retained);
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(
            tauri::async_runtime::block_on(document_states()).unwrap()[&session.path].state,
            "recovery_required"
        );
    }
}
