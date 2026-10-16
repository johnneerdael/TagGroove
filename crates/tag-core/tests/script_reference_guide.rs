//! Exercise published function tables, including context-dependent examples.
use std::collections::{BTreeMap, BTreeSet};
use tag_core::{evaluate_with_context, EvalContext, Fields, ScriptColumn};

const INDEX: &str = include_str!("../../../docs/user-guide/reference/scripting/index.md");
const PAGES: [(&str, &str); 6] = [
    (
        "text",
        include_str!("../../../docs/user-guide/reference/scripting/text.md"),
    ),
    (
        "search",
        include_str!("../../../docs/user-guide/reference/scripting/search-compare.md"),
    ),
    (
        "conditions",
        include_str!("../../../docs/user-guide/reference/scripting/conditions.md"),
    ),
    (
        "numbers",
        include_str!("../../../docs/user-guide/reference/scripting/numbers.md"),
    ),
    (
        "dates",
        include_str!("../../../docs/user-guide/reference/scripting/dates-durations.md"),
    ),
    (
        "metadata",
        include_str!("../../../docs/user-guide/reference/scripting/lists-metadata.md"),
    ),
];

fn fields(metadata: bool) -> Fields {
    let mut values = Fields::from([
        (
            "ARTIST".into(),
            vec!["Juniper Vale".into(), "Mira Solberg".into()],
        ),
        ("TITLE".into(), vec!["Paper Boats".into()]),
    ]);
    if !metadata {
        values.extend([
            ("ALBUM".into(), vec!["Harbour Lights".into()]),
            ("YEAR".into(), vec!["2024".into()]),
            ("TRACK".into(), vec!["3/8".into()]),
        ]);
    }
    values
}

fn literal(cell: &str) -> String {
    if cell == "*(empty)*" {
        return String::new();
    }
    cell.strip_prefix('`')
        .unwrap()
        .strip_suffix('`')
        .unwrap()
        .replace('·', " ")
}

#[test]
fn function_index_is_exactly_the_runtime_builtin_registry() {
    let names: BTreeSet<_> = INDEX
        .lines()
        .filter_map(|line| {
            line.strip_prefix("| [`$")
                .map(|line| line.split('`').next().unwrap())
        })
        .collect();
    let expected: BTreeSet<_> = tag_core::script::SUPPORTED_FUNCTIONS
        .iter()
        .copied()
        .collect();
    assert_eq!(names, expected);
    assert_eq!(names.len(), 64);
}

#[test]
fn every_published_function_example_runs_with_its_stated_result() {
    let columns = vec![
        ScriptColumn {
            name: "Title".into(),
            value: "Paper Boats".into(),
        },
        ScriptColumn {
            name: "Artist".into(),
            value: "Juniper Vale".into(),
        },
    ];
    let environment = BTreeMap::from([("GUIDE_READER".into(), "listener".into())]);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut total = 0;
    for (name, page) in PAGES {
        let fields = fields(name == "metadata");
        let mut section = "";
        let mut examples = false;
        for line in page.lines() {
            if let Some(heading) = line.strip_prefix("### `$") {
                section = heading.split('(').next().unwrap();
                counts.entry(section.into()).or_default();
            }
            if line == "| Format string | With these values | Result |" {
                examples = true;
                continue;
            }
            if !line.starts_with('|') {
                examples = false;
            }
            if !examples || !line.starts_with("| `$") {
                continue;
            }
            let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 3, "{name}: {line}");
            let expression = literal(cells[0]);
            let mut context = EvalContext::new(&fields);
            context.locale = "en-US".into();
            context.timezone_offset_seconds = Some(0);
            context.timestamp_seconds = Some(1_704_164_645);
            context.ansi_codepage = Some("windows-1252".into());
            context.columns = Some(if cells[1] == "No columns in supplied context" {
                &[]
            } else {
                &columns
            });
            context.environment = Some(&environment);
            context.app_directory = Some("/opt/OpenTag");
            context.app_data_directory = Some("/home/listener/.config/opentag");
            let result = evaluate_with_context(&expression, &context)
                .unwrap_or_else(|error| panic!("{name}: {expression}: {error}"));
            if cells[2].starts_with('`') || cells[2] == "*(empty)*" {
                assert_eq!(result, literal(cells[2]), "{name}: {line}");
            } else {
                match expression.as_str() {
                    "$rand()" => assert!(result.parse::<u32>().unwrap() <= 32767),
                    "$num($rand(),5)" => {
                        assert_eq!(result.len(), 5);
                        assert!(result.bytes().all(|byte| byte.is_ascii_digit()));
                    }
                    "$mod($rand(),10)" => assert!(result.parse::<u32>().unwrap() <= 9),
                    "$uuid()" | "$upper($uuid())" => {
                        let uuid = uuid::Uuid::parse_str(&result).unwrap();
                        assert_eq!(uuid.get_version_num(), 4);
                        assert_eq!(result.len(), 36);
                        if expression == "$upper($uuid())" {
                            assert_eq!(result, result.to_uppercase());
                        } else {
                            assert_eq!(result, result.to_lowercase());
                        }
                    }
                    other => panic!("Unvalidated dynamic example: {other}"),
                }
            }
            *counts
                .get_mut(section)
                .expect("example outside a function section") += 1;
            total += 1;
        }
    }
    let expected: BTreeSet<_> = tag_core::script::SUPPORTED_FUNCTIONS
        .iter()
        .copied()
        .filter(|name| *name != "regexp")
        .collect();
    assert_eq!(
        counts.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected
    );
    for (function, count) in counts {
        assert!(count >= 3, "{function} has only {count} examples");
    }
    assert!(total >= 190, "example count decreased: {total}");
}

#[test]
fn every_published_date_token_matches_its_utc_example() {
    let page = PAGES.iter().find(|(name, _)| *name == "dates").unwrap().1;
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.locale = "en-US".into();
    context.timezone_offset_seconds = Some(0);
    context.timestamp_seconds = Some(1_704_164_645);
    let mut in_tokens = false;
    let mut count = 0;
    for line in page.lines() {
        if line == "| Token | Meaning | Example result |" {
            in_tokens = true;
            continue;
        }
        if !line.starts_with('|') {
            in_tokens = false;
        }
        if !in_tokens || !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
        let expression = format!("$fmtdate({})", literal(cells[0]));
        assert_eq!(
            evaluate_with_context(&expression, &context).unwrap(),
            literal(cells[2]),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 20);
}
