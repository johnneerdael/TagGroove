use std::{cell::RefCell, collections::BTreeMap};
use tag_core::{
    CompiledTemplate, EvalContext, Fields, ScriptExtension, ScriptFunctionSpec, ScriptValue,
};

struct Extension {
    variables: RefCell<BTreeMap<String, ScriptValue>>,
    counter: RefCell<usize>,
}
impl ScriptExtension for Extension {
    fn call(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String> {
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
    assert!(CompiledTemplate::compile("$fetch()").is_err());
    let f: Fields = [("ARTIST".into(), vec!["A".into()])].into();
    let mut context = EvalContext::new(&f);
    let extension = extension();
    context.extension = Some(&extension);
    let compile = |text| CompiledTemplate::compile_with_functions(text, &functions()).unwrap();
    assert_eq!(
        compile("$store($eql(a,b))$if($fetch(),yes,no)")
            .evaluate_with_context(&context)
            .unwrap(),
        "0no"
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
