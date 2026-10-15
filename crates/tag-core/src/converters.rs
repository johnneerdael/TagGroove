//! Pure filename rearrangement and text-file tag import planning.
use crate::{
    decode_export_template, CompiledFilenamePattern, CompiledTemplate, EvalContext, Fields,
};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

const MAX_MASK_BYTES: usize = 1_048_576;
const MAX_TEXT_BYTES: usize = 256 * 1_048_576;

pub struct FilenameConverter {
    old_pattern: CompiledFilenamePattern,
    new_pattern: CompiledTemplate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilenameConversion {
    pub source: String,
    pub target: String,
    pub captures: Fields,
}

impl FilenameConverter {
    pub fn compile(old_pattern: &str, new_pattern: &str) -> Result<Self, String> {
        let (old_pattern, parts) = numbered_pattern(old_pattern, None)?;
        if parts.is_empty() {
            return Err("Old filename pattern must contain numbered parts %1 through %9".into());
        }
        let (new_pattern, _) = numbered_pattern(new_pattern, Some(&parts))?;
        Ok(Self {
            old_pattern: CompiledFilenamePattern::compile(&old_pattern)?,
            new_pattern: CompiledTemplate::compile(&new_pattern)?,
        })
    }

    pub fn render(
        &self,
        source: &str,
        context: &EvalContext<'_>,
    ) -> Result<FilenameConversion, String> {
        if source.contains(['\r', '\n', '\0']) {
            return Err("Source filename contains a control character".into());
        }
        let captures = self.old_pattern.parse(source)?;
        let mut fields = context.fields.clone();
        fields.extend(captures.clone());
        let context = EvalContext {
            fields: &fields,
            raw_fields: context.raw_fields.or(Some(context.fields)),
            extension: context.extension,
            max_output_bytes: context.max_output_bytes,
            locale: context.locale.clone(),
            number_format: context.number_format,
            timestamp_seconds: context.timestamp_seconds,
            timezone_offset_seconds: context.timezone_offset_seconds,
            ansi_codepage: context.ansi_codepage.clone(),
            columns: context.columns,
            environment: context.environment,
            app_directory: context.app_directory,
            app_data_directory: context.app_data_directory,
            working_directory: context.working_directory,
            folder_separator: context.folder_separator,
        };
        let mut target = self.new_pattern.evaluate_filename_with_context(&context)?;
        let leaf = basename(&target);
        if leaf.is_empty() || matches!(leaf, "." | "..") || target.contains(['\r', '\n', '\0']) {
            return Err(
                "New filename pattern produced an invalid empty or control-character filename"
                    .into(),
            );
        }
        let original_name = basename(source);
        if let Some(dot) = original_name.rfind('.').filter(|index| *index > 0) {
            // A pattern that already spells the extension does not get it twice.
            if !ends_with_extension(&target, &original_name[dot..]) {
                target.push_str(&original_name[dot..]);
            }
        }
        if target.len() > context.max_output_bytes {
            return Err("Filename converter output limit exceeded".into());
        }
        Ok(FilenameConversion {
            source: source.to_owned(),
            target,
            captures,
        })
    }
}

/// Whether `name` ends with `extension` (including its dot), ignoring case.
pub fn ends_with_extension(name: &str, extension: &str) -> bool {
    name.len() > extension.len()
        && name.is_char_boundary(name.len() - extension.len())
        && name[name.len() - extension.len()..].eq_ignore_ascii_case(extension)
}

/// Translate only unquoted numbered references in generating patterns. Matching
/// patterns treat quotes as literal delimiters, as the shared matcher does.
fn numbered_pattern(
    pattern: &str,
    known: Option<&BTreeSet<u8>>,
) -> Result<(String, BTreeSet<u8>), String> {
    if pattern.len() > MAX_MASK_BYTES {
        return Err("Filename converter pattern limit exceeded".into());
    }
    let mut result = String::new();
    let mut parts = BTreeSet::new();
    let bytes = pattern.as_bytes();
    let mut position = 0;
    let mut quoted = false;
    while position < bytes.len() {
        if bytes[position] == b'\'' && known.is_some() {
            result.push('\'');
            position += 1;
            if bytes.get(position) == Some(&b'\'') {
                result.push('\'');
                position += 1;
            } else {
                quoted = !quoted;
            }
            continue;
        }
        if bytes[position] != b'%' || quoted {
            let character = pattern[position..].chars().next().unwrap();
            result.push(character);
            position += character.len_utf8();
            continue;
        }
        if let Some(part @ b'1'..=b'9') = bytes.get(position + 1).copied() {
            if bytes.get(position + 2).is_some_and(u8::is_ascii_digit) {
                return Err("Numbered filename parts range from %1 through %9".into());
            }
            if let Some(known) = known {
                if !known.contains(&part) {
                    return Err(format!(
                        "New filename references undefined part %{}",
                        char::from(part)
                    ));
                }
            } else if !parts.insert(part) {
                return Err(format!(
                    "Repeated old filename part %{} is ambiguous",
                    char::from(part)
                ));
            }
            result.push('%');
            result.push(char::from(part));
            result.push('%');
            position += 2;
        } else if known.is_some() {
            let end = pattern[position + 1..]
                .find('%')
                .ok_or("Unterminated new filename placeholder")?
                + position
                + 1;
            if bytes.get(position + 1).is_some_and(u8::is_ascii_digit) {
                return Err("Numbered filename parts range from %1 through %9".into());
            }
            result.push_str(&pattern[position..=end]);
            position = end + 1;
        } else {
            return Err("Old filename pattern supports only numbered parts %1 through %9".into());
        }
    }
    Ok((result, parts))
}

pub struct TextTagConverter {
    expression: Regex,
    names: Vec<String>,
    key: Option<MatchKind>,
}

#[derive(Clone, Copy)]
enum MatchKind {
    Filename,
    Path,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextTagImport {
    pub selection_index: usize,
    pub source_line: usize,
    pub path: String,
    pub fields: Fields,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TextImportOptions {
    pub ansi_codepage: Option<String>,
    pub case_sensitive_paths: bool,
}

impl Default for TextImportOptions {
    fn default() -> Self {
        Self {
            ansi_codepage: None,
            case_sensitive_paths: true,
        }
    }
}

impl TextTagConverter {
    pub fn compile(mask: &str) -> Result<Self, String> {
        if mask.len() > MAX_MASK_BYTES || mask.contains(['\r', '\n', '\0']) {
            return Err("Invalid or oversized text import mask".into());
        }
        let mut expression = String::from("\\A");
        let mut names = Vec::new();
        let mut remainder = mask;
        let mut previous_capture = false;
        let mut key = None;
        let mut writable = false;
        while let Some(start) = remainder.find('%') {
            let literal = &remainder[..start];
            if previous_capture && literal.is_empty() {
                return Err("Adjacent text import placeholders are ambiguous".into());
            }
            expression.push_str(&regex::escape(literal));
            remainder = &remainder[start + 1..];
            let end = remainder
                .find('%')
                .ok_or("Unterminated text import placeholder")?;
            let name = remainder[..end].to_uppercase();
            if name.is_empty() || name.contains('=') {
                return Err("Invalid text import field name".into());
            }
            if name.starts_with('_') {
                let matching = match name.as_str() {
                    "_FILENAME_EXT" => MatchKind::Filename,
                    "_PATH" => MatchKind::Path,
                    _ => {
                        return Err(format!(
                            "Read-only field {name} is not a text import target"
                        ))
                    }
                };
                if key.replace(matching).is_some() {
                    return Err("Use exactly one filename or path matching placeholder".into());
                }
            } else if name != "DUMMY" {
                writable = true;
            }
            names.push(name);
            expression.push_str("(.*?)");
            remainder = &remainder[end + 1..];
            previous_capture = true;
        }
        if !writable {
            return Err("Text import mask must contain at least one writable field".into());
        }
        expression.push_str(&regex::escape(remainder));
        expression.push_str("\\z");
        let expression = RegexBuilder::new(&expression)
            .size_limit(8 * MAX_MASK_BYTES)
            .build()
            .map_err(|error| format!("Invalid text import matching pattern: {error}"))?;
        Ok(Self {
            expression,
            names,
            key,
        })
    }

    pub fn plan(
        &self,
        text: &str,
        selected_paths: &[String],
        options: &TextImportOptions,
    ) -> Result<Vec<TextTagImport>, String> {
        if text.len() > MAX_TEXT_BYTES || text.contains('\0') {
            return Err("Text import source is oversized or contains NUL characters".into());
        }
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let lines = text_lines(text);
        if lines.len() != selected_paths.len() {
            return Err(format!(
                "Text import has {} lines for {} selected files; no changes planned",
                lines.len(),
                selected_paths.len()
            ));
        }
        let mut selected_keys = BTreeMap::new();
        let mut seen_paths = HashSet::new();
        for (index, path) in selected_paths.iter().enumerate() {
            let normalized = match_key(path, MatchKind::Path, options)?;
            if !seen_paths.insert(normalized) {
                return Err(format!(
                    "File occurs more than once in text import selection: {path}"
                ));
            }
            if let Some(kind) = self.key {
                let name = match_key(path, kind, options)?;
                if selected_keys.insert(name, index).is_some() {
                    return Err(format!(
                        "Ambiguous text import filename: {}",
                        basename(path)
                    ));
                }
            }
        }
        let mut plans = Vec::with_capacity(lines.len());
        let mut used = HashSet::new();
        for (index, line) in lines.into_iter().enumerate() {
            let number = index + 1;
            if line.len() > MAX_MASK_BYTES {
                return Err(format!("Text import line {number} exceeds the input limit"));
            }
            let captures = self.expression.captures(line).ok_or_else(|| {
                format!("Text import line {number} does not completely match the mask")
            })?;
            let mut fields = Fields::new();
            let mut key_value = None;
            for (position, name) in self.names.iter().enumerate() {
                let value = captures.get(position + 1).unwrap().as_str();
                if name == "_FILENAME_EXT" || name == "_PATH" {
                    key_value = Some(value);
                } else if name != "DUMMY" {
                    fields
                        .entry(name.clone())
                        .or_default()
                        .push(value.to_owned());
                }
            }
            let selection_index = if let Some(kind) = self.key {
                let value = key_value.unwrap();
                if matches!(kind, MatchKind::Path) && !absolute_path(value) {
                    return Err(format!(
                        "Text import line {number} requires an absolute _PATH"
                    ));
                }
                if matches!(kind, MatchKind::Filename) && value.contains(['/', '\\']) {
                    return Err(format!(
                        "Text import line {number} requires a filename without folders"
                    ));
                }
                let key = match_key(value, kind, options)
                    .map_err(|error| format!("Text import line {number}: {error}"))?;
                *selected_keys.get(&key).ok_or_else(|| {
                    format!("Text import line {number} does not match a selected file: {value}")
                })?
            } else {
                index
            };
            if !used.insert(selection_index) {
                return Err(format!(
                    "Text import line {number} targets a file already matched by another line"
                ));
            }
            plans.push(TextTagImport {
                selection_index,
                source_line: number,
                path: selected_paths[selection_index].clone(),
                fields,
            });
        }
        Ok(plans)
    }

    pub fn plan_bytes(
        &self,
        bytes: &[u8],
        selected_paths: &[String],
        options: &TextImportOptions,
    ) -> Result<Vec<TextTagImport>, String> {
        if bytes.len() > MAX_TEXT_BYTES {
            return Err("Text import source exceeds the input limit".into());
        }
        let text = decode_export_template(bytes, options.ansi_codepage.as_deref())
            .map_err(|error| format!("Cannot decode text import source: {error}"))?;
        self.plan(&text, selected_paths, options)
    }
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn text_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut position = 0;
    while position < bytes.len() {
        if matches!(bytes[position], b'\r' | b'\n') {
            lines.push(&text[start..position]);
            if bytes[position] == b'\r' && bytes.get(position + 1) == Some(&b'\n') {
                position += 1;
            }
            position += 1;
            start = position;
        } else {
            position += 1;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

fn absolute_path(path: &str) -> bool {
    path.starts_with(['/', '\\'])
        || (path.as_bytes().get(1) == Some(&b':')
            && path
                .as_bytes()
                .get(2)
                .is_some_and(|byte| matches!(byte, b'/' | b'\\')))
}

fn match_key(path: &str, kind: MatchKind, options: &TextImportOptions) -> Result<String, String> {
    if path.is_empty() || path.contains(['\r', '\n', '\0']) {
        return Err("Invalid empty or control-character text import path".into());
    }
    let value = match kind {
        MatchKind::Filename => basename(path).to_owned(),
        MatchKind::Path => path.replace('\\', "/"),
    };
    Ok(if options.case_sensitive_paths {
        value
    } else {
        value.to_lowercase()
    })
}
