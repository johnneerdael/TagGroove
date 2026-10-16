//! Portable MTE export rendering. No file reads, writes, launches or network IO.
//! Reference: https://docs.mp3tag.de/export/ and /format/.
use crate::{
    CompiledTemplate, EvalContext, Fields, ScriptColumn, ScriptExtension, ScriptFunctionSpec,
    ScriptNumberFormat, ScriptValue,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportRow {
    pub fields: Fields,
    #[serde(default)]
    pub context_fields: Fields,
    pub path: String,
    pub duration: f64,
    pub size: u64,
    #[serde(default)]
    pub columns: Option<Vec<ScriptColumn>>,
}
impl ExportRow {
    pub fn new(fields: Fields, path: impl Into<String>, duration: f64, size: u64) -> Self {
        Self {
            fields,
            context_fields: Fields::new(),
            path: path.into(),
            duration,
            size,
            columns: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportOptions {
    pub filename: String,
    pub filename_override: Option<String>,
    pub base_directory: String,
    /// Used unless `$filename` names an encoding. Mp3tag writes ANSI by
    /// default: UTF-8 and UTF-16 are chosen with `$filename(name,enc)`.
    pub encoding: String,
    pub bom: bool,
    pub partition: Option<String>,
    pub natural_sort: bool,
    pub case_sensitive_sort: bool,
    pub locale: Option<String>,
    pub timestamp_seconds: Option<i64>,
    pub timezone_offset_seconds: Option<i32>,
    pub ansi_codepage: Option<String>,
    pub number_format: Option<ScriptNumberFormat>,
    pub environment: Option<BTreeMap<String, String>>,
    pub app_directory: Option<String>,
    pub app_data_directory: Option<String>,
    /// Folder the file list was last changed to (`%_workingdir%`,
    /// `%_workingpath%`), in the body and the export file name.
    pub working_directory: Option<String>,
    pub max_output_bytes: usize,
    pub max_steps: usize,
}
impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            filename: "export.txt".into(),
            filename_override: None,
            base_directory: ".".into(),
            encoding: "ansi".into(),
            bom: false,
            partition: None,
            natural_sort: false,
            case_sensitive_sort: false,
            locale: None,
            timestamp_seconds: None,
            timezone_offset_seconds: None,
            ansi_codepage: None,
            number_format: None,
            environment: None,
            app_directory: None,
            app_data_directory: None,
            working_directory: None,
            max_output_bytes: 256 * 1024 * 1024,
            max_steps: 10_000_000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportDocument {
    pub filename: String,
    pub encoding: String,
    pub bom: bool,
    pub text: String,
}

#[derive(Debug)]
enum Node {
    Text(String),
    Field(String),
    Optional(Vec<Node>),
    Function(String, Vec<Vec<Node>>),
    Loop {
        sort: Vec<Node>,
        limit: Option<Vec<Node>>,
        body: Vec<Node>,
    },
}
struct Parser {
    chars: Vec<char>,
    position: usize,
}
impl Parser {
    fn sequence(
        &mut self,
        stops: &[char],
        loop_body: bool,
        depth: usize,
    ) -> Result<(Vec<Node>, bool), String> {
        if depth > 128 {
            return Err("Export nesting limit exceeded".into());
        }
        let mut nodes = Vec::new();
        let mut text = String::new();
        while let Some(&character) = self.chars.get(self.position) {
            if stops.contains(&character) {
                break;
            }
            if !matches!(character, '\'' | '%' | '[' | ']' | '$') {
                text.push(character);
                self.position += 1;
                continue;
            }
            if !text.is_empty() {
                nodes.push(Node::Text(std::mem::take(&mut text)));
            }
            self.position += 1;
            match character {
                '\'' => {
                    if self.chars.get(self.position) == Some(&'\'') {
                        self.position += 1;
                        nodes.push(Node::Text("'".into()));
                        continue;
                    }
                    let mut raw = String::new();
                    loop {
                        let character = *self
                            .chars
                            .get(self.position)
                            .ok_or("Unterminated export quotation")?;
                        self.position += 1;
                        if character == '\'' {
                            if self.chars.get(self.position) == Some(&'\'') {
                                raw.push('\'');
                                self.position += 1;
                            } else {
                                break;
                            }
                        } else {
                            raw.push(character);
                        }
                    }
                    nodes.push(Node::Text(raw));
                }
                '%' => {
                    if self.chars.get(self.position) == Some(&'%') {
                        self.position += 1;
                        nodes.push(Node::Text("%".into()));
                        continue;
                    }
                    let start = self.position;
                    while self.chars.get(self.position).is_some_and(|c| *c != '%') {
                        self.position += 1;
                    }
                    if self.position == self.chars.len() {
                        return Err("Unterminated export placeholder".into());
                    }
                    let name: String = self.chars[start..self.position].iter().collect();
                    self.position += 1;
                    nodes.push(Node::Field(name));
                }
                '[' => {
                    let (body, _) = self.sequence(&[']'], false, depth + 1)?;
                    if self.chars.get(self.position) != Some(&']') {
                        return Err("Unclosed optional export section".into());
                    }
                    self.position += 1;
                    nodes.push(Node::Optional(body));
                }
                ']' => return Err("Unexpected optional closing bracket".into()),
                '$' => {
                    if self.chars.get(self.position) == Some(&'$') {
                        self.position += 1;
                        nodes.push(Node::Text("$".into()));
                        continue;
                    }
                    let start = self.position;
                    while self
                        .chars
                        .get(self.position)
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
                    {
                        self.position += 1;
                    }
                    let name = self.chars[start..self.position]
                        .iter()
                        .collect::<String>()
                        .to_ascii_lowercase();
                    if self.chars.get(self.position) != Some(&'(') {
                        return Err(format!("Expected export function call after ${name}"));
                    }
                    self.position += 1;
                    let mut args = Vec::new();
                    if self.chars.get(self.position) == Some(&')') {
                        self.position += 1;
                    } else {
                        loop {
                            let (argument, _) = self.sequence(&[',', ')'], false, depth + 1)?;
                            args.push(argument);
                            match self.chars.get(self.position) {
                                Some(',') => self.position += 1,
                                Some(')') => {
                                    self.position += 1;
                                    break;
                                }
                                _ => return Err(format!("Unterminated export function ${name}")),
                            }
                            if args.len() > 64 {
                                return Err("Export function argument limit exceeded".into());
                            }
                        }
                    }
                    if name == "loopend" {
                        if !args.is_empty() {
                            return Err("$loopend takes no arguments".into());
                        }
                        if !loop_body {
                            return Err("$loopend without a matching $loop".into());
                        }
                        if !text.is_empty() {
                            nodes.push(Node::Text(text));
                        }
                        return Ok((nodes, true));
                    }
                    if name == "loop" {
                        if !(1..=2).contains(&args.len()) {
                            return Err("$loop requires a sort field/expression and an optional per-key limit".into());
                        }
                        let (body, closed) = self.sequence(&[], true, depth + 1)?;
                        if !closed {
                            return Err("$loop without a matching $loopend".into());
                        }
                        let sort = args.remove(0);
                        let limit = args.pop();
                        nodes.push(Node::Loop { sort, limit, body });
                    } else {
                        nodes.push(Node::Function(name, args));
                    }
                }
                _ => unreachable!(),
            }
        }
        if !text.is_empty() {
            nodes.push(Node::Text(text));
        }
        Ok((nodes, false))
    }
}

fn has_loop(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Loop { .. } => true,
        Node::Optional(body) => has_loop(body),
        Node::Function(_, args) => args.iter().any(|arg| has_loop(arg)),
        _ => false,
    })
}
fn literal(nodes: &[Node]) -> Option<String> {
    nodes
        .iter()
        .map(|node| {
            if let Node::Text(text) = node {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.concat())
}
fn quote(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else if text.chars().all(|character| character == '\'') {
        "'".repeat(text.len() * 2)
    } else {
        format!("'{}'", text.replace('\'', "''"))
    }
}

struct LoopPlan {
    key: CompiledTemplate,
    limit: Option<CompiledTemplate>,
    body: CompiledTemplate,
    grouped: bool,
}
struct Program {
    body: CompiledTemplate,
    filename: Option<(CompiledTemplate, Option<CompiledTemplate>)>,
    loops: BTreeMap<String, LoopPlan>,
}
struct Compiler {
    loops: BTreeMap<String, LoopPlan>,
    next: usize,
    prefix: String,
}
impl Compiler {
    fn specs(&self) -> Vec<ScriptFunctionSpec> {
        let mut specs = vec![
            ScriptFunctionSpec {
                name: "get".into(),
                min_args: 1,
                max_args: 1,
            },
            ScriptFunctionSpec {
                name: "put".into(),
                min_args: 2,
                max_args: 2,
            },
            ScriptFunctionSpec {
                name: "puts".into(),
                min_args: 2,
                max_args: 2,
            },
        ];
        specs.extend((0..self.next).map(|index| ScriptFunctionSpec {
            name: format!("{}{index}", self.prefix),
            min_args: 0,
            max_args: 0,
        }));
        specs
    }
    fn compile(&mut self, nodes: &[Node]) -> Result<CompiledTemplate, String> {
        let text = self.emit(nodes)?;
        CompiledTemplate::compile_with_functions(&text, &self.specs())
    }
    fn emit(&mut self, nodes: &[Node]) -> Result<String, String> {
        let mut text = String::new();
        let mut pending = String::new();
        for node in nodes {
            if let Node::Text(value) = node {
                pending.push_str(value);
                continue;
            }
            text.push_str(&quote(&std::mem::take(&mut pending)));
            match node {
                Node::Text(_) => unreachable!(),
                Node::Field(name) => {
                    if name.is_empty() {
                        return Err("Empty export placeholder".into());
                    }
                    text.push('%');
                    text.push_str(name);
                    text.push('%');
                }
                Node::Optional(body) => {
                    text.push('[');
                    text.push_str(&self.emit(body)?);
                    text.push(']');
                }
                Node::Function(name, args) => {
                    if name == "filename" {
                        return Err("$filename must appear once at the beginning of the export configuration".into());
                    }
                    if !crate::SUPPORTED_FUNCTIONS.contains(&name.as_str())
                        && !["get", "put", "puts"].contains(&name.as_str())
                    {
                        return Err(format!("Unsupported export function ${name}"));
                    }
                    text.push('$');
                    text.push_str(name);
                    text.push('(');
                    for (index, arg) in args.iter().enumerate() {
                        if index > 0 {
                            text.push(',');
                        }
                        text.push_str(&self.emit(arg)?);
                    }
                    text.push(')');
                }
                Node::Loop { sort, limit, body } => {
                    let name = format!("{}{}", self.prefix, self.next);
                    self.next += 1;
                    let key = if let Some(field) = literal(sort) {
                        CompiledTemplate::compile(&format!("$meta({},0)", quote(&field)))?
                    } else {
                        self.compile(sort)?
                    };
                    let limit = limit
                        .as_ref()
                        .map(|limit| self.compile(limit))
                        .transpose()?;
                    let compiled_body = self.compile(body)?;
                    self.loops.insert(
                        name.clone(),
                        LoopPlan {
                            key,
                            limit,
                            body: compiled_body,
                            grouped: has_loop(body),
                        },
                    );
                    text.push('$');
                    text.push_str(&name);
                    text.push_str("()");
                }
            }
        }
        text.push_str(&quote(&pending));
        Ok(text)
    }
}

fn compile(template: &str) -> Result<Program, String> {
    if template.len() > 1_048_576 {
        return Err("Export template input limit exceeded".into());
    }
    let mut parser = Parser {
        chars: template.trim_start_matches('\u{feff}').chars().collect(),
        position: 0,
    };
    let (mut nodes, _) = parser.sequence(&[], false, 0)?;
    let first = nodes
        .iter()
        .position(|node| !matches!(node,Node::Text(text)if text.trim().is_empty()));
    let filename = if first
        .is_some_and(|index| matches!(&nodes[index],Node::Function(name,_)if name=="filename"))
    {
        let Node::Function(_, mut args) = nodes.remove(first.unwrap()) else {
            unreachable!()
        };
        if args.len() > 2 {
            return Err("$filename takes at most two parameters".into());
        }
        let name = if args.is_empty() {
            Vec::new()
        } else {
            args.remove(0)
        };
        Some((name, args.pop()))
    } else {
        None
    };
    let mut prefix = "ot_export_loop_".to_string();
    while template.to_ascii_lowercase().contains(&prefix) {
        prefix.push('_');
    }
    let mut compiler = Compiler {
        loops: BTreeMap::new(),
        next: 0,
        prefix,
    };
    let filename = filename
        .map(|(name, encoding)| {
            Ok::<_, String>((
                compiler.compile(&name)?,
                encoding.map(|e| compiler.compile(&e)).transpose()?,
            ))
        })
        .transpose()?;
    let body = compiler.compile(&nodes)?;
    Ok(Program {
        body,
        filename,
        loops: compiler.loops,
    })
}

#[derive(Clone, Copy, Default)]
struct Totals {
    files: usize,
    size: u64,
    seconds: f64,
}
fn totals(rows: &[ExportRow], members: &[usize]) -> Result<Totals, String> {
    let mut total = Totals::default();
    for index in members {
        let row = &rows[*index];
        total.files += 1;
        total.size = total
            .size
            .checked_add(row.size)
            .ok_or("Export total size overflow")?;
        total.seconds += row.duration;
        if !total.seconds.is_finite() {
            return Err("Export duration overflow".into());
        }
    }
    Ok(total)
}
#[derive(Clone)]
struct Frame {
    members: Rc<[usize]>,
    row: Option<usize>,
    counter: usize,
}
struct State {
    frame: Frame,
    totals: Totals,
    max_counter: usize,
    variables: BTreeMap<String, ScriptValue>,
    filename: String,
    steps: usize,
}
struct Engine<'a> {
    rows: &'a [ExportRow],
    options: &'a ExportOptions,
    program: &'a Program,
    state: RefCell<State>,
    empty: Fields,
}
impl<'a> Engine<'a> {
    fn new(
        rows: &'a [ExportRow],
        members: Rc<[usize]>,
        options: &'a ExportOptions,
        program: &'a Program,
    ) -> Result<Self, String> {
        let total = totals(rows, &members)?;
        let row = members.first().copied();
        Ok(Self {
            rows,
            options,
            program,
            state: RefCell::new(State {
                frame: Frame {
                    members,
                    row,
                    counter: usize::from(row.is_some()),
                },
                totals: total,
                max_counter: 0,
                variables: BTreeMap::new(),
                filename: resolve_path(&options.base_directory, &options.filename),
                steps: 0,
            }),
            empty: Fields::new(),
        })
    }
    fn step(&self) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        state.steps = state
            .steps
            .checked_add(1)
            .ok_or("Export work limit exceeded")?;
        if state.steps > self.options.max_steps {
            return Err("Export work limit exceeded".into());
        }
        Ok(())
    }
    fn eval(&self, template: &CompiledTemplate) -> Result<ScriptValue, String> {
        self.step()?;
        let row = self
            .state
            .borrow()
            .frame
            .row
            .and_then(|index| self.rows.get(index));
        let mut fields = row.map(|row| row.fields.clone()).unwrap_or_default();
        if let Some(row) = row {
            fields.extend(row.context_fields.clone());
            let mut set = |name: &str, value: String| {
                fields.entry(name.into()).or_insert_with(|| vec![value]);
            };
            set("_PATH", row.path.clone());
            set("_FILENAME_EXT", file_name(&row.path));
            set("_FILENAME", file_stem(&row.path));
            set("_FOLDERPATH", folder(&row.path));
            set("_EXTENSION", extension(&row.path));
            set("_LENGTH_SECONDS", row.duration.round().to_string());
            set("_FILE_SIZE_BYTES", row.size.to_string());
        }
        let mut context = EvalContext::new(&fields);
        context.raw_fields = Some(row.map(|row| &row.fields).unwrap_or(&self.empty));
        context.extension = Some(self);
        context.max_output_bytes = self.options.max_output_bytes;
        if let Some(locale) = &self.options.locale {
            context.locale = locale.clone();
        }
        context.number_format = self.options.number_format.as_ref();
        context.timestamp_seconds = self.options.timestamp_seconds;
        context.timezone_offset_seconds = self.options.timezone_offset_seconds;
        if let Some(codepage) = &self.options.ansi_codepage {
            context.ansi_codepage = Some(codepage.clone());
        }
        context.columns = row.and_then(|row| row.columns.as_deref());
        context.environment = self.options.environment.as_ref();
        context.app_directory = self.options.app_directory.as_deref();
        context.app_data_directory = self.options.app_data_directory.as_deref();
        context.working_directory = self.options.working_directory.as_deref();
        template.evaluate_value_with_context(&context)
    }
    fn render_loop(&self, plan: &LoopPlan) -> Result<ScriptValue, String> {
        self.step()?;
        let parent = self.state.borrow().frame.clone();
        let mut keyed = Vec::with_capacity(parent.members.len());
        for (position, index) in parent.members.iter().enumerate() {
            {
                let mut state = self.state.borrow_mut();
                state.frame.row = Some(*index);
                state.frame.counter = position + 1;
            }
            let key = self.eval(&plan.key)?.text;
            let key = if self.options.case_sensitive_sort {
                key
            } else {
                key.to_lowercase()
            };
            keyed.push((key, *index));
        }
        keyed.sort_by(|(a, _), (b, _)| {
            compare(a, b, self.options.natural_sort).then_with(|| a.cmp(b))
        });
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (key, index) in keyed {
            if let Some((last, members)) = groups.last_mut() {
                if *last == key {
                    members.push(index);
                    continue;
                }
            }
            groups.push((key, vec![index]));
        }
        let limit = if let Some(limit) = &plan.limit {
            self.state.borrow_mut().frame = parent.clone();
            let limit = self.eval(limit)?.text;
            if limit.is_empty() {
                0
            } else {
                limit
                    .parse::<usize>()
                    .map_err(|_| "$loop limit must be a nonnegative integer")?
            }
        } else {
            0
        };
        let mut result = ScriptValue::default();
        let mut counter = 0;
        for (_, members) in groups {
            let members: Rc<[usize]> = members.into();
            let iterations = if plan.grouped {
                1
            } else if limit == 0 {
                members.len()
            } else {
                limit.min(members.len())
            };
            let group_totals = totals(self.rows, &members)?;
            for index in members.iter().take(iterations) {
                counter += 1;
                {
                    let mut state = self.state.borrow_mut();
                    state.frame = Frame {
                        members: Rc::clone(&members),
                        row: Some(*index),
                        counter,
                    };
                    state.totals = group_totals;
                }
                let value = self.eval(&plan.body)?;
                if result.text.len().saturating_add(value.text.len())
                    > self.options.max_output_bytes
                {
                    return Err("Export output limit exceeded".into());
                }
                result.text.push_str(&value.text);
                result.present |= value.present;
                result.truth |= value.truth;
            }
        }
        let all_totals = totals(self.rows, &parent.members)?;
        let mut state = self.state.borrow_mut();
        state.frame = parent;
        state.totals = all_totals;
        state.max_counter = counter;
        Ok(result)
    }
    fn supplied_hash(&self, name: &str) -> Option<String> {
        let row = self.state.borrow().frame.row?;
        let row = &self.rows[row];
        lookup(&row.context_fields, name)
            .and_then(|values| values.first())
            .cloned()
    }
}
impl ScriptExtension for Engine<'_> {
    fn call(&self, name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String> {
        self.step()?;
        match name {
            "get" => Ok(self
                .state
                .borrow()
                .variables
                .get(&args[0].text)
                .cloned()
                .unwrap_or_default()),
            "put" | "puts" => {
                self.state
                    .borrow_mut()
                    .variables
                    .insert(args[0].text.clone(), args[1].clone());
                Ok(if name == "put" {
                    args[1].clone()
                } else {
                    ScriptValue::default()
                })
            }
            _ => self
                .program
                .loops
                .get(name)
                .ok_or_else(|| format!("Unknown export function ${name}"))
                .and_then(|plan| self.render_loop(plan)),
        }
    }
    fn field(&self, name: &str) -> Result<Option<Vec<String>>, String> {
        let name = name.to_ascii_uppercase();
        let state = self.state.borrow();
        let totals = state.totals;
        let value = match name.as_str() {
            "_COUNTER" => state.frame.counter.to_string(),
            "_MAX_COUNTER" => state.max_counter.to_string(),
            "_TOTAL_FILES" => totals.files.to_string(),
            "_TOTAL_SIZE_RAW" => totals.size.to_string(),
            "_TOTAL_TIME_RAW" => totals.seconds.round().to_string(),
            "_TOTAL_SIZE" => size_text(totals.size, self.options),
            "_TOTAL_TIME" => time_text(totals.seconds, true),
            "_TOTAL_TIME2" => time_text(totals.seconds, false),
            "_FILENAME_REL" | "_FOLDERPATH_REL" => {
                let row = state
                    .frame
                    .row
                    .and_then(|index| self.rows.get(index))
                    .ok_or("Relative export placeholders require an input row")?;
                relative_path(&row.path, &state.filename, name == "_FOLDERPATH_REL")
            }
            "_CRC" | "_MD5" | "_MD5AUDIO" => {
                drop(state);
                let value=self.supplied_hash(&name).ok_or_else(||format!("{name} requires an actual supplied file hash; the pure export renderer does not read files"))?;
                let expected = if name == "_CRC" { 8 } else { 32 };
                // An empty audio MD5 means the format has none.
                if name == "_MD5AUDIO" && value.is_empty() {
                    return Ok(Some(vec![value]));
                }
                if value.len() != expected || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(format!("Invalid supplied {name} hash"));
                }
                return Ok(Some(vec![value]));
            }
            _ => return Ok(None),
        };
        Ok(Some(vec![value]))
    }
}

fn lookup<'a>(fields: &'a Fields, name: &str) -> Option<&'a Vec<String>> {
    fields.get(name).or_else(|| {
        fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value)
    })
}
fn compare(a: &str, b: &str, natural: bool) -> Ordering {
    if !natural {
        return a.cmp(b);
    }
    let (x, y) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < x.len() && j < y.len() {
        if x[i].is_ascii_digit() && y[j].is_ascii_digit() {
            let (start_i, start_j) = (i, j);
            while i < x.len() && x[i].is_ascii_digit() {
                i += 1;
            }
            while j < y.len() && y[j].is_ascii_digit() {
                j += 1;
            }
            let (left, right) = (
                a[start_i..i].trim_start_matches('0'),
                b[start_j..j].trim_start_matches('0'),
            );
            let ordering = left.len().cmp(&right.len()).then_with(|| left.cmp(right));
            if ordering != Ordering::Equal {
                return ordering;
            }
        } else {
            let ordering = x[i].cmp(&y[j]);
            if ordering != Ordering::Equal {
                return ordering;
            }
            i += 1;
            j += 1;
        }
    }
    (x.len() - i).cmp(&(y.len() - j))
}
fn size_text(size: u64, options: &ExportOptions) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024. && unit < units.len() - 1 {
        value /= 1024.;
        unit += 1;
    }
    if unit == 0 {
        return format!("{size} B");
    }
    let text = format!("{value:.2}");
    let locale = options.locale.as_deref().unwrap_or("en");
    let decimal = num_format::Locale::from_name(locale)
        .or_else(|_| {
            num_format::Locale::from_name(locale.split(['-', '_']).next().unwrap_or(locale))
        })
        .map(|locale| locale.decimal())
        .unwrap_or(".");
    format!("{} {}", text.replace('.', decimal), units[unit])
}
fn time_text(seconds: f64, days: bool) -> String {
    let seconds = seconds.round() as u64;
    let hours = seconds / 3600;
    let rest = format!(
        "{:02}:{:02}:{:02}",
        if days { hours % 24 } else { hours },
        seconds / 60 % 60,
        seconds % 60
    );
    if days && seconds >= 86400 {
        format!("{}d {rest}", seconds / 86400)
    } else {
        rest
    }
}
fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).into()
}
fn file_stem(path: &str) -> String {
    let name = file_name(path);
    name.rfind('.')
        .filter(|index| *index > 0)
        .map(|index| name[..index].to_owned())
        .unwrap_or(name)
}
fn extension(path: &str) -> String {
    let name = file_name(path);
    name.rfind('.')
        .filter(|index| *index > 0)
        .map(|index| name[index + 1..].into())
        .unwrap_or_default()
}
fn folder(path: &str) -> String {
    path.rfind(['/', '\\'])
        .map(|index| path[..=index].into())
        .unwrap_or_default()
}
fn rooted(path: &str) -> bool {
    path.starts_with(['/', '\\']) || path.as_bytes().get(1) == Some(&b':')
}
fn resolve_path(base: &str, path: &str) -> String {
    let joined = if rooted(path) || base.is_empty() || base == "." {
        path.to_owned()
    } else {
        format!("{}/{path}", base.trim_end_matches(['/', '\\']))
    };
    let verbatim = verbatim_path(&joined);
    // Other verbatim namespaces are opaque. Never turn their namespace marker
    // into forward slashes or reinterpret it as an ordinary UNC server/share.
    if joined.starts_with(r"\\?\") && verbatim.is_none() {
        return joined.replace('/', "\\");
    }
    let (prefix, parts) = path_parts(&joined);
    if let Some((unc, _)) = verbatim {
        let root = if unc {
            format!(
                r"\\?\UNC\{}",
                prefix.trim_start_matches('/').replace('/', "\\")
            )
        } else {
            format!(r"\\?\{prefix}")
        };
        format!("{root}\\{}", parts.join("\\"))
    } else if prefix == "/" {
        format!("/{}", parts.join("/"))
    } else if prefix.is_empty() {
        parts.join("/")
    } else {
        // Drive and share paths are Windows paths: keep its separator, so the
        // written files are offered as Windows spells them.
        format!("{}\\{}", prefix.replace('/', "\\"), parts.join("\\"))
    }
}
// Remove only the representation wrapper for comparison. A drive or share is
// the same volume whether spelled normally or with Windows' long-path prefix.
fn verbatim_path(path: &str) -> Option<(bool, &str)> {
    let rest = path.strip_prefix(r"\\?\")?;
    if rest
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC\\"))
    {
        Some((true, &rest[4..]))
    } else if rest.as_bytes().get(1) == Some(&b':') {
        Some((false, rest))
    } else {
        None
    }
}
fn path_parts(path: &str) -> (String, Vec<String>) {
    let normalized = match verbatim_path(path) {
        Some((true, rest)) => format!("//{}", rest.replace('\\', "/")),
        Some((false, rest)) => rest.replace('\\', "/"),
        None => path.replace('\\', "/"),
    };
    let mut parts = normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let prefix = if normalized.starts_with("//") && parts.len() >= 2 {
        format!("//{}/{}", parts.remove(0), parts.remove(0))
    } else if parts.first().is_some_and(|part| part.ends_with(':')) {
        parts.remove(0)
    } else if normalized.starts_with('/') {
        "/".into()
    } else {
        String::new()
    };
    let mut normalized = Vec::new();
    for part in parts {
        if part == ".." {
            if normalized.last().is_some_and(|part| part != "..") {
                normalized.pop();
            } else if prefix.is_empty() {
                normalized.push(part);
            }
        } else {
            normalized.push(part);
        }
    }
    (prefix, normalized)
}
fn relative_path(source: &str, output: &str, directory: bool) -> String {
    let (source_prefix, mut source_parts) = path_parts(source);
    let (base_prefix, mut base_parts) = path_parts(output);
    base_parts.pop();
    let windows = source_prefix.ends_with(':') || source_prefix.starts_with("//");
    if !source_prefix.eq_ignore_ascii_case(&base_prefix) {
        return if directory {
            folder(source)
        } else {
            source.to_owned()
        };
    }
    if directory {
        source_parts.pop();
    }
    let shared = source_parts
        .iter()
        .zip(&base_parts)
        .take_while(|(a, b)| {
            if windows {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        })
        .count();
    let mut parts = vec!["..".to_owned(); base_parts.len() - shared];
    parts.extend_from_slice(&source_parts[shared..]);
    let separator = if windows { "\\" } else { "/" };
    let mut text = parts.join(separator);
    if directory {
        if text.is_empty() {
            text.push('.');
        }
        text.push_str(separator);
    }
    text
}
fn change_extension(filename: &str, extension: &str) -> String {
    let last = filename.rfind(['/', '\\']).map_or(0, |index| index + 1);
    let end = filename[last..]
        .rfind('.')
        .map_or(filename.len(), |index| last + index);
    format!("{}.{}", &filename[..end], extension.trim_start_matches('.'))
}
fn encoding(name: &str) -> Result<String, String> {
    let name = name.trim().to_ascii_lowercase();
    if ["utf-8", "utf-16", "ansi"].contains(&name.as_str()) {
        Ok(name)
    } else {
        Err(format!(
            "Unsupported export encoding {name}; use ansi, utf-8, or utf-16"
        ))
    }
}

/// Validate configuration syntax without evaluating fields or producing output.
pub fn validate_export_template(template: &str) -> Result<(), String> {
    compile(template).map(|_| ())
}

pub fn render_export(
    template: &str,
    rows: &[ExportRow],
    options: &ExportOptions,
) -> Result<Vec<ExportDocument>, String> {
    for row in rows {
        if !row.duration.is_finite() || row.duration < 0. {
            return Err(format!("Invalid export duration for {}", row.path));
        }
    }
    if options.max_output_bytes == 0 || options.max_steps == 0 {
        return Err("Export resource limits must be positive".into());
    }
    let program = compile(template)?;
    let mut options = options.clone();
    if options.timestamp_seconds.is_none() {
        options.timestamp_seconds = Some(chrono::Utc::now().timestamp());
    }
    let all = (0..rows.len()).collect::<Vec<_>>();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    if let Some(partition) = &options.partition {
        let expression = CompiledTemplate::compile(partition)?;
        let engine = Engine::new(rows, all.clone().into(), &options, &program)?;
        let mut locations = BTreeMap::new();
        for index in all {
            engine.state.borrow_mut().frame.row = Some(index);
            let key = engine.eval(&expression)?.text;
            let group = *locations.entry(key).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[group].push(index);
        }
    } else {
        groups.push(all);
    }
    let mut documents = Vec::new();
    let mut filenames = HashSet::new();
    for members in groups {
        let engine = Engine::new(rows, members.into(), &options, &program)?;
        let mut filename = engine
            .eval(&CompiledTemplate::compile(&options.filename)?)?
            .text;
        let mut selected_encoding = encoding(&options.encoding)?;
        if let Some((name, enc)) = &program.filename {
            let name = engine.eval(name)?.text;
            if !name.is_empty() {
                filename = if !name.contains(['.', '/', '\\', ':']) {
                    change_extension(&filename, &name)
                } else if rooted(&name) {
                    name
                } else {
                    resolve_path(&folder(&filename), &name)
                };
            }
            if let Some(enc) = enc {
                let enc = engine.eval(enc)?.text;
                if !enc.is_empty() {
                    selected_encoding = encoding(&enc)?;
                }
            }
        }
        if let Some(overridden) = &options.filename_override {
            let overridden = CompiledTemplate::compile(overridden)?;
            filename = engine.eval(&overridden)?.text;
        }
        if filename.trim().is_empty() || filename.contains('\0') {
            return Err("Export filename is empty or contains NUL".into());
        }
        let filename = resolve_path(&options.base_directory, &filename);
        let collision = if filename.as_bytes().get(1) == Some(&b':') {
            filename.to_lowercase()
        } else {
            filename.clone()
        };
        if !filenames.insert(collision) {
            return Err(format!(
                "Multiple export groups map to the same output file: {filename}"
            ));
        }
        engine.state.borrow_mut().filename = filename.clone();
        let text = engine.eval(&program.body)?.text;
        documents.push(ExportDocument {
            filename,
            encoding: selected_encoding.clone(),
            bom: options.bom && selected_encoding != "ansi",
            text,
        });
    }
    Ok(documents)
}

/// ANSI codepage of systems that have none: Windows-1252, the codepage of
/// Mp3tag's own Western installations.
const FALLBACK_ANSI_CODEPAGE: &str = "1252";

/// Encode a rendered document without opening or modifying a file. For append,
/// the host must suppress BOM when the destination already has content.
pub fn encode_export_document(
    document: &ExportDocument,
    ansi_codepage: Option<&str>,
) -> Result<Vec<u8>, String> {
    match encoding(&document.encoding)?.as_str() {
        "utf-8" => {
            let mut bytes = if document.bom {
                vec![0xef, 0xbb, 0xbf]
            } else {
                Vec::new()
            };
            bytes.extend_from_slice(document.text.as_bytes());
            Ok(bytes)
        }
        "utf-16" => {
            let mut bytes = if document.bom {
                vec![0xff, 0xfe]
            } else {
                Vec::new()
            };
            bytes.extend(document.text.encode_utf16().flat_map(u16::to_le_bytes));
            Ok(bytes)
        }
        "ansi" => {
            // Without a system ANSI codepage (macOS, Linux) write Windows-1252,
            // as playlist reading assumes.
            let codepage = ansi_codepage.unwrap_or(FALLBACK_ANSI_CODEPAGE);
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
                Some(value) => {
                    encoding_rs::Encoding::for_label(format!("windows-{value}").as_bytes())
                }
                None => encoding_rs::Encoding::for_label(codepage.as_bytes()),
            }
            .ok_or_else(|| format!("Unsupported ANSI export codepage {codepage}"))?;
            let mut bytes = Vec::new();
            for character in document.text.chars() {
                let mut buffer = [0u8; 4];
                let (encoded, _, unmappable) = encoding.encode(character.encode_utf8(&mut buffer));
                if unmappable {
                    bytes.push(b'?');
                } else {
                    bytes.extend_from_slice(&encoded);
                }
            }
            Ok(bytes)
        }
        _ => unreachable!(),
    }
}

/// Decode configuration text separately from the configured output encoding.
/// Unicode BOMs are authoritative; non-UTF8 legacy text uses the ANSI codepage
/// (Windows-1252 without one) and malformed sequences are reported rather than
/// replaced.
pub fn decode_export_template(bytes: &[u8], ansi_codepage: Option<&str>) -> Result<String, String> {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string());
    }
    let utf16 = if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        Some((bytes, true))
    } else {
        bytes
            .strip_prefix(&[0xfe, 0xff])
            .map(|bytes| (bytes, false))
    };
    if let Some((bytes, little)) = utf16 {
        let (pairs, trailing) = bytes.as_chunks::<2>();
        if !trailing.is_empty() {
            return Err("Truncated UTF-16 export configuration".into());
        }
        let units = pairs
            .iter()
            .map(|&unit| {
                if little {
                    u16::from_le_bytes(unit)
                } else {
                    u16::from_be_bytes(unit)
                }
            })
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|error| error.to_string());
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.to_owned());
    }
    let codepage = ansi_codepage.unwrap_or(FALLBACK_ANSI_CODEPAGE);
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
        None => encoding_rs::Encoding::for_label(codepage.as_bytes()),
    }
    .ok_or_else(|| format!("Unsupported export configuration codepage {codepage}"))?;
    let (text, malformed) = encoding.decode_without_bom_handling(bytes);
    if malformed {
        return Err(format!(
            "Invalid export configuration for codepage {codepage}"
        ));
    }
    Ok(text.into_owned())
}

/// Static provider requests, including literal $meta/_sep names. Dynamic meta
/// names are still checked by the runtime field resolver and cannot silently
/// fabricate hashes. Names are canonical _CRC, _MD5 and _MD5AUDIO.
pub fn requested_hash_fields(
    template: &str,
    options: &ExportOptions,
) -> Result<Vec<String>, String> {
    fn visit(nodes: &[Node], names: &mut HashSet<String>) {
        for node in nodes {
            match node {
                Node::Field(name) => {
                    let name = name.to_ascii_uppercase();
                    if ["_CRC", "_MD5", "_MD5AUDIO"].contains(&name.as_str()) {
                        names.insert(name);
                    }
                }
                Node::Function(name, args) => {
                    if ["meta", "meta_sep"].contains(&name.as_str()) {
                        if let Some(name) = args.first().and_then(|argument| literal(argument)) {
                            let name = name.to_ascii_uppercase();
                            if ["_CRC", "_MD5", "_MD5AUDIO"].contains(&name.as_str()) {
                                names.insert(name);
                            }
                        }
                    }
                    for argument in args {
                        visit(argument, names);
                    }
                }
                Node::Optional(body) => visit(body, names),
                Node::Loop { sort, limit, body } => {
                    visit(sort, names);
                    if let Some(limit) = limit {
                        visit(limit, names);
                    }
                    visit(body, names);
                }
                Node::Text(_) => {}
            }
        }
    }
    compile(template)?;
    let mut names = HashSet::new();
    for text in std::iter::once(template)
        .chain(std::iter::once(options.filename.as_str()))
        .chain(options.filename_override.as_deref())
        .chain(options.partition.as_deref())
    {
        let mut parser = Parser {
            chars: text.trim_start_matches('\u{feff}').chars().collect(),
            position: 0,
        };
        let (nodes, _) = parser.sequence(&[], false, 0)?;
        visit(&nodes, &mut names);
    }
    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
    Ok(names)
}
