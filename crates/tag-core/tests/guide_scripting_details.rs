//! Execute the guide's numeric, optional-field and separator examples unchanged.
use tag_core::{evaluate, Fields};

const NUMBERS: &str = include_str!("../../../docs/user-guide/reference/scripting/numbers.md");
const TEXT: &str = include_str!("../../../docs/user-guide/reference/scripting/text.md");
const FORMAT: &str = include_str!("../../../docs/user-guide/reference/format-strings.md");

fn forms(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    [lf.clone(), lf.replace('\n', "\r\n")]
}

fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    let section = page
        .split_once(&format!("<!-- {marker}:start -->"))
        .unwrap_or_else(|| panic!("Missing published examples: {marker}"))
        .1
        .split_once(&format!("<!-- {marker}:end -->"))
        .unwrap()
        .0;
    section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}

fn literal(cell: &str) -> &str {
    if cell == "*(empty)*" {
        ""
    } else {
        cell.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
    }
}

#[test]
fn published_numeric_examples_distinguish_conversion_from_operation_overflow() {
    for page in forms(NUMBERS) {
        let cases = rows(&page, "numeric-overflow-examples");
        assert_eq!(cases.len(), 2);
        for case in cases {
            assert_eq!(case.len(), 3);
            let expression = literal(case[0]);
            let result = evaluate(expression, &Fields::new());
            if let Some(error) = case[2].strip_prefix("Error: ") {
                assert_eq!(result.unwrap_err(), literal(error), "{expression}");
            } else {
                assert_eq!(result.unwrap(), literal(case[2]), "{expression}");
            }
        }
    }
}

#[test]
fn published_optional_metadata_examples_use_the_written_present_or_missing_fields() {
    for page in forms(FORMAT) {
        let cases = rows(&page, "optional-metadata-examples");
        assert_eq!(cases.len(), 4);
        for case in cases {
            assert_eq!(case.len(), 3);
            let mut fields = Fields::new();
            if case[1] != "`ARTIST` absent" {
                let values = case[1]
                    .split('`')
                    .enumerate()
                    .filter_map(|(index, value)| (index % 2 == 1).then_some(value))
                    .collect::<Vec<_>>();
                assert_eq!(values[0], "ARTIST");
                fields.insert(
                    values[0].into(),
                    values[1..].iter().map(|value| (*value).into()).collect(),
                );
            }
            let expression = literal(case[0]);
            assert_eq!(
                evaluate(expression, &fields).unwrap(),
                literal(case[2]),
                "{case:?}"
            );
        }
    }
}

#[test]
fn published_caps_examples_cover_replacement_ascii_folding_and_distinct_sentence_case() {
    for page in forms(TEXT) {
        for (marker, count) in [
            ("caps-separator-examples", 6),
            ("caps2-separator-examples", 6),
            ("caps3-trigger-examples", 4),
        ] {
            let cases = rows(&page, marker);
            assert_eq!(cases.len(), count);
            for case in cases {
                assert_eq!(case.len(), 3);
                let expression = literal(case[0]);
                assert_eq!(
                    evaluate(expression, &Fields::new()).unwrap(),
                    literal(case[2]),
                    "{expression}"
                );
            }
        }
    }
}
