//! Session undo and redo for commands that write audio files. Each mutating
//! command runs through [`record`], which snapshots its local targets in the
//! portable journal before the work and records the outcome afterwards.
use crate::action_host::{ActionResult, PathChange};
use serde::Serialize;
use std::{collections::HashSet, path::Path, sync::Mutex};
use tag_core::batch_errors::ErrorCause;
use tag_core::history::{Capture, HistoryState, Journal, Limits};
use tag_core::ScanResult;
#[cfg(not(test))]
use tauri::Manager;

#[derive(Default)]
struct HistoryAdapter {
    journal: Option<Journal>,
    unavailable: Option<UnavailableHistory>,
}

struct UnavailableHistory {
    notice: String,
    groups: HashSet<String>,
}

static JOURNAL: Mutex<HistoryAdapter> = Mutex::new(HistoryAdapter {
    journal: None,
    unavailable: None,
});

// Hold this across capture, work and commit/replay. The work's side-file hook
// reacquires JOURNAL, so the adapter lock itself cannot span that work.
static OPERATIONS: Mutex<()> = Mutex::new(());

impl HistoryAdapter {
    fn journal(&mut self) -> Result<&mut Journal, ErrorCause> {
        if self.journal.is_none() {
            #[cfg(not(test))]
            let root = match super::APP_HANDLE.get() {
                Some(app) => app
                    .path()
                    .app_cache_dir()
                    .map_err(ErrorCause::external)?
                    .join("undo"),
                None => std::env::temp_dir().join("opentag-undo"),
            };
            #[cfg(test)]
            let root = TEST_JOURNAL_ROOT
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| {
                    // A test process shares commands' history, but must not load an
                    // experimental or real cache left by a previous process.
                    static DEFAULT_ROOT: std::sync::OnceLock<tempfile::TempDir> =
                        std::sync::OnceLock::new();
                    DEFAULT_ROOT
                        .get_or_init(|| {
                            tempfile::Builder::new()
                                .prefix("taggroove-backend-history-fixture-")
                                .tempdir()
                                .expect("test history directory")
                        })
                        .path()
                        .join("undo")
                });
            self.journal =
                Some(Journal::open(&root, Limits::default()).map_err(ErrorCause::external)?);
        }
        Ok(self.journal.as_mut().expect("journal was opened"))
    }

    fn state(&mut self) -> Result<HistoryState, ErrorCause> {
        let state = self.journal().map(|journal| journal.state());
        match (&self.unavailable, state) {
            (Some(unavailable), state) => {
                let mut state = state.unwrap_or_default();
                state.notice = Some(unavailable.notice.clone());
                Ok(state)
            }
            (None, state) => state,
        }
    }
}

fn with_journal<T>(work: impl FnOnce(&mut Journal) -> T) -> Result<T, String> {
    with_journal_diagnostic(work).map_err(|cause| cause.to_string())
}
fn with_journal_diagnostic<T>(work: impl FnOnce(&mut Journal) -> T) -> Result<T, ErrorCause> {
    let mut guard = JOURNAL
        .lock()
        .map_err(|_| ErrorCause::owned("Undo history lock failed"))?;
    Ok(work(guard.journal()?))
}

fn begin_capture(label: &str, group: Option<&str>, paths: &[String]) -> Option<(Capture, u64)> {
    let Ok(mut history) = JOURNAL.lock() else {
        return None;
    };
    if let Some(unavailable) = history.unavailable.as_ref() {
        if group.is_some_and(|group| unavailable.groups.contains(group)) {
            // No later file in this group can make its earlier unsnapshotted
            // writes part of a truthful Undo step.
            return None;
        }
    }
    match history.journal() {
        Ok(journal) => {
            let before = journal.outcome_revision();
            Some((journal.begin(label, group, paths), before))
        }
        Err(reason) => {
            let unavailable = history
                .unavailable
                .get_or_insert_with(|| UnavailableHistory {
                    notice: format!("{label} cannot be undone: {reason}."),
                    groups: HashSet::new(),
                });
            if let Some(group) = group {
                unavailable.groups.insert(group.to_owned());
            }
            None
        }
    }
}

/// Path changes reported by a command, used to find each file afterwards.
pub trait Changes {
    fn completed(&self) -> Vec<String> {
        Vec::new()
    }
    fn renamed(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    fn directories(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}
fn completed_paths<'a>(
    paths: impl Iterator<Item = &'a str>,
    errors: &[tag_core::FileError],
) -> Vec<String> {
    fn file(path: &str) -> &str {
        tag_core::subsong_parts(path).map_or(path, |(file, _)| file)
    }
    let failed: std::collections::HashSet<_> =
        errors.iter().map(|error| file(&error.path)).collect();
    paths
        .map(file)
        .filter(|path| !failed.contains(path))
        .map(str::to_owned)
        .collect()
}
fn completed_scan(result: &ScanResult) -> Vec<String> {
    completed_paths(
        result.entries.iter().map(|entry| entry.path.as_str()),
        &result.errors,
    )
}
impl Changes for ScanResult {
    fn completed(&self) -> Vec<String> {
        completed_scan(self)
    }
}
impl Changes for crate::file_ops::TransferResult {
    fn completed(&self) -> Vec<String> {
        completed_scan(&self.result)
    }
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .chain(&self.associated_renamed)
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl<T: Changes> Changes for tag_core::batch_errors::Detailed<T> {
    fn completed(&self) -> Vec<String> {
        self.result.completed()
    }
    fn renamed(&self) -> Vec<(String, String)> {
        self.result.renamed()
    }
    fn directories(&self) -> Vec<(String, String)> {
        self.result.directories()
    }
}

impl Changes for super::FilenameImport {
    fn completed(&self) -> Vec<String> {
        completed_scan(&self.result)
    }
}
impl Changes for super::RenameResult {
    fn completed(&self) -> Vec<String> {
        completed_scan(&self.result)
    }
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .chain(&self.associated_renamed)
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl Changes for crate::converters::FilenameResult {
    fn completed(&self) -> Vec<String> {
        completed_scan(&self.result)
    }
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .chain(&self.associated_renamed)
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl Changes for ActionResult {
    fn completed(&self) -> Vec<String> {
        completed_scan(&self.result)
    }
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .chain(&self.associated_renamed)
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
    fn directories(&self) -> Vec<(String, String)> {
        self.moved_directories
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}

#[cfg(test)]
pub(crate) static TEST_ROOTS: Mutex<Vec<std::path::PathBuf>> = Mutex::new(Vec::new());
/// Tests that undo through the shared journal hold this so their steps do not interleave.
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());
#[cfg(test)]
static TEST_JOURNAL_ROOT: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

// Provider documents publish through their own recovery copies and are not journaled.
fn journaled(path: &str) -> bool {
    // Commands share one journal; tests opt in so parallel tests cannot interleave steps.
    #[cfg(test)]
    if !TEST_ROOTS
        .lock()
        .unwrap()
        .iter()
        .any(|root| Path::new(path).starts_with(root))
    {
        return false;
    }
    !cfg!(target_os = "android")
        && !path.starts_with("content://")
        && !super::documents::is_managed(Path::new(path))
}

/// Run `work` with undo snapshots of `paths`. Operations sharing `group`
/// form one undo step. Journal failures never block the operation itself.
pub fn record<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_files(label, group, paths, |_| work())
}

/// [`record`] for work that also creates or replaces files that are not audio
/// (a saved cover image). `work` calls the hook it receives with each such
/// path before writing it; an `Err` from the hook means the file must not be
/// written because its undo copy failed.
pub fn record_with_side_files<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path) -> Result<(), String>) -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_file_epochs(label, group, paths, |capture| {
        work(&mut |path| capture(path, 0))
    })
}

/// Side-file recording whose hook also receives the completed directory-move count.
pub fn record_with_side_file_epochs<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path, usize) -> Result<(), String>) -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_file_epochs_diagnostic(label, group, paths, |capture| {
        work(&mut |path, epoch| capture(path, epoch).map_err(|cause| cause.to_string()))
    })
}

/// The same history/cache transaction, retaining the side-file hook's error provenance.
pub fn record_with_side_file_epochs_diagnostic<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path, usize) -> Result<(), ErrorCause>) -> Result<T, String>,
) -> Result<T, String> {
    let result = journal_work_diagnostic(label, group, paths, |capture| {
        // Capturing history can add a hard link to the live file. Observe its
        // identity/link count after that capture, immediately before the write.
        let source_epoch = super::source_tracking::before_change(Some(paths));
        let result = work(capture);
        let (renamed, directories, completed) = result
            .as_ref()
            .map(|r| (r.renamed(), r.directories(), r.completed()))
            .unwrap_or_default();
        // A grouped commit can remove a redundant before-snapshot. Rebind
        // successful writes before that cleanup changes the old link count.
        super::source_tracking::changed(paths, &renamed, &directories, &source_epoch, &completed);
        result
    });
    // The library reads written, renamed and moved files again.
    let (renamed, directories) = result
        .as_ref()
        .map(|r| (r.renamed(), r.directories()))
        .unwrap_or_default();
    super::library::changed(paths, &renamed, &directories);
    result
}

pub fn record_with_side_files_diagnostic<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path) -> Result<(), ErrorCause>) -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_file_epochs_diagnostic(label, group, paths, |capture| {
        work(&mut |path| capture(path, 0))
    })
}

fn journal_work_diagnostic<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path, usize) -> Result<(), ErrorCause>) -> Result<T, String>,
) -> Result<T, String> {
    // Chapter and cue sheet rows are snapshots of their file.
    let mut local: Vec<String> = Vec::new();
    for path in paths {
        let file = tag_core::subsong_parts(path).map_or(path.as_str(), |(file, _)| file);
        if journaled(file) && !local.iter().any(|p| p == file) {
            local.push(file.to_owned());
        }
    }
    if local.is_empty() {
        return work(&mut |_, _| Ok(()));
    }
    let _operation = OPERATIONS.lock().unwrap_or_else(|error| error.into_inner());
    let mut capture = begin_capture(label, group, &local);
    let result = work(&mut |path, epoch| match capture.as_mut() {
        Some((capture, _)) if journaled(&path.to_string_lossy()) => {
            with_journal_diagnostic(|journal| {
                journal.capture_side_file_after_moves_diagnostic(capture, path, epoch)
            })?
        }
        _ => Ok(()),
    });
    if let Some((capture, before)) = capture {
        let (renamed, directories) = result
            .as_ref()
            .map(|r| (r.renamed(), r.directories()))
            .unwrap_or_default();
        if let Ok(mut history) = JOURNAL.lock() {
            if let Some(journal) = history.journal.as_mut() {
                journal.commit(capture, &renamed, &directories);
                // Reopened steps/notices are not recovery from a later startup
                // failure. Replace it only with a newly recorded result/refusal.
                if journal.outcome_revision() != before {
                    history.unavailable = None;
                }
            }
        }
    }
    result
}

#[derive(Serialize)]
pub struct HistoryResult {
    #[serde(flatten)]
    result: ScanResult,
    renamed: Vec<PathChange>,
    moved_directories: Vec<PathChange>,
    label: String,
    history: HistoryState,
}

#[tauri::command]
pub fn history_state() -> Result<HistoryState, String> {
    JOURNAL
        .lock()
        .map_err(|_| "Undo history lock failed".to_string())?
        .state()
        .map_err(|cause| cause.to_string())
}

#[tauri::command]
pub async fn export_history_recovery(destination: String) -> Result<String, String> {
    super::io(move || {
        let _operation = OPERATIONS.lock().unwrap_or_else(|error| error.into_inner());
        with_journal(|journal| {
            journal
                .export_recovery(Path::new(&destination))
                .map(|path| path.to_string_lossy().into_owned())
        })?
    })
    .await
}

#[cfg(test)]
fn replay(backward: bool) -> Result<HistoryResult, String> {
    replay_diagnostic(backward).map(|report| report.result)
}
fn replay_diagnostic(
    backward: bool,
) -> Result<tag_core::batch_errors::Detailed<HistoryResult>, String> {
    let _operation = OPERATIONS.lock().unwrap_or_else(|error| error.into_inner());
    let source_epoch = super::source_tracking::before_change(None);
    let report = with_journal(|journal| {
        journal.set_tag_options(super::settings::tag_options());
        if backward {
            journal.undo_diagnostic()
        } else {
            journal.redo_diagnostic()
        }
    })??;
    let replay = report.result;
    super::library::changed(&replay.paths, &replay.renamed, &replay.directories);
    let completed = completed_paths(replay.paths.iter().map(String::as_str), &replay.errors);
    super::source_tracking::replayed(
        &replay.paths,
        &replay.renamed,
        &replay.directories,
        &source_epoch,
        &completed,
    );
    let audio_paths: Vec<_> = replay
        .paths
        .iter()
        .filter(|path| !crate::associated_files::is_companion(Path::new(path)))
        .cloned()
        .collect();
    let mut result = super::batch(&audio_paths, super::settings::read_summary);
    let offset = result.errors.len();
    let diagnostics = report
        .error_diagnostics
        .into_iter()
        .map(|mut item| {
            item.index += offset;
            item
        })
        .collect();
    result.errors.extend(replay.errors);
    let changes = |pairs: Vec<(String, String)>| {
        pairs
            .into_iter()
            .map(|(from, to)| PathChange { from, to })
            .collect()
    };
    Ok(tag_core::batch_errors::Detailed {
        result: HistoryResult {
            result,
            renamed: changes(replay.renamed),
            moved_directories: changes(replay.directories),
            label: replay.label,
            history: history_state()?,
        },
        error_diagnostics: diagnostics,
    })
}

#[cfg(test)]
#[tauri::command]
pub async fn undo_last() -> Result<HistoryResult, String> {
    super::io(|| replay(true)).await
}

#[cfg(test)]
#[tauri::command]
pub async fn redo_last() -> Result<HistoryResult, String> {
    super::io(|| replay(false)).await
}

/// Keep callable String/result APIs above; the same command identifiers return additive wire companions.
pub mod ipc {
    #[tauri::command(rename = "undo_last")]
    pub async fn undo_last_with_diagnostics(
    ) -> Result<tag_core::batch_errors::Detailed<super::HistoryResult>, String> {
        crate::io(|| super::replay_diagnostic(true)).await
    }
    #[tauri::command(rename = "redo_last")]
    pub async fn redo_last_with_diagnostics(
    ) -> Result<tag_core::batch_errors::Detailed<super::HistoryResult>, String> {
        crate::io(|| super::replay_diagnostic(false)).await
    }
    #[tauri::command(rename = "convert_tag_to_tag")]
    pub async fn convert_tag_to_tag_with_diagnostics(
        paths: Vec<String>,
        field: String,
        template: String,
        columns: Option<Vec<crate::evaluation::ColumnSpec>>,
    ) -> Result<tag_core::batch_errors::Detailed<crate::action_host::ActionResult>, String> {
        super::convert_tag_to_tag_diagnostic(paths, field, template, columns).await
    }
}

/// Convert → Tag - Tag: format one field (or `_FILENAME`/`_DIRECTORY`) of each
/// selected file from a format string, written immediately like Mp3tag.
#[cfg(test)]
#[tauri::command]
pub async fn convert_tag_to_tag(
    paths: Vec<String>,
    field: String,
    template: String,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<ActionResult, String> {
    convert_tag_to_tag_diagnostic(paths, field, template, columns)
        .await
        .map(|report| report.result)
}
pub async fn convert_tag_to_tag_diagnostic(
    paths: Vec<String>,
    field: String,
    template: String,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<tag_core::batch_errors::Detailed<ActionResult>, String> {
    let settings = super::evaluation_settings(columns)?;
    let field = field.trim().to_owned();
    if field.is_empty() {
        return Err("Enter a field name".into());
    }
    super::io(move || {
        super::reject_provider_operation(&paths, "Tag → Tag")?;
        let group = tag_core::actions::ActionGroup {
            name: "Tag → Tag".into(),
            enabled: true,
            actions: vec![tag_core::actions::ActionSpec::FormatValue { field, template }],
        };
        let groups = [group];
        let companions = crate::action_companions::Prepared::prepare(
            &paths,
            &groups,
            crate::associated_files::enabled(),
        )?;
        let captured = companions.history_sources(&paths);
        record("Tag → Tag", None, &captured, || {
            Ok(crate::action_host::execute_prepared(
                &paths,
                &groups,
                settings,
                companions,
                &mut |_, _| Ok(()),
            ))
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::Fields;

    struct RetainedFixture(Option<tempfile::TempDir>);
    impl RetainedFixture {
        fn new() -> Self {
            Self(Some(tempfile::tempdir().unwrap()))
        }
        fn path(&self) -> &Path {
            self.0.as_ref().unwrap().path()
        }
    }
    impl Drop for RetainedFixture {
        fn drop(&mut self) {
            if std::thread::panicking() {
                eprintln!(
                    "Retained backend history fixture: {}",
                    self.0.take().unwrap().keep().display()
                );
            }
        }
    }

    fn wav(path: &Path, title: &str) {
        let samples = vec![0u8; 8820];
        let mut data = b"RIFF".to_vec();
        data.extend((36 + samples.len() as u32).to_le_bytes());
        data.extend(b"WAVEfmt ");
        data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(44100u32.to_le_bytes());
        data.extend(88200u32.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(16u16.to_le_bytes());
        data.extend(b"data");
        data.extend((samples.len() as u32).to_le_bytes());
        data.extend(samples);
        std::fs::write(path, data).unwrap();
        tag_core::write_fields(path, &Fields::from([("TITLE".into(), vec![title.into()])]))
            .unwrap();
    }
    fn title(path: &Path) -> Vec<String> {
        tag_core::read_entry_summary(path).unwrap().fields["TITLE"].clone()
    }
    fn step() -> (String, usize) {
        let step = history_state().unwrap().undo.unwrap();
        (step.label, step.files)
    }
    fn run<T>(future: impl std::future::Future<Output = Result<T, String>>) -> T {
        tauri::async_runtime::block_on(future).unwrap()
    }

    struct IsolatedHistory {
        history: HistoryAdapter,
        roots: Vec<std::path::PathBuf>,
        journal_root: Option<std::path::PathBuf>,
    }

    impl IsolatedHistory {
        fn new(root: &Path) -> Self {
            Self {
                history: std::mem::take(&mut *JOURNAL.lock().unwrap()),
                roots: std::mem::replace(&mut *TEST_ROOTS.lock().unwrap(), vec![root.to_owned()]),
                journal_root: TEST_JOURNAL_ROOT.lock().unwrap().replace(root.join("undo")),
            }
        }
    }

    impl Drop for IsolatedHistory {
        fn drop(&mut self) {
            *JOURNAL.lock().unwrap_or_else(|error| error.into_inner()) =
                std::mem::take(&mut self.history);
            *TEST_ROOTS.lock().unwrap_or_else(|error| error.into_inner()) =
                std::mem::take(&mut self.roots);
            *TEST_JOURNAL_ROOT
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = self.journal_root.take();
        }
    }

    #[test]
    fn source_tracking_follows_grouped_saves_with_a_retained_hard_link() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temp.path()).unwrap();
        let _history = IsolatedHistory::new(&root);
        let path = root.join("grouped.wav");
        let later = root.join("later.wav");
        wav(&path, "Before");
        let session = run(crate::source_tracking::begin_source_tracking());
        let prepared = run(crate::source_tracking::prepare_source_roots(
            session.clone(),
            vec![path.to_str().unwrap().into()],
        ));
        let prepared = serde_json::to_value(prepared).unwrap();
        for title_value in ["First", "Second"] {
            let result = run(crate::save_fields(
                vec![path.to_str().unwrap().into()],
                Fields::from([("TITLE".into(), vec![title_value.into()])]),
                Some("source-tracking-group".into()),
            ));
            assert!(result.errors.is_empty());
            if title_value == "First" {
                std::fs::hard_link(&path, root.join("retained.wav")).unwrap();
            }
        }
        std::fs::rename(&path, &later).unwrap();
        let resolved = run(crate::source_tracking::resolve_source_roots(
            session.clone(),
            serde_json::from_value(prepared["sources"].clone()).unwrap(),
        ));
        assert_eq!(
            serde_json::to_value(resolved).unwrap()["paths"],
            serde_json::json!([later.to_str().unwrap()])
        );
        assert_eq!(title(&later), vec!["Second"]);
        assert_eq!(title(&root.join("retained.wav")), vec!["First"]);
        run(crate::source_tracking::release_source_roots(
            session,
            serde_json::from_value(prepared["acquired"].clone()).unwrap(),
        ));
    }

    #[test]
    fn source_tracking_rebinds_real_save_undo_and_redo_after_external_refresh() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(temp.path()).unwrap();
        let _history = IsolatedHistory::new(&root);
        let old = root.join("before.wav");
        let followed = root.join("followed.wav");
        let later = root.join("later.wav");
        wav(&old, "Before");
        let original = std::fs::read(&old).unwrap();
        let session = run(crate::source_tracking::begin_source_tracking());
        let prepared = run(crate::source_tracking::prepare_source_roots(
            session.clone(),
            vec![old.to_str().unwrap().into()],
        ));
        let prepared = serde_json::to_value(prepared).unwrap();
        let sources = serde_json::from_value(prepared["sources"].clone()).unwrap();
        std::fs::rename(&old, &followed).unwrap();
        let refreshed = run(crate::source_tracking::resolve_source_roots(
            session.clone(),
            sources,
        ));
        let refreshed = serde_json::to_value(refreshed).unwrap();
        let paths: Vec<String> = serde_json::from_value(refreshed["paths"].clone()).unwrap();
        assert_eq!(paths, vec![followed.to_str().unwrap()]);
        let result = run(crate::save_fields(
            paths,
            Fields::from([("TITLE".into(), vec!["After".into()])]),
            None,
        ));
        assert!(result.errors.is_empty());
        assert_eq!(title(&followed), vec!["After"]);
        let saved = std::fs::read(&followed).unwrap();
        assert!(run(undo_last()).result.errors.is_empty());
        assert_eq!(std::fs::read(&followed).unwrap(), original);
        assert!(run(redo_last()).result.errors.is_empty());
        assert_eq!(std::fs::read(&followed).unwrap(), saved);
        std::fs::rename(&followed, &later).unwrap();
        let sources = serde_json::from_value(refreshed["sources"].clone()).unwrap();
        let found = run(crate::source_tracking::resolve_source_roots(
            session.clone(),
            sources,
        ));
        assert_eq!(
            serde_json::to_value(found).unwrap()["paths"],
            serde_json::json!([later.to_str().unwrap()])
        );
        assert_eq!(std::fs::read(later).unwrap(), saved);
        run(crate::source_tracking::release_source_roots(
            session,
            serde_json::from_value(prepared["acquired"].clone()).unwrap(),
        ));
    }

    fn riff_chunk(path: &Path, wanted: &[u8; 4]) -> Vec<u8> {
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        let mut offset = 12;
        while offset + 8 <= bytes.len() {
            let length =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let end = offset + 8 + length;
            assert!(end <= bytes.len());
            if &bytes[offset..offset + 4] == wanted {
                return bytes[offset + 8..end].to_vec();
            }
            offset = end + (length & 1);
        }
        panic!("missing independently inspected RIFF chunk");
    }

    fn wav_with_opaque_chunk(path: &Path) {
        wav(path, "Original");
        let mut bytes = std::fs::read(path).unwrap();
        let mut offset = 12;
        while &bytes[offset..offset + 4] != b"data" {
            let length =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            offset += 8 + length + (length & 1);
        }
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        for (index, sample) in bytes[offset + 8..offset + 8 + length]
            .iter_mut()
            .enumerate()
        {
            *sample = (index % 251) as u8;
        }
        let opaque = b"\0opaque\xff\0\x01";
        bytes.extend(b"uNKn");
        bytes.extend((opaque.len() as u32).to_le_bytes());
        bytes.extend(opaque);
        if opaque.len() & 1 != 0 {
            bytes.push(0);
        }
        let size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn split_field_native_vectors_reach_real_save_and_exact_history() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../crates/tag-core/tests/fixtures/parity-native/split-field.json"
        ))
        .unwrap();
        let mut exercised = 0;
        for case in cases.as_array().unwrap() {
            // The pure model covers an explicitly empty string. WAV's writer
            // normalizes that input to absence, so it is not a physical oracle.
            if case["id"] == 5 {
                continue;
            }
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            let _scope = IsolatedHistory::new(&root);
            let audio = root.join("selected.wav");
            let other = root.join("unselected.wav");
            wav_with_opaque_chunk(&audio);
            wav(&other, "Untouched");
            let other_before = std::fs::read(&other).unwrap();
            let values: Vec<String> = serde_json::from_value(case["input_values"].clone()).unwrap();
            tag_core::write_fields(&audio, &Fields::from([("TITLE".into(), values.clone())]))
                .unwrap();
            assert_eq!(title(&audio), values, "setup case {}", case["id"]);
            let before = std::fs::read(&audio).unwrap();
            let pcm = riff_chunk(&audio, b"data");
            let opaque = riff_chunk(&audio, b"uNKn");
            let result = run(crate::execute_action_groups(
                vec![audio.to_string_lossy().into_owned()],
                vec![tag_core::actions::ActionGroup {
                    name: "Split native vector".into(),
                    enabled: true,
                    actions: vec![tag_core::actions::ActionSpec::SplitField {
                        field: "TITLE".into(),
                        separator: case["delimiter"].as_str().unwrap().into(),
                    }],
                }],
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let expected: Option<Vec<String>> =
                serde_json::from_value(case["native_output_values"].clone()).unwrap();
            assert_eq!(
                tag_core::read_entry_summary(&audio)
                    .unwrap()
                    .fields
                    .get("TITLE"),
                expected.as_ref()
            );
            assert_eq!(riff_chunk(&audio, b"data"), pcm);
            assert_eq!(riff_chunk(&audio, b"uNKn"), opaque);
            assert_eq!(std::fs::read(&other).unwrap(), other_before);
            let after = std::fs::read(&audio).unwrap();
            let state = history_state().unwrap();
            assert!(state.notice.is_none(), "{:?}", state.notice);
            if case["id"] == 8 {
                assert_eq!(after, before);
                assert!(state.undo.is_none());
            } else {
                assert!(state.undo.is_some());
                assert!(run(undo_last()).result.errors.is_empty());
                assert_eq!(std::fs::read(&audio).unwrap(), before);
                assert!(run(redo_last()).result.errors.is_empty());
                assert_eq!(std::fs::read(&audio).unwrap(), after);
            }
            exercised += 1;
        }
        assert_eq!(exercised, 7);
    }

    #[test]
    fn reopened_old_undo_does_not_clear_a_startup_failure_on_a_noop() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = RetainedFixture::new();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        let one = root.join("old-recorded.wav");
        let two = root.join("unrecorded.wav");
        wav(&one, "Original one");
        wav(&two, "Original two");
        let key = |path: &Path| path.to_string_lossy().into_owned();
        let saved = run(crate::save_fields(
            vec![key(&one)],
            Fields::from([("TITLE".into(), vec!["Old recorded".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty());
        let saved_one = std::fs::read(&one).unwrap();
        std::fs::write(root.join("independent-one-saved.wav"), &saved_one).unwrap();
        JOURNAL.lock().unwrap().journal = None;
        let cache = root.join("undo");
        let held = root.join("independent-held-undo");
        std::fs::rename(&cache, &held).unwrap();
        std::fs::write(&cache, b"Owned startup obstruction").unwrap();
        let saved = run(crate::save_fields(
            vec![key(&two)],
            Fields::from([("TITLE".into(), vec!["Unrecorded two".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty());
        let unrecorded_two = std::fs::read(&two).unwrap();
        std::fs::write(root.join("independent-two-unrecorded.wav"), &unrecorded_two).unwrap();
        let failed_notice = history_state().unwrap().notice.unwrap();
        std::fs::remove_file(&cache).unwrap();
        std::fs::rename(&held, &cache).unwrap();
        let reopened = history_state().unwrap();
        assert!(
            reopened.undo.is_some(),
            "Fixture must reopen a preexisting step"
        );
        assert_eq!(reopened.notice.as_deref(), Some(failed_notice.as_str()));
        record("No changes", None, &[key(&one)], || {
            Ok(ScanResult::default())
        })
        .unwrap();
        assert_eq!(
            history_state().unwrap().notice.as_deref(),
            Some(failed_notice.as_str()),
            "A no-op cannot recover the unrecorded operation just because old Undo exists"
        );
        assert_eq!(std::fs::read(&one).unwrap(), saved_one);
        assert_eq!(std::fs::read(&two).unwrap(), unrecorded_two);
        let saved = run(crate::save_fields(
            vec![key(&two)],
            Fields::from([("TITLE".into(), vec!["New recorded".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty());
        assert!(history_state().unwrap().notice.is_none());
        assert!(run(undo_last()).result.errors.is_empty());
        assert_eq!(std::fs::read(&two).unwrap(), unrecorded_two);
    }

    #[test]
    fn whole_record_operations_serialize_while_side_capture_can_reenter_the_journal() {
        use std::{sync::mpsc, time::Duration};
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = RetainedFixture::new();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        let one = root.join("one.wav");
        let two = root.join("two.wav");
        wav(&one, "One");
        wav(&two, "Two");
        let first_key = one.to_string_lossy().into_owned();
        let second_key = two.to_string_lossy().into_owned();
        let side = root.join("cover-output.bin");
        let (entered_one, ready_one) = mpsc::channel();
        let (release_one, wait_one) = mpsc::channel();
        let (attempt_two, attempted_two) = mpsc::channel();
        let (entered_two, ready_two) = mpsc::channel();
        let early = std::thread::scope(|scope| {
            let first = scope.spawn(move || {
                record_with_side_files("First operation", None, &[first_key], |capture| {
                    capture(&side)?;
                    entered_one.send(()).unwrap();
                    wait_one.recv_timeout(Duration::from_secs(10)).unwrap();
                    Ok(ScanResult::default())
                })
            });
            ready_one.recv_timeout(Duration::from_secs(10)).unwrap();
            let second = scope.spawn(move || {
                attempt_two.send(()).unwrap();
                record("Second operation", None, &[second_key], || {
                    entered_two.send(()).unwrap();
                    Ok(ScanResult::default())
                })
            });
            attempted_two.recv_timeout(Duration::from_secs(10)).unwrap();
            let early = ready_two.recv_timeout(Duration::from_millis(250)).is_ok();
            release_one.send(()).unwrap();
            first.join().unwrap().unwrap();
            second.join().unwrap().unwrap();
            early
        });
        assert!(
            !early,
            "A second operation entered work before the first committed its capture"
        );
        assert!(history_state().unwrap().notice.is_none());
    }

    #[test]
    fn replay_waits_for_an_inflight_record_then_undoes_that_completed_step() {
        use std::{sync::mpsc, time::Duration};
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = RetainedFixture::new();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        let one = root.join("one.wav");
        let two = root.join("two.wav");
        wav(&one, "One");
        wav(&two, "Two");
        let first_key = one.to_string_lossy().into_owned();
        let second_key = two.to_string_lossy().into_owned();
        record("Earlier save", None, &[first_key], || {
            tag_core::write_fields(
                &one,
                &Fields::from([("TITLE".into(), vec!["Earlier saved".into()])]),
            )
            .unwrap();
            Ok(ScanResult::default())
        })
        .unwrap();
        let saved_one = std::fs::read(&one).unwrap();
        let before_two = std::fs::read(&two).unwrap();
        std::fs::write(root.join("independent-one-saved.wav"), &saved_one).unwrap();
        std::fs::write(root.join("independent-two-before.wav"), &before_two).unwrap();
        let (entered, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let (attempt, attempted) = mpsc::channel();
        let (finished, result) = mpsc::channel();
        let (early, report) = std::thread::scope(|scope| {
            let active_two = two.clone();
            let writer = scope.spawn(move || {
                record("Inflight save", None, &[second_key], || {
                    entered.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(10)).unwrap();
                    tag_core::write_fields(
                        &active_two,
                        &Fields::from([("TITLE".into(), vec!["Inflight saved".into()])]),
                    )
                    .unwrap();
                    Ok(ScanResult::default())
                })
            });
            ready.recv_timeout(Duration::from_secs(10)).unwrap();
            let undo = scope.spawn(move || {
                attempt.send(()).unwrap();
                assert!(finished.send(replay_diagnostic(true)).is_ok());
            });
            attempted.recv_timeout(Duration::from_secs(10)).unwrap();
            let early = result.recv_timeout(Duration::from_millis(250)).ok();
            let arrived_early = early.is_some();
            release.send(()).unwrap();
            writer.join().unwrap().unwrap();
            undo.join().unwrap();
            let report =
                early.unwrap_or_else(|| result.recv_timeout(Duration::from_secs(10)).unwrap());
            (arrived_early, report)
        });
        assert!(!early, "Replay ran before the pending save finished");
        let report = report.unwrap();
        assert!(report.result.result.errors.is_empty());
        assert_eq!(std::fs::read(&one).unwrap(), saved_one);
        assert_eq!(std::fs::read(&two).unwrap(), before_two);
        assert_eq!(history_state().unwrap().undo.unwrap().label, "Earlier save");
    }

    #[test]
    fn startup_failure_is_nonfatal_and_notice_survives_until_a_recorded_save() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        let audio = root.join("selected.wav");
        let other = root.join("unselected.wav");
        wav_with_opaque_chunk(&audio);
        wav(&other, "Untouched");
        let other_before = std::fs::read(&other).unwrap();
        let pcm = riff_chunk(&audio, b"data");
        let opaque = riff_chunk(&audio, b"uNKn");
        let obstruction = root.join("undo");
        std::fs::write(&obstruction, b"owned obstruction").unwrap();
        let key = audio.to_string_lossy().into_owned();
        let saved = run(crate::save_fields(
            vec![key.clone()],
            Fields::from([("TITLE".into(), vec!["Unrecorded".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty(), "{:?}", saved.errors);
        assert_eq!(title(&audio), ["Unrecorded"]);
        assert_eq!(riff_chunk(&audio, b"data"), pcm);
        assert_eq!(riff_chunk(&audio, b"uNKn"), opaque);
        assert_eq!(std::fs::read(&other).unwrap(), other_before);
        assert_eq!(std::fs::read(&obstruction).unwrap(), b"owned obstruction");
        let unrecorded = std::fs::read(&audio).unwrap();
        let failed = history_state().expect("a successful Save must expose unavailable Undo");
        assert!(failed.undo.is_none() && failed.redo.is_none());
        assert!(failed
            .notice
            .as_deref()
            .unwrap()
            .contains("Save tags cannot be undone:"));
        assert_eq!(history_state().unwrap(), failed);
        std::fs::remove_file(&obstruction).unwrap();
        assert_eq!(
            history_state().unwrap(),
            failed,
            "opening history must retain the notice"
        );
        record("No changes", None, std::slice::from_ref(&key), || {
            Ok(ScanResult::default())
        })
        .unwrap();
        assert_eq!(history_state().unwrap().notice, failed.notice);
        let saved = run(crate::save_fields(
            vec![key],
            Fields::from([("TITLE".into(), vec!["Recorded".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty());
        let recorded = std::fs::read(&audio).unwrap();
        let state = history_state().unwrap();
        assert_eq!(state.undo.unwrap().label, "Save tags");
        assert!(state.notice.is_none());
        assert!(run(undo_last()).result.errors.is_empty());
        assert_eq!(std::fs::read(&audio).unwrap(), unrecorded);
        assert!(run(redo_last()).result.errors.is_empty());
        assert_eq!(std::fs::read(&audio).unwrap(), recorded);
        assert_eq!(riff_chunk(&audio, b"data"), pcm);
        assert_eq!(riff_chunk(&audio, b"uNKn"), opaque);
    }

    #[test]
    fn startup_failure_keeps_the_rest_of_the_group_unrecorded() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        let first = root.join("first.wav");
        let second = root.join("second.wav");
        wav(&first, "First original");
        wav(&second, "Second original");
        let key = |path: &Path| path.to_string_lossy().into_owned();
        std::fs::write(root.join("undo"), b"owned obstruction").unwrap();
        let saved = run(crate::save_fields(
            vec![key(&first)],
            Fields::from([("TITLE".into(), vec!["First unrecorded".into()])]),
            Some("failed-group".into()),
        ));
        assert!(saved.errors.is_empty());
        let failed = history_state().expect("failed grouped capture must expose its notice");
        record("No changes", Some("blocked-no-op"), &[key(&first)], || {
            Ok(ScanResult::default())
        })
        .unwrap();
        assert_eq!(history_state().unwrap(), failed);
        std::fs::remove_file(root.join("undo")).unwrap();
        assert_eq!(history_state().unwrap(), failed);
        record(
            "No changes",
            Some("unrelated-no-op"),
            &[key(&first)],
            || Ok(ScanResult::default()),
        )
        .unwrap();
        assert_eq!(history_state().unwrap(), failed);
        let saved = run(crate::save_fields(
            vec![key(&second)],
            Fields::from([("TITLE".into(), vec!["Second unrecorded".into()])]),
            Some("failed-group".into()),
        ));
        assert!(saved.errors.is_empty());
        assert_eq!(history_state().unwrap(), failed);
        let saved = run(crate::save_fields(
            vec![key(&first)],
            Fields::from([("TITLE".into(), vec!["First recorded".into()])]),
            Some("later-group".into()),
        ));
        assert!(saved.errors.is_empty());
        assert!(history_state().unwrap().notice.is_none());
        assert_eq!(history_state().unwrap().undo.unwrap().files, 1);
        assert!(run(undo_last()).result.errors.is_empty());
        assert_eq!(title(&first), ["First unrecorded"]);
        assert_eq!(title(&second), ["Second unrecorded"]);
    }

    #[test]
    fn startup_failure_preserves_work_errors_and_excluded_paths_do_not_warn() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        std::fs::write(root.join("undo"), b"owned obstruction").unwrap();
        let outside = tempfile::tempdir().unwrap();
        let audio = outside.path().join("excluded.wav");
        wav(&audio, "Excluded");
        let saved = run(crate::save_fields(
            vec![audio.to_string_lossy().into_owned()],
            Fields::from([("TITLE".into(), vec!["Changed".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty());
        assert!(
            history_state().is_err(),
            "excluded writes must not create an Undo notice"
        );
        let eligible = root.join("eligible.wav");
        wav(&eligible, "Original");
        let result = record::<ScanResult>(
            "Failed work",
            None,
            &[eligible.to_string_lossy().into_owned()],
            || Err("original operation failure".into()),
        );
        assert_eq!(result.unwrap_err(), "original operation failure");
    }

    #[test]
    fn startup_notice_adapter_preserves_initialized_capture_refusal() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _scope = IsolatedHistory::new(&root);
        let audio = root.join("selected.wav");
        wav_with_opaque_chunk(&audio);
        let pcm = riff_chunk(&audio, b"data");
        let opaque = riff_chunk(&audio, b"uNKn");
        JOURNAL.lock().unwrap().journal = Some(
            Journal::open(
                &root.join("undo"),
                Limits {
                    bytes: 1,
                    ..Limits::default()
                },
            )
            .unwrap(),
        );
        let saved = run(crate::save_fields(
            vec![audio.to_string_lossy().into_owned()],
            Fields::from([("TITLE".into(), vec!["Saved without capture".into()])]),
            None,
        ));
        assert!(saved.errors.is_empty());
        assert_eq!(title(&audio), ["Saved without capture"]);
        let state = history_state().unwrap();
        assert!(state.undo.is_none() && state.redo.is_none());
        assert!(state
            .notice
            .as_deref()
            .unwrap()
            .contains("cannot be undone"));
        assert_eq!(riff_chunk(&audio, b"data"), pcm);
        assert_eq!(riff_chunk(&audio, b"uNKn"), opaque);
    }

    #[test]
    fn diagnostic_replay_offsets_owned_errors_after_read_summary_failures() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let raw = root.join("Help; raw.txt");
        let raw_to = root.join("Title; moved.txt");
        std::fs::write(&raw, b"Not audio").unwrap();
        let audio = root.join("Help.wav");
        let audio_to = root.join("Title.wav");
        wav(&audio, "Original");
        let original = std::fs::read(&audio).unwrap();
        let mut journal = Journal::open(&root.join("undo"), Limits::default()).unwrap();
        let capture = journal.begin(
            "Help / action 1: Title",
            None,
            &[
                raw.to_string_lossy().into_owned(),
                audio.to_string_lossy().into_owned(),
            ],
        );
        tag_core::operations::rename_no_clobber(&raw, &raw_to).unwrap();
        tag_core::operations::rename_no_clobber(&audio, &audio_to).unwrap();
        journal.commit(
            capture,
            &[
                (
                    raw.to_string_lossy().into_owned(),
                    raw_to.to_string_lossy().into_owned(),
                ),
                (
                    audio.to_string_lossy().into_owned(),
                    audio_to.to_string_lossy().into_owned(),
                ),
            ],
            &[],
        );
        std::fs::write(&audio, b"External replacement").unwrap();
        struct Restore(HistoryAdapter);
        impl Drop for Restore {
            fn drop(&mut self) {
                *JOURNAL.lock().unwrap_or_else(|error| error.into_inner()) =
                    std::mem::take(&mut self.0);
            }
        }
        let _restore = Restore(std::mem::replace(
            &mut *JOURNAL.lock().unwrap(),
            HistoryAdapter {
                journal: Some(journal),
                unavailable: None,
            },
        ));
        let report = tauri::async_runtime::block_on(ipc::undo_last_with_diagnostics()).unwrap();
        assert_eq!(report.result.result.errors.len(), 2);
        assert_eq!(report.error_diagnostics.len(), 1);
        assert_eq!(report.error_diagnostics[0].index, 1);
        assert_eq!(
            report.result.result.errors[1].error,
            report.error_diagnostics[0].diagnostic.to_string()
        );
        let wire = serde_json::to_value(&report).unwrap();
        assert_eq!(wire["error_diagnostics"][0]["index"], 1);
        assert_eq!(
            wire["errors"][1]["error"],
            report.result.result.errors[1].error
        );
        assert_eq!(std::fs::read(&raw).unwrap(), b"Not audio");
        assert_eq!(std::fs::read(&audio).unwrap(), b"External replacement");
        assert_eq!(std::fs::read(&audio_to).unwrap(), original);
    }

    fn check_new_cover_undo(move_folder: bool) {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let album = root.join("Album");
        std::fs::create_dir(&album).unwrap();
        let audio = album.join("a.wav");
        wav(&audio, "Original");
        let mut image = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        let image = image.into_inner();
        let input = root.join("input.png");
        std::fs::write(&input, &image).unwrap();
        tag_core::add_cover(&audio, &input).unwrap();
        let original = std::fs::read(&audio).unwrap();
        let mut actions = vec![tag_core::actions::ActionSpec::ExportCover {
            mta_source: None,
            filename_template: "art/newcover".into(),
            picture_type: None,
            export_duplicates: false,
        }];
        if move_folder {
            actions.extend([
                tag_core::actions::ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "Changed".into(),
                },
                tag_core::actions::ActionSpec::FormatValue {
                    field: "_DIRECTORY".into(),
                    template: "Moved".into(),
                },
            ]);
        }
        let current = if move_folder {
            root.join("Moved")
        } else {
            album.clone()
        };
        let result = run(crate::execute_action_groups(
            vec![audio.to_string_lossy().into_owned()],
            vec![tag_core::actions::ActionGroup {
                name: "Create cover".into(),
                enabled: true,
                actions,
            }],
            None,
        ));
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            std::fs::read(current.join("art/newcover.png")).unwrap(),
            image
        );
        let written = std::fs::read(current.join("a.wav")).unwrap();
        let undone = run(undo_last());
        assert!(
            undone.result.errors.is_empty(),
            "{:?}",
            undone.result.errors
        );
        assert!(!album.join("art/newcover.png").exists());
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        let redone = run(redo_last());
        assert!(
            redone.result.errors.is_empty(),
            "{:?}",
            redone.result.errors
        );
        assert_eq!(
            std::fs::read(current.join("art/newcover.png")).unwrap(),
            image
        );
        assert_eq!(std::fs::read(current.join("a.wav")).unwrap(), written);
        // Remove this test's final undo entry from the shared command journal.
        assert!(run(undo_last()).result.errors.is_empty());
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn action_cover_creation_only_has_undo_and_redo() {
        check_new_cover_undo(false);
    }

    #[test]
    fn action_cover_creation_then_folder_move_has_undo_and_redo() {
        check_new_cover_undo(true);
    }

    #[cfg(unix)]
    #[test]
    fn action_cover_symlink_replacement_is_journaled_through_the_command() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let a = root.join("a.wav");
        let b = root.join("b.wav");
        wav(&a, "Alpha");
        wav(&b, "Beta");
        let image = |width| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            bytes.into_inner()
        };
        let (first, previous) = (image(2), image(3));
        let original = root.join("original.png");
        let first_image = root.join("first.png");
        std::fs::write(&original, &previous).unwrap();
        std::fs::write(&first_image, &first).unwrap();
        tag_core::add_cover(&a, &first_image).unwrap();
        tag_core::add_cover(&b, &original).unwrap();
        let target = root.join("shared.png");
        std::os::unix::fs::symlink("original.png", &target).unwrap();
        let text = |p: &Path| p.to_string_lossy().into_owned();
        let group = tag_core::actions::ActionGroup {
            name: "Cover link".into(),
            enabled: true,
            actions: vec![tag_core::actions::ActionSpec::ExportCover {
                mta_source: None,
                filename_template: "shared".into(),
                picture_type: None,
                export_duplicates: false,
            }],
        };
        let result = run(crate::execute_action_groups(
            vec![text(&a), text(&b)],
            vec![group],
            None,
        ));
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].path.ends_with("b.wav"));
        assert!(result.errors[0]
            .error
            .contains("already written for another file"));
        assert_eq!(std::fs::read(&target).unwrap(), first);
        assert!(!std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&original).unwrap(), previous);
        let undone = run(undo_last());
        assert!(
            undone.result.errors.is_empty(),
            "{:?}",
            undone.result.errors
        );
        assert_eq!(
            std::fs::read_link(&target).unwrap(),
            Path::new("original.png")
        );
        assert_eq!(std::fs::read(&target).unwrap(), previous);
        let redone = run(redo_last());
        assert!(
            redone.result.errors.is_empty(),
            "{:?}",
            redone.result.errors
        );
        assert!(!std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&target).unwrap(), first);
        assert_eq!(std::fs::read(&original).unwrap(), previous);
        // An externally substituted link is a different directory entry,
        // even when it exposes the exact bytes the journal expects.
        std::fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink("first.png", &target).unwrap();
        let refused = run(undo_last());
        assert!(refused
            .result
            .errors
            .iter()
            .any(|error| error.error.contains("Changed outside")));
        assert_eq!(std::fs::read_link(&target).unwrap(), Path::new("first.png"));
        // The app's existing full-alias protection rejects a dangling
        // destination visibly before any cover write or link replacement.
        let dangling = root.join("dangling.png");
        std::os::unix::fs::symlink("missing.png", &dangling).unwrap();
        let result = run(crate::execute_action_groups(
            vec![text(&a)],
            vec![tag_core::actions::ActionGroup {
                name: "Dangling link".into(),
                enabled: true,
                actions: vec![tag_core::actions::ActionSpec::ExportCover {
                    mta_source: None,
                    filename_template: "dangling".into(),
                    picture_type: None,
                    export_duplicates: false,
                }],
            }],
            None,
        ));
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert_eq!(
            std::fs::read_link(&dangling).unwrap(),
            Path::new("missing.png")
        );
        assert!(!root.join("missing.png").exists());
        assert!(!std::fs::read_dir(&root).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".opentag-link-")));
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn action_cover_undo_follows_later_directory_moves_and_keeps_the_first_snapshot() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for (repeat_after_move, start_identical, move_folder) in [
            (false, false, true),
            (true, false, true),
            (true, true, true),
            (true, true, false),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            TEST_ROOTS.lock().unwrap().push(root.clone());
            let album = root.join("Album");
            std::fs::create_dir(&album).unwrap();
            let audio = album.join("a.wav");
            wav(&audio, "Original");
            let image = |width| {
                let mut bytes = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                    .write_to(&mut bytes, image::ImageFormat::Png)
                    .unwrap();
                bytes.into_inner()
            };
            let embedded = image(2);
            let previous = if start_identical {
                embedded.clone()
            } else {
                image(3)
            };
            let later = image(4);
            let input = root.join("input.png");
            let next = root.join("next.png");
            std::fs::write(&input, &embedded).unwrap();
            std::fs::write(&next, &later).unwrap();
            tag_core::add_cover(&audio, &input).unwrap();
            let before_audio = std::fs::read(&audio).unwrap();
            std::fs::write(album.join("shared.png"), &previous).unwrap();
            let export = || tag_core::actions::ActionSpec::ExportCover {
                mta_source: None,
                filename_template: "shared".into(),
                picture_type: None,
                export_duplicates: false,
            };
            let rename = |name: &str| tag_core::actions::ActionSpec::FormatValue {
                field: "_DIRECTORY".into(),
                template: name.into(),
            };
            let mut actions = vec![export()];
            if move_folder {
                actions.push(rename("Renamed"));
            }
            if repeat_after_move {
                actions.extend([
                    tag_core::actions::ActionSpec::ImportCover {
                        mta_source: None,
                        filename_template: next.to_string_lossy().into_owned(),
                        picture_type: None,
                        delete_existing: true,
                    },
                    export(),
                ]);
                if move_folder {
                    actions.push(rename("Final"));
                }
                actions.push(export());
            }
            let final_folder = if move_folder {
                root.join(if repeat_after_move {
                    "Final"
                } else {
                    "Renamed"
                })
            } else {
                album.clone()
            };
            let result = run(crate::execute_action_groups(
                vec![audio.to_string_lossy().into_owned()],
                vec![tag_core::actions::ActionGroup {
                    name: "Move cover folder".into(),
                    enabled: true,
                    actions,
                }],
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let after_image = if repeat_after_move { &later } else { &embedded };
            assert_eq!(
                std::fs::read(final_folder.join("shared.png")).unwrap(),
                *after_image
            );
            let after_audio = std::fs::read(final_folder.join("a.wav")).unwrap();
            if move_folder {
                assert!(!album.exists());
            }
            let undone = run(undo_last());
            assert!(
                undone.result.errors.is_empty(),
                "{:?}",
                undone.result.errors
            );
            assert_eq!(std::fs::read(album.join("shared.png")).unwrap(), previous);
            assert_eq!(std::fs::read(&audio).unwrap(), before_audio);
            if move_folder {
                assert!(!final_folder.exists());
            }
            let redone = run(redo_last());
            assert!(
                redone.result.errors.is_empty(),
                "{:?}",
                redone.result.errors
            );
            assert_eq!(
                std::fs::read(final_folder.join("shared.png")).unwrap(),
                *after_image
            );
            assert_eq!(
                std::fs::read(final_folder.join("a.wav")).unwrap(),
                after_audio
            );
            if move_folder {
                assert!(!album.exists());
            }
            // Leave no undo step in the shared command journal for the next
            // fixture; its first command will clear this final redo step.
            let cleanup = run(undo_last());
            assert!(
                cleanup.result.errors.is_empty(),
                "{:?}",
                cleanup.result.errors
            );
            TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
        }
    }

    #[test]
    fn cover_export_refuses_an_existing_output_before_exceeding_undo_budget() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let audio = root.join("a.wav");
        wav(&audio, "Original");
        let input = root.join("input.png");
        let mut image = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(&input, image.into_inner()).unwrap();
        tag_core::add_cover(&audio, &input).unwrap();
        let before = std::fs::read(&audio).unwrap();
        let output = root.join("cover.png");
        std::fs::write(&output, [7; 64]).unwrap();
        struct RestoreJournal(HistoryAdapter);
        impl Drop for RestoreJournal {
            fn drop(&mut self) {
                *JOURNAL.lock().unwrap_or_else(|e| e.into_inner()) = std::mem::take(&mut self.0);
            }
        }
        let test_journal = Journal::open(
            &root.join("undo"),
            Limits {
                bytes: before.len() as u64 + 32,
                min_free: 0,
                ..Limits::default()
            },
        )
        .unwrap();
        let _restore = RestoreJournal(std::mem::replace(
            &mut *JOURNAL.lock().unwrap(),
            HistoryAdapter {
                journal: Some(test_journal),
                unavailable: None,
            },
        ));
        let result = run(crate::execute_action_groups(
            vec![audio.to_string_lossy().into_owned()],
            vec![tag_core::actions::ActionGroup {
                name: "Limited cover".into(),
                enabled: true,
                actions: vec![tag_core::actions::ActionSpec::ExportCover {
                    mta_source: None,
                    filename_template: "cover".into(),
                    picture_type: None,
                    export_duplicates: false,
                }],
            }],
            None,
        ));
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(
            result.errors[0].error.contains("undo byte limit"),
            "{:?}",
            result.errors
        );
        assert_eq!(std::fs::read(&output).unwrap(), [7; 64]);
        assert_eq!(std::fs::read(&audio).unwrap(), before);
        assert!(history_state().unwrap().undo.is_none());
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn cover_undo_tracks_outputs_created_at_a_reused_folder_path() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for obstruct in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            TEST_ROOTS.lock().unwrap().push(root.clone());
            let album = root.join("Album");
            let moved = root.join("Moved");
            std::fs::create_dir(&album).unwrap();
            let audio = album.join("a.wav");
            wav(&audio, "Original");
            let input = root.join("input.png");
            let mut image = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
                .write_to(&mut image, image::ImageFormat::Png)
                .unwrap();
            let image = image.into_inner();
            std::fs::write(&input, &image).unwrap();
            tag_core::add_cover(&audio, &input).unwrap();
            let before = std::fs::read(&audio).unwrap();
            std::fs::write(album.join("shared.png"), b"Original output").unwrap();
            let export = |name| tag_core::actions::ActionSpec::ExportCover {
                mta_source: None,
                filename_template: name,
                picture_type: None,
                export_duplicates: false,
            };
            let result = run(crate::execute_action_groups(
                vec![audio.to_string_lossy().into_owned()],
                vec![tag_core::actions::ActionGroup {
                    name: "Reuse old folder".into(),
                    enabled: true,
                    actions: vec![
                        export("shared".into()),
                        tag_core::actions::ActionSpec::FormatValue {
                            field: "_DIRECTORY".into(),
                            template: "Moved".into(),
                        },
                        export(album.join("nested/new").to_string_lossy().into_owned()),
                    ],
                }],
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            assert_eq!(std::fs::read(moved.join("shared.png")).unwrap(), image);
            assert_eq!(std::fs::read(album.join("nested/new.png")).unwrap(), image);
            if obstruct {
                std::fs::write(album.join("external.txt"), b"External data").unwrap();
            }
            let undone = run(undo_last());
            if obstruct {
                assert!(!undone.result.errors.is_empty());
                assert_eq!(
                    std::fs::read(album.join("external.txt")).unwrap(),
                    b"External data"
                );
                assert_eq!(
                    std::fs::read(album.join("nested/new.png")).unwrap(),
                    image,
                    "rolled back side-file removal"
                );
                assert_eq!(std::fs::read(moved.join("shared.png")).unwrap(), image);
                assert_eq!(std::fs::read(moved.join("a.wav")).unwrap(), before);
            } else {
                assert!(
                    undone.result.errors.is_empty(),
                    "{:?}",
                    undone.result.errors
                );
                assert_eq!(
                    std::fs::read(album.join("shared.png")).unwrap(),
                    b"Original output"
                );
                assert_eq!(std::fs::read(&audio).unwrap(), before);
                assert!(!album.join("nested/new.png").exists() && !moved.exists());
                let redone = run(redo_last());
                assert!(
                    redone.result.errors.is_empty(),
                    "{:?}",
                    redone.result.errors
                );
                assert_eq!(std::fs::read(moved.join("shared.png")).unwrap(), image);
                assert_eq!(std::fs::read(album.join("nested/new.png")).unwrap(), image);
                assert_eq!(std::fs::read(moved.join("a.wav")).unwrap(), before);
                assert!(run(undo_last()).result.errors.is_empty());
            }
            TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
        }
    }

    #[test]
    fn tag_to_tag_metadata_outputs_follow_preview_contract() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let audio = root.join("preview.wav");
        wav(&audio, "Unchanged");
        let path = audio.to_string_lossy().into_owned();
        let separator = char::from(92).to_string().repeat(2);
        let cases = [
            (separator.clone(), vec![]),
            (
                format!(" Jazz {separator} {separator} Blues{separator}"),
                vec!["Jazz", "Blues"],
            ),
            (" Live ".into(), vec![" Live "]),
            (format!("AC{}DC", char::from(92)), vec![r"AC\DC"]),
            (
                format!("{0}Jazz{0}{separator}{0}", char::from(0x85)),
                vec!["Jazz"],
            ),
            (
                format!("{0}Jazz{0}{separator}", char::from_u32(0xfeff).unwrap()),
                vec!["\u{feff}Jazz\u{feff}"],
            ),
        ];
        for (template, expected) in cases {
            tag_core::write_fields(
                &audio,
                &Fields::from([("GENRE".into(), vec!["Previous".into()])]),
            )
            .unwrap();
            let before = std::fs::read(&audio).unwrap();
            let preview = run(crate::preview_format(
                vec![path.clone()],
                template.clone(),
                None,
                Some(false),
            ));
            assert_eq!(preview[0].value, template);
            let result = run(convert_tag_to_tag(
                vec![path.clone()],
                "GENRE".into(),
                template.clone(),
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let fields = tag_core::read_entry_summary(&audio).unwrap().fields;
            assert_eq!(
                fields.get("GENRE").map(Vec::as_slice).unwrap_or(&[]),
                expected,
                "{template:?}"
            );
            assert_eq!(fields["TITLE"], ["Unchanged"]);
            let undo = run(undo_last());
            assert!(undo.result.errors.is_empty(), "{:?}", undo.result.errors);
            assert_eq!(std::fs::read(&audio).unwrap(), before);
        }
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn commands_record_grouped_saves_tag_conversions_and_renames() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let a = root.join("a.wav");
        let b = root.join("b.wav");
        wav(&a, "Alpha");
        wav(&b, "Beta");
        let key = |p: &Path| p.to_string_lossy().into_owned();
        let original = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

        // One Save click writes files one at a time but is one undo step.
        for (path, value) in [(&a, "One"), (&b, "Two")] {
            let saved = run(crate::save_fields(
                vec![key(path)],
                Fields::from([("TITLE".into(), vec![value.into()])]),
                Some("save-click".into()),
            ));
            assert!(saved.errors.is_empty());
        }
        assert_eq!(step(), ("Save tags".into(), 2));

        let both = vec![key(&a), key(&b)];
        let formatted = run(convert_tag_to_tag(
            both.clone(),
            "ALBUM".into(),
            "$upper(%title%) album".into(),
            None,
        ));
        assert!(formatted.errors.is_empty(), "{:?}", formatted.errors);
        assert_eq!(
            tag_core::read_entry_summary(&a).unwrap().fields["ALBUM"],
            ["ONE album"]
        );
        assert_eq!(step(), ("Tag → Tag".into(), 2));

        let renamed = run(convert_tag_to_tag(
            vec![key(&a)],
            "_FILENAME".into(),
            "%title% renamed".into(),
            None,
        ));
        let moved = root.join("One renamed.wav");
        assert_eq!(renamed.renamed[0].to, key(&moved));
        assert!(moved.exists() && !a.exists());

        let undone = run(undo_last());
        assert_eq!(undone.label, "Tag → Tag");
        assert_eq!(undone.renamed[0].from, key(&moved));
        assert_eq!(undone.renamed[0].to, key(&a));
        assert_eq!(undone.result.entries[0].path, key(&a));
        assert!(a.exists() && !moved.exists());
        run(undo_last());
        assert!(!tag_core::read_entry_summary(&a)
            .unwrap()
            .fields
            .contains_key("ALBUM"));
        let undone = run(undo_last());
        assert_eq!(undone.label, "Save tags");
        assert_eq!(undone.result.entries.len(), 2);
        assert_eq!(
            (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap()),
            original
        );
        assert!(undone.history.undo.is_none());
        assert_eq!(undone.history.redo.unwrap().label, "Save tags");

        for _ in 0..3 {
            let redone = run(redo_last());
            assert!(
                redone.result.errors.is_empty(),
                "{:?}",
                redone.result.errors
            );
        }
        assert!(moved.exists());
        assert_eq!(title(&moved), ["One"]);
        assert_eq!(title(&b), ["Two"]);

        // Tag → Filename renames are reversed; external edits are reported.
        let preview = run(crate::preview_rename(vec![key(&b)], "Second".into(), None));
        run(crate::rename_files(
            vec![key(&b)],
            "Second".into(),
            preview[0].plan_token.clone(),
        ));
        let second = root.join("Second.wav");
        assert_eq!(step(), ("Tag → Filename".into(), 1));
        tag_core::write_fields(
            &second,
            &Fields::from([("TITLE".into(), vec!["Else".into()])]),
        )
        .unwrap();
        let refused = run(undo_last());
        assert!(refused.result.errors[0].error.contains("Changed outside"));
        assert!(second.exists() && !b.exists());
        assert_eq!(title(&second), ["Else"]);
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn filename_to_tag_skips_files_that_do_not_match_in_one_undo_step() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let key = |p: &Path| p.to_string_lossy().into_owned();
        let a = root.join("Ada - One.wav");
        let odd = root.join("no separator.wav");
        let b = root.join("Bob - Two.wav");
        for path in [&a, &odd, &b] {
            wav(path, "Old");
        }
        let skipped = std::fs::read(&odd).unwrap();
        let originals = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        let paths = vec![key(&a), key(&odd), key(&b)];

        let preview = run(crate::preview_filename(
            paths.clone(),
            "%artist% - %title%".into(),
        ));
        assert_eq!(preview.len(), 3);
        assert_eq!(preview[0].fields.as_ref().unwrap()["ARTIST"], ["Ada"]);
        assert!(preview[1].fields.is_none() && preview[1].error.is_none());
        assert_eq!(preview[2].fields.as_ref().unwrap()["TITLE"], ["Two"]);

        let imported = run(crate::filename_to_tags(paths, "%artist% - %title%".into()));
        assert!(
            imported.result.errors.is_empty(),
            "{:?}",
            imported.result.errors
        );
        assert_eq!(imported.unmatched, [key(&odd)]);
        let written: Vec<_> = imported.result.entries.iter().map(|e| &e.path).collect();
        assert_eq!(written, [&key(&a), &key(&b)]);
        assert_eq!(title(&a), ["One"]);
        assert_eq!(title(&b), ["Two"]);
        assert_eq!(std::fs::read(&odd).unwrap(), skipped);
        assert_eq!(step(), ("Filename → Tag".into(), 2));

        let undone = run(undo_last());
        assert_eq!(undone.label, "Filename → Tag");
        assert_eq!(
            (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap()),
            originals
        );
        // A pattern error is still an error for the whole conversion.
        assert!(tauri::async_runtime::block_on(crate::filename_to_tags(
            vec![key(&a)],
            "%artist%%title%".into()
        ))
        .is_err());
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }
    #[test]
    fn filename_conversion_companions_share_one_history_step_and_refuse_external_edits() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let fixture = dunce::canonicalize(dir.path()).unwrap();
        let _isolated = IsolatedHistory::new(&fixture);
        // Keep journal writes outside the reviewed destination ancestor, as
        // the application's cache is outside an ordinary music folder.
        let root = fixture.join("Music");
        std::fs::create_dir(&root).unwrap();
        let audio = root.join("Artist - Song.wav");
        wav(&audio, "Title");
        let original = std::fs::read(&audio).unwrap();
        let lyric = audio.with_extension("lrc");
        let karaoke = audio.with_extension("CDG");
        std::fs::write(&lyric, b"lyric").unwrap();
        std::fs::write(&karaoke, b"karaoke").unwrap();
        let request = crate::converters::FilenameRequest {
            paths: vec![audio.to_string_lossy().into_owned()],
            old_pattern: "%1 - %2".into(),
            new_pattern: "Output\\%2 - %1".into(),
            columns: None,
        };
        let preview = crate::converters::preview_filename_policy(
            &request,
            &crate::evaluation::EvaluationSettings::default(),
            true,
        )
        .unwrap();
        let captured =
            crate::converters::filename_history_sources(&request, &preview.plan_token, true)
                .unwrap();
        assert_eq!(captured.len(), 3);
        assert!(
            captured.contains(&lyric.to_string_lossy().into_owned())
                && captured.contains(&karaoke.to_string_lossy().into_owned())
        );
        let result = record("Filename → Filename", None, &captured, || {
            crate::converters::apply_filename_policy(
                &request,
                &preview.plan_token,
                true,
                |_| Ok(()),
            )
            .map_err(|error| error.to_string())
        })
        .unwrap();
        let target = std::path::PathBuf::from(&result.renamed[0].to);
        assert_eq!(result.associated_renamed.len(), 2);
        assert_eq!(result.result.entries.len(), 1);
        assert_eq!(
            history_state().unwrap().undo.unwrap().label,
            "Filename → Filename"
        );
        let undone = run(undo_last());
        assert!(
            undone.result.errors.is_empty(),
            "{:?}",
            undone.result.errors
        );
        assert_eq!(undone.result.entries.len(), 1);
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        assert_eq!(std::fs::read(&lyric).unwrap(), b"lyric");
        assert_eq!(std::fs::read(&karaoke).unwrap(), b"karaoke");
        assert!(!target.exists());
        let redone = run(redo_last());
        assert!(
            redone.result.errors.is_empty(),
            "{:?}",
            redone.result.errors
        );
        assert_eq!(redone.result.entries.len(), 1);
        assert_eq!(std::fs::read(&target).unwrap(), original);
        assert_eq!(
            std::fs::read(target.with_extension("lrc")).unwrap(),
            b"lyric"
        );
        assert_eq!(
            std::fs::read(target.with_extension("CDG")).unwrap(),
            b"karaoke"
        );
        assert!(!audio.exists());
        std::fs::write(target.with_extension("lrc"), b"External lyric edit").unwrap();
        let refused = run(undo_last());
        assert!(!refused.result.errors.is_empty());
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        assert_eq!(
            std::fs::read(target.with_extension("lrc")).unwrap(),
            b"External lyric edit"
        );
        assert!(!lyric.exists());
        assert_eq!(std::fs::read(&karaoke).unwrap(), b"karaoke");
    }

    #[test]
    fn action_companions_history_tracks_filename_directory_epochs_and_cover_outputs() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let _isolated = IsolatedHistory::new(&root);
        let album = root.join("Album");
        std::fs::create_dir(&album).unwrap();
        let audio = album.join("old.wav");
        wav(&audio, "Title");
        let lyric = audio.with_extension("lrc");
        std::fs::write(&lyric, b"lyric").unwrap();
        let image_path = root.join("input.png");
        let mut image = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        let image = image.into_inner();
        std::fs::write(&image_path, &image).unwrap();
        tag_core::add_cover(&audio, &image_path).unwrap();
        let original = std::fs::read(&audio).unwrap();
        let format = |field: &str, template: &str| tag_core::actions::ActionSpec::FormatValue {
            field: field.into(),
            template: template.into(),
        };
        let groups = [tag_core::actions::ActionGroup {
            name: "Companion epochs".into(),
            enabled: true,
            actions: vec![
                format("_FILENAME", "middle"),
                tag_core::actions::ActionSpec::ExportCover {
                    mta_source: None,
                    filename_template: "cover".into(),
                    picture_type: None,
                    export_duplicates: false,
                },
                format("_DIRECTORY", "Moved"),
                format("_FILENAME", "final"),
                tag_core::actions::ActionSpec::ExportCover {
                    mta_source: None,
                    filename_template: "later".into(),
                    picture_type: None,
                    export_duplicates: false,
                },
            ],
        }];
        let paths = vec![audio.to_string_lossy().into_owned()];
        let prepared = crate::action_companions::Prepared::prepare(&paths, &groups, true).unwrap();
        let captured = prepared.history_sources(&paths);
        assert!(captured.contains(&lyric.to_string_lossy().into_owned()));
        let result = record_with_side_file_epochs_diagnostic(
            "Companion epochs",
            None,
            &captured,
            |capture| {
                Ok(crate::action_host::execute_prepared(
                    &paths,
                    &groups,
                    crate::evaluation::EvaluationSettings::default(),
                    prepared,
                    capture,
                ))
            },
        )
        .unwrap();
        assert!(
            result.result.errors.is_empty(),
            "{:?}",
            result.result.errors
        );
        assert_eq!(result.result.associated_renamed.len(), 1);
        let moved = root.join("Moved");
        assert_eq!(std::fs::read(moved.join("final.wav")).unwrap(), original);
        assert_eq!(std::fs::read(moved.join("final.lrc")).unwrap(), b"lyric");
        assert_eq!(std::fs::read(moved.join("cover.png")).unwrap(), image);
        assert_eq!(std::fs::read(moved.join("later.png")).unwrap(), image);
        let undone = run(undo_last());
        assert!(
            undone.result.errors.is_empty(),
            "{:?}",
            undone.result.errors
        );
        assert_eq!(undone.result.entries.len(), 1);
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        assert_eq!(std::fs::read(&lyric).unwrap(), b"lyric");
        assert!(
            !moved.exists()
                && !album.join("cover.png").exists()
                && !album.join("later.png").exists()
        );
        let redone = run(redo_last());
        assert!(
            redone.result.errors.is_empty(),
            "{:?}",
            redone.result.errors
        );
        assert_eq!(std::fs::read(moved.join("final.lrc")).unwrap(), b"lyric");
        assert_eq!(std::fs::read(moved.join("final.wav")).unwrap(), original);
        assert_eq!(std::fs::read(moved.join("later.png")).unwrap(), image);
    }
    #[test]
    fn companion_moves_and_case_only_renames_undo_without_audio_parse_errors() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let key = |p: &Path| p.to_string_lossy().into_owned();
        for destination in [root.join("SONG.wav"), root.join("Moved/renamed.wav")] {
            let audio = root.join("Song.wav");
            let lyric = root.join("Song.lrc");
            wav(&audio, "Title");
            std::fs::write(&lyric, b"lyrics").unwrap();
            let original = std::fs::read(&audio).unwrap();
            let plan =
                crate::associated_files::Plan::new(vec![(key(&audio), destination.clone())], true)
                    .unwrap();
            let group = &plan.groups[0];
            let moved_lyric = group.associated[0].target.clone();
            let targets = vec![key(&audio), key(&lyric)];
            let result = record("Move files", None, &targets, || {
                group.execute(&mut tag_core::operations::rename_no_clobber)?;
                Ok(crate::file_ops::TransferResult {
                    result: ScanResult {
                        entries: vec![],
                        errors: vec![],
                    },
                    copied: vec![],
                    renamed: vec![crate::file_ops::PathChange {
                        from: key(&audio),
                        to: key(&destination),
                    }],
                    associated_renamed: vec![crate::file_ops::PathChange {
                        from: key(&lyric),
                        to: key(&moved_lyric),
                    }],
                })
            })
            .unwrap();
            assert_eq!(result.associated_renamed.len(), 1);
            let undone = run(undo_last());
            assert!(
                undone.result.errors.is_empty(),
                "{:?}",
                undone.result.errors
            );
            assert_eq!(undone.result.entries.len(), 1);
            assert_eq!(std::fs::read(&audio).unwrap(), original);
            assert_eq!(std::fs::read(&lyric).unwrap(), b"lyrics");
            let names: Vec<_> = std::fs::read_dir(&root)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert!(names.contains(&std::ffi::OsString::from("Song.lrc")));
            let redone = run(redo_last());
            assert!(
                redone.result.errors.is_empty(),
                "{:?}",
                redone.result.errors
            );
            assert_eq!(std::fs::read(&moved_lyric).unwrap(), b"lyrics");
            // A companion edited elsewhere must be retained, even if the audio can undo.
            std::fs::write(&moved_lyric, b"external lyric edit").unwrap();
            let refused = run(undo_last());
            assert!(refused
                .result
                .errors
                .iter()
                .any(|e| e.error.contains("Changed outside")));
            assert_eq!(std::fs::read(&moved_lyric).unwrap(), b"external lyric edit");
            std::fs::remove_file(&audio).unwrap();
            std::fs::remove_file(&moved_lyric).unwrap();
        }
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }
}
