//! Independently implemented format strings and actions.
//! Semantics reference: https://docs.mp3tag.de/scripting/, /format/, /regex/.
//! User regular expressions use the shared open-source Boost Perl engine.
use crate::batch_errors::ErrorCause;
use crate::tags::Fields;
use chrono::{Datelike, FixedOffset, Local, TimeZone, Timelike, Utc};
use num_format::ToFormattedString;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const MAX_OUTPUT: usize = 1_048_576;
const MAX_DEPTH: usize = 128;

#[derive(Clone, Debug, Default)]
pub struct ScriptValue {
    pub text: String,
    pub present: bool,
    pub truth: bool,
}

impl ScriptValue {
    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScriptFunctionSpec {
    pub name: String,
    pub min_args: usize,
    pub max_args: usize,
}

/// Optional host functions are accepted only by compile_with_functions. The
/// normal scripting grammar and its 64 builtins remain strict and unchanged.
pub trait ScriptExtension {
    fn call(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String>;
    fn field(&self, _name: &str) -> Result<Option<Vec<String>>, String> {
        Ok(None)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptColumn {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptNumberFormat {
    pub separator: String,
    /// Group widths starting at the right. Repeat the final width.
    pub grouping: Vec<usize>,
    pub minus_sign: String,
}

/// Application and system values required by context-sensitive scripting.
/// Supply columns in visible table order with their rendered values.
#[derive(Clone)]
pub struct EvalContext<'a> {
    pub fields: &'a Fields,
    pub raw_fields: Option<&'a Fields>,
    pub extension: Option<&'a dyn ScriptExtension>,
    pub max_output_bytes: usize,
    pub locale: String,
    pub number_format: Option<&'a ScriptNumberFormat>,
    pub timestamp_seconds: Option<i64>,
    pub timezone_offset_seconds: Option<i32>,
    pub ansi_codepage: Option<String>,
    pub columns: Option<&'a [ScriptColumn]>,
    pub environment: Option<&'a BTreeMap<String, String>>,
    pub app_directory: Option<&'a str>,
    pub app_data_directory: Option<&'a str>,
    /// Folder the file list was last changed to (`%_workingdir%`,
    /// `%_workingpath%`). Absent when the list was not loaded from a folder.
    pub working_directory: Option<&'a str>,
    /// Options → Files "Replace folder separator in tag values with". `None`
    /// uses [`DEFAULT_FOLDER_SEPARATOR`]; an empty string removes separators.
    pub folder_separator: Option<&'a str>,
}

impl<'a> EvalContext<'a> {
    pub fn new(fields: &'a Fields) -> Self {
        static LOCALE: OnceLock<String> = OnceLock::new();
        Self {
            fields,
            raw_fields: None,
            extension: None,
            max_output_bytes: MAX_OUTPUT,
            locale: LOCALE
                .get_or_init(|| sys_locale::get_locale().unwrap_or_else(|| "en-US".into()))
                .clone(),
            number_format: None,
            timestamp_seconds: None,
            timezone_offset_seconds: None,
            // Windows must supply its actual GetACP result, not a locale guess.
            ansi_codepage: if cfg!(windows) {
                None
            } else {
                Some("utf-8".into())
            },
            columns: None,
            environment: None,
            app_directory: None,
            app_data_directory: None,
            working_directory: None,
            folder_separator: None,
        }
    }

    /// Resolve an unspecified clock once for the whole expression. Nested
    /// templates and date functions receive the same instant.
    pub(crate) fn snapshot_clock(&self) -> Cow<'_, Self> {
        if self.timestamp_seconds.is_some() {
            Cow::Borrowed(self)
        } else {
            let mut context = self.clone();
            context.timestamp_seconds = Some(Utc::now().timestamp());
            Cow::Owned(context)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Format {
        field: String,
        template: String,
    },
    Replace {
        field: String,
        from: String,
        to: String,
        case_sensitive: bool,
    },
    RegexReplace {
        field: String,
        pattern: String,
        replacement: String,
    },
    Case {
        field: String,
        mode: String,
    },
    Remove {
        fields: Vec<String>,
    },
}

#[derive(Debug)]
enum Node {
    Literal(String),
    Field(String),
    Optional(Vec<Node>),
    Function(String, Vec<Vec<Node>>),
    Extension(String, Vec<Vec<Node>>),
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    functions: BTreeMap<String, (usize, usize)>,
    /// Whether parentheses in argument text nest (see [`Parser::sequence`]).
    nesting: bool,
}

impl Parser {
    fn sequence(&mut self, stops: &[char], depth: usize) -> Result<Vec<Node>, String> {
        if depth > MAX_DEPTH {
            return Err("Format string nesting limit exceeded".into());
        }
        let mut nodes = Vec::new();
        let mut literal = String::new();
        let mut odd_backslashes = false;
        // Inside function arguments, parentheses in literal text nest as in
        // Mp3tag, so `$regexp(%artist%,^(va|v\.a\.)$,Various)` has three
        // arguments; commas and `)` within them belong to the text. An
        // escaped `\(` or `\)` is text: it neither nests nor ends the call.
        let arguments = self.nesting && stops.contains(&')');
        let mut parentheses = 0usize;
        while let Some(&ch) = self.chars.get(self.pos) {
            let escaped = arguments && odd_backslashes;
            if parentheses == 0 && stops.contains(&ch) && !(escaped && ch == ')') {
                break;
            }
            if !matches!(ch, '\'' | '%' | '[' | ']' | '$') {
                if arguments && !escaped && ch == '(' {
                    parentheses += 1;
                } else if arguments && !escaped && ch == ')' && parentheses > 0 {
                    parentheses -= 1;
                }
                literal.push(ch);
                odd_backslashes = ch == '\\' && !odd_backslashes;
                self.pos += 1;
                continue;
            }
            if !literal.is_empty() {
                nodes.push(Node::Literal(std::mem::take(&mut literal)));
            }
            // Escape parity belongs to this literal node, not a quoted value,
            // field, optional section, nested call or literal dollar that follows.
            odd_backslashes = false;
            self.pos += 1;
            match ch {
                '\'' => {
                    if self.chars.get(self.pos) == Some(&'\'') {
                        self.pos += 1;
                        nodes.push(Node::Literal("'".into()));
                        continue;
                    }
                    let mut raw = String::new();
                    loop {
                        let current = *self
                            .chars
                            .get(self.pos)
                            .ok_or("Unterminated quoted literal")?;
                        self.pos += 1;
                        if current == '\'' {
                            if self.chars.get(self.pos) == Some(&'\'') {
                                raw.push('\'');
                                self.pos += 1;
                            } else {
                                break;
                            }
                        } else {
                            raw.push(current);
                        }
                    }
                    nodes.push(Node::Literal(raw));
                }
                '%' => {
                    let start = self.pos;
                    while self.chars.get(self.pos).is_some_and(|c| *c != '%') {
                        self.pos += 1;
                    }
                    if self.pos == self.chars.len() {
                        return Err("Unterminated field placeholder".into());
                    }
                    let name: String = self.chars[start..self.pos].iter().collect();
                    if name.is_empty() {
                        return Err("Empty field placeholder".into());
                    }
                    self.pos += 1;
                    nodes.push(Node::Field(name));
                }
                '[' => {
                    let nested = self.sequence(&[']'], depth + 1)?;
                    if self.chars.get(self.pos) != Some(&']') {
                        return Err("Unterminated optional section".into());
                    }
                    self.pos += 1;
                    nodes.push(Node::Optional(nested));
                }
                ']' => return Err("Unexpected closing optional bracket".into()),
                '$' => {
                    let start = self.pos;
                    while self
                        .chars
                        .get(self.pos)
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                    {
                        self.pos += 1;
                    }
                    let name: String = self.chars[start..self.pos]
                        .iter()
                        .collect::<String>()
                        .to_ascii_lowercase();
                    // A `$` that does not start a function name is text: the end
                    // anchor in `^(va)$` or a reference such as `$1` in an
                    // unquoted regular expression replacement.
                    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
                        literal.push('$');
                        literal.extend(&self.chars[start..self.pos]);
                        continue;
                    }
                    if self.chars.get(self.pos) != Some(&'(') {
                        return Err(format!("Expected function call after ${name}"));
                    }
                    if !SUPPORTED_FUNCTIONS.contains(&name.as_str())
                        && !self.functions.contains_key(&name)
                    {
                        return Err(format!("Unsupported scripting function: ${name}"));
                    }
                    self.pos += 1;
                    // Text that only parses without nesting (an unbalanced
                    // `(` in an argument) keeps its earlier meaning.
                    let open = self.pos;
                    let node = match self.call(name.clone(), depth) {
                        Err(_) if self.nesting => {
                            self.pos = open;
                            self.nesting = false;
                            let flat = self.call(name, depth);
                            self.nesting = true;
                            flat?
                        }
                        result => result?,
                    };
                    nodes.push(node);
                }
                _ => unreachable!(),
            }
        }
        if !literal.is_empty() {
            nodes.push(Node::Literal(literal));
        }
        Ok(nodes)
    }

    /// The arguments and closing parenthesis of `$name(`, as a node.
    fn call(&mut self, name: String, depth: usize) -> Result<Node, String> {
        let mut args = Vec::new();
        if self.chars.get(self.pos) == Some(&')') {
            self.pos += 1;
        } else {
            loop {
                args.push(self.sequence(&[',', ')'], depth + 1)?);
                let maximum = self
                    .functions
                    .get(&name)
                    .map(|(_, maximum)| *maximum)
                    .unwrap_or_else(|| {
                        if ["add", "sub", "mul", "div", "mod"].contains(&name.as_str()) {
                            64
                        } else {
                            63
                        }
                    });
                if args.len() > maximum {
                    return Err(format!("${name} supports at most {maximum} arguments"));
                }
                match self.chars.get(self.pos) {
                    Some(',') => self.pos += 1,
                    Some(')') => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(format!("Unterminated function ${name}")),
                }
            }
        }
        if let Some((minimum, maximum)) = self.functions.get(&name) {
            if !(*minimum..=*maximum).contains(&args.len()) {
                return Err(format!("${name} expects {minimum}..={maximum} arguments"));
            }
            return Ok(Node::Extension(name, args));
        }
        validate_arity(&name, args.len())?;
        Ok(Node::Function(name, args))
    }
}

/// Documented functions implemented here. Others produce an explicit error.
pub const SUPPORTED_FUNCTIONS: &[&str] = &[
    "lower",
    "upper",
    "len",
    "left",
    "right",
    "mid",
    "cutleft",
    "cutright",
    "trim",
    "trimleft",
    "trimright",
    "caps",
    "caps2",
    "caps3",
    "ansi",
    "fmtdate",
    "fmtduration",
    "fmtnum",
    "folderdepth",
    "num",
    "replace",
    "regexp",
    "strstr",
    "strrstr",
    "strchr",
    "strrchr",
    "strcmp",
    "stricmp",
    "reverse",
    "repeat",
    "char",
    "ord",
    "sort",
    "dedup",
    "validate",
    "distance",
    "if",
    "if2",
    "ifgreater",
    "iflonger",
    "eql",
    "neql",
    "grtr",
    "geql",
    "less",
    "leql",
    "and",
    "or",
    "not",
    "odd",
    "isdigit",
    "add",
    "sub",
    "mul",
    "div",
    "mod",
    "rand",
    "uuid",
    "getenv",
    "columnnames",
    "columnvalues",
    "meta",
    "meta_sep",
    "list",
];

#[derive(Default)]
struct Value {
    text: String,
    present: bool,
    truth: bool,
    filename: Option<String>,
}

fn field_values<'a>(fields: &'a Fields, name: &str) -> Option<&'a Vec<String>> {
    fields.get(&name.to_ascii_uppercase()).or_else(|| {
        fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, values)| values)
    })
}

pub(crate) fn context_values<'a>(
    context: &'a EvalContext<'_>,
    name: &str,
) -> Result<Option<Cow<'a, [String]>>, String> {
    context_values_diagnostic(context, name).map_err(|error| error.to_string())
}

fn context_values_diagnostic<'a>(
    context: &'a EvalContext<'_>,
    name: &str,
) -> Result<Option<Cow<'a, [String]>>, ErrorCause> {
    if let Some(extension) = context.extension {
        if let Some(values) = extension.field(name).map_err(ErrorCause::external)? {
            return Ok(Some(Cow::Owned(values)));
        }
    }
    if name.eq_ignore_ascii_case("_DATE") || name.eq_ignore_ascii_case("_DATETIME") {
        let date = local_datetime(
            context
                .timestamp_seconds
                .unwrap_or_else(|| Utc::now().timestamp()),
            context,
        )
        .map_err(ErrorCause::owned_text)?;
        let pattern = if name.eq_ignore_ascii_case("_DATE") {
            "%x"
        } else {
            "%x %X"
        };
        return Ok(Some(Cow::Owned(vec![date
            .format_localized(
                pattern,
                date_locale(&context.locale).map_err(ErrorCause::owned_text)?,
            )
            .to_string()])));
    }
    if name.eq_ignore_ascii_case("_WORKINGDIR") || name.eq_ignore_ascii_case("_WORKINGPATH") {
        if let Some(path) = context.working_directory.filter(|path| !path.is_empty()) {
            return Ok(Some(Cow::Owned(vec![working_directory_value(path, name)])));
        }
    }
    Ok(field_values(context.fields, name).map(|values| Cow::Borrowed(values.as_slice())))
}

/// `%_workingdir%` is the folder name; `%_workingpath%` the full path with a
/// trailing separator, like `%_folderpath%`. Both separators are accepted.
fn working_directory_value(path: &str, name: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let separator = if path.contains('\\') && !path.contains('/') {
        '\\'
    } else {
        std::path::MAIN_SEPARATOR
    };
    if name.eq_ignore_ascii_case("_WORKINGDIR") {
        return trimmed
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_string();
    }
    if trimmed.is_empty() {
        // The file-system root: keep it as given.
        return path.to_string();
    }
    format!("{trimmed}{separator}")
}

fn bounded(text: String) -> Result<String, String> {
    bounded_with_limit(text, MAX_OUTPUT)
}

fn bounded_with_limit(text: String, limit: usize) -> Result<String, String> {
    if text.len() > limit {
        Err("Format string output limit exceeded".into())
    } else {
        Ok(text)
    }
}

/// What replaces `\` and `/` in tag values used as filenames by default.
pub const DEFAULT_FOLDER_SEPARATOR: &str = "_";

/// Characters a tag value cannot carry into a filename besides the folder
/// separators: `$validate`'s set plus control characters.
fn invalid_filename_character(character: char) -> bool {
    (character as u32) < 32 || ":*?\"<>|".contains(character)
}

/// Make a tag value usable as part of a filename: folder separators become
/// `separator` (which may be empty) and other invalid characters `_`.
pub fn filename_value(text: &str, separator: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '/' | '\\') {
            // The replacement itself must never introduce a separator.
            for replacement in separator.chars() {
                output.push(
                    if matches!(replacement, '/' | '\\') || invalid_filename_character(replacement)
                    {
                        '_'
                    } else {
                        replacement
                    },
                );
            }
        } else if invalid_filename_character(character) {
            output.push('_');
        } else {
            output.push(character);
        }
    }
    output
}

/// Check a "Replace folder separator" option value.
pub fn validate_folder_separator(separator: &str) -> Result<(), String> {
    if separator.chars().count() > 16 {
        return Err("The folder separator replacement is limited to 16 characters".into());
    }
    if separator
        .chars()
        .any(|c| matches!(c, '/' | '\\') || invalid_filename_character(c))
    {
        return Err(
            "The folder separator replacement cannot contain \\ / : * ? \" < > | or control characters"
                .into(),
        );
    }
    Ok(())
}

/// Placeholders whose value is a path, used unchanged in filename mode.
const PATH_FIELDS: [&str; 3] = ["_PATH", "_FOLDERPATH", "_WORKINGPATH"];

fn filename_component(text: &str, context: &EvalContext<'_>) -> String {
    filename_value(
        text,
        context.folder_separator.unwrap_or(DEFAULT_FOLDER_SEPARATOR),
    )
}

fn run(
    nodes: &[Node],
    context: &EvalContext<'_>,
    filename_mode: bool,
    native_path: bool,
) -> Result<Value, String> {
    run_diagnostic(nodes, context, filename_mode, native_path).map_err(|error| error.to_string())
}

fn run_diagnostic(
    nodes: &[Node],
    context: &EvalContext<'_>,
    filename_mode: bool,
    native_path: bool,
) -> Result<Value, ErrorCause> {
    let mut output = Value {
        filename: filename_mode.then(String::new),
        ..Value::default()
    };
    for node in nodes {
        let value = match node {
            Node::Literal(text) => Value {
                text: text.clone(),
                present: false,
                truth: false,
                filename: filename_mode.then(|| {
                    if native_path {
                        if cfg!(windows) {
                            text.replace('/', "\\")
                        } else {
                            text.replace('\\', "/")
                        }
                    } else {
                        text.clone()
                    }
                }),
            },
            Node::Field(name) => {
                let text = context_values_diagnostic(context, name)?
                    .as_deref()
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or_default();
                let present = !text.is_empty();
                let filename = filename_mode.then(|| {
                    // A path placeholder already is a path: keep its folders.
                    if PATH_FIELDS
                        .iter()
                        .any(|path| name.eq_ignore_ascii_case(path))
                    {
                        text.clone()
                    } else {
                        filename_component(&text, context)
                    }
                });
                Value {
                    text,
                    present,
                    truth: present,
                    filename,
                }
            }
            Node::Optional(children) => {
                let value = run_diagnostic(children, context, filename_mode, native_path)?;
                if value.present {
                    value
                } else {
                    Value::default()
                }
            }
            Node::Function(name, arguments) | Node::Extension(name, arguments) => {
                let values: Vec<Value> = arguments
                    .iter()
                    .map(|a| run_diagnostic(a, context, filename_mode, native_path))
                    .collect::<Result<_, _>>()?;
                if matches!(node, Node::Extension(_, _)) {
                    let args: Vec<ScriptValue> = values
                        .into_iter()
                        .map(|value| ScriptValue {
                            text: value.text,
                            present: value.present,
                            truth: value.truth,
                        })
                        .collect();
                    let value = context
                        .extension
                        .ok_or_else(|| {
                            ErrorCause::message(
                                "${name} requires its opt-in host extension",
                                [("name", name.clone())],
                            )
                        })?
                        .call(name, &args)
                        .map_err(ErrorCause::external)?;
                    let filename = filename_mode.then(|| filename_component(&value.text, context));
                    Value {
                        text: bounded_with_limit(value.text, context.max_output_bytes)
                            .map_err(ErrorCause::owned_text)?,
                        present: value.present,
                        truth: value.truth,
                        filename,
                    }
                } else {
                    let mut present = values.iter().any(|v| v.present);
                    let args: Vec<&str> = values.iter().map(|v| v.text.as_str()).collect();
                    let truths: Vec<bool> = values.iter().map(|v| v.truth).collect();
                    let text = function_diagnostic(name, &args, &truths, context)?;
                    let selected = match name.as_str() {
                        "if" => Some(if truths[0] { 1 } else { 2 }),
                        "if2" => Some(if truths[0] { 0 } else { 1 }),
                        "ifgreater" => Some(if number(args[0]) > number(args[1]) {
                            2
                        } else {
                            3
                        }),
                        "iflonger" => Some(if args[0].chars().count() > count(args[1]) {
                            2
                        } else {
                            3
                        }),
                        _ => None,
                    };
                    let filename = filename_mode.then(|| {
                        selected
                            .and_then(|index| values[index].filename.clone())
                            .unwrap_or_else(|| filename_component(&text, context))
                    });
                    let truth = match name.as_str() {
                        "eql" | "neql" | "grtr" | "geql" | "less" | "leql" | "and" | "or"
                        | "not" | "odd" | "isdigit" | "strcmp" | "stricmp" => text == "1",
                        "if" => truths[if truths[0] { 1 } else { 2 }],
                        "if2" => truths[if truths[0] { 0 } else { 1 }],
                        "ifgreater" => {
                            truths[if number(args[0]) > number(args[1]) {
                                2
                            } else {
                                3
                            }]
                        }
                        "iflonger" => {
                            truths[if args[0].chars().count() > count(args[1]) {
                                2
                            } else {
                                3
                            }]
                        }
                        "meta" | "meta_sep" => {
                            present |= !text.is_empty();
                            !text.is_empty()
                        }
                        "getenv" | "columnnames" | "columnvalues" => !text.is_empty(),
                        _ => truths.iter().any(|v| *v),
                    };
                    Value {
                        text,
                        present,
                        truth,
                        filename,
                    }
                }
            }
        };
        if output.text.len().saturating_add(value.text.len()) > context.max_output_bytes {
            return Err(ErrorCause::owned("Format string output limit exceeded"));
        }
        output.text.push_str(&value.text);
        if let Some(rendered) = &mut output.filename {
            let component = value.filename.as_deref().unwrap_or_default();
            if rendered.len().saturating_add(component.len()) > context.max_output_bytes {
                return Err(ErrorCause::owned("Filename output limit exceeded"));
            }
            rendered.push_str(component);
        }
        output.present |= value.present;
        output.truth |= value.truth;
    }
    Ok(output)
}

#[derive(Debug)]
pub struct CompiledTemplate {
    nodes: Vec<Node>,
}

impl CompiledTemplate {
    pub fn evaluate_with_context_diagnostic(
        &self,
        context: &EvalContext<'_>,
    ) -> Result<String, ErrorCause> {
        run_diagnostic(&self.nodes, &context.snapshot_clock(), false, false).map(|value| value.text)
    }
    pub fn evaluate_filename_with_context_diagnostic(
        &self,
        context: &EvalContext<'_>,
    ) -> Result<String, ErrorCause> {
        run_diagnostic(&self.nodes, &context.snapshot_clock(), true, false)
            .map(|value| value.filename.unwrap_or_default())
    }
    pub fn evaluate_filename_path_with_context_diagnostic(
        &self,
        context: &EvalContext<'_>,
    ) -> Result<String, ErrorCause> {
        run_diagnostic(&self.nodes, &context.snapshot_clock(), true, true)
            .map(|value| value.filename.unwrap_or_default())
    }
    pub fn compile(template: &str) -> Result<Self, String> {
        Self::compile_with_functions(template, &[])
    }

    pub fn compile_with_functions(
        template: &str,
        functions: &[ScriptFunctionSpec],
    ) -> Result<Self, String> {
        if template.len() > MAX_OUTPUT {
            return Err("Format string input limit exceeded".into());
        }
        let mut allowed = BTreeMap::new();
        for function in functions {
            let name = function.name.to_ascii_lowercase();
            if name.is_empty()
                || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || SUPPORTED_FUNCTIONS.contains(&name.as_str())
                || function.min_args > function.max_args
                || function.max_args > 64
            {
                return Err(format!(
                    "Invalid extra scripting function specification: {}",
                    function.name
                ));
            }
            if allowed
                .insert(name, (function.min_args, function.max_args))
                .is_some()
            {
                return Err("Duplicate extra scripting function".into());
            }
        }
        let mut parser = Parser {
            chars: template.chars().collect(),
            pos: 0,
            functions: allowed,
            nesting: true,
        };
        Ok(Self {
            nodes: parser.sequence(&[], 0)?,
        })
    }

    pub fn evaluate(&self, fields: &Fields) -> Result<String, String> {
        self.evaluate_with_context(&EvalContext::new(fields))
    }

    pub fn evaluate_with_context(&self, context: &EvalContext<'_>) -> Result<String, String> {
        self.evaluate_value_with_context(context)
            .map(|value| value.text)
    }

    pub fn evaluate_value_with_context(
        &self,
        context: &EvalContext<'_>,
    ) -> Result<ScriptValue, String> {
        run(&self.nodes, &context.snapshot_clock(), false, false).map(|value| ScriptValue {
            text: value.text,
            present: value.present,
            truth: value.truth,
        })
    }

    /// Preserve separators from literal AST nodes, including conditional paths.
    /// Treat fields and other function results as filename components after
    /// evaluating against the original text. Filesystem/extension policy belongs
    /// to the caller; this does not create directories or rename files.
    pub fn evaluate_filename_with_context(
        &self,
        context: &EvalContext<'_>,
    ) -> Result<String, String> {
        run(&self.nodes, &context.snapshot_clock(), true, false)
            .map(|value| value.filename.unwrap_or_default())
    }

    /// Render a native path, converting separators only in template literals.
    /// Path placeholders retain native component characters unchanged.
    pub fn evaluate_filename_path_with_context(
        &self,
        context: &EvalContext<'_>,
    ) -> Result<String, String> {
        run(&self.nodes, &context.snapshot_clock(), true, true)
            .map(|value| value.filename.unwrap_or_default())
    }
}

pub fn evaluate(template: &str, fields: &Fields) -> Result<String, String> {
    CompiledTemplate::compile(template)?.evaluate(fields)
}

pub fn evaluate_with_context(template: &str, context: &EvalContext<'_>) -> Result<String, String> {
    CompiledTemplate::compile(template)?.evaluate_with_context(context)
}

pub fn evaluate_filename_with_context(
    template: &str,
    context: &EvalContext<'_>,
) -> Result<String, String> {
    CompiledTemplate::compile(template)?.evaluate_filename_with_context(context)
}

fn arity(name: &str, args: &[&str], min: usize, max: usize) -> Result<(), String> {
    if (min..=max).contains(&args.len()) {
        Ok(())
    } else {
        Err(format!(
            "${name} expects {min}..={max} arguments, received {}",
            args.len()
        ))
    }
}

fn validate_arity(name: &str, count: usize) -> Result<(), String> {
    let (minimum, maximum) = match name {
        "rand" | "uuid" => (0, 0),
        "lower" | "upper" | "len" | "reverse" | "ord" | "char" | "not" | "odd" | "isdigit"
        | "ansi" | "fmtnum" | "folderdepth" | "getenv" => (1, 1),
        "trim" | "trimleft" | "trimright" | "caps" | "caps2" | "caps3" | "meta" | "fmtdate" => {
            (1, 2)
        }
        "fmtduration" => (1, 4),
        "mid" | "dedup" => (2, 3),
        "regexp" | "list" => (3, 4),
        "if" => (3, 3),
        "ifgreater" | "iflonger" => (4, 4),
        "replace" => (3, 63),
        "and" | "or" => (2, 63),
        "add" | "sub" | "mul" | "div" | "mod" => (2, 64),
        _ => (2, 2),
    };
    if !(minimum..=maximum).contains(&count) {
        return Err(format!(
            "${name} expects {minimum}..={maximum} arguments, received {count}"
        ));
    }
    if name == "replace" && count % 2 != 1 {
        return Err("$replace requires complete search/replacement pairs".into());
    }
    Ok(())
}

// $num explicitly parses a numeric prefix (e.g. a track value of 03/12).
fn number(value: &str) -> i64 {
    let value = value.trim_start();
    let prefix: String = value
        .chars()
        .enumerate()
        .take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && matches!(c, '+' | '-')))
        .map(|(_, c)| c)
        .collect();
    prefix.parse().unwrap_or(0)
}
fn count(value: &str) -> usize {
    usize::try_from(number(value)).unwrap_or(0)
}
fn boolean(value: bool) -> String {
    if value {
        "1".into()
    } else {
        "0".into()
    }
}
fn slice(text: &str, start: usize, length: usize) -> String {
    text.chars().skip(start).take(length).collect()
}

fn title_case(text: &str, separators: Option<&str>, preserve: bool) -> String {
    let mut start = true;
    let mut output = String::new();
    for c in text.chars() {
        if start {
            output.extend(c.to_uppercase());
        } else if preserve {
            output.push(c);
        } else {
            output.extend(c.to_lowercase());
        }
        start = c.is_whitespace() || separators.is_some_and(|s| s.contains(c));
    }
    output
}

pub(crate) fn regex(pattern: &str, ignore_case: bool) -> Result<regex_compat::Regex, String> {
    regex_compat::Regex::new(pattern, !ignore_case)
}

fn filename_regex(pattern: &str) -> Result<Regex, String> {
    RegexBuilder::new(pattern)
        .size_limit(8 * MAX_OUTPUT)
        .build()
        .map_err(|error| format!("Invalid filename matching pattern: {error}"))
}

fn regex_replace(
    input: &str,
    expression: &regex_compat::Regex,
    replacement: &str,
) -> Result<String, String> {
    bounded(expression.replace(input, replacement)?)
}

fn literal_regex_replacement(text: &str) -> String {
    text.replace('\\', "\\\\").replace('$', "$$")
}

fn number_locale(name: &str) -> Result<num_format::Locale, String> {
    let name = name.split(['.', '@']).next().unwrap_or(name);
    num_format::Locale::from_name(name)
        .or_else(|_| num_format::Locale::from_name(name.split(['-', '_']).next().unwrap_or(name)))
        .map_err(|_| format!("Unsupported numeric locale: {name}"))
}

fn format_number(value: i64, context: &EvalContext<'_>) -> Result<String, String> {
    if let Some(format) = context.number_format {
        if format.grouping.contains(&0) {
            return Err("Numeric grouping widths must be positive".into());
        }
        let digits = value.unsigned_abs().to_string();
        let mut groups = Vec::new();
        let mut end = digits.len();
        let mut index = 0;
        while end > 0 {
            let width = format
                .grouping
                .get(index)
                .or_else(|| format.grouping.last())
                .copied()
                .unwrap_or(end);
            let start = end.saturating_sub(width);
            groups.push(&digits[start..end]);
            end = start;
            index += 1;
        }
        groups.reverse();
        let prefix = if value < 0 {
            format.minus_sign.as_str()
        } else {
            ""
        };
        return bounded_with_limit(
            format!("{prefix}{}", groups.join(&format.separator)),
            context.max_output_bytes,
        );
    }
    if ["C", "POSIX"].contains(&context.locale.as_str()) {
        return Ok(value.to_string());
    }
    Ok(value.to_formatted_string(&number_locale(&context.locale)?))
}

fn date_locale(name: &str) -> Result<chrono::Locale, String> {
    let name = name
        .split(['.', '@'])
        .next()
        .unwrap_or(name)
        .replace('-', "_");
    if let Ok(locale) = name.parse() {
        return Ok(locale);
    }
    let language = name.split('_').next().unwrap_or(&name);
    // CLDR/POSIX locale tags often omit a territory. Select a locale in the same
    // language, retaining an explicit error when no localized data is available.
    let default = match language {
        "en" | "C" | "POSIX" => "en_US",
        "nl" => "nl_NL",
        "de" => "de_DE",
        "fr" => "fr_FR",
        "es" => "es_ES",
        "it" => "it_IT",
        "pt" => "pt_PT",
        "ru" => "ru_RU",
        "uk" => "uk_UA",
        "ja" => "ja_JP",
        "ko" => "ko_KR",
        "zh" => "zh_CN",
        "pl" => "pl_PL",
        "cs" => "cs_CZ",
        "sv" => "sv_SE",
        "da" => "da_DK",
        "nb" | "no" => "nb_NO",
        "fi" => "fi_FI",
        "tr" => "tr_TR",
        "el" => "el_GR",
        "he" => "he_IL",
        "ar" => "ar_SA",
        "hu" => "hu_HU",
        "ro" => "ro_RO",
        "bg" => "bg_BG",
        "sk" => "sk_SK",
        "sl" => "sl_SI",
        "hr" => "hr_HR",
        "id" => "id_ID",
        "th" => "th_TH",
        "vi" => "vi_VN",
        "hi" => "hi_IN",
        _ => return Err(format!("Unsupported date locale: {name}")),
    };
    default
        .parse()
        .map_err(|_| format!("Unsupported date locale: {name}"))
}

fn format_date(
    pattern: &str,
    timestamp: Option<&str>,
    context: &EvalContext<'_>,
) -> Result<String, String> {
    let seconds = match timestamp {
        Some("") => 0,
        Some(value) => value
            .parse::<i64>()
            .map_err(|_| "$fmtDate expects an integer Unix timestamp")?,
        None => context
            .timestamp_seconds
            .unwrap_or_else(|| Utc::now().timestamp()),
    };
    let date = local_datetime(seconds, context)?;
    let mut remaining = pattern;
    let mut output = String::new();
    const TOKENS: &[&str] = &[
        "yyyy", "MMMM", "dddd", "EEEE", "MMM", "ddd", "yy", "MM", "dd", "HH", "hh", "mm", "ss",
        "EE", "M", "d", "H", "h", "m", "s",
    ];
    while !remaining.is_empty() {
        if let Some(raw) = remaining.strip_prefix('\'') {
            if let Some(raw) = raw.strip_prefix('\'') {
                output.push('\'');
                remaining = raw;
                continue;
            }
            let end = raw.find('\'').ok_or("Unterminated date-format literal")?;
            output.push_str(&raw[..end]);
            remaining = &raw[end + 1..];
            continue;
        }
        let Some(token) = TOKENS.iter().find(|token| remaining.starts_with(**token)) else {
            let character = remaining.chars().next().unwrap();
            output.push(character);
            remaining = &remaining[character.len_utf8()..];
            continue;
        };
        let value = match *token {
            "yyyy" => format!("{:04}", date.year()),
            "yy" => format!("{:02}", date.year().rem_euclid(100)),
            "MMMM" => date
                .format_localized("%B", date_locale(&context.locale)?)
                .to_string(),
            "MMM" => date
                .format_localized("%b", date_locale(&context.locale)?)
                .to_string(),
            "dddd" | "EEEE" => date
                .format_localized("%A", date_locale(&context.locale)?)
                .to_string(),
            "ddd" | "EE" => date
                .format_localized("%a", date_locale(&context.locale)?)
                .to_string(),
            "MM" => format!("{:02}", date.month()),
            "M" => date.month().to_string(),
            "dd" => format!("{:02}", date.day()),
            "d" => date.day().to_string(),
            "HH" => format!("{:02}", date.hour()),
            "H" => date.hour().to_string(),
            "hh" => format!("{:02}", date.hour12().1),
            "h" => date.hour12().1.to_string(),
            "mm" => format!("{:02}", date.minute()),
            "m" => date.minute().to_string(),
            "ss" => format!("{:02}", date.second()),
            "s" => date.second().to_string(),
            _ => unreachable!(),
        };
        output.push_str(&value);
        remaining = &remaining[token.len()..];
        if output.len() > context.max_output_bytes {
            return Err("Date formatting output limit exceeded".into());
        }
    }
    Ok(output)
}

fn local_datetime(
    seconds: i64,
    context: &EvalContext<'_>,
) -> Result<chrono::DateTime<FixedOffset>, String> {
    let utc = Utc
        .timestamp_opt(seconds, 0)
        .single()
        .ok_or("Timestamp is outside the supported date range")?;
    Ok(match context.timezone_offset_seconds {
        Some(offset) => {
            utc.with_timezone(&FixedOffset::east_opt(offset).ok_or("Invalid timezone offset")?)
        }
        None => utc.with_timezone(&Local).fixed_offset(),
    })
}

fn decimal_milliseconds(value: &str, multiplier: i128) -> Result<i128, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(0);
    }
    let (negative, value) = if let Some(raw) = value.strip_prefix('-') {
        (true, raw)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let (integer, fraction) = value.split_once('.').unwrap_or((value, ""));
    if integer.is_empty() && fraction.is_empty()
        || !integer.bytes().all(|c| c.is_ascii_digit())
        || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        return Err("$fmtDuration expects a finite decimal number".into());
    }
    let fraction = &fraction[..fraction.len().min(18)];
    let scale = 10i128.pow(fraction.len() as u32);
    let integer = if integer.is_empty() {
        0
    } else {
        integer.parse::<i128>().map_err(|_| "Duration overflow")?
    };
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().map_err(|_| "Duration overflow")?
    };
    let numerator = integer
        .checked_mul(scale)
        .and_then(|n| n.checked_add(fraction))
        .and_then(|n| n.checked_mul(multiplier))
        .ok_or("Duration overflow")?;
    let milliseconds = numerator / scale;
    Ok(if negative {
        -milliseconds
    } else {
        milliseconds
    })
}

fn format_duration(arguments: &[&str]) -> Result<String, String> {
    let multiplier = match arguments.get(1).copied().unwrap_or("s") {
        "m" => 60_000,
        "s" | "" => 1000,
        "ms" => 1,
        _ => return Err("$fmtDuration units must be m, s, or ms".into()),
    };
    let milliseconds = decimal_milliseconds(arguments[0], multiplier)?;
    let negative = milliseconds < 0;
    let milliseconds = milliseconds.unsigned_abs();
    let hours = arguments.get(2) == Some(&"1");
    let mut output = if hours {
        format!(
            "{:02}:{:02}:{:02}",
            milliseconds / 3_600_000,
            milliseconds / 60_000 % 60,
            milliseconds / 1000 % 60
        )
    } else {
        format!("{}:{:02}", milliseconds / 60_000, milliseconds / 1000 % 60)
    };
    if arguments.get(3) == Some(&"1") {
        output.push_str(&format!(".{:03}", milliseconds % 1000));
    }
    if negative {
        output.insert(0, '-');
    }
    Ok(output)
}

fn folder_depth(path: &str) -> usize {
    let parts: Vec<&str> = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    let prefix = if path.starts_with("\\\\") {
        2
    } else {
        usize::from(
            parts
                .first()
                .is_some_and(|part| part.len() == 2 && part.ends_with(':')),
        )
    };
    let filename = usize::from(!path.ends_with(['/', '\\']) && !parts.is_empty());
    parts.len().saturating_sub(prefix + filename)
}

fn ansi(text: &str, context: &EvalContext<'_>) -> Result<String, String> {
    let codepage = context
        .ansi_codepage
        .as_deref()
        .ok_or("$ansi requires the actual system codepage context")?;
    let numeric = codepage
        .strip_prefix("windows-")
        .or_else(|| codepage.strip_prefix("cp"))
        .unwrap_or(codepage)
        .parse::<u16>()
        .ok();
    let encoding = match numeric {
        Some(65001) => Some(encoding_rs::UTF_8),
        Some(932) => Some(encoding_rs::SHIFT_JIS),
        Some(936) => Some(encoding_rs::GBK),
        Some(949) => Some(encoding_rs::EUC_KR),
        Some(950) => Some(encoding_rs::BIG5),
        Some(value) => encoding_rs::Encoding::for_label(format!("windows-{value}").as_bytes()),
        _ => encoding_rs::Encoding::for_label(codepage.as_bytes()),
    }
    .ok_or_else(|| format!("Unsupported ANSI codepage {codepage}"))?;
    if encoding == encoding_rs::UTF_8 {
        return Ok(text.to_owned());
    }
    let mut output = String::new();
    for character in text.chars() {
        let mut buffer = [0u8; 4];
        let (encoded, output_encoding, unmappable) =
            encoding.encode(character.encode_utf8(&mut buffer));
        if unmappable {
            output.push('?');
        } else {
            let (decoded, malformed) = output_encoding.decode_without_bom_handling(&encoded);
            if malformed {
                return Err("ANSI encoding produced an invalid round trip".into());
            }
            output.push_str(&decoded);
        }
    }
    Ok(output)
}

// These are the only builtins that can return a foreign payload: the Boost
// engine, or an opt-in host's field resolver. Other builtin error factories
// replace library failures with OpenTag-owned diagnostics.
fn function_diagnostic(
    name: &str,
    a: &[&str],
    truths: &[bool],
    context: &EvalContext<'_>,
) -> Result<String, ErrorCause> {
    let output = match name {
        "regexp" => {
            arity(name, a, 3, 4).map_err(ErrorCause::owned_text)?;
            regex(a[1], a.get(3).is_none_or(|flag| *flag != "0"))
                .map_err(ErrorCause::external)?
                .replace(a[0], a[2])
                .map_err(ErrorCause::external)?
        }
        "meta" | "meta_sep" => {
            arity(name, a, if name == "meta" { 1 } else { 2 }, 2)
                .map_err(ErrorCause::owned_text)?;
            let resolved = context_values_diagnostic(context, a[0])?;
            let values = resolved.as_deref().unwrap_or_default();
            if name == "meta_sep" {
                values.join(a[1])
            } else if a.len() == 2 {
                values.get(count(a[1])).cloned().unwrap_or_default()
            } else {
                values.join(", ")
            }
        }
        _ => return function(name, a, truths, context).map_err(ErrorCause::owned_text),
    };
    bounded_with_limit(output, context.max_output_bytes).map_err(ErrorCause::owned_text)
}

fn function(
    name: &str,
    a: &[&str],
    truths: &[bool],
    context: &EvalContext<'_>,
) -> Result<String, String> {
    let fields = context.fields;
    let output = match name {
        "lower" | "upper" | "len" | "reverse" | "ord" | "char" => {
            arity(name, a, 1, 1)?;
            match name {
                "lower" => a[0].to_lowercase(),
                "upper" => a[0].to_uppercase(),
                "len" => a[0].chars().count().to_string(),
                "reverse" => a[0].chars().rev().collect(),
                "ord" => match a[0].chars().next() {
                    Some(c) if (c as u32) <= 0xffff => (c as u32).to_string(),
                    Some(_) => return Err("$ord supports only U+0000..U+FFFF".into()),
                    None => "0".into(),
                },
                "char" => char::from_u32(
                    u32::try_from(number(a[0])).map_err(|_| "Invalid Unicode code point")?,
                )
                .ok_or("Invalid Unicode code point")?
                .to_string(),
                _ => unreachable!(),
            }
        }
        "left" | "right" | "cutleft" | "cutright" | "repeat" | "num" => {
            arity(name, a, 2, 2)?;
            let n = count(a[1]);
            match name {
                "left" => slice(a[0], 0, n),
                "right" => slice(a[0], a[0].chars().count().saturating_sub(n), n),
                "cutleft" => slice(a[0], n, usize::MAX),
                "cutright" => slice(a[0], 0, a[0].chars().count().saturating_sub(n)),
                "repeat" => {
                    if a[0].len().saturating_mul(n) > context.max_output_bytes {
                        return Err("Repeat output limit exceeded".into());
                    }
                    a[0].repeat(n)
                }
                "num" => {
                    if n > context.max_output_bytes {
                        return Err("Numeric padding limit exceeded".into());
                    }
                    format!("{:0width$}", number(a[0]), width = n)
                }
                _ => unreachable!(),
            }
        }
        "mid" => {
            arity(name, a, 2, 3)?;
            let pos = count(a[1]);
            if pos == 0 {
                String::new()
            } else {
                slice(a[0], pos - 1, a.get(2).map_or(1, |n| count(n)))
            }
        }
        "caps" | "caps2" => {
            arity(name, a, 1, 2)?;
            title_case(a[0], a.get(1).copied(), name == "caps2")
        }
        "caps3" => {
            arity(name, a, 1, 2)?;
            let mut uppercase = true;
            let mut value = String::new();
            for character in a[0].chars() {
                if uppercase {
                    value.extend(character.to_uppercase());
                } else {
                    value.extend(character.to_lowercase());
                }
                uppercase = a
                    .get(1)
                    .is_some_and(|triggers| triggers.contains(character));
            }
            value
        }
        "ansi" => {
            arity(name, a, 1, 1)?;
            ansi(a[0], context)?
        }
        "fmtdate" => {
            arity(name, a, 1, 2)?;
            format_date(a[0], a.get(1).copied(), context)?
        }
        "fmtduration" => {
            arity(name, a, 1, 4)?;
            format_duration(a)?
        }
        "fmtnum" => {
            arity(name, a, 1, 1)?;
            format_number(number(a[0]), context)?
        }
        "folderdepth" => {
            arity(name, a, 1, 1)?;
            folder_depth(a[0]).to_string()
        }
        "rand" => {
            arity(name, a, 0, 0)?;
            rand::random_range(0..=32767u32).to_string()
        }
        "uuid" => {
            arity(name, a, 0, 0)?;
            uuid::Uuid::new_v4().hyphenated().to_string()
        }
        "getenv" => {
            arity(name, a, 1, 1)?;
            if a[0].eq_ignore_ascii_case("MP3TAGAPP") {
                context
                    .app_directory
                    .ok_or("$getenv(MP3TAGAPP) requires the application directory context")?
                    .to_owned()
            } else if a[0].eq_ignore_ascii_case("MP3TAGAPPDATA") {
                context
                    .app_data_directory
                    .ok_or("$getenv(MP3TAGAPPDATA) requires the configuration directory context")?
                    .to_owned()
            } else if let Some(environment) = context.environment {
                environment.get(a[0]).cloned().unwrap_or_default()
            } else {
                std::env::var(a[0]).unwrap_or_default()
            }
        }
        "columnnames" | "columnvalues" => {
            arity(name, a, 2, 2)?;
            let columns = context.columns.ok_or(
                "Column scripting functions require the application File List column context",
            )?;
            columns
                .iter()
                .map(|column| {
                    format!(
                        "{}{}{}",
                        a[0],
                        if name == "columnnames" {
                            &column.name
                        } else {
                            &column.value
                        },
                        a[1]
                    )
                })
                .collect()
        }
        "trim" | "trimleft" | "trimright" => {
            arity(name, a, 1, 2)?;
            let trim = |c: char| a.get(1).map_or(c.is_whitespace(), |s| s.contains(c));
            match name {
                "trim" => a[0].trim_matches(trim),
                "trimleft" => a[0].trim_start_matches(trim),
                _ => a[0].trim_end_matches(trim),
            }
            .into()
        }
        "replace" => {
            arity(name, a, 3, 63)?;
            if a.len() % 2 != 1 {
                return Err("$replace requires complete search/replacement pairs".into());
            }
            let mut value = a[0].to_owned();
            for pair in a[1..].chunks(2) {
                if pair[0].is_empty() {
                    return Err("Empty replacement search string".into());
                }
                value =
                    bounded_with_limit(value.replace(pair[0], pair[1]), context.max_output_bytes)?;
            }
            value
        }
        "regexp" => {
            arity(name, a, 3, 4)?;
            // Case is ignored unless the fourth argument is 0; inline `(?-i)`
            // and `(?i)` still override either default through Boost.
            regex(a[1], a.get(3).is_none_or(|flag| *flag != "0"))?.replace(a[0], a[2])?
        }
        "strstr" | "strrstr" | "strchr" | "strrchr" => {
            arity(name, a, 2, 2)?;
            let needle = if matches!(name, "strchr" | "strrchr") {
                a[1].chars()
                    .next()
                    .map(|c| c.to_string())
                    .unwrap_or_default()
            } else {
                a[1].into()
            };
            if needle.is_empty() {
                "0".into()
            } else {
                let pos = if matches!(name, "strrstr" | "strrchr") {
                    a[0].rfind(&needle)
                } else {
                    a[0].find(&needle)
                };
                pos.map_or(0, |p| a[0][..p].chars().count() + 1).to_string()
            }
        }
        "strcmp" | "stricmp" => {
            arity(name, a, 2, 2)?;
            boolean(if name == "strcmp" {
                a[0] == a[1]
            } else {
                a[0].to_lowercase() == a[1].to_lowercase()
            })
        }
        "if" => {
            arity(name, a, 3, 3)?;
            if truths[0] { a[1] } else { a[2] }.into()
        }
        "if2" => {
            arity(name, a, 2, 2)?;
            if truths[0] { a[0] } else { a[1] }.into()
        }
        "ifgreater" | "iflonger" => {
            arity(name, a, 4, 4)?;
            let yes = if name == "ifgreater" {
                number(a[0]) > number(a[1])
            } else {
                a[0].chars().count() > count(a[1])
            };
            if yes { a[2] } else { a[3] }.into()
        }
        "eql" | "neql" | "grtr" | "geql" | "less" | "leql" => {
            arity(name, a, 2, 2)?;
            let eq = match (a[0].parse::<i64>(), a[1].parse::<i64>()) {
                (Ok(x), Ok(y)) => x == y,
                _ => a[0].to_lowercase() == a[1].to_lowercase(),
            };
            boolean(match name {
                "eql" => eq,
                "neql" => !eq,
                "grtr" => number(a[0]) > number(a[1]),
                "geql" => number(a[0]) >= number(a[1]),
                "less" => number(a[0]) < number(a[1]),
                _ => number(a[0]) <= number(a[1]),
            })
        }
        "and" | "or" => {
            arity(name, a, 2, 63)?;
            boolean(if name == "and" {
                truths.iter().all(|s| *s)
            } else {
                truths.iter().any(|s| *s)
            })
        }
        "not" | "odd" | "isdigit" => {
            arity(name, a, 1, 1)?;
            boolean(match name {
                "not" => !truths[0],
                "odd" => number(a[0]) % 2 != 0,
                _ => a[0].starts_with(|c: char| c.is_ascii_digit()),
            })
        }
        "add" | "sub" | "mul" | "div" | "mod" => {
            arity(name, a, 2, 64)?;
            let mut value = number(a[0]);
            for arg in &a[1..] {
                let other = number(arg);
                value = match name {
                    "add" => value.checked_add(other),
                    "sub" => value.checked_sub(other),
                    "mul" => value.checked_mul(other),
                    "div" => value.checked_div(other),
                    _ => value.checked_rem(other),
                }
                .ok_or("Integer overflow or division by zero")?;
            }
            value.to_string()
        }
        "meta" | "meta_sep" => {
            arity(name, a, if name == "meta" { 1 } else { 2 }, 2)?;
            let resolved = context_values(context, a[0])?;
            let values = resolved.as_deref().unwrap_or_default();
            if name == "meta_sep" {
                values.join(a[1])
            } else if a.len() == 2 {
                values.get(count(a[1])).cloned().unwrap_or_default()
            } else {
                values.join(", ")
            }
        }
        "list" => {
            arity(name, a, 3, 4)?;
            context
                .raw_fields
                .unwrap_or(fields)
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}{}{}{}{}",
                        a[0],
                        k,
                        a[1],
                        v.join(a.get(3).copied().unwrap_or(",")),
                        a[2]
                    )
                })
                .collect()
        }
        "sort" | "dedup" => {
            arity(name, a, 2, if name == "dedup" { 3 } else { 2 })?;
            if a[1].is_empty() {
                return Err("Segment delimiter must not be empty".into());
            }
            let mut parts: Vec<&str> = a[0].split(a[1]).collect();
            if name == "sort" {
                parts.sort();
            } else {
                let mut seen = std::collections::HashSet::new();
                parts.retain(|v| {
                    seen.insert(if a.get(2) == Some(&"1") {
                        v.to_lowercase()
                    } else {
                        v.to_string()
                    })
                });
            }
            parts.join(a[1])
        }
        "validate" => {
            arity(name, a, 2, 2)?;
            a[0].chars()
                .map(|c| {
                    if "/?*\"<>|:".contains(c) {
                        a[1].to_owned()
                    } else {
                        c.to_string()
                    }
                })
                .collect()
        }
        "distance" => {
            arity(name, a, 2, 2)?;
            let x: Vec<char> = a[0].chars().collect();
            let y: Vec<char> = a[1].chars().collect();
            if x.len().saturating_mul(y.len()) > 4 * MAX_OUTPUT {
                return Err("Edit distance work limit exceeded".into());
            }
            let mut previous: Vec<usize> = (0..=y.len()).collect();
            for (i, left) in x.iter().enumerate() {
                let mut row = vec![i + 1; y.len() + 1];
                for (j, right) in y.iter().enumerate() {
                    row[j + 1] = (row[j] + 1)
                        .min(previous[j + 1] + 1)
                        .min(previous[j] + usize::from(left != right));
                }
                previous = row;
            }
            previous[y.len()].to_string()
        }
        _ => return Err(format!("Unsupported scripting function: ${name}")),
    };
    bounded_with_limit(output, context.max_output_bytes)
}

/// Import complete matching strings; filename extensions are omitted unless
/// the pattern explicitly includes them. Adjacent captures are ambiguous.
pub fn parse_filename(filename: &str, pattern: &str) -> Result<Fields, String> {
    if filename.len() > MAX_OUTPUT {
        return Err("Filename pattern input limit exceeded".into());
    }
    CompiledFilenamePattern::compile(pattern)?.parse(filename)
}

/// A reusable filename matching pattern. Parsing retains the exact public
/// `parse_filename` matching and extension rules without recompiling per file.
pub struct CompiledFilenamePattern {
    expression: Regex,
    names: Vec<String>,
    pattern: String,
}

impl CompiledFilenamePattern {
    pub fn compile(pattern: &str) -> Result<Self, String> {
        if pattern.len() > MAX_OUTPUT {
            return Err("Filename pattern input limit exceeded".into());
        }
        let mut expression = String::from("\\A");
        let mut names = Vec::new();
        let mut remainder = pattern;
        let mut last_capture = false;
        while let Some(start) = remainder.find('%') {
            let literal = &remainder[..start];
            if last_capture && literal.is_empty() {
                return Err("Adjacent filename placeholders are ambiguous".into());
            }
            expression.push_str(&regex::escape(literal));
            remainder = &remainder[start + 1..];
            let end = remainder
                .find('%')
                .ok_or("Unterminated filename placeholder")?;
            let name = &remainder[..end];
            if name.is_empty() || name.starts_with('_') {
                return Err("Invalid filename capture field".into());
            }
            names.push(name.to_uppercase());
            expression.push_str(if pattern.contains('\\') {
                "([^\\\\]*?)"
            } else {
                "(.*?)"
            });
            remainder = &remainder[end + 1..];
            last_capture = true;
        }
        if names.is_empty() {
            return Err("Filename pattern contains no field placeholders".into());
        }
        expression.push_str(&regex::escape(remainder));
        expression.push_str("\\z");
        Ok(Self {
            expression: filename_regex(&expression)?,
            names,
            pattern: pattern.to_owned(),
        })
    }

    pub fn parse(&self, filename: &str) -> Result<Fields, String> {
        self.try_parse(filename)?
            .ok_or_else(|| "Filename does not completely match the pattern".into())
    }

    /// Like [`parse`](Self::parse), with a filename that does not match the
    /// pattern as `None`: Filename → Tag leaves such files unchanged.
    pub fn try_parse(&self, filename: &str) -> Result<Option<Fields>, String> {
        if filename.len() > MAX_OUTPUT {
            return Err("Filename pattern input limit exceeded".into());
        }
        let pattern = self.pattern.as_str();
        // Windows-style folder separators remain literal matching delimiters.
        let normalized = if pattern.contains('\\') {
            let normalized = filename.replace('/', "\\");
            if pattern.starts_with('\\') || pattern.as_bytes().get(1) == Some(&b':') {
                normalized
            } else {
                let components: Vec<&str> = normalized.split('\\').collect();
                let required = pattern.matches('\\').count() + 1;
                if components.len() < required {
                    return Ok(None);
                }
                components[components.len() - required..].join("\\")
            }
        } else {
            filename
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(filename)
                .to_owned()
        };
        let filename = normalized.as_str();
        let last_component = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
        let extension = last_component
            .rfind('.')
            .filter(|i| *i > 0)
            .map(|i| &last_component[i..]);
        let omit_extension =
            extension.is_some_and(|ext| !pattern.to_lowercase().ends_with(&ext.to_lowercase()));
        let candidate = if omit_extension {
            &filename[..filename.len() - extension.unwrap().len()]
        } else {
            filename
        };
        let Some(captures) = self.expression.captures(candidate) else {
            return Ok(None);
        };
        let mut fields = Fields::new();
        for (i, name) in self.names.iter().enumerate() {
            if name == "DUMMY" {
                continue;
            }
            let value = captures.get(i + 1).unwrap().as_str();
            fields
                .entry(name.clone())
                .or_default()
                .push(value.to_owned());
        }
        Ok(Some(fields))
    }
}

fn selected_keys(fields: &Fields, field: &str) -> Result<Vec<String>, String> {
    if field.eq_ignore_ascii_case("_ALL") || field.eq_ignore_ascii_case("_TAG") {
        return Ok(fields
            .keys()
            .filter(|k| {
                !k.starts_with('_')
                    || (field.eq_ignore_ascii_case("_ALL") && k.eq_ignore_ascii_case("_FILENAME"))
            })
            .cloned()
            .collect());
    }
    if field.is_empty() {
        return Err("Action field must not be empty".into());
    }
    if field.starts_with('_') && !field.eq_ignore_ascii_case("_FILENAME") {
        return Err(format!("Unsupported action target {field}"));
    }
    Ok(vec![fields
        .keys()
        .find(|k| k.eq_ignore_ascii_case(field))
        .cloned()
        .unwrap_or_else(|| field.to_uppercase())])
}

pub fn apply_action(fields: &Fields, action: &Action) -> Result<Fields, String> {
    let mut result = fields.clone();
    if let Action::Remove { fields: names } = action {
        for name in names {
            for key in selected_keys(&result, name)? {
                result.remove(&key);
            }
        }
        return Ok(result);
    }
    let field = match action {
        Action::Format { field, .. }
        | Action::Replace { field, .. }
        | Action::RegexReplace { field, .. }
        | Action::Case { field, .. } => field,
        _ => unreachable!(),
    };
    let keys = selected_keys(fields, field)?;
    // Validate operations even if no matching field values exist.
    let compiled = match action {
        Action::RegexReplace {
            pattern,
            replacement,
            ..
        } => {
            let re = regex(pattern, false)?;
            regex_replace("", &re, replacement)?;
            Some(re)
        }
        Action::Replace {
            from,
            case_sensitive,
            ..
        } => {
            if from.is_empty() {
                return Err("Replacement search must not be empty".into());
            }
            if !case_sensitive {
                Some(regex(&regex::escape(from), true)?)
            } else {
                None
            }
        }
        Action::Case { mode, .. } => {
            if !["lower", "upper", "title", "sentence"].contains(&mode.as_str()) {
                return Err(format!("Unsupported case mode {mode}"));
            }
            None
        }
        _ => None,
    };
    let formatted = if let Action::Format { template, .. } = action {
        Some(evaluate(template, fields)?)
    } else {
        None
    };
    for key in keys {
        if let Some(value) = &formatted {
            if value.is_empty() {
                result.remove(&key);
            } else {
                result.insert(key, vec![value.clone()]);
            }
            continue;
        }
        let Some(values) = fields.get(&key) else {
            continue;
        };
        let transformed: Vec<String> = values
            .iter()
            .map(|value| {
                let text = match action {
                    Action::Replace {
                        from,
                        to,
                        case_sensitive,
                        ..
                    } => {
                        if *case_sensitive {
                            value.replace(from, to)
                        } else {
                            compiled
                                .as_ref()
                                .unwrap()
                                .replace(value, &literal_regex_replacement(to))?
                        }
                    }
                    Action::RegexReplace { replacement, .. } => {
                        return regex_replace(value, compiled.as_ref().unwrap(), replacement)
                    }
                    Action::Case { mode, .. } => match mode.as_str() {
                        "lower" => value.to_lowercase(),
                        "upper" => value.to_uppercase(),
                        "title" => title_case(value, None, false),
                        "sentence" => {
                            let mut chars = value.chars();
                            chars
                                .next()
                                .map(|c| {
                                    c.to_uppercase().collect::<String>()
                                        + &chars.as_str().to_lowercase()
                                })
                                .unwrap_or_default()
                        }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                };
                bounded(text)
            })
            .collect::<Result<_, _>>()?;
        result.insert(key, transformed);
    }
    Ok(result)
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    use crate::batch_errors::ErrorCause;

    #[test]
    fn evaluator_diagnostics_keep_owned_errors_distinct_from_identical_host_errors() {
        let fields = Fields::new();
        let mut context = EvalContext::new(&fields);
        let expression = CompiledTemplate::compile("$div(1,0)").unwrap();
        let error = expression
            .evaluate_with_context_diagnostic(&context)
            .unwrap_err();
        assert!(
            matches!(&error, ErrorCause::OwnedText { value } if value == "Integer overflow or division by zero")
        );
        assert_eq!(
            expression.evaluate_with_context(&context).unwrap_err(),
            error.to_string()
        );

        struct Foreign;
        impl ScriptExtension for Foreign {
            fn call(&self, _: &str, _: &[ScriptValue]) -> Result<ScriptValue, String> {
                Err("Integer overflow or division by zero".into())
            }
            fn field(&self, _: &str) -> Result<Option<Vec<String>>, String> {
                Err("Integer overflow or division by zero".into())
            }
        }
        context.extension = Some(&Foreign);
        let extension = CompiledTemplate::compile_with_functions(
            "$foreign()",
            &[ScriptFunctionSpec {
                name: "foreign".into(),
                min_args: 0,
                max_args: 0,
            }],
        )
        .unwrap();
        for expression in [
            extension,
            CompiledTemplate::compile("%Title%").unwrap(),
            CompiledTemplate::compile("$meta(Title)").unwrap(),
        ] {
            let error = expression
                .evaluate_with_context_diagnostic(&context)
                .unwrap_err();
            assert!(
                matches!(&error, ErrorCause::External { value } if value == "Integer overflow or division by zero")
            );
            assert_eq!(
                expression.evaluate_with_context(&context).unwrap_err(),
                error.to_string()
            );
        }
    }

    #[test]
    fn regex_engine_diagnostics_remain_external_in_all_evaluation_modes() {
        let fields = Fields::new();
        let context = EvalContext::new(&fields);
        let expression = CompiledTemplate::compile("$regexp('a','[','b')").unwrap();
        let legacy = expression.evaluate_with_context(&context).unwrap_err();
        for error in [
            expression
                .evaluate_with_context_diagnostic(&context)
                .unwrap_err(),
            expression
                .evaluate_filename_with_context_diagnostic(&context)
                .unwrap_err(),
            expression
                .evaluate_filename_path_with_context_diagnostic(&context)
                .unwrap_err(),
        ] {
            assert!(matches!(&error, ErrorCause::External { .. }));
            assert_eq!(error.to_string(), legacy);
        }
    }
}
