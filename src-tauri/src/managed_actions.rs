use tag_core::actions::{guess_value_targets, validate_action, ActionGroup, ActionSpec};

fn validate_selector(field: &str, transforms_filename: bool) -> Result<(), String> {
    if field.eq_ignore_ascii_case("_FILENAME")
        || field.eq_ignore_ascii_case("_DIRECTORY")
        || (transforms_filename && field.eq_ignore_ascii_case("_ALL"))
    {
        return Err(
            "Filename and directory actions are not available for selected documents yet".into(),
        );
    }
    let other_metadata_selector = if transforms_filename {
        "_FIELDNAME"
    } else {
        "_ALL"
    };
    if field.starts_with('_')
        && !field.eq_ignore_ascii_case("_TAG")
        && !field.eq_ignore_ascii_case(other_metadata_selector)
    {
        return Err(format!("Unsupported field selector {field}"));
    }
    Ok(())
}

fn validate_in_place(action: &ActionSpec) -> Result<(), String> {
    validate_action(action)?;
    match action {
        ActionSpec::Export { .. }
        | ActionSpec::ExportCover { .. }
        | ActionSpec::GeneratePlaylists { .. }
        | ActionSpec::ImportCover { .. }
        | ActionSpec::ImportTextFile { .. } => {
            Err("This action needs file inputs or outputs that are not available for selected documents yet".into())
        }
        ActionSpec::CaseConversion { field, .. }
        | ActionSpec::ConvertCodepage { field, .. }
        | ActionSpec::Replace { field, .. }
        | ActionSpec::RegexReplace { field, .. } => validate_selector(field, true),
        ActionSpec::MergeDuplicateFields { field, .. }
        | ActionSpec::RemoveDuplicateFields { field, .. }
        | ActionSpec::SplitField { field, .. } => validate_selector(field, false),
        ActionSpec::FormatValue { field, .. } => {
            if field.starts_with('_') {
                Err(format!("Format Value target {field} is not an available tag field for selected documents"))
            } else {
                Ok(())
            }
        }
        ActionSpec::GuessValues { pattern, .. } => {
            for field in guess_value_targets(pattern)? {
                if field.starts_with('_') {
                    return Err(format!("Guess Values target {field} is not an available tag field for selected documents"));
                }
            }
            Ok(())
        }
        ActionSpec::AdjustCover { .. }
        | ActionSpec::SetCoverProperties { .. }
        | ActionSpec::RemoveFields { .. }
        | ActionSpec::RemoveFieldsExcept { .. } => Ok(()),
        ActionSpec::UnresolvedMta { .. } => unreachable!("validated above"),
    }
}

pub(crate) fn validate_groups(groups: &[ActionGroup]) -> Result<(), String> {
    for group in groups.iter().filter(|group| group.enabled) {
        for (index, action) in group.actions.iter().enumerate() {
            validate_in_place(action)
                .map_err(|error| format!("{} / action {}: {error}", group.name, index + 1))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn group(actions: Vec<Value>) -> ActionGroup {
        ActionGroup {
            name: "Tag cleanup".into(),
            enabled: true,
            actions: actions
                .into_iter()
                .map(|value| serde_json::from_value(value).unwrap())
                .collect(),
        }
    }

    #[test]
    fn accepts_metadata_and_embedded_artwork_actions() {
        let actions = vec![
            json!({"type":"adjust_cover","format":"png","quality":80,"resize":null}),
            json!({"type":"case_conversion","field":"TITLE","mode":"upper","word_separators":""}),
            json!({"type":"convert_codepage","field":"TITLE","codepage":"1252"}),
            json!({"type":"format_value","field":"ALBUM","template":"%ARTIST%"}),
            json!({"type":"guess_values","source":"%TITLE%","pattern":"%ARTIST% - %TITLE%"}),
            json!({"type":"merge_duplicate_fields","field":"ARTIST","separator":"; "}),
            json!({"type":"remove_duplicate_fields","field":"ARTIST","only_same_content":false}),
            json!({"type":"remove_fields","fields":["COMMENT","PICTURE"]}),
            json!({"type":"remove_fields_except","fields":["TITLE"]}),
            json!({"type":"replace","field":"TITLE","from":"old","to":"new","whole_word":false,"case_sensitive":false}),
            json!({"type":"regex_replace","field":"TITLE","pattern":"old","replacement":"new","case_sensitive":false}),
            json!({"type":"set_cover_properties","picture_type":"CoverFront","description":"Front","only_picture_type":null}),
            json!({"type":"split_field","field":"ARTIST","separator":";"}),
        ];
        validate_groups(&[group(actions)]).unwrap();
    }

    #[test]
    fn unavailable_external_actions_identify_group_and_position() {
        let unavailable = [
            json!({"type":"export","template":"%TITLE%","filename_template":"tags.txt","append":false,"partition_template":null}),
            json!({"type":"export_cover","filename_template":"front.png","picture_type":null,"export_duplicates":false}),
            json!({"type":"generate_playlists","filename_template":"tracks.m3u8","partition_template":""}),
            json!({"type":"import_cover","filename_template":"front.png"}),
            json!({"type":"import_text_file","field":"COMMENT","filename_template":"notes.txt"}),
            json!({"type":"unresolved_mta","type_id":"999","properties":{},"reason":"Unknown action settings"}),
        ];
        for action in unavailable {
            let error = validate_groups(&[group(vec![
                json!({"type":"format_value","field":"TITLE","template":"new"}),
                action,
            ])])
            .unwrap_err();
            assert!(error.contains("Tag cleanup"), "{error}");
            assert!(error.contains("action 2"), "{error}");
        }
    }

    #[test]
    fn disabled_unavailable_group_does_not_block_tag_actions() {
        let mut disabled = group(vec![
            json!({"type":"import_cover","filename_template":"front.png"}),
        ]);
        disabled.enabled = false;
        validate_groups(&[
            disabled,
            group(vec![
                json!({"type":"format_value","field":"TITLE","template":"new"}),
            ]),
        ])
        .unwrap();
    }

    #[test]
    fn transform_all_is_unavailable_but_metadata_all_is_supported() {
        let transforms = [
            json!({"type":"case_conversion","field":"_ALL","mode":"upper","word_separators":""}),
            json!({"type":"convert_codepage","field":"_ALL","codepage":"1252"}),
            json!({"type":"replace","field":"_ALL","from":"old","to":"new","whole_word":false,"case_sensitive":false}),
            json!({"type":"regex_replace","field":"_ALL","pattern":"old","replacement":"new","case_sensitive":false}),
        ];
        for action in transforms {
            assert!(validate_groups(&[group(vec![action])]).is_err());
        }
        validate_groups(&[group(vec![
            json!({"type":"merge_duplicate_fields","field":"_ALL","separator":"; "}),
            json!({"type":"remove_duplicate_fields","field":"_ALL","only_same_content":false}),
            json!({"type":"split_field","field":"_ALL","separator":";"}),
        ])])
        .unwrap();
    }

    #[test]
    fn tag_and_field_name_transforms_remain_available() {
        for field in ["_TAG", "_FIELDNAME"] {
            validate_groups(&[group(vec![json!({"type":"replace","field":field,"from":"old","to":"new","whole_word":false,"case_sensitive":false})])]).unwrap();
        }
    }

    #[test]
    fn filename_and_directory_targets_are_not_tag_only_settings() {
        for field in ["_FILENAME", "_DIRECTORY", "_filename"] {
            assert!(validate_groups(&[group(vec![
                json!({"type":"format_value","field":field,"template":"new"})
            ])])
            .is_err());
            assert!(validate_groups(&[group(vec![json!({"type":"replace","field":field,"from":"old","to":"new","whole_word":false,"case_sensitive":false})])]).is_err());
        }
        for pattern in ["%TITLE% - %_FILENAME%", "%_DIRECTORY%", "%_ALL%"] {
            assert!(validate_groups(&[group(vec![
                json!({"type":"guess_values","source":"%TITLE%","pattern":pattern})
            ])])
            .is_err());
        }
    }

    #[test]
    fn malformed_patterns_and_unknown_selectors_are_rejected_before_execution() {
        for action in [
            json!({"type":"guess_values","source":"%TITLE%","pattern":"%ARTIST%%TITLE%"}),
            json!({"type":"regex_replace","field":"TITLE","pattern":"(","replacement":"x","case_sensitive":false}),
            json!({"type":"format_value","field":"_TAG","template":"x"}),
            json!({"type":"split_field","field":"_FIELDNAME","separator":";"}),
        ] {
            assert!(validate_groups(&[group(vec![action])]).is_err());
        }
    }

    #[test]
    fn unsupported_later_group_is_validated_with_its_own_name() {
        let first = group(vec![
            json!({"type":"format_value","field":"TITLE","template":"new"}),
        ]);
        let mut later = group(vec![
            json!({"type":"import_cover","filename_template":"front.png"}),
        ]);
        later.name = "Later artwork".into();
        let error = validate_groups(&[first, later]).unwrap_err();
        assert!(error.contains("Later artwork"), "{error}");
        assert!(error.contains("action 1"), "{error}");
    }
}
