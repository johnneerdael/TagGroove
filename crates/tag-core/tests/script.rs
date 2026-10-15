use tag_core::{
    apply_action, evaluate, evaluate_with_context, parse_filename, Action, EvalContext, Fields,
    ScriptColumn,
};

fn fields() -> Fields {
    [
        ("ARTIST".into(), vec!["Björk".into(), "Another".into()]),
        ("TITLE".into(), vec!["it's A SONG".into()]),
        ("TRACK".into(), vec!["3/12".into()]),
    ]
    .into()
}

#[test]
fn formatting_nested_quotes_and_presence() {
    let f = fields();
    assert_eq!(
        evaluate("$upper($left(%artist%,2)) '$replace(x,y,z)' ''", &f).unwrap(),
        "BJ $replace(x,y,z) '"
    );
    assert_eq!(
        evaluate("[CD$num(%discnumber%,1)] [%artist% - ][literal]", &f).unwrap(),
        " Björk - "
    );
    assert_eq!(
        evaluate("[%missing%$upper(%title%)]", &f).unwrap(),
        "IT'S A SONG"
    );
    assert_eq!(
        evaluate("$meta_sep(artist,'; ') / $meta(artist,1)", &f).unwrap(),
        "Björk; Another / Another"
    );
}

#[test]
fn unicode_positions_arithmetic_and_replacement() {
    let f = fields();
    assert_eq!(
        evaluate(
            "$len(é猫🙂)|$mid(é猫🙂,2,1)|$right(é猫🙂,2)|$strstr(é猫🙂,🙂)",
            &f
        )
        .unwrap(),
        "3|猫|猫🙂|3"
    );
    assert_eq!(
        evaluate(
            "$num(%track%,2)|$add(1,2,3)|$sub(10,2,3)|$div(10,3)|$mod(10,3)",
            &f
        )
        .unwrap(),
        "03|6|5|3|1"
    );
    assert_eq!(
        evaluate(
            "$replace(abc,a,x,b,y)|$regexp('Track 12','([0-9]+)','[$1]')",
            &f
        )
        .unwrap(),
        "xyc|Track [12]"
    );
    assert_eq!(
        evaluate("$if($eql(HELLO,hello),yes,no)|$ifgreater(3,2,yes,no)", &f).unwrap(),
        "yes|yes"
    );
}

#[test]
fn invalid_expressions_are_errors() {
    for template in [
        "$not_real(x)",
        "$upper(x",
        "%title",
        "[abc",
        "'abc",
        "$div(1,0)",
        "$left(x)",
        "$replace(x,a)",
        "$regexp(x,'(',y)",
    ] {
        assert!(evaluate(template, &fields()).is_err(), "{template}");
    }
}

#[test]
fn filename_captures_are_whole_and_literal() {
    let f = parse_filename("03 - Björk - 猫.mp3", "%track% - %artist% - %title%").unwrap();
    assert_eq!(f["TITLE"], vec!["猫"]);
    assert_eq!(f["TRACK"], vec!["03"]);
    assert_eq!(
        parse_filename(
            "ignore - Artist - Guest.flac",
            "%dummy% - %artist% - %artist%"
        )
        .unwrap()["ARTIST"],
        vec!["Artist", "Guest"]
    );
    assert!(parse_filename("A - B trailing", "%artist% - %title% END").is_err());
    assert!(parse_filename("ABC", "%artist%%title%").is_err());
    assert!(parse_filename("ABC", "%artist").is_err());
    assert_eq!(
        parse_filename(
            "[2018] Album\\09 - Momentum.mp3",
            "[%year%] %album%\\%track% - %title%"
        )
        .unwrap()["YEAR"],
        vec!["2018"]
    );
}

#[test]
fn actions_preserve_multi_values_and_remove_case_insensitively() {
    let original = fields();
    let transformed = apply_action(
        &original,
        &Action::Case {
            field: "_ALL".into(),
            mode: "lower".into(),
        },
    )
    .unwrap();
    assert_eq!(transformed["ARTIST"], vec!["björk", "another"]);
    assert_eq!(original["ARTIST"][0], "Björk");
    let transformed = apply_action(
        &original,
        &Action::RegexReplace {
            field: "title".into(),
            pattern: "(A) (SONG)".into(),
            replacement: "$2: $1".into(),
        },
    )
    .unwrap();
    assert_eq!(transformed["TITLE"], vec!["it's SONG: A"]);
    let transformed = apply_action(
        &original,
        &Action::Remove {
            fields: vec!["artist".into()],
        },
    )
    .unwrap();
    assert!(!transformed.contains_key("ARTIST"));
    assert!(apply_action(
        &original,
        &Action::Case {
            field: "TITLE".into(),
            mode: "invalid".into()
        }
    )
    .is_err());
}

#[test]
fn action_json_contract() {
    let action: Action =
        serde_json::from_str(r#"{"type":"format","field":"TITLE","template":"$upper(%title%)"}"#)
            .unwrap();
    assert_eq!(
        apply_action(&fields(), &action).unwrap()["TITLE"],
        vec!["IT'S A SONG"]
    );
}

#[test]
fn booleans_are_typed_instead_of_numeric_text() {
    assert_eq!(
        evaluate("$if(1,yes,no)|$if(0,yes,no)|$if(%track%,yes,no)", &fields()).unwrap(),
        "no|no|yes"
    );
    assert_eq!(
        evaluate(
            "$if($strstr(%artist%,absent),yes,no)|$if($grtr($strstr(%artist%,absent),0),yes,no)",
            &fields()
        )
        .unwrap(),
        "yes|no"
    );
    assert_eq!(
        evaluate(
            "$if2(%missing%,fallback)|$if2(%artist%,fallback)",
            &fields()
        )
        .unwrap(),
        "fallback|Björk"
    );
    assert_eq!(
        evaluate(
            "$if($and(%artist%,%missing%),yes,no)|$if($or(%artist%,%missing%),yes,no)",
            &fields()
        )
        .unwrap(),
        "no|yes"
    );
}

#[test]
fn documented_function_families() {
    for (input, expected) in [
        ("$cutLeft(é猫🙂,1)", "猫🙂"),
        ("$cutRight(é猫🙂,1)", "é猫"),
        ("$mid(abc,2)", "b"),
        ("$mid(abc,0,2)", ""),
        ("$caps(hELLO wORLD,-)", "Hello World"),
        ("$caps2(hELLO wORLD)", "HELLO WORLD"),
        ("$trim('--hi--',-)", "hi"),
        ("$trimLeft('--hi--',-)", "hi--"),
        ("$trimRight('--hi--',-)", "--hi"),
        ("$reverse(é猫🙂)", "🙂猫é"),
        ("$char(233)", "é"),
        ("$ord(é)", "233"),
        ("$repeat(猫,3)", "猫猫猫"),
        ("$strchr(é猫猫,猫)", "2"),
        ("$strrchr(é猫猫,猫)", "3"),
        ("$strrstr(abcabc,bc)", "5"),
        ("$sort(b;a;c,;)", "a;b;c"),
        ("$dedup(A;a;B;A,;,1)", "A;B"),
        ("$distance(kitten,sitting)", "3"),
        ("$validate('a:b/c','_')", "a_b_c"),
        ("$mul(2,3,4)", "24"),
        ("$if($not($eql(a,b)),yes,no)", "yes"),
        ("$if($neql(a,a),yes,no)", "no"),
        ("$if($isdigit(12x),yes,no)", "yes"),
        ("$if($odd(3),yes,no)", "yes"),
        ("$if($geql(2,2),yes,no)", "yes"),
        ("$if($leql(2,2),yes,no)", "yes"),
        ("$if($less(2,3),yes,no)", "yes"),
        ("$iflonger(猫🙂,1,yes,no)", "yes"),
        ("$if($strcmp(a,A),yes,no)", "no"),
        ("$if($stricmp(a,A),yes,no)", "yes"),
    ] {
        assert_eq!(
            evaluate(input, &Fields::new()).unwrap(),
            expected,
            "{input}"
        );
    }
}

#[test]
fn regex_boost_references_and_ascii_digits() {
    assert_eq!(
        evaluate(
            "$regexp('abc 12','([0-9]+)','$1suffix ${1} $& $$ \\1')",
            &fields()
        )
        .unwrap(),
        "abc 12suffix 12 12 $ 12"
    );
    assert_eq!(
        evaluate("$regexp('１2','\\d',x)", &fields()).unwrap(),
        "１x"
    );
    // Boost character classes follow icase: \u only means "upper" with c=0.
    assert_eq!(
        evaluate("$regexp('Aa','\\u',x,0)", &fields()).unwrap(),
        "xa"
    );
    assert_eq!(evaluate("$regexp('Aa','\\u',x)", &fields()).unwrap(), "xx");
    assert_eq!(evaluate("$regexp(x,x,'\\U$0')", &fields()).unwrap(), "X");
    assert_eq!(evaluate("$regexp(x,x,'$MATCH')", &fields()).unwrap(), "x");
}

#[test]
fn recursion_and_arithmetic_limits() {
    let args = std::iter::repeat_n("1", 64).collect::<Vec<_>>().join(",");
    assert_eq!(
        evaluate(&format!("$add({args})"), &Fields::new()).unwrap(),
        "64"
    );
    assert!(evaluate(&format!("$add({args},1)"), &Fields::new()).is_err());
    assert!(evaluate("$add(9223372036854775807,1)", &Fields::new()).is_err());
    assert!(evaluate("$repeat(a,1048577)", &Fields::new()).is_err());
    assert!(evaluate(
        &format!("{}x{}", "[".repeat(130), "]".repeat(130)),
        &Fields::new()
    )
    .is_err());
}

#[test]
fn folder_patterns_use_relative_suffix_and_only_remove_real_extension() {
    let fields = parse_filename(
        "/library/Artist/Album/03 - Mix.version.mp3",
        "%artist%\\%album%\\%track% - %title%",
    )
    .unwrap();
    assert_eq!(fields["ARTIST"], vec!["Artist"]);
    assert_eq!(fields["TITLE"], vec!["Mix.version"]);
    assert_eq!(
        parse_filename(
            "/library/Artist/Album/03 - Mix.version.mp3",
            "%track% - %title%"
        )
        .unwrap()["TITLE"],
        vec!["Mix.version"]
    );
    assert_eq!(
        parse_filename("03 - Mix.version.mp3", "%track% - %title%.mp3").unwrap()["TITLE"],
        vec!["Mix.version"]
    );
    assert!(parse_filename("/Wrong/03 - Name.mp3", "C:\\%artist%\\%track% - %title%").is_err());
}

#[test]
fn sentence_case_uses_only_explicit_trigger_characters() {
    for (template, expected) in [
        ("$caps3(hELLO wORLD)", "Hello world"),
        ("$caps3('01..HARD AS ROCK')", "01..hard as rock"),
        ("$caps3('first|SECOND',|)", "First|Second"),
        ("$caps3('FIRST. SECOND',.)", "First. second"),
    ] {
        assert_eq!(evaluate(template, &fields()).unwrap(), expected);
    }
}

#[test]
fn localized_dates_cover_every_documented_pattern() {
    let f = Fields::new();
    let mut context = EvalContext::new(&f);
    context.locale = "en-US".into();
    context.timestamp_seconds = Some(981173106);
    context.timezone_offset_seconds = Some(0);
    assert_eq!(
        evaluate_with_context(
            "$fmtDate('yyyy|yy|MMMM|MMM|MM|M|dddd|ddd|dd|d|HH|H|hh|h|mm|m|ss|s|EEEE|EE')",
            &context
        )
        .unwrap(),
        "2001|01|February|Feb|02|2|Saturday|Sat|03|3|04|4|04|4|05|5|06|6|Saturday|Sat"
    );
    assert_eq!(
        evaluate_with_context("$fmtDate('yyyy-MM-dd HH:mm:ss',0)", &context).unwrap(),
        "1970-01-01 00:00:00"
    );
    context.timezone_offset_seconds = Some(5400);
    assert_eq!(
        evaluate_with_context("$fmtDate('yyyy-MM-dd HH:mm:ss',0)", &context).unwrap(),
        "1970-01-01 01:30:00"
    );
    context.locale = "nl-NL".into();
    assert_eq!(
        evaluate_with_context("$fmtDate('MMMM dddd')", &context).unwrap(),
        "februari zaterdag"
    );
}

#[test]
fn durations_use_units_hours_and_milliseconds() {
    for (template, expected) in [
        ("$fmtDuration(61)", "1:01"),
        ("$fmtDuration(1.5,m)", "1:30"),
        ("$fmtDuration(61234,ms,0,1)", "1:01.234"),
        ("$fmtDuration(3661,s,1)", "01:01:01"),
        ("$fmtDuration(90061,s,1)", "25:01:01"),
        ("$fmtDuration(61.234,s,1,1)", "00:01:01.234"),
    ] {
        assert_eq!(
            evaluate(template, &fields()).unwrap(),
            expected,
            "{template}"
        );
    }
    assert!(evaluate("$fmtDuration(3,unknown)", &fields()).is_err());
}

#[test]
fn number_grouping_follows_explicit_locale() {
    let f = Fields::new();
    let mut context = EvalContext::new(&f);
    for (locale, expected) in [
        ("en-US", "1,234,567"),
        ("de-DE", "1.234.567"),
        ("en-IN", "12,34,567"),
    ] {
        context.locale = locale.into();
        assert_eq!(
            evaluate_with_context("$fmtNum(1234567.89)", &context).unwrap(),
            expected
        );
    }
    context.locale = "en-US".into();
    assert_eq!(
        evaluate_with_context("$fmtNum(-1234.99)", &context).unwrap(),
        "-1,234"
    );
}

#[test]
fn ansi_conversion_uses_real_encoding_and_replaces_unrepresentable_characters() {
    let f = Fields::new();
    let mut context = EvalContext::new(&f);
    context.ansi_codepage = Some("windows-1252".into());
    assert_eq!(
        evaluate_with_context("$ansi('Björk 猫 🙂')", &context).unwrap(),
        "Björk ? ?"
    );
    context.ansi_codepage = Some("windows-1251".into());
    assert_eq!(
        evaluate_with_context("$ansi('Привет 猫')", &context).unwrap(),
        "Привет ?"
    );
    context.ansi_codepage = Some("utf-8".into());
    assert_eq!(
        evaluate_with_context("$ansi('Привет 猫 🙂')", &context).unwrap(),
        "Привет 猫 🙂"
    );
    context.ansi_codepage = Some("not-an-encoding".into());
    assert!(evaluate_with_context("$ansi(test)", &context).is_err());
}

#[test]
fn folder_depth_understands_cross_platform_paths() {
    for (template, expected) in [
        ("$folderdepth('C:\\Music\\Album\\Track.mp3')", "2"),
        ("$folderdepth('C:\\Music\\Album\\')", "2"),
        ("$folderdepth('/Music/Album/Track.mp3')", "2"),
        ("$folderdepth('Music/Album/Track.mp3')", "2"),
        ("$folderdepth('Track.mp3')", "0"),
    ] {
        assert_eq!(
            evaluate(template, &fields()).unwrap(),
            expected,
            "{template}"
        );
    }
}

#[test]
fn column_functions_require_context_and_keep_column_order() {
    let f = Fields::new();
    let columns = vec![
        ScriptColumn {
            name: "Title".into(),
            value: "猫".into(),
        },
        ScriptColumn {
            name: "Artist".into(),
            value: "Björk".into(),
        },
    ];
    let mut context = EvalContext::new(&f);
    assert!(evaluate_with_context("$columnNames('<','>')", &context).is_err());
    context.columns = Some(&columns);
    assert_eq!(
        evaluate_with_context("$columnNames('<','>')", &context).unwrap(),
        "<Title><Artist>"
    );
    assert_eq!(
        evaluate_with_context("$columnValues('<','>')", &context).unwrap(),
        "<猫><Björk>"
    );
}

#[test]
fn environment_and_app_paths_are_explicit_and_absent_variables_are_empty() {
    let f = Fields::new();
    let environment = [("TEST_ENV".into(), "Value".into())].into();
    let mut context = EvalContext::new(&f);
    context.environment = Some(&environment);
    context.app_directory = Some("/apps/OpenTag");
    context.app_data_directory = Some("/config/OpenTag");
    assert_eq!(
        evaluate_with_context(
            "$getenv(TEST_ENV)|$getenv(ABSENT)|$getenv(MP3TAGAPP)|$getenv(MP3TAGAPPDATA)",
            &context
        )
        .unwrap(),
        "Value||/apps/OpenTag|/config/OpenTag"
    );
}

#[test]
fn random_and_uuid_functions_return_valid_values_and_enforce_arity() {
    let random = evaluate("$rand()", &fields())
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(random <= 32767);
    let first = uuid::Uuid::parse_str(&evaluate("$uuid()", &fields()).unwrap()).unwrap();
    let second = uuid::Uuid::parse_str(&evaluate("$uuid()", &fields()).unwrap()).unwrap();
    assert_eq!(first.get_version_num(), 4);
    assert_ne!(first, second);
    assert!(evaluate("$rand(1)", &fields()).is_err());
    assert!(evaluate("$uuid(1)", &fields()).is_err());
}

#[test]
fn every_documented_function_is_registered() {
    let names = [
        "ansi",
        "caps",
        "caps2",
        "caps3",
        "char",
        "cutleft",
        "cutright",
        "dedup",
        "distance",
        "fmtdate",
        "fmtduration",
        "fmtnum",
        "folderdepth",
        "left",
        "len",
        "lower",
        "mid",
        "num",
        "ord",
        "repeat",
        "regexp",
        "replace",
        "reverse",
        "right",
        "sort",
        "strcmp",
        "stricmp",
        "strchr",
        "strrchr",
        "strstr",
        "strrstr",
        "trim",
        "trimleft",
        "trimright",
        "upper",
        "validate",
        "and",
        "eql",
        "geql",
        "grtr",
        "if",
        "if2",
        "ifgreater",
        "iflonger",
        "isdigit",
        "leql",
        "less",
        "neql",
        "not",
        "odd",
        "or",
        "add",
        "div",
        "mod",
        "mul",
        "rand",
        "sub",
        "uuid",
        "getenv",
        "columnnames",
        "columnvalues",
        "list",
        "meta",
        "meta_sep",
    ];
    assert_eq!(names.len(), 64);
    for name in names {
        assert!(tag_core::SUPPORTED_FUNCTIONS.contains(&name), "{name}");
    }
}

#[test]
fn list_handles_arbitrary_fields_multivalues_and_explicit_delimiters() {
    let f: Fields = [
        ("ARTIST".into(), vec!["Björk".into(), "Guest".into()]),
        ("CUSTOM FIELD".into(), vec!["猫".into()]),
    ]
    .into();
    assert_eq!(
        evaluate("$list('<',=,'>',';')", &f).unwrap(),
        "<ARTIST=Björk;Guest><CUSTOM FIELD=猫>"
    );
    assert_eq!(
        evaluate("$list('<',=,'>')", &f).unwrap(),
        "<ARTIST=Björk,Guest><CUSTOM FIELD=猫>"
    );
}

#[test]
fn compiled_templates_reuse_different_contexts_without_freezing_metadata() {
    let first: Fields = [("ARTIST".into(), vec!["First".into()])].into();
    let second: Fields = [("ARTIST".into(), vec!["Second".into()])].into();
    let expression = tag_core::CompiledTemplate::compile("%artist% $fmtDate(yyyy)").unwrap();
    let mut context = EvalContext::new(&first);
    context.timestamp_seconds = Some(981173106);
    context.timezone_offset_seconds = Some(0);
    assert_eq!(
        expression.evaluate_with_context(&context).unwrap(),
        "First 2001"
    );
    context.fields = &second;
    context.timestamp_seconds = Some(0);
    assert_eq!(
        expression.evaluate_with_context(&context).unwrap(),
        "Second 1970"
    );
}

#[test]
fn optional_sections_track_resolved_fields_through_nested_functions() {
    let f = fields();
    assert_eq!(
        evaluate("'[literal $%]' 'it''s' [literal] [CD$num(%missing%,1)]", &f).unwrap(),
        "[literal $%] it's  "
    );
    assert_eq!(
        evaluate("[prefix$replace(%artist%,Björk,)]", &f).unwrap(),
        "prefix"
    );
    assert_eq!(
        evaluate("[outer[inner%missing%]%title%]", &f).unwrap(),
        "outerit's A SONG"
    );
    assert_eq!(
        evaluate("[before$meta(artist)after]", &f).unwrap(),
        "beforeBjörk, Anotherafter"
    );
}

#[test]
fn durations_avoid_float_rounding_and_handle_invalid_numbers() {
    assert_eq!(
        evaluate("$fmtDuration(0.29,s,0,1)", &fields()).unwrap(),
        "0:00.290"
    );
    assert_eq!(
        evaluate("$fmtDuration(-1.25,s,0,1)", &fields()).unwrap(),
        "-0:01.250"
    );
    for template in [
        "$fmtDuration(NaN)",
        "$fmtDuration(inf)",
        "$fmtDuration(1.2.3)",
        "$fmtDate(yyyy,not-a-time)",
        "$fmtDate()",
        "$fmtDuration()",
        "$fmtNum()",
        "$folderdepth()",
        "$columnNames(x)",
    ] {
        assert!(evaluate(template, &fields()).is_err(), "{template}");
    }
}

#[test]
fn all_documented_placeholders_resolve_from_supplied_context() {
    // Resolution verifies the format engine. Computing each value from real
    // files is the metadata/app/export adapter's separate obligation.
    let names = [
        "_bitrate",
        "_bitspersample",
        "_codec",
        "_cover_description",
        "_cover_mimetype",
        "_cover_size",
        "_cover_type",
        "_cover_height",
        "_cover_width",
        "_covers",
        "_id3v2_character_encoding",
        "_id3v2_unknown_frames",
        "_length",
        "_length_seconds",
        "_mode",
        "_mp4_creation_time",
        "_mp4_creation_time_raw",
        "_mp4_modification_time",
        "_mp4_modification_time_raw",
        "_samplerate",
        "_subsong",
        "_subsong_count",
        "_tag",
        "_tag_read",
        "_tag_size",
        "_tag_size_appended",
        "_tag_size_prepended",
        "_tool",
        "_vbr",
        "_video_bitrate",
        "_video_codec",
        "_video_height",
        "_video_width",
        "_directory",
        "_extension",
        "_filename",
        "_filename_ext",
        "_folderpath",
        "_path",
        "_parent_directory",
        "_volume",
        "_workingdir",
        "_workingpath",
        "_file_create_date",
        "_file_create_datetime",
        "_file_create_datetime_raw",
        "_file_mod_date",
        "_file_mod_datetime",
        "_file_mod_datetime_raw",
        "_file_size",
        "_file_size_bytes",
        "_file_size_kb",
        "_file_size_mb",
        "_app",
        "_counter",
        "_date",
        "_datetime",
        "_filename_rel",
        "_folderpath_rel",
        "_crc",
        "_md5",
        "_md5audio",
        "_max_counter",
        "_total_files",
        "_total_size",
        "_total_size_raw",
        "_total_time",
        "_total_time2",
        "_total_time_raw",
        "_playlist_filename",
        "_playlist_filename_ext",
        "_playlist_folderpath",
    ];
    assert_eq!(names.len(), 72);
    let f: Fields = names
        .iter()
        .map(|name| (name.to_uppercase(), vec![format!("value:{name}")]))
        .collect();
    let mut context = tag_core::EvalContext::new(&f);
    context.timestamp_seconds = Some(981_173_106);
    context.timezone_offset_seconds = Some(0);
    context.locale = "de-DE".into();
    for name in names {
        let expected = match name {
            "_date" => "03.02.2001".to_owned(),
            "_datetime" => "03.02.2001 04:05:06".to_owned(),
            _ => format!("value:{name}"),
        };
        assert_eq!(
            tag_core::evaluate_with_context(&format!("%{name}%"), &context).unwrap(),
            expected
        );
    }
}

#[test]
fn boost_regex_is_used_end_to_end_by_scripting_and_legacy_actions() {
    assert_eq!(
        evaluate("$regexp('abc abc xyz','\\b(\\w+) \\1\\b','$1')", &fields()).unwrap(),
        "abc xyz"
    );
    assert_eq!(
        evaluate(
            "$regexp('abc def','(?<=\\b)(\\w+)(?=\\b)','\\U$1\\E')",
            &fields()
        )
        .unwrap(),
        "ABC DEF"
    );
    assert_eq!(evaluate("$regexp('Ab','a',x,1)", &fields()).unwrap(), "xb");
    assert_eq!(evaluate("$regexp('Ab','a',x,0)", &fields()).unwrap(), "Ab");
    let f: Fields = [("TITLE".into(), vec!["abc abc".into()])].into();
    assert_eq!(
        apply_action(
            &f,
            &Action::RegexReplace {
                field: "TITLE".into(),
                pattern: "(abc) \\1".into(),
                replacement: "\\U$1\\E".into()
            }
        )
        .unwrap()["TITLE"],
        vec!["ABC"]
    );
    assert_eq!(
        apply_action(
            &f,
            &Action::Replace {
                field: "TITLE".into(),
                from: "ABC".into(),
                to: "$1\\literal".into(),
                case_sensitive: false
            }
        )
        .unwrap()["TITLE"],
        vec!["$1\\literal $1\\literal"]
    );
}

#[test]
fn matching_format_strings_treat_quotes_and_brackets_as_literals() {
    let f = parse_filename("'Björk' - [2024] 猫.flac", "'%artist%' - [%year%] %title%").unwrap();
    assert_eq!(f["ARTIST"], vec!["Björk"]);
    assert_eq!(f["YEAR"], vec!["2024"]);
    assert_eq!(f["TITLE"], vec!["猫"]);
}

#[test]
fn explicit_system_number_settings_override_locale_defaults() {
    let f = Fields::new();
    let format = tag_core::ScriptNumberFormat {
        separator: "_".into(),
        grouping: vec![3],
        minus_sign: "−".into(),
    };
    let mut context = EvalContext::new(&f);
    context.number_format = Some(&format);
    assert_eq!(
        evaluate_with_context("$fmtNum(-1234567)", &context).unwrap(),
        "−1_234_567"
    );
}

#[test]
fn raw_list_excludes_phantom_aliases_and_preserves_actual_names() {
    let raw: Fields = [
        ("DATE".into(), vec!["2024".into()]),
        ("TRACKNUMBER".into(), vec!["3".into()]),
        ("_CUSTOM".into(), vec!["stored".into()]),
    ]
    .into();
    let mut fields = raw.clone();
    fields.insert("YEAR".into(), vec!["2024".into()]);
    fields.insert("TRACK".into(), vec!["3".into()]);
    let mut context = EvalContext::new(&fields);
    context.raw_fields = Some(&raw);
    assert_eq!(
        evaluate_with_context("$list('<',=,'>')", &context).unwrap(),
        "<DATE=2024><TRACKNUMBER=3><_CUSTOM=stored>"
    );
    assert_eq!(
        evaluate_with_context("$meta(year)|%track%", &context).unwrap(),
        "2024|3"
    );
}

#[test]
fn filename_mode_preserves_documented_optional_directories() {
    let fields: Fields = [
        ("DISCNUMBER".into(), vec!["2".into()]),
        ("TITLE".into(), vec!["猫:Song".into()]),
    ]
    .into();
    let context = EvalContext::new(&fields);
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "[CD$num(%discnumber%,1)\\]$replace(%title%,':','-')",
            &context
        )
        .unwrap(),
        "CD2\\猫-Song"
    );
    let mut without_disc = fields.clone();
    without_disc.remove("DISCNUMBER");
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "[CD$num(%discnumber%,1)\\]%title%",
            &EvalContext::new(&without_disc)
        )
        .unwrap(),
        "猫_Song"
    );
}

#[test]
fn filename_mode_sanitizes_derived_values_after_evaluation() {
    let fields: Fields = [
        ("ARTIST".into(), vec!["AC/DC".into()]),
        ("TITLE".into(), vec!["猫:Song".into()]),
    ]
    .into();
    let context = EvalContext::new(&fields);
    assert_eq!(
        tag_core::evaluate_filename_with_context("%artist%\\%title%", &context).unwrap(),
        "AC_DC\\猫_Song"
    );
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "$replace(%artist%,'/','-')\\$replace(%title%,':','-')",
            &context
        )
        .unwrap(),
        "AC-DC\\猫-Song"
    );
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "$if($eql(%artist%,'AC/DC'),Correct,Wrong)\\%title%",
            &context
        )
        .unwrap(),
        "Correct\\猫_Song"
    );
    assert_eq!(
        tag_core::evaluate_filename_with_context("$replace(%artist%,AC,Label)", &context).unwrap(),
        "Label_DC"
    );
}

#[test]
fn filename_mode_keeps_selected_literal_conditional_path_branches() {
    let fields: Fields = [
        ("ARTIST".into(), vec!["Björk".into()]),
        ("DISCNUMBER".into(), vec!["2".into()]),
    ]
    .into();
    let context = EvalContext::new(&fields);
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "$if(%discnumber%,CD%discnumber%\\,Single\\)%artist%",
            &context
        )
        .unwrap(),
        "CD2\\Björk"
    );
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "$ifgreater(%discnumber%,1,Multi\\,Single\\)%artist%",
            &context
        )
        .unwrap(),
        "Multi\\Björk"
    );
    assert_eq!(
        tag_core::evaluate_filename_with_context(
            "$iflonger(%artist%,1,Artists\\,Other\\)%artist%",
            &context
        )
        .unwrap(),
        "Artists\\Björk"
    );
    assert_eq!(
        tag_core::evaluate_filename_with_context("$if2(%missing%,Default\\)%artist%", &context)
            .unwrap(),
        "Default\\Björk"
    );
    // Non-conditional functions produce components; they do not smuggle new
    // directories through argument text. Record this policy for differential RE.
    assert_eq!(
        tag_core::evaluate_filename_with_context("$upper('folder/file')", &context).unwrap(),
        "FOLDER_FILE"
    );
}

#[test]
fn remaining_literal_string_and_metadata_forms() {
    assert_eq!(
        evaluate(
            "$lower(ÄBC)|$upper(äbc)|$trim(' x ')|$meta(artist)|$meta(artist,0)|$meta_sep(artist,)",
            &fields()
        )
        .unwrap(),
        "äbc|ÄBC|x|Björk, Another|Björk|BjörkAnother"
    );
    let fields = fields();
    let mut context = EvalContext::new(&fields);
    context.ansi_codepage = Some("1252".into());
    assert_eq!(
        evaluate_with_context("$ansi('Björk 猫')", &context).unwrap(),
        "Björk ?"
    );
}

#[test]
fn ansi_conversion_roundtrips_encoded_glyphs_instead_of_assuming_identity() {
    let fields = Fields::new();
    let mut context = EvalContext::new(&fields);
    context.ansi_codepage = Some("932".into());
    assert_eq!(
        evaluate_with_context("$ansi('¥‾猫')", &context).unwrap(),
        "\\~猫"
    );
}

#[test]
fn regexp_ignores_case_by_default_and_honours_the_flag_and_inline_modifiers() {
    // scripting: c=1 enables ignore case, c=0 disables the ignore-case setting.
    for (script, expected) in [
        ("$regexp('Live At Home','live',x)", "x At Home"),
        ("$regexp('Live At Home','live',x,1)", "x At Home"),
        ("$regexp('Live At Home','live',x,0)", "Live At Home"),
        ("$regexp('Live At Home','(?-i)live',x)", "Live At Home"),
        ("$regexp('Live At Home','(?-i)Live',x)", "x At Home"),
        ("$regexp('Live At Home','(?i)LIVE',x,0)", "x At Home"),
        ("$regexp('Live at home','(?-i)(?i:AT) home',x)", "Live x"),
        ("$regexp('ABC abc','abc',x)", "x x"),
        ("$regexp('ABC abc','abc',x,0)", "ABC x"),
    ] {
        assert_eq!(evaluate(script, &fields()).unwrap(), expected, "{script}");
    }
}

#[test]
fn working_directory_placeholders_come_from_the_context_and_reach_filters() {
    let fields = fields();
    let mut context = EvalContext::new(&fields);
    assert_eq!(
        evaluate_with_context("[%_workingdir%]|[%_workingpath%]", &context).unwrap(),
        "|"
    );
    context.working_directory = Some("/music/rock/");
    assert_eq!(
        evaluate_with_context("%_workingdir%|%_WorkingPath%", &context).unwrap(),
        format!("rock|/music/rock{}", std::path::MAIN_SEPARATOR)
    );
    context.working_directory = Some("C:\\Music\\Jazz");
    assert_eq!(
        evaluate_with_context("%_workingdir%|%_workingpath%", &context).unwrap(),
        "Jazz|C:\\Music\\Jazz\\"
    );
    context.working_directory = Some("/");
    assert_eq!(
        evaluate_with_context("[%_workingdir%]%_workingpath%", &context).unwrap(),
        "/"
    );
    context.working_directory = Some("/music/rock");
    let filter = tag_core::compile_filter("_WORKINGDIR IS rock").unwrap();
    assert!(filter.matches_with_context(&context).unwrap());
}
