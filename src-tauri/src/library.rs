//! Options → Library (https://docs.mp3tag.de/customization/options/library/):
//! the page's values, the database in `<config>/library/library.db`, loading
//! folders through it, and keeping it current when OpenTag changes files.
//! The database is not part of a configuration backup (utilities.rs lists
//! the files that are).
use opentag_library::{fingerprint, CleanupReport, Library, Scope, Session};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tag_core::{ScanResult, TagOptions};
use tauri::Manager;

/// `pages.library` of the settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LibraryOptions {
    /// Enable Library.
    pub enabled: bool,
    /// Only include these directories.
    pub restrict: bool,
    pub directories: Vec<String>,
}
impl LibraryOptions {
    pub fn scope(&self) -> Scope {
        if self.restrict {
            Scope::only(&self.directories)
        } else {
            Scope::everything()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.directories.iter().any(|d| d.trim().is_empty()) {
            return Err("Library: included directories must not be empty".into());
        }
        Ok(())
    }
}

pub fn options() -> LibraryOptions {
    crate::settings::current()
        .pages
        .get("library")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

static LIBRARY: Mutex<Option<Arc<Library>>> = Mutex::new(None);
/// Files the last load took from the library and read and stored.
static LAST_HITS: AtomicUsize = AtomicUsize::new(0);
static LAST_STORED: AtomicUsize = AtomicUsize::new(0);

fn database_path() -> Option<PathBuf> {
    let app = crate::APP_HANDLE.get()?;
    Some(
        app.path()
            .app_config_dir()
            .ok()?
            .join("library")
            .join("library.db"),
    )
}

/// The open library. Without `create`, a library that was never created
/// stays absent.
fn library(create: bool) -> Result<Option<Arc<Library>>, String> {
    let mut slot = LIBRARY.lock().map_err(|_| "Library lock failed")?;
    if let Some(library) = slot.as_ref() {
        return Ok(Some(library.clone()));
    }
    let Some(path) = database_path() else {
        return Ok(None);
    };
    if !create && !path.exists() {
        return Ok(None);
    }
    let library = Arc::new(Library::open(&path)?);
    *slot = Some(library.clone());
    Ok(Some(library))
}

/// Read ordinary files and folders, taking unchanged files from the library
/// when it is enabled. `refresh` (File → Reload tags) reads every file and
/// replaces the stored entries.
pub fn scan(
    paths: &[String],
    recursive: bool,
    tags: &TagOptions,
    extensions: Option<&[String]>,
    progress: &dyn tag_core::parallel::Progress,
    refresh: bool,
) -> ScanResult {
    let settings = options();
    let library = if settings.enabled {
        library(true).ok().flatten()
    } else {
        None
    };
    scan_with(
        library.as_deref(),
        &settings,
        paths,
        recursive,
        tags,
        extensions,
        progress,
        refresh,
    )
}

#[allow(clippy::too_many_arguments)]
fn scan_with(
    library: Option<&Library>,
    settings: &LibraryOptions,
    paths: &[String],
    recursive: bool,
    tags: &TagOptions,
    extensions: Option<&[String]>,
    progress: &dyn tag_core::parallel::Progress,
    refresh: bool,
) -> ScanResult {
    let Some(library) = library else {
        return tag_core::scan_paths_with_progress(paths, recursive, tags, extensions, progress);
    };
    let scope = settings.scope();
    let session = Session::new(
        library,
        &scope,
        fingerprint(tags, env!("CARGO_PKG_VERSION")),
        refresh,
    );
    let result =
        tag_core::scan_paths_cached(paths, recursive, tags, extensions, progress, Some(&session));
    LAST_HITS.store(session.hits(), Ordering::Relaxed);
    LAST_STORED.store(session.stored(), Ordering::Relaxed);
    result
}

/// The file of a path that may name a chapter or cue sheet row.
fn file_of(path: &str) -> &str {
    tag_core::subsong_parts(path).map_or(path, |(file, _)| file)
}

/// Forget files that OpenTag wrote, renamed, moved or deleted, and folders
/// it moved, so the next load reads them again. Runs whether or not the
/// library is enabled: it may be enabled again later.
pub fn changed(paths: &[String], renamed: &[(String, String)], directories: &[(String, String)]) {
    let Ok(Some(library)) = library(false) else {
        return;
    };
    let mut files: Vec<&str> = paths.iter().map(|p| file_of(p)).collect();
    for (from, to) in renamed {
        files.push(file_of(from));
        files.push(file_of(to));
    }
    let _ = library.forget(&files);
    for (from, to) in directories {
        let _ = library.forget_below(from);
        let _ = library.forget_below(to);
    }
}

#[derive(Debug, Serialize)]
pub struct LibraryStatus {
    pub enabled: bool,
    /// Whether the database exists.
    pub created: bool,
    pub files: usize,
    pub bytes: u64,
    pub location: Option<String>,
    /// Files the last folder load took from the library, and read and stored.
    pub last_hits: usize,
    pub last_stored: usize,
}

fn status() -> Result<LibraryStatus, String> {
    let stats = match library(false)? {
        Some(library) => Some(library.stats()?),
        None => None,
    };
    Ok(LibraryStatus {
        enabled: options().enabled,
        created: stats.is_some(),
        files: stats.as_ref().map_or(0, |s| s.files),
        bytes: stats.as_ref().map_or(0, |s| s.bytes),
        location: database_path().map(|p| p.to_string_lossy().into_owned()),
        last_hits: LAST_HITS.load(Ordering::Relaxed),
        last_stored: LAST_STORED.load(Ordering::Relaxed),
    })
}

// Library commands do not take the file operation lock: they only read file
// metadata, and a cleanup must not hold up loading or saving files.
async fn background<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn library_status() -> Result<LibraryStatus, String> {
    background(status).await
}

/// Cleanup Library with the directory choice shown in the Options dialog.
#[tauri::command]
pub async fn cleanup_library(
    restrict: bool,
    directories: Vec<String>,
    job: Option<String>,
) -> Result<CleanupReport, String> {
    background(move || {
        let job = crate::progress::Job::new(job);
        let Some(library) = library(false)? else {
            return Ok(CleanupReport::default());
        };
        let settings = LibraryOptions {
            enabled: true,
            restrict,
            directories,
        };
        library.cleanup(&settings.scope(), &job)
    })
    .await
}

/// Start with a fresh library.
#[tauri::command]
pub async fn clear_library() -> Result<LibraryStatus, String> {
    background(|| {
        if let Some(library) = library(false)? {
            library.clear()?;
        }
        LAST_HITS.store(0, Ordering::Relaxed);
        LAST_STORED.store(0, Ordering::Relaxed);
        status()
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tag_core::parallel::Silent;

    fn wav(path: &Path) {
        let mut b = b"RIFF".to_vec();
        b.extend(100u32.to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(8000u32.to_le_bytes());
        b.extend(16000u32.to_le_bytes());
        b.extend(2u16.to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(64u32.to_le_bytes());
        b.extend([0u8; 64]);
        fs::write(path, b).unwrap();
    }

    #[test]
    fn options_default_to_disabled_and_reject_empty_directories() {
        let parsed: LibraryOptions = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(parsed, LibraryOptions::default());
        assert!(!parsed.enabled);
        let bad = LibraryOptions {
            directories: vec![" ".into()],
            ..Default::default()
        };
        assert!(bad.validate().is_err());
        assert!(
            serde_json::from_value::<LibraryOptions>(serde_json::json!({"enabled": "yes"}))
                .is_err()
        );
    }

    #[test]
    fn configuration_backups_leave_the_library_out() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("settings.json"), "{}").unwrap();
        Library::open(&dir.path().join("library").join("library.db")).unwrap();
        let bundle = crate::utilities::collect(dir.path()).unwrap();
        assert_eq!(bundle.files.keys().collect::<Vec<_>>(), ["settings.json"]);
    }

    // One test: the load counters and the open library are process-wide.
    #[test]
    fn loads_use_the_library_and_changed_files_are_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let (a, b) = (root.join("a.wav"), root.join("b.wav"));
        wav(&a);
        wav(&b);
        let library = Library::in_memory().unwrap();
        let settings = LibraryOptions {
            enabled: true,
            ..Default::default()
        };
        let tags = TagOptions::default();
        let roots = [root.to_string_lossy().into_owned()];
        let load = |refresh| {
            scan_with(
                Some(&library),
                &settings,
                &roots,
                false,
                &tags,
                None,
                &Silent,
                refresh,
            )
        };
        assert_eq!(load(false).entries.len(), 2);
        assert_eq!(library.stats().unwrap().files, 2);
        load(false);
        assert_eq!(LAST_HITS.load(Ordering::Relaxed), 2);
        load(true);
        assert_eq!(LAST_HITS.load(Ordering::Relaxed), 0);
        assert_eq!(LAST_STORED.load(Ordering::Relaxed), 2);

        // Files changed through history and file operations.
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let files: Vec<PathBuf> = ["a", "b", "c", "d"]
            .iter()
            .map(|name| root.join("album").join(format!("{name}.wav")))
            .collect();
        fs::create_dir_all(root.join("album")).unwrap();
        files.iter().for_each(|f| wav(f));
        let library = Arc::new(Library::in_memory().unwrap());
        *LIBRARY.lock().unwrap() = Some(library.clone());
        let roots = [root.to_string_lossy().into_owned()];
        let fill = || {
            scan_with(
                Some(&library),
                &settings,
                &roots,
                true,
                &tags,
                None,
                &Silent,
                false,
            );
        };
        fill();
        assert_eq!(library.stats().unwrap().files, 4);
        let text = |p: &Path| p.to_string_lossy().into_owned();
        // A save of a chapter row forgets its file.
        let row = tag_core::subsong_path(&text(&files[0]), 1);
        crate::history::record("Save tags", None, &[row], || Ok(ScanResult::default())).unwrap();
        assert_eq!(library.stats().unwrap().files, 3);
        // Even a failed operation may have written some files.
        let _ = crate::history::record("Save tags", None, &[text(&files[1])], || {
            Err::<ScanResult, _>("failed".into())
        });
        assert_eq!(library.stats().unwrap().files, 2);
        fill();
        changed(
            &[],
            &[(text(&files[2]), text(&root.join("moved.wav")))],
            &[],
        );
        assert_eq!(library.stats().unwrap().files, 3);
        changed(
            &[],
            &[],
            &[(text(&root.join("album")), text(&root.join("renamed")))],
        );
        assert_eq!(library.stats().unwrap().files, 0);
        *LIBRARY.lock().unwrap() = None;
    }
}
