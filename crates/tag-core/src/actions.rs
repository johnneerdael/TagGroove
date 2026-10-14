//! Independently authored action catalog and ordered action-group execution.
//! References: https://docs.mp3tag.de/actions/ and its individual action pages.
//! Native effects are explicit: a caller must execute them or return an error.
use crate::{
    script::{evaluate, Action},
    tags::Fields,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 1_048_576;
pub const ACTION_CATALOG: [(&str, &str); 18] = [
    ("adjust_cover", "Adjust Cover"),
    ("case_conversion", "Case Conversion"),
    ("convert_codepage", "Convert Codepage"),
    ("export", "Export"),
    ("export_cover", "Export Cover to File"),
    ("format_value", "Format Value"),
    ("generate_playlists", "Generate Playlists"),
    ("guess_values", "Guess Values"),
    ("import_cover", "Import Cover from File"),
    ("import_text_file", "Import Text File"),
    ("merge_duplicate_fields", "Merge Duplicate Fields"),
    ("remove_duplicate_fields", "Remove Duplicate Fields"),
    ("remove_fields", "Remove Fields"),
    ("remove_fields_except", "Remove Fields Except"),
    ("replace", "Replace"),
    ("regex_replace", "Replace with Regular Expression"),
    ("set_cover_properties", "Set Cover Properties"),
    ("split_field", "Split Field by Separator"),
];
/// Additional numeric IDs corroborated by original `.mta` reports on the
/// official forum: /t/testing-mp3tag-behaviour/42469/22. Property schemas require
/// separate evidence; a known ID alone does not make an import executable.
pub const MTA_TYPE_REGISTRY: [(u32, &str); 15] = [
    (1, "Case Conversion"),
    (2, "Replace"),
    (4, "Replace with Regular Expression"),
    (5, "Format Value"),
    (6, "Remove Duplicate Fields"),
    (7, "Guess Values"),
    (8, "Merge Duplicate Fields"),
    (9, "Remove Fields"),
    (10, "Remove Fields Except"),
    (11, "Convert Codepage"),
    (12, "Import Cover from File"),
    (13, "Export Cover to File"),
    (14, "Import Text File"),
    (15, "Export"),
    (16, "Split Field by Separator"),
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    Lower,
    Mixed,
    Sentence,
    Upper,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverImageFormat {
    Original,
    Jpeg,
    Png,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverSizeMode {
    Maximum,
    Minimum,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverScaleMode {
    Proportional,
    Square,
    CropSquare,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverResize {
    pub pixels: u32,
    pub size_mode: CoverSizeMode,
    pub scale_mode: CoverScaleMode,
}

/// One variant for every action type listed in the official documentation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionSpec {
    AdjustCover {
        format: CoverImageFormat,
        quality: u8,
        resize: Option<CoverResize>,
    },
    CaseConversion {
        field: String,
        mode: CaseMode,
        word_separators: String,
    },
    ConvertCodepage {
        field: String,
        codepage: String,
    },
    Export {
        template: String,
        filename_template: String,
        append: bool,
        partition_template: Option<String>,
    },
    ExportCover {
        filename_template: String,
        picture_type: Option<String>,
        export_duplicates: bool,
    },
    FormatValue {
        field: String,
        template: String,
    },
    GeneratePlaylists {
        filename_template: String,
        partition_template: String,
    },
    GuessValues {
        source: String,
        pattern: String,
    },
    ImportCover {
        filename_template: String,
    },
    ImportTextFile {
        field: String,
        filename_template: String,
    },
    MergeDuplicateFields {
        field: String,
        separator: String,
    },
    RemoveDuplicateFields {
        field: String,
        only_same_content: bool,
    },
    RemoveFields {
        fields: Vec<String>,
    },
    RemoveFieldsExcept {
        fields: Vec<String>,
    },
    Replace {
        field: String,
        from: String,
        to: String,
        whole_word: bool,
        case_sensitive: bool,
    },
    RegexReplace {
        field: String,
        pattern: String,
        replacement: String,
        case_sensitive: bool,
    },
    SetCoverProperties {
        picture_type: Option<String>,
        description: Option<String>,
        only_picture_type: Option<String>,
    },
    SplitField {
        field: String,
        separator: String,
    },
    /// Preserve unresolved imported settings rather than silently skipping them.
    UnresolvedMta {
        type_id: String,
        properties: BTreeMap<String, String>,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionGroup {
    pub name: String,
    pub enabled: bool,
    pub actions: Vec<ActionSpec>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionPreset {
    pub name: String,
    pub enabled_groups: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionLibrary {
    pub groups: Vec<ActionGroup>,
    pub presets: Vec<SelectionPreset>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionState {
    pub fields: Fields,
    pub context: Fields,
}
impl ActionState {
    pub fn evaluation_fields(&self) -> Fields {
        let mut fields = self.context.clone();
        fields.extend(self.fields.clone());
        fields
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionEffect {
    RenameFile {
        filename: String,
        includes_extension: bool,
    },
    RenameDirectory {
        directory: String,
    },
    RemoveCovers,
    ImportCover {
        filename: String,
    },
    ExportCover {
        filename: String,
        picture_type: Option<String>,
        export_duplicates: bool,
    },
    AdjustCover {
        format: CoverImageFormat,
        quality: u8,
        resize: Option<CoverResize>,
    },
    SetCoverProperties {
        picture_type: Option<String>,
        description: Option<String>,
        only_picture_type: Option<String>,
    },
    ImportTextFile {
        field: String,
        filename: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BatchActionEffect {
    Export {
        template: String,
        filename_template: String,
        append: bool,
        partition_template: Option<String>,
    },
    GeneratePlaylists {
        filename_template: String,
        partition_template: String,
    },
}
/// The native adapter supplies file, image, text and set-of-files execution.
/// Imports and renames must update state before the next action is evaluated.
pub trait ActionHost {
    /// Override with actual application EvalContext (columns, paths, locale,
    /// Windows ANSI codepage), rebuilding tag context after each action.
    fn evaluate(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        evaluate(template, &state.evaluation_fields())
    }
    fn execute(&mut self, effect: &ActionEffect, state: &mut ActionState) -> Result<(), String>;
    fn execute_batch(
        &mut self,
        effect: &BatchActionEffect,
        states: &mut [ActionState],
    ) -> Result<(), String>;
}

fn checked(value: String) -> Result<String, String> {
    if value.len() > LIMIT {
        Err("Action output limit exceeded".into())
    } else {
        Ok(value)
    }
}
fn actual_key(fields: &Fields, name: &str) -> String {
    fields
        .keys()
        .find(|key| key.eq_ignore_ascii_case(name))
        .cloned()
        .unwrap_or_else(|| name.to_uppercase())
}
fn is_cover(name: &str) -> bool {
    name.eq_ignore_ascii_case("COVER") || name.eq_ignore_ascii_case("PICTURE")
}
fn metadata_keys(fields: &Fields, selector: &str) -> Result<Vec<String>, String> {
    if selector.trim().is_empty() {
        return Err("Action field must not be empty".into());
    }
    if selector.eq_ignore_ascii_case("_TAG") || selector.eq_ignore_ascii_case("_ALL") {
        return Ok(fields.keys().cloned().collect());
    }
    if selector.starts_with('_') {
        return Err(format!(
            "Field selector {selector} requires a filename, directory or field-name action"
        ));
    }
    Ok(vec![actual_key(fields, selector)])
}
fn put(fields: &mut Fields, key: String, values: Vec<String>) {
    if values.is_empty() {
        fields.remove(&key);
    } else {
        fields.insert(key, values);
    }
}
fn case_value(value: &str, mode: CaseMode, separators: &str) -> String {
    match mode {
        CaseMode::Lower => value.to_lowercase(),
        CaseMode::Upper => value.to_uppercase(),
        CaseMode::Sentence => {
            let mut chars = value.chars();
            chars
                .next()
                .map(|first| {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                })
                .unwrap_or_default()
        }
        CaseMode::Mixed => {
            let mut start = true;
            let mut out = String::new();
            for c in value.chars() {
                if start {
                    out.extend(c.to_uppercase())
                } else {
                    out.extend(c.to_lowercase())
                }
                start = c.is_whitespace() || separators.contains(c);
            }
            out
        }
    }
}
fn replace_values(
    values: &[String],
    from: &str,
    to: &str,
    whole_word: bool,
    case_sensitive: bool,
) -> Result<Vec<String>, String> {
    if from.is_empty() {
        return Err("Replacement search must not be empty".into());
    }
    let re = regex::RegexBuilder::new(&regex::escape(from))
        .case_insensitive(!case_sensitive)
        .size_limit(8 * LIMIT)
        .build()
        .map_err(|error| error.to_string())?;
    let boundary = |c: char| c.is_whitespace() || ".,;:!?".contains(c);
    values
        .iter()
        .map(|value| {
            let mut out = String::new();
            let mut end = 0;
            for found in re.find_iter(value) {
                let left = value[..found.start()].chars().next_back();
                let right = value[found.end()..].chars().next();
                if whole_word && (!left.is_none_or(boundary) || !right.is_none_or(boundary)) {
                    continue;
                }
                out.push_str(&value[end..found.start()]);
                out.push_str(to);
                end = found.end();
            }
            out.push_str(&value[end..]);
            checked(out)
        })
        .collect()
}
fn regex_values(
    values: &[String],
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Result<Vec<String>, String> {
    let expression = regex_compat::Regex::new(pattern, case_sensitive)?;
    values
        .iter()
        .map(|value| expression.replace(value, replacement).and_then(checked))
        .collect()
}
fn transform<F>(
    state: &mut ActionState,
    selector: &str,
    filename_extension: bool,
    mut operation: F,
) -> Result<Vec<ActionEffect>, String>
where
    F: FnMut(&[String]) -> Result<Vec<String>, String>,
{
    let mut effects = Vec::new();
    let mut fields = state.fields.clone();
    if selector.eq_ignore_ascii_case("_FIELDNAME") {
        let format = state
            .context
            .get("_TAG")
            .and_then(|v| v.first())
            .map(String::as_str)
            .unwrap_or("");
        if !["APEv2", "VorbisComments"]
            .iter()
            .any(|candidate| format.eq_ignore_ascii_case(candidate))
        {
            return Err(
                "Field-name actions require native APEv2 or VorbisComments format context".into(),
            );
        }
        let mut renamed = Fields::new();
        for (key, values) in &state.fields {
            let names = operation(std::slice::from_ref(key))?;
            if names.len() != 1 || names[0].is_empty() {
                return Err("Field-name transformation must produce one nonempty name".into());
            }
            let name = &names[0];
            if renamed.keys().any(|key| key.eq_ignore_ascii_case(name)) {
                return Err(format!("Field-name transformation collision: {name}"));
            }
            renamed.insert(name.clone(), values.clone());
        }
        fields = renamed;
    } else {
        let includes_tags = selector.eq_ignore_ascii_case("_TAG")
            || selector.eq_ignore_ascii_case("_ALL")
            || !selector.starts_with('_');
        if includes_tags {
            for key in metadata_keys(&fields, selector)? {
                if let Some(values) = fields.get(&key) {
                    let values = operation(values)?;
                    put(&mut fields, key, values);
                }
            }
        }
        if selector.eq_ignore_ascii_case("_FILENAME") || selector.eq_ignore_ascii_case("_ALL") {
            let source = if filename_extension {
                "_FILENAME_EXT"
            } else {
                "_FILENAME"
            };
            let values = state
                .context
                .get(source)
                .ok_or_else(|| format!("Missing {source} action context"))?;
            let value = operation(values)?;
            if value.len() != 1 || value[0].is_empty() {
                return Err("Filename transformation must produce one nonempty name".into());
            }
            effects.push(ActionEffect::RenameFile {
                filename: value[0].clone(),
                includes_extension: filename_extension,
            });
        } else if selector.eq_ignore_ascii_case("_DIRECTORY") {
            let values = state
                .context
                .get("_DIRECTORY")
                .ok_or("Missing _DIRECTORY action context")?;
            let value = operation(values)?;
            if value.len() != 1 || value[0].is_empty() {
                return Err("Directory transformation must produce one nonempty name".into());
            }
            effects.push(ActionEffect::RenameDirectory {
                directory: value[0].clone(),
            });
        } else if selector.starts_with('_')
            && !selector.eq_ignore_ascii_case("_TAG")
            && !selector.eq_ignore_ascii_case("_FIELDNAME")
        {
            return Err(format!("Unsupported action selector {selector}"));
        }
    }
    state.fields = fields;
    Ok(effects)
}

/// Match an entire source string without filename extension or path processing.
pub fn guess_values(source: &str, pattern: &str) -> Result<Fields, String> {
    if source.len() > LIMIT || pattern.len() > LIMIT {
        return Err("Guess Values input limit exceeded".into());
    }
    let mut expression = String::from("\\A");
    let mut names = Vec::new();
    let mut rest = pattern;
    let mut prior_capture = false;
    while let Some(start) = rest.find('%') {
        let literal = &rest[..start];
        if prior_capture && literal.is_empty() {
            return Err("Adjacent guessing placeholders are ambiguous".into());
        }
        expression.push_str(&regex::escape(literal));
        rest = &rest[start + 1..];
        let end = rest.find('%').ok_or("Unterminated guessing placeholder")?;
        let name = &rest[..end];
        if name.is_empty() {
            return Err("Empty guessing placeholder".into());
        }
        names.push(name.to_uppercase());
        expression.push_str("(.*?)");
        rest = &rest[end + 1..];
        prior_capture = true;
    }
    if names.is_empty() {
        return Err("Guessing pattern needs at least one field placeholder".into());
    }
    expression.push_str(&regex::escape(rest));
    expression.push_str("\\z");
    let expression = regex::RegexBuilder::new(&expression)
        .dot_matches_new_line(true)
        .size_limit(8 * LIMIT)
        .build()
        .map_err(|error| error.to_string())?;
    let captures = expression
        .captures(source)
        .ok_or("Source does not completely match the guessing pattern")?;
    let mut result = Fields::new();
    for (index, name) in names.into_iter().enumerate() {
        if name != "DUMMY" {
            result
                .entry(name)
                .or_default()
                .push(captures[index + 1].into());
        }
    }
    Ok(result)
}

fn decode_codepage(value: &str, codepage: &str) -> Result<String, String> {
    let normalized = codepage.trim().to_ascii_lowercase();
    let normalized = normalized.strip_prefix("cp").unwrap_or(&normalized);
    // Numeric identifiers follow Microsoft's Code Page Identifiers table.
    let label = match normalized {
        "65001" => "utf-8",
        "1200" => "utf-16le",
        "1201" => "utf-16be",
        "1250" => "windows-1250",
        "1251" => "windows-1251",
        "1252" => "windows-1252",
        "1253" => "windows-1253",
        "1254" => "windows-1254",
        "1255" => "windows-1255",
        "1256" => "windows-1256",
        "1257" => "windows-1257",
        "1258" => "windows-1258",
        "866" => "ibm866",
        "874" => "windows-874",
        "932" => "shift_jis",
        "936" => "gbk",
        "949" => "euc-kr",
        "950" => "big5",
        "10000" => "macintosh",
        "10007" => "x-mac-cyrillic",
        "20866" => "koi8-r",
        "21866" => "koi8-u",
        "20932" | "51932" => "euc-jp",
        "51949" => "euc-kr",
        "54936" => "gb18030",
        "28591" => "iso-8859-1",
        "28592" => "iso-8859-2",
        "28593" => "iso-8859-3",
        "28594" => "iso-8859-4",
        "28595" => "iso-8859-5",
        "28596" => "iso-8859-6",
        "28597" => "iso-8859-7",
        "28598" => "iso-8859-8",
        "28603" => "iso-8859-13",
        "28605" => "iso-8859-15",
        "20127" => "us-ascii",
        other => other,
    };
    // Preserve legacy byte values. Windows-1252 glyphs above U+00FF are encoded
    // back to their byte only when they have a lossless mapping.
    let mut bytes = Vec::new();
    for c in value.chars() {
        if u32::from(c) <= 255 {
            bytes.push(c as u8);
        } else {
            let text = c.to_string();
            let (encoded, _, lossy) = encoding_rs::WINDOWS_1252.encode(&text);
            if lossy {
                return Err(format!("Character {c:?} has no legacy byte representation"));
            }
            bytes.extend_from_slice(&encoded);
        }
    }
    // WHATWG aliases ISO-8859-1 and ASCII to Windows-1252; Windows codepage
    // semantics require their actual byte mappings instead.
    if ["iso-8859-1", "latin1", "latin-1"].contains(&label) {
        return Ok(bytes.into_iter().map(char::from).collect());
    }
    if ["us-ascii", "ascii"].contains(&label) {
        if bytes.iter().any(|byte| *byte > 127) {
            return Err("Invalid non-ASCII byte for US-ASCII codepage".into());
        }
        return String::from_utf8(bytes).map_err(|error| error.to_string());
    }
    let encoding = encoding_rs::Encoding::for_label(label.as_bytes())
        .ok_or_else(|| format!("Unsupported codepage {codepage}"))?;
    encoding
        .decode_without_bom_handling_and_without_replacement(&bytes)
        .map(|value| value.into_owned())
        .ok_or_else(|| format!("Invalid byte sequence for codepage {codepage}"))
}

pub fn validate_action(action: &ActionSpec) -> Result<(), String> {
    match action {
        ActionSpec::UnresolvedMta {
            type_id, reason, ..
        } => Err(format!(
            "Unresolved imported action type {type_id}: {reason}"
        )),
        ActionSpec::AdjustCover {
            quality, resize, ..
        } if *quality > 100 || resize.as_ref().is_some_and(|s| s.pixels == 0) => {
            Err("Cover quality must be 0–100 and resize pixels must be positive".into())
        }
        ActionSpec::Replace { from, .. } if from.is_empty() => {
            Err("Replacement search must not be empty".into())
        }
        ActionSpec::SplitField { separator, .. } if separator.is_empty() => {
            Err("Split separator must not be empty".into())
        }
        ActionSpec::ImportTextFile { field, .. }
        | ActionSpec::FormatValue { field, .. }
        | ActionSpec::CaseConversion { field, .. }
        | ActionSpec::ConvertCodepage { field, .. }
        | ActionSpec::MergeDuplicateFields { field, .. }
        | ActionSpec::RemoveDuplicateFields { field, .. }
        | ActionSpec::SplitField { field, .. }
        | ActionSpec::Replace { field, .. }
        | ActionSpec::RegexReplace { field, .. }
            if field.trim().is_empty() =>
        {
            Err("Action field must not be empty".into())
        }
        ActionSpec::RegexReplace {
            pattern,
            replacement,
            case_sensitive,
            ..
        } => regex_values(&[], pattern, replacement, *case_sensitive).map(|_| ()),
        ActionSpec::ConvertCodepage { codepage, .. } => decode_codepage("", codepage).map(|_| ()),
        _ => Ok(()),
    }
}

/// Apply one action's metadata changes and return mandatory native effects.
pub fn apply_action_spec(
    state: &mut ActionState,
    action: &ActionSpec,
) -> Result<Vec<ActionEffect>, String> {
    apply_action_spec_with_evaluator(state, action, &mut |template, state| {
        evaluate(template, &state.evaluation_fields())
    })
}

pub fn apply_action_spec_with_evaluator<F>(
    state: &mut ActionState,
    action: &ActionSpec,
    evaluate: &mut F,
) -> Result<Vec<ActionEffect>, String>
where
    F: FnMut(&str, &ActionState) -> Result<String, String>,
{
    validate_action(action)?;
    match action {
        ActionSpec::CaseConversion {
            field,
            mode,
            word_separators,
        } => transform(state, field, false, |values| {
            values
                .iter()
                .map(|value| checked(case_value(value, *mode, word_separators)))
                .collect()
        }),
        ActionSpec::ConvertCodepage { field, codepage } => {
            transform(state, field, false, |values| {
                values
                    .iter()
                    .map(|value| decode_codepage(value, codepage).and_then(checked))
                    .collect()
            })
        }
        ActionSpec::Replace {
            field,
            from,
            to,
            whole_word,
            case_sensitive,
        } => transform(state, field, false, |values| {
            replace_values(values, from, to, *whole_word, *case_sensitive)
        }),
        ActionSpec::RegexReplace {
            field,
            pattern,
            replacement,
            case_sensitive,
        } => transform(state, field, true, |values| {
            regex_values(values, pattern, replacement, *case_sensitive)
        }),
        ActionSpec::FormatValue { field, template } => {
            let result = evaluate(template, state)?;
            if field.eq_ignore_ascii_case("_FILENAME") {
                if result.is_empty() {
                    return Err("Filename must not be empty".into());
                }
                return Ok(vec![ActionEffect::RenameFile {
                    filename: result,
                    includes_extension: false,
                }]);
            }
            if field.eq_ignore_ascii_case("_DIRECTORY") {
                if result.is_empty() {
                    return Err("Directory must not be empty".into());
                }
                return Ok(vec![ActionEffect::RenameDirectory { directory: result }]);
            }
            if field.starts_with('_') {
                return Err(format!("Unsupported Format Value target {field}"));
            }
            let key = actual_key(&state.fields, field);
            put(
                &mut state.fields,
                key,
                if result.is_empty() {
                    vec![]
                } else {
                    vec![result]
                },
            );
            Ok(vec![])
        }
        ActionSpec::GuessValues { source, pattern } => {
            let result = guess_values(&evaluate(source, state)?, pattern)?;
            let mut effects = Vec::new();
            let mut fields = state.fields.clone();
            for (name, values) in result {
                if name == "_FILENAME" || name == "_DIRECTORY" {
                    if values.len() != 1 || values[0].is_empty() {
                        return Err(format!("Guess Values must produce one nonempty {name}"));
                    }
                    effects.push(if name == "_FILENAME" {
                        ActionEffect::RenameFile {
                            filename: values[0].clone(),
                            includes_extension: false,
                        }
                    } else {
                        ActionEffect::RenameDirectory {
                            directory: values[0].clone(),
                        }
                    });
                } else if name.starts_with('_') {
                    return Err(format!("Unsupported Guess Values target {name}"));
                } else {
                    let key = actual_key(&fields, &name);
                    put(&mut fields, key, values);
                }
            }
            state.fields = fields;
            Ok(effects)
        }
        ActionSpec::MergeDuplicateFields { field, separator } => {
            let mut fields = state.fields.clone();
            for key in metadata_keys(&fields, field)? {
                if let Some(values) = fields.get(&key) {
                    if values.len() > 1 {
                        let merged = checked(values.join(separator))?;
                        fields.insert(key, vec![merged]);
                    }
                }
            }
            state.fields = fields;
            Ok(vec![])
        }
        ActionSpec::RemoveDuplicateFields {
            field,
            only_same_content,
        } => {
            let mut fields = state.fields.clone();
            for key in metadata_keys(&fields, field)? {
                if let Some(values) = fields.get_mut(&key) {
                    if *only_same_content {
                        let mut seen = BTreeSet::new();
                        values.retain(|value| seen.insert(value.clone()));
                    } else {
                        values.truncate(1);
                    }
                }
            }
            state.fields = fields;
            Ok(vec![])
        }
        ActionSpec::SplitField { field, separator } => {
            let mut fields = state.fields.clone();
            for key in metadata_keys(&fields, field)? {
                if let Some(values) = fields.get(&key) {
                    let split = values
                        .iter()
                        .flat_map(|value| value.split(separator).map(|part| part.trim().to_owned()))
                        .collect();
                    fields.insert(key, split);
                }
            }
            state.fields = fields;
            Ok(vec![])
        }
        ActionSpec::RemoveFields { fields } => {
            let mut effects = Vec::new();
            for name in fields {
                if is_cover(name) {
                    if !effects.contains(&ActionEffect::RemoveCovers) {
                        effects.push(ActionEffect::RemoveCovers);
                    }
                } else {
                    state.fields.remove(&actual_key(&state.fields, name));
                }
            }
            Ok(effects)
        }
        ActionSpec::RemoveFieldsExcept { fields } => {
            let keep: BTreeSet<String> = fields.iter().map(|name| name.to_uppercase()).collect();
            state
                .fields
                .retain(|name, _| keep.contains(&name.to_uppercase()));
            Ok(if fields.iter().any(|name| is_cover(name)) {
                vec![]
            } else {
                vec![ActionEffect::RemoveCovers]
            })
        }
        ActionSpec::ImportCover { filename_template } => Ok(vec![ActionEffect::ImportCover {
            filename: evaluate(filename_template, state)?,
        }]),
        ActionSpec::ExportCover {
            filename_template,
            picture_type,
            export_duplicates,
        } => Ok(vec![ActionEffect::ExportCover {
            filename: evaluate(filename_template, state)?,
            picture_type: picture_type.clone(),
            export_duplicates: *export_duplicates,
        }]),
        ActionSpec::ImportTextFile {
            field,
            filename_template,
        } => {
            if field.starts_with('_') {
                return Err("Import Text File requires a metadata field".into());
            }
            Ok(vec![ActionEffect::ImportTextFile {
                field: actual_key(&state.fields, field),
                filename: evaluate(filename_template, state)?,
            }])
        }
        ActionSpec::AdjustCover {
            format,
            quality,
            resize,
        } => Ok(vec![ActionEffect::AdjustCover {
            format: *format,
            quality: *quality,
            resize: resize.clone(),
        }]),
        ActionSpec::SetCoverProperties {
            picture_type,
            description,
            only_picture_type,
        } => Ok(vec![ActionEffect::SetCoverProperties {
            picture_type: picture_type.clone(),
            description: description.clone(),
            only_picture_type: only_picture_type.clone(),
        }]),
        ActionSpec::Export { .. } | ActionSpec::GeneratePlaylists { .. } => {
            Err("Set-of-files actions must execute through the group runner".into())
        }
        ActionSpec::UnresolvedMta { .. } => unreachable!("validated above"),
    }
}

pub fn execute_groups<H: ActionHost>(
    states: &mut [ActionState],
    groups: &[ActionGroup],
    host: &mut H,
) -> Result<(), String> {
    // Unknown imported actions fail before any host effect can mutate files.
    for group in groups.iter().filter(|group| group.enabled) {
        for action in &group.actions {
            validate_action(action).map_err(|error| format!("{}: {error}", group.name))?;
        }
    }
    for group in groups.iter().filter(|group| group.enabled) {
        let mut deferred = Vec::new();
        for (index, action) in group.actions.iter().enumerate() {
            let batch = match action {
                ActionSpec::Export {
                    template,
                    filename_template,
                    append,
                    partition_template,
                } => Some(BatchActionEffect::Export {
                    template: template.clone(),
                    filename_template: filename_template.clone(),
                    append: *append,
                    partition_template: partition_template.clone(),
                }),
                ActionSpec::GeneratePlaylists {
                    filename_template,
                    partition_template,
                } => Some(BatchActionEffect::GeneratePlaylists {
                    filename_template: filename_template.clone(),
                    partition_template: partition_template.clone(),
                }),
                _ => None,
            };
            if let Some(batch) = batch {
                deferred.push((index, batch));
                continue;
            }
            for state in states.iter_mut() {
                let effects =
                    apply_action_spec_with_evaluator(state, action, &mut |template, state| {
                        host.evaluate(template, state)
                    })
                    .map_err(|error| format!("{} / action {}: {error}", group.name, index + 1))?;
                for effect in effects {
                    host.execute(&effect, state).map_err(|error| {
                        format!("{} / action {}: {error}", group.name, index + 1)
                    })?;
                }
            }
        }
        for (index, effect) in deferred {
            host.execute_batch(&effect, states)
                .map_err(|error| format!("{} / action {}: {error}", group.name, index + 1))?;
        }
    }
    Ok(())
}

impl ActionLibrary {
    fn ensure_name(&self, name: &str, except: Option<usize>) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Action group name must not be empty".into());
        }
        if self
            .groups
            .iter()
            .enumerate()
            .any(|(index, group)| Some(index) != except && group.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("Action group already exists: {name}"));
        }
        Ok(())
    }
    pub fn add_group(&mut self, group: ActionGroup) -> Result<usize, String> {
        self.ensure_name(&group.name, None)?;
        self.groups.push(group);
        Ok(self.groups.len() - 1)
    }
    pub fn duplicate_group(&mut self, index: usize, name: String) -> Result<usize, String> {
        let mut group = self
            .groups
            .get(index)
            .ok_or("Action group index out of range")?
            .clone();
        group.name = name;
        self.add_group(group)
    }
    pub fn remove_group(&mut self, index: usize) -> Result<ActionGroup, String> {
        if index >= self.groups.len() {
            return Err("Action group index out of range".into());
        }
        let group = self.groups.remove(index);
        for preset in &mut self.presets {
            preset.enabled_groups.retain(|name| name != &group.name);
        }
        Ok(group)
    }
    pub fn rename_group(&mut self, index: usize, name: String) -> Result<(), String> {
        if index >= self.groups.len() {
            return Err("Action group index out of range".into());
        }
        self.ensure_name(&name, Some(index))?;
        let old = std::mem::replace(&mut self.groups[index].name, name.clone());
        for preset in &mut self.presets {
            for selected in &mut preset.enabled_groups {
                if *selected == old {
                    *selected = name.clone();
                }
            }
        }
        Ok(())
    }
    pub fn move_group(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.groups.len() || to >= self.groups.len() {
            return Err("Action group index out of range".into());
        }
        let group = self.groups.remove(from);
        self.groups.insert(to, group);
        Ok(())
    }
    pub fn move_action(&mut self, group: usize, from: usize, to: usize) -> Result<(), String> {
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        if from >= actions.len() || to >= actions.len() {
            return Err("Action index out of range".into());
        }
        let action = actions.remove(from);
        actions.insert(to, action);
        Ok(())
    }
    pub fn add_action(&mut self, group: usize, action: ActionSpec) -> Result<usize, String> {
        validate_action(&action)?;
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        actions.push(action);
        Ok(actions.len() - 1)
    }
    pub fn edit_action(
        &mut self,
        group: usize,
        index: usize,
        action: ActionSpec,
    ) -> Result<(), String> {
        validate_action(&action)?;
        let old = self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions
            .get_mut(index)
            .ok_or("Action index out of range")?;
        *old = action;
        Ok(())
    }
    pub fn remove_action(&mut self, group: usize, index: usize) -> Result<ActionSpec, String> {
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        if index >= actions.len() {
            return Err("Action index out of range".into());
        }
        Ok(actions.remove(index))
    }
    pub fn set_group_enabled(&mut self, index: usize, enabled: bool) -> Result<(), String> {
        self.groups
            .get_mut(index)
            .ok_or("Action group index out of range")?
            .enabled = enabled;
        Ok(())
    }
    pub fn duplicate_action(&mut self, group: usize, index: usize) -> Result<usize, String> {
        let actions = &mut self
            .groups
            .get_mut(group)
            .ok_or("Action group index out of range")?
            .actions;
        let action = actions
            .get(index)
            .ok_or("Action index out of range")?
            .clone();
        actions.insert(index + 1, action);
        Ok(index + 1)
    }
    pub fn save_selection_preset(&mut self, name: String) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Preset name must not be empty".into());
        }
        let enabled_groups = self
            .groups
            .iter()
            .filter(|group| group.enabled)
            .map(|group| group.name.clone())
            .collect();
        let preset = SelectionPreset {
            name,
            enabled_groups,
        };
        if let Some(index) = self.presets.iter().position(|old| old.name == preset.name) {
            self.presets[index] = preset;
        } else {
            self.presets.push(preset);
        }
        Ok(())
    }
    pub fn apply_selection_preset(&mut self, name: &str) -> Result<(), String> {
        let preset = self
            .presets
            .iter()
            .find(|preset| preset.name == name)
            .ok_or("Selection preset not found")?;
        for group in &mut self.groups {
            group.enabled = preset.enabled_groups.contains(&group.name);
        }
        Ok(())
    }
    pub fn remove_selection_preset(&mut self, name: &str) -> Result<(), String> {
        let index = self
            .presets
            .iter()
            .position(|preset| preset.name == name)
            .ok_or("Selection preset not found")?;
        self.presets.remove(index);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionMenuSegment {
    pub label: String,
    pub mnemonic: Option<char>,
}
/// '#' gives submenu segments; '&' marks each segment's mnemonic.
pub fn group_menu_segments(name: &str) -> Vec<ActionMenuSegment> {
    name.split('#')
        .map(|part| {
            let mut chars = part.chars().peekable();
            let mut label = String::new();
            let mut mnemonic = None;
            while let Some(c) = chars.next() {
                if c == '&' {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        label.push('&');
                    } else if let Some(next) = chars.next() {
                        if mnemonic.is_none() {
                            mnemonic = Some(next.to_lowercase().next().unwrap_or(next));
                        }
                        label.push(next);
                    } else {
                        label.push(c);
                    }
                } else {
                    label.push(c);
                }
            }
            ActionMenuSegment { label, mnemonic }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MtaRecord {
    pub index: usize,
    pub properties: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MtaIssue {
    pub index: usize,
    pub type_id: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MtaImport {
    pub group: ActionGroup,
    pub records: Vec<MtaRecord>,
    pub issues: Vec<MtaIssue>,
}

fn mta_value(value: &str) -> Result<String, String> {
    let mut out = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '\\' && index + 1 < chars.len() {
            if chars[index + 1] == '\\' {
                out.push('\\');
                index += 2;
                continue;
            }
            let count = match chars[index + 1] {
                'x' => 2,
                'u' => 4,
                _ => 0,
            };
            if count > 0
                && index + 2 + count <= chars.len()
                && chars[index + 2..index + 2 + count]
                    .iter()
                    .all(char::is_ascii_hexdigit)
            {
                let digits: String = chars[index + 2..index + 2 + count].iter().collect();
                let value =
                    u32::from_str_radix(&digits, 16).map_err(|_| "Invalid MTA Unicode escape")?;
                out.push(char::from_u32(value).ok_or("MTA Unicode escape is not a scalar value")?);
                index += 2 + count;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    Ok(out)
}
fn mta_action(record: &MtaRecord) -> Result<ActionSpec, String> {
    let required = |name: &str| {
        record
            .properties
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Missing MTA setting {name}"))
    };
    let type_id = required("T")?;
    let field = required("F")?;
    match type_id.as_str() {
        "1" => {
            if record
                .properties
                .keys()
                .any(|key| !["T", "F", "1", "2"].contains(&key.as_str()))
            {
                return Err("Unverified Case Conversion settings".into());
            }
            // Only mode 1 is present in the supplied 3.36.1 action files.
            if required("1")? != "1" {
                return Err("Case mode ID is absent from the supplied artifact evidence".into());
            }
            Ok(ActionSpec::CaseConversion {
                field,
                mode: CaseMode::Mixed,
                word_separators: required("2")?,
            })
        }
        "2" => {
            if record
                .properties
                .keys()
                .any(|key| !["T", "F", "1", "2", "3"].contains(&key.as_str()))
            {
                return Err("Unverified Replace settings".into());
            }
            let flags = required("3")?;
            let (whole_word, case_sensitive) = match flags.as_str() {
                "0" | "0|0" => (false, false),
                "1|0" => (true, false),
                "0|1" => (false, true),
                "1|1" => (true, true),
                _ => return Err(format!("Unknown Replace flags {flags:?}")),
            };
            Ok(ActionSpec::Replace {
                field,
                from: required("1")?,
                to: required("2")?,
                whole_word,
                case_sensitive,
            })
        }
        "5" | "7" | "8" | "16" => {
            if record
                .properties
                .keys()
                .any(|key| !["T", "F", "1"].contains(&key.as_str()))
            {
                return Err(format!("Unverified settings for action type {type_id}"));
            }
            let value = required("1")?;
            Ok(match type_id.as_str() {
                "5" => ActionSpec::FormatValue {
                    field,
                    template: value,
                },
                "7" => ActionSpec::GuessValues {
                    source: field,
                    pattern: value,
                },
                "8" => ActionSpec::MergeDuplicateFields {
                    field,
                    separator: value,
                },
                "16" => ActionSpec::SplitField {
                    field,
                    separator: value,
                },
                _ => unreachable!(),
            })
        }
        "9" | "10" => {
            if record
                .properties
                .keys()
                .any(|key| !["T", "F"].contains(&key.as_str()))
            {
                return Err(format!("Unverified removal settings for type {type_id}"));
            }
            let fields = field
                .split(';')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect();
            Ok(if type_id == "9" {
                ActionSpec::RemoveFields { fields }
            } else {
                ActionSpec::RemoveFieldsExcept { fields }
            })
        }
        _ => {
            let known = type_id.parse::<u32>().ok().and_then(|id| {
                MTA_TYPE_REGISTRY
                    .iter()
                    .find(|(candidate, _)| *candidate == id)
            });
            Err(if let Some((_, name)) = known {
                format!("{name} type ID is verified; its imported option schema remains unresolved")
            } else {
                format!("Numeric action type {type_id} has no verified registry evidence")
            })
        }
    }
}
/// Read UTF-8 or BOM-marked UTF-16. Legacy encodings require explicit selection.
/// Unknown type IDs/settings become preserved, non-executable records.
pub fn import_mta(
    name: String,
    bytes: &[u8],
    legacy_encoding: Option<&str>,
) -> Result<MtaImport, String> {
    if bytes.len() > 4 * LIMIT {
        return Err("Action group import limit exceeded".into());
    }
    let (encoding, input) = if bytes.starts_with(&[0xff, 0xfe]) {
        (encoding_rs::UTF_16LE, &bytes[2..])
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (encoding_rs::UTF_16BE, &bytes[2..])
    } else if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        (encoding_rs::UTF_8, &bytes[3..])
    } else if let Some(label) = legacy_encoding {
        (
            encoding_rs::Encoding::for_label(label.as_bytes())
                .ok_or("Unsupported action-file encoding")?,
            bytes,
        )
    } else {
        (encoding_rs::UTF_8, bytes)
    };
    let text = encoding
        .decode_without_bom_handling_and_without_replacement(input)
        .ok_or("Invalid action-file encoding; choose its original legacy codepage")?;
    let mut records: BTreeMap<usize, MtaRecord> = BTreeMap::new();
    let mut current = None;
    for (line_index, line) in text.lines().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty()
            || line.trim_start().starts_with('!')
            || line.trim_start().starts_with(';')
        {
            continue;
        }
        if line.starts_with("[#") && line.ends_with(']') {
            let index = line[2..line.len() - 1]
                .parse::<usize>()
                .map_err(|_| format!("Invalid action section at line {}", line_index + 1))?;
            if records
                .insert(
                    index,
                    MtaRecord {
                        index,
                        properties: BTreeMap::new(),
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate action section #{index}"));
            }
            current = Some(index);
            continue;
        }
        let index = current.ok_or_else(|| {
            format!(
                "Setting outside an action section at line {}",
                line_index + 1
            )
        })?;
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("Invalid action setting at line {}", line_index + 1))?;
        if key.trim().is_empty() {
            return Err("Empty action setting name".into());
        }
        if records
            .get_mut(&index)
            .unwrap()
            .properties
            .insert(key.trim().into(), mta_value(value)?)
            .is_some()
        {
            return Err(format!("Duplicate setting {key} in action #{index}"));
        }
    }
    let records: Vec<MtaRecord> = records.into_values().collect();
    if records.is_empty() {
        return Err("Action group contains no actions".into());
    }
    let mut actions = Vec::new();
    let mut issues = Vec::new();
    for record in &records {
        match mta_action(record) {
            Ok(action) => actions.push(action),
            Err(reason) => {
                let type_id = record.properties.get("T").cloned().unwrap_or_default();
                issues.push(MtaIssue {
                    index: record.index,
                    type_id: type_id.clone(),
                    reason: reason.clone(),
                });
                actions.push(ActionSpec::UnresolvedMta {
                    type_id,
                    properties: record.properties.clone(),
                    reason,
                });
            }
        }
    }
    Ok(MtaImport {
        group: ActionGroup {
            name,
            enabled: issues.is_empty(),
            actions,
        },
        records,
        issues,
    })
}

/// Adapter for existing five-type Quick Actions requests during migration.
pub fn from_quick_action(action: &Action) -> Result<ActionSpec, String> {
    Ok(match action {
        Action::Format { field, template } => ActionSpec::FormatValue {
            field: field.clone(),
            template: template.clone(),
        },
        Action::Replace {
            field,
            from,
            to,
            case_sensitive,
        } => ActionSpec::Replace {
            field: field.clone(),
            from: from.clone(),
            to: to.clone(),
            case_sensitive: *case_sensitive,
            whole_word: false,
        },
        Action::RegexReplace {
            field,
            pattern,
            replacement,
        } => ActionSpec::RegexReplace {
            field: field.clone(),
            pattern: pattern.clone(),
            replacement: replacement.clone(),
            case_sensitive: true,
        },
        Action::Case { field, mode } => ActionSpec::CaseConversion {
            field: field.clone(),
            mode: match mode.as_str() {
                "lower" => CaseMode::Lower,
                "upper" => CaseMode::Upper,
                "title" => CaseMode::Mixed,
                "sentence" => CaseMode::Sentence,
                _ => return Err(format!("Unsupported case mode {mode}")),
            },
            word_separators: String::new(),
        },
        Action::Remove { fields } => ActionSpec::RemoveFields {
            fields: fields.clone(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> ActionState {
        ActionState {
            fields: Fields::from([
                (
                    "ARTIST".into(),
                    vec!["Ada".into(), "Bob".into(), "Ada".into()],
                ),
                ("TITLE".into(), vec!["mIXED (aRTIST-title_name".into()]),
                ("COMMENT".into(), vec!["one\ntwo".into()]),
            ]),
            context: Fields::from([
                ("_FILENAME".into(), vec!["old".into()]),
                ("_FILENAME_EXT".into(), vec!["old.flac".into()]),
                ("_DIRECTORY".into(), vec!["album".into()]),
            ]),
        }
    }
    #[derive(Default)]
    struct TestHost {
        events: Vec<String>,
    }
    impl ActionHost for TestHost {
        fn execute(
            &mut self,
            effect: &ActionEffect,
            state: &mut ActionState,
        ) -> Result<(), String> {
            self.events.push(format!("{effect:?}"));
            match effect {
                ActionEffect::ImportTextFile { field, .. } => {
                    state
                        .fields
                        .insert(field.clone(), vec!["imported\ntext".into()]);
                }
                ActionEffect::RenameFile { filename, .. } => {
                    state
                        .context
                        .insert("_FILENAME".into(), vec![filename.clone()]);
                }
                _ => {}
            }
            Ok(())
        }
        fn execute_batch(
            &mut self,
            effect: &BatchActionEffect,
            states: &mut [ActionState],
        ) -> Result<(), String> {
            self.events
                .push(format!("{effect:?} title={}", states[0].fields["TITLE"][0]));
            Ok(())
        }
    }
    #[test]
    fn mixed_case_uses_configured_delimiters_and_whitespace() {
        let mut state = state();
        apply_action_spec(
            &mut state,
            &ActionSpec::CaseConversion {
                field: "TITLE".into(),
                mode: CaseMode::Mixed,
                word_separators: "(-_".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["TITLE"], ["Mixed (Artist-Title_Name"]);
    }
    #[test]
    fn merge_and_split_preserve_order_and_trim_split_values() {
        let mut state = state();
        apply_action_spec(
            &mut state,
            &ActionSpec::MergeDuplicateFields {
                field: "ARTIST".into(),
                separator: " ; ".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["ARTIST"], ["Ada ; Bob ; Ada"]);
        apply_action_spec(
            &mut state,
            &ActionSpec::SplitField {
                field: "ARTIST".into(),
                separator: ";".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["ARTIST"], ["Ada", "Bob", "Ada"]);
        assert_eq!(state.fields["COMMENT"], ["one\ntwo"]);
    }
    #[test]
    fn duplicate_fields_same_content_differs_from_keep_first() {
        let mut same = state();
        apply_action_spec(
            &mut same,
            &ActionSpec::RemoveDuplicateFields {
                field: "ARTIST".into(),
                only_same_content: true,
            },
        )
        .unwrap();
        assert_eq!(same.fields["ARTIST"], ["Ada", "Bob"]);
        let mut first = state();
        apply_action_spec(
            &mut first,
            &ActionSpec::RemoveDuplicateFields {
                field: "_ALL".into(),
                only_same_content: false,
            },
        )
        .unwrap();
        assert_eq!(first.fields["ARTIST"], ["Ada"]);
    }
    #[test]
    fn remove_except_explicitly_preserves_or_removes_covers() {
        let mut first = state();
        let effects = apply_action_spec(
            &mut first,
            &ActionSpec::RemoveFieldsExcept {
                fields: vec!["artist".into(), "picture".into()],
            },
        )
        .unwrap();
        assert!(effects.is_empty());
        assert_eq!(first.fields.len(), 1);
        let mut second = state();
        let effects = apply_action_spec(
            &mut second,
            &ActionSpec::RemoveFieldsExcept {
                fields: vec!["TITLE".into()],
            },
        )
        .unwrap();
        assert_eq!(effects, [ActionEffect::RemoveCovers]);
    }
    #[test]
    fn cover_aliases_produce_one_effect() {
        let mut state = state();
        let effects = apply_action_spec(
            &mut state,
            &ActionSpec::RemoveFields {
                fields: vec!["COMMENT".into(), "cover".into(), "PICTURE".into()],
            },
        )
        .unwrap();
        assert!(!state.fields.contains_key("COMMENT"));
        assert_eq!(effects, [ActionEffect::RemoveCovers]);
    }
    #[test]
    fn whole_word_boundaries_match_documented_punctuation() {
        let values = vec!["DJ, dj! xdj DJ-name DJ_name (DJ) DJ.".into()];
        assert_eq!(
            replace_values(&values, "DJ", "X", true, false).unwrap(),
            ["X, X! xdj DJ-name DJ_name (DJ) X."]
        );
    }
    #[test]
    fn regex_filename_uses_extension_while_literal_uses_stem() {
        let mut first = state();
        let effects = apply_action_spec(
            &mut first,
            &ActionSpec::RegexReplace {
                field: "_FILENAME".into(),
                pattern: "\\.flac$".into(),
                replacement: ".FLAC".into(),
                case_sensitive: true,
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameFile {
                filename: "old.FLAC".into(),
                includes_extension: true
            }]
        );
        let mut second = state();
        let effects = apply_action_spec(
            &mut second,
            &ActionSpec::Replace {
                field: "_ALL".into(),
                from: "old".into(),
                to: "new".into(),
                whole_word: false,
                case_sensitive: true,
            },
        )
        .unwrap();
        assert_eq!(
            effects,
            [ActionEffect::RenameFile {
                filename: "new".into(),
                includes_extension: false
            }]
        );
    }
    #[test]
    fn regex_references_and_case_flag_reuse_legacy_expander() {
        let mut state = state();
        state.fields.insert("TITLE".into(), vec!["ABC-12".into()]);
        apply_action_spec(
            &mut state,
            &ActionSpec::RegexReplace {
                field: "TITLE".into(),
                pattern: "(abc)-(\\d+)".into(),
                replacement: "$2:$1suffix".into(),
                case_sensitive: false,
            },
        )
        .unwrap();
        assert_eq!(state.fields["TITLE"], ["12:ABCsuffix"]);
    }
    #[test]
    fn regex_action_supports_native_boost_lookaround_backrefs_and_case_replacement() {
        assert_eq!(
            regex_values(&["abc abc".into()], r"\b(\w+) \1\b", r"\U$1\E", true).unwrap(),
            ["ABC"]
        );
        assert_eq!(
            regex_values(&["cat dog".into()], r"(?<=\b)cat(?=\b)", "X", true).unwrap(),
            ["X dog"]
        );
    }
    #[test]
    fn guess_values_handles_paths_dots_dummy_and_repeated_values() {
        assert_eq!(
            guess_values(
                "C:/Ada/Bob. live / skip / ignored",
                "%artist% / %title% / %dummy%"
            )
            .unwrap(),
            Fields::from([
                ("ARTIST".into(), vec!["C:/Ada/Bob. live".into()]),
                ("TITLE".into(), vec!["skip".into()])
            ])
        );
    }
    #[test]
    fn guess_values_has_no_filename_extension_processing() {
        let fields = guess_values("Artist / A.title.flac", "%artist% / %title%").unwrap();
        assert_eq!(fields["TITLE"], ["A.title.flac"]);
        let fields = guess_values("A + B", "%artist% + %artist%").unwrap();
        assert_eq!(fields["ARTIST"], ["A", "B"]);
        assert!(guess_values("anything", "%artist%%title%").is_err());
    }
    #[test]
    fn guess_source_and_target_can_reference_same_field() {
        let mut state = state();
        state
            .fields
            .insert("TITLE".into(), vec!["Ada / Song".into()]);
        apply_action_spec(
            &mut state,
            &ActionSpec::GuessValues {
                source: "%title%".into(),
                pattern: "%artist% / %title%".into(),
            },
        )
        .unwrap();
        assert_eq!(state.fields["ARTIST"], ["Ada"]);
        assert_eq!(state.fields["TITLE"], ["Song"]);
    }
    #[test]
    fn codepage_conversion_recovers_known_legacy_bytes_losslessly() {
        assert_eq!(
            decode_codepage("\u{cf}\u{f0}\u{e8}\u{e2}\u{e5}\u{f2}", "1251").unwrap(),
            "Привет"
        );
        assert_eq!(decode_codepage("\u{c3}\u{a9}", "65001").unwrap(), "é");
        assert!(decode_codepage("💿", "1251").is_err());
        assert!(decode_codepage("abc", "not-a-codepage").is_err());
        assert_eq!(decode_codepage("\u{80}", "28591").unwrap(), "\u{80}");
        assert_eq!(decode_codepage("\u{80}", "1252").unwrap(), "€");
        assert!(decode_codepage("\u{80}", "20127").is_err());
    }
    #[test]
    fn context_sensitive_action_evaluation_uses_real_app_options() {
        let mut state = state();
        let columns = vec![crate::script::ScriptColumn {
            name: "Title".into(),
            value: "Song".into(),
        }];
        let mut evaluate = |template: &str, state: &ActionState| {
            let fields = state.evaluation_fields();
            let mut context = crate::script::EvalContext::new(&fields);
            context.columns = Some(&columns);
            context.app_directory = Some("/applications/opentag");
            crate::script::evaluate_with_context(template, &context)
        };
        apply_action_spec_with_evaluator(
            &mut state,
            &ActionSpec::FormatValue {
                field: "COMMENT".into(),
                template: "$columnnames('<','>'):$getenv(MP3TAGAPP)".into(),
            },
            &mut evaluate,
        )
        .unwrap();
        assert_eq!(state.fields["COMMENT"], ["<Title>:/applications/opentag"]);
    }
    #[test]
    fn group_action_management_preserves_order_and_settings() {
        let mut library = ActionLibrary::default();
        let index = library
            .add_group(ActionGroup {
                name: "Workflow".into(),
                enabled: true,
                actions: vec![],
            })
            .unwrap();
        let a = ActionSpec::FormatValue {
            field: "TITLE".into(),
            template: "A".into(),
        };
        let b = ActionSpec::FormatValue {
            field: "TITLE".into(),
            template: "B".into(),
        };
        library.add_action(index, a.clone()).unwrap();
        library.add_action(index, b.clone()).unwrap();
        library.duplicate_action(index, 0).unwrap();
        library.move_action(index, 2, 0).unwrap();
        assert_eq!(
            library.groups[index].actions,
            vec![b.clone(), a.clone(), a.clone()]
        );
        library.edit_action(index, 1, b.clone()).unwrap();
        assert_eq!(library.remove_action(index, 2).unwrap(), a);
        library.set_group_enabled(index, false).unwrap();
        assert!(!library.groups[index].enabled);
        assert_eq!(library.groups[index].actions, vec![b.clone(), b]);
        assert!(library
            .add_group(ActionGroup {
                name: "workflow".into(),
                enabled: true,
                actions: vec![]
            })
            .is_err());
    }
    #[test]
    fn groups_execute_enabled_order_and_defer_batch_actions() {
        let mut states = vec![state()];
        let groups = vec![
            ActionGroup {
                name: "first".into(),
                enabled: true,
                actions: vec![
                    ActionSpec::Export {
                        template: "test".into(),
                        filename_template: "out.txt".into(),
                        append: false,
                        partition_template: None,
                    },
                    ActionSpec::FormatValue {
                        field: "TITLE".into(),
                        template: "updated".into(),
                    },
                    ActionSpec::GeneratePlaylists {
                        filename_template: "list.m3u8".into(),
                        partition_template: "%album%".into(),
                    },
                ],
            },
            ActionGroup {
                name: "disabled".into(),
                enabled: false,
                actions: vec![ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "wrong".into(),
                }],
            },
        ];
        let mut host = TestHost::default();
        execute_groups(&mut states, &groups, &mut host).unwrap();
        assert_eq!(states[0].fields["TITLE"], ["updated"]);
        assert_eq!(host.events.len(), 2);
        assert!(host.events[0].starts_with("Export"));
        assert!(host
            .events
            .iter()
            .all(|event| event.ends_with("title=updated")));
    }
    #[test]
    fn imported_text_is_available_to_later_actions() {
        let mut states = vec![state()];
        let group = ActionGroup {
            name: "import".into(),
            enabled: true,
            actions: vec![
                ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: "%_directory%/lyrics.txt".into(),
                },
                ActionSpec::FormatValue {
                    field: "COMMENT".into(),
                    template: "%lyrics%".into(),
                },
            ],
        };
        let mut host = TestHost::default();
        execute_groups(&mut states, &[group], &mut host).unwrap();
        assert_eq!(states[0].fields["COMMENT"], ["imported\ntext"]);
        assert!(host.events[0].contains("album/lyrics.txt"));
    }
    #[test]
    fn unresolved_import_preflight_prevents_earlier_native_effects() {
        let mut states = vec![state()];
        let group = ActionGroup {
            name: "unsupported".into(),
            enabled: true,
            actions: vec![
                ActionSpec::ImportCover {
                    filename_template: "cover.jpg".into(),
                },
                ActionSpec::UnresolvedMta {
                    type_id: "99".into(),
                    properties: BTreeMap::new(),
                    reason: "unknown ID".into(),
                },
            ],
        };
        let mut host = TestHost::default();
        assert!(execute_groups(&mut states, &[group], &mut host).is_err());
        assert!(host.events.is_empty());
    }
    #[test]
    fn cover_effects_preserve_keep_and_empty_description_distinction() {
        let mut state = state();
        let effect = apply_action_spec(
            &mut state,
            &ActionSpec::SetCoverProperties {
                picture_type: None,
                description: Some(String::new()),
                only_picture_type: Some("Front Cover".into()),
            },
        )
        .unwrap();
        assert_eq!(
            effect,
            [ActionEffect::SetCoverProperties {
                picture_type: None,
                description: Some(String::new()),
                only_picture_type: Some("Front Cover".into())
            }]
        );
    }
    #[test]
    fn invalid_operations_do_not_mutate_fields() {
        let mut state = state();
        let original = state.clone();
        assert!(apply_action_spec(
            &mut state,
            &ActionSpec::SplitField {
                field: "ARTIST".into(),
                separator: String::new()
            }
        )
        .is_err());
        assert_eq!(state, original);
        assert!(apply_action_spec(
            &mut state,
            &ActionSpec::RegexReplace {
                field: "ABSENT".into(),
                pattern: "(".into(),
                replacement: String::new(),
                case_sensitive: true
            }
        )
        .is_err());
        assert_eq!(state, original);
    }
    #[test]
    fn mta_preserves_values_section_order_and_decodes_observed_escapes() {
        let imported=import_mta("Original test".into(),b"[#1]\r\nT=2\r\nF=ARTIST\r\n1=\\xdf\r\n2=ss \r\n3=0\r\n\r\n[#0]\r\nT=1\r\nF=_ALL\r\n1=1\r\n2=(-_\r\n",None).unwrap();
        assert!(imported.issues.is_empty());
        assert!(matches!(
            &imported.group.actions[0],
            ActionSpec::CaseConversion { .. }
        ));
        assert!(
            matches!(&imported.group.actions[1],ActionSpec::Replace{from,to,..}if from=="ß"&&to=="ss ")
        );
    }
    #[test]
    fn mta_unknown_types_flags_and_modes_are_retained_without_guessing() {
        for text in [
            "[#0]\nT=44\nF=TITLE\n1=test",
            "[#0]\nT=1\nF=TITLE\n1=3\n2=",
            "[#0]\nT=2\nF=TITLE\n1=a\n2=b\n3=2|1",
        ] {
            let import = import_mta("Unknown".into(), text.as_bytes(), None).unwrap();
            assert!(!import.group.enabled);
            assert_eq!(import.issues.len(), 1);
            assert!(matches!(
                import.group.actions[0],
                ActionSpec::UnresolvedMta { .. }
            ));
        }
        assert!(import_mta("Duplicate".into(), b"[#0]\nT=1\nT=2", None).is_err());
    }
    #[test]
    fn mta_utf16_is_detected_only_by_bom() {
        let text = "[#0]\r\nT=1\r\nF=TITLE\r\n1=1\r\n2=\r\n";
        let bytes: Vec<u8> = std::iter::once(0xfeffu16)
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(import_mta("UTF16".into(), &bytes, None)
            .unwrap()
            .issues
            .is_empty());
    }
    #[test]
    fn mta_verified_extra_schemas_import_without_type_guessing() {
        let input=b"[#0]\nT=5\nF=TITLE\n1=%artist%\n[#1]\nT=7\nF=%title%\n1=%artist% / %title%\n[#2]\nT=8\nF=ARTIST\n1=; \n[#3]\nT=9\nF=COMMENT;PICTURE\n[#4]\nT=10\nF=ARTIST;COVER\n[#5]\nT=16\nF=ARTIST\n1=;\n";
        let import = import_mta("Original syntax".into(), input, None).unwrap();
        assert!(import.issues.is_empty());
        assert_eq!(import.group.actions.len(), 6);
        assert!(
            matches!(&import.group.actions[3],ActionSpec::RemoveFields{fields}if fields==&["COMMENT","PICTURE"])
        );
    }
    #[test]
    fn mta_verified_replace_flags_and_escaped_backslashes_are_preserved() {
        let import = import_mta(
            "Flags".into(),
            b"[#0]\nT=2\nF=TITLE\n1=Word\n2=\\\\folder\n3=1|1\n",
            None,
        )
        .unwrap();
        assert!(
            matches!(&import.group.actions[0],ActionSpec::Replace{whole_word:true,case_sensitive:true,to,..}if to==r"\folder")
        );
        assert_eq!(mta_value(r"\\x24").unwrap(), r"\x24");
    }
    #[test]
    fn presets_capture_only_selection_and_follow_group_rename() {
        let mut library = ActionLibrary::default();
        let a = library
            .add_group(ActionGroup {
                name: "A".into(),
                enabled: true,
                actions: vec![],
            })
            .unwrap();
        library
            .add_group(ActionGroup {
                name: "B".into(),
                enabled: false,
                actions: vec![],
            })
            .unwrap();
        library.save_selection_preset("Only A".into()).unwrap();
        library.rename_group(a, "Renamed".into()).unwrap();
        library.groups[0].enabled = false;
        library.groups[1].enabled = true;
        library.apply_selection_preset("Only A").unwrap();
        assert!(library.groups[0].enabled);
        assert!(!library.groups[1].enabled);
        library.move_group(0, 1).unwrap();
        assert_eq!(library.groups[1].name, "Renamed");
        library.remove_group(1).unwrap();
        assert!(library.presets[0].enabled_groups.is_empty());
    }
    #[test]
    fn menu_names_support_documented_submenu_mnemonics() {
        assert_eq!(
            group_menu_segments("&Default values#Set &COMMENT"),
            vec![
                ActionMenuSegment {
                    label: "Default values".into(),
                    mnemonic: Some('d')
                },
                ActionMenuSegment {
                    label: "Set COMMENT".into(),
                    mnemonic: Some('c')
                }
            ]
        );
    }
    #[test]
    fn action_specs_serde_roundtrip_without_silent_unknown_settings() {
        let action = ActionSpec::AdjustCover {
            format: CoverImageFormat::Jpeg,
            quality: 90,
            resize: Some(CoverResize {
                pixels: 600,
                size_mode: CoverSizeMode::Maximum,
                scale_mode: CoverScaleMode::CropSquare,
            }),
        };
        let text = serde_json::to_string(&action).unwrap();
        assert_eq!(serde_json::from_str::<ActionSpec>(&text).unwrap(), action);
        assert!(serde_json::from_str::<ActionSpec>(
            r#"{"type":"format_value","field":"TITLE","template":"text","unknown":true}"#
        )
        .is_err());
    }
}
