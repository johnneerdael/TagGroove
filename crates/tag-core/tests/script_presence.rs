use std::cell::RefCell;
use tag_core::{
    CompiledTemplate, EvalContext, Fields, ScriptExtension, ScriptFunctionSpec, ScriptValue,
};

// Literal static native predictions from presence-guards/29-static-oracles.json.
// Public flags are the zero/nonzero projection; no original runtime is invoked.
#[test]
fn native_found_state_static_oracles() {
    let fields: Fields = [("TITLE".into(), vec!["Rock".into()])].into();
    let cases = [
        ("1", "1", false),
        ("'1'", "1", false),
        ("$if(1,T,F)", "F", false),
        ("$if(%TITLE%,T,F)", "T", true),
        ("$if2(ready,fallback)", "fallback", false),
        ("$eql(1,1)", "", true),
        ("[$eql(1,1)]", "", true),
        ("[X$eql(1,1)]", "X", true),
        ("$eql(1,2)", "", false),
        ("$if($eql(1,1),T,F)", "T", true),
        ("%TITLE%$not(%TITLE%)", "Rock", false),
        ("[%TITLE%$not(%TITLE%)]", "", false),
        ("%TITLE%$not(%MISSING%)", "Rock", true),
        ("$if(%TITLE%$not(%TITLE%),T,F)", "F", false),
        ("$if($if(%TITLE%,yes,no),T,F)", "T", true),
        ("$if($if(1,%TITLE%,no),T,F)", "T", true),
        ("$if2($if(%TITLE%,yes,no),fallback)", "yes", true),
        ("$if($ifgreater(3,2,yes,%TITLE%),T,F)", "T", true),
        ("$if($iflonger(abc,2,yes,%TITLE%),T,F)", "T", true),
        ("$and(%TITLE%,%TITLE%)", "", true),
        ("$or(%MISSING%,%TITLE%)", "", true),
        ("$not(1)", "", true),
        ("[$not(1)X]", "X", true),
        (
            "[$zz_unknown_3361(%TITLE%)]",
            "$zz_unknown_3361(Rock)",
            true,
        ),
        (
            "%TITLE%$zz_unknown_3361($not(%TITLE%))",
            "Rock$zz_unknown_3361()",
            false,
        ),
        ("[%TITLE%$zz_unknown_3361($not(%TITLE%))]", "", false),
        (
            "$zz_unknown_3361($zz_unknown_3361(%TITLE%))",
            "$zz_unknown_3361($zz_unknown_3361(Rock))",
            true,
        ),
    ];
    for (expression, text, present) in cases {
        let value = CompiledTemplate::compile(expression)
            .unwrap()
            .evaluate_value_with_context(&EvalContext::new(&fields))
            .unwrap();
        assert_eq!(
            (&*value.text, value.present, value.truth),
            (text, present, present),
            "{expression}"
        );
    }
}

struct Events {
    calls: RefCell<Vec<String>>,
    fail_at: Option<usize>,
}
impl ScriptExtension for Events {
    fn call(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String> {
        assert_eq!(name, "event");
        let mut calls = self.calls.borrow_mut();
        calls.push(args[0].text.clone());
        if self.fail_at == Some(calls.len()) {
            return Err("Format string output limit exceeded".into());
        }
        Ok(ScriptValue {
            text: format!("{}{}", args[0].text, calls.len()),
            present: calls.len() == 1,
            truth: calls.len() == 1,
        })
    }
}
fn event_template(expression: &str) -> CompiledTemplate {
    CompiledTemplate::compile_with_functions(
        expression,
        &[ScriptFunctionSpec {
            name: "event".into(),
            min_args: 1,
            max_args: 1,
        }],
    )
    .unwrap()
}
#[test]
fn unknown_replays_original_arguments_after_all_eager_calls() {
    let fields = Fields::new();
    let events = Events {
        calls: RefCell::new(Vec::new()),
        fail_at: None,
    };
    let mut context = EvalContext::new(&fields);
    context.extension = Some(&events);
    let value = event_template("[$zz_unknown_3361($event(a),$event(b))]")
        .evaluate_value_with_context(&context)
        .unwrap();
    assert_eq!(events.calls.borrow().as_slice(), ["a", "b", "a", "b"]);
    assert_eq!(value.text, "$zz_unknown_3361(a3,b4)");
    assert!(
        value.present && value.truth,
        "eager presence survives empty replay presence"
    );
}
#[test]
fn unknown_first_and_second_pass_failures_preserve_external_provenance_and_order() {
    for (fail_at, expected) in [(1, vec!["a"]), (3, vec!["a", "b", "a"])] {
        let fields = Fields::new();
        let events = Events {
            calls: RefCell::new(Vec::new()),
            fail_at: Some(fail_at),
        };
        let mut context = EvalContext::new(&fields);
        context.extension = Some(&events);
        let error = event_template("$zz_unknown_3361($event(a),$event(b))")
            .evaluate_with_context_diagnostic(&context)
            .unwrap_err();
        assert!(matches!(
            error,
            tag_core::batch_errors::ErrorCause::External { .. }
        ));
        assert_eq!(events.calls.borrow().as_slice(), expected);
    }
}
#[test]
fn unknown_literals_keep_original_spelling_and_never_reparse_field_output() {
    let fields: Fields = [("TITLE".into(), vec!["$div(1,0)".into()])].into();
    assert_eq!(
        CompiledTemplate::compile("$Zz_Unknown_3361(%TITLE%) $bare_unknown")
            .unwrap()
            .evaluate(&fields)
            .unwrap(),
        "$Zz_Unknown_3361($div(1,0)) $bare_unknown"
    );
    for source in [
        "$zz_unknown_3361('%TITLE%)",
        "$zz_unknown_3361(%TITLE)",
        "$zz_unknown_3361([x)",
        "$upper()",
        "$get(k)",
    ] {
        assert!(CompiledTemplate::compile(source).is_err(), "{source}");
    }
}
#[test]
fn unknown_filename_replay_preserves_literal_paths_and_sanitizes_field_components() {
    let fields: Fields = [("TITLE".into(), vec!["A/B".into()])].into();
    let context = EvalContext::new(&fields);
    let template = CompiledTemplate::compile("$zz_unknown_3361(dir/%TITLE%)").unwrap();
    assert_eq!(
        template.evaluate_with_context(&context).unwrap(),
        "$zz_unknown_3361(dir/A/B)"
    );
    assert_eq!(
        template.evaluate_filename_with_context(&context).unwrap(),
        "$zz_unknown_3361(dir/A_B)"
    );
    let expected = if cfg!(windows) {
        "$zz_unknown_3361(dir\\A_B)"
    } else {
        "$zz_unknown_3361(dir/A_B)"
    };
    assert_eq!(
        template
            .evaluate_filename_path_with_context(&context)
            .unwrap(),
        expected
    );
}
#[test]
fn unknown_work_limit_has_owned_provenance_at_the_approved_threshold() {
    let fields = Fields::new();
    let context = EvalContext::new(&fields);
    let nested = |depth| {
        format!(
            "{}x{}",
            "$zz_unknown_3361(".repeat(depth),
            ")".repeat(depth)
        )
    };
    assert!(CompiledTemplate::compile(&nested(19))
        .unwrap()
        .evaluate(&fields)
        .is_ok());
    let template = CompiledTemplate::compile(&nested(20)).unwrap();
    for error in [
        template
            .evaluate_with_context_diagnostic(&context)
            .unwrap_err(),
        template
            .evaluate_filename_with_context_diagnostic(&context)
            .unwrap_err(),
        template
            .evaluate_filename_path_with_context_diagnostic(&context)
            .unwrap_err(),
    ] {
        assert!(matches!(
            error,
            tag_core::batch_errors::ErrorCause::Owned { .. }
        ));
        assert_eq!(error.to_string(), "Format string work limit exceeded");
    }
}

#[test]
fn selectors_eagerly_execute_unused_arguments_and_keep_all_child_presence() {
    let fields = Fields::new();
    let events = Events {
        calls: RefCell::new(Vec::new()),
        fail_at: None,
    };
    let mut context = EvalContext::new(&fields);
    context.extension = Some(&events);
    let value = event_template("$if(1,$event(a),$event(b))")
        .evaluate_value_with_context(&context)
        .unwrap();
    assert_eq!(events.calls.borrow().as_slice(), ["a", "b"]);
    assert_eq!(
        (&*value.text, value.present, value.truth),
        ("b2", true, true)
    );
    let events = Events {
        calls: RefCell::new(Vec::new()),
        fail_at: Some(1),
    };
    context.extension = Some(&events);
    assert!(event_template("$if(1,$event(a),unused)")
        .evaluate_with_context(&context)
        .is_err());
    assert_eq!(events.calls.borrow().as_slice(), ["a"]);
}
#[test]
fn native_known_unsupported_names_are_reserved_without_unregistered_dispatch() {
    for name in [
        "fmtdatelocal",
        "get",
        "listtec",
        "put",
        "puts",
        "repl",
        "rg2sc",
        "substr",
    ] {
        let error = CompiledTemplate::compile(&format!("${name}(x)")).unwrap_err();
        assert_eq!(error, format!("Unsupported scripting function: ${name}"));
    }
}

#[test]
fn every_native_predicate_emits_no_text_and_replaces_previous_sibling_flags() {
    let fields: Fields = [("TITLE".into(), vec!["Rock".into()])].into();
    for (predicate, expected) in [
        ("$eql(a,a)", true),
        ("$neql(a,a)", false),
        ("$grtr(2,1)", true),
        ("$geql(1,2)", false),
        ("$less(1,2)", true),
        ("$leql(2,1)", false),
        ("$and(%TITLE%,%MISSING%)", false),
        ("$or(1,%TITLE%)", true),
        ("$not(%TITLE%)", false),
        ("$odd(3)", true),
        ("$isdigit(x)", false),
        ("$strcmp(a,A)", false),
        ("$stricmp(a,A)", true),
    ] {
        let value = CompiledTemplate::compile(&format!("%TITLE%{predicate}"))
            .unwrap()
            .evaluate_value_with_context(&EvalContext::new(&fields))
            .unwrap();
        assert_eq!(
            (&*value.text, value.present, value.truth),
            ("Rock", expected, expected),
            "{predicate}"
        );
    }
}

// Existing context-adapter contracts; their native callback found effects are
// deliberately outside the static native predicate/selector proof.
#[test]
fn review_boundary_getenv_keeps_empty_result_truth_separate_from_argument_presence() {
    let fields: Fields = [("KEY".into(), vec!["ABSENT_VAR".into()])].into();
    let environment = std::collections::BTreeMap::new();
    let mut context = EvalContext::new(&fields);
    context.environment = Some(&environment);
    let value = CompiledTemplate::compile("$getenv(%KEY%)")
        .unwrap()
        .evaluate_value_with_context(&context)
        .unwrap();
    assert_eq!(
        (&*value.text, value.present, value.truth),
        ("", true, false)
    );
    assert_eq!(
        CompiledTemplate::compile("$if($getenv(%KEY%),yes,no)")
            .unwrap()
            .evaluate_with_context(&context)
            .unwrap(),
        "no"
    );
}

#[test]
fn review_boundary_empty_columns_keep_result_truth_separate_from_argument_presence() {
    let fields: Fields = [("KEY".into(), vec!["prefix".into()])].into();
    let mut context = EvalContext::new(&fields);
    context.columns = Some(&[]);
    for expression in ["$columnnames(%KEY%,)", "$columnvalues(,%KEY%)"] {
        let value = CompiledTemplate::compile(expression)
            .unwrap()
            .evaluate_value_with_context(&context)
            .unwrap();
        assert_eq!(
            (&*value.text, value.present, value.truth),
            ("", true, false),
            "{expression}"
        );
        assert_eq!(
            CompiledTemplate::compile(&format!("$if({expression},yes,no)"))
                .unwrap()
                .evaluate_with_context(&context)
                .unwrap(),
            "no",
            "{expression}"
        );
    }
}

#[test]
fn review_boundary_context_callbacks_preserve_prior_enclosing_truth() {
    let fields: Fields = [("KEY".into(), vec!["ABSENT_VAR".into()])].into();
    let environment = std::collections::BTreeMap::new();
    let mut context = EvalContext::new(&fields);
    context.environment = Some(&environment);
    context.columns = Some(&[]);
    for callback in [
        "$getenv(%KEY%)",
        "$columnnames(%KEY%,)",
        "$columnvalues(,%KEY%)",
    ] {
        for (preceding, expected) in [("$eql(1,1)", true), ("$eql(1,2)", false)] {
            let value = CompiledTemplate::compile(&format!("{preceding}{callback}"))
                .unwrap()
                .evaluate_value_with_context(&context)
                .unwrap();
            assert_eq!(
                (&*value.text, value.present, value.truth),
                ("", true, expected),
                "{preceding}{callback}"
            );
        }
    }
}
