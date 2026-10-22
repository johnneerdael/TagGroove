//! Run the beginner lessons' copyable expressions on their stated practice data.
mod guide_examples;
use guide_examples::text_blocks;

use tag_core::{evaluate, Fields};

const FORMAT: &str = include_str!("../../../docs/user-guide/scripting/format-strings-from-zero.md");
const REGEX: &str = include_str!("../../../docs/user-guide/scripting/regex-from-zero.md");

fn fields(title: &str) -> Fields {
    [
        ("TITLE".into(), vec![title.into()]),
        ("ARTIST".into(), vec!["Juniper Vale".into()]),
        ("ALBUM".into(), vec!["Harbour Lights".into()]),
        ("TRACK".into(), vec!["3/8".into()]),
        ("YEAR".into(), vec!["2024".into()]),
    ]
    .into()
}

#[test]
fn format_lesson_code_blocks_produce_every_stated_intermediate_result() {
    let examples = text_blocks(FORMAT);
    let expected = [
        "Paper Boats",
        "Juniper Vale - Paper Boats",
        "PAPER BOATS",
        "03 - Paper Boats",
        "PAPER",
        "Harbour Lights (2024)",
        "Harbour Lights",
        "Juniper Vale",
        "Paper, folded Boats",
        "Juniper Vale - Paper Boats",
    ];
    assert_eq!(examples.len(), expected.len());
    for (example, expected) in examples.into_iter().zip(expected) {
        assert_eq!(
            evaluate(&example, &fields("Paper Boats")).unwrap(),
            expected,
            "{example}"
        );
    }
}

#[test]
fn regex_lesson_code_blocks_preserve_the_source_until_the_final_apply_step() {
    let examples = text_blocks(REGEX);
    let expected = [
        "03 - Paper Boats (Live 2024)",
        " - Paper_Boats (Live 2024)",
        "Paper_Boats (Live 2024)",
        "Paper_Boats (Live 2024)",
        "03 - Paper_Boats",
        "Paper Boats",
        "03 - Paper_Boats (Live YEAR)",
    ];
    assert_eq!(examples.len(), expected.len());
    for (example, expected) in examples.into_iter().zip(expected) {
        assert_eq!(
            evaluate(&example, &fields("03 - Paper_Boats (Live 2024)")).unwrap(),
            expected,
            "{example}"
        );
    }
}

#[test]
fn format_lesson_expression_tables_handle_truth_quotes_and_visible_spaces() {
    let mut in_examples = false;
    let mut count = 0;
    for line in FORMAT.lines() {
        if line.starts_with("| Expression |") {
            in_examples = true;
            continue;
        }
        if !line.starts_with('|') {
            in_examples = false;
        }
        if !in_examples || !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
        let expression = cells[0]
            .strip_prefix('`')
            .unwrap()
            .strip_suffix('`')
            .unwrap();
        let result = cells[1]
            .strip_prefix("<code>")
            .and_then(|cell| cell.strip_suffix("</code>"))
            .unwrap_or_else(|| {
                cells[1]
                    .strip_prefix('`')
                    .unwrap()
                    .strip_suffix('`')
                    .unwrap()
            });
        assert_eq!(
            evaluate(expression, &fields("Paper Boats")).unwrap(),
            result,
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 11);
}
