//! Execute the actual written number/date recipes with current core semantics.
mod guide_examples;
use guide_examples::text_blocks;
use std::collections::BTreeMap;
use tag_core::{
    actions::{apply_action_spec, ActionSpec, ActionState},
    evaluate,
    numbering::{plan_numbering, NumberingItem, NumberingOptions},
    Fields,
};
const TRACK: &str = include_str!("../../../docs/user-guide/cookbook/split-track-total.md");
const DISC: &str = include_str!("../../../docs/user-guide/cookbook/disc-numbers-from-folders.md");
const FOLDER: &str = include_str!("../../../docs/user-guide/cookbook/number-tracks-per-folder.md");
const YEAR: &str = include_str!("../../../docs/user-guide/cookbook/year-from-date.md");
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
fn act(fields: Fields, context: Fields, actions: &[ActionSpec]) -> Fields {
    let mut state = ActionState { fields, context };
    for action in actions {
        assert!(apply_action_spec(&mut state, action).unwrap().is_empty());
    }
    state.fields
}
#[test]
fn track_total_extraction_uses_both_written_formulas_in_written_order() {
    for page in forms(TRACK) {
        let body = section(&page, "track-total-formulas");
        let fields = body
            .lines()
            .filter_map(|line| line.strip_prefix("**Field:** "))
            .map(code)
            .collect::<Vec<_>>();
        let formulas = text_blocks(body);
        assert_eq!(fields, ["TRACKTOTAL", "TRACK"]);
        assert_eq!(formulas.len(), 2);
        let actions = fields
            .iter()
            .zip(&formulas)
            .map(|(field, formula)| ActionSpec::FormatValue {
                field: (*field).into(),
                template: formula.clone(),
            })
            .collect::<Vec<_>>();
        let cases = rows(&page, "track-total-cases");
        assert_eq!(cases.len(), 6);
        for row in &cases {
            let input = Fields::from([
                ("TRACK".into(), vec![code(row[0]).into()]),
                ("TRACKTOTAL".into(), vec![code(row[1]).into()]),
            ]);
            let actual = act(input, Fields::new(), &actions);
            assert_eq!(actual["TRACK"], [code(row[2])]);
            assert_eq!(actual["TRACKTOTAL"], [code(row[3])]);
        }
        let row = &cases[0];
        let input = Fields::from([
            ("TRACK".into(), vec![code(row[0]).into()]),
            ("TRACKTOTAL".into(), vec![code(row[1]).into()]),
        ]);
        let reversed = act(
            input,
            Fields::new(),
            &actions.into_iter().rev().collect::<Vec<_>>(),
        );
        assert_eq!(reversed["TRACKTOTAL"], [code(row[1])]);
        assert_ne!(reversed["TRACKTOTAL"], [code(row[3])]);
    }
}
#[test]
fn disc_folder_guard_and_unguarded_extraction_match_each_written_case() {
    for page in forms(DISC) {
        let guard = text_blocks(section(&page, "disc-folder-formula"));
        let extraction = text_blocks(section(&page, "disc-extraction-formula"));
        assert_eq!(guard.len(), 1);
        assert_eq!(extraction.len(), 1);
        let cases = rows(&page, "disc-folder-cases");
        assert_eq!(cases.len(), 6);
        for row in cases {
            let actual = act(
                Fields::from([("DISCNUMBER".into(), vec![code(row[1]).into()])]),
                Fields::from([("_DIRECTORY".into(), vec![code(row[0]).into()])]),
                &[ActionSpec::FormatValue {
                    field: "DISCNUMBER".into(),
                    template: guard[0].clone(),
                }],
            );
            assert_eq!(actual["DISCNUMBER"], [code(row[2])]);
        }
        let cases = rows(&page, "disc-extraction-cases");
        assert_eq!(cases.len(), 4);
        for row in cases {
            assert_eq!(
                evaluate(
                    &extraction[0],
                    &Fields::from([("_DIRECTORY".into(), vec![code(row[0]).into()])])
                )
                .unwrap(),
                code(row[1])
            );
        }
    }
}
#[test]
fn year_shortcut_reads_four_characters_including_the_written_trailing_space() {
    for page in forms(YEAR) {
        let cases = rows(&page, "year-left-cases");
        assert_eq!(cases.len(), 4);
        for row in cases {
            assert_eq!(
                evaluate(
                    code(row[1]),
                    &Fields::from([("YEAR".into(), vec![code(row[0]).into()])])
                )
                .unwrap(),
                code(row[2])
            );
        }
    }
}
#[test]
fn year_regex_processes_all_values_without_claiming_calendar_validation() {
    for page in forms(YEAR) {
        let settings = rows(&page, "year-regex-settings")
            .into_iter()
            .map(|row| (row[0], row[1]))
            .collect::<BTreeMap<_, _>>();
        let cases = rows(&page, "year-regex-cases");
        assert_eq!(cases.len(), 6);
        assert_eq!(settings["Case-sensitive comparison"], "Off");
        let input = cases.iter().map(|row| code(row[0]).to_owned()).collect();
        let expected = cases
            .iter()
            .map(|row| code(row[1]).to_owned())
            .collect::<Vec<_>>();
        let actual = act(
            Fields::from([("YEAR".into(), input)]),
            Fields::new(),
            &[ActionSpec::RegexReplace {
                field: code(settings["Field"]).into(),
                pattern: code(settings["Pattern"]).into(),
                replacement: code(settings["Replace matches with"]).into(),
                case_sensitive: false,
            }],
        );
        assert_eq!(actual["YEAR"], expected);
    }
}
fn options(page: &str) -> NumberingOptions {
    let s = rows(page, "folder-numbering-options")
        .into_iter()
        .map(|row| (row[0], row[1]))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(s["Disc numbers"], "Off");
    assert_eq!(s["Next disc on change of field"], "Do not change counter");
    assert_eq!(
        s["Reset disc counter on change of field"],
        "Do not change counter"
    );
    NumberingOptions {
        track_start: code(s["Begin at track"]).parse().unwrap(),
        track_digits: code(s["Minimum digits under Track numbers"])
            .parse()
            .unwrap(),
        count_track_total: s["Save total track count"] == "On",
        next_disc_on_directory: s["Next disc on change of directory"] == "On",
        reset_track_on_next_disc: s["Reset track counter on next disc"] == "On",
        ..Default::default()
    }
}
#[test]
fn per_folder_controls_produce_the_written_plan_without_writing_disc_values() {
    for page in forms(FOLDER) {
        let cases = rows(&page, "folder-numbering-cases");
        assert_eq!(cases.len(), 5);
        let items = cases
            .iter()
            .map(|row| NumberingItem {
                path: code(row[0]).into(),
                fields: Fields::from([
                    ("TRACK".into(), vec![code(row[1]).into()]),
                    ("DISCNUMBER".into(), vec!["7".into()]),
                ]),
            })
            .collect::<Vec<_>>();
        let plan = plan_numbering(&items, &options(&page)).unwrap();
        for (update, row) in plan.iter().zip(cases) {
            assert_eq!(update.path, code(row[0]));
            assert_eq!(update.fields["TRACK"], [code(row[2])]);
            assert!(!update.fields.contains_key("DISCNUMBER"));
        }
    }
}
#[test]
fn numbering_totals_use_the_final_number_of_each_adjacent_run() {
    for page in forms(FOLDER) {
        let cases = rows(&page, "folder-total-variants");
        assert_eq!(cases.len(), 3);
        for row in cases {
            let count = code(row[1]).parse::<usize>().unwrap();
            let mut opts = options(&page);
            opts.track_start = code(row[0]).parse().unwrap();
            let items = (0..count)
                .map(|i| NumberingItem {
                    path: format!("/Practice/Bonus/{i}.mp3"),
                    fields: Fields::new(),
                })
                .collect::<Vec<_>>();
            let plan = plan_numbering(&items, &opts).unwrap();
            assert_eq!(plan.first().unwrap().fields["TRACK"], [code(row[2])]);
            assert_eq!(plan.last().unwrap().fields["TRACK"], [code(row[3])]);
        }
        let cases = rows(&page, "folder-interleaved-cases");
        assert_eq!(cases.len(), 3);
        let items = cases
            .iter()
            .map(|row| NumberingItem {
                path: code(row[0]).into(),
                fields: Fields::new(),
            })
            .collect::<Vec<_>>();
        let plan = plan_numbering(&items, &options(&page)).unwrap();
        for (update, row) in plan.iter().zip(cases) {
            assert_eq!(update.fields["TRACK"], [code(row[1])]);
        }
    }
}
