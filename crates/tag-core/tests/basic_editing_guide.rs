//! Written beginner examples against the actual metadata core and file writers.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    id3::{v1::Id3v1Tag, v2::Id3v2Tag},
    prelude::TagExt,
    tag::{Accessor, ItemValue},
};
use std::{fs, path::Path};
use tag_core::{evaluate, remove_tag_kinds_with_options, Fields, RemovableTag, TagOptions};
const COPY: &str = include_str!("../../../docs/user-guide/guides/copy-paste-tags.md");
const REMOVE: &str = include_str!("../../../docs/user-guide/guides/remove-tags.md");
fn section<'a>(page: &'a str, marker: &str) -> &'a str {
    page.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    section(page, marker)
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(text: &str) -> &str {
    text.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
#[test]
fn copy_as_text_executes_the_written_input_and_expression() {
    for page in [COPY.to_owned(), COPY.replace('\n', "\r\n")] {
        let fields: Fields = section(&page, "copy-text-fields")
            .split('`')
            .filter_map(|text| text.split_once('='))
            .map(|(name, value)| (name.to_owned(), vec![value.to_owned()]))
            .collect();
        assert_eq!(fields.len(), 3);
        let examples = rows(&page, "copy-text-example");
        assert_eq!(examples.len(), 1);
        for row in examples {
            assert_eq!(evaluate(code(row[0]), &fields).unwrap(), code(row[1]));
        }
    }
}
fn seed(path: &Path) -> Vec<u8> {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xFF, 0xFB, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
    let mut v2 = Id3v2Tag::new();
    v2.set_title("Paper Boats".into());
    v2.save_to_path(path, WriteOptions::default()).unwrap();
    let mut ape = ApeTag::new();
    ape.insert(ApeItem::new("Title".into(), ItemValue::Text("North Pier".into())).unwrap());
    ape.save_to_path(path, WriteOptions::default()).unwrap();
    let mut v1 = Id3v1Tag::new();
    v1.title = Some("Quiet Harbour".into());
    v1.save_to_path(path, WriteOptions::default()).unwrap();
    audio
}
#[test]
fn written_tag_type_removals_leave_the_named_blocks_and_audio() {
    let dir = tempfile::tempdir().unwrap();
    for page in [REMOVE.to_owned(), REMOVE.replace('\n', "\r\n")] {
        let examples = rows(&page, "remove-kinds-examples");
        assert_eq!(examples.len(), 3);
        for (index, row) in examples.iter().enumerate() {
            let path = dir.path().join(format!("practice-{index}.mp3"));
            let audio = seed(&path);
            let kind = match code(row[0]) {
                "ID3v1" => RemovableTag::Id3v1,
                "ID3v2" => RemovableTag::Id3v2,
                "APEv2" => RemovableTag::Ape,
                other => panic!("Unknown written tag kind {other}"),
            };
            let result =
                remove_tag_kinds_with_options(&path, &[kind], false, &TagOptions::default())
                    .unwrap();
            let mut actual = result.technical["_TAG"][0]
                .split_whitespace()
                .collect::<Vec<_>>();
            let mut expected = code(row[1]).split_whitespace().collect::<Vec<_>>();
            actual.sort_unstable();
            expected.sort_unstable();
            assert_eq!(actual, expected);
            assert!(fs::read(&path)
                .unwrap()
                .windows(audio.len())
                .any(|part| part == audio));
        }
    }
}
