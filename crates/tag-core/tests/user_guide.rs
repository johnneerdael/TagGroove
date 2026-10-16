//! Execute copyable public-guide examples with OpenTag's core, not Mp3tag.
use tag_core::{
    actions::{apply_action_spec, ActionSpec, ActionState},
    evaluate, evaluate_filename_with_context, evaluate_with_context, parse_filename, EvalContext,
    Fields,
};

const FORMAT: &str = include_str!("../../../docs/user-guide/reference/format-strings.md");
const REGEX: &str = include_str!("../../../docs/user-guide/reference/regular-expressions.md");
const PLACEHOLDERS: &str = include_str!("../../../docs/user-guide/reference/placeholders.md");
const GUEST: &str = include_str!("../../../docs/user-guide/cookbook/feat-artists-to-title.md");
const FOLDERS: &str =
    include_str!("../../../docs/user-guide/cookbook/tags-from-folder-and-filename.md");

fn fields() -> Fields {
    [
        ("ARTIST".into(), vec!["Juniper Vale".into()]),
        ("ALBUM".into(), vec!["Harbour Lights".into()]),
        ("TITLE".into(), vec!["Lanterns on the Water".into()]),
        ("TRACK".into(), vec!["2/8".into()]),
        ("YEAR".into(), vec!["2024".into()]),
    ]
    .into()
}

#[test]
fn published_timestamp_example_uses_the_documented_argument_order_and_date_tokens() {
    let example = PLACEHOLDERS
        .lines()
        .find_map(|line| {
            line.strip_prefix('`')
                .and_then(|line| line.strip_suffix("`."))
                .filter(|line| line.to_lowercase().starts_with("$fmtdate("))
        })
        .expect("copyable timestamp expression");
    let values = [("_FILE_MOD_DATETIME_RAW".into(), vec!["1704164645".into()])].into();
    let mut context = EvalContext::new(&values);
    context.timezone_offset_seconds = Some(0);
    assert_eq!(
        evaluate_with_context(example, &context).unwrap(),
        "2024-01-02"
    );
}

#[test]
fn published_conditions_multivalues_and_nested_functions() {
    let mut values = fields();
    for (example, expected) in [
        ("$upper($left(%title%,8))", "LANTERNS"),
        ("$if(1,yes,no)", "no"),
        ("$if(%year%,%year%,unknown year)", "2024"),
        ("$if($eql(%year%,2024),this year,older)", "this year"),
        ("$if2(%albumartist%,%artist%)", "Juniper Vale"),
        ("$if($and(%artist%,%discnumber%),both,not both)", "not both"),
        ("$if($strstr(%title%,Snow),yes,no)", "yes"),
        ("$if($grtr($strstr(%title%,Snow),0),yes,no)", "no"),
        ("$num(%track%,3)", "002"),
        ("$add(%track%,10)", "12"),
    ] {
        assert!(FORMAT.contains(example), "example disappeared: {example}");
        assert_eq!(evaluate(example, &values).unwrap(), expected, "{example}");
    }
    values.insert(
        "ARTIST".into(),
        vec!["Juniper Vale".into(), "Mira Solberg".into()],
    );
    for (example, expected) in [
        ("$meta(artist)", "Juniper Vale, Mira Solberg"),
        ("$meta(artist,1)", "Mira Solberg"),
        ("$meta_sep(artist,' & ')", "Juniper Vale & Mira Solberg"),
    ] {
        assert!(FORMAT.contains(example));
        assert_eq!(evaluate(example, &values).unwrap(), expected, "{example}");
    }
}

#[test]
fn published_filename_templates_and_folder_capture_patterns() {
    let mut values = fields();
    for (example, expected) in [
        (r"[CD$num(%discnumber%,1)\]%title%", "Lanterns on the Water"),
        (
            r"$if2(%albumartist%,%artist%)\%album%[ (%year%)]\$num(%track%,2) - %title%",
            r"Juniper Vale\Harbour Lights (2024)\02 - Lanterns on the Water",
        ),
        (
            r"$upper($left(%artist%,1))\%artist%\%album%",
            r"J\Juniper Vale\Harbour Lights",
        ),
        ("$upper('a/b')", "A_B"),
    ] {
        assert!(FORMAT.contains(example));
        assert_eq!(
            evaluate_filename_with_context(example, &EvalContext::new(&values)).unwrap(),
            expected
        );
    }
    values.insert("DISCNUMBER".into(), vec!["2".into()]);
    let example = r"$if(%discnumber%,CD%discnumber%\,)%title%";
    assert!(FORMAT.contains(example));
    assert_eq!(
        evaluate_filename_with_context(example, &EvalContext::new(&values)).unwrap(),
        r"CD2\Lanterns on the Water"
    );
    let pattern = r"%artist%\%album% (%year%)\%track% - %title%";
    assert!(FOLDERS.contains(pattern));
    let parsed = parse_filename(
        r"Music\Juniper Vale\Harbour Lights (2024)\02 - Lanterns on the Water.mp3",
        pattern,
    )
    .unwrap();
    assert_eq!(parsed["ARTIST"], ["Juniper Vale"]);
    assert_eq!(parsed["ALBUM"], ["Harbour Lights"]);
    assert_eq!(parsed["YEAR"], ["2024"]);
    assert_eq!(parsed["TITLE"], ["Lanterns on the Water"]);
}

#[test]
fn published_guest_artist_recipe_keeps_both_steps_in_order() {
    let expression = GUEST
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("%title% (feat. "))
        .unwrap();
    let mut values = fields();
    values.insert("TITLE".into(), vec!["Paper Boats".into()]);
    for artist in [
        "Juniper Vale feat. Mira Solberg",
        "Juniper Vale FT Mira Solberg",
        "Juniper Vale featuring Mira Solberg",
    ] {
        values.insert("ARTIST".into(), vec![artist.into()]);
        assert_eq!(
            evaluate(expression, &values).unwrap(),
            "Paper Boats (feat. Mira Solberg)"
        );
        let pattern = r"\s+(feat\.?|ft\.?|featuring)\s+.*$";
        assert!(GUEST.contains(pattern));
        assert_eq!(
            regex_compat::replace(artist, pattern, "", false).unwrap(),
            "Juniper Vale"
        );
    }
}

#[test]
fn published_bracketed_guest_recipe_runs_the_written_actions_in_order() {
    let mut state = ActionState {
        fields: [
            (
                "ARTIST".into(),
                vec!["Juniper Vale (feat. Mira Solberg)".into()],
            ),
            ("TITLE".into(), vec!["Paper Boats".into()]),
        ]
        .into(),
        context: Fields::new(),
    };
    let mut count = 0;
    for line in GUEST.lines().filter(|line| {
        line.starts_with("| 1 |") || line.starts_with("| 2 |") || line.starts_with("| 3 |")
    }) {
        let cells = table_cells(line);
        assert_eq!(cells.len(), 5, "{line}");
        count += 1;
        assert_eq!(cells[0], count.to_string(), "recipe order changed");
        let field = literal(&cells[2]);
        let action = match cells[1].as_str() {
            "Replace with regular expression" => ActionSpec::RegexReplace {
                field,
                pattern: literal(&cells[3]),
                replacement: literal(&cells[4]),
                case_sensitive: false,
            },
            "Format value" => ActionSpec::FormatValue {
                field,
                template: literal(&cells[3]),
            },
            other => panic!("unrecognized recipe action: {other}"),
        };
        apply_action_spec(&mut state, &action).unwrap();
    }
    assert_eq!(count, 3, "missing bracketed-guest recipe actions");
    assert_eq!(state.fields["ARTIST"], ["Juniper Vale"]);
    assert_eq!(state.fields["TITLE"], ["Paper Boats (feat. Mira Solberg)"]);
}

fn table_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    for character in line.trim().trim_matches('|').chars() {
        if character == '`' {
            quoted = !quoted;
        }
        if character == '|' && !quoted {
            cells.push(cell.trim().to_string());
            cell.clear();
        } else {
            cell.push(character);
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

fn literal(cell: &str) -> String {
    let text = if cell == "*(empty)*" {
        ""
    } else if let Some(text) = cell.strip_prefix("<code>") {
        text.strip_suffix("</code>").unwrap()
    } else {
        cell.strip_prefix('`').unwrap().split('`').next().unwrap()
    };
    text.replace("&#92;", "\\")
        .replace("&#124;", "|")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace('·', " ")
}

#[test]
fn every_regex_text_pattern_replacement_result_table_runs_as_written() {
    let mut in_examples = false;
    let mut count = 0;
    for line in REGEX.lines() {
        if line == "| Text | Pattern | Replacement | Result |" {
            in_examples = true;
            continue;
        }
        if !line.starts_with('|') {
            in_examples = false;
        }
        if !in_examples || line.starts_with("|---") {
            continue;
        }
        let cells = table_cells(line);
        assert_eq!(cells.len(), 4, "{line}");
        let values: Vec<_> = cells.iter().map(|cell| literal(cell)).collect();
        assert_eq!(
            regex_compat::replace(&values[0], &values[1], &values[2], false).unwrap(),
            values[3],
            "{line}"
        );
        count += 1;
    }
    assert!(count >= 30, "missing regex examples: {count}");
}

#[test]
fn every_copyable_regexp_expression_table_matches_its_result() {
    let mut values = fields();
    let mut count = 0;
    for line in REGEX.lines().filter(|line| line.starts_with("| `$regexp(")) {
        let cells = table_cells(line);
        if cells.len() == 3 && cells[1] != "same" {
            let parts: Vec<_> = cells[1].split('`').collect();
            values.insert(parts[1].into(), vec![parts[3].into()]);
        }
        let expression = literal(&cells[0]);
        let expected = literal(cells.last().unwrap());
        assert_eq!(evaluate(&expression, &values).unwrap(), expected, "{line}");
        count += 1;
    }
    assert_eq!(count, 11);
}

#[test]
fn every_copyable_regex_library_recipe_matches_its_before_and_after() {
    let mut in_library = false;
    let mut count = 0;
    for line in REGEX.lines() {
        if line == "| Goal | Pattern | Replacement | Before → After |" {
            in_library = true;
            continue;
        }
        if !line.starts_with('|') {
            in_library = false;
        }
        if !in_library || line.starts_with("|---") {
            continue;
        }
        let cells = table_cells(line);
        assert_eq!(cells.len(), 4, "{line}");
        let examples: Vec<_> = cells[3].split('`').collect();
        let values = [("TITLE".into(), vec![examples[1].replace('·', " ")])].into();
        let replacement = if cells[2] == "**Leave empty**" {
            String::new()
        } else {
            literal(&cells[2])
        };
        let expression = format!("$regexp(%title%,{},{replacement})", literal(&cells[1]));
        assert_eq!(
            evaluate(&expression, &values).unwrap(),
            examples[3],
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 13);
}
