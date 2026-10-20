//! Independently authored semantic vectors from the pinned Windows static
//! action-reader audit, not original-generated .mta files or runtime oracles.
use serde_json::{json, Value};
use tag_core::actions::{
    export_mta, export_mta_utf16, import_mta, import_mta_with_dialect, ActionGroup, ActionSpec,
    MtaDialect,
};

fn import(record: &str) -> tag_core::actions::MtaImport {
    import_mta(
        "Interchange".into(),
        format!("[#0]\n{record}\n").as_bytes(),
        None,
    )
    .unwrap()
}

fn semantic(action: &ActionSpec) -> Value {
    let mut value = serde_json::to_value(action).unwrap();
    value.as_object_mut().unwrap().remove("mta_source");
    value
}

fn canonical(record: &str, expected: Value) -> ActionGroup {
    let imported = import_mta_with_dialect(
        "Interchange".into(),
        format!("[#0]\n{record}\n").as_bytes(),
        None,
        MtaDialect::Mp3tag,
    )
    .unwrap();
    assert!(
        imported.issues.is_empty(),
        "{record}: {:?}",
        imported.issues
    );
    assert!(imported.group.enabled);
    assert_eq!(semantic(&imported.group.actions[0]), expected, "{record}");
    for wire in [
        export_mta(&imported.group).unwrap().into_bytes(),
        export_mta_utf16(&imported.group).unwrap(),
    ] {
        let again =
            import_mta_with_dialect("Interchange".into(), &wire, None, MtaDialect::Mp3tag).unwrap();
        assert_eq!(again.group, imported.group, "{record}");
        assert_eq!(
            again.records[0].properties, imported.records[0].properties,
            "{record}"
        );
    }
    imported.group
}

#[test]
fn native_case_modes_are_bit_flags_not_dialog_indices() {
    for (number, mode) in [(1, "mixed"), (2, "upper"), (4, "lower"), (8, "sentence")] {
        let group = canonical(
            &format!("T=1\nF=TITLE\n1={number}\n2= -"),
            json!({
                "type":"case_conversion", "field":"TITLE", "mode":mode, "word_separators":" -"
            }),
        );
        assert!(export_mta(&group)
            .unwrap()
            .lines()
            .any(|line| line == format!("1={number}")));
    }
}

#[test]
fn native_import_cover_uses_filename_one_and_optional_type_three() {
    for (suffix, picture, delete) in [
        ("", Value::Null, false),
        ("\n2=1\n3=4", json!("CoverBack"), true),
        ("\n2=0\n3=", json!("Other"), false),
    ] {
        canonical(
            &format!("T=12\n1=4{suffix}"),
            json!({
                "type":"import_cover", "filename_template":"4", "picture_type":picture, "delete_existing":delete
            }),
        );
    }
}

#[test]
fn native_export_cover_preserves_disabled_filter_when_filename_is_edited() {
    let mut group = canonical(
        "T=13\n1=%album%\n2=1\n3=0|4",
        json!({
            "type":"export_cover", "filename_template":"%album%", "picture_type":null, "export_duplicates":true
        }),
    );
    let ActionSpec::ExportCover {
        filename_template, ..
    } = &mut group.actions[0]
    else {
        panic!()
    };
    *filename_template = "new".into();
    let wire = export_mta(&group).unwrap();
    assert!(wire.lines().any(|line| line == "3=0|4"));
    assert!(wire.lines().any(|line| line == "1=new"));
    canonical(
        "T=13\n1=cover\n2=0\n3=1|3",
        json!({
            "type":"export_cover", "filename_template":"cover", "picture_type":"CoverFront", "export_duplicates":false
        }),
    );
    canonical(
        "T=13\n1=cover",
        json!({
            "type":"export_cover", "filename_template":"cover", "picture_type":null, "export_duplicates":false
        }),
    );
}

#[test]
fn native_export_partition_uses_flag_and_expression_remainder() {
    for (tuple, partition) in [
        ("0|%album%", Value::Null),
        ("1|%album%", json!("%album%")),
        ("1|", json!("%_folderpath%")),
        ("1|$if(%album%,a|b,c)", json!("$if(%album%,a|b,c)")),
    ] {
        let mut group = canonical(
            &format!("T=15\nF=Text\n1=out.txt\n2=0\n3={tuple}"),
            json!({
                "type":"export", "template":"Text", "filename_template":"out.txt", "append":false, "partition_template":partition
            }),
        );
        let ActionSpec::Export { append, .. } = &mut group.actions[0] else {
            panic!()
        };
        *append = true;
        let wire = export_mta(&group).unwrap();
        assert!(
            wire.lines().any(|line| line == format!("3={tuple}")),
            "{wire}"
        );
        assert!(wire.lines().any(|line| line == "2=1"));
    }
}

#[test]
fn native_codepage_is_resolved_from_lcid_and_old_json_stays_a_codepage() {
    for (lcid, cp) in [
        (1033, "1252"),
        (1049, "1251"),
        (1041, "932"),
        (1043, "1252"),
    ] {
        canonical(
            &format!("T=11\nF=TITLE\n1={lcid}"),
            json!({
                "type":"convert_codepage", "field":"TITLE", "codepage":cp
            }),
        );
    }
    let action: ActionSpec = serde_json::from_value(
        json!({"type":"convert_codepage","field":"TITLE","codepage":"1251"}),
    )
    .unwrap();
    assert_eq!(semantic(&action)["codepage"], "1251");
    let group = ActionGroup {
        name: "raw".into(),
        enabled: true,
        actions: vec![action],
    };
    let wire = export_mta(&group).unwrap();
    assert!(
        wire.lines().any(|line| line == "T=1004"),
        "raw codepage must not be emitted as native LCID: {wire}"
    );
    assert_eq!(
        semantic(
            &import_mta("raw".into(), wire.as_bytes(), None)
                .unwrap()
                .group
                .actions[0]
        ),
        semantic(&group.actions[0])
    );
}

#[test]
fn unknown_flags_and_conflicting_native_legacy_layouts_stay_opaque() {
    for record in [
        "T=1\nF=TITLE\n1=0",
        "T=1\nF=TITLE\n1=3",
        "T=1\nF=TITLE\n1=7",
        "T=1\nF=TITLE\n1=15",
        "T=11\nF=TITLE\n1=1251",
        "T=11\nF=TITLE\n1=65001",
        "T=11\nF=TITLE\n1=4096",
        "T=12\nF=old.jpg\n1=3\n3=4",
        "T=12\n1=cover\n3=21",
        "T=13\nF=old\n1=3\n3=1|4",
        "T=13\n1=cover\n3=2|3",
        "T=13\n1=cover\n3=1|3|4",
        "T=15\nF=Text\n1=out\n3=2|%album%",
    ] {
        let imported = import(record);
        assert_eq!(imported.issues.len(), 1, "{record}");
        assert!(!imported.group.enabled, "{record}");
        assert!(tag_core::actions::validate_action(&imported.group.actions[0]).is_err());
        for wire in [
            export_mta(&imported.group).unwrap().into_bytes(),
            export_mta_utf16(&imported.group).unwrap(),
        ] {
            let again = import_mta("Interchange".into(), &wire, None).unwrap();
            assert_eq!(
                again.records[0].properties, imported.records[0].properties,
                "{record}"
            );
            assert_eq!(again.group, imported.group, "{record}");
        }
    }
}

#[test]
fn unmarked_colliding_native_and_old_port_options_require_an_origin_choice() {
    for record in [
        "T=1\nF=TITLE\n1=2",
        "T=12\nF=old.jpg\n1=3\n2=0",
        "T=13\nF=old\n1=3\n2=0",
        "T=15\nF=Text\n1=out\n3=1|%album%",
        "T=15\nF=Text\n1=out\n3=%album%",
    ] {
        let imported = import(record);
        assert!(
            !imported.group.enabled,
            "unmarked collision was guessed: {record}"
        );
        assert_eq!(imported.issues.len(), 1, "{record}");
        for wire in [
            export_mta(&imported.group).unwrap().into_bytes(),
            export_mta_utf16(&imported.group).unwrap(),
        ] {
            let again = import_mta("Interchange".into(), &wire, None).unwrap();
            assert_eq!(again.group, imported.group);
            assert_eq!(again.records, imported.records);
        }
    }
}

#[test]
fn explicit_older_taggroove_origin_preserves_former_semantics_and_exports_safely() {
    for (record, expected) in [
        (
            "T=1\nF=TITLE\n1=2\n2=",
            json!({"type":"case_conversion","field":"TITLE","mode":"sentence","word_separators":""}),
        ),
        (
            "T=11\nF=TITLE\n1=1251",
            json!({"type":"convert_codepage","field":"TITLE","codepage":"1251"}),
        ),
        (
            "T=12\nF=cover.jpg\n1=4\n2=1",
            json!({"type":"import_cover","filename_template":"cover.jpg","picture_type":"CoverBack","delete_existing":true}),
        ),
        (
            "T=13\nF=cover\n1=3\n2=1",
            json!({"type":"export_cover","filename_template":"cover","picture_type":"CoverFront","export_duplicates":true}),
        ),
        (
            "T=15\nF=Text\n1=out\n2=0\n3=1|%album%",
            json!({"type":"export","template":"Text","filename_template":"out","append":false,"partition_template":"1|%album%"}),
        ),
    ] {
        let imported = import_mta_with_dialect(
            "old".into(),
            format!("[#0]\n{record}\n").as_bytes(),
            None,
            MtaDialect::LegacyTaggroove,
        )
        .unwrap();
        assert!(
            imported.issues.is_empty(),
            "{record}: {:?}",
            imported.issues
        );
        assert_eq!(semantic(&imported.group.actions[0]), expected);
        // New output is declared current/native, and never silently interpreted
        // through the old-port numeric meanings again.
        for wire in [
            export_mta(&imported.group).unwrap().into_bytes(),
            export_mta_utf16(&imported.group).unwrap(),
        ] {
            let again =
                import_mta_with_dialect("new".into(), &wire, None, MtaDialect::Mp3tag).unwrap();
            assert!(again.issues.is_empty(), "{:?}", again.issues);
            assert_eq!(semantic(&again.group.actions[0]), expected);
        }
    }
}

#[test]
fn every_recognized_schema_preserves_unknown_options_and_refuses_execution() {
    let records = [
        "T=1\nF=TITLE\n1=2\n2= ",
        "T=2\nF=TITLE\n1=a\n2=b\n3=0|0",
        "T=4\nF=TITLE\n1=a\n2=b\n3=0",
        "T=5\nF=TITLE\n1=%album%",
        "T=6\nF=ARTIST\n1=0",
        "T=7\nF=%title%\n1=%artist%",
        "T=8\nF=ARTIST\n1=; ",
        "T=9\nF=COMMENT",
        "T=10\nF=TITLE;ARTIST",
        "T=11\nF=TITLE\n1=1033",
        "T=12\n1=cover.jpg\n2=0\n3=3",
        "T=13\n1=cover\n2=0\n3=0|3",
        "T=14\nF=LYRICS\n1=lyrics.txt",
        "T=15\nF=Text\n1=out\n2=0\n3=1|%album%",
        "T=16\nF=ARTIST\n1=;",
        "T=17\n1=1\n2=600|scaleProportionally|max|enabled\n3=90",
        "T=18\nF=-1\n1=3\n2=Caption",
        "T=19\nF=list.m3u\n1=%album%",
        "T=1001\n1=1\n2=85\n3=600\n4=0\n5=0",
        "T=1002\n1=3\n2=Caption\n3=",
        "T=1003\nF=list.m3u\n1=%album%",
        "T=1004\nF=TITLE\n1=850",
        "T=1005\nF=Text\n1=out\n2=0\n3=",
    ];
    assert_eq!(
        records.len(),
        tag_core::actions::MTA_TYPE_REGISTRY.len() + tag_core::actions::MTA_OPENTAG_TYPES.len()
    );
    let read = |text: &str| {
        import_mta_with_dialect("matrix".into(), text.as_bytes(), None, MtaDialect::Mp3tag).unwrap()
    };
    for record in records {
        let valid = read(&format!("[#0]\n{record}\n"));
        assert!(valid.issues.is_empty(), "{record}: {:?}", valid.issues);
        for key in ["99", "FutureOption"] {
            let unknown = read(&format!(
                "[#0]\n{record}\n{key}=retained\\x0aoption\\\\literal\n"
            ));
            assert_eq!(unknown.issues.len(), 1, "{record}: {key}");
            assert!(!unknown.group.enabled);
            assert!(tag_core::actions::validate_action(&unknown.group.actions[0]).is_err());
            for wire in [
                export_mta(&unknown.group).unwrap().into_bytes(),
                export_mta_utf16(&unknown.group).unwrap(),
            ] {
                let again =
                    import_mta_with_dialect("matrix".into(), &wire, None, MtaDialect::Mp3tag)
                        .unwrap();
                assert_eq!(again.records, unknown.records);
                assert_eq!(again.group, unknown.group);
            }
        }
    }
}

#[test]
fn retained_locale_survives_field_edits_but_raw_codepage_and_empty_partition_warn() {
    use tag_core::actions::mta_export_report;
    let mut group = canonical(
        "T=11\nF=TITLE\n1=1033",
        json!({"type":"convert_codepage","field":"TITLE","codepage":"1252"}),
    );
    let ActionSpec::ConvertCodepage { field, .. } = &mut group.actions[0] else {
        panic!()
    };
    *field = "ALBUM".into();
    let native = export_mta(&group).unwrap();
    assert!(native.lines().any(|line| line == "T=11"));
    assert!(native.lines().any(|line| line == "F=ALBUM"));
    assert!(native.lines().any(|line| line == "1=1033"));
    assert!(mta_export_report(&group).unwrap().legacy_actions.is_empty());
    let ActionSpec::ConvertCodepage { codepage, .. } = &mut group.actions[0] else {
        panic!()
    };
    *codepage = "1251".into();
    let wire = export_mta(&group).unwrap();
    assert!(wire.lines().any(|line| line == "T=1004"));
    assert!(wire.lines().any(|line| line == "1=1251"));
    assert_eq!(
        mta_export_report(&group).unwrap().legacy_actions,
        ["Convert Codepage"]
    );
    let again = import_mta("raw".into(), wire.as_bytes(), None).unwrap();
    assert_eq!(
        semantic(&again.group.actions[0]),
        semantic(&group.actions[0])
    );

    let empty:ActionSpec=serde_json::from_value(json!({"type":"export","template":"Text","filename_template":"out","append":false,"partition_template":""})).unwrap();
    group.actions = vec![empty];
    assert_eq!(
        mta_export_report(&group).unwrap().legacy_actions,
        ["Export"]
    );
    for wire in [
        export_mta(&group).unwrap().into_bytes(),
        export_mta_utf16(&group).unwrap(),
    ] {
        let again = import_mta("empty".into(), &wire, None).unwrap();
        assert!(again.issues.is_empty());
        assert_eq!(
            semantic(&again.group.actions[0]),
            semantic(&group.actions[0])
        );
    }
}
