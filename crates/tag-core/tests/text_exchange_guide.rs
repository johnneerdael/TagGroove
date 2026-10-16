//! Execute the text-exchange recipes' written records, paths and CSV templates.
mod guide_examples;
use tag_core::{
    actions::{apply_action_spec, ActionEffect, ActionSpec, ActionState},
    decode_export_template, encode_export_document, evaluate_filename_with_context, render_export,
    EvalContext, ExportOptions, ExportRow, Fields, TextImportOptions, TextTagConverter,
};
const TRACKS: &str = include_str!("../../../docs/user-guide/cookbook/tracklist-from-text-file.md");
const LYRICS: &str = include_str!("../../../docs/user-guide/cookbook/lyrics-from-text-files.md");
const CSV: &str = include_str!("../../../docs/user-guide/cookbook/csv-for-spreadsheets.md");
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
fn selected() -> Vec<String> {
    ["/Music/01.flac", "/Music/02.flac"]
        .map(String::from)
        .to_vec()
}
#[test]
fn written_positional_tracklist_discards_listing_numbers_and_reads_literal_brackets() {
    for page in forms(TRACKS) {
        let converter = TextTagConverter::compile(&block(&page, "tracklist-mask")).unwrap();
        let records = block(&page, "tracklist-records");
        for ending in ["\n", "\r\n", "\r"] {
            let records = format!("{}{ending}", records.replace('\n', ending));
            let plans = converter
                .plan(&records, &selected(), &TextImportOptions::default())
                .unwrap();
            assert_eq!(plans.len(), 2);
            for (index, title) in ["Paper Boats", "Lanterns on the Water"].iter().enumerate() {
                assert_eq!(plans[index].path, selected()[index]);
                assert_eq!(plans[index].source_line, index + 1);
                assert_eq!(plans[index].fields["ARTIST"], ["Juniper Vale"]);
                assert_eq!(plans[index].fields["ALBUM"], ["Harbour Lights"]);
                assert_eq!(plans[index].fields["TRACK"], [(index + 1).to_string()]);
                assert_eq!(plans[index].fields["TITLE"], [*title]);
                assert!(!plans[index].fields.contains_key("DUMMY"));
            }
        }
        assert!(converter
            .plan(&records, &selected()[..1], &TextImportOptions::default())
            .is_err());
        assert!(converter
            .plan(
                &format!("{records}\n\n"),
                &selected(),
                &TextImportOptions::default()
            )
            .is_err());
    }
}
#[test]
fn written_keyed_tracklist_matches_reordered_files_skips_unselected_and_leaves_unlisted() {
    for page in forms(TRACKS) {
        let converter = TextTagConverter::compile(&block(&page, "tracklist-keyed-mask")).unwrap();
        let records = block(&page, "tracklist-keyed-records");
        let mut paths = selected();
        paths.push("/Music/04.flac".into());
        let options = TextImportOptions::default();
        let plans = converter.plan(&records, &paths, &options).unwrap();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].path, paths[1]);
        assert_eq!(plans[0].fields["TITLE"], ["Lanterns on the Water"]);
        assert_eq!(plans[1].path, paths[0]);
        assert_eq!(plans[1].fields["TITLE"], ["Paper Boats"]);
        assert!(plans
            .iter()
            .all(|plan| !plan.fields.contains_key("ARTIST") && !plan.fields.contains_key("ALBUM")));
        assert!(converter
            .plan(&format!("{records}\n01.flac;Duplicate;1"), &paths, &options)
            .is_err());
        paths.push("/Elsewhere/01.flac".into());
        assert!(converter.plan(&records, &paths, &options).is_err());
    }
}
#[test]
fn written_album_source_and_lyrics_sidecar_paths_evaluate_with_actual_filename_rules() {
    for page in forms(LYRICS) {
        let template = block(&page, "lyrics-filename");
        let subfolder = block(&page, "lyrics-subfolder");
        let mut count = 0;
        for line in section(&page, "lyrics-path-cases")
            .lines()
            .filter(|line| line.starts_with("| `"))
        {
            let row: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
            let audio = row[0].trim_matches('`');
            let expected = row[1].trim_matches('`');
            let stem = audio.rsplit_once('.').unwrap().0;
            let mut state = ActionState {
                fields: Fields::from([(
                    "LYRICS".into(),
                    vec!["Old first value".into(), "Old second value".into()],
                )]),
                context: Fields::from([("_FILENAME".into(), vec![stem.into()])]),
            };
            let before = state.fields.clone();
            let effects = apply_action_spec(
                &mut state,
                &ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: template.clone(),
                },
            )
            .unwrap();
            assert_eq!(
                effects,
                [ActionEffect::ImportTextFile {
                    field: "LYRICS".into(),
                    filename: expected.into()
                }]
            );
            // Planning requests a host read/write; it has not pretended to read the sidecar.
            assert_eq!(state.fields, before);
            let effects = apply_action_spec(
                &mut state,
                &ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: subfolder.clone(),
                },
            )
            .unwrap();
            assert_eq!(
                effects,
                [ActionEffect::ImportTextFile {
                    field: "LYRICS".into(),
                    filename: format!("Lyrics\\{expected}")
                }]
            );
            count += 1;
        }
        assert_eq!(count, 3);
    }
    for page in forms(TRACKS) {
        let fields = Fields::from([("_FOLDERPATH".into(), vec!["/Music/Harbour Lights/".into()])]);
        assert_eq!(
            evaluate_filename_with_context(
                &block(&page, "tracklist-source"),
                &EvalContext::new(&fields)
            )
            .unwrap(),
            "/Music/Harbour Lights/tracks.txt"
        );
    }
}
#[test]
fn written_lyrics_decode_as_whole_text_preserving_blank_lines_spaces_and_final_endings() {
    for page in forms(LYRICS) {
        let practice = block(&page, "lyrics-practice");
        for text in [
            practice.clone(),
            format!("{practice}\n"),
            practice.replace('\n', "\r\n"),
            format!("  {practice}\n\n"),
            String::new(),
        ] {
            assert_eq!(decode_export_template(text.as_bytes(), None).unwrap(), text);
            let mut bom = vec![0xef, 0xbb, 0xbf];
            bom.extend_from_slice(text.as_bytes());
            assert_eq!(decode_export_template(&bom, None).unwrap(), text);
        }
        assert_eq!(
            decode_export_template(b"Caf\xe9\r\n", Some("windows-1252")).unwrap(),
            "Café\r\n"
        );
    }
}
fn rows() -> Vec<ExportRow> {
    [
        ("/Music/01.flac", "Paper, \"folded\" Boats", "03"),
        ("/Music/02.flac", "Café at Low Tide", "04"),
    ]
    .into_iter()
    .map(|(path, title, track)| {
        ExportRow::new(
            Fields::from([
                ("TITLE".into(), vec![title.into()]),
                ("TRACK".into(), vec![track.into()]),
            ]),
            path,
            180.,
            1024,
        )
    })
    .collect()
}
#[test]
fn written_spreadsheet_template_emits_exact_quoted_unicode_records_and_utf8_bytes() {
    for page in forms(CSV) {
        let options = ExportOptions {
            filename: "/Reports/catalogue.txt".into(),
            ..Default::default()
        };
        let docs = render_export(&block(&page, "spreadsheet-template"), &rows(), &options).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].filename, "/Reports/catalogue.csv");
        assert_eq!(docs[0].encoding, "utf-8");
        assert_eq!(
            docs[0].text,
            format!(
                "{}\r\n",
                block(&page, "spreadsheet-output").replace('\n', "\r\n")
            )
        );
        assert_eq!(
            encode_export_document(&docs[0], None).unwrap(),
            docs[0].text.as_bytes()
        );
    }
}
#[test]
fn written_spreadsheet_template_escapes_each_cell_and_preserves_multiline_and_text_keys() {
    for page in forms(CSV) {
        let row = ExportRow::new(
            Fields::from([
                ("TITLE".into(), vec!["First\r\nSecond, \"third\"".into()]),
                ("TRACK".into(), vec!["03\"take".into()]),
            ]),
            "/Music/a,\"b\".flac",
            180.,
            1024,
        );
        let docs = render_export(
            &block(&page, "spreadsheet-template"),
            &[row],
            &ExportOptions::default(),
        )
        .unwrap();
        assert_eq!(docs[0].text, "\"Path\",\"Title\",\"Track\"\r\n\"/Music/a,\"\"b\"\".flac\",\"First\r\nSecond, \"\"third\"\"\",\"03\"\"take\"\r\n");
    }
}
