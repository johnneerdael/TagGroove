//! Session undo and redo for commands that write audio files. Each mutating
//! command runs through [`record`], which snapshots its local targets in the
//! portable journal before the work and records the outcome afterwards.
use crate::action_host::{ActionResult, PathChange};
use serde::Serialize;
use std::{path::Path, sync::Mutex};
use tag_core::history::{HistoryState, Journal, Limits};
use tag_core::ScanResult;
use tauri::Manager;

static JOURNAL: Mutex<Option<Journal>> = Mutex::new(None);

fn with_journal<T>(work: impl FnOnce(&mut Journal) -> T) -> Result<T, String> {
    let mut guard = JOURNAL
        .lock()
        .map_err(|_| "Undo history lock failed".to_string())?;
    if guard.is_none() {
        let root = match super::APP_HANDLE.get() {
            Some(app) => app
                .path()
                .app_cache_dir()
                .map_err(|e| e.to_string())?
                .join("undo"),
            None => std::env::temp_dir().join("opentag-undo"),
        };
        *guard = Some(Journal::open(&root, Limits::default())?);
    }
    Ok(work(guard.as_mut().expect("journal was opened")))
}

/// Path changes reported by a command, used to find each file afterwards.
pub trait Changes {
    fn renamed(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    fn directories(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}
impl Changes for ScanResult {}
impl Changes for super::RenameResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl Changes for crate::converters::FilenameResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl Changes for ActionResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
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
static TEST_ROOTS: Mutex<Vec<std::path::PathBuf>> = Mutex::new(Vec::new());

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
    let local: Vec<String> = paths.iter().filter(|p| journaled(p)).cloned().collect();
    if local.is_empty() {
        return work();
    }
    let capture = with_journal(|journal| journal.begin(label, group, &local));
    let result = work();
    if let Ok(capture) = capture {
        let (renamed, directories) = result
            .as_ref()
            .map(|r| (r.renamed(), r.directories()))
            .unwrap_or_default();
        let _ = with_journal(|journal| journal.commit(capture, &renamed, &directories));
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
    with_journal(|journal| journal.state())
}

fn replay(backward: bool) -> Result<HistoryResult, String> {
    let replay = with_journal(|journal| {
        if backward {
            journal.undo()
        } else {
            journal.redo()
        }
    })??;
    let mut result = super::batch(&replay.paths, tag_core::read_entry_summary);
    result.errors.extend(replay.errors);
    let changes = |pairs: Vec<(String, String)>| {
        pairs
            .into_iter()
            .map(|(from, to)| PathChange { from, to })
            .collect()
    };
    Ok(HistoryResult {
        result,
        renamed: changes(replay.renamed),
        moved_directories: changes(replay.directories),
        label: replay.label,
        history: with_journal(|journal| journal.state())?,
    })
}

#[tauri::command]
pub async fn undo_last() -> Result<HistoryResult, String> {
    super::io(|| replay(true)).await
}

#[tauri::command]
pub async fn redo_last() -> Result<HistoryResult, String> {
    super::io(|| replay(false)).await
}

/// Convert → Tag - Tag: format one field (or `_FILENAME`/`_DIRECTORY`) of each
/// selected file from a format string, written immediately like Mp3tag.
#[tauri::command]
pub async fn convert_tag_to_tag(
    paths: Vec<String>,
    field: String,
    template: String,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<ActionResult, String> {
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
        record("Tag → Tag", None, &paths, || {
            Ok(crate::action_host::execute(&paths, &[group], settings))
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::Fields;

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

    #[test]
    fn commands_record_grouped_saves_tag_conversions_and_renames() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
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
}
