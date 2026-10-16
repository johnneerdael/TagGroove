//! Execute the written cookbook instructions in order, not parallel example formulas.
use tag_core::{
    actions::{apply_action_spec, ActionSpec, ActionState},
    evaluate, Fields,
};
const SPACES: &str = include_str!("../../../docs/user-guide/cookbook/trim-spaces.md");
const BRACKETS: &str = include_str!("../../../docs/user-guide/cookbook/remove-text-in-brackets.md");
const ARTISTS: &str = include_str!("../../../docs/user-guide/cookbook/split-multiple-artists.md");
const CREDIT: &str = include_str!("../../../docs/user-guide/cookbook/split-artist-title.md");
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
        .filter(|line| line.starts_with("| "))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>()
        })
        .filter(|row| row[0].starts_with('`') || row[0].parse::<usize>().is_ok())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn visible(value: &str) -> String {
    code(value)
        .replace('·', " ")
        .replace('⇥', "\t")
        .replace('↵', "\n")
}
fn actions(page: &str, marker: &str, count: usize) -> Vec<ActionSpec> {
    let rows = rows(page, marker);
    assert_eq!(rows.len(), count);
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            assert_eq!(row[0].parse::<usize>().unwrap(), index + 1);
            match row[1] {
                "Replace with Regular Expression" => {
                    assert!(page.contains("**Case-sensitive comparison** to **Off**"));
                    ActionSpec::RegexReplace {
                        field: code(row[2]).into(),
                        pattern: code(row[3]).into(),
                        replacement: match row[4] {
                            "Leave empty" => "",
                            "One space" => " ",
                            text => code(text),
                        }
                        .into(),
                        case_sensitive: false,
                    }
                }
                "Split Field by Separator" => ActionSpec::SplitField {
                    field: code(row[2]).into(),
                    separator: code(row[3]).into(),
                },
                "Remove Duplicate Fields" => {
                    assert_eq!(row[3], "Only duplicated fields with the same content");
                    assert_eq!(row[4], "On");
                    ActionSpec::RemoveDuplicateFields {
                        field: code(row[2]).into(),
                        only_same_content: true,
                    }
                }
                "Guess Values" => ActionSpec::GuessValues {
                    source: code(row[2]).into(),
                    pattern: code(row[3]).into(),
                },
                "Format Value" => ActionSpec::FormatValue {
                    field: code(row[2]).into(),
                    template: code(row[3]).into(),
                },
                other => panic!("Unknown written action {other}"),
            }
        })
        .collect()
}
fn run(fields: Fields, recipe: &[ActionSpec]) -> Fields {
    let mut state = ActionState {
        fields,
        context: Fields::new(),
    };
    for action in recipe {
        assert!(apply_action_spec(&mut state, action).unwrap().is_empty());
    }
    state.fields
}
#[test]
fn spacing_recipe_preserves_all_title_values_and_executes_every_written_row() {
    for page in forms(SPACES) {
        let examples = rows(&page, "spacing-cases");
        assert_eq!(examples.len(), 5);
        let input = examples.iter().map(|row| visible(row[0])).collect();
        let expected = examples
            .iter()
            .map(|row| visible(row[1]))
            .collect::<Vec<_>>();
        let actual = run(
            Fields::from([("TITLE".into(), input)]),
            &actions(&page, "spacing-actions", 3),
        );
        assert_eq!(actual["TITLE"], expected);
    }
}
#[test]
fn spacing_preview_formulas_use_the_written_spaces_and_actual_single_space_argument() {
    for page in forms(SPACES) {
        let input = page
            .split("For `TITLE` = `")
            .nth(1)
            .unwrap()
            .split('`')
            .next()
            .unwrap()
            .replace('·', " ");
        let fields = Fields::from([("TITLE".into(), vec![input])]);
        let examples = rows(&page, "spacing-formulas");
        assert_eq!(examples.len(), 2);
        for row in examples {
            assert_eq!(evaluate(code(row[0]), &fields).unwrap(), visible(row[1]));
        }
    }
}
#[test]
fn remaster_suffix_recipe_preserves_nonmatches_and_nested_title_text() {
    for page in forms(BRACKETS) {
        let examples = rows(&page, "bracket-cases");
        assert_eq!(examples.len(), 7);
        let recipe = actions(&page, "bracket-actions", 2);
        for row in examples {
            let actual = run(
                Fields::from([("TITLE".into(), vec![visible(row[0])])]),
                &recipe,
            );
            assert_eq!(actual["TITLE"], [visible(row[1])]);
        }
    }
}
#[test]
fn broader_parenthesis_pattern_matches_exactly_the_written_cautions() {
    for page in forms(BRACKETS) {
        let examples = rows(&page, "broad-bracket-examples");
        assert_eq!(examples.len(), 3);
        for row in examples {
            let recipe = [ActionSpec::RegexReplace {
                field: "TITLE".into(),
                pattern: code(row[0]).into(),
                replacement: String::new(),
                case_sensitive: false,
            }];
            let actual = run(
                Fields::from([("TITLE".into(), vec![visible(row[1])])]),
                &recipe,
            );
            assert_eq!(actual["TITLE"], [visible(row[2])]);
        }
    }
}
#[test]
fn artist_recipe_cleans_before_splitting_and_only_removes_exact_duplicates() {
    for page in forms(ARTISTS) {
        let examples = rows(&page, "artist-list-cases");
        assert_eq!(examples.len(), 5);
        let recipe = actions(&page, "artist-list-actions", 5);
        for row in examples {
            let input: Vec<String> = serde_json::from_str(code(row[0])).unwrap();
            let expected: Vec<String> = serde_json::from_str(code(row[1])).unwrap();
            let actual = run(Fields::from([("ARTIST".into(), input)]), &recipe);
            assert_eq!(actual["ARTIST"], expected);
        }
    }
}
#[test]
fn credit_recipe_keeps_written_order_including_nonmatch_trim_and_delimiter_ambiguity() {
    for page in forms(CREDIT) {
        let examples = rows(&page, "credit-cases");
        assert_eq!(examples.len(), 5);
        let recipe = actions(&page, "credit-actions", 3);
        for row in examples {
            let actual = run(
                Fields::from([
                    ("TITLE".into(), vec![visible(row[0])]),
                    ("ARTIST".into(), vec![visible(row[1])]),
                ]),
                &recipe,
            );
            assert_eq!(actual["ARTIST"], [visible(row[2])]);
            assert_eq!(actual["TITLE"], [visible(row[3])]);
        }
    }
}
