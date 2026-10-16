//! Execute the guide's filter questions and check its native identifier tables.
//! These tests validate OpenTag documentation, not original-Mp3tag runtime parity.
use tag_core::{compile_filter, matches_filter, Fields, FIELD_MAPPINGS};

const FILTER: &str = include_str!("../../../docs/user-guide/reference/filter-syntax.md");
const MAPPING: &str = include_str!("../../../docs/user-guide/reference/tag-mapping.md");
const NAMES: &str = include_str!("../../../docs/user-guide/reference/field-names.md");

fn examples(document: &str, marker: &str) -> Vec<Vec<String>> {
    let start = format!("<!-- {marker}:start -->");
    let end = format!("<!-- {marker}:end -->");
    let body = document.split_once(&start).expect("start marker").1;
    let body = body.split_once(&end).expect("end marker").0;
    body.lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect()
        })
        .collect()
}

fn fixture(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(field, values)| {
            (
                (*field).to_owned(),
                values.iter().map(|value| (*value).to_owned()).collect(),
            )
        })
        .collect()
}

#[test]
fn every_published_filter_question_has_the_documented_result() {
    let files = [
        (
            "A",
            fixture(&[
                ("ARTIST", &["Juniper Vale"]),
                ("TITLE", &["Harbour Lights"]),
                ("GENRE", &["Indie Folk"]),
                ("TRACK", &["01/08"]),
                ("YEAR", &["2024"]),
                ("ALBUM", &["Harbour Lights"]),
                ("_BITRATE", &["320"]),
                ("_FILENAME_EXT", &["01 - Harbour Lights.flac"]),
            ]),
        ),
        (
            "B",
            fixture(&[
                ("ARTIST", &["Juniper Vale"]),
                ("TITLE", &["Paper Boats (Live)"]),
                ("GENRE", &["Folk"]),
                ("TRACK", &["2/8"]),
                ("YEAR", &["2024-03-15"]),
                ("ALBUM", &["Harbour Lights"]),
                ("ALBUMARTIST", &["Juniper Vale"]),
                ("_BITRATE", &["256"]),
                ("_FILENAME_EXT", &["02 - Paper Boats.mp3"]),
            ]),
        ),
        (
            "C",
            fixture(&[
                ("ARTIST", &["Mira Solberg", "Juniper Vale"]),
                ("TITLE", &["North \"Wind\""]),
                ("GENRE", &["Ambient"]),
                ("TRACK", &["10"]),
                ("YEAR", &["2023"]),
                ("ALBUM", &["Night Harbour"]),
                ("ALBUMARTIST", &["Mira Solberg"]),
                ("_BITRATE", &["1411"]),
                ("_FILENAME_EXT", &["10 - North Wind.flac"]),
            ]),
        ),
        (
            "D",
            fixture(&[
                ("ARTIST", &["The Lanterns"]),
                ("TITLE", &["Low Tide"]),
                ("GENRE", &["Folk"]),
                ("TRACK", &["B2"]),
                ("YEAR", &[""]),
                ("_BITRATE", &["128"]),
                ("_FILENAME_EXT", &["bonus.flac"]),
            ]),
        ),
    ];
    let rows = examples(FILTER, "filter-examples");
    assert_eq!(
        rows.len(),
        24,
        "verify each published question, including quote handling"
    );
    for row in rows {
        assert_eq!(row.len(), 3);
        let actual: Vec<_> = files
            .iter()
            .filter_map(|(name, fields)| {
                matches_filter(&row[0], fields)
                    .unwrap_or_else(|error| panic!("{}: {error}", row[0]))
                    .then_some(*name)
            })
            .collect();
        let expected: Vec<_> = if row[1] == "None" {
            vec![]
        } else {
            row[1].split(", ").collect()
        };
        assert_eq!(actual, expected, "{}", row[0]);
    }
}

#[test]
fn documented_invalid_filter_forms_fail_instead_of_producing_matches() {
    for query in [
        "TRACK GREATER 3.5",
        "TRACK GREATER 3:30",
        "TRACK GREATER 9223372036854775808",
        "* PRESENT",
        "* IS x",
        "TITLE MATCH x",
        "TITLE IS",
        "TITLE MATCHES \"(\"",
        "\"$len(%title%)\" PRESENT",
        "TITLE IS \"unfinished",
    ] {
        assert!(compile_filter(query).is_err(), "{query}");
    }
}

fn native(value: &str) -> String {
    if value.is_empty() {
        "—".into()
    } else {
        value.replace(" | ", "; ")
    }
}

#[test]
fn every_native_table_row_matches_the_checked_mapping_vocabulary() {
    let id3_mp4 = examples(MAPPING, "native-mapping-id3-mp4");
    let containers = examples(MAPPING, "native-mapping-containers");
    assert_eq!(id3_mp4.len(), FIELD_MAPPINGS.len());
    assert_eq!(containers.len(), FIELD_MAPPINGS.len());
    for ((id3, container), mapping) in id3_mp4.iter().zip(&containers).zip(FIELD_MAPPINGS) {
        // The public table documents the effective MP4 writer spelling.
        let mp4 = if mapping.field == "ARTIST" {
            "©ART"
        } else {
            mapping.mp4
        };
        assert_eq!(
            id3,
            &vec![
                mapping.field.into(),
                native(mapping.id3v23),
                native(mapping.id3v24),
                native(mp4)
            ],
            "{}",
            mapping.field
        );
        assert_eq!(
            container,
            &vec![
                mapping.field.into(),
                native(mapping.matroska),
                native(mapping.asf),
                native(mapping.riff)
            ],
            "{}",
            mapping.field
        );
    }
}

#[test]
fn every_mapped_field_has_one_vocabulary_definition() {
    let rows: Vec<_> = NAMES
        .lines()
        .filter(|line| line.starts_with("| `"))
        .collect();
    for mapping in FIELD_MAPPINGS {
        let prefix = format!("| `{}` |", mapping.field);
        assert_eq!(
            rows.iter().filter(|line| line.starts_with(&prefix)).count(),
            1,
            "{} must have exactly one meaning",
            mapping.field
        );
    }
}
