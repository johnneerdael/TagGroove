//! Execute the export chapters' actual templates and written output blocks.
mod guide_examples;
use guide_examples::text_blocks;
use tag_core::{encode_export_document, render_export, ExportOptions, ExportRow};

const REFERENCE: &str = include_str!("../../../docs/user-guide/export/template-language.md");
const CSV: &str = include_str!("../../../docs/user-guide/export/csv.md");
const GALLERY: &str = include_str!("../../../docs/user-guide/export/template-gallery.md");
const LESSON: &str = include_str!("../../../docs/user-guide/export/export-a-cd-catalogue.md");

fn rows() -> Vec<ExportRow> {
    [
        (
            "Juniper Vale",
            "Harbour Lights",
            "2",
            "Lanterns on the Water",
            200.,
            2048,
        ),
        (
            "Juniper Vale",
            "Harbour Lights",
            "1",
            "Paper Boats",
            180.,
            1024,
        ),
        ("Mira Solberg", "Low Tide", "1", "North Pier", 240., 4096),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (artist, album, track, title, duration, size))| {
        ExportRow::new(
            [
                ("ARTIST".into(), vec![artist.into()]),
                ("ALBUMARTIST".into(), vec![artist.into()]),
                ("ALBUM".into(), vec![album.into()]),
                ("TRACK".into(), vec![track.into()]),
                ("TITLE".into(), vec![title.into()]),
                ("YEAR".into(), vec!["2024".into()]),
            ]
            .into(),
            format!("/Music/{index}.mp3"),
            duration,
            size,
        )
    })
    .collect()
}

fn options() -> ExportOptions {
    ExportOptions {
        filename: "/Reports/catalogue.txt".into(),
        ..Default::default()
    }
}

fn both_checkouts(page: &str, check: impl Fn(&str)) {
    let lf = page.replace("\r\n", "\n");
    check(&lf);
    check(&lf.replace('\n', "\r\n"));
}

#[test]
fn catalogue_lesson_writes_its_actual_before_and_after_reports_and_partitions() {
    both_checkouts(LESSON, |page| {
        let examples = text_blocks(page);
        assert_eq!(examples.len(), 4);
        for (template, output) in [(0, 1), (2, 3)] {
            let report = render_export(&examples[template], &rows(), &options()).unwrap();
            assert_eq!(report[0].text, format!("{}\n", examples[output]));
        }
        let step = page
            .lines()
            .find(|line| line.contains("format** to `"))
            .unwrap();
        let patterns: Vec<_> = step.split('`').collect();
        let options = ExportOptions {
            filename_override: Some(patterns[3].into()),
            base_directory: "/Reports".into(),
            partition: Some(patterns[1].into()),
            ..options()
        };
        let documents = render_export(&examples[2], &rows(), &options).unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0].filename, "/Reports/Harbour Lights.txt");
        assert_eq!(
            documents[0].text,
            "Harbour Lights — files: 2\n  01 Paper Boats\n  02 Lanterns on the Water\n"
        );
        assert_eq!(documents[1].filename, "/Reports/Low Tide.txt");
        assert_eq!(documents[1].text, "Low Tide — files: 1\n  01 North Pier\n");
    });
}

#[test]
fn template_reference_runs_sorting_unique_groups_nested_groups_and_variables() {
    both_checkouts(REFERENCE, |page| {
        let examples = text_blocks(page);
        assert_eq!(examples.len(), 6);
        let rows = rows();
        let ordered = vec![rows[1].clone(), rows[0].clone()];
        assert_eq!(
            render_export(&examples[0], &ordered, &options()).unwrap()[0].text,
            "Juniper Vale - Paper Boats\nJuniper Vale - Lanterns on the Water\n"
        );
        let mut numbered = rows.clone();
        for (row, track) in numbered.iter_mut().zip(["10", "2", "1"]) {
            row.fields.insert("TRACK".into(), vec![track.into()]);
        }
        assert_eq!(
            render_export(&examples[1], &numbered, &options()).unwrap()[0].text,
            "1 - North Pier\n2 - Paper Boats\n10 - Lanterns on the Water\n"
        );
        let unique = render_export(&examples[2], &rows, &options()).unwrap();
        assert_eq!(unique[0].text, "Juniper Vale\nMira Solberg\n");
        assert_eq!(
            render_export(&examples[3], &rows, &options()).unwrap()[0].text,
            format!("{}\n", examples[4])
        );
        assert_eq!(
            render_export(&examples[5], &rows, &options()).unwrap()[0].text,
            "Lanterns on the Water\nPaper Boats\nNorth Pier\nRows written: 3"
        );
    });
}

#[test]
fn csv_template_doubles_embedded_quotes_and_matches_its_written_records() {
    both_checkouts(CSV, |page| {
        let examples = text_blocks(page);
        assert_eq!(examples.len(), 2);
        let mut row = rows().remove(1);
        row.fields
            .insert("TITLE".into(), vec!["Paper, \"folded\" Boats".into()]);
        row.fields.insert("TRACK".into(), vec!["03".into()]);
        let doc = render_export(&examples[0], std::slice::from_ref(&row), &options())
            .unwrap()
            .remove(0);
        assert_eq!(doc.filename, "/Reports/catalogue.csv");
        assert_eq!(doc.encoding, "utf-8");
        assert_eq!(
            doc.text,
            format!("{}\r\n", examples[1].replace('\n', "\r\n"))
        );
        assert_eq!(
            encode_export_document(&doc, None).unwrap(),
            doc.text.as_bytes()
        );
        row.fields.insert("TRACK".into(), vec!["03\"take".into()]);
        let doc = render_export(&examples[0], &[row], &options())
            .unwrap()
            .remove(0);
        assert!(doc.text.ends_with(",\"03\"\"take\"\r\n"), "{}", doc.text);
    });
}

#[test]
fn gallery_templates_preserve_totals_unique_albums_and_html_text_escaping() {
    both_checkouts(GALLERY, |page| {
        let examples = text_blocks(page);
        assert_eq!(examples.len(), 3);
        let rows = rows();
        assert_eq!(render_export(&examples[0], &rows[..2], &options()).unwrap()[0].text,
            "01. Paper Boats — Juniper Vale\n02. Lanterns on the Water — Juniper Vale\nFiles: 2; duration: 00:06:20");
        assert_eq!(
            render_export(&examples[1], &rows, &options()).unwrap()[0].text,
            "Juniper Vale — Harbour Lights (2024)\nMira Solberg — Low Tide (2024)\n"
        );
        let mut row = rows[0].clone();
        row.fields
            .insert("ARTIST".into(), vec!["Kofi & the Tidewater Band".into()]);
        row.fields.insert("TITLE".into(), vec!["Low <Tide>".into()]);
        let document = render_export(&examples[2], &[row], &options())
            .unwrap()
            .remove(0);
        assert_eq!(document.filename, "/Reports/catalogue.html");
        assert!(document
            .text
            .contains("<li>Kofi &amp; the Tidewater Band — Low &lt;Tide&gt;</li>\n"));
        assert!(page.contains("`Kofi &amp; the Tidewater Band — Low &lt;Tide&gt;`"));
        assert!(document.text.ends_with("</ul><p>Files: 1</p>"));
    });
}
