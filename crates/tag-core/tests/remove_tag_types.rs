//! Utils → Remove Tag Types against independently built MPEG files.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    id3::{v1::Id3v1Tag, v2::Id3v2Tag},
    prelude::TagExt,
    tag::ItemValue,
};
use std::{fs, path::Path};
use tag_core::{read_entry_with_options, remove_tag_kinds_with_options, RemovableTag, TagOptions};

fn mp3(path: &Path) -> Vec<u8> {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xFF, 0xFB, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
    audio
}

fn seed_all_kinds(path: &Path) -> Vec<u8> {
    let audio = mp3(path);
    let mut v2 = Id3v2Tag::new();
    lofty::tag::Accessor::set_title(&mut v2, "Two".into());
    v2.save_to_path(path, WriteOptions::default()).unwrap();
    let mut ape = ApeTag::new();
    ape.insert(ApeItem::new("Title".into(), ItemValue::Text("Ape".into())).unwrap());
    ape.insert(ApeItem::new("MP3GAIN_UNDO".into(), ItemValue::Text("+001,+001,N".into())).unwrap());
    ape.save_to_path(path, WriteOptions::default()).unwrap();
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("One".into());
    v1.save_to_path(path, WriteOptions::default()).unwrap();
    audio
}

fn tags(path: &Path) -> String {
    read_entry_with_options(path, &[], &TagOptions::default())
        .unwrap()
        .technical
        .get("_TAG")
        .map(|v| v.join(" "))
        .unwrap_or_default()
}

#[test]
fn each_kind_is_removed_on_its_own_and_audio_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    for (kinds, expected) in [
        (vec![RemovableTag::Ape], "ID3v2.4 ID3v1"),
        (vec![RemovableTag::Id3v1], "ID3v2.4 APE"),
        (vec![RemovableTag::Id3v2], "APE ID3v1"),
        (
            vec![RemovableTag::Id3v1, RemovableTag::Id3v2, RemovableTag::Ape],
            "",
        ),
    ] {
        let path = dir.path().join("song.mp3");
        let audio = seed_all_kinds(&path);
        assert!(tags(&path).contains("APE"));
        let entry =
            remove_tag_kinds_with_options(&path, &kinds, false, &TagOptions::default()).unwrap();
        let mut remaining: Vec<_> = tags(&path)
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let mut wanted: Vec<_> = expected
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        remaining.sort();
        wanted.sort();
        assert_eq!(remaining, wanted, "{kinds:?}");
        assert_eq!(
            entry
                .technical
                .get("_TAG")
                .map(|v| v.join(" "))
                .unwrap_or_default(),
            tags(&path)
        );
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.windows(audio.len()).any(|w| w == audio.as_slice()));
        if kinds.contains(&RemovableTag::Ape) {
            assert!(!bytes.windows(12).any(|w| w == b"MP3GAIN_UNDO"));
        }
    }
}

#[test]
fn absent_kinds_leave_the_file_untouched_and_other_formats_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.mp3");
    mp3(&path);
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("One".into());
    v1.save_to_path(&path, WriteOptions::default()).unwrap();
    let before = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    remove_tag_kinds_with_options(&path, &[RemovableTag::Ape], false, &TagOptions::default())
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    assert!(remove_tag_kinds_with_options(&path, &[], false, &TagOptions::default()).is_err());

    let flac = dir.path().join("x.flac");
    fs::write(&flac, b"fLaC").unwrap();
    assert!(remove_tag_kinds_with_options(
        &flac,
        &[RemovableTag::Id3v2],
        false,
        &TagOptions::default()
    )
    .is_err());
}
