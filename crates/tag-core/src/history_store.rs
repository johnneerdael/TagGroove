//! Versioned bounded index and atomic publication for local edit history.
use super::*;
use serde::{de::Error as _, Deserializer, Serializer};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
};

const INDEX: &str = "index.json";
const MAX_INDEX: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) enum DirectoryId {
    #[cfg(unix)]
    Unix { device: u64, inode: u64 },
    #[cfg(windows)]
    Windows(crate::file_baseline::WindowsIdentity),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DirectoryGuard {
    #[serde(with = "native_path")]
    pub(super) path: PathBuf,
    identity: Option<DirectoryId>,
}

fn optional_directory_id(path: &Path) -> Result<Option<DirectoryId>, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
        Ok(_) => directory_id(path).map(Some),
    }
}

pub(super) fn parent_guards(path: &Path) -> Result<Vec<DirectoryGuard>, String> {
    parent_guards_after_moves(path, &[])
}

pub(super) fn parent_guards_after_moves(
    path: &Path,
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<DirectoryGuard>, String> {
    path.ancestors()
        .skip(1)
        .map(|parent| {
            Ok(DirectoryGuard {
                path: parent.to_owned(),
                identity: optional_directory_id(&rebase(parent, moves))?,
            })
        })
        .collect()
}

pub(super) fn directory_guards(
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<DirectoryGuard>, String> {
    directory_guard_paths(moves)
        .into_iter()
        .map(|path| {
            Ok(DirectoryGuard {
                identity: optional_directory_id(&path)?,
                path,
            })
        })
        .collect()
}

fn directory_guard_paths(moves: &[(PathBuf, PathBuf)]) -> BTreeSet<PathBuf> {
    moves
        .iter()
        .flat_map(|(from, to)| [from, to])
        .flat_map(|path| path.ancestors().map(Path::to_path_buf))
        .collect()
}

pub(super) fn check_guards(guards: &[DirectoryGuard]) -> Result<(), String> {
    for guard in guards {
        if optional_directory_id(&guard.path)? != guard.identity {
            return Err(format!(
                "An undo parent directory changed: {}",
                display(&guard.path)
            ));
        }
    }
    Ok(())
}

pub(super) fn check_captured_guards(
    guards: &[DirectoryGuard],
    moves: &[(PathBuf, PathBuf)],
) -> Result<(), String> {
    // Missing output parents may be created by the operation. Existing parents
    // must still identify the same directories after its reported folder moves.
    for guard in guards.iter().filter(|guard| guard.identity.is_some()) {
        if optional_directory_id(&rebase(&guard.path, moves))? != guard.identity {
            return Err("An undo parent directory changed during the operation".into());
        }
    }
    Ok(())
}

// Each batch belongs to one recreation. A native ID can be reused by another
// directory in that same batch; only later batches may follow its new identity.
pub(super) type DirectoryChanges = Vec<Vec<(DirectoryId, DirectoryId)>>;

pub(super) fn replace_guard_ids(guards: &mut [DirectoryGuard], changes: &DirectoryChanges) {
    for batch in changes {
        for guard in &mut *guards {
            if let Some((_, new)) = batch.iter().find(|(old, _)| guard.identity == Some(*old)) {
                guard.identity = Some(*new);
            }
        }
    }
}

pub(super) fn created_parent_guards(
    paths: Vec<PathBuf>,
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<DirectoryGuard>, String> {
    paths
        .into_iter()
        .map(|path| {
            Ok(DirectoryGuard {
                identity: optional_directory_id(&rebase(&path, moves))?,
                path,
            })
        })
        .collect()
}

pub(super) fn refresh_created_parents(
    guards: &mut [DirectoryGuard],
) -> Result<DirectoryChanges, String> {
    let mut changes = Vec::new();
    for guard in guards {
        let current = optional_directory_id(&guard.path)?;
        if let (Some(old), Some(new)) = (guard.identity, current) {
            if old != new {
                changes.push((old, new));
            }
        }
        guard.identity = current;
    }
    Ok(if changes.is_empty() {
        Vec::new()
    } else {
        vec![changes]
    })
}

fn validate_parent_guards(path: &Path, guards: &[DirectoryGuard]) -> Result<(), String> {
    if !path
        .ancestors()
        .skip(1)
        .eq(guards.iter().map(|guard| guard.path.as_path()))
    {
        return Err("Invalid undo parent directory guards".into());
    }
    Ok(())
}

fn directory_id(path: &Path) -> Result<DirectoryId, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err("The undo store directory changed".into());
    }
    #[cfg(all(test, unix))]
    if let Some(identity) = tests::directory_identity(path) {
        return Ok(identity);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(DirectoryId::Unix {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_READ_ATTRIBUTES,
        };
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("The undo store directory changed".into());
        }
        let file = fs::File::options()
            .access_mode(FILE_READ_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|error| error.to_string())?;
        crate::file_baseline::windows_identity(&file)
            .map(DirectoryId::Windows)
            .map_err(|error| error.to_string())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err("Native undo directory identity is unavailable".into())
    }
}

/// Captured paths are canonical. Refuse new parent aliases, including reparse
/// points, rather than following them to a different recovery destination.
pub(super) fn safe_parents(path: &Path) -> Result<(), String> {
    for parent in path.ancestors().skip(1) {
        match fs::symlink_metadata(parent) {
            Ok(metadata) if metadata.is_dir() => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
                    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                        return Err("An undo parent directory changed".into());
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("An undo parent directory changed".into()),
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    version: u32,
    platform: String,
    lock_stamp: crate::file_baseline::MetadataStamp,
    next: u64,
    pending: Option<RecoveryOperation>,
    recovery: Vec<RecoveryOperation>,
    undo: Vec<Record>,
    redo: Vec<Record>,
    notice: Option<String>,
}

fn directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err("The undo store is not a regular directory".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path).map_err(|error| error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn regular(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(metadata)),
        Ok(_) => Err("The undo store contains a nonregular entry".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(super) fn open(root: &Path, limits: Limits) -> Result<Journal, String> {
    directory(root)?;
    let root = dunce::canonicalize(root).map_err(|error| error.to_string())?;
    let dir = root.join("session-store");
    directory(&dir)?;
    let store_id = directory_id(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    regular(&dir.join(LOCK))?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(LOCK))
        .map_err(|error| error.to_string())?;
    lock.try_lock()
        .map_err(|error| format!("Undo history is already open: {error}"))?;
    let lock_stamp = crate::file_baseline::metadata_stamp_for_handle(&lock)
        .map_err(|error| error.to_string())?;
    let mut journal = Journal {
        dir,
        store_id,
        lock_stamp,
        limits,
        undo: Vec::new(),
        redo: Vec::new(),
        next: 0,
        notice: None,
        abandoned_group: None,
        options: TagOptions::default(),
        pending: false,
        intent: None,
        recovery: Vec::new(),
        store_failed: false,
        active_capture: None,
        outcome_revision: 0,
        _lock: lock,
    };
    journal.validate_store()?;
    if let Some(metadata) = regular(&journal.dir.join(INDEX))? {
        if metadata.len() > MAX_INDEX {
            return Err("The undo index exceeds its size limit".into());
        }
        let mut bytes = Vec::new();
        fs::File::open(journal.dir.join(INDEX))
            .map_err(|error| error.to_string())?
            .take(MAX_INDEX + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_INDEX {
            return Err("The undo index exceeds its size limit".into());
        }
        let index: Index = serde_json::from_slice(&bytes)
            .map_err(|error| format!("The undo index is invalid: {error}"))?;
        if index.version != 2 || index.platform != std::env::consts::OS {
            return Err("The undo index has an unsupported version or platform".into());
        }
        if index.lock_stamp != journal.lock_stamp {
            return Err("The undo store lock was replaced".into());
        }
        journal.next = index.next;
        journal.intent = index.pending;
        journal.recovery = index.recovery;
        journal.undo = index.undo;
        journal.redo = index.redo;
        journal.notice = index.notice;
        validate(&mut journal)?;
        // Interrupted work may have changed paths used by older operations.
        // Keep their verified copies for explicit export; never replay/resume
        // them automatically. New operations use independent fresh history.
        if let Some(intent) = journal.intent.take() {
            journal
                .recovery
                .extend(
                    journal
                        .undo
                        .drain(..)
                        .chain(journal.redo.drain(..))
                        .map(|record| RecoveryOperation {
                            kind: RecoveryKind::RetiredHistory,
                            record,
                        }),
                );
            if !intent.record.files.is_empty()
                || !intent.record.side_files.is_empty()
                || !intent.record.directories.is_empty()
            {
                journal.recovery.push(intent);
            }
            journal.notice = Some("An operation was interrupted. Current files were left unchanged; retained recovery copies are separate from new Undo history.".into());
        }
        // Group IDs belong to their originating process. A restarted caller
        // cannot accidentally merge new work into a previously completed prefix.
        for record in journal.undo.iter_mut().chain(&mut journal.redo) {
            record.group = None;
        }
        journal.enforce_limits();
    } else if fs::read_dir(&journal.dir)
        .map_err(|error| error.to_string())?
        .flatten()
        .any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "snapshot")
        })
    {
        return Err("The undo index is missing; its recovery files were retained".into());
    }
    journal.checkpoint(false)?;
    // Legacy session stores have no durable index. Preserve locked sessions;
    // only remove unlocked old-layout directories below this designated root.
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path == journal.dir || !entry.file_name().to_string_lossy().starts_with("session-") {
                continue;
            }
            if !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_dir()) {
                continue;
            }
            let stale = match fs::File::open(path.join(LOCK)) {
                Ok(file) => file.try_lock().is_ok(),
                Err(_) => true,
            };
            if stale {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
    Ok(journal)
}

fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}

fn validate(journal: &mut Journal) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    let mut entries = journal
        .undo
        .len()
        .checked_add(journal.redo.len())
        .and_then(|n| n.checked_add(journal.recovery.len()))
        .and_then(|n| n.checked_add(usize::from(journal.intent.is_some())))
        .ok_or("Undo entry count overflow")?;
    if entries > MAX_ENTRIES {
        return Err("The undo index has too many entries".into());
    }
    let mut bytes = 0u64;
    let records = journal
        .undo
        .iter_mut()
        .chain(&mut journal.redo)
        .map(|record| (record, false))
        .chain(
            journal
                .recovery
                .iter_mut()
                .map(|op| (&mut op.record, matches!(op.kind, RecoveryKind::Capture))),
        )
        .chain(
            journal
                .intent
                .iter_mut()
                .map(|op| (&mut op.record, matches!(op.kind, RecoveryKind::Capture))),
        );
    for (record, allow_empty) in records {
        if !allow_empty
            && record.files.is_empty()
            && record.directories.is_empty()
            && record.side_files.is_empty()
        {
            return Err("The undo index contains an empty operation".into());
        }
        if record.label.is_empty() || record.label.len() > 64 * 1024 {
            return Err("Invalid undo operation label".into());
        }
        entries = entries
            .checked_add(record.files.len())
            .and_then(|n| n.checked_add(record.side_files.len()))
            .and_then(|n| n.checked_add(record.directories.len()))
            .ok_or("Undo entry count overflow")?;
        if entries > MAX_ENTRIES {
            return Err("The undo index has too many entries".into());
        }
        for file in &record.files {
            if !absolute(&file.before.path)
                || !absolute(&file.after.path)
                || file.cost > file.before.fingerprint.len
                || file.cost < OVERHEAD.min(file.before.fingerprint.len)
            {
                return Err("Invalid undo audio path or cost".into());
            }
            if file.content && file.before.blob.is_none() {
                return Err("The undo index lacks a required audio snapshot".into());
            }
            for side in [&file.before, &file.after] {
                validate_parent_guards(&side.path, &side.parents)?;
                if let Some(blob) = &side.blob {
                    if blob.symlink_target.is_some()
                        || !blob.fingerprint.same_content(&side.fingerprint)
                    {
                        return Err(
                            "An undo audio snapshot does not match its recorded file".into()
                        );
                    }
                    if blob.linked
                        && blob.cost != file.cost.min(blob.fingerprint.len)
                        && blob.cost != blob.fingerprint.len
                    {
                        return Err("An undo audio snapshot has an inconsistent replay cost".into());
                    }
                }
            }
        }
        for (from, to) in &record.directories {
            if !absolute(from) || !absolute(to) {
                return Err("Invalid undo directory path".into());
            }
        }
        if directory_guard_paths(&record.directories)
            .into_iter()
            .ne(record
                .directory_guards
                .iter()
                .map(|guard| guard.path.clone()))
        {
            return Err("Invalid undo directory move guards".into());
        }
        for side in &record.side_files {
            validate_parent_guards(&side.path, &side.parents)?;
            if !absolute(&side.path)
                || side.epoch > record.directories.len()
                || side
                    .created_parents
                    .iter()
                    .any(|parent| !absolute(&parent.path) || !side.path.starts_with(&parent.path))
            {
                return Err("Invalid undo side-file path or epoch".into());
            }
            if [side.before.as_ref(), side.after.as_ref()]
                .into_iter()
                .flatten()
                .any(|blob| blob.linked || blob.cost != blob.fingerprint.len)
            {
                return Err("Invalid undo side-file snapshot cost".into());
            }
        }
        for blob in record
            .files
            .iter_mut()
            .flat_map(|file| [&mut file.before.blob, &mut file.after.blob])
            .chain(
                record
                    .side_files
                    .iter_mut()
                    .flat_map(|side| [&mut side.before, &mut side.after]),
            )
            .flatten()
        {
            let id = snapshot_id(&blob.path).ok_or("Invalid undo snapshot ID")?;
            if id > journal.next || !ids.insert(id) {
                return Err("Duplicate or future undo snapshot ID".into());
            }
            if blob.cost > blob.fingerprint.len
                || blob.cost < OVERHEAD.min(blob.fingerprint.len)
                || !blob.linked && blob.cost != blob.fingerprint.len
            {
                return Err("Invalid undo snapshot cost".into());
            }
            bytes = bytes
                .checked_add(blob.cost)
                .ok_or("Undo byte count overflow")?;
            blob.path = journal.dir.join(format!("{id}.snapshot"));
            let metadata = regular(&blob.path)?.ok_or("An undo snapshot is missing")?;
            if metadata.len() != blob.fingerprint.len {
                return Err("An undo snapshot has an invalid length".into());
            }
            let stamp = metadata_stamp(&blob.path).map_err(|error| error.to_string())?;
            if !blob.fingerprint.matches_metadata(&stamp) {
                return Err("An undo snapshot was replaced or modified".into());
            }
        }
    }
    Ok(())
}

pub(super) fn sync_blob(blob: &Blob) -> Result<(), String> {
    sync_path(&blob.path)
}

pub(super) fn sync_path(path: &Path) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    options.write(true);
    options
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())
}

struct BoundedWriter<'a> {
    file: &'a mut fs::File,
    left: u64,
}
impl Write for BoundedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.left {
            return Err(std::io::Error::other(
                "The undo index exceeds its size limit",
            ));
        }
        let n = self.file.write(bytes)?;
        self.left -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// Publish before collecting old blobs. The Windows path needs a write-through
/// rename rather than tempfile's rename alone. Native crash qualification is
/// separate from selecting the documented publication API.
fn publish(file: tempfile::NamedTempFile, dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        file.persist(dir.join(INDEX))
            .map_err(|error| error.to_string())?;
        fs::File::open(dir)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        // Drop the writable handle before moving so native last-write metadata
        // has settled and the file can be renamed on Windows.
        let staged = file.into_temp_path();
        let source = wide_absolute(&staged)?;
        let destination = wide_absolute(&dir.join(INDEX))?;
        // SAFETY: both buffers are live, NUL-terminated native absolute paths.
        // The staging file is on the destination volume; no copy fallback is allowed.
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (file, dir);
        Err("Durable undo index publication is unavailable on this platform".into())
    }
}

#[cfg(windows)]
fn wide_absolute(path: &Path) -> Result<Vec<u16>, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Component, Prefix};
    let units: Vec<_> = path.as_os_str().encode_wide().collect();
    if units.contains(&0) {
        return Err("Invalid undo index path".into());
    }
    let mut result = match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(_) => r"\\?\"
                .encode_utf16()
                .chain(units.into_iter().map(|u| if u == 47 { 92 } else { u }))
                .collect::<Vec<_>>(),
            Prefix::UNC(_, _) => r"\\?\UNC\"
                .encode_utf16()
                .chain(
                    units
                        .into_iter()
                        .skip(2)
                        .map(|u| if u == 47 { 92 } else { u }),
                )
                .collect(),
            Prefix::Verbatim(_) | Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(_, _) => units,
            _ => return Err("Invalid undo index path".into()),
        },
        _ => return Err("Invalid undo index path".into()),
    };
    result.push(0);
    Ok(result)
}

impl Journal {
    pub(super) fn validate_store(&self) -> Result<(), String> {
        if directory_id(&self.dir)? != self.store_id {
            return Err("The undo store directory changed".into());
        }
        if metadata_stamp(&self.dir.join(LOCK)).map_err(|error| error.to_string())?
            != self.lock_stamp
        {
            return Err("The undo store lock was replaced".into());
        }
        safe_parents(&self.dir)
    }

    pub(super) fn checkpoint(&mut self, pending: bool) -> Result<(), String> {
        self.validate_store()?;
        regular(&self.dir.join(INDEX))?;
        for blob in self
            .undo
            .iter()
            .chain(&self.redo)
            .chain(self.recovery.iter().map(|op| &op.record))
            .chain(self.intent.iter().filter(|_| pending).map(|op| &op.record))
            .flat_map(Record::blobs)
        {
            sync_blob(blob)?;
        }
        let index = Index {
            version: 2,
            platform: std::env::consts::OS.into(),
            lock_stamp: self.lock_stamp,
            next: self.next,
            pending: if pending {
                Some(self.intent.clone().ok_or("Missing undo operation intent")?)
            } else {
                None
            },
            recovery: self.recovery.clone(),
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            notice: self.notice.clone(),
        };
        let mut file = tempfile::Builder::new()
            .prefix(".history-index-")
            .tempfile_in(&self.dir)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer(
            BoundedWriter {
                file: file.as_file_mut(),
                left: MAX_INDEX,
            },
            &index,
        )
        .map_err(|error| error.to_string())?;
        file.as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        publish(file, &self.dir)?;
        self.pending = pending;
        if !pending {
            self.intent = None;
            self.collect_unreferenced();
        }
        Ok(())
    }

    fn collect_unreferenced(&self) {
        if self.validate_store().is_err() {
            return;
        }
        let keep: BTreeSet<_> = self
            .undo
            .iter()
            .chain(&self.redo)
            .chain(self.recovery.iter().map(|op| &op.record))
            .flat_map(Record::blobs)
            .map(|blob| blob.path.clone())
            .collect();
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if snapshot_id(&path).is_some()
                    && !keep.contains(&path)
                    && fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file())
                {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
}

fn snapshot_id(path: &Path) -> Option<u64> {
    let text = path.file_name()?.to_str()?.strip_suffix(".snapshot")?;
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|n| *n != 0)
}

#[derive(Serialize)]
struct ExportManifest {
    version: u32,
    platform: &'static str,
    operations: Vec<ExportOperation>,
}

#[derive(Serialize)]
struct ExportOperation {
    kind: RecoveryKind,
    label: String,
    #[serde(with = "moves")]
    audio_paths: Vec<(PathBuf, PathBuf)>,
    #[serde(with = "moves")]
    directory_moves: Vec<(PathBuf, PathBuf)>,
    entries: Vec<ExportEntry>,
}

#[derive(Serialize)]
struct ExportEntry {
    #[serde(with = "native_path")]
    source: PathBuf,
    display_path: String,
    side: &'static str,
    kind: &'static str,
    epoch: Option<usize>,
    was_present: bool,
    copy: Option<String>,
    #[serde(with = "optional_path")]
    symlink_target: Option<PathBuf>,
}

pub(super) fn export_recovery(journal: &Journal, destination: &Path) -> Result<PathBuf, String> {
    journal.validate_store()?;
    if journal.recovery.is_empty() {
        return Err("There are no retained recovery copies".into());
    }
    let destination = dunce::canonicalize(destination).map_err(|error| error.to_string())?;
    directory_id(&destination)?;
    // Export may need full physical copies even for cheap clone-backed history.
    let mut required = MAX_INDEX;
    for blob in journal
        .recovery
        .iter()
        .flat_map(|operation| operation.record.blobs())
    {
        required = required
            .checked_add(blob.fingerprint.len)
            .ok_or("Recovery export size overflow")?;
        if fingerprint(&blob.path)? != blob.fingerprint {
            return Err("A recovery snapshot was replaced or modified".into());
        }
    }
    if available_space(&destination)
        .is_some_and(|free| free < required.saturating_add(journal.limits.min_free))
    {
        return Err("There is not enough free disk space to export recovery copies".into());
    }
    let output = destination.join(format!("TagGroove recovery {}", uuid::Uuid::new_v4()));
    let parents = parent_guards(&output)?;
    check_guards(&parents)?;
    fs::create_dir(&output).map_err(|error| error.to_string())?;
    let owned = directory_id(&output)?;
    let result = (|| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output, fs::Permissions::from_mode(0o700))
                .map_err(|error| error.to_string())?;
        }
        let mut manifest = ExportManifest {
            version: 1,
            platform: std::env::consts::OS,
            operations: Vec::new(),
        };
        let mut next = 0u64;
        for operation in &journal.recovery {
            let record = &operation.record;
            let mut exported = ExportOperation {
                kind: operation.kind,
                label: record.label.clone(),
                audio_paths: record
                    .files
                    .iter()
                    .map(|file| (file.before.path.clone(), file.after.path.clone()))
                    .collect(),
                directory_moves: record.directories.clone(),
                entries: Vec::new(),
            };
            let mut entry =
                |path: &Path, blob: Option<&Blob>, side, kind, epoch| -> Result<(), String> {
                    journal.validate_store()?;
                    check_guards(&parents)?;
                    if directory_id(&output)? != owned {
                        return Err("The recovery export directory changed".into());
                    }
                    let copy = if let Some(blob) = blob {
                        if fingerprint(&blob.path)? != blob.fingerprint {
                            return Err("A recovery snapshot was replaced or modified".into());
                        }
                        next = next.checked_add(1).ok_or("Recovery copy count overflow")?;
                        let extension = if blob.symlink_target.is_some() {
                            "link-target.bin"
                        } else {
                            path.extension()
                                .and_then(|extension| extension.to_str())
                                .filter(|extension| {
                                    !extension.is_empty()
                                        && extension.len() <= 12
                                        && extension
                                            .bytes()
                                            .all(|byte| byte.is_ascii_alphanumeric())
                                })
                                .unwrap_or("bin")
                        };
                        let name = format!("{next:06}-{side}.{extension}");
                        let target = output.join(&name);
                        let mut file = fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&target)
                            .map_err(|error| error.to_string())?;
                        let input =
                            fs::File::open(&blob.path).map_err(|error| error.to_string())?;
                        std::io::copy(
                            &mut input.take(blob.fingerprint.len.saturating_add(1)),
                            &mut file,
                        )
                        .map_err(|error| error.to_string())?;
                        file.sync_all().map_err(|error| error.to_string())?;
                        drop(file);
                        if !fingerprint(&target)?.same_content(&blob.fingerprint)
                            || fingerprint(&blob.path)? != blob.fingerprint
                        {
                            return Err("A recovery snapshot changed while it was exported".into());
                        }
                        Some(name)
                    } else {
                        None
                    };
                    exported.entries.push(ExportEntry {
                        source: path.to_owned(),
                        display_path: display(path),
                        side,
                        kind,
                        epoch,
                        was_present: blob.is_some(),
                        copy,
                        symlink_target: blob.and_then(|blob| blob.symlink_target.clone()),
                    });
                    Ok(())
                };
            for file in &record.files {
                for (side, name) in [(&file.before, "before"), (&file.after, "after")] {
                    if let Some(blob) = side.blob.as_ref() {
                        entry(&side.path, Some(blob), name, "audio", None)?;
                    }
                }
            }
            for side in &record.side_files {
                entry(
                    &side.path,
                    side.before.as_ref(),
                    "before",
                    "side_file",
                    Some(side.epoch),
                )?;
                if !matches!(operation.kind, RecoveryKind::Capture) {
                    entry(
                        &side.path,
                        side.after.as_ref(),
                        "after",
                        "side_file",
                        Some(side.epoch),
                    )?;
                }
            }
            manifest.operations.push(exported);
        }
        check_guards(&parents)?;
        if directory_id(&output)? != owned {
            return Err("The recovery export directory changed".into());
        }
        // Write the manifest last: a failed partial export cannot present a
        // completed manifest. Existing files are never opened for replacement.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("manifest.json"))
            .map_err(|error| error.to_string())?;
        serde_json::to_writer(
            BoundedWriter {
                file: &mut file,
                left: MAX_INDEX,
            },
            &manifest,
        )
        .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        #[cfg(unix)]
        fs::File::open(&output)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
        Ok(())
    })();
    result.map_err(|error: String| {
        format!(
            "Recovery export incomplete in {}: {error}",
            display(&output)
        )
    })?;
    Ok(output)
}

pub(super) mod blob_name {
    use super::*;
    pub(crate) fn serialize<S: Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
        let id = snapshot_id(path)
            .ok_or_else(|| serde::ser::Error::custom("Invalid undo snapshot ID"))?;
        id.serialize(s)
    }
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PathBuf, D::Error> {
        let id = u64::deserialize(d)?;
        if id == 0 {
            return Err(D::Error::custom("Invalid undo snapshot ID"));
        }
        Ok(PathBuf::from(format!("{id}.snapshot")))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "encoding", content = "units")]
enum NativePath {
    Unix(Vec<u8>),
    Windows(Vec<u16>),
}
fn encode(path: &Path) -> NativePath {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        NativePath::Unix(path.as_os_str().as_bytes().to_vec())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        NativePath::Windows(path.as_os_str().encode_wide().collect())
    }
    #[cfg(not(any(unix, windows)))]
    {
        NativePath::Unix(path.to_string_lossy().as_bytes().to_vec())
    }
}
fn decode(path: NativePath) -> Result<PathBuf, &'static str> {
    match path {
        #[cfg(unix)]
        NativePath::Unix(units) if units.len() <= 64 * 1024 && !units.contains(&0) => {
            use std::os::unix::ffi::OsStringExt;
            Ok(std::ffi::OsString::from_vec(units).into())
        }
        #[cfg(windows)]
        NativePath::Windows(units) if units.len() <= 64 * 1024 && !units.contains(&0) => {
            use std::os::windows::ffi::OsStringExt;
            Ok(std::ffi::OsString::from_wide(&units).into())
        }
        _ => Err("Invalid undo native path encoding"),
    }
}
pub(super) mod native_path {
    use super::*;
    pub(crate) fn serialize<S: Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
        encode(path).serialize(s)
    }
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PathBuf, D::Error> {
        decode(NativePath::deserialize(d)?).map_err(D::Error::custom)
    }
}
pub(super) mod optional_path {
    use super::*;
    pub(crate) fn serialize<S: Serializer>(
        path: &Option<PathBuf>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        path.as_deref().map(encode).serialize(s)
    }
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<PathBuf>, D::Error> {
        Option::<NativePath>::deserialize(d)?
            .map(decode)
            .transpose()
            .map_err(D::Error::custom)
    }
}
pub(super) mod moves {
    use super::*;
    pub(crate) fn serialize<S: Serializer>(
        paths: &[(PathBuf, PathBuf)],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        paths
            .iter()
            .map(|(a, b)| (encode(a), encode(b)))
            .collect::<Vec<_>>()
            .serialize(s)
    }
    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Vec<(PathBuf, PathBuf)>, D::Error> {
        Vec::<(NativePath, NativePath)>::deserialize(d)?
            .into_iter()
            .map(|(a, b)| {
                Ok((
                    decode(a).map_err(D::Error::custom)?,
                    decode(b).map_err(D::Error::custom)?,
                ))
            })
            .collect()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::cell::RefCell;

    // APFS does not reliably recycle directory IDs within a short test. Keep
    // actual directories, writes, guards and durable replay; control only the
    // two native IDs to reproduce the reuse case implicated by Linux CI.
    thread_local! {
        static DIRECTORY_IDS: RefCell<Vec<(PathBuf, DirectoryId)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn directory_identity(path: &Path) -> Option<DirectoryId> {
        DIRECTORY_IDS.with(|ids| {
            ids.borrow()
                .iter()
                .find_map(|(known, id)| (known == path).then_some(*id))
        })
    }

    fn id(inode: u64) -> DirectoryId {
        DirectoryId::Unix { device: 7, inode }
    }

    struct RecycledDirectories(Option<tempfile::TempDir>);
    impl RecycledDirectories {
        fn new() -> Self {
            assert!(DIRECTORY_IDS.with(|ids| ids.borrow().is_empty()));
            Self(Some(
                tempfile::Builder::new()
                    .prefix("taggroove-directory-reuse-")
                    .tempdir()
                    .unwrap(),
            ))
        }
        fn path(&self) -> &Path {
            self.0.as_ref().unwrap().path()
        }
        fn generation(&self, outer: u64, inner: u64) {
            let root = dunce::canonicalize(self.path()).unwrap();
            DIRECTORY_IDS.with(|ids| {
                *ids.borrow_mut() = vec![
                    (root.join("covers"), id(outer)),
                    (root.join("covers/nested"), id(inner)),
                ]
            });
        }
    }
    impl Drop for RecycledDirectories {
        fn drop(&mut self) {
            DIRECTORY_IDS.with(|ids| ids.borrow_mut().clear());
            if std::thread::panicking() {
                eprintln!(
                    "Retained directory-ID reuse fixture: {}",
                    self.0.take().unwrap().keep().display()
                );
            }
        }
    }

    #[test]
    fn recycled_directory_ids_do_not_chain_within_one_recreation() {
        let mut guards = vec![
            DirectoryGuard {
                path: PathBuf::from("outer"),
                identity: Some(id(11)),
            },
            DirectoryGuard {
                path: PathBuf::from("inner"),
                identity: Some(id(22)),
            },
        ];
        replace_guard_ids(&mut guards, &vec![vec![(id(11), id(22)), (id(22), id(11))]]);
        assert_eq!(guards[0].identity, Some(id(22)));
        assert_eq!(guards[1].identity, Some(id(11)));
    }

    #[test]
    fn distinct_directory_recreations_update_the_previous_generation() {
        let mut guards = vec![DirectoryGuard {
            path: PathBuf::from("parent"),
            identity: Some(id(11)),
        }];
        replace_guard_ids(
            &mut guards,
            &vec![vec![(id(11), id(22))], vec![(id(22), id(33))]],
        );
        assert_eq!(guards[0].identity, Some(id(33)));
    }

    #[test]
    fn recycled_directory_ids_preserve_side_outputs_and_outside_refusal_after_restart() {
        let directory = RecycledDirectories::new();
        let root = directory.path().join("history");
        let parent = directory.path().join("covers/nested");
        let cover = parent.join("cover.bin");
        let other = parent.join("other.bin");
        directory.generation(11, 22);
        let mut journal = Journal::open(&root, Limits::default()).unwrap();
        let mut capture = journal.begin("Export two covers", None, &[]);
        journal.capture_side_file(&mut capture, &cover).unwrap();
        journal.capture_side_file(&mut capture, &other).unwrap();
        fs::create_dir_all(&parent).unwrap();
        fs::write(&cover, b"Cover").unwrap();
        fs::write(&other, b"Other").unwrap();
        fs::write(directory.path().join("independent-cover.bin"), b"Cover").unwrap();
        fs::write(directory.path().join("independent-other.bin"), b"Other").unwrap();
        journal.commit(capture, &[], &[]);
        drop(journal);
        for cycle in 0..3 {
            let mut journal = Journal::open(&root, Limits::default()).unwrap();
            let undo = journal.undo().unwrap();
            assert!(
                undo.errors.is_empty(),
                "cycle {cycle} Undo: {:?}",
                undo.errors
            );
            assert!(!parent.exists());
            drop(journal);
            let (outer, inner) = if cycle % 2 == 0 { (22, 11) } else { (11, 22) };
            directory.generation(outer, inner);
            let mut journal = Journal::open(&root, Limits::default()).unwrap();
            let redo = journal.redo().unwrap();
            assert!(
                redo.errors.is_empty(),
                "cycle {cycle} Redo: {:?}",
                redo.errors
            );
            assert_eq!(fs::read(&cover).unwrap(), b"Cover");
            assert_eq!(fs::read(&other).unwrap(), b"Other");
        }
        fs::rename(
            directory.path().join("covers"),
            directory.path().join("outside-held"),
        )
        .unwrap();
        fs::create_dir_all(&parent).unwrap();
        fs::write(&cover, b"Cover").unwrap();
        fs::write(&other, b"Other").unwrap();
        directory.generation(33, 44);
        let mut journal = Journal::open(&root, Limits::default()).unwrap();
        assert!(!journal.undo().unwrap().errors.is_empty());
        assert_eq!(fs::read(&cover).unwrap(), b"Cover");
        assert_eq!(fs::read(&other).unwrap(), b"Other");
        assert!(parent.is_dir());
    }

    #[test]
    fn rolled_back_directory_recreation_keeps_a_later_redo_eligible() {
        let directory = RecycledDirectories::new();
        let root = directory.path().join("history");
        let conflict = directory.path().join("conflict.bin");
        let before = directory.path().join("X");
        let after = directory.path().join("Y");
        let parent = directory.path().join("covers/nested");
        let cover = parent.join("cover.bin");
        fs::write(&conflict, b"Before").unwrap();
        fs::create_dir(&before).unwrap();
        let mut journal = Journal::open(&root, Limits::default()).unwrap();
        let mut first = journal.begin("First operation", None, &[]);
        journal.capture_side_file(&mut first, &conflict).unwrap();
        fs::write(&conflict, b"First").unwrap();
        fs::rename(&before, &after).unwrap();
        journal
            .capture_side_file_after_moves(&mut first, &cover, 1)
            .unwrap();
        fs::create_dir_all(&parent).unwrap();
        fs::write(&cover, b"First cover").unwrap();
        journal.commit(
            first,
            &[],
            &[(
                before.to_string_lossy().into(),
                after.to_string_lossy().into(),
            )],
        );
        let mut second = journal.begin("Second operation", None, &[]);
        journal.capture_side_file(&mut second, &cover).unwrap();
        fs::write(&cover, b"Second cover").unwrap();
        journal.commit(second, &[], &[]);
        assert!(journal.undo().unwrap().errors.is_empty());
        assert_eq!(fs::read(&cover).unwrap(), b"First cover");
        drop(journal);

        // Real open directory handles prevent reuse of these unlinked inodes.
        // No ID override is installed for this rollback/reopen regression.
        let _held_inner = fs::File::open(&parent).unwrap();
        let _held_outer = fs::File::open(parent.parent().unwrap()).unwrap();
        fs::write(&conflict, b"External").unwrap();
        fs::write(
            directory.path().join("independent-external.bin"),
            b"External",
        )
        .unwrap();
        fs::write(
            directory.path().join("independent-second-cover.bin"),
            b"Second cover",
        )
        .unwrap();
        let mut journal = Journal::open(&root, Limits::default()).unwrap();
        assert!(!journal.undo().unwrap().errors.is_empty());
        assert_eq!(fs::read(&conflict).unwrap(), b"External");
        assert_eq!(fs::read(&cover).unwrap(), b"First cover");
        assert!(after.is_dir());
        assert!(!before.exists());
        drop(journal);

        let mut journal = Journal::open(&root, Limits::default()).unwrap();
        let redo = journal.redo().unwrap();
        assert!(
            redo.errors.is_empty(),
            "Later Redo lost its rolled-back parents: {:?}",
            redo.errors
        );
        assert_eq!(fs::read(&cover).unwrap(), b"Second cover");
        assert_eq!(fs::read(&conflict).unwrap(), b"External");
    }
}
