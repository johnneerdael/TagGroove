use tag_core::{
    compile_filter, evaluate_with_context, render_export, CompiledTemplate, EvalContext,
    ExportOptions, ExportRow, Fields,
};

// 2001-02-03 04:05:06 UTC. All checks supply the clock, zone and locale.
const TIME: i64 = 981_173_106;

#[test]
fn current_date_placeholders_use_the_evaluation_clock_instead_of_stored_fields() {
    let fields = Fields::from([
        ("_DATE".into(), vec!["stale".into()]),
        ("_DATETIME".into(), vec!["stale".into()]),
    ]);
    let mut context = EvalContext::new(&fields);
    context.timestamp_seconds = Some(TIME);
    context.timezone_offset_seconds = Some(0);
    context.locale = "en-US".into();
    let date = evaluate_with_context("%_date%", &context).unwrap();
    let datetime = evaluate_with_context("%_datetime%", &context).unwrap();
    assert_eq!(date, "02/03/2001");
    assert_eq!(datetime, "02/03/2001 04:05:06 AM");
    assert_eq!(
        evaluate_with_context(
            "[$meta(_DATE)]|$meta_sep(_datetime,;)|$if(%_date%,yes,no)",
            &context
        )
        .unwrap(),
        format!("{date}|{datetime}|yes")
    );
    assert_eq!(fields["_DATE"], ["stale"]);
}

#[test]
fn clock_fields_follow_locale_and_timezone_across_midnight() {
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.timestamp_seconds = Some(TIME);
    context.timezone_offset_seconds = Some(-5 * 3600);
    context.locale = "de-DE".into();
    assert_eq!(
        evaluate_with_context("%_DATE%|%_DATETIME%", &context).unwrap(),
        "02.02.2001|02.02.2001 23:05:06"
    );
    assert_eq!(
        evaluate_with_context("$fmtDate('yyyy-MM-dd HH:mm:ss')", &context).unwrap(),
        "2001-02-02 23:05:06"
    );
}

#[test]
fn direct_and_scripted_filter_fields_use_the_same_clock_values() {
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.timestamp_seconds = Some(TIME);
    context.timezone_offset_seconds = Some(0);
    context.locale = "de-DE".into();
    for query in [
        "_DATE IS 03.02.2001",
        "_DATETIME IS 03.02.2001 04:05:06",
        "\"%_date%\" IS 03.02.2001",
        "_DATE PRESENT AND NOT _DATETIME MISSING",
    ] {
        assert!(
            compile_filter(query)
                .unwrap()
                .matches_with_context(&context)
                .unwrap(),
            "{query}"
        );
    }
    assert!(!compile_filter("_DATE IS yesterday")
        .unwrap()
        .matches_with_context(&context)
        .unwrap());
}

#[test]
fn custom_field_names_do_not_expand_into_a_different_ascii_name() {
    let fields = Fields::from([
        ("maß".into(), vec!["first".into()]),
        ("MASS".into(), vec!["second".into()]),
    ]);
    assert!(compile_filter("maß IS first")
        .unwrap()
        .matches(&fields)
        .unwrap());
    assert!(compile_filter("MASS IS second")
        .unwrap()
        .matches(&fields)
        .unwrap());
    assert_eq!(
        tag_core::evaluate("%maß%|%MAß%|%MASS%|$meta(maß)|$meta_sep(MASS,;)", &fields).unwrap(),
        "first|first|second|first|second"
    );
}

#[test]
fn export_clock_is_available_outside_and_inside_loops_without_becoming_tag_data() {
    let mut row = ExportRow::new(
        Fields::from([("TITLE".into(), vec!["Track".into()])]),
        "/music/test.flac",
        1.,
        100,
    );
    row.context_fields
        .insert("_DATE".into(), vec!["old snapshot".into()]);
    let options = ExportOptions {
        timestamp_seconds: Some(TIME),
        timezone_offset_seconds: Some(0),
        locale: Some("de-DE".into()),
        ..Default::default()
    };
    let result = render_export(
        "%_date%|$loop(1)%_datetime%:$meta(_date):$list('[',=,']')$loopend()",
        &[row],
        &options,
    )
    .unwrap();
    assert_eq!(
        result[0].text,
        "03.02.2001|03.02.2001 04:05:06:03.02.2001:[TITLE=Track]"
    );
}

#[test]
fn clock_values_work_in_filename_templates_and_invalid_clock_context_is_reported() {
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.timestamp_seconds = Some(TIME);
    context.timezone_offset_seconds = Some(0);
    context.locale = "de-DE".into();
    assert_eq!(
        CompiledTemplate::compile("%_datetime%")
            .unwrap()
            .evaluate_filename_with_context(&context)
            .unwrap(),
        "03.02.2001 04_05_06"
    );
    context.timestamp_seconds = Some(i64::MAX);
    assert!(evaluate_with_context("%_date%", &context).is_err());
    context.timestamp_seconds = Some(TIME);
    context.timezone_offset_seconds = Some(100_000);
    assert!(evaluate_with_context("%_datetime%", &context).is_err());
}
