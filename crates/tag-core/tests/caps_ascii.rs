//! Independently derived ASCII behavior from the shipped 3.36.1 callback branches.
//! Target SHA-256: 484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911.
//! Callbacks: caps 0x14010c394, caps2 0x14010c430, caps3 0x14010c4bc;
//! trigger helpers 0x14010c1c8 and 0x1402b8044. No original runtime capture.
//! Unicode/locale/UTF-16 equivalence and native upper arity remain unqualified.

use tag_core::{evaluate, Fields};

type Case = (
    &'static str,
    &'static str,
    Option<&'static str>,
    &'static str,
);

const CAPS: &[Case] = &[
    ("caps-omitted", "hELLO wORLD", None, "Hello World"),
    ("caps-empty", "hELLO wORLD", Some(""), "Hello world"),
    ("caps-hyphen", "hELLO wORLD", Some("-"), "Hello world"),
    ("caps-space", "hELLO wORLD", Some(" "), "Hello World"),
    ("caps-tab-newline-default", "a\tb\nc", None, "A\tb\nc"),
    (
        "caps-tab-newline-explicit",
        "a\tb\nc",
        Some("\t\n"),
        "A\tB\nC",
    ),
    (
        "caps-hyphen-with-space",
        "hELLO-wORLD nEXT",
        Some("-"),
        "Hello-World next",
    ),
    ("caps-leading-spaces-empty", "  a b", Some(""), "  a b"),
    ("caps-cr-default", "a\rb", None, "A\rb"),
    ("caps-initial-letter-is-trigger", "xABC", Some("x"), "xAbc"),
    ("caps-trigger-letter-case", "aXb", Some("x"), "AxB"),
    ("caps-empty-text", "", None, ""),
    ("caps-d-trigger-aDb", "aDb", Some("d"), "AdB"),
    ("caps-d-trigger-dABC", "dABC", Some("d"), "dAbc"),
];

const CAPS2: &[Case] = &[
    ("caps2-omitted", "hELLO wORLD", None, "HELLO WORLD"),
    ("caps2-empty", "hELLO wORLD", Some(""), "HELLO wORLD"),
    ("caps2-hyphen", "hELLO wORLD", Some("-"), "HELLO wORLD"),
    ("caps2-space", "hELLO wORLD", Some(" "), "HELLO WORLD"),
    ("caps2-tab-newline-default", "a\tb\nc", None, "A\tb\nc"),
    (
        "caps2-tab-newline-explicit",
        "a\tb\nc",
        Some("\t\n"),
        "A\tB\nC",
    ),
    (
        "caps2-hyphen-with-space",
        "hELLO-wORLD nEXT",
        Some("-"),
        "HELLO-WORLD nEXT",
    ),
    ("caps2-leading-spaces-empty", "  a b", Some(""), "  a b"),
    ("caps2-cr-default", "a\rb", None, "A\rb"),
    ("caps2-initial-letter-is-trigger", "xABC", Some("x"), "xABC"),
    ("caps2-trigger-letter-case", "aXb", Some("x"), "AXB"),
    ("caps2-empty-text", "", None, ""),
    ("caps2-d-trigger-aDb", "aDb", Some("d"), "ADB"),
    ("caps2-d-trigger-dABC", "dABC", Some("d"), "dABC"),
];

const CAPS3: &[Case] = &[
    ("caps3-omitted", "hELLO wORLD", None, "Hello world"),
    ("caps3-empty", "hELLO wORLD", Some(""), "Hello world"),
    ("caps3-hyphen", "hELLO wORLD", Some("-"), "Hello world"),
    ("caps3-space", "hELLO wORLD", Some(" "), "Hello World"),
    ("caps3-tab-newline-default", "a\tb\nc", None, "A\tb\nc"),
    (
        "caps3-tab-newline-explicit",
        "a\tb\nc",
        Some("\t\n"),
        "A\tB\nC",
    ),
    (
        "caps3-hyphen-with-space",
        "hELLO-wORLD nEXT",
        Some("-"),
        "Hello-World next",
    ),
    (
        "caps3-immediate-trigger-not-skip-space",
        "FIRST. SECOND",
        Some("."),
        "First. second",
    ),
    (
        "caps3-consecutive-trigger",
        "FIRST..SECOND",
        Some("."),
        "First..Second",
    ),
    ("caps3-initial-letter-is-trigger", "xABC", Some("x"), "xAbc"),
    ("caps3-trigger-letter-case", "aXb", Some("x"), "Axb"),
    ("caps3-empty-text", "", None, ""),
    ("caps3-d-trigger-aDb", "aDb", Some("d"), "Adb"),
    ("caps3-d-trigger-dABC", "dABC", Some("d"), "dAbc"),
    ("caps3-d-trigger-ddA", "ddA", Some("d"), "ddA"),
];

fn check(function: &str, cases: &[Case]) {
    for &(id, input, separators, expected) in cases {
        let mut fields = Fields::from([("TITLE".into(), vec![input.into()])]);
        let template = if let Some(separators) = separators {
            fields.insert("SEPARATORS".into(), vec![separators.into()]);
            format!("${function}(%title%,%separators%)")
        } else {
            format!("${function}(%title%)")
        };
        assert_eq!(
            evaluate(&template, &fields).unwrap(),
            expected,
            "{id}: {template}"
        );
    }
}

#[test]
fn caps_matches_static_ascii_separator_and_start_cases() {
    check("caps", CAPS);
}

#[test]
fn caps2_matches_static_ascii_preservation_and_trigger_cases() {
    check("caps2", CAPS2);
}

#[test]
fn caps3_matches_static_ascii_exact_and_consecutive_trigger_cases() {
    check("caps3", CAPS3);
}

#[test]
fn caps_family_keeps_the_existing_one_or_two_argument_contract() {
    for function in ["caps", "caps2", "caps3"] {
        let error = evaluate(&format!("${function}(a,-,unused)"), &Fields::new()).unwrap_err();
        assert!(
            error.contains("expects 1..=2 arguments"),
            "{function}: {error}"
        );
    }
}

#[test]
fn caps_family_retains_safe_rust_unicode_case_expansion() {
    // This is the retained TagGroove contract, not an original Unicode oracle.
    let fields = Fields::from([("TITLE".into(), vec!["ßETA".into()])]);
    for (function, expected) in [("caps", "SSeta"), ("caps2", "SSETA"), ("caps3", "SSeta")] {
        assert_eq!(
            evaluate(&format!("${function}(%title%)"), &fields).unwrap(),
            expected
        );
    }
}
