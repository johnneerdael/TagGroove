//! Execute the action guide's copyable examples against TagGroove's core.
mod guide_examples;
use guide_examples::text_blocks;

use tag_core::{
    actions::{
        apply_action_spec, ActionSpec, ActionState, CaseMode, CoverImageFormat, CoverResize,
        CoverScaleMode, CoverSizeMode,
    },
    adjust_cover_image, evaluate, render_export, ExportOptions, ExportRow, Fields,
};

const CASE: &str = include_str!("../../../docs/user-guide/actions/case-conversion.md");
const CODEPAGE: &str = include_str!("../../../docs/user-guide/actions/convert-codepage.md");
const REGEX: &str = include_str!("../../../docs/user-guide/actions/replace-regexp.md");
const GUESS: &str = include_str!("../../../docs/user-guide/actions/guess-values.md");
const EXPORT: &str = include_str!("../../../docs/user-guide/actions/export.md");
const COVER: &str = include_str!("../../../docs/user-guide/actions/adjust-cover.md");
const FORMAT: &str = include_str!("../../../docs/user-guide/actions/format-value.md");

fn cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn inline(cell: &str) -> &str {
    cell.strip_prefix('`').unwrap().split('`').next().unwrap()
}

fn state(field: &str, values: &[&str]) -> ActionState {
    ActionState {
        fields: [(
            field.into(),
            values.iter().map(|value| (*value).into()).collect(),
        )]
        .into(),
        context: Fields::new(),
    }
}

#[test]
fn case_conversion_tables_run_with_the_written_modes_and_separators() {
    let mut count = 0;
    for line in CASE.lines().filter(|line| {
        line.starts_with("| **lower case**")
            || line.starts_with("| **Mixed Case**")
            || line.starts_with("| **Sentence**")
            || line.starts_with("| **UPPER CASE**")
    }) {
        let row = cells(line);
        let mode = match row[0] {
            "**lower case**" => CaseMode::Lower,
            "**Mixed Case**" => CaseMode::Mixed,
            "**Sentence**" => CaseMode::Sentence,
            "**UPPER CASE**" => CaseMode::Upper,
            _ => unreachable!(),
        };
        let mut input = state("TITLE", &[inline(row[1])]);
        apply_action_spec(
            &mut input,
            &ActionSpec::CaseConversion {
                field: "TITLE".into(),
                mode,
                word_separators: "(-_".into(),
            },
        )
        .unwrap();
        assert_eq!(input.fields["TITLE"], [inline(row[2])], "{line}");
        count += 1;
    }
    assert_eq!(count, 4);
    let mut count = 0;
    for line in CASE.lines().filter(|line| line.starts_with("| `")) {
        let row = cells(line);
        if row.len() != 3 {
            continue;
        }
        let mut input = state("TITLE", &[inline(row[0])]);
        apply_action_spec(
            &mut input,
            &ActionSpec::CaseConversion {
                field: "TITLE".into(),
                mode: CaseMode::Mixed,
                word_separators: if row[1] == "Leave empty" {
                    ""
                } else {
                    inline(row[1])
                }
                .into(),
            },
        )
        .unwrap();
        assert_eq!(input.fields["TITLE"], [inline(row[2])], "{line}");
        count += 1;
    }
    assert_eq!(count, 4);
}

#[test]
fn codepage_examples_decode_the_published_input_with_the_published_page() {
    let mut count = 0;
    for line in CODEPAGE.lines().filter(|line| line.starts_with("| `")) {
        let row = cells(line);
        let codepage = row[1].split('`').nth(1).unwrap();
        let mut input = state("TITLE", &[inline(row[0])]);
        apply_action_spec(
            &mut input,
            &ActionSpec::ConvertCodepage {
                mta_source: None,
                field: "TITLE".into(),
                codepage: codepage.into(),
            },
        )
        .unwrap();
        assert_eq!(input.fields["TITLE"], [inline(row[2])], "{line}");
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn regex_action_table_preserves_each_written_before_and_after() {
    let mut count = 0;
    for line in REGEX.lines().filter(|line| {
        line.starts_with("| Remove ")
            || line.starts_with("| Collapse ")
            || line.starts_with("| Keep only ")
            || line.starts_with("| Put \"The\"")
    }) {
        let row = cells(line);
        let example: Vec<_> = row[4].split('`').collect();
        let field = inline(row[1]);
        let mut input = state(field, &[example[1]]);
        let replacement = match row[3] {
            "Leave empty" => "",
            "One space" => " ",
            text => inline(text),
        };
        apply_action_spec(
            &mut input,
            &ActionSpec::RegexReplace {
                field: field.into(),
                pattern: inline(row[2]).into(),
                replacement: replacement.into(),
                case_sensitive: false,
            },
        )
        .unwrap();
        assert_eq!(input.fields[field], [example[3]], "{line}");
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn guess_values_examples_replace_destinations_and_skip_dummy_text() {
    let rows: Vec<_> = GUESS
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(cells)
        .collect();
    assert_eq!(rows.len(), 4);
    let expected: Vec<Fields> = vec![
        [
            ("ARTIST".into(), vec!["Juniper Vale".into()]),
            ("TITLE".into(), vec!["Paper Boats".into()]),
        ]
        .into(),
        [
            ("YEAR".into(), vec!["2024".into()]),
            ("ALBUM".into(), vec!["Harbour Lights".into()]),
        ]
        .into(),
        [
            ("TRACK".into(), vec!["02".into()]),
            ("TITLE".into(), vec!["Paper Boats".into()]),
        ]
        .into(),
        [(
            "ARTIST".into(),
            vec!["Juniper Vale".into(), "Mira Solberg".into()],
        )]
        .into(),
    ];
    for (row, expected) in rows.into_iter().zip(expected) {
        let mut input = ActionState {
            fields: Fields::new(),
            context: Fields::new(),
        };
        // The source expression reads TITLE, including when TITLE is also a destination.
        input
            .fields
            .insert("TITLE".into(), vec![inline(row[0]).into()]);
        apply_action_spec(
            &mut input,
            &ActionSpec::GuessValues {
                source: "%title%".into(),
                pattern: inline(row[1]).into(),
            },
        )
        .unwrap();
        for (field, values) in expected {
            assert_eq!(input.fields[&field], values);
        }
        assert!(!input.fields.contains_key("DUMMY"));
    }
}

#[test]
fn guess_values_regex_preparation_runs_the_written_stages() {
    let lf = GUESS.replace("\r\n", "\n");
    for page in [lf.clone(), lf.replace('\n', "\r\n")] {
        let section = page
            .split_once("<!-- guess-regex-stages:start -->")
            .unwrap()
            .1
            .split_once("<!-- guess-regex-stages:end -->")
            .unwrap()
            .0;
        let settings = text_blocks(section);
        assert_eq!(settings.len(), 2);
        let rows: Vec<_> = section
            .lines()
            .filter(|line| {
                line.starts_with('|')
                    && !line.starts_with("| Case |")
                    && cells(line).get(1).is_some_and(|v| v.starts_with('`'))
            })
            .map(cells)
            .collect();
        assert_eq!(rows.len(), 4);
        for row in rows {
            assert_eq!(row.len(), 5);
            let comment = inline(row[1]).replace('·', " ").replace('⇥', "\t");
            let mut input = state("COMMENT", &[&comment]);
            input.fields.extend([
                ("YEAR".into(), vec!["1999".into()]),
                ("ALBUM".into(), vec!["Keep this album".into()]),
                ("TITLE".into(), vec!["Unchanged title".into()]),
            ]);
            assert_eq!(
                evaluate(&settings[0], &input.fields).unwrap(),
                inline(row[2])
            );
            let effects = apply_action_spec(
                &mut input,
                &ActionSpec::GuessValues {
                    source: settings[0].clone(),
                    pattern: settings[1].clone(),
                },
            )
            .unwrap();
            assert!(effects.is_empty());
            assert_eq!(input.fields["YEAR"], [inline(row[3])]);
            assert_eq!(input.fields["ALBUM"], [inline(row[4])]);
            assert_eq!(input.fields["COMMENT"], [comment]);
            assert_eq!(input.fields["TITLE"], ["Unchanged title"]);
        }
    }
}

#[test]
fn export_action_configuration_runs_exactly_as_published() {
    let template = text_blocks(EXPORT).remove(0);
    let rows: Vec<_> = [
        ("/music/z.mp3", "Paper Boats"),
        ("/music/a.mp3", "North Pier"),
    ]
    .into_iter()
    .map(|(path, title)| {
        ExportRow::new(
            [
                ("ARTIST".into(), vec!["Juniper Vale".into()]),
                ("TITLE".into(), vec![title.into()]),
            ]
            .into(),
            path,
            1.0,
            1,
        )
    })
    .collect();
    let result = render_export(
        &template,
        &rows,
        &ExportOptions {
            base_directory: "/music".into(),
            filename_override: Some("tags.txt".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].filename, "/music/tags.txt");
    assert_eq!(result[0].encoding, "utf-8");
    assert_eq!(
        result[0].text,
        "Juniper Vale - Paper Boats\nJuniper Vale - North Pier\n"
    );
}

#[test]
fn cover_size_table_matches_the_actual_image_resizer() {
    let original = image::DynamicImage::new_rgb8(600, 300);
    let mut encoded = std::io::Cursor::new(Vec::new());
    original
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let mut count = 0;
    for line in COVER.lines().filter(|line| {
        line.starts_with("| Maximum ")
            || line.starts_with("| Minimum ")
            || line.starts_with("| Size disabled +")
    }) {
        let row = cells(line);
        let (policy, shape) = row[0].split_once(" + ").unwrap();
        let enabled = policy != "Size disabled";
        let pixels = if enabled {
            policy.split_whitespace().nth(1).unwrap().parse().unwrap()
        } else {
            0
        };
        let size_mode = if policy.starts_with("Minimum") {
            CoverSizeMode::Minimum
        } else {
            CoverSizeMode::Maximum
        };
        let modes = match shape {
            "proportional" => vec![CoverScaleMode::Proportional],
            "square" => vec![CoverScaleMode::Square],
            "crop" => vec![CoverScaleMode::CropSquare],
            "square or crop" => vec![CoverScaleMode::Square, CoverScaleMode::CropSquare],
            _ => panic!("{line}"),
        };
        let dimensions: Vec<u32> = row[1]
            .split(',')
            .next()
            .unwrap()
            .split(" × ")
            .map(|number| number.parse().unwrap())
            .collect();
        for scale_mode in modes {
            let result = adjust_cover_image(
                encoded.get_ref(),
                CoverImageFormat::Original,
                90,
                Some(&CoverResize {
                    native_geometry: true,
                    size_enabled: enabled,
                    pixels,
                    size_mode,
                    scale_mode,
                }),
            )
            .unwrap()
            .unwrap();
            let result = image::load_from_memory(&result).unwrap();
            assert_eq!(
                (result.width(), result.height()),
                (dimensions[0], dimensions[1]),
                "{line}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 6);
}

#[test]
fn format_value_output_examples_store_the_written_value_lists() {
    let examples = FORMAT
        .split("<!-- format-value-output-examples:start -->")
        .nth(1)
        .unwrap()
        .split("<!-- format-value-output-examples:end -->")
        .next()
        .unwrap();
    let mut count = 0;
    for line in examples.lines().filter(|line| line.starts_with("| `")) {
        let row = cells(line);
        let expected: Vec<String> = row[1]
            .split('`')
            .skip(1)
            .step_by(2)
            .map(|value| value.replace('·', " "))
            .collect();
        let mut input = state("GENRE", &["Previous", "Other"]);
        input
            .fields
            .insert("TITLE".into(), vec!["Unchanged".into()]);
        let effects = apply_action_spec(
            &mut input,
            &ActionSpec::FormatValue {
                field: "GENRE".into(),
                template: inline(row[0]).into(),
            },
        )
        .unwrap();
        assert!(effects.is_empty(), "{line}");
        if expected.is_empty() {
            assert!(row[1].contains("field removed"), "{line}");
            assert!(!input.fields.contains_key("GENRE"), "{line}");
        } else {
            assert_eq!(input.fields["GENRE"], expected, "{line}");
        }
        assert_eq!(input.fields["TITLE"], ["Unchanged"]);
        count += 1;
    }
    assert_eq!(count, 6);
}
