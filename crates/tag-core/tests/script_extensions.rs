use std::{cell::RefCell, collections::BTreeMap};
use tag_core::{
    CompiledTemplate, EvalContext, Fields, ScriptExtension, ScriptFunctionSpec, ScriptValue,
};

struct Extension {
    variables: RefCell<BTreeMap<String, ScriptValue>>,
    counter: RefCell<usize>,
    calls: RefCell<Vec<String>>,
}
impl ScriptExtension for Extension {
    fn call(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String> {
        self.calls.borrow_mut().push(name.into());
        match name {
            "store" => {
                self.variables
                    .borrow_mut()
                    .insert("value".into(), args[0].clone());
                Ok(args[0].clone())
            }
            "fetch" => Ok(self
                .variables
                .borrow()
                .get("value")
                .cloned()
                .unwrap_or_default()),
            "advance" => {
                *self.counter.borrow_mut() += 1;
                Ok(ScriptValue::default())
            }
            "big" => Ok(ScriptValue::literal("x".repeat(1_048_580))),
            _ => Err("Unknown extension function".into()),
        }
    }
    fn field(&self, name: &str) -> Result<Option<Vec<String>>, String> {
        Ok(name
            .eq_ignore_ascii_case("_counter")
            .then(|| vec![self.counter.borrow().to_string()]))
    }
}
fn extension() -> Extension {
    Extension {
        variables: RefCell::new(BTreeMap::new()),
        counter: RefCell::new(1),
        calls: RefCell::new(Vec::new()),
    }
}
fn functions() -> Vec<ScriptFunctionSpec> {
    [
        ("store", 1, 1),
        ("fetch", 0, 0),
        ("advance", 0, 0),
        ("big", 0, 0),
    ]
    .into_iter()
    .map(|(name, min_args, max_args)| ScriptFunctionSpec {
        name: name.into(),
        min_args,
        max_args,
    })
    .collect()
}

#[test]
fn extensions_are_opt_in_and_keep_typed_boolean_and_presence_flags() {
    let f: Fields = [("ARTIST".into(), vec!["A".into()])].into();
    let mut context = EvalContext::new(&f);
    let extension = extension();
    context.extension = Some(&extension);
    let compile = |text| CompiledTemplate::compile_with_functions(text, &functions()).unwrap();
    assert_eq!(
        compile("$store($eql(a,b))$if($fetch(),yes,no)")
            .evaluate_with_context(&context)
            .unwrap(),
        "no"
    );
    let stored = extension.variables.borrow()["value"].clone();
    assert_eq!(
        (&*stored.text, stored.present, stored.truth),
        ("", false, false)
    );
    assert_eq!(
        compile("$store(%artist%)[x$fetch()]")
            .evaluate_with_context(&context)
            .unwrap(),
        "AxA"
    );
    assert_eq!(
        compile("$store('X')[x$fetch()]")
            .evaluate_with_context(&context)
            .unwrap(),
        "X"
    );
    assert!(compile("$fetch()").evaluate(&f).is_err());
    assert!(CompiledTemplate::compile_with_functions("$store()", &functions()).is_err());
}

#[test]
fn field_callbacks_capture_counter_values_in_actual_evaluation_order_and_meta() {
    let f: Fields = [("_COUNTER".into(), vec!["999".into()])].into();
    let mut context = EvalContext::new(&f);
    let extension = extension();
    context.extension = Some(&extension);
    let template = CompiledTemplate::compile_with_functions(
        "%_counter%$advance()%_counter%:$meta(_counter)",
        &functions(),
    )
    .unwrap();
    assert_eq!(template.evaluate_with_context(&context).unwrap(), "12:2");
}

#[test]
fn export_context_can_set_a_larger_output_limit_without_weakening_defaults() {
    let f = Fields::new();
    let mut context = EvalContext::new(&f);
    let extension = extension();
    context.extension = Some(&extension);
    let template =
        CompiledTemplate::compile_with_functions("$replace($big(),x,y)", &functions()).unwrap();
    assert!(template.evaluate_with_context(&context).is_err());
    context.max_output_bytes = 2_097_152;
    let text = template.evaluate_with_context(&context).unwrap();
    assert_eq!(text.len(), 1_048_580);
    assert!(text.bytes().all(|c| c == b'y'));
}

// These are direct PORT host-contract records, not native callback predictions.
#[test]
fn direct_host_flags_remain_independent_of_text_and_each_other() {
    for (text, present, truth) in [
        ("visible", false, true),
        ("visible", true, false),
        ("", true, false),
        ("", false, true),
    ] {
        let fields = Fields::new();
        let extension = extension();
        extension.variables.borrow_mut().insert(
            "value".into(),
            ScriptValue {
                text: text.into(),
                present,
                truth,
            },
        );
        let mut context = EvalContext::new(&fields);
        context.extension = Some(&extension);
        let compile = |expression| {
            CompiledTemplate::compile_with_functions(expression, &functions()).unwrap()
        };
        let value = compile("$fetch()")
            .evaluate_value_with_context(&context)
            .unwrap();
        assert_eq!(
            (&*value.text, value.present, value.truth),
            (text, present, truth)
        );
        assert_eq!(extension.calls.borrow().as_slice(), ["fetch"]);
        assert_eq!(
            compile("$if($fetch(),yes,no)")
                .evaluate_with_context(&context)
                .unwrap(),
            if truth { "yes" } else { "no" }
        );
        assert_eq!(
            compile("[X$fetch()]")
                .evaluate_with_context(&context)
                .unwrap(),
            if present {
                format!("X{text}")
            } else {
                String::new()
            }
        );
        let forwarded = compile("$store($fetch())")
            .evaluate_value_with_context(&context)
            .unwrap();
        assert_eq!(
            (&*forwarded.text, forwarded.present, forwarded.truth),
            (text, present, truth)
        );
        assert_eq!(
            extension.calls.borrow().as_slice(),
            ["fetch", "fetch", "fetch", "fetch", "store"]
        );
    }
}
#[test]
fn unregistered_functions_literalize_without_dispatching_available_host() {
    struct NeverCall;
    impl ScriptExtension for NeverCall {
        fn call(&self, _: &str, _: &[ScriptValue]) -> Result<ScriptValue, String> {
            panic!("unregistered host function dispatched")
        }
    }
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.extension = Some(&NeverCall);
    let template = CompiledTemplate::compile("$fetch()").unwrap();
    assert_eq!(
        template.evaluate_with_context(&context).unwrap(),
        "$fetch()"
    );
    assert_eq!(template.evaluate(&fields).unwrap(), "$fetch()");
}

#[test]
fn explicitly_registered_native_known_names_keep_the_custom_host_contract() {
    struct Custom;
    impl ScriptExtension for Custom {
        fn call(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String> {
            assert!(["get", "put", "puts"].contains(&name));
            assert!(args.is_empty());
            Ok(ScriptValue {
                text: "custom".into(),
                present: false,
                truth: true,
            })
        }
    }
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.extension = Some(&Custom);
    for name in ["get", "put", "puts"] {
        let template = CompiledTemplate::compile_with_functions(
            &format!("${name}()"),
            &[ScriptFunctionSpec {
                name: name.into(),
                min_args: 0,
                max_args: 0,
            }],
        )
        .unwrap();
        let value = template.evaluate_value_with_context(&context).unwrap();
        assert_eq!(
            (&*value.text, value.present, value.truth),
            ("custom", false, true)
        );
    }
}
