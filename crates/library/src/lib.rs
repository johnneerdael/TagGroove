//! Options → Library (https://docs.mp3tag.de/customization/options/library/).
//!
//! Metadata read from files is kept in an SQLite database in the configuration
//! folder. A later read of the same file takes the stored entry while the
//! file's modification time and size are unchanged, so large folders load
//! without parsing every file again. Cover art is not stored: list entries
//! only carry image descriptors, and image data is read from the file when it
//! is shown.
//!
//! SQLite (bundled, so every platform gets the same engine) gives atomic
//! batched writes, crash safety through its write-ahead log, indexed lookups by
//! path and cheap range deletes when a folder moves. Mp3tag keeps its library
//! in SQLite as well.
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tag_core::{
    parallel::{map_ordered, Progress, Silent},
    AudioEntry, EntryCache, Fields, TagOptions,
};

/// Schema of the database file; other versions are discarded and rebuilt.
const SCHEMA: i64 = 1;
/// Changes when stored entries no longer match what the reader produces.
const ENTRY_FORMAT: &str = "1";
/// Rows written or deleted per statement batch.
const CHUNK: usize = 500;

/// When a file was last seen: its modification time in nanoseconds since the
/// Unix epoch (negative before it) and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub modified: i64,
    pub size: i64,
}
impl Stamp {
    /// The stamp of a regular file, or `None` when it cannot be read.
    pub fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        let modified = metadata.modified().ok()?;
        let modified = match modified.duration_since(UNIX_EPOCH) {
            Ok(after) => i64::try_from(after.as_nanos()).ok()?,
            Err(before) => -i64::try_from(before.duration().as_nanos()).ok()?,
        };
        Some(Self {
            modified,
            size: i64::try_from(metadata.len()).ok()?,
        })
    }
}

/// Identifies the read options and reader that produced an entry. Entries
/// stored under other options are read again.
pub fn fingerprint(options: &TagOptions, reader: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(ENTRY_FORMAT);
    hash.update([0]);
    hash.update(reader);
    hash.update([0]);
    hash.update(serde_json::to_vec(options).unwrap_or_default());
    hash.finalize()[..12]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Options → Library → Only include these directories. Files elsewhere are
/// read from disk every time and never stored.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    directories: Option<Vec<PathBuf>>,
}
impl Scope {
    /// Every file belongs to the library.
    pub fn everything() -> Self {
        Self { directories: None }
    }
    /// Only files below one of `directories` (an empty list includes nothing).
    pub fn only<S: AsRef<str>>(directories: &[S]) -> Self {
        let directories = directories
            .iter()
            .map(|d| d.as_ref().trim())
            .filter(|d| !d.is_empty())
            .map(|d| dunce::canonicalize(d).unwrap_or_else(|_| PathBuf::from(d)))
            .collect();
        Self {
            directories: Some(directories),
        }
    }
    pub fn contains(&self, path: &Path) -> bool {
        match &self.directories {
            None => true,
            Some(directories) => directories.iter().any(|d| inside(path, d)),
        }
    }
}

/// Whether `path` is `directory` or below it. Windows and macOS file systems
/// usually ignore case, so names are compared without it there.
fn inside(path: &Path, directory: &Path) -> bool {
    let mut parts = path.components();
    directory.components().all(|expected| {
        parts.next().is_some_and(|part| {
            let (a, b) = (part.as_os_str(), expected.as_os_str());
            if cfg!(any(windows, target_os = "macos")) {
                a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
            } else {
                a == b
            }
        })
    })
}

/// Outcome of Cleanup Library.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CleanupReport {
    /// Entries checked before the cleanup ended.
    pub checked: usize,
    /// Entries removed: missing files, and with a directory restriction files
    /// outside the included directories.
    pub removed: usize,
    pub cancelled: bool,
}

/// Size of the library.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Stats {
    pub files: usize,
    /// Bytes on disk, including the write-ahead log.
    pub bytes: u64,
}

pub struct Library {
    path: Option<PathBuf>,
    connection: Mutex<Connection>,
}

fn error(e: rusqlite::Error) -> String {
    format!("Library database: {e}")
}

fn initialise(connection: &Connection, file: bool) -> rusqlite::Result<()> {
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    if file {
        connection.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
    }
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version != SCHEMA {
        connection.execute_batch("DROP TABLE IF EXISTS files;")?;
    }
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (
             path TEXT PRIMARY KEY NOT NULL,
             modified INTEGER NOT NULL,
             size INTEGER NOT NULL,
             options TEXT NOT NULL,
             entry BLOB NOT NULL
         );",
    )?;
    connection.pragma_update(None, "user_version", SCHEMA)
}

impl Library {
    /// Open or create the database at `path`. A file that is not a usable
    /// library database is replaced: it only holds data read from files.
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Library folder: {e}"))?;
        }
        let attempt = || -> rusqlite::Result<Connection> {
            let connection = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            initialise(&connection, true)?;
            Ok(connection)
        };
        let connection = match attempt() {
            Ok(connection) => connection,
            Err(rusqlite::Error::SqliteFailure(failure, _))
                if matches!(
                    failure.code,
                    rusqlite::ErrorCode::NotADatabase | rusqlite::ErrorCode::DatabaseCorrupt
                ) =>
            {
                for file in Self::files_of(path) {
                    let _ = fs::remove_file(file);
                }
                attempt().map_err(error)?
            }
            Err(e) => return Err(error(e)),
        };
        Ok(Self {
            path: Some(path.to_path_buf()),
            connection: Mutex::new(connection),
        })
    }

    /// A library that lives only as long as the value (tests).
    pub fn in_memory() -> Result<Self, String> {
        let connection = Connection::open_in_memory().map_err(error)?;
        initialise(&connection, false).map_err(error)?;
        Ok(Self {
            path: None,
            connection: Mutex::new(connection),
        })
    }

    /// The database file and its SQLite companions.
    fn files_of(path: &Path) -> [PathBuf; 3] {
        let with = |suffix: &str| {
            let mut name = path.as_os_str().to_owned();
            name.push(suffix);
            PathBuf::from(name)
        };
        [path.to_path_buf(), with("-wal"), with("-shm")]
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while holding the lock leaves SQLite consistent; keep going.
        self.connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Stored entries for `paths` whose stamp and options still match.
    pub fn lookup(
        &self,
        paths: &[(&str, Stamp)],
        fingerprint: &str,
    ) -> Result<Vec<Option<AudioEntry>>, String> {
        let blobs: Vec<Option<Vec<u8>>> = {
            let connection = self.lock();
            let mut statement = connection
                .prepare_cached("SELECT modified, size, options, entry FROM files WHERE path = ?1")
                .map_err(error)?;
            let mut blobs = Vec::with_capacity(paths.len());
            for (path, stamp) in paths {
                let row = statement
                    .query_row([path], |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Vec<u8>>(3)?,
                        ))
                    })
                    .optional()
                    .map_err(error)?;
                blobs.push(row.and_then(|(modified, size, options, entry)| {
                    (modified == stamp.modified && size == stamp.size && options == fingerprint)
                        .then_some(entry)
                }));
            }
            blobs
        };
        // Parse outside the lock and in parallel; large folders hold thousands.
        let parsed = map_ordered(&blobs, &Silent, |blob| {
            blob.as_deref()
                .and_then(|bytes| serde_json::from_slice::<AudioEntry>(bytes).ok())
        });
        Ok(parsed
            .into_iter()
            .zip(paths)
            .map(|(entry, (path, _))| entry.flatten().filter(|e| e.path == *path))
            .collect())
    }

    /// Store entries read from files with the stamps the files had before
    /// they were read.
    pub fn store(&self, entries: &[(&AudioEntry, Stamp)], fingerprint: &str) -> Result<(), String> {
        let rows: Vec<(&str, Stamp, Vec<u8>)> = entries
            .iter()
            .filter_map(|(entry, stamp)| {
                let bytes = serde_json::to_vec(entry).ok()?;
                Some((entry.path.as_str(), *stamp, bytes))
            })
            .collect();
        let mut connection = self.lock();
        for chunk in rows.chunks(CHUNK) {
            let transaction = connection.transaction().map_err(error)?;
            {
                let mut statement = transaction
                    .prepare_cached(
                        "INSERT OR REPLACE INTO files (path, modified, size, options, entry)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                    )
                    .map_err(error)?;
                for (path, stamp, bytes) in chunk {
                    statement
                        .execute(params![
                            path,
                            stamp.modified,
                            stamp.size,
                            fingerprint,
                            bytes
                        ])
                        .map_err(error)?;
                }
            }
            transaction.commit().map_err(error)?;
        }
        Ok(())
    }

    /// Remove the entries of files that changed, moved or were deleted.
    pub fn forget<S: AsRef<str>>(&self, paths: &[S]) -> Result<usize, String> {
        let mut connection = self.lock();
        let mut removed = 0;
        for chunk in paths.chunks(CHUNK) {
            let transaction = connection.transaction().map_err(error)?;
            {
                let mut statement = transaction
                    .prepare_cached("DELETE FROM files WHERE path = ?1")
                    .map_err(error)?;
                for path in chunk {
                    removed += statement.execute([path.as_ref()]).map_err(error)?;
                }
            }
            transaction.commit().map_err(error)?;
        }
        Ok(removed)
    }

    /// Remove the entries of every file below `directory` (it moved).
    pub fn forget_below(&self, directory: &str) -> Result<usize, String> {
        let base = directory.trim_end_matches(['/', '\\']);
        if base.is_empty() {
            return Ok(0);
        }
        let connection = self.lock();
        let mut removed = 0;
        // Paths below `base` sort between `base/` and `base0` ('0' follows '/'),
        // and for Windows separators between `base\` and `base]`.
        for (separator, after) in [('/', '0'), ('\\', ']')] {
            removed += connection
                .execute(
                    "DELETE FROM files WHERE path >= ?1 AND path < ?2",
                    params![format!("{base}{separator}"), format!("{base}{after}")],
                )
                .map_err(error)?;
        }
        Ok(removed)
    }

    /// Cleanup Library: remove entries whose file is gone or, with a directory
    /// restriction, lies outside the included directories. Checks run in
    /// parallel and stop when `progress` cancels; entries checked so far are
    /// still cleaned.
    pub fn cleanup(&self, scope: &Scope, progress: &dyn Progress) -> Result<CleanupReport, String> {
        let paths: Vec<String> = {
            let connection = self.lock();
            let mut statement = connection
                .prepare("SELECT path FROM files")
                .map_err(error)?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(error)?;
            rows.collect::<rusqlite::Result<_>>().map_err(error)?
        };
        progress.start(paths.len());
        let kept = map_ordered(&paths, progress, |path| {
            let path = Path::new(path);
            scope.contains(path) && fs::metadata(path).is_ok_and(|m| m.is_file())
        });
        let checked = kept.iter().filter(|k| k.is_some()).count();
        let orphans: Vec<&String> = paths
            .iter()
            .zip(&kept)
            .filter(|(_, kept)| **kept == Some(false))
            .map(|(path, _)| path)
            .collect();
        let removed = self.forget(&orphans)?;
        Ok(CleanupReport {
            checked,
            removed,
            cancelled: checked < paths.len(),
        })
    }

    /// Start with a fresh library.
    pub fn clear(&self) -> Result<(), String> {
        let connection = self.lock();
        connection
            .execute_batch("DELETE FROM files; VACUUM;")
            .map_err(error)?;
        if self.path.is_some() {
            connection
                .pragma_update_and_check(None, "wal_checkpoint", "TRUNCATE", |_| Ok(()))
                .map_err(error)?;
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<Stats, String> {
        let files: i64 = self
            .lock()
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .map_err(error)?;
        let bytes = self.path.as_deref().map_or(0, |path| {
            Self::files_of(path)
                .iter()
                .filter_map(|file| fs::metadata(file).ok())
                .map(|m| m.len())
                .sum()
        });
        Ok(Stats {
            files: usize::try_from(files).unwrap_or_default(),
            bytes,
        })
    }
}

/// One scan's view of the library: lookups and stores for files in `scope`
/// under the current read options. With `refresh`, every file is read again
/// (File → Reload tags) and the fresh entries replace the stored ones.
pub struct Session<'a> {
    library: &'a Library,
    scope: &'a Scope,
    fingerprint: String,
    refresh: bool,
    seen: Mutex<HashMap<String, Stamp>>,
    hits: AtomicUsize,
    stored: AtomicUsize,
}
impl<'a> Session<'a> {
    pub fn new(library: &'a Library, scope: &'a Scope, fingerprint: String, refresh: bool) -> Self {
        Self {
            library,
            scope,
            fingerprint,
            refresh,
            seen: Mutex::new(HashMap::new()),
            hits: AtomicUsize::new(0),
            stored: AtomicUsize::new(0),
        }
    }
    /// Files taken from the library instead of being read.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
    /// Files read and stored in the library.
    pub fn stored(&self) -> usize {
        self.stored.load(Ordering::Relaxed)
    }
}

impl EntryCache for Session<'_> {
    fn lookup(&self, paths: &[PathBuf]) -> Vec<Option<AudioEntry>> {
        let stamps = map_ordered(paths, &Silent, |path| {
            path.to_str()?;
            if !self.scope.contains(path) {
                return None;
            }
            Stamp::of(path)
        });
        let known: Vec<(usize, (&str, Stamp))> = stamps
            .into_iter()
            .zip(paths)
            .enumerate()
            .filter_map(|(index, (stamp, path))| Some((index, (path.to_str()?, stamp.flatten()?))))
            .collect();
        if let Ok(mut seen) = self.seen.lock() {
            seen.extend(known.iter().map(|(_, (p, s))| ((*p).to_owned(), *s)));
        }
        let mut found: Vec<Option<AudioEntry>> = vec![None; paths.len()];
        if self.refresh || known.is_empty() {
            return found;
        }
        let keys: Vec<(&str, Stamp)> = known.iter().map(|(_, key)| *key).collect();
        // A library that cannot be read only costs the time to read the files.
        if let Ok(entries) = self.library.lookup(&keys, &self.fingerprint) {
            for ((index, _), entry) in known.iter().zip(entries) {
                let entry = entry.and_then(|mut entry| {
                    refresh_cached_dates(&mut entry)?;
                    Some(entry)
                });
                if entry.is_some() {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                }
                found[*index] = entry;
            }
        }
        found
    }

    fn store(&self, entries: &[&AudioEntry]) {
        let seen = match self.seen.lock() {
            Ok(seen) => seen,
            Err(_) => return,
        };
        // Skip files that changed while they were read; the next read stores them.
        let rows: Vec<(&AudioEntry, Stamp)> = entries
            .iter()
            .filter_map(|entry| {
                let before = *seen.get(&entry.path)?;
                (Stamp::of(Path::new(&entry.path)) == Some(before)).then_some((*entry, before))
            })
            .collect();
        drop(seen);
        if self.library.store(&rows, &self.fingerprint).is_ok() {
            self.stored.fetch_add(rows.len(), Ordering::Relaxed);
        }
    }
}

/// Formatting is presentation state, not part of the file stamp. Reuse the
/// actual reader's formatter without reparsing audio, including after restart.
/// Legacy or invalid timestamp data is a cache miss rather than stale output.
fn refresh_cached_dates(entry: &mut AudioEntry) -> Option<()> {
    fn raw_time(fields: &Fields, raw: &str, shown: &[&str]) -> Option<Option<SystemTime>> {
        let Some(values) = fields.get(raw) else {
            return (!shown.iter().any(|key| fields.contains_key(*key))).then_some(None);
        };
        let [value] = values.as_slice() else {
            return None;
        };
        let seconds = value.parse::<i64>().ok()?;
        let duration = Duration::from_secs(seconds.unsigned_abs());
        let time = if seconds < 0 {
            UNIX_EPOCH.checked_sub(duration)
        } else {
            UNIX_EPOCH.checked_add(duration)
        }?;
        Some(Some(time))
    }
    let modified = raw_time(
        &entry.technical,
        "_FILE_MOD_DATETIME_RAW",
        &["_FILE_MOD_DATE", "_FILE_MOD_DATETIME"],
    )??;
    let created = raw_time(
        &entry.technical,
        "_FILE_CREATE_DATETIME_RAW",
        &["_FILE_CREATE_DATE", "_FILE_CREATE_DATETIME"],
    )?;
    let current = tag_core::file_time_fields(created, Some(modified));
    if !current.contains_key("_FILE_MOD_DATETIME")
        || (created.is_some() && !current.contains_key("_FILE_CREATE_DATETIME"))
    {
        return None;
    }
    entry.technical.extend(current);
    for key in ["_MP4_CREATION_TIME", "_MP4_MODIFICATION_TIME"] {
        let raw = format!("{key}_RAW");
        if let Some(time) = raw_time(&entry.technical, &raw, &[key])? {
            let current = tag_core::file_time_fields(None, Some(time));
            entry
                .technical
                .insert(key.into(), current.get("_FILE_MOD_DATETIME")?.clone());
        }
    }
    Some(())
}
