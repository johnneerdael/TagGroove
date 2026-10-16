//! Execute written title/encoding/keep-list recipes with actual current action semantics.
mod guide_examples;
use guide_examples::text_blocks;
use std::collections::BTreeMap;
use tag_core::{
    actions::{apply_action_spec, ActionEffect, ActionSpec, ActionState, CaseMode},
    Fields,
};
const CASE: &str = include_str!("../../../docs/user-guide/cookbook/fix-capitalisation.md");
const CODEPAGE: &str = include_str!("../../../docs/user-guide/cookbook/fix-mojibake.md");
const KEEP: &str = include_str!("../../../docs/user-guide/cookbook/remove-junk-fields.md");
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
fn literal(value: &str) -> String {
    code(value).replace('·', " ")
}
fn state(fields: Fields) -> ActionState {
    ActionState {
        fields,
        context: Fields::new(),
    }
}
fn case_actions(page: &str) -> Vec<ActionSpec> {
    let rows = rows(page, "capitalisation-actions");
    assert_eq!(rows.len(), 7);
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            assert_eq!(row[0].parse::<usize>().unwrap(), index + 1);
            match row[1] {
                "Case Conversion" => {
                    assert_eq!(row[3], "Mixed Case");
                    ActionSpec::CaseConversion {
                        field: code(row[2]).into(),
                        mode: CaseMode::Mixed,
                        word_separators: code(row[4]).into(),
                    }
                }
                "Replace with Regular Expression" => ActionSpec::RegexReplace {
                    field: code(row[2]).into(),
                    pattern: code(row[3]).into(),
                    replacement: if row[4] == "Leave empty" {
                        String::new()
                    } else {
                        code(row[4]).into()
                    },
                    case_sensitive: false,
                },
                other => panic!("Unknown written action {other}"),
            }
        })
        .collect()
}
fn run(state: &mut ActionState, actions: &[ActionSpec]) {
    for action in actions {
        assert!(apply_action_spec(state, action).unwrap().is_empty());
    }
}
#[test]
fn capitalization_recipe_processes_all_values_and_restores_only_written_exceptions() {
    for page in forms(CASE) {
        assert!(page
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("**Case-sensitive comparison** to **Off**"));
        let cases = rows(&page, "capitalisation-cases");
        assert_eq!(cases.len(), 7);
        let original_artist = vec!["iOS & AC/DC".into()];
        let mut input = state(Fields::from([
            (
                "TITLE".into(),
                cases.iter().map(|row| literal(row[0])).collect(),
            ),
            ("ARTIST".into(), original_artist.clone()),
        ]));
        run(&mut input, &case_actions(&page));
        assert_eq!(
            input.fields["TITLE"],
            cases.iter().map(|row| literal(row[1])).collect::<Vec<_>>()
        );
        assert_eq!(input.fields["ARTIST"], original_artist);
    }
}
#[test]
fn mixed_case_after_exceptions_reproduces_the_written_ordering_caution() {
    for page in forms(CASE) {
        let examples = rows(&page, "capitalisation-cases");
        let actions = case_actions(&page);
        let mut input = state(Fields::from([(
            "TITLE".into(),
            vec![literal(examples[0][0])],
        )]));
        run(&mut input, &actions[..3]);
        let intermediate = page
            .split("The first practice title becomes `")
            .nth(1)
            .unwrap()
            .split('`')
            .next()
            .unwrap();
        assert_eq!(input.fields["TITLE"], [intermediate]);
        run(&mut input, &actions[3..]);
        assert_eq!(input.fields["TITLE"], [literal(examples[0][1])]);
        run(&mut input, &actions[2..3]);
        assert_eq!(input.fields["TITLE"], [intermediate]);
    }
}
#[test]
fn known_utf8_repairs_and_error_cases_follow_the_actual_written_settings() {
    for page in forms(CODEPAGE) {
        let settings = rows(&page, "mojibake-settings")
            .into_iter()
            .map(|row| (row[0], row[1]))
            .collect::<BTreeMap<_, _>>();
        let field = code(settings["Field"]);
        let codepage = settings["Codepage"].split('`').nth(1).unwrap();
        let action = ActionSpec::ConvertCodepage {
            field: field.into(),
            codepage: codepage.into(),
        };
        let examples = rows(&page, "mojibake-cases");
        assert_eq!(examples.len(), 7);
        for row in examples {
            let original = Fields::from([(field.into(), vec![code(row[0]).into()])]);
            let mut input = state(original.clone());
            let result = apply_action_spec(&mut input, &action);
            if row[1] == "Conversion error" {
                assert!(result.is_err(), "{}", row[0]);
                assert_eq!(input.fields, original);
            } else {
                assert!(result.unwrap().is_empty());
                assert_eq!(input.fields[field], [code(row[1])]);
            }
        }
    }
}
fn inventory(page: &str) -> Fields {
    let rows = rows(page, "keep-list-cases");
    assert_eq!(rows.len(), 11);
    rows.into_iter()
        .map(|row| (code(row[0]).into(), vec![code(row[1]).into()]))
        .collect()
}
#[test]
fn full_written_keep_list_preserves_named_values_and_requests_no_cover_removal() {
    for page in forms(KEEP) {
        let blocks = text_blocks(section(&page, "keep-list-value"));
        assert_eq!(blocks.len(), 1);
        let fields = blocks[0]
            .split(';')
            .map(|name| name.trim().to_uppercase())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        let mut input = state(inventory(&page));
        let effects =
            apply_action_spec(&mut input, &ActionSpec::RemoveFieldsExcept { fields }).unwrap();
        assert!(effects.is_empty());
        let cases = rows(&page, "keep-list-cases");
        for row in cases {
            assert_eq!(
                input.fields.contains_key(code(row[0])),
                row[2] == "Yes",
                "{}",
                row[0]
            );
            if row[2] == "Yes" {
                assert_eq!(input.fields[code(row[0])], [code(row[1])]);
            }
        }
        assert!(!input.fields.contains_key("ALBUMARTIST"));
    }
}
#[test]
fn four_written_artwork_variants_make_removal_an_explicit_effect() {
    for page in forms(KEEP) {
        let cases = rows(&page, "keep-artwork-cases");
        assert_eq!(cases.len(), 4);
        for row in cases {
            let names = if row[0] == "Leave empty" {
                Vec::new()
            } else {
                code(row[0]).split(';').map(str::to_owned).collect()
            };
            let mut input = state(inventory(&page));
            let effects = apply_action_spec(
                &mut input,
                &ActionSpec::RemoveFieldsExcept { fields: names },
            )
            .unwrap();
            if row[2] == "Keep embedded pictures" {
                assert!(effects.is_empty());
            } else {
                assert_eq!(row[2], "Remove embedded pictures");
                assert_eq!(effects, [ActionEffect::RemoveCovers]);
            }
            if row[1] == "Keep TITLE and ARTIST only" {
                assert_eq!(
                    input.fields.keys().map(String::as_str).collect::<Vec<_>>(),
                    ["ARTIST", "TITLE"]
                );
            } else {
                assert_eq!(row[1], "Remove all exposed tag fields");
                assert!(input.fields.is_empty());
            }
        }
    }
}
