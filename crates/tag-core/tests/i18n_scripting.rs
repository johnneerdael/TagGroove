use tag_core::{evaluate, render_export, ExportOptions, Fields};

#[test]
fn scripting_diagnostic_fixtures_execute_real_format_and_export_producers() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/scripting-diagnostics.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let error = match case["kind"].as_str().unwrap() {
            "format" => evaluate(input, &Fields::new()).unwrap_err(),
            "export" => render_export(input, &[], &ExportOptions::default()).unwrap_err(),
            kind => panic!("Unknown diagnostic fixture {kind}"),
        };
        assert_eq!(error, case["error"].as_str().unwrap(), "{input}");
    }
}
