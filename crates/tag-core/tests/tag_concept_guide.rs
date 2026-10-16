//! Written foundational snapshots and tag-priority examples against actual core.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    id3::{v1::Id3v1Tag, v2::Id3v2Tag},
    prelude::TagExt,
    tag::{Accessor, ItemValue},
};
use std::{fs, path::Path};
use tag_core::{evaluate_with_context, read_entry_with_options, EvalContext, Fields, TagOptions};
const BASICS: &str = include_str!("../../../docs/user-guide/concepts/what-are-tags.md");
const FORMATS: &str = include_str!("../../../docs/user-guide/concepts/tag-formats.md");
const NUMBERS: &str = include_str!("../../../docs/user-guide/concepts/track-and-disc-numbers.md");
fn forms(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    [lf.clone(), lf.replace('\n', "\r\n")]
}
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
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn fields(page: &str, marker: &str) -> (Fields, Fields) {
    let mut raw = Fields::new();
    let mut enriched = Fields::new();
    for row in rows(page, marker) {
        let name = code(row[1]).to_owned();
        let values: Vec<String> = serde_json::from_str(code(row[2])).unwrap();
        assert!(row[0] == "Metadata" || row[0] == "Information");
        if row[0] == "Metadata" {
            raw.insert(name.clone(), values.clone());
        }
        enriched.insert(name, values);
    }
    (raw, enriched)
}
#[test]
fn tag_basics_reads_actual_raw_metadata_and_information_snapshot_without_writing() {
    for page in forms(BASICS) {
        let (raw, enriched) = fields(&page, "tag-basics-fields");
        assert_eq!(raw.len(), 4);
        assert_eq!(enriched.len(), 5);
        assert!(!raw.contains_key("_FILENAME_EXT"));
        let mut context = EvalContext::new(&enriched);
        context.raw_fields = Some(&raw);
        let cases = rows(&page, "tag-basics-expressions");
        assert_eq!(cases.len(), 5);
        for row in cases {
            assert_eq!(
                evaluate_with_context(code(row[0]), &context).unwrap(),
                code(row[1])
            );
        }
        assert_eq!(raw["TITLE"], ["Paper Boats"]);
        assert_eq!(enriched["_FILENAME_EXT"], ["07 - Old Label.mp3"]);
    }
}
#[test]
fn number_concept_distinguishes_saved_track_pair_and_current_operation_counter() {
    for page in forms(NUMBERS) {
        let (raw, enriched) = fields(&page, "number-concept-fields");
        assert_eq!(raw.len(), 3);
        assert_eq!(enriched.len(), 4);
        assert!(!raw.contains_key("_COUNTER"));
        let mut context = EvalContext::new(&enriched);
        context.raw_fields = Some(&raw);
        let cases = rows(&page, "number-concept-expressions");
        assert_eq!(cases.len(), 5);
        for row in cases {
            assert_eq!(
                evaluate_with_context(code(row[0]), &context).unwrap(),
                code(row[1])
            );
        }
        assert_eq!(raw["TRACK"], ["3/8"]);
        assert_eq!(raw["DISCNUMBER"], ["2/3"]);
    }
}
fn mp3(path: &Path) -> Vec<u8> {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xff, 0xfb, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
    audio
}
#[test]
fn written_stored_blocks_and_read_switches_choose_one_tag_without_merging_or_writing() {
    for page in forms(FORMATS) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("practice.mp3");
        let audio = mp3(&path);
        let blocks = rows(&page, "format-stored-blocks");
        assert_eq!(blocks.len(), 3);
        for row in blocks {
            match row[0] {
                "ID3v1" => {
                    let mut tag = Id3v1Tag::new();
                    tag.title = Some(code(row[1]).into());
                    assert_eq!(row[2], "Absent");
                    tag.save_to_path(&path, WriteOptions::default()).unwrap();
                }
                "ID3v2" => {
                    let mut tag = Id3v2Tag::new();
                    tag.set_title(code(row[1]).into());
                    tag.set_album(code(row[2]).into());
                    tag.save_to_path(&path, WriteOptions::default()).unwrap();
                }
                "APEv2" => {
                    let mut tag = ApeTag::new();
                    tag.insert(
                        ApeItem::new("TITLE".into(), ItemValue::Text(code(row[1]).into())).unwrap(),
                    );
                    assert_eq!(row[2], "Absent");
                    tag.save_to_path(&path, WriteOptions::default()).unwrap();
                }
                other => panic!("Unknown written block {other}"),
            }
        }
        let before = fs::read(&path).unwrap();
        let cases = rows(&page, "format-read-priority");
        assert_eq!(cases.len(), 4);
        for row in cases {
            let mut opts = TagOptions::default();
            opts.mpeg.read_id3v1 = row[0] == "On";
            opts.mpeg.read_id3v2 = row[1] == "On";
            opts.mpeg.read_ape = row[2] == "On";
            let entry = read_entry_with_options(&path, &[], &opts).unwrap();
            let get = |key: &str| {
                entry
                    .fields
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(key))
                    .map(|(_, values)| values.clone())
            };
            assert_eq!(get("TITLE"), Some(vec![code(row[3]).into()]));
            if row[4] == "Absent" {
                assert_eq!(get("ALBUM"), None);
            } else {
                assert_eq!(get("ALBUM"), Some(vec![code(row[4]).into()]));
            }
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert!(before.windows(audio.len()).any(|part| part == audio));
    }
}
