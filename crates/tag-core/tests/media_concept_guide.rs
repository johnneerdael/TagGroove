//! Execute the concept pages' stored metadata and preservation examples.
use lofty::{
    ape::{ApeItem, ApeTag},
    config::WriteOptions,
    prelude::TagExt,
    tag::ItemValue,
};
use std::{fs, path::Path};
use tag_core::{
    evaluate, read_entry, read_entry_with_options, write_fields, write_fields_with_options, Fields,
    TagOptions,
};
const SORT: &str = include_str!("../../../docs/user-guide/tagging/concepts/sort-fields.md");
const GAIN: &str = include_str!("../../../docs/user-guide/tagging/concepts/replaygain.md");
fn forms(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    [lf.clone(), lf.replace('\n', "\r\n")]
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    page.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn written_fields(page: &str, marker: &str) -> Fields {
    rows(page, marker)
        .into_iter()
        .map(|row| (code(row[0]).into(), vec![code(row[1]).into()]))
        .collect()
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
fn retains_audio(path: &Path, audio: &[u8]) {
    assert!(fs::read(path)
        .unwrap()
        .windows(audio.len())
        .any(|part| part == audio));
}
#[test]
fn written_sort_metadata_roundtrips_and_fallbacks_do_not_replace_display_credit() {
    for page in forms(SORT) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sort.mp3");
        let audio = mp3(&path);
        let fields = written_fields(&page, "sort-concept-fields");
        assert_eq!(fields.len(), 7);
        write_fields(&path, &fields).unwrap();
        let actual = read_entry(&path).unwrap().fields;
        for (key, values) in &fields {
            assert_eq!(&actual[key], values);
        }
        let cases = rows(&page, "sort-concept-expressions");
        assert_eq!(cases.len(), 3);
        for row in &cases {
            assert_eq!(evaluate(code(row[1]), &actual).unwrap(), code(row[2]));
        }
        let mut no_sort = actual.clone();
        no_sort.remove("ARTISTSORT");
        assert_eq!(
            evaluate(code(cases[1][1]), &no_sort).unwrap(),
            "The Lanterns"
        );
        assert_eq!(actual["ARTIST"], ["The Lanterns"]);
        retains_audio(&path, &audio);
    }
}
#[test]
fn written_compilation_flag_requests_reject_zero_and_true_without_changing_file() {
    for page in forms(SORT) {
        let cases = rows(&page, "compilation-concept-cases");
        assert_eq!(cases.len(), 4);
        for row in cases {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("compilation.mp3");
            let audio = mp3(&path);
            write_fields(
                &path,
                &Fields::from([("COMPILATION".into(), vec!["1".into()])]),
            )
            .unwrap();
            let before = fs::read(&path).unwrap();
            let values = if row[0] == "Remove field" {
                Vec::new()
            } else {
                vec![code(row[0]).into()]
            };
            let result = write_fields(&path, &Fields::from([("COMPILATION".into(), values)]));
            match row[1] {
                "Error; existing `1` remains" => {
                    assert!(result.unwrap_err().contains("requires 1 or deletion"));
                    assert_eq!(fs::read(&path).unwrap(), before);
                    assert_eq!(read_entry(&path).unwrap().fields["COMPILATION"], ["1"]);
                }
                "Absent" => {
                    assert!(!result.unwrap().fields.contains_key("COMPILATION"));
                }
                value => {
                    assert_eq!(result.unwrap().fields["COMPILATION"], [code(value)]);
                }
            }
            retains_audio(&path, &audio);
        }
    }
}
#[test]
fn four_written_gain_values_roundtrip_as_metadata_and_explicit_deletion_removes_them() {
    for page in forms(GAIN) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gain.mp3");
        let audio = mp3(&path);
        let fields = written_fields(&page, "replaygain-concept-fields");
        assert_eq!(fields.len(), 4);
        let entry = write_fields(&path, &fields).unwrap();
        for (key, values) in &fields {
            assert_eq!(&entry.fields[key], values);
        }
        assert_eq!(
            evaluate("%replaygain_track_gain%", &entry.fields).unwrap(),
            "-4.25 dB"
        );
        let removal = fields.keys().map(|key| (key.clone(), Vec::new())).collect();
        let entry = write_fields(&path, &removal).unwrap();
        for key in fields.keys() {
            assert!(!entry.fields.contains_key(key));
        }
        retains_audio(&path, &audio);
    }
}
#[test]
fn written_undisplayed_ape_preservation_keeps_gains_in_their_original_kinds() {
    for page in forms(GAIN) {
        let cases = rows(&page, "replaygain-concept-migration");
        assert_eq!(cases.len(), 3);
        for row in cases {
            assert_eq!(row.len(), 4);
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("hidden-ape.mp3");
            let audio = mp3(&path);
            let mut options = TagOptions::default();
            options.mpeg.read_ape = false;
            options.mpeg.read_id3v2 = true;
            options.mpeg.write_id3v2 = true;
            options.mpeg.write_ape = false;
            options.mpeg.remove_ape = true;
            let mut fields = Fields::from([("TITLE".into(), vec!["Paper Boats".into()])]);
            if row[1] != "Absent" {
                fields.insert("REPLAYGAIN_TRACK_GAIN".into(), vec![code(row[1]).into()]);
            }
            write_fields_with_options(&path, &fields, false, &[], &options).unwrap();
            let mut ape = ApeTag::new();
            ape.insert(
                ApeItem::new("TITLE".into(), ItemValue::Text("Unshown APE title".into())).unwrap(),
            );
            if row[0] != "Absent" {
                ape.insert(
                    ApeItem::new(
                        "REPLAYGAIN_TRACK_GAIN".into(),
                        ItemValue::Text(code(row[0]).into()),
                    )
                    .unwrap(),
                );
            }
            ape.save_to_path(&path, WriteOptions::default()).unwrap();
            write_fields_with_options(
                &path,
                &Fields::from([("ARTIST".into(), vec!["Juniper Vale".into()])]),
                false,
                &[],
                &options,
            )
            .unwrap();
            let entry = read_entry_with_options(&path, &[], &options).unwrap();
            assert_eq!(entry.fields["TITLE"], ["Paper Boats"]);
            assert_eq!(entry.fields["ARTIST"], ["Juniper Vale"]);
            if row[2] == "Absent" {
                assert!(!entry.fields.contains_key("REPLAYGAIN_TRACK_GAIN"));
            } else {
                assert_eq!(entry.fields["REPLAYGAIN_TRACK_GAIN"], [code(row[2])]);
            }
            assert!(entry.technical["_TAG"]
                .iter()
                .any(|value| value.contains("APE")));
            let mut ape_view = options.clone();
            ape_view.mpeg.read_ape = true;
            let retained = read_entry_with_options(&path, &[], &ape_view).unwrap();
            assert_eq!(retained.fields["TITLE"], ["Unshown APE title"]);
            assert!(!retained.fields.contains_key("ARTIST"));
            if row[3] == "Absent" {
                assert!(!retained.fields.contains_key("REPLAYGAIN_TRACK_GAIN"));
            } else {
                assert_eq!(retained.fields["REPLAYGAIN_TRACK_GAIN"], [code(row[3])]);
            }
            retains_audio(&path, &audio);
        }
    }
}
