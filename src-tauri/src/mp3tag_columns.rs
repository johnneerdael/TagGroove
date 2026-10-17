//! Import of File List column definitions from an Mp3tag configuration file the
//! user supplies (`columns.ini` in Mp3tag's configuration folder). Only what the
//! file contains is read: one `[section]` per column with the keys Mp3tag's
//! Customize columns dialog edits (name, value, field, sort by, width, numeric,
//! alignment and visibility). Spellings vary between Mp3tag versions, so each
//! setting accepts the usual key names; unknown keys and sections are ignored.
use crate::columns::{validate, Align, Column, Layout, MAX_WIDTH, MIN_WIDTH};
use crate::import_diagnostics::ImportFailure;
use serde::Serialize;
use std::collections::HashSet;
use tag_core::batch_errors::{ErrorCause, IndexedCause};
use tag_core::CompiledTemplate;

#[derive(Debug, Serialize)]
pub struct ColumnImport {
    pub layout: Layout,
    /// Columns left out, with the reason (for example an unsupported function).
    pub skipped: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ColumnImportReport {
    #[serde(flatten)]
    pub result: ColumnImport,
    pub skipped_diagnostics: Vec<IndexedCause>,
}
fn skip_column(
    skipped: &mut Vec<String>,
    diagnostics: &mut Vec<IndexedCause>,
    name: &str,
    cause: ErrorCause,
) {
    let cause = ErrorCause::wrap(
        "{value}: {error}",
        [("value", name.to_string())],
        "error",
        cause,
    );
    diagnostics.push(IndexedCause {
        index: skipped.len(),
        cause: cause.clone(),
    });
    skipped.push(cause.to_string());
}
/// UTF-16 (with BOM), UTF-8 or, failing that, Windows-1252 text.
pub fn decode(bytes: &[u8]) -> String {
    let text = match bytes {
        [0xFF, 0xFE, rest @ ..] => encoding_rs::UTF_16LE.decode(rest).0.into_owned(),
        [0xFE, 0xFF, rest @ ..] => encoding_rs::UTF_16BE.decode(rest).0.into_owned(),
        _ => match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
        },
    };
    text.trim_start_matches('\u{feff}').to_owned()
}

/// Sections in file order, each with its lower-case keys and raw values.
pub(crate) fn sections(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut result: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') && !line.contains('=')
        {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            result.push((name.trim().to_string(), Vec::new()));
        } else if let (Some((key, value)), Some(section)) =
            (line.split_once('='), result.last_mut())
        {
            section
                .1
                .push((key.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    result
}

pub(crate) fn lookup<'a>(keys: &'a [(String, String)], names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|name| {
        keys.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    })
}
pub(crate) fn flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
/// Windows list-view alignment: 0 left, 1 right, 2 center.
fn align(value: &str) -> Align {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "right" => Align::Right,
        "2" | "center" | "centre" => Align::Center,
        _ => Align::Left,
    }
}
/// A bare field name (ISRC) becomes its placeholder (%ISRC%).
fn field_placeholder(value: &str) -> String {
    let value = value.trim();
    if !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ' ')
    {
        format!("%{value}%")
    } else {
        value.to_string()
    }
}
fn identifier(name: &str, taken: &mut HashSet<String>) -> String {
    let base: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .take(48)
        .collect();
    let base = if base.is_empty() {
        "column".to_string()
    } else {
        base
    };
    let mut id = base.clone();
    let mut n = 2;
    while !taken.insert(id.clone()) {
        id = format!("{base}{n}");
        n += 1;
    }
    id
}

#[cfg(test)]
pub fn parse(text: &str) -> Result<ColumnImport, String> {
    parse_diagnostic(text)
        .map(|report| report.result)
        .map_err(|error| error.to_string())
}
pub fn parse_diagnostic(text: &str) -> Result<ColumnImportReport, ImportFailure> {
    let mut found: Vec<(Option<i64>, Column)> = Vec::new();
    let mut skipped = Vec::new();
    let mut skipped_diagnostics = Vec::new();
    let mut taken = HashSet::new();
    for (section, keys) in sections(text) {
        let Some(value) = lookup(&keys, &["value", "format", "fmt", "display format"])
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let name = lookup(&keys, &["name", "title", "caption"])
            .filter(|name| !name.is_empty())
            .unwrap_or(if section.is_empty() { value } else { &section })
            .to_string();
        if let Err(error) = CompiledTemplate::compile(value) {
            skip_column(
                &mut skipped,
                &mut skipped_diagnostics,
                &name,
                ErrorCause::owned_text(error),
            );
            continue;
        }
        let sort_by = lookup(&keys, &["sort", "sortby", "sort by", "sortorder"])
            .unwrap_or("")
            .to_string();
        if !sort_by.is_empty() && CompiledTemplate::compile(&sort_by).is_err() {
            skip_column(
                &mut skipped,
                &mut skipped_diagnostics,
                &name,
                ErrorCause::owned("the sort format string is not supported"),
            );
            continue;
        }
        let mut field =
            field_placeholder(lookup(&keys, &["field", "edit", "editfield"]).unwrap_or(""));
        if crate::columns::editable_field(&field).is_err() {
            // Columns that cannot be edited in OpenTag stay read-only.
            field.clear();
        }
        let width = lookup(&keys, &["width", "size"])
            .and_then(|width| width.trim().parse::<i64>().ok())
            .map(|width| width.clamp(i64::from(MIN_WIDTH), i64::from(MAX_WIDTH)) as u32)
            .unwrap_or(100);
        let visible = match lookup(&keys, &["show", "visible", "shown", "enabled"]).and_then(flag) {
            Some(shown) => shown,
            None => !lookup(&keys, &["hidden", "hide"])
                .and_then(flag)
                .unwrap_or(false),
        };
        let order = lookup(&keys, &["pos", "position", "order", "index"])
            .and_then(|pos| pos.trim().parse().ok());
        found.push((
            order,
            Column {
                id: identifier(&name, &mut taken),
                name,
                value: value.to_string(),
                field,
                sort_by,
                width,
                align: lookup(&keys, &["align", "alignment"])
                    .map(align)
                    .unwrap_or_default(),
                numeric: lookup(&keys, &["num", "numeric", "isnumeric"])
                    .and_then(flag)
                    .unwrap_or(false),
                visible,
            },
        ));
    }
    if found.is_empty() {
        return Err(if skipped.is_empty() {
            ImportFailure::parts(ErrorCause::owned("The file contains no column definitions. Choose columns.ini from the Mp3tag configuration folder."))
        } else {
            ImportFailure::parts(ErrorCause::children(
                "No column could be imported: {error}",
                [(
                    "error",
                    ErrorCause::joined(
                        skipped_diagnostics
                            .into_iter()
                            .map(|item| item.cause)
                            .collect(),
                        "; ",
                    ),
                )],
            ))
        });
    }
    // Columns without a position keep their place after the positioned ones.
    found.sort_by_key(|(order, _)| order.unwrap_or(i64::MAX));
    let mut columns: Vec<Column> = found.into_iter().map(|(_, column)| column).collect();
    if !columns.iter().any(|column| column.visible) {
        columns[0].visible = true;
    }
    columns.truncate(200);
    let layout = Layout {
        version: 1,
        columns,
        sort: Vec::new(),
    };
    validate(&layout).map_err(ImportFailure::Plain)?;
    Ok(ColumnImportReport {
        result: ColumnImport { layout, skipped },
        skipped_diagnostics,
    })
}

#[cfg(test)]
#[tauri::command]
pub async fn import_mp3tag_columns(path: String) -> Result<ColumnImport, String> {
    crate::io(move || {
        let input = crate::auxiliary_input(&path)?;
        let bytes =
            std::fs::read(&input).map_err(|e| format!("Cannot read {}: {e}", input.display()))?;
        parse(&decode(&bytes))
    })
    .await
}

#[tauri::command(rename = "import_mp3tag_columns")]
pub async fn import_mp3tag_columns_with_diagnostics(
    path: String,
) -> Result<ColumnImportReport, ImportFailure> {
    crate::io(move || {
        let input = crate::auxiliary_input(&path)?;
        Ok(std::fs::read(&input)
            .map_err(|error| {
                ImportFailure::parts(ErrorCause::wrap(
                    "Cannot read {value}: {e}",
                    [("value", input.to_string_lossy().into_owned())],
                    "e",
                    ErrorCause::external(error),
                ))
            })
            .and_then(|bytes| parse_diagnostic(&decode(&bytes))))
    })
    .await
    .map_err(ImportFailure::Plain)?
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[#1]\r\nName=Title\r\nValue=%title%\r\nField=%title%\r\nWidth=180\r\nAlign=0\r\nNum=0\r\nShow=1\r\nPos=1\r\n\
        [#0]\r\nName=Filename\r\nValue=%_filename_ext%\r\nField=%_filename_ext%\r\nWidth=250\r\nPos=0\r\n\
        [#2]\r\nName=Track\r\nValue=$num(%track%,2)\r\nField=TRACK\r\nSort=%track%\r\nWidth=9999\r\nAlign=1\r\nNum=1\r\nShow=0\r\nPos=2\r\n\
        [#3]\r\nName=Length\r\nValue=%_length%\r\nField=%_length%\r\nWidth=50\r\nPos=3\r\n\
        [#4]\r\nName=Broken\r\nValue=$nosuchfunction(%title%)\r\nPos=4\r\n\
        [General]\r\nVersion=3\r\n";

    #[test]
    fn reads_columns_in_position_order_with_their_settings() {
        let import = parse(SAMPLE).unwrap();
        let columns = &import.layout.columns;
        assert_eq!(
            columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["Filename", "Title", "Track", "Length"]
        );
        assert_eq!(columns[0].id, "filename");
        assert_eq!(columns[0].field, "%_filename_ext%");
        assert_eq!(columns[1].width, 180);
        let track = &columns[2];
        assert_eq!(
            (track.field.as_str(), track.sort_by.as_str()),
            ("%TRACK%", "%track%")
        );
        assert_eq!(
            (track.width, track.align, track.numeric, track.visible),
            (MAX_WIDTH, Align::Right, true, false)
        );
        assert_eq!(columns[3].field, "", "technical fields stay read-only");
        assert_eq!(import.skipped.len(), 1);
        assert!(
            import.skipped[0].starts_with("Broken:"),
            "{:?}",
            import.skipped
        );
    }

    #[test]
    fn decodes_utf16_and_rejects_files_without_columns() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "[a]\nName=Über\nValue=%artist%\nHidden=1\n".encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        let import = parse(&decode(&bytes)).unwrap();
        assert_eq!(import.layout.columns[0].name, "Über");
        assert!(
            import.layout.columns[0].visible,
            "one column always stays visible"
        );
        assert_eq!(decode(b"Name=\xe9"), "Name=é");
        assert!(parse("[General]\nVersion=1\n")
            .unwrap_err()
            .contains("no column definitions"));
        assert!(parse("[a]\nValue=$nosuchfunction()\n")
            .unwrap_err()
            .starts_with("No column could be imported"));
    }
    #[test]
    fn import_column_companions_keep_raw_names_partial_and_all_rejected_english() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/import-diagnostics.json"))
                .unwrap();
        let text = fixture["columns"]["text"].as_str().unwrap();
        let report = parse_diagnostic(text).unwrap();
        let legacy = parse(text).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Help; Title.ini");
        std::fs::write(&file, text).unwrap();
        let old_command = tauri::async_runtime::block_on(import_mp3tag_columns(
            file.to_string_lossy().into_owned(),
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&old_command).unwrap(),
            serde_json::to_value(&legacy).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&report.result).unwrap(),
            serde_json::to_value(&legacy).unwrap()
        );
        assert_eq!(report.result.layout.columns.len(), 1);
        assert_eq!(report.skipped_diagnostics.len(), 2);
        for (position, item) in report.skipped_diagnostics.iter().enumerate() {
            assert_eq!(item.index, position);
            assert_eq!(item.cause.to_string(), legacy.skipped[position]);
            let json = serde_json::to_value(&item.cause).unwrap();
            assert_eq!(
                json["params"]["value"],
                fixture["columns"]["skipped_names"][position]
            );
        }
        let rejected = text
            .split("[Rejected]")
            .nth(1)
            .map(|rest| format!("[Rejected]{rest}"))
            .unwrap();
        let error = parse_diagnostic(&rejected).unwrap_err();
        assert_eq!(error.to_string(), parse(&rejected).unwrap_err());
        let wire = serde_json::to_value(&error).unwrap();
        let ipc: tauri::ipc::InvokeError = error.clone().into();
        assert_eq!(ipc.0, wire);
        assert_eq!(wire["message"], error.to_string());
        assert_eq!(
            wire["import_failure"]["children"]["error"]["parts"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
