use tag_core::converters::{FilenameConverter, TextImportOptions, TextTagConverter};
use tag_core::{CompiledFilenamePattern, EvalContext, Fields};

fn paths() -> Vec<String> {
    vec!["/music/first.mp3".into(), "/music/second.mp3".into()]
}

#[test]
fn filename_rearrangement_retains_extension_and_uses_numbered_parts() {
    let converter = FilenameConverter::compile("%1 %2 - %3", "%2 - %3").unwrap();
    let fields = Fields::new();
    let result = converter
        .render("/music/001 Artist - Title.MP3", &EvalContext::new(&fields))
        .unwrap();
    assert_eq!(result.target, "Artist - Title.MP3");
    assert_eq!(result.captures["1"], vec!["001"]);
    assert_eq!(result.captures["3"], vec!["Title"]);
}

#[test]
fn filename_directory_captures_and_quoted_literals_follow_documented_example() {
    let converter = FilenameConverter::compile(
        r"%1 - %2\%3 %4 (%5_EDIT)",
        r"D:\Music\$upper(%1) - %2 '['%5']'\%3. %4 (EDIT)",
    )
    .unwrap();
    let fields = Fields::new();
    let result = converter
        .render(
            r"D:\Music\Artist - Album\01 Track Title (2025_EDIT).mp3",
            &EvalContext::new(&fields),
        )
        .unwrap();
    assert_eq!(
        result.target,
        r"D:\Music\ARTIST - Album [2025]\01. Track Title (EDIT).mp3"
    );
}

#[test]
fn filename_context_and_optional_paths_evaluate_captures_before_sanitizing() {
    let converter = FilenameConverter::compile(
        "%1 - %2",
        r"[CD$num(%discnumber%,1)\]$replace(%1,':','-')\%2",
    )
    .unwrap();
    let fields: Fields = [("DISCNUMBER".into(), vec!["2".into()])].into();
    let result = converter
        .render("Artist: Name - 猫.mp3", &EvalContext::new(&fields))
        .unwrap();
    assert_eq!(result.target, r"CD2\Artist- Name\猫.mp3");
}

#[test]
fn compiled_filename_converter_reuses_values_without_stale_captures() {
    let converter = FilenameConverter::compile("%1 - %2", "$upper(%2) - %1").unwrap();
    let fields = Fields::new();
    for (source, expected) in [
        ("A - first.flac", "FIRST - A.flac"),
        ("B - second.ogg", "SECOND - B.ogg"),
    ] {
        assert_eq!(
            converter
                .render(source, &EvalContext::new(&fields))
                .unwrap()
                .target,
            expected
        );
    }
    assert!(converter
        .render("bad.mp3", &EvalContext::new(&fields))
        .is_err());
}

#[test]
fn invalid_filename_masks_and_unknown_numbered_references_fail() {
    for (old, new) in [
        ("%0 - %1", "%1"),
        ("%10 - %2", "%2"),
        ("%1%2", "%1"),
        ("%1 - %2", "%3"),
        ("no captures", "new"),
        ("%1 - %2", "$unknown(%1)"),
    ] {
        assert!(
            FilenameConverter::compile(old, new).is_err(),
            "{old} => {new}"
        );
    }
}

#[test]
fn positional_text_import_keeps_selected_order_and_never_drops_dots_or_paths() {
    let converter = TextTagConverter::compile("%track%. %artist% - [%album%] %title%").unwrap();
    let plans = converter
        .plan(
            "1. Sigur Rós - [Ágætis byrjun] Intro.mp3\r\n2. 猫 - [A/B] First/Second.flac\r\n",
            &paths(),
            &TextImportOptions::default(),
        )
        .unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].path, paths()[0]);
    assert_eq!(plans[1].selection_index, 1);
    assert_eq!(plans[1].source_line, 2);
    assert_eq!(plans[0].fields["TITLE"], vec!["Intro.mp3"]);
    assert_eq!(plans[1].fields["TITLE"], vec!["First/Second.flac"]);
    assert_eq!(plans[1].fields["ALBUM"], vec!["A/B"]);
}

#[test]
fn text_separators_and_quotes_are_mask_literals_without_implicit_csv_decoding() {
    let converter = TextTagConverter::compile("\"%artist%\";\"%title%\"").unwrap();
    let plans = converter
        .plan(
            "\"A; B\";\"A \"\"quote\"\"\"",
            &paths()[..1],
            &TextImportOptions::default(),
        )
        .unwrap();
    assert_eq!(plans[0].fields["ARTIST"], vec!["A; B"]);
    assert_eq!(plans[0].fields["TITLE"], vec!["A \"\"quote\"\""]);
    let converter = TextTagConverter::compile("%dummy%,%artist%,%title%").unwrap();
    let plans = converter
        .plan(
            "87,Sigur Rós,Intro",
            &paths()[..1],
            &TextImportOptions::default(),
        )
        .unwrap();
    assert!(!plans[0].fields.contains_key("DUMMY"));
    assert_eq!(plans[0].fields["ARTIST"], vec!["Sigur Rós"]);
}

#[test]
fn text_import_boms_codepages_and_all_line_endings_are_decoded_strictly() {
    let converter = TextTagConverter::compile("%title%").unwrap();
    let utf16: Vec<_> = [0xFF, 0xFE]
        .into_iter()
        .chain("猫\rSecond\r".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let plans = converter
        .plan_bytes(&utf16, &paths(), &TextImportOptions::default())
        .unwrap();
    assert_eq!(plans[0].fields["TITLE"], vec!["猫"]);
    assert_eq!(plans[1].fields["TITLE"], vec!["Second"]);
    let options = TextImportOptions {
        ansi_codepage: Some("1252".into()),
        ..Default::default()
    };
    let plans = converter
        .plan_bytes(b"Caf\xe9\nSecond", &paths(), &options)
        .unwrap();
    assert_eq!(plans[0].fields["TITLE"], vec!["Café"]);
    assert!(converter
        .plan_bytes(b"\xff\xfe\x00", &paths()[..1], &options)
        .is_err());
    // Without a system ANSI codepage, legacy text is Windows-1252.
    assert_eq!(
        converter
            .plan_bytes(b"\xff", &paths()[..1], &TextImportOptions::default())
            .unwrap()[0]
            .fields["TITLE"],
        vec!["ÿ"]
    );
    assert!(converter
        .plan_bytes(
            b"\x81",
            &paths()[..1],
            &TextImportOptions {
                ansi_codepage: Some("932".into()),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(
        converter
            .plan("\u{feff}First\nSecond", &paths(), &options)
            .unwrap()[0]
            .fields["TITLE"],
        vec!["First"]
    );
}

#[test]
fn filename_keyed_text_import_matches_reordered_rows_without_storing_the_key() {
    let converter = TextTagConverter::compile("%_filename_ext%|%title%").unwrap();
    let plans = converter
        .plan(
            "second.mp3|Two\nfirst.mp3|One",
            &paths(),
            &TextImportOptions::default(),
        )
        .unwrap();
    assert_eq!(plans[0].selection_index, 1);
    assert_eq!(plans[0].fields["TITLE"], vec!["Two"]);
    assert!(!plans[0].fields.contains_key("_FILENAME_EXT"));
    assert_eq!(plans[1].selection_index, 0);
}

#[test]
fn absolute_path_keys_disambiguate_duplicate_filenames_and_normalize_separators() {
    let converter = TextTagConverter::compile("%_path%|%artist%").unwrap();
    let selected = vec![r"C:\a\same.mp3".into(), r"C:\b\same.mp3".into()];
    let plans = converter
        .plan(
            "C:/b/same.mp3|B\nC:/a/same.mp3|A",
            &selected,
            &TextImportOptions::default(),
        )
        .unwrap();
    assert_eq!(plans[0].selection_index, 1);
    let converter = TextTagConverter::compile("%_filename_ext%|%artist%").unwrap();
    assert!(converter
        .plan(
            "same.mp3|B\nsame.mp3|A",
            &selected,
            &TextImportOptions::default()
        )
        .is_err());
}

#[test]
fn text_matching_errors_are_atomic_and_report_line_numbers() {
    let converter = TextTagConverter::compile("%artist%|%title%").unwrap();
    let error = converter
        .plan("A|Good\nmalformed", &paths(), &TextImportOptions::default())
        .unwrap_err();
    assert!(error.contains("line 2"), "{error}");
    for text in ["A|One", "A|One\nB|Two\nC|Three", "A|One\n\nB|Two"] {
        assert!(converter
            .plan(text, &paths(), &TextImportOptions::default())
            .is_err());
    }
    let keyed = TextTagConverter::compile("%_filename_ext%|%title%").unwrap();
    for text in [
        "first.mp3|One\nfirst.mp3|Two",
        "unknown.mp3|One\nsecond.mp3|Two",
        "first.mp3|One",
    ] {
        assert!(keyed
            .plan(text, &paths(), &TextImportOptions::default())
            .is_err());
    }
}

#[test]
fn empty_values_repeated_fields_and_literal_special_characters_are_preserved() {
    let converter = TextTagConverter::compile("[%artist%] '%title%'|%artist%").unwrap();
    let plans = converter
        .plan("[A] ''|B", &paths()[..1], &TextImportOptions::default())
        .unwrap();
    assert_eq!(plans[0].fields["ARTIST"], vec!["A", "B"]);
    assert_eq!(plans[0].fields["TITLE"], vec![""]);
    assert!(TextTagConverter::compile("%artist%%title%").is_err());
    assert!(TextTagConverter::compile("%_codec%|%title%").is_err());
    assert!(TextTagConverter::compile("%_filename_ext%|%_path%|%title%").is_err());
    assert!(TextTagConverter::compile("%title").is_err());
}

#[test]
fn all_nine_filename_parts_and_quoted_numbered_literals_are_distinct() {
    let converter = FilenameConverter::compile(
        "%1-%2-%3-%4-%5-%6-%7-%8-%9",
        "%9-%8-%7-%6-%5-%4-%3-%2-%1 '%9' ''",
    )
    .unwrap();
    let fields = Fields::new();
    assert_eq!(
        converter
            .render("A-B-C-D-E-F-G-H-猫.flac", &EvalContext::new(&fields))
            .unwrap()
            .target,
        "猫-H-G-F-E-D-C-B-A %9 '.flac"
    );
    let converter = FilenameConverter::compile("'%1'", "%1").unwrap();
    assert_eq!(
        converter
            .render("'No extension'", &EvalContext::new(&fields))
            .unwrap()
            .target,
        "No extension"
    );
    assert!(FilenameConverter::compile("%1 - %1", "%1").is_err());
}

#[test]
fn compiled_filename_matcher_preserves_the_existing_public_contract() {
    let fixtures = [
        (
            "%artist% - %title%",
            "/music/A - Track.with.dots.mp3",
            vec![("ARTIST", vec!["A"]), ("TITLE", vec!["Track.with.dots"])],
        ),
        (
            r"%artist%\%title%",
            "/music/猫/Track.mp3",
            vec![("ARTIST", vec!["猫"]), ("TITLE", vec!["Track"])],
        ),
        (
            r"C:\%artist%\%title%.mp3",
            r"C:\A\Track.mp3",
            vec![("ARTIST", vec!["A"]), ("TITLE", vec!["Track"])],
        ),
        (
            "[%artist%] '%title%'",
            "[A] 'Track'.flac",
            vec![("ARTIST", vec!["A"]), ("TITLE", vec!["Track"])],
        ),
        (
            "%artist%|%artist%|%dummy%",
            "A|B|unused.ogg",
            vec![("ARTIST", vec!["A", "B"])],
        ),
    ];
    for (mask, source, expected) in fixtures {
        let matcher = CompiledFilenamePattern::compile(mask).unwrap();
        let expected: Fields = expected
            .into_iter()
            .map(|(name, values)| {
                (
                    name.to_owned(),
                    values.into_iter().map(str::to_owned).collect(),
                )
            })
            .collect();
        assert_eq!(matcher.parse(source).unwrap(), expected);
        assert!(matcher.parse("unmatched.mp3").is_err());
    }
}

#[test]
fn filename_plans_preserve_original_context_and_do_not_mutate_raw_metadata() {
    let converter =
        FilenameConverter::compile("%1 - %2", "$getenv(PREFIX)\\%2 $list(,'=',';')").unwrap();
    let fields: Fields = [
        ("ARTIST".into(), vec!["Stored".into()]),
        ("1".into(), vec!["Tag numeric field".into()]),
    ]
    .into();
    let raw: Fields = [("ARTIST".into(), vec!["Raw".into()])].into();
    let environment = [("PREFIX".into(), "Collection".into())].into();
    let mut context = EvalContext::new(&fields);
    context.raw_fields = Some(&raw);
    context.environment = Some(&environment);
    assert_eq!(
        converter
            .render("Captured - Title.mp3", &context)
            .unwrap()
            .target,
        "Collection\\Title ARTIST=Raw;.mp3"
    );
    assert_eq!(fields["1"], vec!["Tag numeric field"]);
}

#[test]
fn text_import_case_matching_is_explicit_and_missing_targets_never_fall_back_to_position() {
    let converter = TextTagConverter::compile("%_filename_ext%|%title%").unwrap();
    let text = "SECOND.MP3|Two\nFIRST.MP3|One";
    assert!(converter
        .plan(text, &paths(), &TextImportOptions::default())
        .is_err());
    let options = TextImportOptions {
        case_sensitive_paths: false,
        ..Default::default()
    };
    let plans = converter.plan(text, &paths(), &options).unwrap();
    assert_eq!(plans[0].selection_index, 1);
    assert_eq!(plans[1].selection_index, 0);
    let duplicate_paths = vec!["/music/first.mp3".into(), "/MUSIC/FIRST.MP3".into()];
    assert!(converter.plan(text, &duplicate_paths, &options).is_err());
    let converter = TextTagConverter::compile("%_path%|%title%").unwrap();
    assert!(converter
        .plan("relative.mp3|Title", &paths()[..1], &options)
        .is_err());
}

#[test]
fn text_blank_records_and_trailing_terminators_do_not_shift_selected_rows() {
    let converter = TextTagConverter::compile("%title%").unwrap();
    let plans = converter
        .plan("\r\nSecond\r\n", &paths(), &TextImportOptions::default())
        .unwrap();
    assert_eq!(plans[0].fields["TITLE"], vec![""]);
    assert_eq!(plans[1].fields["TITLE"], vec!["Second"]);
    assert!(converter
        .plan("First\n\nSecond", &paths(), &TextImportOptions::default())
        .is_err());
    assert!(converter
        .plan("", &[], &TextImportOptions::default())
        .unwrap()
        .is_empty());
    assert!(converter
        .plan("data", &[], &TextImportOptions::default())
        .is_err());
    assert!(converter
        .plan("First\0", &paths()[..1], &TextImportOptions::default())
        .is_err());
    assert!(converter
        .plan(
            &"x".repeat(1_048_577),
            &paths()[..1],
            &TextImportOptions::default()
        )
        .is_err());
}

#[test]
fn large_keyed_import_indexes_the_selection_and_matches_every_reversed_record() {
    let count = 20_000;
    let converter = TextTagConverter::compile("%_filename_ext%;%title%").unwrap();
    let selected: Vec<String> = (0..count)
        .map(|index| format!("/music/{index}.mp3"))
        .collect();
    let source: String = (0..count)
        .rev()
        .map(|index| format!("{index}.mp3;Title {index}\n"))
        .collect();
    let started = std::time::Instant::now();
    let plans = converter
        .plan(&source, &selected, &TextImportOptions::default())
        .unwrap();
    eprintln!("Keyed text import: {count} rows in {:?}", started.elapsed());
    assert_eq!(plans.len(), count);
    for (line, plan) in plans.iter().enumerate() {
        let index = count - line - 1;
        assert_eq!(plan.selection_index, index);
        assert_eq!(plan.path, selected[index]);
        assert_eq!(plan.fields["TITLE"], vec![format!("Title {index}")]);
    }
}
