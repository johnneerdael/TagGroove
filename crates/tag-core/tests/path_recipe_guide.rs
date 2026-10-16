//! Execute written naming recipes and the cover-export action request.
//! Native external-image writes/undo are source-reviewed, not reproduced here.
mod guide_examples;
use tag_core::{
    actions::{apply_action_spec, ActionEffect, ActionSpec, ActionState},
    evaluate, evaluate_filename_with_context, EvalContext, Fields, FilenameConverter,
};
const LOWER: &str = include_str!("../../../docs/user-guide/cookbook/rename-lowercase-filenames.md");
const PORTABLE: &str = include_str!("../../../docs/user-guide/cookbook/rename-safe-characters.md");
const COVER: &str = include_str!("../../../docs/user-guide/cookbook/export-folder-jpg.md");
fn forms(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    [lf.clone(), lf.replace('\n', "\r\n")]
}
fn section<'a>(page: &'a str, marker: &str) -> &'a str {
    page.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
}
fn block(page: &str, marker: &str) -> String {
    let blocks = guide_examples::text_blocks(section(page, marker));
    assert_eq!(blocks.len(), 1);
    blocks[0].clone()
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    section(page, marker)
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(s: &str) -> &str {
    s.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
#[test]
fn actual_filename_converter_uses_written_whole_basename_capture_and_preserves_extension_and_tags()
{
    for page in forms(LOWER) {
        let converter = FilenameConverter::compile(
            &block(&page, "lowercase-old-pattern"),
            &block(&page, "lowercase-new-pattern"),
        )
        .unwrap();
        let fields = Fields::from([
            ("TITLE".into(), vec!["KEEP My Title".into()]),
            ("COMMENT".into(), vec!["Keep this too".into()]),
        ]);
        let before = fields.clone();
        let cases = rows(&page, "lowercase-name-cases");
        assert_eq!(cases.len(), 4);
        for row in cases {
            let rendered = converter
                .render(code(row[0]), &EvalContext::new(&fields))
                .unwrap();
            assert_eq!(rendered.target, code(row[1]));
        }
        assert_eq!(fields, before);
    }
}
#[test]
fn written_collapsing_and_tag_based_variations_use_their_distinct_actual_converter_contexts() {
    for page in forms(LOWER) {
        let fields = Fields::from([
            ("TRACK".into(), vec!["3/8".into()]),
            ("TITLE".into(), vec!["Paper Boats".into()]),
        ]);
        let converter = FilenameConverter::compile(
            &block(&page, "lowercase-old-pattern"),
            &block(&page, "lowercase-collapsed-pattern"),
        )
        .unwrap();
        assert_eq!(
            converter
                .render("06 Paper  Boats.mp3", &EvalContext::new(&fields))
                .unwrap()
                .target,
            "06-paper-boats.mp3"
        );
        assert_eq!(
            evaluate_filename_with_context(
                &block(&page, "lowercase-tag-pattern"),
                &EvalContext::new(&fields)
            )
            .unwrap(),
            "03-paper-boats"
        );
    }
}
#[test]
fn written_component_cleanup_replaces_its_exact_characters_and_trims_only_the_resulting_end() {
    for page in forms(PORTABLE) {
        let template = block(&page, "portable-name-format");
        let cases = rows(&page, "portable-name-cases");
        assert_eq!(cases.len(), 5);
        for row in cases {
            let fields = Fields::from([
                ("TRACK".into(), vec!["3/8".into()]),
                ("TITLE".into(), vec![code(row[0]).replace('·', " ")]),
            ]);
            let name =
                evaluate_filename_with_context(&template, &EvalContext::new(&fields)).unwrap();
            assert_eq!(format!("{name}.mp3"), code(row[1]));
        }
        let fields = Fields::from([("TITLE".into(), vec!["A/B".into()])]);
        assert_eq!(evaluate("$validate(%title%,)", &fields).unwrap(), "AB");
        // The public text function and filename mode deliberately have different scopes.
        let fields = Fields::from([("TITLE".into(), vec!["A\\B\tC".into()])]);
        assert_eq!(
            evaluate("$validate(%title%,-)", &fields).unwrap(),
            "A\\B\tC"
        );
        assert_eq!(
            evaluate_filename_with_context("$validate(%title%,-)", &EvalContext::new(&fields))
                .unwrap(),
            "A_B_C"
        );
    }
}
#[test]
fn written_shortening_policy_counts_unicode_scalars_without_claiming_filesystem_byte_limits() {
    for page in forms(PORTABLE) {
        let template = block(&page, "portable-short-format");
        let title = "é".repeat(81);
        let fields = Fields::from([
            ("TRACK".into(), vec!["3/8".into()]),
            ("TITLE".into(), vec![title]),
        ]);
        let output = evaluate_filename_with_context(&template, &EvalContext::new(&fields)).unwrap();
        assert_eq!(output, format!("03 - {}", "é".repeat(80)));
        assert_eq!(output.chars().count(), 85);
        assert!(output.len() > 85);
        let title = format!("{}..Rest", "a".repeat(78));
        let fields = Fields::from([
            ("TRACK".into(), vec!["3/8".into()]),
            ("TITLE".into(), vec![title]),
        ]);
        assert_eq!(
            evaluate_filename_with_context(&template, &EvalContext::new(&fields)).unwrap(),
            format!("03 - {}", "a".repeat(78))
        );
    }
}
#[test]
fn written_front_cover_export_settings_and_per_track_format_produce_actual_host_requests() {
    for page in forms(COVER) {
        let settings: std::collections::BTreeMap<_, _> = rows(&page, "folder-export-settings")
            .into_iter()
            .map(|r| (r[0], r[1]))
            .collect();
        assert_eq!(settings["Export covers of type"], "Front Cover");
        assert_eq!(settings["Export duplicate covers"], "Off");
        let mut state = ActionState {
            fields: Fields::from([("TITLE".into(), vec!["Paper Boats".into()])]),
            context: Fields::from([("_FILENAME".into(), vec!["03 - Paper Boats".into()])]),
        };
        let before = state.fields.clone();
        for (template, expected) in [
            (
                code(settings["Output filename format (extension is added)"]).to_string(),
                "folder",
            ),
            (
                block(&page, "folder-export-per-track"),
                "03 - Paper Boats-cover",
            ),
        ] {
            let effects = apply_action_spec(
                &mut state,
                &ActionSpec::ExportCover {
                    filename_template: template,
                    picture_type: Some("CoverFront".into()),
                    export_duplicates: false,
                },
            )
            .unwrap();
            assert_eq!(
                effects,
                [ActionEffect::ExportCover {
                    filename: expected.into(),
                    picture_type: Some("CoverFront".into()),
                    export_duplicates: false
                }]
            );
            assert_eq!(state.fields, before);
        }
        assert_eq!(rows(&page, "folder-export-name-cases").len(), 5);
        // No extension/write/undo is simulated: those are NativeHost responsibilities.
    }
}
