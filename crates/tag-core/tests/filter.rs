use tag_core::{compile_filter, matches_filter, Fields};

fn fields() -> Fields {
    [
        (
            "ARTIST".into(),
            vec!["Hidden Orchestra".into(), "Björk".into()],
        ),
        ("TITLE".into(), vec!["Symphony No. 3 (猫)".into()]),
        ("GENRE".into(), vec!["Electronic".into()]),
        ("YEAR".into(), vec!["02024".into()]),
        ("TRACK".into(), vec!["3/12".into()]),
        ("_BITRATE".into(), vec!["320".into()]),
        ("_FILENAME_EXT".into(), vec!["03 - Momentum.flac".into()]),
        (
            "_PATH".into(),
            vec!["/SecretFolder/03 - Momentum.flac".into()],
        ),
    ]
    .into()
}

#[test]
fn free_search_requires_all_words_across_tags_or_filename() {
    let f = fields();
    for query in ["", "  ", "hidden momentum", "BJÖRK 猫", "symphony FLAC"] {
        assert!(matches_filter(query, &f).unwrap(), "{query}");
    }
    for query in [
        "hidden absent",
        "SecretFolder",
        "320",
        "Hidden*",
        "artist has Hidden",
    ] {
        assert!(!matches_filter(query, &f).unwrap(), "{query}");
    }
}

#[test]
fn field_has_and_is_use_documented_literal_semantics() {
    let f = fields();
    assert!(matches_filter("artist HAS Orchestra Hidden", &f).unwrap());
    assert!(matches_filter("artist IS Hidden Orchestra", &f).unwrap());
    assert!(matches_filter("artist IS björk", &f).unwrap());
    assert!(!matches_filter("artist IS Hidden*", &f).unwrap());
    assert!(!matches_filter("artist HAS Momentum", &f).unwrap());
    assert!(matches_filter("* HAS electronic", &f).unwrap());
    assert!(!matches_filter("* HAS SecretFolder", &f).unwrap());
    assert!(matches_filter("\"ALBUM ARTIST\" MISSING", &f).unwrap());
}

#[test]
fn boolean_precedence_grouping_and_negation() {
    let f = fields();
    assert!(matches_filter(
        "(genre IS Electronic AND NOT artist IS Other) OR title IS Wrong",
        &f
    )
    .unwrap());
    assert!(!matches_filter("NOT (genre IS Electronic OR artist IS Other)", &f).unwrap());
    assert!(matches_filter("artist IS Björk OR genre IS Other AND title IS Wrong", &f).unwrap());
    assert!(!matches_filter("(artist IS Björk OR genre IS Other) AND title IS Wrong", &f).unwrap());
    assert!(matches_filter("NOT NOT artist IS Björk", &f).unwrap());
}

#[test]
fn missing_present_absent_and_empty_values_are_distinct() {
    let mut f = fields();
    f.insert("EMPTY".into(), vec![String::new()]);
    f.insert("REMOVED".into(), vec![]);
    for query in [
        "albumartist MISSING",
        "albumartist ABSENT",
        "artist PRESENT",
        "EMPTY PRESENT",
        "EMPTY IS \"\"",
        "REMOVED MISSING",
        "%albumartist% MISSING",
    ] {
        assert!(matches_filter(query, &f).unwrap(), "{query}");
    }
    assert!(!matches_filter("albumartist IS \"\"", &f).unwrap());
    assert!(matches_filter("\"%albumartist%\" IS \"\"", &f).unwrap());
}

#[test]
fn quoted_format_expressions_and_numeric_filters() {
    let f = fields();
    for query in [
        "\"%_bitrate%\" GREATER 180",
        "year EQUAL 2024",
        "year LESS 2025",
        "\"$if($grtr($len(%title%),3),YES,NO)\" IS yes",
        "\"$num(%track%,2)\" EQUAL 3",
        "\"$replace(%artist%,'Orchestra','AND (OR)')\" HAS \"AND (OR)\"",
    ] {
        assert!(matches_filter(query, &f).unwrap(), "{query}");
    }
    assert!(!matches_filter("year GREATER 2025", &f).unwrap());
    assert!(compile_filter("year GREATER 3:30").is_err());
    assert!(compile_filter("year GREATER 3.5").is_err());
}

#[test]
fn regex_supports_grouping_unicode_and_boost_digit_escape() {
    let f = fields();
    for query in [
        "title MATCHES \"^symphony.*(猫)\\)$\"",
        "track MATCHES \\d\\/\\d",
        "(artist IS Björk AND title MATCHES (Symphony|Wrong))",
        "* MATCHES \"björk|cat\"",
        "title MATCHES \\(猫\\)",
    ] {
        assert!(matches_filter(query, &f).unwrap(), "{query}");
    }
    assert!(!matches_filter("artist MATCHES ^Orchestra$", &f).unwrap());
    let f: Fields = [("TITLE".into(), vec!["a\"b".into()])].into();
    assert!(matches_filter("title MATCHES \\\"", &f).unwrap());
}

#[test]
fn quoted_boolean_words_are_literals() {
    let f: Fields = [("TITLE".into(), vec!["AND OR NOT".into()])].into();
    assert!(matches_filter("title IS \"AND OR NOT\"", &f).unwrap());
    assert!(matches_filter("\"AND\"", &f).unwrap());
}

#[test]
fn malformed_queries_and_unsupported_regex_never_succeed_silently() {
    for query in [
        "artist IS",
        "artist HAS",
        "artist PRESENT extra",
        "AND artist IS x",
        "artist IS x OR",
        "NOT",
        "(artist IS x",
        "artist IS x)",
        "artist IS \"unfinished",
        "title MATCHES \"(\"",
        "\"$not_a_function(%title%)\" IS x",
        "* PRESENT",
        "* IS x",
        "artist MATCH x",
    ] {
        assert!(compile_filter(query).is_err(), "{query}");
    }
}

#[test]
fn compiled_filter_is_reusable_and_owns_its_query() {
    let filter = compile_filter(&String::from(
        "artist MATCHES \"björk\" AND NOT albumartist PRESENT",
    ))
    .unwrap();
    let f = fields();
    let mut different = f.clone();
    different.insert("ALBUMARTIST".into(), vec!["Other".into()]);
    for _ in 0..10_000 {
        assert!(filter.matches(&f).unwrap());
        assert!(!filter.matches(&different).unwrap());
    }
}

#[test]
fn regex_character_classes_do_not_become_filter_groups() {
    let f = fields();
    for query in [
        "title MATCHES [()]",
        "(title MATCHES [(]) AND artist PRESENT",
        "title MATCHES [ )]",
        "title MATCHES \\(",
    ] {
        assert!(matches_filter(query, &f).unwrap(), "{query}");
    }
}

#[test]
fn malformed_presence_and_script_arity_fail_during_compilation() {
    for query in [
        "% MISSING",
        "%% PRESENT",
        "\"$upper()\" IS x",
        "\"$left(%title%)\" IS x",
        "\"$replace(x,a)\" IS x",
    ] {
        assert!(compile_filter(query).is_err(), "{query}");
    }
}

#[test]
fn ordinary_search_preserves_literal_parentheses_and_brackets() {
    let f = fields();
    assert!(matches_filter("symphony (猫)", &f).unwrap());
    assert!(matches_filter("(猫)", &f).unwrap());
    assert!(!matches_filter("(wrong)", &f).unwrap());
    assert!(matches_filter("\"symphony (猫)\"", &f).unwrap());
    assert!(matches_filter("genre HAS electronic AND title HAS (猫)", &f).unwrap());
}

#[test]
fn matches_uses_the_same_boost_engine_as_scripting() {
    let f: Fields = [("TITLE".into(), vec!["abc abc".into()])].into();
    assert!(matches_filter("title MATCHES \"(abc) \\1\"", &f).unwrap());
    assert!(matches_filter("title MATCHES \"(?<=a)bc\"", &f).unwrap());
    assert!(matches_filter("title MATCHES \"ABC\"", &f).unwrap());
}

#[test]
fn compiled_filters_accept_application_and_date_context() {
    let f = fields();
    let columns = vec![tag_core::ScriptColumn {
        name: "Title".into(),
        value: "Symphony".into(),
    }];
    let mut context = tag_core::EvalContext::new(&f);
    context.columns = Some(&columns);
    context.timestamp_seconds = Some(0);
    context.timezone_offset_seconds = Some(0);
    context.app_directory = Some("/apps/OpenTag");
    let filter = compile_filter("\"$columnNames(,',')\" HAS title AND \"$fmtDate(yyyy)\" IS 1970 AND \"$getenv(MP3TAGAPP)\" IS /apps/OpenTag").unwrap();
    assert!(filter.matches_with_context(&context).unwrap());
    assert!(filter.matches(&f).is_err());
}

#[test]
fn numeric_filters_compare_the_numeric_part_of_values() {
    let f: Fields = [
        ("TRACK".into(), vec!["3/12".into()]),
        ("DISCNUMBER".into(), vec![" 02 / 03".into()]),
        ("YEAR".into(), vec!["2024-05-01".into()]),
        ("BPM".into(), vec!["128.6".into()]),
        ("MOOD".into(), vec!["calm".into()]),
        ("GAIN".into(), vec!["-6 dB".into()]),
        ("SERIAL".into(), vec!["99999999999999999999999".into()]),
    ]
    .into();
    for (query, expected) in [
        ("track EQUAL 3", true),
        ("track GREATER 2", true),
        ("track GREATER 10", false),
        ("track LESS 4", true),
        ("track EQUAL 12", false),
        ("discnumber EQUAL 2", true),
        ("year GREATER 2023", true),
        ("year EQUAL 2024", true),
        ("bpm EQUAL 128", true),
        ("gain LESS 0", true),
        ("gain EQUAL -6", true),
        ("serial GREATER 1000", true),
        ("mood EQUAL 0", false),
        ("mood LESS 1", false),
        ("NOT mood GREATER 0", true),
        ("title GREATER 0", false),
    ] {
        assert_eq!(matches_filter(query, &f).unwrap(), expected, "{query}");
    }
}
