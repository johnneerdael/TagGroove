use tag_core::{evaluate, render_export, ExportOptions, Fields};

#[test]
fn scripting_diagnostic_fixtures_execute_real_format_and_export_producers() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/scripting-diagnostics.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in cases.as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let error = match case["kind"].as_str().unwrap() {
            "format" => evaluate(input, &Fields::new()).err(),
            "export" => render_export(input, &[], &ExportOptions::default()).err(),
            kind => panic!("Unknown diagnostic fixture {kind}"),
        };
        let expected = case["error"].as_str().unwrap();
        if error.as_deref() != Some(expected) {
            failures.push(format!("{input}: expected {expected:?}, got {error:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
