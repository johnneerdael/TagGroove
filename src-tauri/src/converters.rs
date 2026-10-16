//! Reviewed native converter plans. GUI integration is a separate task.
use crate::evaluation::{ColumnSpec, EvaluationSettings};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};
use tag_core::{AudioEntry, Fields, ScanResult, TextImportOptions, TextTagImport};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilenameRequest {
    pub paths: Vec<String>,
    pub old_pattern: String,
    pub new_pattern: String,
    pub columns: Option<Vec<ColumnSpec>>,
}

#[derive(Debug, Serialize)]
pub struct FilenamePreview {
    pub plan_token: String,
    pub items: Vec<FilenameItem>,
    /// Selected files whose name does not match the old pattern. They are
    /// left unchanged, as Filename → Tag leaves them.
    pub unmatched: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FilenameItem {
    pub path: String,
    pub target: String,
    pub captures: Fields,
}

#[derive(Debug, Serialize)]
pub struct FilenameResult {
    #[serde(flatten)]
    pub result: ScanResult,
    pub renamed: Vec<PathChange>,
}

#[derive(Debug, Serialize)]
pub struct PathChange {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextSource {
    Text { text: String },
    File { path: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRequest {
    pub paths: Vec<String>,
    pub source: TextSource,
    pub mask: String,
    pub options: TextImportOptions,
}

#[derive(Debug, Serialize)]
pub struct TextPreview {
    pub plan_token: String,
    pub items: Vec<TextTagImport>,
    /// The text file read, after evaluating a format-string source.
    pub source_path: Option<String>,
    /// Selected files that no record names (keyed records only). They are
    /// left unchanged.
    pub unmatched: Vec<String>,
}

/// Rollback diagnostics retain their paths and recovery provenance for the UI.
/// Other failures keep their original string IPC representation.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ConverterError {
    Plain(String),
    Rollback {
        converter_rollback: Box<RollbackFailure>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackOperation {
    Rename,
    TextImport,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum ConverterMessage {
    Message(String),
    External(String),
}
impl ConverterMessage {
    fn text(&self) -> &str {
        match self {
            Self::Message(text) | Self::External(text) => text,
        }
    }
}
impl From<String> for ConverterMessage {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}
impl From<&str> for ConverterMessage {
    fn from(message: &str) -> Self {
        Self::Message(message.into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RollbackRecovery {
    Retained { path: String },
    Failed { error: String },
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackItem {
    path: String,
    error: ConverterMessage,
    recovery: Option<RollbackRecovery>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackFailure {
    operation: RollbackOperation,
    error: ConverterMessage,
    failures: Vec<RollbackItem>,
}
impl From<String> for ConverterError {
    fn from(message: String) -> Self {
        Self::Plain(message)
    }
}
impl From<&str> for ConverterError {
    fn from(message: &str) -> Self {
        Self::Plain(message.into())
    }
}
impl std::fmt::Display for ConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(message) => write!(f, "{message}"),
            Self::Rollback {
                converter_rollback: failure,
            } => {
                let operation = match failure.operation {
                    RollbackOperation::Rename => "rename",
                    RollbackOperation::TextImport => "text import",
                };
                write!(f, "{}; {operation} rollback ", failure.error.text())?;
                if failure.failures.is_empty() {
                    return write!(f, "completed");
                }
                for (index, item) in failure.failures.iter().enumerate() {
                    if index > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{}: {}", item.path, item.error.text())?;
                    match &item.recovery {
                        Some(RollbackRecovery::Retained { path }) => {
                            write!(f, "; original backup retained at {path}")?
                        }
                        Some(RollbackRecovery::Failed { error }) => {
                            write!(f, "; could not retain recovery copy: {error}")?
                        }
                        Some(RollbackRecovery::Unavailable) => {
                            write!(f, "; no recovery copy is available")?
                        }
                        None => {}
                    }
                }
                Ok(())
            }
        }
    }
}
/// The journal still observes the same English String error and result-derived changes.
/// Only after it returns do the two converter commands restore the structured IPC error.
fn record_conversion<T: super::history::Changes>(
    label: &str,
    paths: &[String],
    work: impl FnOnce() -> Result<T, ConverterError>,
) -> Result<T, ConverterError> {
    let mut failure = None;
    let result = super::history::record(label, None, paths, || {
        capture_conversion_error(work(), &mut failure)
    });
    result.map_err(|error| failure.unwrap_or(ConverterError::Plain(error)))
}

fn capture_conversion_error<T>(
    result: Result<T, ConverterError>,
    failure: &mut Option<ConverterError>,
) -> Result<T, String> {
    result.map_err(|error| {
        let text = error.to_string();
        *failure = Some(error);
        text
    })
}

#[tauri::command]
pub async fn preview_filename_conversion(
    request: FilenameRequest,
) -> Result<FilenamePreview, String> {
    let settings = super::evaluation_settings(request.columns.clone())?;
    super::io(move || {
        super::reject_provider_operation(&request.paths, "Filename conversion")?;
        preview_filename(&request, &settings)
    })
    .await
}

#[tauri::command]
pub async fn apply_filename_conversion(
    request: FilenameRequest,
    plan_token: String,
) -> Result<FilenameResult, ConverterError> {
    super::io(move || {
        super::reject_provider_operation(&request.paths, "Filename conversion")?;
        Ok(record_conversion(
            "Filename → Filename",
            &request.paths,
            || apply_filename(&request, &plan_token),
        ))
    })
    .await
    .map_err(ConverterError::Plain)?
}

#[tauri::command]
pub async fn preview_text_import(request: TextRequest) -> Result<TextPreview, String> {
    super::io(move || {
        reject_provider_text(&request)?;
        preview_text(&request)
    })
    .await
}

#[tauri::command]
pub async fn apply_text_import(
    request: TextRequest,
    plan_token: String,
) -> Result<ScanResult, ConverterError> {
    super::io(move || {
        reject_provider_text(&request)?;
        Ok(record_conversion(
            "Text File → Tag",
            &request.paths,
            || apply_text(&request, &plan_token),
        ))
    })
    .await
    .map_err(ConverterError::Plain)?
}

fn reject_provider_text(request: &TextRequest) -> Result<(), String> {
    super::reject_provider_operation(&request.paths, "Text import")?;
    if let TextSource::File { path } = &request.source {
        super::reject_provider_operation(std::slice::from_ref(path), "Text import source")?;
    }
    Ok(())
}

static PLANS: Mutex<VecDeque<ReviewedPlan>> = Mutex::new(VecDeque::new());
const MAX_TEXT: u64 = 256 * 1024 * 1024;

#[derive(Clone)]
struct Snapshot {
    requested: PathBuf,
    canonical: PathBuf,
    metadata: fs::Metadata,
    hash: [u8; 32],
    writable: bool,
}

fn same_metadata(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if a.dev() != b.dev() || a.ino() != b.ino() {
            return false;
        }
    }
    a.len() == b.len()
        && a.modified().ok() == b.modified().ok()
        && a.created().ok() == b.created().ok()
        && a.permissions() == b.permissions()
}

fn digest_file(path: &Path) -> Result<([u8; 32], fs::Metadata), String> {
    digest_message(path).map_err(|error| error.text().to_owned())
}
fn digest_message(path: &Path) -> Result<([u8; 32], fs::Metadata), ConverterMessage> {
    let mut file = fs::File::open(path).map_err(|e| ConverterMessage::External(e.to_string()))?;
    let before = file
        .metadata()
        .map_err(|e| ConverterMessage::External(e.to_string()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut read = 0u64;
    loop {
        let length = file
            .read(&mut buffer)
            .map_err(|e| ConverterMessage::External(e.to_string()))?;
        if length == 0 {
            break;
        }
        read = read
            .checked_add(length as u64)
            .ok_or("Source size overflow")?;
        if read > before.len() {
            return Err("File changed while its contents were reviewed".into());
        }
        hash.update(&buffer[..length]);
    }
    if read != before.len()
        || !same_metadata(
            &before,
            &file
                .metadata()
                .map_err(|e| ConverterMessage::External(e.to_string()))?,
        )
        || !same_metadata(
            &before,
            &fs::metadata(path).map_err(|e| ConverterMessage::External(e.to_string()))?,
        )
    {
        return Err("File changed while its contents were reviewed".into());
    }
    Ok((hash.finalize().into(), before))
}

impl Snapshot {
    fn read(path: &Path, writable: bool) -> Result<Self, String> {
        if !fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err("Converter inputs must be regular files, not symbolic links".into());
        }
        let canonical = dunce::canonicalize(path).map_err(|e| e.to_string())?;
        let (hash, metadata) = digest_file(&canonical)?;
        if writable && metadata.permissions().readonly() {
            return Err("The audio file is read-only".into());
        }
        Ok(Self {
            requested: path.to_owned(),
            canonical,
            hash,
            metadata,
            writable,
        })
    }
    fn validate(&self) -> Result<(), String> {
        if dunce::canonicalize(&self.requested).map_err(|e| e.to_string())? != self.canonical
            || !same_metadata(
                &fs::metadata(&self.canonical).map_err(|e| e.to_string())?,
                &self.metadata,
            )
        {
            return Err(format!(
                "{} changed after preview; preview again",
                self.requested.display()
            ));
        }
        let current = Self::read(&self.requested, self.writable)?;
        if current.canonical != self.canonical
            || current.hash != self.hash
            || !same_metadata(&current.metadata, &self.metadata)
        {
            return Err(format!(
                "{} changed after preview; preview again",
                self.requested.display()
            ));
        }
        Ok(())
    }
}

struct Move {
    /// Position of the file in the request's selection.
    selection_index: usize,
    raw_target: PathBuf,
    target: PathBuf,
    ancestor: PathBuf,
    ancestor_metadata: fs::Metadata,
}
enum Operation {
    Filename(Vec<Move>),
    Text(Vec<TextTagImport>),
}
struct ReviewedPlan {
    token: String,
    request_hash: [u8; 32],
    created: Instant,
    sources: Vec<Snapshot>,
    text_source: Option<Snapshot>,
    operation: Operation,
}

fn request_hash(kind: &str, request: &impl Serialize) -> Result<[u8; 32], String> {
    let mut hash = Sha256::new();
    hash.update(kind.as_bytes());
    hash.update([0]);
    serde_json::to_writer(&mut hash, request).map_err(|e| e.to_string())?;
    Ok(hash.finalize().into())
}

fn remember(mut plan: ReviewedPlan) -> Result<String, String> {
    // Reuse the core's OS-random UUID implementation; the token has no client
    // paths or values and cannot authorize a different request or operation.
    plan.token = tag_core::evaluate("$uuid()", &Fields::new())?;
    let token = plan.token.clone();
    let mut plans = PLANS.lock().map_err(|_| "Converter plan lock failed")?;
    plans.retain(|plan| plan.created.elapsed().as_secs() < 300);
    while plans.len() >= 32 {
        plans.pop_front();
    }
    plans.push_back(plan);
    Ok(token)
}

fn consume(token: &str, hash: [u8; 32]) -> Result<ReviewedPlan, String> {
    let plan = {
        let mut plans = PLANS.lock().map_err(|_| "Converter plan lock failed")?;
        let index = plans
            .iter()
            .position(|p| p.token == token)
            .ok_or("Converter preview is no longer available; preview again")?;
        plans.remove(index).ok_or("Converter preview disappeared")?
    };
    if plan.created.elapsed().as_secs() >= 300 || plan.request_hash != hash {
        return Err("Converter request changed; preview again".into());
    }
    validate_sources(&plan)?;
    Ok(plan)
}

fn validate_sources(plan: &ReviewedPlan) -> Result<(), String> {
    for source in &plan.sources {
        source.validate()?;
    }
    if let Some(source) = &plan.text_source {
        source.validate()?;
    }
    Ok(())
}

fn path_key(path: &Path) -> String {
    let path = path.to_string_lossy().into_owned();
    // Conservative duplicate protection on the usual case-insensitive hosts.
    if cfg!(any(windows, target_os = "macos")) {
        path.to_lowercase()
    } else {
        path
    }
}

fn read_sources(paths: &[String], writable: bool) -> Result<Vec<(Snapshot, AudioEntry)>, String> {
    if paths.is_empty() {
        return Err("Select at least one file".into());
    }
    let mut seen = HashSet::new();
    paths
        .iter()
        .map(|path| {
            let source = Snapshot::read(Path::new(path), writable)?;
            if !seen.insert(path_key(&source.canonical)) {
                return Err("A selected file occurs more than once".into());
            }
            let entry = crate::settings::read_summary(&source.canonical)?;
            source.validate()?;
            Ok((source, entry))
        })
        .collect()
}

fn validate_moves(sources: &[Snapshot], moves: &[Move]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for (source, movement) in sources.iter().zip(moves) {
        let current = crate::action_host::normalized_rename_target(&movement.raw_target)?;
        if current != movement.target {
            return Err("A destination directory changed after preview; preview again".into());
        }
        let ancestor = fs::metadata(&movement.ancestor)
            .map_err(|_| "A reviewed destination directory is missing; preview again")?;
        if !ancestor.is_dir() || !same_metadata(&ancestor, &movement.ancestor_metadata) {
            return Err("A reviewed destination directory changed; preview again".into());
        }
        if !seen.insert(path_key(&current)) {
            return Err("More than one selected file maps to the same destination".into());
        }
        match fs::symlink_metadata(&current) {
            Ok(_)
                if current != source.canonical
                    && !tag_core::operations::is_case_alias(&source.canonical, &current) =>
            {
                return Err(format!("Destination already exists: {}", current.display()))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Cannot inspect destination {}: {error}",
                    current.display()
                ))
            }
        }
    }
    for movement in moves {
        if movement
            .target
            .ancestors()
            .skip(1)
            .any(|parent| seen.contains(&path_key(parent)))
        {
            return Err("A planned file is also another destination's parent directory".into());
        }
    }
    Ok(())
}

fn destination_ancestor(target: &Path) -> Result<(PathBuf, fs::Metadata), String> {
    let mut path = target
        .parent()
        .ok_or("Destination has no parent directory")?;
    loop {
        match fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => return Ok((path.to_owned(), metadata)),
            Ok(_) => return Err("Destination parent is not a directory".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                path = path
                    .parent()
                    .ok_or("Destination has no existing ancestor")?;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn preview_filename(
    request: &FilenameRequest,
    settings: &EvaluationSettings,
) -> Result<FilenamePreview, String> {
    let converter =
        tag_core::FilenameConverter::compile(&request.old_pattern, &request.new_pattern)?;
    let loaded = read_sources(&request.paths, false)?;
    let mut sources = Vec::new();
    let mut moves = Vec::new();
    let mut items = Vec::new();
    let mut unmatched = Vec::new();
    for (index, (source, entry)) in loaded.into_iter().enumerate() {
        let fields = crate::evaluation::enriched_fields(&entry, index + 1);
        let columns =
            settings.column_values(&fields, &entry.fields, &request.paths[index], true)?;
        // The shared context: working folder, folder separator and clock as
        // in Tag → Filename and actions.
        let context = settings.context(&fields, &entry.fields, Some(&columns));
        let Some(rendered) = converter.try_render(&entry.path, &context)? else {
            unmatched.push(request.paths[index].clone());
            continue;
        };
        let raw_target = super::filename_target(&entry, rendered.target)?;
        let target = crate::action_host::normalized_rename_target(&raw_target)?;
        let (ancestor, ancestor_metadata) = destination_ancestor(&target)?;
        items.push(FilenameItem {
            path: request.paths[index].clone(),
            target: target.to_string_lossy().into_owned(),
            captures: rendered.captures,
        });
        moves.push(Move {
            selection_index: index,
            raw_target,
            target,
            ancestor,
            ancestor_metadata,
        });
        sources.push(source);
    }
    validate_moves(&sources, &moves)?;
    let plan = ReviewedPlan {
        token: String::new(),
        request_hash: request_hash("filename", request)?,
        created: Instant::now(),
        sources,
        text_source: None,
        operation: Operation::Filename(moves),
    };
    validate_sources(&plan)?;
    Ok(FilenamePreview {
        plan_token: remember(plan)?,
        items,
        unmatched,
    })
}

fn apply_filename(
    request: &FilenameRequest,
    token: &str,
) -> Result<FilenameResult, ConverterError> {
    let plan = consume(token, request_hash("filename", request)?)?;
    let Operation::Filename(moves) = &plan.operation else {
        return Err("Wrong converter preview kind".into());
    };
    validate_moves(&plan.sources, moves)?;
    let mut completed: Vec<usize> = Vec::new();
    let mut result = FilenameResult {
        result: ScanResult::default(),
        renamed: Vec::new(),
    };
    let operation = (|| {
        for (index, (source, movement)) in plan.sources.iter().zip(moves).enumerate() {
            source.validate()?;
            if source.canonical != movement.target {
                tag_core::operations::rename_no_clobber(&source.canonical, &movement.target)?;
                completed.push(index);
                result.renamed.push(PathChange {
                    from: request.paths[movement.selection_index].clone(),
                    to: movement.target.to_string_lossy().into_owned(),
                });
            }
            result
                .result
                .entries
                .push(crate::settings::read_summary(&movement.target)?);
        }
        Ok::<_, ConverterMessage>(())
    })();
    if let Err(error) = operation {
        let mut rollback = Vec::new();
        for index in completed.into_iter().rev() {
            let source = &plan.sources[index];
            let target = &moves[index].target;
            let restored = (|| {
                if digest_message(target)?.0 != source.hash {
                    return Err("Moved file changed externally; retained at destination".into());
                }
                tag_core::operations::rename_no_clobber(target, &source.canonical)
                    .map_err(ConverterMessage::External)
            })();
            if let Err(error) = restored {
                rollback.push(RollbackItem {
                    path: target.to_string_lossy().into_owned(),
                    error,
                    recovery: None,
                });
            }
        }
        return Err(ConverterError::Rollback {
            converter_rollback: Box::new(RollbackFailure {
                operation: RollbackOperation::Rename,
                error,
                failures: rollback,
            }),
        });
    }
    Ok(result)
}

/// The text file of Convert → Text File → Tag. The source may be a format
/// string such as `%_folderpath%tracks.txt`, evaluated for the first selected
/// file; a relative result is relative to that file's folder. An existing
/// file whose name merely contains `%` or `$` is used as named.
fn text_source_path(template: &str, paths: &[String]) -> Result<PathBuf, String> {
    let literal = Path::new(template);
    if !template.contains(['%', '$']) || literal.is_file() {
        return Ok(literal.to_path_buf());
    }
    let first = paths.first().ok_or("Select at least one audio file")?;
    let entry = crate::settings::read_summary(Path::new(first))?;
    let settings = super::evaluation_settings(None)?;
    let technical = crate::evaluation::technical_fields(&entry, 1);
    let value = settings.evaluate(template, &entry.fields, &technical, &entry.path, false)?;
    let value = value.trim();
    if value.is_empty() {
        return Err("The source file name is empty for the first selected file".into());
    }
    let path = Path::new(value);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(first)
            .parent()
            .unwrap_or(Path::new("."))
            .join(path)
    })
}

fn preview_text(request: &TextRequest) -> Result<TextPreview, String> {
    let converter = tag_core::TextTagConverter::compile(&request.mask)?;
    let sources = read_sources(&request.paths, true)?
        .into_iter()
        .map(|(source, _)| source)
        .collect::<Vec<_>>();
    let (items, text_source) = match &request.source {
        TextSource::Text { text } => (
            converter.plan(text, &request.paths, &request.options)?,
            None,
        ),
        TextSource::File { path } => {
            let path = &text_source_path(path, &request.paths)?;
            let shown = path.to_string_lossy().into_owned();
            super::reject_provider_operation(std::slice::from_ref(&shown), "Text import source")?;
            if fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_TEXT {
                return Err("Text import source exceeds the input limit".into());
            }
            let source = Snapshot::read(Path::new(path), false)?;
            if sources
                .iter()
                .any(|selected| path_key(&selected.canonical) == path_key(&source.canonical))
            {
                return Err("Text source cannot be a selected audio file".into());
            }
            let mut bytes = Vec::new();
            fs::File::open(&source.canonical)
                .map_err(|e| e.to_string())?
                .take(MAX_TEXT + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_TEXT {
                return Err("Text import source exceeds the input limit".into());
            }
            (
                converter.plan_bytes(&bytes, &request.paths, &request.options)?,
                Some(source),
            )
        }
    };
    let plan = ReviewedPlan {
        token: String::new(),
        request_hash: request_hash("text", request)?,
        created: Instant::now(),
        sources,
        text_source,
        operation: Operation::Text(items.clone()),
    };
    validate_sources(&plan)?;
    let source_path = plan
        .text_source
        .as_ref()
        .map(|source| source.requested.to_string_lossy().into_owned());
    let planned: HashSet<usize> = items.iter().map(|item| item.selection_index).collect();
    let unmatched = request
        .paths
        .iter()
        .enumerate()
        .filter(|(index, _)| !planned.contains(index))
        .map(|(_, path)| path.clone())
        .collect();
    Ok(TextPreview {
        plan_token: remember(plan)?,
        items,
        source_path,
        unmatched,
    })
}

struct PreparedEdit {
    source: Snapshot,
    staged: Option<tempfile::TempPath>,
    backup: Option<tempfile::TempPath>,
    after_hash: [u8; 32],
    committed_metadata: Option<fs::Metadata>,
}

fn sibling_copy(source: &Snapshot, backup: bool) -> Result<tempfile::TempPath, String> {
    let suffix = source
        .canonical
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    let temporary = tag_core::staging_copy(
        &source.canonical,
        source.canonical.parent().ok_or("Source has no parent")?,
        ".opentag-converter-",
        &suffix,
    )?
    .into_temp_path();
    if digest_file(&temporary)?.0 != source.hash {
        return Err("Source changed while converter staging was prepared".into());
    }
    if backup {
        let mut times = fs::FileTimes::new();
        if let Ok(time) = source.metadata.modified() {
            times = times.set_modified(time);
        }
        if let Ok(time) = source.metadata.accessed() {
            times = times.set_accessed(time);
        }
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?
            .set_times(times)
            .map_err(|e| e.to_string())?;
    }
    Ok(temporary)
}

fn apply_text(request: &TextRequest, token: &str) -> Result<ScanResult, ConverterError> {
    apply_text_with(request, token, crate::settings::write_summary, |_| Ok(()))
}

fn apply_text_with(
    request: &TextRequest,
    token: &str,
    mut writer: impl FnMut(&Path, &Fields) -> Result<AudioEntry, String>,
    mut before_commit: impl FnMut(usize) -> Result<(), String>,
) -> Result<ScanResult, ConverterError> {
    let plan = consume(token, request_hash("text", request)?)?;
    let Operation::Text(items) = &plan.operation else {
        return Err("Wrong converter preview kind".into());
    };
    let mut prepared = Vec::new();
    for item in items {
        let source = &plan.sources[item.selection_index];
        let staged = sibling_copy(source, false)?;
        let backup = sibling_copy(source, true)?;
        writer(&staged, &item.fields)?;
        fs::set_permissions(&staged, source.metadata.permissions()).map_err(|e| e.to_string())?;
        let after_hash = digest_file(&staged)?.0;
        prepared.push(PreparedEdit {
            source: source.clone(),
            staged: Some(staged),
            backup: Some(backup),
            after_hash,
            committed_metadata: None,
        });
    }
    // Parsing, format validation and every real tag write have succeeded on
    // private copies. Recheck the complete selection before the first commit.
    validate_sources(&plan)?;
    let mut completed = Vec::new();
    let mut result = ScanResult::default();
    let operation = (|| {
        for (index, item) in prepared.iter_mut().enumerate() {
            before_commit(index)?;
            item.source.validate()?;
            item.staged
                .take()
                .ok_or("Missing prepared edit")?
                .persist(&item.source.canonical)
                .map_err(|e| ConverterMessage::External(e.error.to_string()))?;
            completed.push(index);
            let committed = fs::symlink_metadata(&item.source.canonical)
                .map_err(|e| ConverterMessage::External(e.to_string()))?;
            if !committed.is_file() {
                return Err("Committed file path changed externally".into());
            }
            item.committed_metadata = Some(committed);
            result
                .entries
                .push(crate::settings::read_summary(&item.source.canonical)?);
        }
        Ok::<_, ConverterMessage>(())
    })();
    if let Err(error) = operation {
        let mut failures = Vec::new();
        for index in completed.into_iter().rev() {
            let item = &mut prepared[index];
            let restore = (|| {
                let committed = item
                    .committed_metadata
                    .as_ref()
                    .ok_or("Committed file identity unavailable; rollback did not overwrite it")?;
                let current = fs::symlink_metadata(&item.source.canonical)
                    .map_err(|e| ConverterMessage::External(e.to_string()))?;
                if !current.is_file() || !same_metadata(&current, committed) {
                    return Err(
                        "File changed externally after commit; rollback did not overwrite it"
                            .into(),
                    );
                }
                let (current_hash, stable_metadata) = digest_message(&item.source.canonical)?;
                if current_hash != item.after_hash || !same_metadata(&stable_metadata, committed) {
                    return Err(
                        "File changed externally after commit; rollback did not overwrite it"
                            .into(),
                    );
                }
                match item
                    .backup
                    .take()
                    .ok_or("Missing rollback copy")?
                    .persist(&item.source.canonical)
                {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        let message = error.error.to_string();
                        item.backup = Some(error.path);
                        Err(ConverterMessage::External(message))
                    }
                }
            })();
            if let Err(error) = restore {
                let recovery = match item.backup.take() {
                    Some(backup) => match backup.keep() {
                        Ok(path) => RollbackRecovery::Retained {
                            path: path.to_string_lossy().into_owned(),
                        },
                        Err(error) => RollbackRecovery::Failed {
                            error: error.error.to_string(),
                        },
                    },
                    None => RollbackRecovery::Unavailable,
                };
                failures.push(RollbackItem {
                    path: item.source.canonical.to_string_lossy().into_owned(),
                    error,
                    recovery: Some(recovery),
                });
            }
        }
        return Err(ConverterError::Rollback {
            converter_rollback: Box::new(RollbackFailure {
                operation: RollbackOperation::TextImport,
                error,
                failures,
            }),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    fn wav(path: &Path, title: &str) {
        let samples = vec![0u8; 8820];
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36 + samples.len() as u32).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(44100u32.to_le_bytes());
        bytes.extend(88200u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((samples.len() as u32).to_le_bytes());
        bytes.extend(samples);
        fs::write(path, bytes).unwrap();
        tag_core::write_fields_summary(
            path,
            &[
                ("TITLE".into(), vec![title.into()]),
                ("ARTIST".into(), vec!["Keep artist".into()]),
            ]
            .into(),
        )
        .unwrap();
    }
    fn filename_request(paths: &[PathBuf], new_pattern: &str) -> FilenameRequest {
        FilenameRequest {
            paths: paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            old_pattern: "%1 - %2".into(),
            new_pattern: new_pattern.into(),
            columns: None,
        }
    }
    fn text_request(paths: &[PathBuf], text: &str) -> TextRequest {
        TextRequest {
            paths: paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            source: TextSource::Text { text: text.into() },
            mask: "%title%".into(),
            options: TextImportOptions::default(),
        }
    }

    #[test]
    fn rollback_ipc_and_history_bridge_keep_paths_parts_and_plain_errors() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/converter-rollback.json"))
                .unwrap();
        for case in cases.as_array().unwrap() {
            let failure: RollbackFailure = serde_json::from_value(case["payload"].clone()).unwrap();
            let error = ConverterError::Rollback {
                converter_rollback: Box::new(failure),
            };
            assert_eq!(error.to_string(), case["source"].as_str().unwrap());
            let wire: tauri::ipc::InvokeError = error.clone().into();
            assert_eq!(
                wire.0,
                serde_json::json!({"converter_rollback":case["payload"]})
            );
            let mut captured = None;
            let journal_result: Result<ScanResult, String> =
                capture_conversion_error(Err(error.clone()), &mut captured);
            assert_eq!(
                journal_result.unwrap_err(),
                case["source"].as_str().unwrap()
            );
            assert_eq!(serde_json::to_value(captured.unwrap()).unwrap(), wire.0);
            let recorded =
                record_conversion::<ScanResult>("converter diagnostic", &[], || Err(error));
            assert_eq!(serde_json::to_value(recorded.unwrap_err()).unwrap(), wire.0);
        }
        let ordinary = ConverterError::from("Select files first.");
        let wire: tauri::ipc::InvokeError = ordinary.clone().into();
        assert_eq!(wire.0, "Select files first.");
        assert_eq!(
            record_conversion::<ScanResult>("converter ordinary", &[], || Err(ordinary))
                .unwrap_err()
                .to_string(),
            "Select files first."
        );
    }

    #[test]
    fn converter_outer_io_guards_keep_plain_string_ipc_errors() {
        let paths = vec!["content://provider/Help".into()];
        let filename = FilenameRequest {
            paths: paths.clone(),
            old_pattern: "%1".into(),
            new_pattern: "%1".into(),
            columns: None,
        };
        let text = TextRequest {
            paths,
            source: TextSource::Text {
                text: "Help".into(),
            },
            mask: "%title%".into(),
            options: TextImportOptions::default(),
        };
        let errors = [
            tauri::async_runtime::block_on(apply_filename_conversion(filename, "unused".into()))
                .unwrap_err(),
            tauri::async_runtime::block_on(apply_text_import(text, "unused".into())).unwrap_err(),
        ];
        for error in errors {
            assert!(matches!(error, ConverterError::Plain(_)));
            let shown = error.to_string();
            let wire: tauri::ipc::InvokeError = error.into();
            assert_eq!(wire.0, shown);
            assert!(shown.contains("provider"));
        }
    }

    #[test]
    fn actual_multi_file_rollback_preserves_external_edits_and_serializes_recovery_paths() {
        let dir = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        let names = ["Help: one; Title.wav", "Title: two; Help.wav", "third.wav"];
        #[cfg(not(unix))]
        let names = ["Help; one.wav", "Title; two.wav", "third.wav"];
        let paths = names.map(|name| dir.path().join(name));
        for (index, path) in paths.iter().enumerate() {
            wav(path, &format!("Original {index}"));
        }
        let originals = paths
            .iter()
            .map(|path| fs::read(path).unwrap())
            .collect::<Vec<_>>();
        let request = text_request(&paths, "New one\nNew two\nNew three");
        let preview = preview_text(&request).unwrap();
        let error = apply_text_with(
            &request,
            &preview.plan_token,
            tag_core::write_fields_summary,
            |index| {
                if index == 2 {
                    fs::write(&paths[0], b"External Help one").unwrap();
                    fs::write(&paths[1], b"External Title two").unwrap();
                    Err("Select files first.".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        let ConverterError::Rollback {
            converter_rollback: failure,
        } = &error
        else {
            panic!("expected structured rollback")
        };
        assert_eq!(failure.failures.len(), 2);
        for (item, index) in failure.failures.iter().zip([1, 0]) {
            assert_eq!(
                Path::new(&item.path),
                dunce::canonicalize(&paths[index]).unwrap()
            );
            assert_eq!(
                item.error.text(),
                "File changed externally after commit; rollback did not overwrite it"
            );
            let Some(RollbackRecovery::Retained { path }) = &item.recovery else {
                panic!("expected retained backup")
            };
            assert_eq!(fs::read(path).unwrap(), originals[index]);
        }
        assert_eq!(fs::read(&paths[0]).unwrap(), b"External Help one");
        assert_eq!(fs::read(&paths[1]).unwrap(), b"External Title two");
        assert_eq!(fs::read(&paths[2]).unwrap(), originals[2]);
        let wire: tauri::ipc::InvokeError = error.clone().into();
        assert_eq!(wire.0, serde_json::to_value(error).unwrap());
    }

    #[test]
    fn reviewed_random_filename_is_exact_single_use_and_preserves_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Artist - Song.wav");
        wav(&path, "Original");
        let before = fs::read(&path).unwrap();
        let request = filename_request(std::slice::from_ref(&path), "Album\\%2-$uuid()");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        let target = PathBuf::from(&preview.items[0].target);
        assert_eq!(target.parent().unwrap().file_name().unwrap(), "Album");
        assert!(!target.exists());
        let result = apply_filename(&request, &preview.plan_token).unwrap();
        assert_eq!(result.renamed[0].to, preview.items[0].target);
        assert_eq!(fs::read(target).unwrap(), before);
        assert!(!path.exists());
        assert!(apply_filename(&request, &preview.plan_token).is_err());
    }

    #[test]
    fn non_matching_file_is_left_unchanged_and_matching_files_rename_together() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        let odd = dir.path().join("no separator.wav");
        let b = dir.path().join("B - Two.wav");
        wav(&a, "One");
        wav(&odd, "Odd");
        wav(&b, "Two");
        let before = fs::read(&odd).unwrap();
        let request = filename_request(&[a.clone(), odd.clone(), b.clone()], "%2 - %1");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        assert_eq!(preview.items.len(), 2);
        assert_eq!(preview.items[0].path, request.paths[0]);
        assert_eq!(preview.items[1].path, request.paths[2]);
        assert_eq!(preview.unmatched, [request.paths[1].clone()]);
        let result = apply_filename(&request, &preview.plan_token).unwrap();
        assert_eq!(result.renamed.len(), 2);
        assert_eq!(result.renamed[1].from, request.paths[2]);
        assert!(dir.path().join("One - A.wav").exists());
        assert!(dir.path().join("Two - B.wav").exists());
        assert_eq!(fs::read(&odd).unwrap(), before);
        // Nothing matches: a preview with no planned rename.
        let request = filename_request(std::slice::from_ref(&odd), "%2 - %1");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        assert!(preview.items.is_empty());
        assert_eq!(preview.unmatched.len(), 1);
        assert!(apply_filename(&request, &preview.plan_token)
            .unwrap()
            .renamed
            .is_empty());
        assert!(odd.exists());
    }

    #[test]
    fn new_filename_pattern_uses_the_working_folder_and_folder_separator() {
        let dir = tempfile::tempdir().unwrap();
        let working = dir.path().join("Working");
        fs::create_dir(&working).unwrap();
        let a = working.join("AC - DC.wav");
        wav(&a, "Title/Part");
        let request = filename_request(std::slice::from_ref(&a), "%_workingdir% %2 %title%");
        let settings = EvaluationSettings {
            working_directory: Some(working.to_string_lossy().into_owned()),
            folder_separator: "+".into(),
            ..Default::default()
        };
        let preview = preview_filename(&request, &settings).unwrap();
        assert!(
            preview.items[0]
                .target
                .ends_with("Working DC Title+Part.wav"),
            "{}",
            preview.items[0].target
        );
    }

    #[test]
    fn case_only_rearrangement_renames_without_doubling_the_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artist - song.wav");
        wav(&path, "Song");
        wav(&dir.path().join("Keep - Me.wav"), "Keep");
        let request = filename_request(std::slice::from_ref(&path), "$upper(%1) - %2.WAV");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        assert!(
            preview.items[0].target.ends_with("ARTIST - song.WAV"),
            "{}",
            preview.items[0].target
        );
        apply_filename(&request, &preview.plan_token).unwrap();
        let mut names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["ARTIST - song.WAV", "Keep - Me.wav"]);
    }

    #[test]
    fn changed_request_or_same_size_restored_mtime_source_rejects_the_whole_rename() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        let b = dir.path().join("B - Two.wav");
        wav(&a, "One");
        wav(&b, "Two");
        let request = filename_request(&[a.clone(), b.clone()], "new-%2");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        let mut edited = request.clone();
        edited.new_pattern = "different-%2".into();
        assert!(apply_filename(&edited, &preview.plan_token).is_err());
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        let modified = fs::metadata(&b).unwrap().modified().unwrap();
        let mut bytes = fs::read(&b).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(&b, &bytes).unwrap();
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&b)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
        assert!(apply_filename(&request, &preview.plan_token).is_err());
        assert!(a.exists() && b.exists());
        assert!(preview
            .items
            .iter()
            .all(|item| !Path::new(&item.target).exists()));
    }

    #[test]
    fn collision_or_missing_source_after_preview_rejects_before_any_move() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        let b = dir.path().join("B - Two.wav");
        wav(&a, "One");
        wav(&b, "Two");
        let request = filename_request(&[a.clone(), b.clone()], "new-%2");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        fs::write(&preview.items[1].target, b"occupied").unwrap();
        assert!(apply_filename(&request, &preview.plan_token).is_err());
        assert!(a.exists() && b.exists());
        assert!(!Path::new(&preview.items[0].target).exists());
        fs::remove_file(&preview.items[1].target).unwrap();
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        fs::remove_file(&b).unwrap();
        assert!(apply_filename(&request, &preview.plan_token).is_err());
        assert!(a.exists());
        assert!(!Path::new(&preview.items[0].target).exists());
    }

    #[test]
    fn duplicate_sources_and_destinations_and_unknown_tokens_are_not_bypasses() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        let b = dir.path().join("B - Two.wav");
        wav(&a, "One");
        wav(&b, "Two");
        assert!(preview_filename(
            &filename_request(&[a.clone(), a.clone()], "%2"),
            &EvaluationSettings::default()
        )
        .is_err());
        assert!(preview_filename(
            &filename_request(&[a.clone(), b.clone()], "same"),
            &EvaluationSettings::default()
        )
        .is_err());
        assert!(apply_filename(&filename_request(&[a], "%2"), "guessed-token").is_err());
        let mut json = serde_json::to_value(filename_request(&[b], "%2")).unwrap();
        json["destinations"] = serde_json::json!(["arbitrary.wav"]);
        assert!(serde_json::from_value::<FilenameRequest>(json).is_err());
    }

    #[test]
    fn text_preview_imports_bom_keyed_rows_without_losing_unmentioned_tags() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "Original A");
        wav(&b, "Original B");
        let source = dir.path().join("import.txt");
        let text = "\u{feff}b.wav|Café 猫\r\na.wav|Second\r\n";
        fs::write(
            &source,
            text.encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let mut request = text_request(&[a.clone(), b.clone()], "");
        request.source = TextSource::File {
            path: source.to_string_lossy().into_owned(),
        };
        request.mask = "%_filename_ext%|%title%".into();
        let preview = preview_text(&request).unwrap();
        assert_eq!(preview.items[0].selection_index, 1);
        let before = fs::read(&a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), before);
        let result = apply_text(&request, &preview.plan_token).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(
            tag_core::read_entry_summary(&a).unwrap().fields["TITLE"],
            ["Second"]
        );
        let entry = tag_core::read_entry_summary(&b).unwrap();
        assert_eq!(entry.fields["TITLE"], ["Café 猫"]);
        assert_eq!(entry.fields["ARTIST"], ["Keep artist"]);
        assert!(apply_text(&request, &preview.plan_token).is_err());
    }

    #[test]
    fn keyed_text_list_longer_than_the_selection_imports_only_the_named_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        let c = dir.path().join("c.wav");
        wav(&a, "Original A");
        wav(&b, "Original B");
        wav(&c, "Original C");
        let before_b = fs::read(&b).unwrap();
        let mut request = text_request(
            &[a.clone(), b.clone(), c.clone()],
            "z.wav|Elsewhere\nc.wav|Third\nx.wav|Other\na.wav|First\n",
        );
        request.mask = "%_filename_ext%|%title%".into();
        let preview = preview_text(&request).unwrap();
        assert_eq!(preview.items.len(), 2);
        assert_eq!(preview.unmatched, [request.paths[1].clone()]);
        let result = apply_text(&request, &preview.plan_token).unwrap();
        assert!(result.errors.is_empty());
        let title =
            |path: &Path| tag_core::read_entry_summary(path).unwrap().fields["TITLE"].clone();
        assert_eq!(title(&a), ["First"]);
        assert_eq!(title(&c), ["Third"]);
        assert_eq!(fs::read(&b).unwrap(), before_b);
        // A list naming none of the selected files plans nothing.
        request.source = TextSource::Text {
            text: "x.wav|Other\ny.wav|Other".into(),
        };
        assert!(preview_text(&request).is_err());
    }

    #[test]
    fn text_source_can_be_a_format_string_for_the_first_selected_file() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("Album");
        fs::create_dir_all(&album).unwrap();
        let a = album.join("a.wav");
        let b = album.join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        fs::write(album.join("Album tracks.txt"), "One\nTwo\n").unwrap();
        let mut request = text_request(&[a.clone(), b.clone()], "");
        request.mask = "%title%".into();
        for template in [
            "%_folderpath%%_directory% tracks.txt".to_string(),
            "$left(%_directory%,5) tracks.txt".to_string(),
        ] {
            request.source = TextSource::File { path: template };
            let preview = preview_text(&request).unwrap();
            assert_eq!(
                dunce::canonicalize(preview.source_path.unwrap()).unwrap(),
                dunce::canonicalize(album.join("Album tracks.txt")).unwrap()
            );
            assert_eq!(preview.items[1].fields["TITLE"], ["Two"]);
        }
        let result = apply_text(&request, &preview_text(&request).unwrap().plan_token).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(
            tag_core::read_entry_summary(&b).unwrap().fields["TITLE"],
            ["Two"]
        );
        // A name that evaluates to nothing, or to a missing file, is refused.
        request.source = TextSource::File {
            path: "[%comment%]".into(),
        };
        assert!(preview_text(&request).is_err());
        request.source = TextSource::File {
            path: "%_filename%.txt".into(),
        };
        assert!(preview_text(&request).is_err());
        // An existing file named with a percent sign is read as named.
        let literal = album.join("100% hits.txt");
        fs::write(&literal, "Three\nFour\n").unwrap();
        request.source = TextSource::File {
            path: literal.to_string_lossy().into_owned(),
        };
        assert_eq!(
            preview_text(&request).unwrap().items[0].fields["TITLE"],
            ["Three"]
        );
    }

    #[test]
    fn malformed_text_and_changed_source_or_options_never_write_selected_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let before_a = fs::read(&a).unwrap();
        let before_b = fs::read(&b).unwrap();
        let mut request = text_request(&[a.clone(), b.clone()], "A|One\nbad");
        request.mask = "%artist%|%title%".into();
        assert!(preview_text(&request).is_err());
        let source = dir.path().join("tags.txt");
        fs::write(&source, "One\nTwo").unwrap();
        request = text_request(&[a.clone(), b.clone()], "");
        request.source = TextSource::File {
            path: source.to_string_lossy().into_owned(),
        };
        let preview = preview_text(&request).unwrap();
        fs::write(&source, "New\nTwo").unwrap();
        assert!(apply_text(&request, &preview.plan_token).is_err());
        let preview = preview_text(&request).unwrap();
        request.options.case_sensitive_paths = false;
        assert!(apply_text(&request, &preview.plan_token).is_err());
        assert_eq!(fs::read(a).unwrap(), before_a);
        assert_eq!(fs::read(b).unwrap(), before_b);
    }

    #[test]
    fn second_prepare_failure_never_commits_the_first_real_staged_tag_write() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let before = [fs::read(&a).unwrap(), fs::read(&b).unwrap()];
        let request = text_request(&[a.clone(), b.clone()], "New A\nNew B");
        let preview = preview_text(&request).unwrap();
        let mut calls = 0;
        let error = apply_text_with(
            &request,
            &preview.plan_token,
            |path, fields| {
                calls += 1;
                if calls == 2 {
                    return Err("Injected prepare failure".into());
                }
                tag_core::write_fields_summary(path, fields)
            },
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(error.to_string().contains("prepare failure"));
        assert_eq!(calls, 2);
        assert_eq!(fs::read(&a).unwrap(), before[0]);
        assert_eq!(fs::read(&b).unwrap(), before[1]);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn handled_second_commit_failure_restores_prior_bytes_permissions_and_modified_time() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let before = [fs::read(&a).unwrap(), fs::read(&b).unwrap()];
        let metadata = fs::metadata(&a).unwrap();
        let request = text_request(&[a.clone(), b.clone()], "New A\nNew B");
        let preview = preview_text(&request).unwrap();
        let error = apply_text_with(
            &request,
            &preview.plan_token,
            tag_core::write_fields_summary,
            |index| {
                if index == 1 {
                    Err("Injected commit failure".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("commit failure")
                && error.to_string().contains("rollback completed"),
            "{error}"
        );
        let ConverterError::Rollback {
            converter_rollback: failure,
        } = &error
        else {
            panic!("expected structured completed rollback")
        };
        assert!(failure.failures.is_empty());
        let wire: tauri::ipc::InvokeError = error.clone().into();
        assert_eq!(wire.0, serde_json::to_value(&error).unwrap());
        assert_eq!(fs::read(&a).unwrap(), before[0]);
        assert_eq!(fs::read(&b).unwrap(), before[1]);
        assert_eq!(
            fs::metadata(&a).unwrap().permissions(),
            metadata.permissions()
        );
        assert_eq!(
            fs::metadata(&a).unwrap().modified().unwrap(),
            metadata.modified().unwrap()
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn externally_edited_selected_metadata_and_changed_inline_text_are_stale() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let request = text_request(&[a.clone(), b.clone()], "New A\nNew B");
        let preview = preview_text(&request).unwrap();
        tag_core::write_fields_summary(&b, &[("TITLE".into(), vec!["External".into()])].into())
            .unwrap();
        let before_a = fs::read(&a).unwrap();
        let external = fs::read(&b).unwrap();
        assert!(apply_text(&request, &preview.plan_token).is_err());
        assert_eq!(fs::read(&a).unwrap(), before_a);
        assert_eq!(fs::read(&b).unwrap(), external);
        let preview = preview_text(&request).unwrap();
        let mut changed = request.clone();
        changed.source = TextSource::Text {
            text: "Different A\nDifferent B".into(),
        };
        assert!(apply_text(&changed, &preview.plan_token).is_err());
        assert_eq!(fs::read(&a).unwrap(), before_a);
        assert_eq!(fs::read(&b).unwrap(), external);
    }

    #[test]
    fn tokens_cannot_cross_converter_kinds_or_reuse_legacy_rename_receipts() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        wav(&a, "Original");
        let original = fs::read(&a).unwrap();
        let request = filename_request(std::slice::from_ref(&a), "%2");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        assert!(apply_text(
            &text_request(std::slice::from_ref(&a), "Forged"),
            &preview.plan_token
        )
        .is_err());
        let legacy = crate::rename_plan::record(
            &request.paths,
            "%2",
            vec![dir.path().join("arbitrary.wav")],
        )
        .unwrap();
        assert!(apply_filename(&request, &legacy).is_err());
        assert_eq!(fs::read(&a).unwrap(), original);
        assert!(!dir.path().join("arbitrary.wav").exists());
    }

    #[test]
    fn missing_reviewed_destination_directory_is_stale_before_any_move() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        let b = dir.path().join("B - Two.wav");
        wav(&a, "A");
        wav(&b, "B");
        let output = dir.path().join("Reviewed");
        fs::create_dir(&output).unwrap();
        let request = filename_request(&[a.clone(), b.clone()], "Reviewed\\%2");
        let preview = preview_filename(&request, &EvaluationSettings::default()).unwrap();
        fs::remove_dir(&output).unwrap();
        assert!(apply_filename(&request, &preview.plan_token).is_err());
        assert!(a.exists() && b.exists());
        assert!(!output.exists());
    }

    #[test]
    fn rollback_does_not_overwrite_external_edits_and_retains_original_backup_for_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let original = fs::read(&a).unwrap();
        let request = text_request(&[a.clone(), b.clone()], "New A\nNew B");
        let preview = preview_text(&request).unwrap();
        let error = apply_text_with(
            &request,
            &preview.plan_token,
            tag_core::write_fields_summary,
            |index| {
                if index == 1 {
                    fs::write(&a, b"External update must survive").unwrap();
                    Err("Injected later failure".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("externally"), "{error}");
        assert_eq!(fs::read(&a).unwrap(), b"External update must survive");
        let recovery: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|item| item.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".opentag-converter-")
            })
            .collect();
        assert_eq!(recovery.len(), 1, "{error}");
        assert_eq!(fs::read(&recovery[0]).unwrap(), original);
        assert!(error.to_string().contains("backup retained"));
    }

    #[test]
    fn a_planned_file_cannot_also_be_another_planned_destination_directory() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("A - One.wav");
        let b = dir.path().join("B - Two.wav");
        wav(&a, "A");
        wav(&b, "B");
        let request = filename_request(
            &[a.clone(), b.clone()],
            "$if($eql(%1,A),Collision,Collision.wav\\Nested)",
        );
        assert!(preview_filename(&request, &EvaluationSettings::default()).is_err());
        assert!(a.exists() && b.exists());
        assert!(!dir.path().join("Collision.wav").exists());
    }

    #[test]
    fn metadata_only_mtime_change_survives_failed_rollback_with_recovery_copy() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let original = fs::read(&a).unwrap();
        let request = text_request(&[a.clone(), b], "New A\nNew B");
        let preview = preview_text(&request).unwrap();
        let external_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_123);
        let mut committed = Vec::new();
        let error = apply_text_with(
            &request,
            &preview.plan_token,
            tag_core::write_fields_summary,
            |index| {
                if index == 1 {
                    committed = fs::read(&a).unwrap();
                    fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&a)
                        .unwrap()
                        .set_times(fs::FileTimes::new().set_modified(external_time))
                        .unwrap();
                    Err("Injected failure after external timestamp edit".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(
            fs::metadata(&a).unwrap().modified().unwrap(),
            external_time,
            "{error}"
        );
        assert_eq!(fs::read(&a).unwrap(), committed);
        let recovery: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|item| item.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".opentag-converter-")
            })
            .collect();
        assert_eq!(recovery.len(), 1, "{error}");
        assert_eq!(fs::read(&recovery[0]).unwrap(), original);
        assert!(error.to_string().contains("backup retained"));
    }

    #[cfg(unix)]
    #[test]
    fn metadata_only_permission_change_survives_failed_rollback_with_recovery_copy() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a, "A");
        wav(&b, "B");
        let original = fs::read(&a).unwrap();
        let request = text_request(&[a.clone(), b], "New A\nNew B");
        let preview = preview_text(&request).unwrap();
        let mut committed = Vec::new();
        let error = apply_text_with(
            &request,
            &preview.plan_token,
            tag_core::write_fields_summary,
            |index| {
                if index == 1 {
                    committed = fs::read(&a).unwrap();
                    fs::set_permissions(&a, fs::Permissions::from_mode(0o400)).unwrap();
                    Err("Injected failure after external permission edit".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(
            fs::metadata(&a).unwrap().permissions().mode() & 0o777,
            0o400,
            "{error}"
        );
        assert_eq!(fs::read(&a).unwrap(), committed);
        let recovery: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|item| item.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".opentag-converter-")
            })
            .collect();
        assert_eq!(recovery.len(), 1, "{error}");
        assert_eq!(fs::read(&recovery[0]).unwrap(), original);
        assert!(error.to_string().contains("backup retained"));
    }
}
