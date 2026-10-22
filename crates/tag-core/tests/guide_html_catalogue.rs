//! Execute the actual HTML recipe, not a copied template or a substitute renderer.
//! Image fixtures are separate local files; these tests do not exercise a browser,
//! native export dialog, artwork extraction, or audio-file mutation.
mod guide_examples;

use guide_examples::text_blocks;
use std::path::Path;
use tag_core::{encode_export_document, render_export, ExportOptions, ExportRow};

const RECIPE: &str = include_str!("../../../docs/user-guide/export/recipes/html-album-catalogue.md");

fn row(
    folder: &Path,
    album: &str,
    artist: &str,
    track: &str,
    title: &str,
    disc: &str,
) -> ExportRow {
    ExportRow::new(
        [
            ("ALBUM", album),
            ("ALBUMARTIST", artist),
            ("ARTIST", artist),
            ("TRACK", track),
            ("TITLE", title),
            ("DISCNUMBER", disc),
        ]
        .into_iter()
        .map(|(field, value)| (field.into(), vec![value.into()]))
        .collect(),
        folder
            .join(format!("input-{disc}-{track}.flac"))
            .to_string_lossy(),
        180.,
        1000,
    )
}

fn options(folder: &Path) -> ExportOptions {
    ExportOptions {
        filename: "export.txt".into(),
        filename_override: Some("catalogue.html".into()),
        base_directory: folder.to_string_lossy().into(),
        timestamp_seconds: Some(0),
        ..Default::default()
    }
}

fn cover(folder: &Path, extension: &str) {
    std::fs::create_dir_all(folder).unwrap();
    image::RgbImage::from_pixel(2, 2, image::Rgb([40, 100, 160]))
        .save(folder.join(format!("folder.{extension}")))
        .unwrap();
}

#[test]
fn complete_recipe_escapes_text_sorts_tracks_and_encodes_utf8_for_both_checkout_newlines() {
    let folder = tempfile::tempdir().unwrap();
    cover(folder.path(), "jpg");
    let rows = [
        row(
            folder.path(),
            "Harbour <Lights> & More",
            "Kofi & the Tidewater Band",
            "2",
            "Lanterns on the Water",
            "1",
        ),
        row(
            folder.path(),
            "Harbour <Lights> & More",
            "Kofi & the Tidewater Band",
            "1",
            "Low <Tide> & \"Foam\" — Café",
            "1",
        ),
    ];
    let expected = concat!(
        "<!doctype html>\n<html lang=\"en\">\n<meta charset=\"utf-8\">\n",
        "<title>Album catalogue</title>\n<body>\n",
        "<h1>Harbour &lt;Lights&gt; &amp; More</h1>\n",
        "<p>Kofi &amp; the Tidewater Band</p>\n",
        "<p><img src=\"folder.jpg\" alt=\"Album cover\" width=\"240\"></p>\n<ul>\n",
        "<li>01. Low &lt;Tide&gt; &amp; \"Foam\" — Café</li>\n",
        "<li>02. Lanterns on the Water</li>\n</ul>\n",
        "<p>Tracks: 2</p>\n</body>\n</html>"
    );
    let lf = RECIPE.replace("\r\n", "\n");
    for page in [&lf, &lf.replace('\n', "\r\n")] {
        let blocks = text_blocks(page);
        let documents = render_export(&blocks[0], &rows, &options(folder.path())).unwrap();
        assert_eq!(documents.len(), 1);
        let document = &documents[0];
        assert_eq!(
            Path::new(&document.filename),
            folder.path().join("catalogue.html")
        );
        assert_eq!(document.encoding, "utf-8");
        assert_eq!(document.text, expected);
        assert_eq!(
            encode_export_document(document, None).unwrap(),
            expected.as_bytes()
        );
        assert!(Path::new(&document.filename)
            .parent()
            .unwrap()
            .join("folder.jpg")
            .is_file());
    }
}

#[test]
fn png_variation_keeps_the_real_image_extension_and_falls_back_to_artist() {
    let folder = tempfile::tempdir().unwrap();
    cover(folder.path(), "png");
    let mut input = row(
        folder.path(),
        "Low Tide",
        "Mira Solberg",
        "1",
        "North Pier",
        "1",
    );
    input.fields.remove("ALBUMARTIST");
    let blocks = text_blocks(RECIPE);
    let png_section = RECIPE
        .split("### Use a PNG cover")
        .nth(1)
        .unwrap()
        .split("### ")
        .next()
        .unwrap();
    let replacement: Vec<_> = png_section.split('`').collect();
    let template = blocks[0].replace(replacement[1], replacement[3]);
    let report = render_export(&template, &[input], &options(folder.path())).unwrap();
    assert!(report[0].text.contains("<p>Mira Solberg</p>"));
    assert!(report[0]
        .text
        .contains("<img src=\"folder.png\" alt=\"Album cover\" width=\"240\">"));
    assert!(report[0].text.contains("<li>01. North Pier</li>"));
    assert!(folder.path().join("folder.png").is_file());
    assert!(!folder.path().join("folder.jpg").exists());
}

#[test]
fn folder_partition_snippets_keep_album_rows_and_covers_with_each_report() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("Harbour Lights");
    let second = root.path().join("Low Tide");
    cover(&first, "jpg");
    cover(&second, "jpg");
    let rows = [
        row(
            &first,
            "Harbour Lights",
            "Juniper Vale",
            "2",
            "Lanterns on the Water",
            "1",
        ),
        row(&second, "Low Tide", "Mira Solberg", "1", "North Pier", "1"),
        row(
            &first,
            "Harbour Lights",
            "Juniper Vale",
            "1",
            "Paper Boats",
            "1",
        ),
    ];
    let blocks = text_blocks(RECIPE);
    let mut config = ExportOptions {
        partition: Some(blocks[1].clone()),
        filename_override: Some(blocks[2].clone()),
        ..options(root.path())
    };
    let documents = render_export(&blocks[0], &rows, &config).unwrap();
    assert_eq!(documents.len(), 2);
    assert_eq!(
        Path::new(&documents[0].filename),
        first.join("catalogue.html")
    );
    assert_eq!(
        Path::new(&documents[1].filename),
        second.join("catalogue.html")
    );
    assert!(documents[0].text.contains("<h1>Harbour Lights</h1>"));
    assert!(documents[0].text.contains("<p>Tracks: 2</p>"));
    assert!(!documents[0].text.contains("North Pier"));
    assert!(documents[1].text.contains("<h1>Low Tide</h1>"));
    assert!(documents[1].text.contains("<p>Tracks: 1</p>"));
    assert!(!documents[1].text.contains("Paper Boats"));
    for document in &documents {
        assert!(Path::new(&document.filename)
            .parent()
            .unwrap()
            .join("folder.jpg")
            .is_file());
    }
    config.filename_override = Some(root.path().join("collision.html").to_string_lossy().into());
    assert!(render_export(&blocks[0], &rows, &config)
        .unwrap_err()
        .contains("Multiple export groups map to the same output file"));
}

#[test]
fn multidisc_snippets_sort_and_print_disc_numbers_including_missing_or_nonnumeric_values() {
    let folder = tempfile::tempdir().unwrap();
    let rows = [
        row(
            folder.path(),
            "Harbour Lights",
            "Juniper Vale",
            "2",
            "Disc Two Last",
            "2",
        ),
        row(
            folder.path(),
            "Harbour Lights",
            "Juniper Vale",
            "1",
            "Disc One First",
            "1",
        ),
        row(
            folder.path(),
            "Harbour Lights",
            "Juniper Vale",
            "1",
            "Disc Two First",
            "2",
        ),
        row(
            folder.path(),
            "Harbour Lights",
            "Juniper Vale",
            "1",
            "Missing Disc",
            "",
        ),
        row(
            folder.path(),
            "Harbour Lights",
            "Juniper Vale",
            "2",
            "Letter Disc",
            "B",
        ),
    ];
    let blocks = text_blocks(RECIPE);
    let section = RECIPE.split("Replace the printed ").nth(1).unwrap();
    let printed: Vec<_> = section.split('`').collect();
    let template = blocks[0]
        .replace("$loop($num(%track%,3))", &blocks[3])
        .replace(printed[1], printed[3]);
    let documents = render_export(&template, &rows, &options(folder.path())).unwrap();
    let list = documents[0]
        .text
        .split("<ul>\n")
        .nth(1)
        .unwrap()
        .split("</ul>")
        .next()
        .unwrap();
    assert_eq!(
        list,
        concat!(
            "<li>0-01. Missing Disc</li>\n",
            "<li>0-02. Letter Disc</li>\n",
            "<li>1-01. Disc One First</li>\n",
            "<li>2-01. Disc Two First</li>\n",
            "<li>2-02. Disc Two Last</li>\n"
        )
    );
}
