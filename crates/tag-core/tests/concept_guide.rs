//! Exercise the concept pages and beginner workflow lessons as written.
mod guide_examples;
use guide_examples::text_blocks;

use tag_core::{
    actions::{apply_action_spec, ActionSpec, ActionState, CaseMode},
    evaluate_filename_with_context, evaluate_with_context, EvalContext, Fields,
};

const VALUES: &str =
    include_str!("../../../docs/user-guide/tagging/concepts/multi-value-fields.md");
const ENCODINGS: &str = include_str!("../../../docs/user-guide/tagging/concepts/text-encodings.md");
const ACTIONS: &str = include_str!("../../../docs/user-guide/actions/first-action-group.md");
const RENAME: &str =
    include_str!("../../../docs/user-guide/converters/rename-files-and-folders.md");

fn cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn literal(cell: &str) -> &str {
    cell.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}

#[test]
fn concept_expression_tables_keep_multivalues_and_codepage_loss_explicit() {
    let fields: Fields = [(
        "ARTIST".into(),
        vec!["Juniper Vale".into(), "Mira Solberg".into()],
    )]
    .into();
    let mut context = EvalContext::new(&fields);
    context.ansi_codepage = Some("windows-1252".into());
    let mut count = 0;
    for page in [VALUES, ENCODINGS] {
        for line in page
            .lines()
            .filter(|line| line.starts_with("| `%") || line.starts_with("| `$"))
        {
            let row = cells(line);
            assert_eq!(row.len(), 2, "{line}");
            assert_eq!(
                evaluate_with_context(literal(row[0]), &context).unwrap(),
                literal(row[1]),
                "{line}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 8);
}

fn action_settings(label: &str) -> &str {
    ACTIONS
        .lines()
        .find_map(|line| {
            line.strip_prefix(&format!("| **{label}** | "))
                .map(|value| value.strip_suffix(" |").unwrap())
        })
        .unwrap()
}

#[test]
fn first_action_lesson_executes_the_published_settings_in_the_written_order() {
    let mut state = ActionState {
        fields: [("TITLE".into(), vec!["paper_boats (LIVE)".into()])].into(),
        context: Fields::new(),
    };
    assert_eq!(action_settings("Replace with"), "One actual space");
    assert_eq!(action_settings("Case"), "**Mixed Case**");
    let mut count = 0;
    for line in ACTIONS.lines().filter(|line| {
        line.starts_with("| 1 |") || line.starts_with("| 2 |") || line.starts_with("| 3 |")
    }) {
        let row = cells(line);
        count += 1;
        assert_eq!(row[0], count.to_string());
        let field = literal(row[2]).to_owned();
        let action = match row[1] {
            "Format Value" => ActionSpec::FormatValue {
                field: field.clone(),
                template: literal(action_settings("Format string")).into(),
            },
            "Replace" => ActionSpec::Replace {
                field: field.clone(),
                from: literal(action_settings("Original string")).into(),
                to: " ".into(),
                whole_word: false,
                case_sensitive: false,
            },
            "Case Conversion" => ActionSpec::CaseConversion {
                field: field.clone(),
                mode: CaseMode::Mixed,
                word_separators: literal(action_settings("Words begin after")).into(),
            },
            other => panic!("Unknown lesson action: {other}"),
        };
        apply_action_spec(&mut state, &action).unwrap();
        assert_eq!(state.fields[&field], [literal(row[3])], "{line}");
    }
    assert_eq!(count, 3);
    assert_eq!(state.fields["COMMENT"], ["paper_boats (LIVE)"]);
    assert_eq!(state.fields["TITLE"], ["Paper Boats (Live)"]);
}

#[test]
fn filename_lesson_progression_matches_both_discs() {
    let blocks = text_blocks(RENAME);
    assert_eq!(blocks.len(), 5);
    for (disc, title, expected_stem) in [
        ("1", "Paper Boats", "01 - Paper Boats"),
        ("2", "North Pier", "01 - North Pier"),
    ] {
        let fields: Fields = [
            ("TITLE".into(), vec![title.into()]),
            ("TRACK".into(), vec!["1".into()]),
            ("DISCNUMBER".into(), vec![disc.into()]),
            ("ALBUM".into(), vec!["Harbour Lights".into()]),
        ]
        .into();
        let context = EvalContext::new(&fields);
        assert_eq!(
            evaluate_filename_with_context(&blocks[0], &context).unwrap(),
            expected_stem
        );
        let album = evaluate_filename_with_context(&blocks[1], &context)
            .unwrap()
            .replace('\\', "/")
            + ".mp3";
        assert!(blocks[2].lines().any(|line| line == album));
        let with_disc = evaluate_filename_with_context(&blocks[3], &context)
            .unwrap()
            .replace('\\', "/")
            + ".mp3";
        assert!(blocks[4].lines().any(|line| line == with_disc));
    }
}
