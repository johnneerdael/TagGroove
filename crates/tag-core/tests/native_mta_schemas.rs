use serde::Deserialize;
use std::collections::BTreeMap;
use tag_core::actions::{export_mta, export_mta_utf16, import_mta, ActionSpec};

#[derive(Deserialize)]
struct Oracle {
    id: String,
    decoded_record: BTreeMap<String, String>,
}

#[test]
fn native_cover_and_playlist_records_are_executable_without_losing_options() {
    let records: Vec<Oracle> =
        serde_json::from_str(include_str!("fixtures/parity-native/mta-schemas.json")).unwrap();
    let mut count = 0;
    let mut unresolved = Vec::new();
    for record in records {
        if !["17", "18", "19"].contains(&record.decoded_record["T"].as_str()) {
            continue;
        }
        count += 1;
        let mut text = String::from("[#0]\r\n");
        for (key, value) in &record.decoded_record {
            text.push_str(&format!("{key}={value}\r\n"));
        }
        let imported = import_mta(record.id.clone(), text.as_bytes(), None).unwrap();
        if !imported.issues.is_empty()
            || !imported.group.enabled
            || matches!(imported.group.actions[0], ActionSpec::UnresolvedMta { .. })
        {
            unresolved.push(format!("{}: {:?}", record.id, imported.issues));
            continue;
        }
        let utf8 = export_mta(&imported.group).unwrap();
        let utf16 = export_mta_utf16(&imported.group).unwrap();
        for bytes in [utf8.as_bytes(), utf16.as_slice()] {
            let reloaded = import_mta(record.id.clone(), bytes, None).unwrap();
            assert!(
                reloaded.issues.is_empty(),
                "{}: {:?}",
                record.id,
                reloaded.issues
            );
            assert_eq!(reloaded.group, imported.group, "{}", record.id);
        }
        // The known inactive native option must survive; it is not the legacy
        // type-filter slot and cannot be silently consumed or dropped.
        if record.id == "properties-renumbering-unsafe" {
            assert!(utf8.lines().any(|line| line == "3=3"));
            assert!(utf8.lines().any(|line| line == "1="));
        }
    }
    assert_eq!(count, 13);
    assert!(unresolved.is_empty(), "{}", unresolved.join("\n"));
}

fn one(text: &str) -> tag_core::actions::ActionGroup {
    let result = import_mta("Native".into(), text.as_bytes(), None).unwrap();
    assert!(result.issues.is_empty(), "{:?}", result.issues);
    result.group
}
#[test]
fn native_options_survive_edits_and_conflicting_legacy_fallback_is_rejected() {
    let mut group = one("[#0]\nT=18\n1=\n3=3\n");
    let ActionSpec::SetCoverProperties { description, .. } = &mut group.actions[0] else {
        panic!()
    };
    *description = Some("Caption".into());
    let text = export_mta(&group).unwrap();
    assert!(text.lines().any(|line| line == "1="));
    assert!(text.lines().any(|line| line == "3=3"));
    assert!(text.lines().any(|line| line == "2=Caption"));
    let ActionSpec::SetCoverProperties { description, .. } = &mut group.actions[0] else {
        panic!()
    };
    *description = Some("-1".into());
    assert!(export_mta(&group)
        .unwrap_err()
        .contains("meanings conflict"));
    assert!(export_mta_utf16(&group)
        .unwrap_err()
        .contains("meanings conflict"));
    let mut adjust = one("[#0]\nT=17\n1=1\n2=0|scaleProportionally|max|disabled\n3=10\n");
    let ActionSpec::AdjustCover {
        format, quality, ..
    } = &mut adjust.actions[0]
    else {
        panic!()
    };
    assert_eq!(*quality, 80);
    *format = tag_core::actions::CoverImageFormat::Png;
    let text = export_mta(&adjust).unwrap();
    assert!(text.lines().any(|line| line == "1=2"));
    assert!(text.lines().any(|line| line == "3=10"));
}
#[test]
fn unknown_native_options_and_ambiguous_numeric_spellings_stay_opaque() {
    for record in [
        "T=17\nF=unqualified\n1=1\n2=600|scaleProportionally|max|enabled\n3=90",
        "T=17\n1=3\n2=600|scaleProportionally|max|enabled\n3=90",
        "T=17\n1=1\n2=600|unknown|max|enabled\n3=90",
        "T=18\n1=3suffix",
        "T=18\n2=-1suffix",
        "T=18\n2= -1",
        "T=19\nF=list.m3u\n2=extra",
    ] {
        let text = format!("[#0]\n{record}\n");
        let imported = import_mta("Opaque".into(), text.as_bytes(), None).unwrap();
        assert!(!imported.group.enabled);
        assert_eq!(imported.issues.len(), 1);
        for bytes in [
            export_mta(&imported.group).unwrap().into_bytes(),
            export_mta_utf16(&imported.group).unwrap(),
        ] {
            let again = import_mta("Opaque".into(), &bytes, None).unwrap();
            assert_eq!(again.group, imported.group);
        }
    }
}
#[test]
fn native_quality_and_size_boundaries_do_not_wrap_or_become_legacy_validation_errors() {
    for quality in ["19", "20", "100", "101", "256", "-1"] {
        let group = one(&format!(
            "[#0]\nT=17\n1=1\n2=10|scaleProportionally|max|enabled\n3={quality}\n"
        ));
        let ActionSpec::AdjustCover {
            quality: actual, ..
        } = &group.actions[0]
        else {
            panic!()
        };
        assert_eq!(
            *actual,
            match quality {
                "20" => 20,
                "100" => 100,
                _ => 80,
            }
        );
    }
    for pixels in [0, 9, 10, 10000, 10001] {
        let group = one(&format!(
            "[#0]\nT=17\n1=1\n2={pixels}|scaleProportionally|max|enabled\n3=80\n"
        ));
        let ActionSpec::AdjustCover {
            resize: Some(resize),
            ..
        } = &group.actions[0]
        else {
            panic!()
        };
        assert_eq!(
            resize.native_preflight_skips_adjustment(),
            !(10..=10000).contains(&pixels)
        );
    }
}
#[test]
fn legacy_json_and_extension_readers_keep_their_geometry_and_truthful_fallbacks() {
    use tag_core::actions::mta_export_report;
    for (quality, pixels, shape) in [
        (19, 600, "proportional"),
        (90, 9, "proportional"),
        (90, 10001, "proportional"),
        (90, 600, "square"),
        (90, 600, "crop_square"),
    ] {
        let value = serde_json::json!({"type":"adjust_cover","format":"original","quality":quality,
            "resize":{"pixels":pixels,"size_mode":"maximum","scale_mode":shape}});
        let action: ActionSpec = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&action).unwrap(), value);
        let group = tag_core::actions::ActionGroup {
            name: "Legacy".into(),
            enabled: true,
            actions: vec![action],
        };
        let text = export_mta(&group).unwrap();
        assert!(text.lines().any(|line| line == "T=1001"));
        assert_eq!(
            mta_export_report(&group).unwrap().legacy_actions,
            ["Adjust Cover"]
        );
        let again = import_mta("Legacy".into(), text.as_bytes(), None).unwrap();
        assert_eq!(again.group, group);
    }
    for description in ["-1", "-1suffix", "+1", " -01"] {
        let group = tag_core::actions::ActionGroup {
            name: "Legacy".into(),
            enabled: true,
            actions: vec![ActionSpec::SetCoverProperties {
                picture_type: None,
                description: Some(description.into()),
                only_picture_type: None,
                mta_source: None,
            }],
        };
        let text = export_mta(&group).unwrap();
        assert!(text.lines().any(|line| line == "T=1002"));
        assert_eq!(
            import_mta("Legacy".into(), text.as_bytes(), None)
                .unwrap()
                .group,
            group
        );
    }
    assert!(
        mta_export_report(&one("[#0]\nT=19\nF=list.m3u\n1=%album%\n"))
            .unwrap()
            .legacy_actions
            .is_empty()
    );
}

#[test]
fn native_policy_canonical_numeric_descriptions_keep_native_options() {
    let group = one("[#0]\nT=18\n1=-1\n2=0\n3=kept\n");
    let text = export_mta(&group).unwrap();
    assert!(text.lines().any(|line| line == "T=18"));
    assert!(text.lines().any(|line| line == "3=kept"));
}
#[test]
fn native_policy_invalid_resize_hybrids_are_rejected() {
    let action: ActionSpec = serde_json::from_value(serde_json::json!({
        "type":"adjust_cover","format":"original","quality":90,
        "resize":{"pixels":600,"size_mode":"maximum","scale_mode":"proportional","size_enabled":false}
    })).unwrap();
    assert!(tag_core::actions::validate_action(&action).is_err());
}
