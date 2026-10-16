//! Execute the actual column-definition examples through the metadata core.
use tag_core::{evaluate, Fields};
const PAGE: &str = include_str!("../../../docs/user-guide/customize/columns.md");
fn rows(marker: &str) -> Vec<Vec<&str>> {
    PAGE.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn literal(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
#[test]
fn column_values_and_sort_keys_match_the_written_metadata_and_results() {
    let input = rows("column-example-fields");
    assert_eq!(input.len(), 3);
    let fields: Fields = input
        .into_iter()
        .map(|row| {
            (
                literal(row[0]).into(),
                row[1..]
                    .iter()
                    .filter(|value| **value != "*(absent)*")
                    .map(|value| literal(value).into())
                    .collect(),
            )
        })
        .collect();
    let examples = rows("column-expression-examples");
    assert_eq!(examples.len(), 3);
    for row in examples {
        let value = literal(row[1]);
        let sort = if row[3] == "*(empty)*" {
            value
        } else {
            literal(row[3])
        };
        assert_eq!(
            evaluate(value, &fields).unwrap(),
            literal(row[4]),
            "{}",
            row[0]
        );
        assert_eq!(
            evaluate(sort, &fields).unwrap(),
            literal(row[5]),
            "{}",
            row[0]
        );
    }
}
