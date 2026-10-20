//! Import of a Tag Panel field configuration from an Mp3tag file the user
//! supplies: `usrfields.ini` in Mp3tag's configuration folder, or a
//! configuration saved with the panel's Save configuration command (in
//! `data\panels`). Only the chosen file is read. Each field is one `[#n]`
//! section with these keys:
//!
//! - `name`: the label, `&` marking the Alt shortcut letter;
//! - `value`: the field, or `_SEPARATOR` (line break) / `_DIRECTORY`;
//! - `enabled`: `0` hides the field;
//! - `multiline`: `0` text, `1` multiline (with `rows`), `2` checkbox;
//! - `size`: with `version=1`, the columns out of 12 the field spans
//!   (2, 3, 4, 6, 8, 9 or 12); older files use 1–4 for full, normal, small and smaller;
//! - `default`: the default value (`< keep >`, `< blank >` or text).
//!
//! Legacy per-field `custom_values` and `prefer_tag` become Field Values settings.
use crate::import_diagnostics::ImportFailure;
use crate::mp3tag_columns::{flag, lookup};
use crate::tag_panel::{
    field_id, text, validate_layout, Control, Layout, PanelField, Size, DIRECTORY, KEEP,
    MAX_FIELDS, SEPARATOR,
};
use serde::Serialize;
use std::collections::HashSet;
use tag_core::batch_errors::{ErrorCause, IndexedCause};

#[derive(Debug, Serialize)]
pub struct TagPanelImport {
    pub panel: Layout,
    pub field_values: crate::mp3tag_field_values::ImportedFieldValues,
    /// Entries or settings left out, with the reason.
    pub skipped: Vec<String>,
    /// Companions of `skipped` that keep the entry and the reason apart, so a
    /// field label from the file stays as written and only the reason translates.
    pub skipped_diagnostics: Vec<IndexedCause>,
}

/// Record a skipped entry as `{entry}: {reason}` with both parts retained.
fn skip(
    skipped: &mut Vec<String>,
    diagnostics: &mut Vec<IndexedCause>,
    entry: &str,
    reason: ErrorCause,
) {
    let cause = ErrorCause::wrap(
        "{value}: {error}",
        [("value", entry.to_string())],
        "error",
        reason,
    );
    diagnostics.push(IndexedCause {
        index: skipped.len(),
        cause: cause.clone(),
    });
    skipped.push(cause.to_string());
}

/// Panel grid span (out of 12 columns) for an Mp3tag `size` value.
fn size(value: Option<&str>, version: i64) -> Size {
    let value = value
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0);
    let span = if version >= 1 {
        value
    } else {
        match value {
            2 => 6,
            3 => 4,
            4 => 3,
            _ => 12,
        }
    };
    match span {
        2 => Size::Tiny,
        3 => Size::Smaller,
        4 => Size::Small,
        6 => Size::Normal,
        8 => Size::Medium,
        9 => Size::Large,
        _ => Size::Full,
    }
}

#[cfg(test)]
pub fn parse(text_value: &str) -> Result<TagPanelImport, String> {
    parse_diagnostic(text_value).map_err(|error| error.to_string())
}

/// Like `parse`, but a file that yields no field fails with the retained
/// causes, so the outer message and each reason translate on their own.
pub fn parse_diagnostic(text_value: &str) -> Result<TagPanelImport, ImportFailure> {
    let mut found: Vec<(usize, Vec<(String, String)>)> =
        crate::mp3tag_columns::sections_raw(text_value)
            .into_iter()
            .map(|(section, keys)| {
                (
                    section,
                    keys.into_iter()
                        .map(|(key, value)| {
                            let value = if key == "custom_values" {
                                value
                            } else {
                                value.trim().to_owned()
                            };
                            (key, value)
                        })
                        .collect(),
                )
            })
            .enumerate()
            .filter_map(|(position, (section, keys))| {
                let index = section.strip_prefix('#')?.trim().parse::<usize>().ok();
                Some((index.unwrap_or(usize::MAX - 1_000_000 + position), keys))
            })
            .collect();
    // Sections are numbered; a file edited by hand may list them out of order.
    found.sort_by_key(|(index, _)| *index);
    let mut fields = Vec::new();
    let mut skipped = Vec::new();
    let mut skipped_diagnostics = Vec::new();
    let mut used = HashSet::new();
    let mut field_values = crate::mp3tag_field_values::ImportedFieldValues::default();
    for (index, keys) in found {
        let Some(raw_field) = lookup(&keys, &["value", "field"]) else {
            continue;
        };
        let id = raw_field.trim().to_uppercase();
        let label = lookup(&keys, &["name"]).unwrap_or("").to_string();
        let shown = if label.is_empty() {
            id.clone()
        } else {
            label.clone()
        };
        let entry = if index < usize::MAX / 2 {
            format!("Field {index} ({shown})")
        } else {
            shown.clone()
        };
        if id.is_empty() {
            skip(
                &mut skipped,
                &mut skipped_diagnostics,
                &entry,
                ErrorCause::owned("no field name"),
            );
            continue;
        }
        if let Err(error) = field_id(&id, true) {
            skip(
                &mut skipped,
                &mut skipped_diagnostics,
                &entry,
                ErrorCause::owned_text(error),
            );
            continue;
        }
        let separator = id == SEPARATOR;
        if !separator && !used.insert(id.clone()) {
            skip(
                &mut skipped,
                &mut skipped_diagnostics,
                &entry,
                ErrorCause::message("{id} is already on the panel", [("id", id.clone())]),
            );
            continue;
        }
        if fields.len() >= MAX_FIELDS {
            skip(
                &mut skipped,
                &mut skipped_diagnostics,
                &entry,
                ErrorCause::message(
                    "the tag panel holds at most {MAX_FIELDS} fields",
                    [("MAX_FIELDS", MAX_FIELDS.to_string())],
                ),
            );
            continue;
        }
        let version = lookup(&keys, &["version"])
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        let control = match lookup(&keys, &["multiline"]).map(str::trim) {
            Some("1") => Control::Multiline,
            Some("2") => Control::Check,
            _ => Control::Text,
        };
        let rows = lookup(&keys, &["rows"])
            .and_then(|v| v.trim().parse::<i64>().ok())
            .map(|rows| rows.clamp(1, 20) as u8)
            .unwrap_or(3);
        let default_value = match lookup(&keys, &["default"]) {
            Some(value) if !value.trim().is_empty() && !separator && id != DIRECTORY => {
                value.to_string()
            }
            _ => KEEP.to_string(),
        };
        let name = if separator {
            label
        } else if label.trim().is_empty() {
            id.clone()
        } else {
            label
        };
        if let Err(error) = text(&name, "Field label", false).and_then(|()| {
            text(
                &default_value,
                "Default value",
                control == Control::Multiline,
            )
        }) {
            skip(
                &mut skipped,
                &mut skipped_diagnostics,
                &entry,
                ErrorCause::owned_text(error),
            );
            continue;
        }
        match crate::mp3tag_field_values::from_keys(&id, &keys, false) {
            Ok(values) => field_values.extend(values),
            Err(reason) => skip(
                &mut skipped,
                &mut skipped_diagnostics,
                &entry,
                ErrorCause::owned_text(reason),
            ),
        }
        fields.push(PanelField {
            name,
            field: id,
            control,
            size: size(lookup(&keys, &["size"]), version),
            rows,
            default_value,
            visible: lookup(&keys, &["enabled"]).and_then(flag).unwrap_or(true),
        });
    }
    if fields.iter().all(|field| field.field == SEPARATOR) {
        return Err(if skipped.is_empty() {
            ImportFailure::parts(ErrorCause::owned("The file contains no tag panel fields. Choose usrfields.ini from the Mp3tag configuration folder, or a saved tag panel configuration."))
        } else {
            ImportFailure::parts(ErrorCause::children(
                "No field could be imported: {error}",
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
    let panel = Layout {
        fields,
        show_panel: true,
        show_cover: true,
    };
    validate_layout(&panel).map_err(ImportFailure::Plain)?;
    Ok(TagPanelImport {
        panel,
        field_values,
        skipped,
        skipped_diagnostics,
    })
}

#[tauri::command]
pub async fn import_mp3tag_tag_panel(path: String) -> Result<TagPanelImport, ImportFailure> {
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
            .and_then(|bytes| parse_diagnostic(&crate::mp3tag_columns::decode(&bytes))))
    })
    .await
    .map_err(ImportFailure::Plain)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_value_migration_keeps_native_delimiters_and_each_fields_preference() {
        let text = "[#0]\nname=Artist\nvalue=ARTIST\ncustom_values= Alice \u{b}A|B\u{b}東京\nprefer_tag=0\n[#1]\nvalue=COMPOSER\ncustom_values=Bach\u{b}Mozart\n";
        let parsed = parse(text).unwrap();
        let wire = serde_json::to_value(&parsed).unwrap();
        assert_eq!(
            wire["field_values"]["lists"]["ARTIST"],
            serde_json::json!([" Alice ", "A|B", "東京"])
        );
        assert_eq!(wire["field_values"]["prefer_tag_fields"]["ARTIST"], false);
        assert_eq!(wire["field_values"]["prefer_tag_fields"]["COMPOSER"], true);
        assert!(parsed.skipped.is_empty());
    }

    // Written by hand in the documented key layout; no Mp3tag file is used.
    const CURRENT: &str = "[#0]\r\nname=Tit&le\r\nvalue=TITLE\r\nsize=12\r\nenabled=1\r\ndefault=< keep >\r\nmultiline=0\r\nversion=1\r\n\
        [#2]\r\nname=T&rack\r\nvalue=TRACK\r\nsize=6\r\nenabled=1\r\nmultiline=0\r\nversion=1\r\n\
        [#1]\r\nname=&Year\r\nvalue=YEAR\r\nsize=6\r\nversion=1\r\n\
        [#3]\r\nname=\r\nvalue=_SEPARATOR\r\nversion=1\r\n\
        [#4]\r\nname=Co&mment\r\nvalue=COMMENT\r\nsize=12\r\nmultiline=1\r\nrows=5\r\nversion=1\r\n\
        [#5]\r\nname=Compilation\r\nvalue=compilation\r\nsize=4\r\nmultiline=2\r\ndefault=1\r\nversion=1\r\n\
        [#6]\r\nname=Mood\r\nvalue=MOOD\r\nsize=9\r\nenabled=0\r\ndefault=< blank >\r\nversion=1\r\n\
        [#7]\r\nname=Path\r\nvalue=_PATH\r\nversion=1\r\n\
        [#8]\r\nname=Title again\r\nvalue=TITLE\r\nversion=1\r\n\
        [#9]\r\nname=&Directory\r\nvalue=_DIRECTORY\r\nsize=8\r\nversion=1\r\n";

    #[test]
    fn reads_fields_in_section_order_with_types_sizes_and_defaults() {
        let import = parse(CURRENT).unwrap();
        let fields = &import.panel.fields;
        let ids: Vec<_> = fields.iter().map(|f| f.field.as_str()).collect();
        assert_eq!(
            ids,
            [
                "TITLE",
                "YEAR",
                "TRACK",
                "_SEPARATOR",
                "COMMENT",
                "COMPILATION",
                "MOOD",
                "_DIRECTORY"
            ]
        );
        assert_eq!(
            (fields[0].name.as_str(), fields[0].size),
            ("Tit&le", Size::Full)
        );
        assert_eq!(
            (fields[1].size, fields[2].size),
            (Size::Normal, Size::Normal)
        );
        assert_eq!(fields[3].name, "");
        assert_eq!((fields[4].control, fields[4].rows), (Control::Multiline, 5));
        assert_eq!(
            (
                fields[5].control,
                fields[5].size,
                fields[5].default_value.as_str()
            ),
            (Control::Check, Size::Small, "1")
        );
        assert_eq!(
            (
                fields[6].visible,
                fields[6].size,
                fields[6].default_value.as_str()
            ),
            (false, Size::Large, "< blank >")
        );
        assert_eq!(fields[7].size, Size::Medium);
        assert!(fields[1].default_value == KEEP && fields[7].default_value == KEEP);
        assert_eq!(import.skipped.len(), 2, "{:?}", import.skipped);
        assert!(
            import.skipped[0].contains("Field 7 (Path)") && import.skipped[0].contains("technical")
        );
        assert!(import.skipped[1].contains("already on the panel"));
    }

    #[test]
    fn older_files_use_the_four_legacy_sizes_and_import_custom_values() {
        let text = "[#0]\nname=Artist\nvalue=ARTIST\nsize=1\n[#1]\nname=Year\nvalue=YEAR\nsize=2\n\
            [#2]\nname=Disc\nvalue=DISCNUMBER\nsize=3\n[#3]\nname=BPM\nvalue=BPM\nsize=4\n\
            [#4]\nname=Genre\nvalue=GENRE\ncustom_values=Rock\u{b}Jazz\nprefer_tag=1\n[#5]\nvalue=ISRC\n";
        let import = parse(text).unwrap();
        let sizes: Vec<_> = import.panel.fields.iter().map(|f| f.size).collect();
        assert_eq!(
            sizes,
            [
                Size::Full,
                Size::Normal,
                Size::Small,
                Size::Smaller,
                Size::Full,
                Size::Full
            ]
        );
        // A field without a label shows its field name.
        assert_eq!(import.panel.fields[5].name, "ISRC");
        assert!(import.skipped.is_empty());
        assert_eq!(import.field_values.lists["GENRE"], ["Rock", "Jazz"]);
        assert!(import.field_values.prefer_tag_fields["GENRE"]);
    }

    #[test]
    fn a_file_with_no_importable_field_fails_with_the_retained_reasons() {
        let text = "[#0]\nname=Mood\nvalue=\n[#1]\nname=Copy\nvalue=_BOGUS?\n";
        let partial =
            "[#0]\nname=Mood\nvalue= \n[#1]\nname=Genre\nvalue=GENRE\ncustom_values=Rock\n";
        // A skip that still leaves a field is a partial import, not a failure.
        assert!(parse_diagnostic(partial).is_ok());
        let error = parse_diagnostic(text).unwrap_err();
        // The wire message is exactly the legacy English string.
        assert_eq!(error.to_string(), parse(text).unwrap_err());
        assert!(error
            .to_string()
            .starts_with("No field could be imported: "));
        let wire = serde_json::to_value(&error).unwrap();
        let ipc: tauri::ipc::InvokeError = error.clone().into();
        assert_eq!(ipc.0, wire);
        assert_eq!(wire["message"], error.to_string());
        assert_eq!(
            wire["import_failure"]["key"],
            "No field could be imported: {error}"
        );
        let parts = wire["import_failure"]["children"]["error"]["parts"]
            .as_array()
            .unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["params"]["value"], "Field 0 (Mood)");
        assert_eq!(parts[0]["children"]["error"]["key"], "no field name");
        let empty = parse_diagnostic("[General]\nVersion=1\n").unwrap_err();
        assert_eq!(
            serde_json::to_value(&empty).unwrap()["import_failure"]["key"],
            "The file contains no tag panel fields. Choose usrfields.ini from the Mp3tag configuration folder, or a saved tag panel configuration."
        );
    }

    #[test]
    fn skipped_entries_keep_the_authored_entry_apart_from_the_owned_reason() {
        let text = "[#0]\nname=Genre\nvalue=GENRE\ncustom_values=Rock\tJazz\nprefer_tag=1\n\
            [#1]\nname=Mood\nvalue=GENRE\n";
        let import = parse(text).unwrap();
        assert_eq!(import.skipped.len(), 2, "{:?}", import.skipped);
        assert_eq!(import.skipped_diagnostics.len(), 2);
        let expected = [
            (
                "Field 0 (Genre)",
                "Custom value contains control characters",
            ),
            ("Field 1 (Mood)", "{id} is already on the panel"),
        ];
        for (position, ((line, item), (entry, reason))) in import
            .skipped
            .iter()
            .zip(&import.skipped_diagnostics)
            .zip(expected)
            .enumerate()
        {
            assert_eq!(item.index, position);
            // The legacy English line is the rendering of the retained parts.
            assert_eq!(&item.cause.to_string(), line);
            let ErrorCause::Owned {
                key,
                params,
                children,
            } = &item.cause
            else {
                panic!("{:?}", item.cause);
            };
            assert_eq!(key, "{value}: {error}");
            assert_eq!(params["value"], entry, "the authored entry stays verbatim");
            match &children["error"] {
                ErrorCause::Owned { key, .. } => assert_eq!(key, reason),
                ErrorCause::OwnedText { value } => assert_eq!(value, reason),
                other => panic!("the reason must remain owned: {other:?}"),
            }
        }
    }

    #[test]
    fn utf16_files_decode_and_files_without_fields_are_rejected() {
        let utf16: Vec<u8> = [0xff, 0xfe]
            .into_iter()
            .chain(
                "[#0]\r\nname=K&ünstler\r\nvalue=ARTIST\r\nsize=12\r\nversion=1\r\n"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes),
            )
            .collect();
        let import = parse(&crate::mp3tag_columns::decode(&utf16)).unwrap();
        assert_eq!(import.panel.fields[0].name, "K&ünstler");
        assert!(parse("[General]\nVersion=3\n")
            .unwrap_err()
            .contains("usrfields.ini"));
        assert!(parse("[#0]\nname=Path\nvalue=_PATH\n")
            .unwrap_err()
            .contains("technical"));
        assert!(parse("[#0]\nvalue=_SEPARATOR\n").is_err());
    }
}
