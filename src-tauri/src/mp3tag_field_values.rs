//! Native per-field suggestions, shared by legacy panels and usrfieldvalues.ini.
use crate::mp3tag_columns::{flag, lookup, sections_raw};
use crate::tag_panel::{field_id, text, FieldValues, MAX_VALUES};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Default, Serialize)]
pub struct ImportedFieldValues {
    pub lists: BTreeMap<String, Vec<String>>,
    pub prefer_tag_fields: BTreeMap<String, bool>,
}
impl ImportedFieldValues {
    pub fn apply(&self, target: &mut FieldValues) {
        target.lists.extend(self.lists.clone());
        target
            .prefer_tag_fields
            .extend(self.prefer_tag_fields.clone());
    }
    pub fn extend(&mut self, other: Self) {
        self.lists.extend(other.lists);
        self.prefer_tag_fields.extend(other.prefer_tag_fields);
    }
}

/// U+000B separates native values. Pipes, spaces and backslashes are literal.
pub fn from_keys(
    field: &str,
    keys: &[(String, String)],
    native_record: bool,
) -> Result<ImportedFieldValues, String> {
    let custom = lookup(keys, &["custom_values"]);
    let preference = lookup(keys, &["prefer_tag"]);
    if custom.is_none() && preference.is_none() && !native_record {
        return Ok(ImportedFieldValues::default());
    }
    field_id(field, false)?;
    let mut result = ImportedFieldValues::default();
    if let Some(custom) = custom {
        let mut values = Vec::new();
        let mut seen = HashSet::new();
        for value in custom.split('\u{b}').filter(|value| !value.is_empty()) {
            text(value, "Custom value", false)?;
            if seen.insert(value) {
                if values.len() == MAX_VALUES {
                    return Err(format!(
                        "{field}: at most {MAX_VALUES} custom values are supported"
                    ));
                }
                values.push(value.to_owned());
            }
        }
        result.lists.insert(field.to_owned(), values);
    }
    let prefer = preference
        .map(|value| flag(value).ok_or("prefer_tag must be 0 or 1"))
        .transpose()?
        .unwrap_or(true);
    result.prefer_tag_fields.insert(field.to_owned(), prefer);
    Ok(result)
}

pub struct FieldValuesImport {
    pub values: ImportedFieldValues,
    pub skipped: Vec<String>,
}

/// Native flat key/value escape codec (3.36.1: 29dd28 / 2b8b90),
/// distinct from usrfieldvalues.ini's section/vertical-tab format.
fn genre_name(input: &str) -> Result<String, String> {
    fn hex(
        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
        count: usize,
    ) -> Result<u16, String> {
        (0..count).try_fold(0u16, |value, _| {
            let digit = chars
                .next()
                .and_then(|c| c.to_digit(16))
                .ok_or("Invalid hexadecimal escape in genre name")?;
            Ok(value * 16 + digit as u16)
        })
    }
    let mut units = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\\' {
            units.extend(character.encode_utf16(&mut [0; 2]).iter().copied());
            continue;
        }
        match chars.next().ok_or("Unfinished escape in genre name")? {
            '0' => units.push(0),
            'n' => units.push(10),
            'r' => units.push(13),
            't' => units.push(9),
            'z' => units.push(26),
            'u' => units.push(hex(&mut chars, 4)?),
            'x' => {
                let braced = chars.peek() == Some(&'{');
                if braced {
                    chars.next();
                }
                units.push(hex(&mut chars, if braced { 4 } else { 2 })?);
                if braced && chars.next() != Some('}') {
                    return Err("Invalid braced hexadecimal escape in genre name".into());
                }
            }
            character => units.extend(character.encode_utf16(&mut [0; 2]).iter().copied()),
        }
    }
    String::from_utf16(&units).map_err(|_| "Invalid UTF-16 escape in genre name".into())
}

pub fn parse_genres(input: &str) -> Result<FieldValuesImport, String> {
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    let mut skipped = Vec::new();
    let mut rejected = 0usize;
    let mut records = 0usize;
    for (index, line) in input.trim_start_matches('\u{feff}').split('\n').enumerate() {
        // The original removes CR before taking the first '=' token. It does
        // not trim genre names or apply ordinary INI section/comment rules.
        let line = line.replace('\r', "");
        if line.is_empty() {
            continue;
        }
        records += 1;
        let key = line.split_once('=').map_or(line.as_str(), |(key, _)| key);
        let parsed = genre_name(key).and_then(|name| {
            if name.is_empty() {
                return Err("Genre names must not be empty".into());
            }
            text(&name, "Genre name", false)?;
            Ok(name)
        });
        match parsed {
            Ok(name) => {
                if seen.insert(name.clone()) {
                    if names.len() == MAX_VALUES {
                        return Err(format!(
                            "GENRE: at most {MAX_VALUES} custom values are supported"
                        ));
                    }
                    names.push(name);
                }
            }
            Err(error) => {
                rejected += 1;
                if skipped.len() < 50 {
                    skipped.push(format!("Genre line {}: {error}", index + 1));
                }
            }
        }
    }
    if rejected > skipped.len() {
        skipped.push(format!(
            "Additional invalid genre records omitted: {}",
            rejected - skipped.len()
        ));
    }
    if names.is_empty() && records != 0 {
        return Err(format!(
            "The file contains no supported genre names; the current genre list is kept. {details}",
            details = skipped.join(" ")
        ));
    }
    if records == 0 {
        skipped.push("The empty genre list replaces existing custom genres when selected.".into());
    }
    Ok(FieldValuesImport {
        values: ImportedFieldValues {
            lists: BTreeMap::from([("GENRE".into(), names)]),
            prefer_tag_fields: BTreeMap::new(),
        },
        skipped,
    })
}

pub fn parse(input: &str) -> Result<FieldValuesImport, String> {
    let mut result = FieldValuesImport {
        values: ImportedFieldValues::default(),
        skipped: Vec::new(),
    };
    let mut seen = HashSet::new();
    for (section, keys) in sections_raw(input) {
        let Some(field) = lookup(&keys, &["field"]) else {
            continue;
        };
        let field = field.trim().to_uppercase();
        if !seen.insert(field.clone()) {
            result.skipped.push(format!(
                "{section}: duplicate field-value definition for {field}"
            ));
            continue;
        }
        match from_keys(&field, &keys, true) {
            Ok(values) => result.values.extend(values),
            Err(error) => result.skipped.push(format!("{section} ({field}): {error}")),
        }
    }
    if result.values.prefer_tag_fields.is_empty() {
        return Err("The file contains no supported custom field values.".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genres_migration_reports_invalid_records_and_never_truncates_an_oversized_list() {
        let error = parse_genres("Bad\\u0000name=\n").err().unwrap();
        assert!(error.contains("Genre line 1"));
        assert!(error.contains("control characters"));
        let excessive = (0..=MAX_VALUES)
            .map(|index| format!("Genre{index}=\n"))
            .collect::<String>();
        assert!(parse_genres(&excessive).err().unwrap().contains("5000"));
        let bad = format!("Valid=\n{}", "Bad\\u0000name=\n".repeat(1000));
        let result = parse_genres(&bad).unwrap();
        assert_eq!(result.values.lists["GENRE"], ["Valid"]);
        assert_eq!(result.skipped.len(), 51);
        assert!(result.skipped.last().unwrap().contains("950"));
    }

    #[test]
    fn field_value_migration_decodes_native_lists_without_guessing_other_separators() {
        let parsed = parse("[#0]\r\nfield=ARTIST\r\ncustom_values=\u{b}\u{b} Alice \u{b}A|B\u{b}C\\D\u{b} Alice \u{b}\r\nprefer_tag=0\r\n[#1]\r\nfield=GENRE\r\ncustom_values=\r\n").unwrap();
        assert_eq!(parsed.values.lists["ARTIST"], [" Alice ", "A|B", "C\\D"]);
        assert!(!parsed.values.prefer_tag_fields["ARTIST"]);
        assert!(parsed.values.lists["GENRE"].is_empty());
        assert!(parsed.values.prefer_tag_fields["GENRE"]);
        assert!(parsed.skipped.is_empty());
    }

    #[test]
    fn field_value_migration_keeps_unmentioned_fields_and_reports_invalid_records() {
        let parsed = parse("[a]\nfield=ARTIST\ncustom_values=Valid\n[b]\nfield=COMPOSER\ncustom_values=Bach\nprefer_tag=bogus\n[c]\nfield=_PATH\ncustom_values=No\n[d]\nfield=artist\ncustom_values=Duplicate\n").unwrap();
        assert_eq!(parsed.skipped.len(), 3);
        let mut target = FieldValues::default();
        target.lists.insert("MOOD".into(), vec!["Existing".into()]);
        target.only_user_genres = true;
        parsed.values.apply(&mut target);
        assert_eq!(target.lists["ARTIST"], ["Valid"]);
        assert_eq!(target.lists["MOOD"], ["Existing"]);
        assert!(!target.lists.contains_key("COMPOSER"));
        assert!(target.only_user_genres);
        assert!(!target.prefer_tag_values);
        assert!(from_keys("ARTIST", &[], false)
            .unwrap()
            .prefer_tag_fields
            .is_empty());
    }

    #[test]
    fn field_value_migration_refuses_oversized_lists_without_truncating_them() {
        let values = (0..=MAX_VALUES)
            .map(|index| format!("Value{index}"))
            .collect::<Vec<_>>()
            .join("\u{b}");
        let input = format!("[large]\nfield=ARTIST\ncustom_values={values}\n[valid]\nfield=MOOD\ncustom_values=Calm\n");
        let parsed = parse(&input).unwrap();
        assert_eq!(parsed.skipped.len(), 1);
        assert!(!parsed.values.lists.contains_key("ARTIST"));
        assert_eq!(parsed.values.lists["MOOD"], ["Calm"]);
    }
}
