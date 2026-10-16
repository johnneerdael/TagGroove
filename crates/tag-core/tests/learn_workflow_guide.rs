//! Run the three learning workflows' actual written patterns, steps and images.
mod guide_examples;
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path};
use tag_core::{
    actions::{
        apply_action_spec, ActionEffect, ActionSpec, ActionState, CaseMode, CoverImageFormat,
        CoverResize, CoverScaleMode, CoverSizeMode,
    },
    adjust_cover_image, evaluate_filename_with_context,
    filter::compile_filter,
    import_cover_with_options,
    numbering::{plan_numbering, NumberingItem, NumberingOptions},
    read_entry, resolve_cover_source, CompiledFilenamePattern, EvalContext, Fields, TagOptions,
};
const FILES: &str = include_str!("../../../docs/user-guide/learn/tags-from-filenames.md");
const CLEAN: &str = include_str!("../../../docs/user-guide/learn/clean-a-messy-library.md");
const COVERS: &str = include_str!("../../../docs/user-guide/learn/give-every-album-a-cover.md");
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
fn block(page: &str, marker: &str) -> String {
    let blocks = guide_examples::text_blocks(section(page, marker));
    assert_eq!(blocks.len(), 1);
    blocks[0].clone()
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    section(page, marker)
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(s: &str) -> &str {
    s.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
#[test]
fn filename_lesson_written_layout_and_simple_captures_match_without_fabricating_other_fields() {
    for page in forms(FILES) {
        let paths = block(&page, "learn-filename-layout");
        let cases = rows(&page, "learn-filename-basic-cases");
        assert_eq!(cases.len(), 2);
        let basic =
            CompiledFilenamePattern::compile(&block(&page, "learn-filename-basic")).unwrap();
        let folders =
            CompiledFilenamePattern::compile(&block(&page, "learn-filename-folders")).unwrap();
        for (path, row) in paths.lines().zip(cases) {
            assert!(path.ends_with(code(row[0])));
            let basic = basic.parse(path).unwrap();
            assert_eq!(basic["TRACK"], [code(row[1])]);
            assert_eq!(basic["TITLE"], [code(row[2])]);
            assert_eq!(basic.len(), 2);
            let full = folders.parse(path).unwrap();
            assert_eq!(full["TRACK"], basic["TRACK"]);
            assert_eq!(full["TITLE"], basic["TITLE"]);
            assert_eq!(full["ARTIST"], ["Juniper Vale"]);
            assert_eq!(full["ALBUM"], ["Harbour Lights"]);
            assert_eq!(full["YEAR"], ["2024"]);
        }
        assert!(basic.try_parse("01. Paper Boats.mp3").unwrap().is_none());
    }
}
#[test]
fn filename_lesson_written_ambiguous_deluxe_and_dummy_patterns_obey_actual_lazy_captures() {
    for page in forms(FILES) {
        let folders =
            CompiledFilenamePattern::compile(&block(&page, "learn-filename-folders")).unwrap();
        let cases = rows(&page, "learn-filename-ambiguity");
        assert_eq!(cases.len(), 2);
        for row in cases {
            let path = format!("Music/Juniper Vale/{}/01 - Paper Boats.mp3", code(row[0]));
            let values = folders.parse(&path).unwrap();
            assert_eq!(values["ALBUM"], [code(row[1])]);
            assert_eq!(values["YEAR"], [code(row[2])]);
        }
        let values = CompiledFilenamePattern::compile(&block(&page, "learn-filename-deluxe"))
            .unwrap()
            .parse("Music/Juniper Vale/Harbour Lights (Deluxe) (2024)/01 - Paper Boats.mp3")
            .unwrap();
        assert_eq!(values["ALBUM"], ["Harbour Lights"]);
        assert_eq!(values["YEAR"], ["2024"]);
        let values = CompiledFilenamePattern::compile(&block(&page, "learn-filename-dummy"))
            .unwrap()
            .parse("03 - North Pier - archive.mp3")
            .unwrap();
        assert_eq!(values["TRACK"], ["03"]);
        assert_eq!(values["TITLE"], ["North Pier"]);
        assert!(!values.contains_key("DUMMY"));
        assert!(CompiledFilenamePattern::compile("%artist%%title%").is_err());
        let mut state = ActionState {
            fields: Fields::from([("TITLE".into(), vec!["Juniper Vale / Paper Boats".into()])]),
            context: Fields::new(),
        };
        assert!(apply_action_spec(
            &mut state,
            &ActionSpec::GuessValues {
                source: "%title%".into(),
                pattern: "%artist% / %title%".into()
            }
        )
        .unwrap()
        .is_empty());
        assert_eq!(state.fields["ARTIST"], ["Juniper Vale"]);
        assert_eq!(state.fields["TITLE"], ["Paper Boats"]);
    }
}
fn actions(page: &str) -> Vec<ActionSpec> {
    let rows = rows(page, "learn-cleanup-actions");
    assert_eq!(rows.len(), 6);
    rows.iter()
        .map(|r| match r[1] {
            "Replace with Regular Expression" => ActionSpec::RegexReplace {
                field: code(r[2]).into(),
                pattern: code(r[3]).into(),
                replacement: if r[4].starts_with("One actual space") {
                    " ".into()
                } else {
                    assert!(r[4].starts_with("Leave empty"));
                    String::new()
                },
                case_sensitive: false,
            },
            "Case Conversion" => {
                assert_eq!(r[3], "Mixed Case");
                assert_eq!(r[4], "Words begin after `(-_`");
                ActionSpec::CaseConversion {
                    field: code(r[2]).into(),
                    mode: CaseMode::Mixed,
                    word_separators: "(-_".into(),
                }
            }
            "Format Value" => ActionSpec::FormatValue {
                field: code(r[2]).into(),
                template: code(r[3]).into(),
            },
            "Remove Fields" => ActionSpec::RemoveFields {
                fields: vec![code(r[2]).into()],
            },
            other => panic!("Unknown written action {other}"),
        })
        .collect()
}
fn clean(page: &str, title: &str) -> Fields {
    let before = Fields::from([
        ("TITLE".into(), vec![title.replace('·', " ")]),
        ("ARTIST".into(), vec!["Juniper Vale".into()]),
        ("ALBUM".into(), vec!["Harbour Lights".into()]),
        (
            "PRACTICE_NOTE".into(),
            vec!["Delete after this lesson".into()],
        ),
        ("COMMENT".into(), vec!["Retain my useful note".into()]),
    ]);
    assert!(compile_filter(&block(page, "learn-cleanup-filter"))
        .unwrap()
        .matches(&before)
        .unwrap());
    let mut state = ActionState {
        fields: before.clone(),
        context: Fields::new(),
    };
    for action in actions(page) {
        assert!(apply_action_spec(&mut state, &action).unwrap().is_empty());
    }
    assert_eq!(state.fields["ARTIST"], before["ARTIST"]);
    assert_eq!(state.fields["ALBUM"], before["ALBUM"]);
    assert_eq!(state.fields["COMMENT"], before["COMMENT"]);
    assert_eq!(state.fields["ALBUMARTIST"], ["Juniper Vale"]);
    assert!(!state.fields.contains_key("PRACTICE_NOTE"));
    assert!(!compile_filter(&block(page, "learn-cleanup-filter"))
        .unwrap()
        .matches(&state.fields)
        .unwrap());
    state.fields
}
#[test]
fn cleanup_lesson_actual_six_written_actions_obey_before_after_and_retention_policies() {
    for page in forms(CLEAN) {
        let cases = rows(&page, "learn-cleanup-cases");
        assert_eq!(cases.len(), 3);
        for row in cases {
            let after = clean(&page, code(row[1]));
            assert_eq!(after["TITLE"], [code(row[2])]);
        }
        let empty = Fields::from([("ALBUMARTIST".into(), vec![String::new()])]);
        assert!(!compile_filter(&block(&page, "learn-cleanup-filter"))
            .unwrap()
            .matches(&empty)
            .unwrap());
    }
}
#[test]
fn cleanup_lesson_actual_numbering_then_written_format_match_all_final_destination_rows() {
    for page in forms(CLEAN) {
        let cases = rows(&page, "learn-cleanup-cases");
        let final_rows = rows(&page, "learn-cleanup-final");
        assert_eq!(final_rows.len(), 3);
        let items: Vec<_> = cases
            .iter()
            .map(|row| NumberingItem {
                path: format!("/Practice/{}", code(row[0])),
                fields: clean(&page, code(row[1])),
            })
            .collect();
        let numbered = plan_numbering(&items, &NumberingOptions::default()).unwrap();
        for ((item, update), row) in items.iter().zip(numbered).zip(final_rows) {
            assert!(item.path.ends_with(code(row[0])));
            assert_eq!(update.fields["TRACK"], [code(row[1])]);
            assert_eq!(update.fields.len(), 1);
            let mut fields = item.fields.clone();
            fields.extend(update.fields);
            let name = evaluate_filename_with_context(
                &block(&page, "learn-cleanup-rename"),
                &EvalContext::new(&fields),
            )
            .unwrap();
            assert_eq!(format!("{name}.mp3"), code(row[2]));
        }
    }
}
fn picture(width: u32, height: u32, colour: u8) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb([colour, 40, 180])))
        .write_to(&mut out, ImageFormat::Jpeg)
        .unwrap();
    out.into_inner()
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
fn settings<'a>(page: &'a str, marker: &str) -> BTreeMap<&'a str, &'a str> {
    rows(page, marker)
        .into_iter()
        .map(|r| (r[0], r[1]))
        .collect()
}
#[test]
fn cover_lesson_written_count_snapshots_distinguish_embedded_entries_from_folder_images() {
    for page in forms(COVERS) {
        let filter = compile_filter(&block(&page, "learn-covers-filter")).unwrap();
        let cases = rows(&page, "learn-covers-counts");
        assert_eq!(cases.len(), 3);
        for row in cases {
            let fields = Fields::from([("_COVERS".into(), vec![code(row[1]).into()])]);
            assert_eq!(filter.matches(&fields).unwrap(), row[3] == "Yes");
        }
    }
}
#[test]
fn cover_lesson_written_per_folder_imports_save_distinct_images_without_changing_audio_or_sidecars()
{
    for page in forms(COVERS) {
        let dir = tempfile::tempdir().unwrap();
        let layout = block(&page, "learn-covers-layout");
        let mut paths = Vec::new();
        for line in layout.lines() {
            let path = dir.path().join(line);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if line.ends_with(".jpg") {
                let colour = if line.contains("Harbour Lights") {
                    210
                } else {
                    40
                };
                fs::write(path, picture(40, 30, colour)).unwrap();
            } else {
                let audio = mp3(&path);
                paths.push((path, audio));
            }
        }
        assert_eq!(paths.len(), 3);
        let settings = settings(&page, "learn-covers-import");
        assert_eq!(settings["Import cover as"], "Front Cover");
        assert_eq!(settings["Delete existing covers"], "Off");
        let query = compile_filter(&block(&page, "learn-covers-filter")).unwrap();
        let mut cover_digests = Vec::new();
        for (path, audio) in paths {
            assert!(query
                .matches(&read_entry(&path).unwrap().technical)
                .unwrap());
            let mut state = ActionState {
                fields: Fields::new(),
                context: Fields::from([(
                    "_FOLDERPATH".into(),
                    vec![format!(
                        "{}{}",
                        path.parent().unwrap().display(),
                        std::path::MAIN_SEPARATOR
                    )],
                )]),
            };
            let effects = apply_action_spec(
                &mut state,
                &ActionSpec::ImportCover {
                    filename_template: code(settings["Image filename format"]).into(),
                    picture_type: Some("CoverFront".into()),
                    delete_existing: false,
                },
            )
            .unwrap();
            let [ActionEffect::ImportCover { filename, .. }] = effects.as_slice() else {
                panic!("Unexpected effects {effects:?}")
            };
            let source = resolve_cover_source(Path::new(filename)).unwrap();
            let original = fs::read(&source).unwrap();
            import_cover_with_options(
                &path,
                &source,
                Some("CoverFront"),
                false,
                &TagOptions::default(),
            )
            .unwrap();
            let entry = read_entry(&path).unwrap();
            assert_eq!(entry.covers.len(), 1);
            assert_eq!(entry.covers[0].picture_type, "CoverFront");
            assert!(!query.matches(&entry.technical).unwrap());
            assert_eq!(fs::read(&source).unwrap(), original);
            assert!(fs::read(&path)
                .unwrap()
                .windows(audio.len())
                .any(|part| part == audio));
            cover_digests.push(entry.covers[0].clone());
            import_cover_with_options(
                &path,
                &source,
                Some("CoverFront"),
                false,
                &TagOptions::default(),
            )
            .unwrap();
            assert_eq!(read_entry(&path).unwrap().covers.len(), 2);
        }
        assert_eq!(cover_digests[0].digest, cover_digests[1].digest);
        assert_ne!(cover_digests[0].digest, cover_digests[2].digest);
    }
}
#[test]
fn cover_lesson_written_adjustment_dimensions_follow_maximum_proportions_and_retain_small_bytes() {
    for page in forms(COVERS) {
        let values = settings(&page, "learn-covers-adjust");
        assert_eq!(values["Image format"], "Original");
        assert_eq!(values["Size policy"], "Maximum size");
        assert_eq!(values["Scaling"], "Scale proportionally");
        assert_eq!(values["Resize cover"], "On");
        assert_eq!(values["Quality"], "High (`90` for JPEG)");
        let size = code(values["Size in pixels"]).parse().unwrap();
        let cases = rows(&page, "learn-covers-dimensions");
        assert_eq!(cases.len(), 2);
        let dimensions = |s: &str| {
            let mut n = code(s).split(" × ").map(|n| n.parse::<u32>().unwrap());
            (n.next().unwrap(), n.next().unwrap())
        };
        for row in cases {
            let (w, h) = dimensions(row[0]);
            let expected = dimensions(row[1]);
            let original = picture(w, h, 110);
            let result = adjust_cover_image(
                &original,
                CoverImageFormat::Original,
                90,
                Some(&CoverResize {
                    pixels: size,
                    size_mode: CoverSizeMode::Maximum,
                    scale_mode: CoverScaleMode::Proportional,
                }),
            )
            .unwrap();
            let image = image::load_from_memory(result.as_deref().unwrap_or(&original)).unwrap();
            assert_eq!((image.width(), image.height()), expected);
            if (w, h) == expected {
                assert!(result.is_none());
            }
        }
    }
}
