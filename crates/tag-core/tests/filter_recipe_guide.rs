//! Actual written inspection recipes against the compiled filter and file reader.
mod guide_examples;
use image::{DynamicImage, ImageFormat, RgbImage};
use std::{fs, path::Path};
use tag_core::{add_cover, filter::compile_filter, read_entry, Fields};
const MISSING: &str = include_str!("../../../docs/user-guide/tagging/recipes/find-missing-fields.md");
const COVERS: &str = include_str!("../../../docs/user-guide/tagging/recipes/find-missing-covers.md");
const BITRATE: &str = include_str!("../../../docs/user-guide/tagging/recipes/find-low-bitrate.md");
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
fn check_queries(page: &str, marker: &str, data: &[(&str, Fields)], expected_count: usize) {
    let cases = rows(page, marker);
    assert_eq!(cases.len(), expected_count);
    for row in &cases {
        let filter = compile_filter(code(row[0])).unwrap();
        let found: Vec<_> = data
            .iter()
            .filter_map(|(id, fields)| filter.matches(fields).unwrap().then_some(*id))
            .collect();
        assert_eq!(found.join(","), code(row[1]), "{}", row[0]);
    }
    let examples = guide_examples::text_blocks(page);
    assert_eq!(examples.len(), 1);
    let filter = compile_filter(&examples[0]).unwrap();
    let found: Vec<_> = data
        .iter()
        .filter_map(|(id, fields)| filter.matches(fields).unwrap().then_some(*id))
        .collect();
    assert_eq!(
        found.join(","),
        code(
            cases[if marker == "low-bitrate-queries" {
                1
            } else {
                0
            }][1]
        )
    );
}
#[test]
fn missing_fields_written_snapshots_distinguish_absence_empty_whitespace_and_any_value() {
    for page in forms(MISSING) {
        let snapshots = rows(&page, "missing-fields-snapshots");
        assert_eq!(snapshots.len(), 5);
        let data: Vec<_> = snapshots
            .into_iter()
            .map(|row| {
                let fields = if row[1] == "Absent" {
                    Fields::new()
                } else {
                    Fields::from([(
                        "ALBUMARTIST".into(),
                        serde_json::from_str::<Vec<String>>(code(row[1])).unwrap(),
                    )])
                };
                (row[0], fields)
            })
            .collect();
        check_queries(&page, "missing-fields-queries", &data, 5);
        for (id, fields) in &data {
            assert_eq!(
                compile_filter("ALBUMARTIST ABSENT")
                    .unwrap()
                    .matches(fields)
                    .unwrap(),
                *id == "A"
            );
            let mut album = fields.clone();
            album.insert("ALBUM".into(), vec!["Harbour Lights".into()]);
            assert_eq!(
                compile_filter("(ALBUMARTIST MISSING) AND ALBUM IS \"Harbour Lights\"")
                    .unwrap()
                    .matches(&album)
                    .unwrap(),
                *id == "A"
            );
        }
        assert_eq!(data[2].1["ALBUMARTIST"], ["   "]);
    }
}
#[test]
fn missing_covers_written_queries_count_embedded_entries_without_folder_image_input() {
    for page in forms(COVERS) {
        let snapshots = rows(&page, "missing-covers-snapshots");
        assert_eq!(snapshots.len(), 4);
        let data: Vec<_> = snapshots
            .into_iter()
            .map(|row| {
                assert!(matches!(row[3], "Yes" | "No"));
                (
                    row[0],
                    Fields::from([
                        ("ALBUM".into(), vec![code(row[1]).into()]),
                        ("_COVERS".into(), vec![code(row[2]).into()]),
                    ]),
                )
            })
            .collect();
        check_queries(&page, "missing-covers-queries", &data, 4);
        for (_, fields) in &data {
            assert!(!compile_filter("_COVERS MISSING")
                .unwrap()
                .matches(fields)
                .unwrap());
        }
    }
}
#[test]
fn low_bitrate_written_queries_keep_boundary_zero_and_codec_distinctions() {
    for page in forms(BITRATE) {
        let snapshots = rows(&page, "low-bitrate-snapshots");
        assert_eq!(snapshots.len(), 6);
        let data: Vec<_> = snapshots
            .into_iter()
            .map(|row| {
                (
                    row[0],
                    Fields::from([
                        ("_BITRATE".into(), vec![code(row[1]).into()]),
                        ("_CODEC".into(), vec![code(row[2]).into()]),
                    ]),
                )
            })
            .collect();
        check_queries(&page, "low-bitrate-queries", &data, 5);
        for (id, fields) in &data {
            assert_eq!(
                compile_filter("\"%_bitrate%\" EQUAL 0")
                    .unwrap()
                    .matches(fields)
                    .unwrap(),
                *id == "E"
            );
        }
        assert!(!compile_filter("\"%_bitrate%\" LESS 192")
            .unwrap()
            .matches(&Fields::new())
            .unwrap());
        assert!(compile_filter("\"%_bitrate%\" LESS 191.5").is_err());
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
fn actual_file_measurements_match_written_cover_and_positive_mpeg_bitrate_queries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("practice.mp3");
    let audio = mp3(&path);
    let image = dir.path().join("folder.png");
    DynamicImage::ImageRgb8(RgbImage::new(40, 40))
        .save_with_format(&image, ImageFormat::Png)
        .unwrap();
    let original_image = fs::read(&image).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.technical["_COVERS"], ["0"]);
    assert_eq!(entry.technical["_CODEC"], ["MPEG-1 Layer III"]);
    assert_eq!(entry.technical["_BITRATE"], ["128"]);
    for page in forms(COVERS) {
        let blocks = guide_examples::text_blocks(&page);
        let query = &blocks[0];
        assert!(compile_filter(query)
            .unwrap()
            .matches(&entry.technical)
            .unwrap());
    }
    for page in forms(BITRATE) {
        let blocks = guide_examples::text_blocks(&page);
        let query = &blocks[0];
        assert!(compile_filter(query)
            .unwrap()
            .matches(&entry.technical)
            .unwrap());
    }
    add_cover(&path, &image).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.technical["_COVERS"], ["1"]);
    assert!(!compile_filter(&guide_examples::text_blocks(COVERS)[0])
        .unwrap()
        .matches(&entry.technical)
        .unwrap());
    assert_eq!(fs::read(&image).unwrap(), original_image);
    assert!(fs::read(&path)
        .unwrap()
        .windows(audio.len())
        .any(|part| part == audio));
}
