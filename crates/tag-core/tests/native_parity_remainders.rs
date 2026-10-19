use serde::Deserialize;
use tag_core::actions::{apply_action_spec, ActionSpec, ActionState};
use tag_core::{compile_filter, Fields};

#[derive(Deserialize)]
struct FilterVector {
    id: usize,
    fields: Fields,
    expression: String,
    native_compiles: bool,
    native_matches: Option<bool>,
}

#[test]
fn frozen_native_filter_quoting_and_precedence_vectors() {
    let cases: Vec<FilterVector> =
        serde_json::from_str(include_str!("fixtures/parity-native/filter-quoting.json")).unwrap();
    assert_eq!(cases.len(), 12);
    let mut mismatches = Vec::new();
    for mut case in cases {
        case.fields
            .insert("_FILENAME_EXT".into(), vec!["song.mp3".into()]);
        let actual = compile_filter(&case.expression);
        if actual.is_ok() != case.native_compiles {
            mismatches.push(format!("case {}: compile={:?}", case.id, actual.err()));
        } else if let Some(expected) = case.native_matches {
            let actual = actual.unwrap().matches(&case.fields).unwrap();
            if actual != expected {
                mismatches.push(format!(
                    "case {}: expected {expected}, got {actual}",
                    case.id
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[derive(Deserialize)]
struct SplitVector {
    id: usize,
    field: String,
    input_values: Vec<String>,
    delimiter: String,
    native_output_values: Option<Vec<String>>,
    native_field_present: bool,
}

#[test]
fn native_filter_operands_keep_their_meaning_under_boolean_grouping() {
    let cases: Vec<FilterVector> =
        serde_json::from_str(include_str!("fixtures/parity-native/filter-quoting.json")).unwrap();
    for mut case in cases.into_iter().filter(|case| case.native_compiles) {
        case.fields
            .insert("_FILENAME_EXT".into(), vec!["song.mp3".into()]);
        let wrapped = format!("NOT NOT ({})", case.expression);
        let actual = compile_filter(&wrapped)
            .unwrap_or_else(|error| panic!("case {}: {wrapped}: {error}", case.id))
            .matches(&case.fields)
            .unwrap();
        assert_eq!(
            Some(actual),
            case.native_matches,
            "case {}: {wrapped}",
            case.id
        );
    }
}

#[test]
fn frozen_native_split_field_publication_vectors() {
    let cases: Vec<SplitVector> =
        serde_json::from_str(include_str!("fixtures/parity-native/split-field.json")).unwrap();
    assert_eq!(cases.len(), 8);
    let mut mismatches = Vec::new();
    for case in cases {
        let mut state = ActionState {
            fields: Fields::from([
                (case.field.clone(), case.input_values),
                ("COMMENT".into(), vec!["keep\nopaque text".into()]),
            ]),
            context: Fields::new(),
        };
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::SplitField {
                field: case.field.clone(),
                separator: case.delimiter,
            },
        )
        .unwrap();
        assert!(effects.is_empty());
        assert_eq!(state.fields["COMMENT"], ["keep\nopaque text"]);
        if state.fields.contains_key(&case.field) != case.native_field_present
            || state.fields.get(&case.field) != case.native_output_values.as_ref()
        {
            mismatches.push(format!(
                "case {}: expected {:?}, got {:?}",
                case.id,
                case.native_output_values,
                state.fields.get(&case.field)
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
