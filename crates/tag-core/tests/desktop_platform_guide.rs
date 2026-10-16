//! Execute platform guide expressions/known-codepage examples, not native OS actions.
mod guide_examples;
use tag_core::{
    decode_export_template, evaluate, evaluate_filename_with_context, EvalContext, Fields,
};
const MAC: &str = include_str!("../../../docs/user-guide/platforms/macos.md");
const WIN: &str = include_str!("../../../docs/user-guide/platforms/windows.md");
const LINUX: &str = include_str!("../../../docs/user-guide/platforms/linux.md");
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
    let values = guide_examples::text_blocks(section(page, marker));
    assert_eq!(values.len(), 1);
    values[0].clone()
}
fn code(s: &str) -> &str {
    s.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
#[test]
fn mac_guide_written_folder_format_uses_core_components_not_a_shell_or_native_rename() {
    for page in forms(MAC) {
        let fields = Fields::from([
            ("ARTIST".into(), vec!["Juniper Vale".into()]),
            ("ALBUM".into(), vec!["Harbour Lights".into()]),
            ("TRACK".into(), vec!["3/8".into()]),
            ("TITLE".into(), vec!["Paper Boats".into()]),
        ]);
        assert_eq!(
            evaluate_filename_with_context(
                &block(&page, "mac-name-format"),
                &EvalContext::new(&fields)
            )
            .unwrap(),
            "Juniper Vale\\Harbour Lights\\03 - Paper Boats"
        );
    }
}
#[test]
fn windows_guide_exact_hex_examples_distinguish_utf8_detection_from_known_legacy_fallback() {
    for page in forms(WIN) {
        let rows: Vec<_> = section(&page, "windows-encoding-cases")
            .lines()
            .filter(|line| line.starts_with("| `"))
            .map(|line| {
                line.trim_matches('|')
                    .split('|')
                    .map(str::trim)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(rows.len(), 2);
        for row in rows {
            let bytes: Vec<_> = code(row[0])
                .split_whitespace()
                .map(|hex| u8::from_str_radix(hex, 16).unwrap())
                .collect();
            assert_eq!(
                decode_export_template(&bytes, Some(code(row[1]))).unwrap(),
                code(row[2])
            );
        }
    }
}
#[test]
fn linux_guide_volume_display_falls_back_for_missing_or_empty_information_without_mounting() {
    for page in forms(LINUX) {
        let template = block(&page, "linux-volume-format");
        assert_eq!(evaluate(&template, &Fields::new()).unwrap(), "Unlabeled");
        assert_eq!(
            evaluate(
                &template,
                &Fields::from([("_VOLUME".into(), vec![String::new()])])
            )
            .unwrap(),
            "Unlabeled"
        );
        assert_eq!(
            evaluate(
                &template,
                &Fields::from([("_VOLUME".into(), vec!["Music Drive".into()])])
            )
            .unwrap(),
            "Music Drive"
        );
    }
}
