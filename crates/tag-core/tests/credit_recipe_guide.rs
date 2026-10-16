//! Execute actual written credit/sort/title recipes and their selection preconditions.
mod guide_examples;
use tag_core::{
    actions::{apply_action_spec, ActionSpec, ActionState},
    evaluate,
    filter::compile_filter,
    Fields,
};
const ALBUM: &str = include_str!("../../../docs/user-guide/cookbook/album-artist-from-artist.md");
const TITLE: &str =
    include_str!("../../../docs/user-guide/cookbook/strip-track-numbers-from-titles.md");
const SORT: &str = include_str!("../../../docs/user-guide/cookbook/the-artist-sort-names.md");
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
    let values = guide_examples::text_blocks(section(page, marker));
    assert_eq!(values.len(), 1);
    values[0].clone()
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    section(page, marker)
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn list(value: &str) -> Option<Vec<String>> {
    if value == "Absent" {
        None
    } else {
        Some(serde_json::from_str(code(value)).unwrap())
    }
}
fn visible(value: &str) -> String {
    if value == "(empty text)" {
        String::new()
    } else {
        code(value)
            .replace('·', " ")
            .replace('⇥', "\t")
            .replace('↵', "\n")
    }
}
fn snapshot(artist: &str, target: &str, value: &str) -> Fields {
    let mut fields = Fields::new();
    if let Some(values) = list(artist) {
        fields.insert("ARTIST".into(), values);
    }
    if let Some(values) = list(value) {
        fields.insert(target.into(), values);
    }
    fields
}
fn apply(fields: Fields, action: &ActionSpec) -> Fields {
    let mut state = ActionState {
        fields,
        context: Fields::new(),
    };
    assert!(apply_action_spec(&mut state, action).unwrap().is_empty());
    state.fields
}
#[test]
fn album_credit_written_primary_rule_obeys_actual_presence_selection_and_preserves_unselected_lists(
) {
    for page in forms(ALBUM) {
        let cases = rows(&page, "album-credit-cases");
        assert_eq!(cases.len(), 8);
        let filter = compile_filter(&block(&page, "album-credit-filter")).unwrap();
        let action = ActionSpec::FormatValue {
            field: "ALBUMARTIST".into(),
            template: block(&page, "album-credit-format"),
        };
        for row in cases {
            let before = snapshot(row[1], "ALBUMARTIST", row[2]);
            let matches = filter.matches(&before).unwrap();
            assert_eq!(matches, row[3] == "Yes", "{}", row[0]);
            let after = if matches {
                apply(before.clone(), &action)
            } else {
                before.clone()
            };
            assert_eq!(
                after.get("ALBUMARTIST").cloned(),
                list(row[4]),
                "{}",
                row[0]
            );
            assert_eq!(after.get("ARTIST"), before.get("ARTIST"));
        }
    }
}
#[test]
fn list_aware_album_variation_uses_actual_separator_specific_trim_and_empty_rules() {
    for page in forms(ALBUM) {
        let cases = rows(&page, "album-credit-list-cases");
        assert_eq!(cases.len(), 4);
        let action = ActionSpec::FormatValue {
            field: "ALBUMARTIST".into(),
            template: block(&page, "album-credit-list-format"),
        };
        for row in cases {
            let before = snapshot(row[0], "ALBUMARTIST", "Absent");
            let after = apply(before.clone(), &action);
            assert_eq!(after.get("ALBUMARTIST").cloned(), list(row[1]));
            assert_eq!(after.get("ARTIST"), before.get("ARTIST"));
        }
    }
}
#[test]
fn title_prefix_action_preserves_nonmatches_newlines_empty_values_and_track_while_preview_reads_first(
) {
    for page in forms(TITLE) {
        let cases = rows(&page, "numbered-title-cases");
        assert_eq!(cases.len(), 10);
        let input: Vec<String> = cases.iter().map(|row| visible(row[0])).collect();
        let expected: Vec<String> = cases.iter().map(|row| visible(row[1])).collect();
        let fields = Fields::from([
            ("TITLE".into(), input.clone()),
            ("TRACK".into(), vec!["03/8".into()]),
        ]);
        let action = ActionSpec::RegexReplace {
            field: "TITLE".into(),
            pattern: block(&page, "numbered-title-pattern"),
            replacement: String::new(),
            case_sensitive: false,
        };
        let after = apply(fields, &action);
        assert_eq!(after["TITLE"], expected);
        assert_eq!(after["TRACK"], ["03/8"]);
        let preview = block(&page, "numbered-title-preview");
        for (input, expected) in input.iter().zip(expected) {
            assert_eq!(
                evaluate(
                    &preview,
                    &Fields::from([("TITLE".into(), vec![input.clone()])])
                )
                .unwrap(),
                expected
            );
            if input.contains('\n') {
                let crlf_input = input.replace('\n', "\r\n");
                let crlf_expected = expected.replace('\n', "\r\n");
                assert_eq!(
                    evaluate(
                        &preview,
                        &Fields::from([("TITLE".into(), vec![crlf_input])])
                    )
                    .unwrap(),
                    crlf_expected
                );
            }
        }
    }
}
#[test]
fn sort_expression_matches_actual_written_cases_and_case_sensitive_variation_without_changing_artist(
) {
    for page in forms(SORT) {
        let cases = rows(&page, "artist-sort-cases");
        assert_eq!(cases.len(), 7);
        let template = block(&page, "artist-sort-format");
        let action = ActionSpec::FormatValue {
            field: "ARTISTSORT".into(),
            template: template.clone(),
        };
        for row in cases {
            let before = Fields::from([("ARTIST".into(), vec![visible(row[0])])]);
            let expected = visible(row[1]);
            assert_eq!(evaluate(&template, &before).unwrap(), expected);
            let after = apply(before.clone(), &action);
            assert_eq!(after.get("ARTIST"), before.get("ARTIST"));
            assert_eq!(
                after.get("ARTISTSORT").cloned(),
                (!expected.is_empty()).then_some(vec![expected])
            );
        }
        let sensitive = block(&page, "artist-sort-sensitive");
        let cases = rows(&page, "artist-sort-sensitive-cases");
        assert_eq!(cases.len(), 3);
        for row in cases {
            assert_eq!(
                evaluate(
                    &sensitive,
                    &Fields::from([("ARTIST".into(), vec![visible(row[0])])])
                )
                .unwrap(),
                visible(row[1])
            );
        }
    }
}
#[test]
fn sort_recipe_actual_filter_and_primary_credit_policy_keep_existing_multi_value_metadata() {
    for page in forms(SORT) {
        let cases = rows(&page, "artist-sort-list-cases");
        assert_eq!(cases.len(), 4);
        let filter = compile_filter(&block(&page, "artist-sort-filter")).unwrap();
        let action = ActionSpec::FormatValue {
            field: "ARTISTSORT".into(),
            template: block(&page, "artist-sort-format"),
        };
        for row in cases {
            let before = snapshot(row[0], "ARTISTSORT", row[1]);
            let matched = filter.matches(&before).unwrap();
            assert_eq!(matched, row[2] == "Yes");
            let after = if matched {
                apply(before.clone(), &action)
            } else {
                before.clone()
            };
            assert_eq!(after.get("ARTIST"), before.get("ARTIST"));
            assert_eq!(after.get("ARTISTSORT").cloned(), list(row[3]));
        }
    }
}
