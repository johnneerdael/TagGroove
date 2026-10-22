//! Check initial-publication prose and fenced examples beyond the table tests.
mod guide_examples;
use guide_examples::text_blocks as blocks;

use tag_core::{
    compile_filter, evaluate, evaluate_filename_with_context, parse_filename, EvalContext, Fields,
};

const FORMAT: &str = include_str!("../../../docs/user-guide/scripting/format-strings.md");
const REGEX: &str = include_str!("../../../docs/user-guide/scripting/regular-expressions.md");
const FIRST: &str = include_str!("../../../docs/user-guide/getting-started/first-tagging-session.md");
const NUMBERS: &str =
    include_str!("../../../docs/user-guide/converters/recipes/rename-zero-padded-track-numbers.md");
const FOLDERS: &str =
    include_str!("../../../docs/user-guide/converters/recipes/tags-from-folder-and-filename.md");
const GUESTS: &str = include_str!("../../../docs/user-guide/actions/recipes/feat-artists-to-title.md");

fn fields() -> Fields {
    [
        ("ARTIST".into(), vec!["Juniper Vale".into()]),
        ("TITLE".into(), vec!["Lanterns on the Water".into()]),
        ("ALBUM".into(), vec!["Harbour Lights".into()]),
        ("YEAR".into(), vec!["2024".into()]),
        ("TRACK".into(), vec!["2/8".into()]),
        ("GENRE".into(), vec!["Indie Folk".into()]),
    ]
    .into()
}

fn newline_variants(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}

#[test]
fn inline_empty_pattern_note_produces_its_own_written_result() {
    let line = REGEX
        .lines()
        .find(|line| line.trim_start().starts_with("`$regexp(ab,"))
        .unwrap();
    let code: Vec<_> = line.split('`').collect();
    assert_eq!(
        evaluate(code[1], &Fields::new()).unwrap(),
        code[3],
        "{line}"
    );
}

#[test]
fn guarded_guest_expression_handles_the_regex_default_case_rule_and_fallback() {
    for regex_page in newline_variants(REGEX) {
        check_guarded_guest_expression_handles_the_regex_default_case_rule_and_fallback(
            &regex_page,
        );
    }
}

fn check_guarded_guest_expression_handles_the_regex_default_case_rule_and_fallback(
    regex_page: &str,
) {
    let expression = blocks(regex_page)
        .into_iter()
        .find(|block| block.starts_with("$if("))
        .unwrap();
    let mut values = fields();
    for title in [
        "Paper Boats (feat. Mira Solberg)",
        "Paper Boats (FEAT. Mira Solberg)",
    ] {
        values.insert("TITLE".into(), vec![title.into()]);
        assert_eq!(
            evaluate(&expression, &values).unwrap(),
            "Mira Solberg",
            "{title}"
        );
    }
    values.insert("TITLE".into(), vec!["Paper Boats".into()]);
    assert_eq!(evaluate(&expression, &values).unwrap(), "Juniper Vale");
}

#[test]
fn every_format_reference_fence_runs_in_its_written_context() {
    for format_page in newline_variants(FORMAT) {
        check_every_format_reference_fence_runs_in_its_written_context(&format_page);
    }
}

fn check_every_format_reference_fence_runs_in_its_written_context(format_page: &str) {
    let examples = blocks(format_page);
    assert_eq!(examples.len(), 10);
    let mut values = fields();
    for (index, expected) in [(0, "Lanterns"), (1, "LANTERNS"), (3, "no")] {
        assert_eq!(evaluate(&examples[index], &values).unwrap(), expected);
    }
    for (index, expected) in [
        (4, r"Juniper Vale\Harbour Lights\02 - Lanterns on the Water"),
        (6, "02 Lanterns on the Water"),
        (
            7,
            r"Juniper Vale\Harbour Lights (2024)\02 - Lanterns on the Water",
        ),
        (8, r"J\Juniper Vale\Harbour Lights"),
    ] {
        assert_eq!(
            evaluate_filename_with_context(&examples[index], &EvalContext::new(&values)).unwrap(),
            expected
        );
    }
    let result = parse_filename("03 - Juniper Vale - Paper Boats.mp3", &examples[5]).unwrap();
    assert_eq!(result["TRACK"], ["03"]);
    assert_eq!(result["ARTIST"], ["Juniper Vale"]);
    assert_eq!(result["TITLE"], ["Paper Boats"]);
    values.insert("TITLE".into(), vec!["Fog Horn (Interlude)".into()]);
    assert_eq!(evaluate(&examples[2], &values).unwrap(), "Fog Horn");
    for (seconds, expected) in [(300, ""), (301, "long")] {
        values.insert("_LENGTH_SECONDS".into(), vec![seconds.to_string()]);
        assert_eq!(evaluate(&examples[9], &values).unwrap(), expected);
    }
}

#[test]
fn inline_optional_quote_whitespace_and_numeric_examples_are_literal() {
    let mut values = fields();
    for (example, expected) in [
        ("$replace(%title%, on, at)", "Lanterns at the Water"),
        ("$replace(%title%,on,at)", "Lanterns at the Water"),
        ("[CD$num(%discnumber%,1)]", ""),
        (
            "[%album%[ (%year%)] - ]%title%",
            "Harbour Lights (2024) - Lanterns on the Water",
        ),
        ("$upper([%year%, ]%album%)", "2024, HARBOUR LIGHTS"),
        ("%artist%''s best", "Juniper Vale's best"),
        ("$strstr(%title%,Water)", "17"),
        ("$num(-3,3)", "-03"),
        ("$num(' 3/8',2)", "03"),
    ] {
        assert!(FORMAT.contains(example), "example disappeared: {example}");
        assert_eq!(evaluate(example, &values).unwrap(), expected, "{example}");
    }
    values.insert("ARTIST".into(), vec!["Kofi & the Tidewater Band".into()]);
    let example = "$replace(%artist%,' & ',', ')";
    assert!(FORMAT.contains(example));
    assert_eq!(
        evaluate(example, &values).unwrap(),
        "Kofi, the Tidewater Band"
    );
    values.insert("TITLE".into(), vec!["Fog Horn (Interlude)".into()]);
    let nested_literal = "$replace(%title%, (Interlude),)";
    assert!(FORMAT.contains(nested_literal));
    assert_eq!(evaluate(nested_literal, &values).unwrap(), "Fog Horn");
    for invalid in ["%%", "%title", "%artist%'s best"] {
        assert!(FORMAT.contains(invalid));
        assert!(
            evaluate(invalid, &values).is_err(),
            "documented error accepted: {invalid}"
        );
    }
}

#[test]
fn remaining_regex_fences_preserve_capture_quotes_and_empty_replacements() {
    for regex_page in newline_variants(REGEX) {
        check_remaining_regex_fences_preserve_capture_quotes_and_empty_replacements(&regex_page);
    }
}

fn check_remaining_regex_fences_preserve_capture_quotes_and_empty_replacements(regex_page: &str) {
    let examples = blocks(regex_page);
    assert_eq!(examples.len(), 6);
    // The literal Input/Pattern/Replacement/Result fence is exercised through
    // the actual regex action in user_guide.rs; these are the scripting fences.
    let examples: Vec<_> = examples
        .into_iter()
        .filter(|block| block.starts_with('$'))
        .collect();
    assert_eq!(examples.len(), 5);
    let mut values = fields();
    for (index, title, expected) in [
        (0, "03_Paper Boats", "Paper Boats"),
        (0, "Paper Boats", "Paper Boats"),
        (1, "03_Paper Boats", "Paper Boats [03]"),
        (2, "the keeper's song", "The Keeper's Song"),
        (3, "Paper Boats (feat. Mira Solberg)", "Paper Boats"),
    ] {
        values.insert("TITLE".into(), vec![title.into()]);
        assert_eq!(evaluate(&examples[index], &values).unwrap(), expected);
    }
}

#[test]
fn first_session_and_number_recipe_fences_match_their_tutorial_outputs() {
    for (first_page, numbers_page) in newline_variants(FIRST)
        .into_iter()
        .zip(newline_variants(NUMBERS))
    {
        check_first_session_and_number_recipe_fences_match_their_tutorial_outputs(
            &first_page,
            &numbers_page,
        );
    }
}

fn check_first_session_and_number_recipe_fences_match_their_tutorial_outputs(
    first_page: &str,
    numbers_page: &str,
) {
    let values = fields();
    let first = blocks(first_page);
    assert_eq!(first.len(), 1);
    assert_eq!(
        evaluate(&first[0], &values).unwrap(),
        "02 - Juniper Vale - Lanterns on the Water"
    );
    let examples = blocks(numbers_page);
    assert_eq!(examples.len(), 5);
    for (index, expected) in [
        (0, "02 Lanterns on the Water"),
        (2, "002 Lanterns on the Water"),
        (3, "02 Lanterns on the Water"),
        (4, "02 - Juniper Vale - Lanterns on the Water"),
    ] {
        assert_eq!(evaluate(&examples[index], &values).unwrap(), expected);
    }
    let filter = compile_filter(&examples[1]).unwrap();
    assert!(!filter.matches(&values).unwrap());
    let mut missing = values.clone();
    missing.remove("TRACK");
    assert!(filter.matches(&missing).unwrap());
    missing.insert("TRACK".into(), vec!["B2".into()]);
    assert!(filter.matches(&missing).unwrap());
    let mut disc = values;
    disc.insert("DISCNUMBER".into(), vec!["2".into()]);
    disc.insert("TRACK".into(), vec!["5".into()]);
    disc.insert("TITLE".into(), vec!["Salt & Cedar".into()]);
    assert_eq!(evaluate(&examples[3], &disc).unwrap(), "2-05 Salt & Cedar");
}

#[test]
fn every_folder_recipe_pattern_runs_including_its_inline_deluxe_variation() {
    for folders_page in newline_variants(FOLDERS) {
        check_every_folder_recipe_pattern_runs_including_its_inline_deluxe_variation(&folders_page);
    }
}

fn check_every_folder_recipe_pattern_runs_including_its_inline_deluxe_variation(
    folders_page: &str,
) {
    let examples = blocks(folders_page);
    assert_eq!(examples.len(), 8); // Two blocks illustrate a tree/alignment, not copyable patterns.
    for (index, path, key, expected) in [
        (
            1,
            r"Music\Juniper Vale\Harbour Lights (2024)\02 - Lanterns on the Water.mp3",
            "YEAR",
            vec!["2024"],
        ),
        (
            3,
            r"Music\Juniper Vale\Harbour Lights\02 - Lanterns on the Water.mp3",
            "ALBUM",
            vec!["Harbour Lights"],
        ),
        (
            4,
            r"Music\Juniper Vale\Harbour Lights\Paper Boats.mp3",
            "TITLE",
            vec!["Paper Boats"],
        ),
        (
            5,
            r"Music\Juniper Vale\Harbour Lights\CD2\02 - Lanterns on the Water.mp3",
            "DISCNUMBER",
            vec!["2"],
        ),
        (
            6,
            r"Music\Juniper Vale\Harbour Lights\Juniper Vale - 02 - Lanterns on the Water.mp3",
            "TRACK",
            vec!["02"],
        ),
        (
            7,
            "Juniper Vale & Mira Solberg - Paper Boats.mp3",
            "ARTIST",
            vec!["Juniper Vale", "Mira Solberg"],
        ),
    ] {
        let result = parse_filename(path, &examples[index]).unwrap();
        assert_eq!(result[key], expected, "{}: {path}", examples[index]);
        assert!(!result.contains_key("DUMMY"));
    }
    let path = r"Music\Juniper Vale\Harbour Lights (Deluxe) (2024)\02 - Lanterns on the Water.mp3";
    let result = parse_filename(path, &examples[1]).unwrap();
    assert_eq!(result["ALBUM"], ["Harbour Lights"]);
    assert_eq!(result["YEAR"], ["Deluxe) (2024"]);
    let pattern = r"%artist%\%album% (Deluxe) (%year%)\%track% - %title%";
    assert!(FOLDERS.contains(pattern));
    let result = parse_filename(path, pattern).unwrap();
    assert_eq!(result["ALBUM"], ["Harbour Lights"]);
    assert_eq!(result["YEAR"], ["2024"]);
}

#[test]
fn guest_recipe_filter_and_both_match_and_no_match_examples_are_explicit() {
    for guests_page in newline_variants(GUESTS) {
        check_guest_recipe_filter_and_both_match_and_no_match_examples_are_explicit(&guests_page);
    }
}

fn check_guest_recipe_filter_and_both_match_and_no_match_examples_are_explicit(guests_page: &str) {
    let examples = blocks(guests_page);
    assert_eq!(examples.len(), 3);
    let filter = compile_filter(&examples[0]).unwrap();
    let mut values = fields();
    values.insert("TITLE".into(), vec!["Paper Boats".into()]);
    for artist in [
        "Juniper Vale feat. Mira Solberg",
        "Juniper Vale FT Mira Solberg",
    ] {
        values.insert("ARTIST".into(), vec![artist.into()]);
        assert!(filter.matches(&values).unwrap());
        assert_eq!(
            evaluate(&examples[1], &values).unwrap(),
            "Paper Boats (feat. Mira Solberg)"
        );
        assert_eq!(
            regex_compat::replace(artist, &examples[2], "", false).unwrap(),
            "Juniper Vale"
        );
    }
    values.insert("TITLE".into(), vec!["Low Tide".into()]);
    values.insert("ARTIST".into(), vec!["Juniper Vale".into()]);
    assert!(!filter.matches(&values).unwrap());
    assert_eq!(
        evaluate(&examples[1], &values).unwrap(),
        "Low Tide (feat. Juniper Vale)"
    );
}

#[test]
fn written_fences_support_both_checkout_newlines_and_preserve_spaces() {
    for (page, count) in [
        (FORMAT, 10),
        (REGEX, 6),
        (FIRST, 1),
        (NUMBERS, 5),
        (FOLDERS, 8),
        (GUESTS, 3),
    ] {
        let lf = page.replace("\r\n", "\n");
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(blocks(&crlf).len(), count, "CRLF checkout");
        assert_eq!(blocks(&lf), blocks(&crlf));
    }
    for newline in ["\n", "\r\n"] {
        let page = ["    ```text", "      literal  ", "    ", "    ```"].join(newline);
        assert_eq!(blocks(&page), ["  literal  \n"]);
    }
}
