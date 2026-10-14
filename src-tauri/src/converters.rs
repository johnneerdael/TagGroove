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
use tag_core::{AudioEntry, EvalContext, Fields, ScanResult, TextImportOptions, TextTagImport};

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
) -> Result<FilenameResult, String> {
    super::io(move || {
        super::reject_provider_operation(&request.paths, "Filename conversion")?;
        apply_filename(&request, &plan_token)
    })
    .await
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
) -> Result<ScanResult, String> {
    super::io(move || {
        reject_provider_text(&request)?;
        apply_text(&request, &plan_token)
    })
    .await
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
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut read = 0u64;
    loop {
        let length = file.read(&mut buffer).map_err(|e| e.to_string())?;
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
        || !same_metadata(&before, &file.metadata().map_err(|e| e.to_string())?)
        || !same_metadata(&before, &fs::metadata(path).map_err(|e| e.to_string())?)
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
        let canonical = path.canonicalize().map_err(|e| e.to_string())?;
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
        if self.requested.canonicalize().map_err(|e| e.to_string())? != self.canonical
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
            let entry = tag_core::read_entry_summary(&source.canonical)?;
            source.validate()?;
            Ok((source, entry))
        })
        .collect()
}

fn validate_moves(sources: &[Snapshot], moves: &[Move]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for (source, movement) in sources.iter().zip(moves) {
        let current = crate::action_host::normalized_output_path(&movement.raw_target)?;
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
            Ok(_) if current != source.canonical => {
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
    for (index, (source, entry)) in loaded.into_iter().enumerate() {
        let fields = crate::evaluation::enriched_fields(&entry, index + 1);
        let columns =
            settings.column_values(&fields, &entry.fields, &request.paths[index], true)?;
        let mut context = EvalContext::new(&fields);
        context.raw_fields = Some(&entry.fields);
        context.columns = Some(&columns);
        context.app_directory = Some(&settings.app_directory);
        context.app_data_directory = Some(&settings.app_data_directory);
        context.ansi_codepage = settings.ansi_codepage().or(context.ansi_codepage);
        context.timestamp_seconds = Some(settings.timestamp_seconds);
        let rendered = converter.render(&entry.path, &context)?;
        let raw_target = super::filename_target(&entry, rendered.target)?;
        let target = crate::action_host::normalized_output_path(&raw_target)?;
        let (ancestor, ancestor_metadata) = destination_ancestor(&target)?;
        items.push(FilenameItem {
            path: request.paths[index].clone(),
            target: target.to_string_lossy().into_owned(),
            captures: rendered.captures,
        });
        moves.push(Move {
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
    })
}

fn apply_filename(request: &FilenameRequest, token: &str) -> Result<FilenameResult, String> {
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
                    from: request.paths[index].clone(),
                    to: movement.target.to_string_lossy().into_owned(),
                });
            }
            result
                .result
                .entries
                .push(tag_core::read_entry_summary(&movement.target)?);
        }
        Ok::<_, String>(())
    })();
    if let Err(error) = operation {
        let mut rollback = Vec::new();
        for index in completed.into_iter().rev() {
            let source = &plan.sources[index];
            let target = &moves[index].target;
            let restored = (|| {
                if digest_file(target)?.0 != source.hash {
                    return Err("Moved file changed externally; retained at destination".into());
                }
                tag_core::operations::rename_no_clobber(target, &source.canonical)
            })();
            if let Err(error) = restored {
                rollback.push(format!("{}: {error}", target.display()));
            }
        }
        return Err(format!(
            "{error}; rename rollback {}",
            if rollback.is_empty() {
                "completed".into()
            } else {
                rollback.join("; ")
            }
        ));
    }
    Ok(result)
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
    Ok(TextPreview {
        plan_token: remember(plan)?,
        items,
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
    let temporary = tempfile::Builder::new()
        .prefix(".opentag-converter-")
        .suffix(&suffix)
        .tempfile_in(source.canonical.parent().ok_or("Source has no parent")?)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    fs::copy(&source.canonical, &temporary).map_err(|e| e.to_string())?;
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

fn apply_text(request: &TextRequest, token: &str) -> Result<ScanResult, String> {
    apply_text_with(request, token, tag_core::write_fields_summary, |_| Ok(()))
}

fn apply_text_with(
    request: &TextRequest,
    token: &str,
    mut writer: impl FnMut(&Path, &Fields) -> Result<AudioEntry, String>,
    mut before_commit: impl FnMut(usize) -> Result<(), String>,
) -> Result<ScanResult, String> {
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
                .map_err(|e| e.error.to_string())?;
            completed.push(index);
            let committed =
                fs::symlink_metadata(&item.source.canonical).map_err(|e| e.to_string())?;
            if !committed.is_file() {
                return Err("Committed file path changed externally".into());
            }
            item.committed_metadata = Some(committed);
            result
                .entries
                .push(tag_core::read_entry_summary(&item.source.canonical)?);
        }
        Ok::<_, String>(())
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
                let current =
                    fs::symlink_metadata(&item.source.canonical).map_err(|e| e.to_string())?;
                if !current.is_file() || !same_metadata(&current, committed) {
                    return Err(
                        "File changed externally after commit; rollback did not overwrite it"
                            .into(),
                    );
                }
                let (current_hash, stable_metadata) = digest_file(&item.source.canonical)?;
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
                        Err(message)
                    }
                }
            })();
            if let Err(error) = restore {
                let recovery = match item.backup.take() {
                    Some(backup) => match backup.keep() {
                        Ok(path) => format!("original backup retained at {}", path.display()),
                        Err(error) => format!("could not retain recovery copy: {}", error.error),
                    },
                    None => "no recovery copy is available".into(),
                };
                failures.push(format!(
                    "{}: {error}; {recovery}",
                    item.source.canonical.display()
                ));
            }
        }
        return Err(format!(
            "{error}; text import rollback {}",
            if failures.is_empty() {
                "completed".into()
            } else {
                failures.join("; ")
            }
        ));
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
        assert!(error.contains("prepare failure"));
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
            error.contains("commit failure") && error.contains("rollback completed"),
            "{error}"
        );
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
        assert!(error.contains("externally"), "{error}");
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
        assert!(error.contains("backup retained"));
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
        assert!(error.contains("backup retained"));
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
        assert!(error.contains("backup retained"));
    }
}
