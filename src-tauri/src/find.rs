//! Read-only regex Find. Compile and match on the same blocking worker because
//! the shared Boost handle must remain on its creating thread.
use regex_compat::Regex;

const MAX_PATTERN_BYTES: usize = 65_536;
const MAX_VALUES: usize = 100_000;
const MAX_VALUE_BYTES: usize = 1_048_576;
const MAX_TOTAL_BYTES: usize = 64 * 1_048_576;
static REQUESTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

fn matches(pattern: &str, case_sensitive: bool, values: &[String]) -> Result<Vec<bool>, String> {
    if pattern.is_empty() {
        return Err("Enter text to find.".into());
    }
    if pattern.len() > MAX_PATTERN_BYTES {
        return Err("Find pattern is too long (maximum 64 KiB).".into());
    }
    if values.len() > MAX_VALUES
        || values.iter().any(|value| value.len() > MAX_VALUE_BYTES)
        || values.iter().map(String::len).sum::<usize>() > MAX_TOTAL_BYTES
    {
        return Err("Too much text for one regex Find. Filter the File List to a smaller set, then try again (100,000 values, 1 MiB per value, 64 MiB total).".into());
    }
    let expression = Regex::new(pattern, case_sensitive)
        .map_err(|error| format!("Invalid regular expression: {error}"))?;
    values
        .iter()
        .map(|value| expression.is_match(value).map_err(|error| format!("Regular expression search failed: {error}. Simplify the pattern and try again.")))
        .collect()
}

#[tauri::command]
pub async fn regex_find(
    pattern: String,
    case_sensitive: bool,
    values: Vec<String>,
) -> Result<Vec<bool>, String> {
    let permit = REQUESTS.try_acquire().map_err(|_| {
        "Regex Find is still processing earlier searches. Wait for them to finish, then try again."
            .to_string()
    })?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        matches(&pattern, case_sensitive, &values)
    })
    .await
    .map_err(|error| error.to_string())?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boost_patterns_and_case_options_are_used_in_find() {
        let values = vec![
            "Paper Boats".into(),
            "paper boats".into(),
            "Boats\nPaper".into(),
        ];
        assert_eq!(
            matches(r"\APaper\s+Boats\z", false, &values).unwrap(),
            [true, true, false]
        );
        assert_eq!(
            matches(r"(?i)\Apaper boats\z", true, &values).unwrap(),
            [true, true, false]
        );
        assert_eq!(
            matches("Paper", true, &values).unwrap(),
            [true, false, true]
        );
        assert_eq!(
            matches("^Paper", true, &values).unwrap(),
            [true, false, true]
        );
        assert_eq!(
            matches("Boats.Paper", true, &values).unwrap(),
            [false, false, true]
        );
        assert!(matches("(", false, &[])
            .unwrap_err()
            .starts_with("Invalid regular expression:"));
    }

    #[test]
    fn copied_patterns_agree_across_find_filter_scripting_and_actions() {
        use tag_core::actions::{apply_action_spec, ActionSpec, ActionState};
        for pattern in [
            r"(?i)\APaper\s+Boats\z",
            r"(?i)\APaper\h+Boats\z",
            r"(?<=Paper )Boats",
            r"Paper\K Boats",
        ] {
            for value in [
                "Paper Boats",
                "paper boats",
                "Paper Boats Live",
                "Different",
            ] {
                let fields: tag_core::Fields = [("TITLE".into(), vec![value.into()])].into();
                let found = matches(pattern, false, &[value.into()]).unwrap()[0];
                assert_eq!(
                    tag_core::matches_filter(&format!("title MATCHES \"{pattern}\""), &fields)
                        .unwrap(),
                    found,
                    "filter {pattern} {value}"
                );
                let formatted =
                    tag_core::evaluate(&format!("$regexp(%title%,'{pattern}','#')"), &fields)
                        .unwrap();
                let mut state = ActionState {
                    fields: fields.clone(),
                    ..Default::default()
                };
                apply_action_spec(
                    &mut state,
                    &ActionSpec::RegexReplace {
                        field: "TITLE".into(),
                        pattern: pattern.into(),
                        replacement: "#".into(),
                        case_sensitive: false,
                    },
                )
                .unwrap();
                assert_eq!(
                    state.fields["TITLE"].as_slice(),
                    std::slice::from_ref(&formatted),
                    "action {pattern} {value}"
                );
                assert_eq!(formatted != value, found, "scripting {pattern} {value}");
            }
        }
    }

    #[test]
    fn request_limits_are_explicit_and_do_not_return_partial_results() {
        assert!(matches("", false, &[]).is_err());
        assert!(matches(&"x".repeat(MAX_PATTERN_BYTES + 1), false, &[]).is_err());
        assert!(matches("x", false, &vec![String::new(); MAX_VALUES + 1]).is_err());
        assert!(matches("x", false, &["x".repeat(MAX_VALUE_BYTES + 1)]).is_err());
        assert!(matches("x", false, &vec!["x".repeat(MAX_VALUE_BYTES); 65]).is_err());
    }

    #[test]
    fn documented_find_examples_use_the_native_engine() {
        let guide = include_str!("../../docs/user-guide/scripting/regular-expressions.md");
        let table = guide
            .split("<!-- find-regex-examples:start -->")
            .nth(1)
            .unwrap()
            .split("<!-- find-regex-examples:end -->")
            .next()
            .unwrap();
        let mut count = 0;
        for row in table.lines().filter(|row| row.starts_with("| `")) {
            let cells: Vec<_> = row
                .split('|')
                .skip(1)
                .take(4)
                .map(|cell| cell.trim().trim_matches('`'))
                .collect();
            let result = matches(cells[0], cells[1] == "On", &[cells[2].into()]);
            if cells[3] == "Invalid pattern" {
                assert!(result.is_err(), "{row}");
            } else {
                assert_eq!(result.unwrap(), [cells[3] == "Match"], "{row}");
            }
            count += 1;
        }
        assert!(count >= 5);
    }
}
