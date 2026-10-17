//! Explicit, source-bound setup import. Audio and credential files are never written.
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const LIMIT: u64 = 128 * 1024 * 1024;
const ENTRIES: usize = 4096;
const DEPTH: usize = 64;
const STORE: &str = ".profile-migration";
const NATIVE: [&str; 7] = [
    "settings.json",
    "actions.json",
    "exports.json",
    "columns.json",
    "tag-panel.json",
    "directories.json",
    "lists.json",
];
type Files = BTreeMap<String, Vec<u8>>;
type Encoded = BTreeMap<String, String>;

pub const PREFERENCE_KEYS: &[&str] = &[
    "opentag.language",
    "opentag.appearance",
    "opentag.main-layout",
    "opentag.panel-width",
    "opentag.filter-auto-apply",
    "opentag.filter-shown",
    "opentag.numbering",
    "opentag.actions.quick",
    "opentag.cover.aspect",
    "opentag.cover.extractFormat",
    "opentag.tagSources.lastSource",
    "opentag.tagSources.last",
    "opentag.tagSources.saveCover",
    "opentag.tagSources.suggestSorting",
    "opentag.last-version",
    "opentag.tagSources.disabled.musicbrainz",
    "opentag.tagSources.disabled.discogs",
    "opentag.tagSources.disabled.itunes",
    "opentag.tagSources.disabled.deezer",
    "opentag.tagSources.disabled.lastfm",
    "opentag.tagSources.disabled.fanart",
    "opentag.tagSources.disabled.lrclib",
    "opentag.tagSources.disabled.acoustid",
    "opentag.tagSources.disabled.freedb",
    "opentag.tagSources.disabled.local_freedb",
    "opentag.tagSources.disabled.web",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub paths: Vec<String>,
    pub selected: Vec<String>,
    pub roots: Vec<String>,
    pub location: String,
    pub location_is_data: bool,
    pub filter: String,
    pub filter_draft: Option<String>,
    pub anchor: Option<String>,
    pub playlist: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WindowState {
    pub version: u32,
    pub preferences: BTreeMap<String, String>,
    pub workspace: Workspace,
}

fn validate_window(window: &WindowState) -> Result<(), String> {
    if window.version != 1
        || window
            .preferences
            .keys()
            .any(|key| !PREFERENCE_KEYS.contains(&key.as_str()))
    {
        return Err("Unsupported setup window-state version or preference key.".into());
    }
    let workspace = &window.workspace;
    if workspace.paths.len() > 10000
        || workspace.roots.len() > 10000
        || workspace.selected.len() > workspace.paths.len()
        || workspace.paths.iter().collect::<BTreeSet<_>>().len() != workspace.paths.len()
        || workspace
            .selected
            .iter()
            .any(|path| !workspace.paths.contains(path))
        || workspace
            .anchor
            .as_ref()
            .is_some_and(|path| !workspace.paths.contains(path))
    {
        return Err("The setup workspace has invalid file identities.".into());
    }
    for path in workspace
        .paths
        .iter()
        .chain(&workspace.roots)
        .chain(workspace.playlist.iter())
    {
        if path.is_empty()
            || path.contains(['\0', '\n', '\r'])
            || path.contains("://")
            || !Path::new(path).is_absolute()
        {
            return Err("Setup workspace recovery requires absolute local file paths.".into());
        }
    }
    if serde_json::to_vec(window).map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024 {
        return Err("The setup window state exceeds 2 MB.".into());
    }
    Ok(())
}

fn relative(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.split('/').count() > DEPTH
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("Unsafe setup path: {name}"));
    }
    Ok(())
}
fn managed(name: &str) -> bool {
    NATIVE.contains(&name) || name.starts_with("export/")
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!(
            "{} must be a bounded regular file without symbolic links.",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("The setup file exceeded its read limit.".into());
    }
    Ok(bytes)
}
fn bounded(files: &Files) -> Result<(), String> {
    if files.len() > ENTRIES || files.values().map(|bytes| bytes.len() as u64).sum::<u64>() > LIMIT
    {
        return Err("Setup files exceed 128 MB or 4096 entries.".into());
    }
    Ok(())
}
fn walk(
    root: &Path,
    prefix: &str,
    depth: usize,
    files: &mut Files,
    count: &mut usize,
) -> Result<(), String> {
    if depth > DEPTH {
        return Err("Setup folders exceed 64 levels.".into());
    }
    for entry in std::fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        *count += 1;
        if *count > ENTRIES {
            return Err("Setup files exceed 128 MB or 4096 entries.".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Setup file names must be Unicode.")?;
        let name = format!("{prefix}{name}");
        relative(&name)?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            walk(&entry.path(), &format!("{name}/"), depth + 1, files, count)?;
        } else {
            files.insert(name, read(&entry.path(), LIMIT)?);
            bounded(files)?;
        }
    }
    Ok(())
}
fn input(path: &Path) -> Result<Files, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let mut files = Files::new();
    if metadata.is_dir() {
        walk(path, "", 0, &mut files, &mut 0)?;
    } else {
        let bytes = read(path, LIMIT)?;
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|e| format!("Choose a setup folder or ZIP: {e}"))?;
        if zip.len() > ENTRIES {
            return Err("Setup files exceed 128 MB or 4096 entries.".into());
        }
        let mut seen = BTreeSet::new();
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let name = entry.name().replace('\\', "/");
            let name = name.trim_end_matches('/');
            relative(name)?;
            if !seen.insert(name.to_lowercase()) {
                return Err(format!("Duplicate setup ZIP path: {name}"));
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(format!("Setup ZIP contains a symbolic link: {name}"));
            }
            if entry.is_dir() {
                continue;
            }
            if entry.size() > LIMIT {
                return Err("Setup files exceed 128 MB or 4096 entries.".into());
            }
            let mut bytes = Vec::new();
            (&mut entry)
                .take(LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            files.insert(name.into(), bytes);
            bounded(&files)?;
        }
    }
    let mut names = BTreeSet::new();
    for name in files.keys() {
        if !names.insert(name.to_lowercase()) {
            return Err(format!("Duplicate setup path ignoring case: {name}"));
        }
    }
    if files.is_empty() {
        return Err("The setup contains no files.".into());
    }
    Ok(files)
}
fn native(directory: &Path) -> Result<Files, String> {
    let mut files = Files::new();
    for name in NATIVE {
        match std::fs::symlink_metadata(directory.join(name)) {
            Ok(_) => {
                files.insert(name.into(), read(&directory.join(name), LIMIT)?);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let export = directory.join("export");
    match std::fs::symlink_metadata(&export) {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("The export folder must be a real directory.".into());
            }
            walk(&export, "export/", 0, &mut files, &mut 0)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    bounded(&files)?;
    Ok(files)
}
fn hash(files: &Files, window: Option<&WindowState>) -> Result<String, String> {
    let mut digest = Sha256::new();
    for (name, bytes) in files {
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    if let Some(window) = window {
        digest.update(serde_json::to_vec(window).map_err(|e| e.to_string())?);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn encode(files: &Files) -> Encoded {
    files
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                base64::engine::general_purpose::STANDARD.encode(bytes),
            )
        })
        .collect()
}
fn decode(files: &Encoded) -> Result<Files, String> {
    let mut decoded = Files::new();
    for (name, encoded) in files {
        relative(name)?;
        if !managed(name) {
            return Err(format!("The setup snapshot cannot write {name}."));
        }
        decoded.insert(
            name.clone(),
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| e.to_string())?,
        );
        bounded(&decoded)?;
    }
    Ok(decoded)
}
fn write_stage(root: &Path, files: &Files) -> Result<(), String> {
    for (name, bytes) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Clone)]
enum Candidate {
    Columns(crate::columns::Layout),
    Panel(crate::tag_panel::Layout),
    Actions(tag_core::actions::ActionGroup),
    Export(crate::exports::Configuration),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub file: String,
    pub kind: String,
    pub name: String,
    pub warnings: Vec<String>,
    pub collision: bool,
    pub supported: bool,
}
#[derive(Clone)]
struct Plan {
    path: String,
    source_hash: String,
    target_hash: String,
    items: Vec<Item>,
    candidates: BTreeMap<String, Candidate>,
    window: WindowState,
}
#[derive(Serialize)]
pub struct Preview {
    pub token: String,
    pub items: Vec<Item>,
    pub source_hash: String,
}
static PLANS: OnceLock<Mutex<BTreeMap<String, Plan>>> = OnceLock::new();

fn inspect(
    directory: &Path,
    source: &Files,
) -> Result<(Vec<Item>, BTreeMap<String, Candidate>), String> {
    let extracted = tempfile::tempdir().map_err(|e| e.to_string())?;
    write_stage(extracted.path(), source)?;
    let actions = crate::action_host::load_library(&directory.join("actions.json"))?;
    let exports = crate::exports::load(&directory.join("exports.json"))?;
    let mut items = Vec::new();
    let mut candidates = BTreeMap::new();
    for (file, bytes) in source {
        let lower = file.to_lowercase();
        let name = Path::new(file)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut item = Item {
            file: file.clone(),
            kind: "unsupported".into(),
            name,
            warnings: Vec::new(),
            collision: false,
            supported: false,
        };
        let result: Result<Option<Candidate>, String> = (|| {
            if lower.rsplit('/').next() == Some("columns.ini")
                || (lower.contains("/columns/") && lower.ends_with(".ini"))
            {
                item.kind = "columns".into();
                item.collision = directory.join("columns.json").exists();
                let parsed =
                    crate::mp3tag_columns::parse_diagnostic(&crate::mp3tag_columns::decode(bytes))
                        .map_err(|e| e.to_string())?;
                item.warnings = parsed.result.skipped;
                Ok(Some(Candidate::Columns(parsed.result.layout)))
            } else if lower.rsplit('/').next() == Some("usrfields.ini")
                || (lower.contains("/panels/") && lower.ends_with(".ini"))
            {
                item.kind = "panel".into();
                item.collision = directory.join("tag-panel.json").exists();
                let parsed = crate::mp3tag_tag_panel::parse_diagnostic(
                    &crate::mp3tag_columns::decode(bytes),
                )
                .map_err(|e| e.to_string())?;
                item.warnings = parsed.skipped;
                Ok(Some(Candidate::Panel(parsed.panel)))
            } else if lower.ends_with(".mta") {
                item.kind = "actions".into();
                let mut parsed =
                    crate::action_host::import_group_diagnostic(&extracted.path().join(file), None)
                        .map_err(|e| e.to_string())?;
                item.warnings = parsed
                    .result
                    .issues
                    .iter()
                    .map(|issue| format!("Action {}: {}", issue.index + 1, issue.reason))
                    .collect();
                parsed.result.group.enabled = false;
                item.collision = actions
                    .groups
                    .iter()
                    .any(|group| group.name.eq_ignore_ascii_case(&parsed.result.group.name));
                Ok(Some(Candidate::Actions(parsed.result.group)))
            } else if lower.ends_with(".mte") {
                item.kind = "export".into();
                let parsed = crate::exports::import(&extracted.path().join(file), None)?;
                item.collision = exports
                    .configurations
                    .iter()
                    .any(|configuration| configuration.name.eq_ignore_ascii_case(&parsed.name));
                Ok(Some(Candidate::Export(parsed)))
            } else {
                item.warnings.push(
                    if lower.ends_with(".src")
                        || lower.ends_with(".inc")
                        || lower.ends_with(".settings")
                    {
                        "Install complete source collections separately with Install tag sources."
                    } else {
                        "No supported setup mapping; this file is left untouched."
                    }
                    .into(),
                );
                Ok(None)
            }
        })();
        match result {
            Ok(Some(candidate)) => {
                item.supported = true;
                candidates.insert(file.clone(), candidate);
            }
            Ok(None) => {}
            Err(error) => {
                item.warnings.push(error);
            }
        }
        items.push(item);
    }
    Ok((items, candidates))
}

fn preview(directory: &Path, path: &str, window: WindowState) -> Result<Preview, String> {
    validate_window(&window)?;
    let _ = status(directory, None, None, None)?;
    let files = input(Path::new(path))?;
    let target = native(directory)?;
    let (items, candidates) = inspect(directory, &files)?;
    let source_hash = hash(&files, None)?;
    let token = uuid::Uuid::new_v4().to_string();
    let plan = Plan {
        path: path.into(),
        source_hash: source_hash.clone(),
        target_hash: hash(&target, Some(&window))?,
        items: items.clone(),
        candidates,
        window,
    };
    let mut plans = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?;
    plans.clear();
    plans.insert(token.clone(), plan);
    Ok(Preview {
        token,
        items,
        source_hash,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub items: Vec<Item>,
    pub imported: Vec<String>,
    pub kept: Vec<String>,
    pub source_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    id: String,
    attempted: bool,
    window: WindowState,
}
#[derive(Clone, Serialize, Deserialize)]
struct Journal {
    version: u32,
    phase: String,
    before: Encoded,
    after: Encoded,
    window: WindowState,
    report: Report,
    pending: Option<Pending>,
    rollback: Option<Encoded>,
    rollback_window: Option<WindowState>,
    previous: bool,
}
#[derive(Serialize)]
pub struct Status {
    pub report: Option<Report>,
    pub restorable: bool,
    pub pending: Option<Pending>,
    pub preference_keys: Vec<String>,
}

fn store(directory: &Path) -> Result<std::path::PathBuf, String> {
    let path = directory.join(STORE);
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("The setup recovery location must be a real directory.".into());
        }
    } else {
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(path)
}
fn persist(directory: &Path, journal: &Journal) -> Result<(), String> {
    crate::action_host::atomic_output(
        &store(directory)?.join("current.json"),
        &serde_json::to_vec(journal).map_err(|e| e.to_string())?,
        false,
    )
}
fn journal(directory: &Path) -> Result<Option<Journal>, String> {
    let path = store(directory)?.join("current.json");
    if !path.exists() {
        return Ok(None);
    }
    let journal: Journal = serde_json::from_slice(&read(&path, LIMIT * 5)?)
        .map_err(|e| format!("Setup recovery journal: {e}"))?;
    if journal.version != 1 {
        return Err("Unsupported setup recovery journal version.".into());
    }
    decode(&journal.before)?;
    decode(&journal.after)?;
    validate_window(&journal.window)?;
    if let Some(pending) = &journal.pending {
        validate_window(&pending.window)?;
    }
    if let Some(rollback) = &journal.rollback {
        decode(rollback)?;
    }
    if let Some(window) = &journal.rollback_window {
        validate_window(window)?;
    }
    Ok(Some(journal))
}
fn current_file(directory: &Path, name: &str) -> Result<Option<Vec<u8>>, String> {
    relative(name)?;
    let mut path = directory.to_path_buf();
    let parts = name.split('/').collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("Setup path is a symbolic link: {name}"))
            }
            Ok(metadata) if index + 1 < parts.len() && !metadata.is_dir() => {
                return Err(format!("Setup path has a non-directory parent: {name}"))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        }
    }
    read(&path, LIMIT).map(Some)
}
fn replace(directory: &Path, expected: &Files, desired: &Files) -> Result<(), String> {
    replace_with(directory, expected, desired, |_| Ok(()))
}
fn replace_with(
    directory: &Path,
    expected: &Files,
    desired: &Files,
    mut before_write: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    if native(directory)? != *expected {
        return Err(
            "Setup changed while its replacement was prepared; nothing was overwritten.".into(),
        );
    }
    let names: BTreeSet<_> = expected.keys().chain(desired.keys()).cloned().collect();
    for name in names {
        let path = directory.join(&name);
        // Recheck each path immediately before mutation, including any export parents.
        let current = current_file(directory, &name)?;
        if current.as_ref() != expected.get(&name) {
            return Err(format!("Setup changed during replacement: {name}"));
        }
        if desired.get(&name) != expected.get(&name) {
            before_write(&name)?;
        }
        match desired.get(&name) {
            Some(bytes) if expected.get(&name) != Some(bytes) => {
                crate::action_host::atomic_output(&path, bytes, false)?
            }
            None if expected.contains_key(&name) => {
                std::fs::remove_file(path).map_err(|e| e.to_string())?
            }
            _ => {}
        }
    }
    if native(directory)? != *desired {
        return Err("Setup changed before replacement completed.".into());
    }
    Ok(())
}
fn rollback(directory: &Path, before: &Files, desired: &Files) -> Result<(), String> {
    let current = native(directory)?;
    for name in current.keys().chain(before.keys()).chain(desired.keys()) {
        if current.get(name) != before.get(name) && current.get(name) != desired.get(name) {
            return Err(format!("Recovery retained its snapshot because {name} changed outside the setup operation."));
        }
    }
    replace(directory, &current, before)
}
fn recovered(directory: &Path, mut state: Journal) -> Result<Journal, String> {
    if state.phase == "applying" || state.phase == "restoring" {
        let before = decode(
            state
                .rollback
                .as_ref()
                .ok_or("Setup recovery has no rollback snapshot.")?,
        )?;
        let desired = if state.phase == "applying" {
            decode(&state.after)?
        } else {
            decode(&state.before)?
        };
        rollback(directory, &before, &desired).map_err(|error| {
            format!(
                "{error} Recovery snapshot retained in {}.",
                directory.join(STORE).display()
            )
        })?;
        if state.phase == "applying" && state.previous {
            let previous = read(&store(directory)?.join("previous.json"), LIMIT * 5)?;
            crate::action_host::atomic_output(
                &store(directory)?.join("current.json"),
                &previous,
                false,
            )?;
            let mut previous = journal(directory)?.ok_or("Previous setup report disappeared.")?;
            previous.pending = Some(Pending {
                id: uuid::Uuid::new_v4().to_string(),
                attempted: false,
                window: state
                    .rollback_window
                    .take()
                    .ok_or("Setup recovery has no window snapshot.")?,
            });
            persist(directory, &previous)?;
            return Ok(previous);
        }
        state.pending = Some(Pending {
            id: uuid::Uuid::new_v4().to_string(),
            attempted: false,
            window: state
                .rollback_window
                .take()
                .ok_or("Setup recovery has no window snapshot.")?,
        });
        state.phase = if state.phase == "restoring" {
            "applied"
        } else {
            "failed"
        }
        .into();
        state.rollback = None;
        persist(directory, &state)?;
    }
    Ok(state)
}
fn status(
    directory: &Path,
    start: Option<&str>,
    ack: Option<&str>,
    retry: Option<&str>,
) -> Result<Status, String> {
    let state = journal(directory)?
        .map(|state| recovered(directory, state))
        .transpose()?;
    if let Some(mut state) = state {
        if let Some(id) = start {
            let pending = state
                .pending
                .as_mut()
                .filter(|pending| pending.id == id)
                .ok_or("The setup workspace receipt expired.")?;
            pending.attempted = true;
            persist(directory, &state)?;
        }
        if let Some(id) = retry {
            let pending = state
                .pending
                .as_mut()
                .filter(|pending| pending.id == id)
                .ok_or("The setup workspace receipt expired.")?;
            pending.attempted = false;
            persist(directory, &state)?;
        }
        if let Some(id) = ack {
            if state
                .pending
                .as_ref()
                .is_none_or(|pending| pending.id != id)
            {
                return Err("The setup workspace receipt expired.".into());
            }
            state.pending = None;
            persist(directory, &state)?;
        }
        Ok(Status {
            report: Some(state.report),
            restorable: state.phase == "applied",
            pending: state.pending,
            preference_keys: PREFERENCE_KEYS.iter().map(|key| (*key).into()).collect(),
        })
    } else {
        Ok(Status {
            report: None,
            restorable: false,
            pending: None,
            preference_keys: PREFERENCE_KEYS.iter().map(|key| (*key).into()).collect(),
        })
    }
}

fn apply(
    directory: &Path,
    token: &str,
    selected: &[String],
    replace_names: bool,
    window: WindowState,
) -> Result<Status, String> {
    validate_window(&window)?;
    let plan = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?
        .remove(token)
        .ok_or("Setup preview expired; inspect the profile again.")?;
    let _ = status(directory, None, None, None)?;
    let before = native(directory)?;
    if hash(&input(Path::new(&plan.path))?, None)? != plan.source_hash
        || hash(&before, Some(&window))? != plan.target_hash
        || window != plan.window
    {
        return Err(
            "The source profile or current setup changed after preview; inspect it again.".into(),
        );
    }
    let names: BTreeSet<_> = selected.iter().collect();
    if names.is_empty()
        || names.len() != selected.len()
        || names
            .iter()
            .any(|name| !plan.candidates.contains_key(*name))
    {
        return Err("Select supported setup items from the current preview.".into());
    }
    for kind in ["columns", "panel"] {
        if plan
            .items
            .iter()
            .filter(|item| item.kind == kind && names.contains(&item.file))
            .count()
            > 1
        {
            return Err("Select at most one column layout and one tag-panel layout.".into());
        }
    }
    let stage = tempfile::tempdir().map_err(|e| e.to_string())?;
    write_stage(stage.path(), &before)?;
    let mut actions = crate::action_host::load_library(&stage.path().join("actions.json"))?;
    let mut exports = crate::exports::load(&stage.path().join("exports.json"))?;
    let mut report = Report {
        items: plan.items,
        imported: Vec::new(),
        kept: Vec::new(),
        source_hash: plan.source_hash,
    };
    let mut actions_changed = false;
    let mut exports_changed = false;
    for name in selected {
        match &plan.candidates[name] {
            Candidate::Columns(layout) => {
                crate::columns::save(&stage.path().join("columns.json"), layout)?
            }
            Candidate::Panel(layout) => {
                let path = stage.path().join("tag-panel.json");
                let mut panel = crate::tag_panel::load(&path)?;
                panel.panel = layout.clone();
                crate::tag_panel::save(&path, &panel)?;
            }
            Candidate::Actions(group) => {
                if let Some(index) = actions
                    .groups
                    .iter()
                    .position(|old| old.name.eq_ignore_ascii_case(&group.name))
                {
                    if !replace_names {
                        report.kept.push(name.clone());
                        continue;
                    }
                    actions.groups[index] = group.clone();
                } else {
                    actions.groups.push(group.clone());
                }
                actions_changed = true;
            }
            Candidate::Export(configuration) => {
                let mut configuration = configuration.clone();
                if let Some(index) = exports
                    .configurations
                    .iter()
                    .position(|old| old.name.eq_ignore_ascii_case(&configuration.name))
                {
                    if !replace_names {
                        report.kept.push(name.clone());
                        continue;
                    }
                    configuration.id = exports.configurations[index].id.clone();
                    configuration.file = exports.configurations[index].file.clone();
                    exports.configurations[index] = configuration;
                } else {
                    exports.configurations.push(configuration);
                }
                exports_changed = true;
            }
        }
        report.imported.push(name.clone());
    }
    if report.imported.is_empty() {
        return Err("No setup items would change; review the collision choices.".into());
    }
    if actions_changed {
        crate::action_host::save_library(&stage.path().join("actions.json"), &actions)?;
    }
    if exports_changed {
        crate::exports::save(&stage.path().join("exports.json"), &exports)?;
    }
    let after = native(stage.path())?;
    // Persist the complete rollback journal before replacing a single managed file.
    let old = journal(directory)?;
    if let Some(old) = &old {
        crate::action_host::atomic_output(
            &store(directory)?.join("previous.json"),
            &serde_json::to_vec(old).map_err(|e| e.to_string())?,
            false,
        )?;
    }
    let mut state = Journal {
        version: 1,
        phase: "applying".into(),
        before: encode(&before),
        after: encode(&after),
        window: window.clone(),
        report,
        pending: None,
        rollback: Some(encode(&before)),
        rollback_window: Some(window.clone()),
        previous: old.is_some(),
    };
    persist(directory, &state)?;
    if let Err(error) = replace(directory, &before, &after) {
        recovered(directory, state)?;
        return Err(format!(
            "Setup import failed and its previous configuration was restored: {error}"
        ));
    }
    state.phase = "applied".into();
    state.rollback = None;
    state.rollback_window = None;
    state.pending = Some(Pending {
        id: uuid::Uuid::new_v4().to_string(),
        attempted: false,
        window,
    });
    persist(directory, &state)?;
    status(directory, None, None, None)
}

fn restore(directory: &Path, window: WindowState) -> Result<Status, String> {
    validate_window(&window)?;
    let state = journal(directory)?.ok_or("No setup migration snapshot is available.")?;
    let mut state = recovered(directory, state)?;
    if state.phase != "applied" {
        return Err("No completed setup import is available to restore.".into());
    }
    let current = native(directory)?;
    let before = decode(&state.before)?;
    state.phase = "restoring".into();
    state.rollback = Some(encode(&current));
    state.rollback_window = Some(window);
    persist(directory, &state)?;
    if let Err(error) = replace(directory, &current, &before) {
        recovered(directory, state)?;
        return Err(format!(
            "Setup restore failed; its previous current configuration was retained: {error}"
        ));
    }
    state.phase = "restored".into();
    state.rollback = None;
    state.rollback_window = None;
    state.pending = Some(Pending {
        id: uuid::Uuid::new_v4().to_string(),
        attempted: false,
        window: state.window.clone(),
    });
    persist(directory, &state)?;
    status(directory, None, None, None)
}

#[tauri::command]
pub async fn profile_migration_preview(
    app: tauri::AppHandle,
    path: String,
    window: WindowState,
) -> Result<Preview, String> {
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || preview(&directory, &path, window)).await
}
#[tauri::command]
pub async fn profile_migration_apply(
    app: tauri::AppHandle,
    token: String,
    selected: Vec<String>,
    replace_names: bool,
    window: WindowState,
) -> Result<Status, String> {
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || apply(&directory, &token, &selected, replace_names, window)).await
}
#[tauri::command]
pub async fn profile_migration_status(
    app: tauri::AppHandle,
    start: Option<String>,
    ack: Option<String>,
    retry: Option<String>,
) -> Result<Status, String> {
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || {
        let result = status(
            &directory,
            start.as_deref(),
            ack.as_deref(),
            retry.as_deref(),
        )?;
        crate::settings::reload(&app)?;
        Ok(result)
    })
    .await
}
#[tauri::command]
pub async fn profile_migration_restore(
    app: tauri::AppHandle,
    window: WindowState,
) -> Result<Status, String> {
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || {
        let result = restore(&directory, window)?;
        crate::settings::reload(&app)?;
        Ok(result)
    })
    .await
}
