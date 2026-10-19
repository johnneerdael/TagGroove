//! Session undo journal for operations that write audio files.
//!
//! Before an operation writes, every target file is snapshotted into the
//! journal directory. Staged writes replace files atomically, so a hard link
//! retains the previous inode without copying audio; other volumes fall back
//! to a bounded copy. Undo and redo first verify that each file is still
//! exactly as the journal left it, then restore through a verified sibling
//! staging file. Files changed elsewhere are reported and never overwritten.
//!
//! Retained bytes are counted by what each snapshot really keeps on disk. A
//! staged write that clones the original and rewrites only a region in place
//! leaves the hard-linked snapshot sharing every other block with the new
//! file, so it costs the rewritten region plus a small overhead; full copies
//! and full rewrites cost the whole file.
//!
//! Operations may also create or replace small files beside the audio, such
//! as a cover image saved by a tag source import. These side files are copied
//! whole, absent or not, and undo puts back exactly what was there before.
//! Symbolic-link side entries retain their link target verbatim; their
//! referents are never captured or changed.
#[cfg(all(windows, test))]
use crate::file_baseline::windows_legacy_identity;
use crate::{
    batch_errors::{push_error, BatchError, Detailed, Direction, ErrorCause},
    file_baseline::{fingerprint_diagnostic, metadata_stamp, Fingerprint},
    FileError, TagOptions,
};
use serde::{Deserialize, Serialize};
#[path = "history_store.rs"]
mod store;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Filesystem allocation granularity used to round rewritten regions.
const BLOCK: u64 = 4096;
/// Per-file allowance for block rounding and filesystem metadata of a clone.
const OVERHEAD: u64 = 16 * 1024;
/// Planning estimate for one clone-backed file before its write reports the
/// real region (a metadata region with typical artwork).
const ESTIMATE: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug)]
struct Writes {
    count: u32,
    /// Bytes rewritten in clones so far, or `None` once any write was a full
    /// copy or a full rewrite.
    rewritten: Option<u64>,
}

struct Watched {
    owner: Option<uuid::Uuid>,
    current: PathBuf,
    writes: Writes,
    expected: Fingerprint,
    valid: bool,
}
#[derive(Clone, Copy)]
struct TrackedWrite {
    writes: Writes,
    expected: Fingerprint,
    valid: bool,
}

/// Original journal identities and their current paths during staged operations.
struct WriteTracking {
    watched: BTreeMap<PathBuf, Watched>,
    paths: BTreeMap<PathBuf, PathBuf>,
}

static WRITES: Mutex<WriteTracking> = Mutex::new(WriteTracking {
    watched: BTreeMap::new(),
    paths: BTreeMap::new(),
});

/// Called by staged writes after replacing `path`.
pub(crate) fn note_write(path: &Path, cloned: bool, rewritten: Option<u64>, produced: Fingerprint) {
    if let Ok(mut tracking) = WRITES.lock() {
        let WriteTracking { watched, paths } = &mut *tracking;
        if let Some(entry) = paths
            .get(path)
            .and_then(|original| watched.get_mut(original))
        {
            entry.expected = produced;
            entry.writes.count += 1;
            entry.writes.rewritten = match (entry.writes.rewritten, cloned, rewritten) {
                (Some(total), true, Some(bytes)) => Some(total.saturating_add(bytes)),
                _ => None,
            };
        }
    }
}

/// Check the capture epoch before changing a watched file. Preserve the caller's
/// leaf path so canonicalization cannot hide a substituted symlink.
pub(crate) fn captured_source_matches(
    requested: &Path,
    resolved: &Path,
    current: Fingerprint,
) -> bool {
    let Ok(mut tracking) = WRITES.lock() else {
        return false;
    };
    // Resolve directory aliases (/var versus /private/var on macOS), while
    // preserving the leaf whose substitution we need to detect.
    let parent = requested
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let leaf = requested.file_name().and_then(|name| {
        dunce::canonicalize(parent)
            .ok()
            .map(|parent| parent.join(name))
    });
    let requested = leaf.as_deref().unwrap_or(requested);
    let raw_watched = tracking.paths.contains_key(requested);
    let original = tracking
        .paths
        .get(requested)
        .or_else(|| tracking.paths.get(resolved))
        .cloned();
    let Some(original) = original else {
        return true;
    };
    let Some(entry) = tracking.watched.get_mut(&original) else {
        return false;
    };
    let regular_leaf = !raw_watched || fs::symlink_metadata(requested).is_ok_and(|m| m.is_file());
    entry.valid &= regular_leaf && entry.expected == current;
    entry.valid
}

/// Follow a completed file/folder move without changing its journal identity.
/// A copied move cannot share the old snapshot's blocks with later clones.
pub(crate) fn note_move(from: &Path, to: &Path, copied: bool) {
    if from == to {
        return;
    }
    if let Ok(mut tracking) = WRITES.lock() {
        let moved: Vec<_> = tracking
            .paths
            .range(from.to_path_buf()..)
            .take_while(|(current, _)| current.starts_with(from))
            .filter_map(|(current, original)| {
                current
                    .strip_prefix(from)
                    .ok()
                    .map(|suffix| (current.clone(), original.clone(), to.join(suffix)))
            })
            .collect();
        for (current, original, destination) in moved {
            tracking.paths.remove(&current);
            tracking.paths.insert(destination.clone(), original.clone());
            if let Some(entry) = tracking.watched.get_mut(&original) {
                if copied {
                    match fingerprint(&destination) {
                        Ok(current) if current.same_content(&entry.expected) => {
                            entry.expected = current
                        }
                        _ => entry.valid = false,
                    }
                    entry.writes.rewritten = None;
                }
                entry.current = destination;
            }
        }
    }
}

#[cfg(test)]
fn watch(paths: impl IntoIterator<Item = (PathBuf, Fingerprint)>) {
    watch_owned(paths, None);
}

fn watch_owned(paths: impl IntoIterator<Item = (PathBuf, Fingerprint)>, owner: Option<uuid::Uuid>) {
    if let Ok(mut tracking) = WRITES.lock() {
        for (path, expected) in paths {
            if let Some(old) = tracking.watched.remove(&path) {
                tracking.paths.remove(&old.current);
            }
            tracking.paths.insert(path.clone(), path.clone());
            tracking.watched.insert(
                path.clone(),
                Watched {
                    owner,
                    current: path,
                    expected,
                    valid: true,
                    writes: Writes {
                        count: 0,
                        rewritten: Some(0),
                    },
                },
            );
        }
    }
}

#[cfg(test)]
fn unwatch<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> BTreeMap<PathBuf, TrackedWrite> {
    unwatch_owned(paths, None)
}

fn unwatch_owned<'a>(
    paths: impl IntoIterator<Item = &'a PathBuf>,
    owner: Option<uuid::Uuid>,
) -> BTreeMap<PathBuf, TrackedWrite> {
    let Ok(mut tracking) = WRITES.lock() else {
        return BTreeMap::new();
    };
    paths
        .into_iter()
        .filter_map(|path| {
            if !tracking
                .watched
                .get(path)
                .is_some_and(|entry| entry.owner == owner)
            {
                return None;
            }
            tracking.watched.remove(path).map(|entry| {
                if tracking.paths.get(&entry.current) == Some(path) {
                    tracking.paths.remove(&entry.current);
                }
                (
                    path.clone(),
                    TrackedWrite {
                        writes: entry.writes,
                        expected: entry.expected,
                        valid: entry.valid,
                    },
                )
            })
        })
        .collect()
}

fn unwatch_owner(owner: uuid::Uuid) {
    let Ok(mut tracking) = WRITES.lock() else {
        return;
    };
    let originals: Vec<_> = tracking
        .watched
        .iter()
        .filter(|(_, entry)| entry.owner == Some(owner))
        .map(|(original, _)| original.clone())
        .collect();
    for original in originals {
        if let Some(entry) = tracking.watched.remove(&original) {
            if tracking.paths.get(&entry.current) == Some(&original) {
                tracking.paths.remove(&entry.current);
            }
        }
    }
}

struct WatchLease(uuid::Uuid);
impl Drop for WatchLease {
    fn drop(&mut self) {
        unwatch_owner(self.0);
    }
}

/// Bytes a snapshot of a `len`-byte file keeps on disk after the operation.
/// Only a hard-linked snapshot whose writes all cloned and rewrote a region is
/// cheaper than the whole file.
fn retained(len: u64, linked: bool, writes: Option<Writes>) -> u64 {
    match writes {
        Some(Writes {
            count: 1..,
            rewritten: Some(bytes),
        }) if linked => (bytes.div_ceil(BLOCK) * BLOCK + OVERHEAD).min(len),
        _ => len,
    }
}

fn fingerprint(path: &Path) -> Result<Fingerprint, String> {
    fingerprint_diagnostic(path).map_err(|cause| cause.to_string())
}

/// Bounds for retained snapshots. An operation whose snapshots would retain
/// more than `bytes` is not recorded, so the journal never keeps more than
/// this much snapshot data. On Unix, journaling is also refused when it would
/// leave less than `min_free` bytes available on the journal's volume; other
/// platforms currently have no free-space query here.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub operations: usize,
    pub bytes: u64,
    pub min_free: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            operations: 50,
            bytes: 2 * 1024 * 1024 * 1024,
            min_free: 2 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    #[serde(with = "store::blob_name")]
    path: PathBuf,
    fingerprint: Fingerprint,
    /// A hard link to the file's previous inode rather than a copy.
    linked: bool,
    /// Bytes this snapshot keeps on disk.
    cost: u64,
    /// Side-file links retain the entry target verbatim, not its referent.
    #[serde(with = "store::optional_path")]
    symlink_target: Option<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Side {
    #[serde(with = "store::native_path")]
    path: PathBuf,
    blob: Option<Blob>,
    fingerprint: Fingerprint,
    parents: Vec<store::DirectoryGuard>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    before: Side,
    after: Side,
    content: bool,
    /// What one snapshot of this file retains when its counterpart was
    /// written by cloning (see [`retained`]).
    cost: u64,
}
impl FileRecord {
    /// The side the file is on now, and the side a replay moves it to.
    fn sides(&mut self, backward: bool) -> (&mut Side, &mut Side) {
        if backward {
            (&mut self.after, &mut self.before)
        } else {
            (&mut self.before, &mut self.after)
        }
    }
}
/// A file that is not audio, created or replaced by an operation: its
/// complete bytes or symlink entry on each side, or `None` where absent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SideFile {
    /// Path at the capture epoch, before subsequent directory moves.
    #[serde(with = "store::native_path")]
    path: PathBuf,
    epoch: usize,
    created_parents: Vec<store::DirectoryGuard>,
    before: Option<Blob>,
    after: Option<Blob>,
    parents: Vec<store::DirectoryGuard>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    label: String,
    group: Option<String>,
    files: Vec<FileRecord>,
    #[serde(with = "store::moves")]
    directories: Vec<(PathBuf, PathBuf)>,
    side_files: Vec<SideFile>,
    directory_guards: Vec<store::DirectoryGuard>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecoveryKind {
    Capture,
    Undo,
    Redo,
    RetiredHistory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryOperation {
    kind: RecoveryKind,
    record: Record,
}
impl Record {
    fn blobs(&self) -> impl Iterator<Item = &Blob> {
        self.files
            .iter()
            .flat_map(|f| [f.before.blob.as_ref(), f.after.blob.as_ref()])
            .chain(
                self.side_files
                    .iter()
                    .flat_map(|f| [f.before.as_ref(), f.after.as_ref()]),
            )
            .flatten()
    }
    fn bytes(&self) -> u64 {
        self.blobs().map(|b| b.cost).sum()
    }
    fn rebase_directory_ids(&mut self, changes: &store::DirectoryChanges) {
        for file in &mut self.files {
            store::replace_guard_ids(&mut file.before.parents, changes);
            store::replace_guard_ids(&mut file.after.parents, changes);
        }
        for side in &mut self.side_files {
            side.rebase_directory_ids(changes);
        }
        store::replace_guard_ids(&mut self.directory_guards, changes);
    }
}
impl SideFile {
    fn rebase_directory_ids(&mut self, changes: &store::DirectoryChanges) {
        store::replace_guard_ids(&mut self.parents, changes);
        store::replace_guard_ids(&mut self.created_parents, changes);
    }
}

type Moved = Vec<(PathBuf, Fingerprint, Fingerprint)>;

struct Captured {
    raw: String,
    path: PathBuf,
    fingerprint: Fingerprint,
    blob: Blob,
    parents: Vec<store::DirectoryGuard>,
}
struct SideCapture {
    path: PathBuf,
    before: Option<Blob>,
    epoch: usize,
    created_parents: Vec<PathBuf>,
    parents: Vec<store::DirectoryGuard>,
}
/// Snapshots taken before one operation; pass it back to [`Journal::commit`].
pub struct Capture {
    transaction: Option<uuid::Uuid>,
    _watcher: Option<WatchLease>,
    label: String,
    group: Option<String>,
    files: Vec<Captured>,
    side_files: Vec<SideCapture>,
    planned_bytes: u64,
    abandoned: Option<String>,
}

impl Capture {
    fn recovery_operation(&self) -> RecoveryOperation {
        RecoveryOperation {
            kind: RecoveryKind::Capture,
            record: Record {
                label: self.label.clone(),
                group: None,
                files: self
                    .files
                    .iter()
                    .map(|file| FileRecord {
                        before: Side {
                            path: file.path.clone(),
                            blob: Some(file.blob.clone()),
                            fingerprint: file.fingerprint,
                            parents: file.parents.clone(),
                        },
                        after: Side {
                            path: file.path.clone(),
                            blob: None,
                            fingerprint: file.fingerprint,
                            parents: file.parents.clone(),
                        },
                        content: true,
                        cost: file.blob.cost,
                    })
                    .collect(),
                directories: Vec::new(),
                directory_guards: Vec::new(),
                side_files: self
                    .side_files
                    .iter()
                    .map(|side| SideFile {
                        path: side.path.clone(),
                        epoch: 0,
                        created_parents: Vec::new(),
                        before: side.before.clone(),
                        after: None,
                        parents: side.parents.clone(),
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Step {
    pub label: String,
    pub files: usize,
}
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct HistoryState {
    pub undo: Option<Step>,
    pub redo: Option<Step>,
    pub notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoverySummary>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecoverySummary {
    pub operations: usize,
    pub copies: usize,
    pub bytes: u64,
}
/// The outcome of one undo or redo: final paths of restored files, path
/// changes in the same shape as rename results, and per-file failures.
#[derive(Clone, Debug, Default)]
pub struct Replay {
    pub label: String,
    pub paths: Vec<String>,
    pub errors: Vec<FileError>,
    pub renamed: Vec<(String, String)>,
    pub directories: Vec<(String, String)>,
}

pub struct Journal {
    dir: PathBuf,
    store_id: store::DirectoryId,
    lock_stamp: crate::file_baseline::MetadataStamp,
    limits: Limits,
    undo: Vec<Record>,
    redo: Vec<Record>,
    next: u64,
    notice: Option<String>,
    abandoned_group: Option<String>,
    options: TagOptions,
    pending: bool,
    intent: Option<RecoveryOperation>,
    recovery: Vec<RecoveryOperation>,
    store_failed: bool,
    active_capture: Option<uuid::Uuid>,
    outcome_revision: u64,
    _lock: fs::File,
}

const LOCK: &str = ".session-lock";

impl Journal {
    /// Open the exclusively locked durable store. Completed operations survive
    /// process exit; malformed or interrupted state is never replayed implicitly.
    pub fn open(root: &Path, limits: Limits) -> Result<Self, String> {
        store::open(root, limits)
    }

    /// Tag preferences used to verify that a restored copy still reads as audio.
    /// Restores put back the exact previous bytes, so no tag is rewritten.
    pub fn set_tag_options(&mut self, options: TagOptions) {
        self.options = options;
    }

    /// Process-local revision of new commit results (a recorded change or a
    /// fresh refusal). Loading existing history and no-op captures do not advance it.
    pub fn outcome_revision(&self) -> u64 {
        self.outcome_revision
    }

    pub fn state(&self) -> HistoryState {
        let step = |r: &Record| Step {
            label: r.label.clone(),
            files: r.files.len(),
        };
        HistoryState {
            undo: (!self.store_failed)
                .then(|| self.undo.last().map(step))
                .flatten(),
            redo: (!self.store_failed)
                .then(|| self.redo.last().map(step))
                .flatten(),
            notice: self.notice.clone(),
            recovery: (!self.recovery.is_empty()).then(|| RecoverySummary {
                operations: self.recovery.len(),
                copies: self
                    .recovery
                    .iter()
                    .flat_map(|op| op.record.blobs())
                    .count(),
                bytes: self.recovery.iter().map(|op| op.record.bytes()).sum(),
            }),
        }
    }

    /// Bytes currently retained by audio and side-file snapshots.
    pub fn retained_bytes(&self) -> u64 {
        self.undo
            .iter()
            .chain(&self.redo)
            .chain(self.recovery.iter().map(|op| &op.record))
            .map(Record::bytes)
            .sum()
    }

    /// Copy retained recovery data into a new folder without changing originals
    /// or consuming the recovery entries.
    pub fn export_recovery(&self, destination: &Path) -> Result<PathBuf, String> {
        store::export_recovery(self, destination)
    }

    fn blob_path(&mut self) -> Result<PathBuf, ErrorCause> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| ErrorCause::owned("The undo snapshot counter is exhausted"))?;
        Ok(self.dir.join(format!("{}.snapshot", self.next)))
    }

    fn create_blob(&mut self, path: &Path) -> Result<fs::File, ErrorCause> {
        self.validate_store().map_err(ErrorCause::external)?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists
                || fs::symlink_metadata(path).is_ok()
            {
                self.store_failed = true;
            }
            ErrorCause::external(error)
        })
    }

    // Prefer a hard link: the staged write replaces the inode, so the link keeps
    // the previous bytes for free. Copies are verified against a stable source.
    fn snapshot(&mut self, source: &Path, expected: Fingerprint) -> Result<Blob, String> {
        self.snapshot_diagnostic(source, expected)
            .map_err(|cause| cause.to_string())
    }
    fn snapshot_diagnostic(
        &mut self,
        source: &Path,
        expected: Fingerprint,
    ) -> Result<Blob, ErrorCause> {
        self.validate_store().map_err(ErrorCause::external)?;
        let path = self.blob_path()?;
        let mut created = false;
        let result = (|| {
            let linked = match fs::hard_link(source, &path) {
                Ok(()) => {
                    created = true;
                    true
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    self.store_failed = true;
                    return Err(ErrorCause::external(error));
                }
                Err(_) => {
                    let mut output = self.create_blob(&path)?;
                    created = true;
                    use std::io::Read;
                    let input = fs::File::open(source).map_err(ErrorCause::external)?;
                    std::io::copy(&mut input.take(expected.len.saturating_add(1)), &mut output)
                        .map_err(ErrorCause::external)?;
                    output.sync_all().map_err(ErrorCause::external)?;
                    false
                }
            };
            store::sync_path(&path).map_err(ErrorCause::external)?;
            let print = fingerprint_diagnostic(&path)?;
            if !print.same_content(&expected) || fingerprint_diagnostic(source)? != expected {
                return Err(ErrorCause::owned(
                    "The file changed while its undo snapshot was taken",
                ));
            }
            Ok(Blob {
                path: path.clone(),
                fingerprint: print,
                linked,
                cost: expected.len,
                symlink_target: None,
            })
        })();
        if result.is_err() && created {
            let _ = fs::remove_file(&path);
        }
        result
    }

    /// Snapshot `paths` before an operation writes them. Operations sharing a
    /// `group` (one Save click writing files one by one) merge into one step.
    pub fn begin(&mut self, label: &str, group: Option<&str>, paths: &[String]) -> Capture {
        if self.validate_store().is_err() {
            self.store_failed = true;
        }
        if self.store_failed || self.pending {
            return Capture {
                transaction: None,
                _watcher: None,
                label: label.into(),
                group: group.map(str::to_owned),
                files: Vec::new(),
                side_files: Vec::new(),
                planned_bytes: 0,
                abandoned: Some("history has an interrupted or unsaved operation".into()),
            };
        }
        let mut capture = self.begin_inner(label, group, paths);
        if capture.abandoned.is_none() {
            let transaction = uuid::Uuid::new_v4();
            self.active_capture = Some(transaction);
            capture.transaction = Some(transaction);
            watch_owned(
                capture
                    .files
                    .iter()
                    .map(|file| (file.path.clone(), file.fingerprint)),
                Some(transaction),
            );
            capture._watcher = Some(WatchLease(transaction));
            self.intent = Some(capture.recovery_operation());
            let result = capture
                .files
                .iter()
                .try_for_each(|file| store::sync_blob(&file.blob))
                .and_then(|()| self.checkpoint(true));
            if let Err(error) = result {
                capture.abandoned = Some(format!("history could not be prepared: {error}"));
                self.store_failed = true;
            }
        }
        capture
    }

    fn begin_inner(&mut self, label: &str, group: Option<&str>, paths: &[String]) -> Capture {
        let mut capture = Capture {
            transaction: None,
            _watcher: None,
            label: label.into(),
            group: group.map(str::to_owned),
            files: Vec::new(),
            side_files: Vec::new(),
            planned_bytes: 0,
            abandoned: None,
        };
        if group.is_some() && self.abandoned_group.as_deref() == group {
            capture.abandoned = Some("the operation is too large to undo".into());
            return capture;
        }
        let mut seen = std::collections::HashSet::new();
        let mut targets = Vec::new();
        for raw in paths {
            let Ok(path) = dunce::canonicalize(Path::new(raw)) else {
                continue;
            };
            let print = match metadata_stamp(&path).map_err(|cause| cause.to_string()) {
                Ok(print) => print,
                Err(error) => {
                    #[cfg(windows)]
                    {
                        capture.abandoned = Some(format!("an undo snapshot failed: {error}"));
                        return capture;
                    }
                    #[cfg(not(windows))]
                    {
                        let _ = error;
                        continue;
                    }
                }
            };
            if seen.insert(path.clone()) {
                targets.push((raw.clone(), path, print));
            }
        }
        let grouped = match self.undo.last() {
            Some(top) if group.is_some() && top.group.as_deref() == group => top.bytes(),
            _ => 0,
        };
        // Files on the journal's volume that clone keep only rewritten blocks;
        // everything else keeps (or copies) the whole file.
        let journal = volume_id(&self.dir);
        let mut clones = BTreeMap::new();
        let (mut full, mut estimate) = (0u64, 0u64);
        for (_, path, print) in &targets {
            let cheap = journal.is_some()
                && volume_id(path) == journal
                && path.parent().is_some_and(|dir| {
                    *clones
                        .entry(dir.to_path_buf())
                        .or_insert_with(|| crate::clones_in(dir))
                });
            if cheap {
                estimate = estimate.saturating_add(ESTIMATE.min(print.len));
            } else {
                full = full.saturating_add(print.len);
                estimate = estimate.saturating_add(print.len);
            }
        }
        capture.planned_bytes = grouped.saturating_add(estimate);
        if grouped.saturating_add(full) > self.limits.bytes {
            capture.abandoned = Some("the operation is too large to undo".into());
            return capture;
        }
        if available_space(&self.dir)
            .is_some_and(|free| free < estimate.saturating_add(self.limits.min_free))
        {
            capture.abandoned = Some("there is not enough free disk space for undo".into());
            return capture;
        }
        for (raw, path, stamp) in targets {
            let print = match fingerprint(&path) {
                Ok(print) if print.matches_metadata(&stamp) => print,
                result => {
                    capture.files.iter().for_each(|f| discard(&f.blob));
                    capture.files.clear();
                    let error = result.err().unwrap_or_else(|| {
                        "The file changed while its undo snapshot was taken".into()
                    });
                    capture.abandoned = Some(format!("an undo snapshot failed: {error}"));
                    return capture;
                }
            };
            let parents = match store::parent_guards(&path) {
                Ok(parents) => parents,
                Err(error) => {
                    capture.files.clear();
                    capture.abandoned =
                        Some(format!("an undo parent could not be captured: {error}"));
                    return capture;
                }
            };
            match self.snapshot(&path, print) {
                Ok(blob) => capture.files.push(Captured {
                    raw,
                    path,
                    fingerprint: print,
                    blob,
                    parents,
                }),
                Err(error) => {
                    capture.files.iter().for_each(|f| discard(&f.blob));
                    capture.files.clear();
                    capture.abandoned = Some(format!("an undo snapshot failed: {error}"));
                    return capture;
                }
            }
        }
        capture
    }

    /// Call before the operation of `capture` creates or replaces `path`, a
    /// file that is not audio (a saved cover image). Undo then puts back the
    /// previous file or removes the new one. A capture that is not recorded
    /// ignores the call; an error means the file should not be written.
    pub fn capture_side_file(&mut self, capture: &mut Capture, path: &Path) -> Result<(), String> {
        self.capture_side_file_after_moves(capture, path, 0)
    }

    /// Capture an entry after `completed_moves` directory moves in the list
    /// subsequently passed to `commit`. Only later moves carry this entry.
    pub fn capture_side_file_after_moves(
        &mut self,
        capture: &mut Capture,
        path: &Path,
        completed_moves: usize,
    ) -> Result<(), String> {
        self.capture_side_file_after_moves_diagnostic(capture, path, completed_moves)
            .map_err(|cause| cause.to_string())
    }
    pub fn capture_side_file_after_moves_diagnostic(
        &mut self,
        capture: &mut Capture,
        path: &Path,
        completed_moves: usize,
    ) -> Result<(), ErrorCause> {
        if capture.abandoned.is_some() {
            return Ok(());
        }
        let path = side_path(path)?;
        if capture
            .side_files
            .iter()
            .any(|known| known.path == path && known.epoch == completed_moves)
        {
            return Ok(());
        }
        let created_parents = missing_parents(&path)?;
        let parents = store::parent_guards(&path).map_err(ErrorCause::external)?;
        let retained = capture
            .side_files
            .iter()
            .filter_map(|side| side.before.as_ref())
            .fold(capture.planned_bytes, |total, blob| {
                total.saturating_add(blob.cost)
            });
        let before = self.copy_side_file(&path, retained)?;
        capture.side_files.push(SideCapture {
            path,
            before,
            epoch: completed_moves,
            created_parents,
            parents,
        });
        if let Some(blob) = capture
            .side_files
            .last()
            .and_then(|side| side.before.as_ref())
        {
            store::sync_blob(blob).map_err(ErrorCause::external)?;
        }
        self.intent = Some(capture.recovery_operation());
        if let Err(error) = self.checkpoint(true) {
            self.store_failed = true;
            capture.abandoned = Some(format!(
                "history could not retain its side-file intent: {error}"
            ));
            return Err(ErrorCause::external(error));
        }
        Ok(())
    }

    fn copy_side_file(&mut self, source: &Path, retained: u64) -> Result<Option<Blob>, ErrorCause> {
        self.validate_store().map_err(ErrorCause::external)?;
        match fs::symlink_metadata(source) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::FileTypeExt;
                    if metadata.file_type().is_symlink_dir() {
                        return Err(ErrorCause::owned(
                            "Cannot capture a directory symlink as a side file",
                        ));
                    }
                }
                let target = fs::read_link(source).map_err(ErrorCause::external)?;
                self.check_side_budget(
                    target.as_os_str().as_encoded_bytes().len() as u64,
                    retained,
                )?;
                // Prove this platform can recreate the entry before allowing
                // its replacement. The private sibling probe is always removed.
                let folder = source
                    .parent()
                    .ok_or_else(|| ErrorCause::owned("The side file has no folder"))?;
                let _probe = stage_side_link_diagnostic(&target, folder)?;
                let path = self.blob_path()?;
                let mut output = self.create_blob(&path)?;
                use std::io::Write;
                if let Err(error) = output
                    .write_all(target.as_os_str().as_encoded_bytes())
                    .and_then(|()| output.sync_all())
                {
                    drop(output);
                    let _ = fs::remove_file(&path);
                    return Err(ErrorCause::external(error));
                }
                drop(output);
                let fingerprint = match fingerprint_diagnostic(&path) {
                    Ok(print) => print,
                    Err(error) => {
                        let _ = fs::remove_file(&path);
                        return Err(error);
                    }
                };
                Ok(Some(Blob {
                    path,
                    cost: fingerprint.len,
                    fingerprint,
                    linked: false,
                    symlink_target: Some(target),
                }))
            }
            Ok(metadata) if metadata.is_file() => {
                self.check_side_budget(metadata.len(), retained)?;
                self.copy_blob(source, metadata.len()).map(Some)
            }
            Ok(_) => Err(ErrorCause::message(
                "{value} is not a regular file or symlink",
                [("value", display(source))],
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ErrorCause::external(error)),
        }
    }

    fn check_side_budget(&self, bytes: u64, retained: u64) -> Result<(), ErrorCause> {
        if retained.saturating_add(bytes) > self.limits.bytes {
            return Err(ErrorCause::owned(
                "The side-file undo snapshot exceeds the undo byte limit",
            ));
        }
        if available_space(&self.dir)
            .is_some_and(|free| free < bytes.saturating_add(self.limits.min_free))
        {
            return Err(ErrorCause::owned(
                "There is not enough free disk space for the side-file undo snapshot",
            ));
        }
        Ok(())
    }

    /// Side files may be rewritten in place, so keep a bounded complete copy.
    fn copy_blob(&mut self, source: &Path, bytes: u64) -> Result<Blob, ErrorCause> {
        use std::io::Read;
        let expected = fingerprint_diagnostic(source)?;
        if expected.len != bytes {
            return Err(ErrorCause::owned(
                "The side file changed size while its undo snapshot was taken",
            ));
        }
        let path = self.blob_path()?;
        let mut output = self.create_blob(&path)?;
        let result = (|| {
            let mut input = fs::File::open(source).map_err(ErrorCause::external)?;
            let copied = std::io::copy(&mut (&mut input).take(bytes), &mut output)
                .map_err(ErrorCause::external)?;
            let mut extra = [0];
            if copied != bytes || input.read(&mut extra).map_err(ErrorCause::external)? != 0 {
                return Err(ErrorCause::owned(
                    "The side file changed size while its undo snapshot was taken",
                ));
            }
            output.sync_all().map_err(ErrorCause::external)?;
            drop(output);
            let print = fingerprint_diagnostic(&path)?;
            if !print.same_content(&expected) || fingerprint_diagnostic(source)? != expected {
                return Err(ErrorCause::owned(
                    "The file changed while its undo snapshot was taken",
                ));
            }
            Ok(Blob {
                fingerprint: print,
                path: path.clone(),
                cost: bytes,
                linked: false,
                symlink_target: None,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(&path);
        }
        result
    }

    /// Side files as the operation left them; unchanged ones are dropped.
    /// Also returns how many could not be recorded.
    fn commit_side_files(
        &mut self,
        captured: Vec<SideCapture>,
        directories: &[(PathBuf, PathBuf)],
        planned_bytes: u64,
    ) -> (Vec<SideFile>, usize) {
        let mut final_paths = std::collections::BTreeSet::new();
        let mut planned = Vec::new();
        let mut lost = 0;
        for capture in captured {
            let Some(later_moves) = directories.get(capture.epoch..) else {
                capture.before.iter().for_each(discard);
                lost += 1;
                continue;
            };
            let final_path = rebase(&capture.path, later_moves);
            if final_paths.insert(final_path.clone()) {
                planned.push((final_path, capture));
            } else {
                capture.before.iter().for_each(discard);
            }
        }
        let mut retained = planned
            .iter()
            .filter_map(|(_, capture)| capture.before.as_ref())
            .fold(planned_bytes, |total, blob| total.saturating_add(blob.cost));
        let mut recorded = Vec::new();
        for (path, capture) in planned {
            let SideCapture {
                path: captured_path,
                before,
                epoch,
                created_parents,
                parents: captured_parents,
            } = capture;
            let later_moves = &directories[epoch..];
            let guards =
                store::check_captured_guards(&captured_parents, later_moves).and_then(|()| {
                    Ok((
                        store::parent_guards_after_moves(&captured_path, later_moves)?,
                        store::created_parent_guards(created_parents, later_moves)?,
                    ))
                });
            let (parents, created_parents) = match guards {
                Ok(guards) => guards,
                Err(_) => {
                    lost += 1;
                    before.iter().for_each(discard);
                    retained = retained.saturating_sub(before.as_ref().map_or(0, |blob| blob.cost));
                    continue;
                }
            };
            let before_cost = before.as_ref().map_or(0, |blob| blob.cost);
            let after = match self.copy_side_file(&path, retained) {
                Ok(after) => after,
                Err(_) => {
                    before.iter().for_each(discard);
                    retained = retained.saturating_sub(before_cost);
                    lost += 1;
                    continue;
                }
            };
            let after_cost = after.as_ref().map_or(0, |blob| blob.cost);
            retained = retained.saturating_add(after_cost);
            let unchanged = match (&before, &after) {
                (None, None) => true,
                (Some(old), Some(new)) => {
                    old.symlink_target == new.symlink_target && same_bytes(&old.path, &new.path)
                }
                _ => false,
            };
            if unchanged {
                retained = retained
                    .saturating_sub(before_cost)
                    .saturating_sub(after_cost);
                [before.as_ref(), after.as_ref()]
                    .into_iter()
                    .flatten()
                    .for_each(discard);
                continue;
            }
            recorded.push(SideFile {
                path: captured_path,
                epoch,
                created_parents,
                before,
                after,
                parents,
            });
        }
        (recorded, lost)
    }

    /// Record what the operation changed. `renamed` maps an input path (as
    /// passed to `begin`, or canonical) to its final path; `directories` lists
    /// directory moves in the order they happened.
    pub fn commit(
        &mut self,
        capture: Capture,
        renamed: &[(String, String)],
        directories: &[(String, String)],
    ) {
        if (self.pending || self.active_capture.is_some() || capture.transaction.is_some())
            && (capture.transaction.is_none() || capture.transaction != self.active_capture)
        {
            self.notice =
                Some("An overlapping operation cannot be recorded in Undo history".into());
            self.outcome_revision = self.outcome_revision.wrapping_add(1);
            return;
        }
        let mut changed = self.commit_inner(capture, renamed, directories);
        self.active_capture = None;
        if !self.store_failed {
            if let Err(error) = self.checkpoint(false) {
                self.store_failed = true;
                self.notice = Some(format!("Undo history could not be saved: {error}"));
                changed = true;
            }
        }
        if changed {
            self.outcome_revision = self.outcome_revision.wrapping_add(1);
        }
    }

    fn commit_inner(
        &mut self,
        mut capture: Capture,
        renamed: &[(String, String)],
        directories: &[(String, String)],
    ) -> bool {
        let directories: Vec<(PathBuf, PathBuf)> = directories
            .iter()
            .map(|(from, to)| {
                let normalize =
                    |raw: &str| side_path(Path::new(raw)).unwrap_or_else(|_| PathBuf::from(raw));
                (normalize(from), normalize(to))
            })
            .collect();
        let writes = unwatch_owned(capture.files.iter().map(|f| &f.path), capture.transaction);
        let directory_guards = match store::directory_guards(&directories) {
            Ok(guards) => guards,
            Err(error) => {
                capture.abandoned = Some(format!(
                    "undo directory identities could not be retained: {error}"
                ));
                Vec::new()
            }
        };
        let (mut side_files, side_lost) = if capture.abandoned.is_some() {
            capture
                .side_files
                .iter()
                .filter_map(|side| side.before.as_ref())
                .for_each(discard);
            (Vec::new(), 0)
        } else {
            self.commit_side_files(capture.side_files, &directories, capture.planned_bytes)
        };
        // A missing epoch entry can obstruct the directory replay itself.
        // Do not offer a partial folder operation without all its side entries.
        if side_lost > 0 && !directories.is_empty() {
            for side in side_files.drain(..) {
                [side.before.as_ref(), side.after.as_ref()]
                    .into_iter()
                    .flatten()
                    .for_each(discard);
            }
            capture.abandoned = Some("side-file undo snapshots could not be retained".into());
        }
        if let Some(reason) = capture.abandoned {
            capture.files.iter().for_each(|file| discard(&file.blob));
            if capture.group.is_some() {
                if let Some(record) = self.undo.pop_if(|top| top.group == capture.group) {
                    discard_record(&record);
                }
                self.abandoned_group = capture.group.clone();
            }
            self.clear_redo();
            self.notice = Some(format!("{} cannot be undone: {reason}.", capture.label));
            return true;
        }
        let mut files = Vec::new();
        let mut lost = side_lost;
        for captured in capture.files {
            let to = renamed
                .iter()
                .find(|(from, _)| {
                    from == &captured.raw || Path::new(from) == captured.path.as_path()
                })
                .map(|(_, to)| PathBuf::from(to))
                .unwrap_or_else(|| rebase(&captured.path, &directories));
            let to = dunce::canonicalize(&to).unwrap_or(to);
            let parents = store::check_captured_guards(&captured.parents, &directories)
                .and_then(|()| store::parent_guards(&to));
            let Ok(parents) = parents else {
                discard(&captured.blob);
                lost += 1;
                continue;
            };
            let Ok(after) = fingerprint(&to) else {
                // The operation removed or lost the file; there is nothing to restore onto.
                discard(&captured.blob);
                lost += 1;
                continue;
            };
            if writes.get(&captured.path).is_some_and(|track| {
                !track.valid || track.writes.count > 0 && track.expected != after
            }) {
                discard(&captured.blob);
                lost += 1;
                continue;
            }
            // A confirmed staged replacement still needs its snapshot when size
            // and mtime are preserved and the platform supplies no file identity.
            let content = after != captured.fingerprint
                || writes
                    .get(&captured.path)
                    .is_some_and(|write| write.writes.count > 0);
            if !content && to == captured.path {
                discard(&captured.blob);
                continue;
            }
            // A hard-linked snapshot is only valid if nothing wrote into the old inode.
            if content && fingerprint(&captured.blob.path).ok() != Some(captured.blob.fingerprint) {
                discard(&captured.blob);
                lost += 1;
                continue;
            }
            let len = captured.fingerprint.len;
            let cost = retained(
                len,
                true,
                writes.get(&captured.path).map(|track| track.writes),
            );
            let blob = if content {
                let mut blob = captured.blob;
                blob.cost = if blob.linked { cost } else { len };
                Some(blob)
            } else {
                discard(&captured.blob);
                None
            };
            files.push(FileRecord {
                before: Side {
                    path: captured.path,
                    blob,
                    fingerprint: captured.fingerprint,
                    parents: captured.parents,
                },
                after: Side {
                    path: to,
                    blob: None,
                    fingerprint: after,
                    parents,
                },
                content,
                cost,
            });
        }
        if files.is_empty() && directories.is_empty() && side_files.is_empty() {
            if lost > 0 {
                self.notice = Some(format!(
                    "{} cannot be undone for {lost} file{}.",
                    capture.label,
                    if lost == 1 { "" } else { "s" }
                ));
            }
            return lost > 0;
        }
        self.clear_redo();
        self.notice = (lost > 0).then(|| {
            format!(
                "{} cannot be undone for {lost} file{}.",
                capture.label,
                if lost == 1 { "" } else { "s" }
            )
        });
        if capture.group.is_none() || self.abandoned_group != capture.group {
            self.abandoned_group = None;
        }
        let merge = capture.group.is_some()
            && directories.is_empty()
            && self
                .undo
                .last()
                .is_some_and(|top| top.group == capture.group && top.directories.is_empty());
        if merge {
            let top = self.undo.last_mut().expect("checked top record");
            for file in files {
                if let Some(existing) = top
                    .files
                    .iter_mut()
                    .find(|f| f.after.path == file.before.path)
                {
                    if !existing.content {
                        existing.before.blob = file.before.blob;
                        existing.cost = file.cost;
                    } else if let Some(blob) = &file.before.blob {
                        // The first snapshot now also keeps what this write replaced.
                        discard(blob);
                        let whole = existing.before.fingerprint.len;
                        existing.cost = existing.cost.saturating_add(file.cost).min(whole);
                        if let Some(first) = existing.before.blob.as_mut() {
                            first.cost = if first.linked { existing.cost } else { whole };
                        }
                    }
                    existing.content |= file.content;
                    existing.after = file.after;
                } else {
                    top.files.push(file);
                }
            }
            for side in side_files {
                match top.side_files.iter_mut().find(|f| f.path == side.path) {
                    // The step still starts from the first write's previous file.
                    Some(existing) => {
                        let SideFile { before, after, .. } = side;
                        before.iter().for_each(discard);
                        if let Some(old) = std::mem::replace(&mut existing.after, after) {
                            discard(&old);
                        }
                    }
                    None => top.side_files.push(side),
                }
            }
        } else {
            self.undo.push(Record {
                label: capture.label.clone(),
                group: capture.group.clone(),
                files,
                directories,
                side_files,
                directory_guards,
            });
        }
        let limit = self.limits.bytes;
        if let Some(record) = self.undo.pop_if(|top| top.bytes() > limit) {
            discard_record(&record);
            if capture.group.is_some() {
                self.abandoned_group = capture.group;
            }
            self.notice = Some(format!(
                "{} cannot be undone: the operation is too large to undo.",
                capture.label
            ));
        }
        self.enforce_limits();
        true
    }

    fn enforce_limits(&mut self) {
        let mut count = self.undo.len() + self.redo.len() + self.recovery.len();
        let mut bytes = self.retained_bytes();
        let mut recovery_removed = 0;
        for operation in &self.recovery {
            if count <= self.limits.operations && bytes <= self.limits.bytes {
                break;
            }
            count -= 1;
            bytes = bytes.saturating_sub(operation.record.bytes());
            recovery_removed += 1;
        }
        self.recovery.drain(..recovery_removed);
        if recovery_removed > 0 {
            self.notice = Some("Older recovery copies were removed to keep history within its operation and storage limits.".into());
        }
        let mut obsolete = [0, 0];
        for (index, records) in [&self.undo, &self.redo].into_iter().enumerate() {
            for record in records {
                if count <= self.limits.operations && bytes <= self.limits.bytes {
                    break;
                }
                count -= 1;
                bytes = bytes.saturating_sub(record.bytes());
                obsolete[index] += 1;
            }
        }
        for (records, count) in [(&mut self.undo, obsolete[0]), (&mut self.redo, obsolete[1])] {
            for record in records.drain(..count) {
                discard_record(&record);
            }
        }
        if obsolete != [0, 0] && self.undo.is_empty() && self.redo.is_empty() {
            self.notice = Some(
                "History was cleared because its retained storage exceeded the undo limit".into(),
            );
        }
    }

    fn clear_redo(&mut self) {
        for record in self.redo.drain(..) {
            discard_record(&record);
        }
    }

    fn prepare_replay(&mut self, record: &Record, backward: bool) -> Result<(), String> {
        self.intent = Some(RecoveryOperation {
            kind: if backward {
                RecoveryKind::Undo
            } else {
                RecoveryKind::Redo
            },
            record: record.clone(),
        });
        if let Err(error) = self.checkpoint(true) {
            self.store_failed = true;
            self.notice = Some(format!("Undo history could not be prepared: {error}"));
            return Err(error);
        }
        Ok(())
    }

    /// Restore the most recent operation. Errors when there is nothing to undo.
    pub fn undo(&mut self) -> Result<Replay, String> {
        self.undo_diagnostic().map(|report| report.result)
    }

    /// Undo with additive presentation companions; bookkeeping is shared with undo.
    pub fn undo_diagnostic(&mut self) -> Result<Detailed<Replay>, String> {
        if self.store_failed || self.pending {
            return Err("Undo history has an interrupted or unsaved operation".into());
        }
        if self.undo.is_empty() {
            return Err("There is nothing to undo".into());
        }
        let record = self.undo.pop().ok_or("There is nothing to undo")?;
        self.prepare_replay(&record, true)?;
        self.notice = None;
        let (record, replay, moved, directory_changes) = self.replay(record, true);
        self.rebase_directory_ids(&directory_changes);
        // The previous step now expects the restored file, not the bytes it wrote.
        for (path, old, new) in moved {
            let side = self
                .undo
                .iter_mut()
                .rev()
                .find_map(|r| r.files.iter_mut().find(|f| f.after.path == path));
            if let Some(file) = side.filter(|f| f.after.fingerprint == old) {
                file.after.fingerprint = new;
            }
        }
        if let Some(record) = record {
            self.redo.push(record);
        }
        self.enforce_limits();
        if !self.store_failed {
            if let Err(error) = self.checkpoint(false) {
                self.store_failed = true;
                self.notice = Some(format!("Undo history could not be saved: {error}"));
            }
        }
        Ok(replay)
    }

    /// Re-apply the most recently undone operation.
    pub fn redo(&mut self) -> Result<Replay, String> {
        self.redo_diagnostic().map(|report| report.result)
    }

    /// Redo with additive presentation companions; bookkeeping is shared with redo.
    pub fn redo_diagnostic(&mut self) -> Result<Detailed<Replay>, String> {
        if self.store_failed || self.pending {
            return Err("Undo history has an interrupted or unsaved operation".into());
        }
        if self.redo.is_empty() {
            return Err("There is nothing to redo".into());
        }
        let record = self.redo.pop().ok_or("There is nothing to redo")?;
        self.prepare_replay(&record, false)?;
        self.notice = None;
        let (record, replay, moved, directory_changes) = self.replay(record, false);
        self.rebase_directory_ids(&directory_changes);
        for (path, old, new) in moved {
            let side = self
                .redo
                .iter_mut()
                .rev()
                .find_map(|r| r.files.iter_mut().find(|f| f.before.path == path));
            if let Some(file) = side.filter(|f| f.before.fingerprint == old) {
                file.before.fingerprint = new;
            }
        }
        if let Some(record) = record {
            self.undo.push(record);
        }
        self.enforce_limits();
        if !self.store_failed {
            if let Err(error) = self.checkpoint(false) {
                self.store_failed = true;
                self.notice = Some(format!("Undo history could not be saved: {error}"));
            }
        }
        Ok(replay)
    }

    fn rebase_directory_ids(&mut self, changes: &store::DirectoryChanges) {
        for record in self.undo.iter_mut().chain(&mut self.redo) {
            record.rebase_directory_ids(changes);
        }
    }

    /// Move one record's files to its other side. Also returns, per restored
    /// file, its path with the fingerprint that side had before and has now.
    fn replay(
        &mut self,
        mut record: Record,
        backward: bool,
    ) -> (
        Option<Record>,
        Detailed<Replay>,
        Moved,
        store::DirectoryChanges,
    ) {
        let mut diagnostics = Vec::new();
        let mut moved = Vec::new();
        let mut directory_changes = Vec::new();
        let mut replay = Replay {
            label: record.label.clone(),
            ..Default::default()
        };
        let direction = if backward {
            Direction::Undo
        } else {
            Direction::Redo
        };
        // 1. Verify every file is exactly as this journal left it.
        let mut files = Vec::new();
        for mut file in std::mem::take(&mut record.files) {
            let content = file.content;
            let (from, _) = file.sides(backward);
            if store::check_guards(&from.parents).is_err()
                || store::safe_parents(&from.path).is_err()
                || fingerprint(&from.path).ok() != Some(from.fingerprint)
            {
                push_error(
                    &mut replay.errors,
                    &mut diagnostics,
                    display(&from.path),
                    BatchError::HistoryChanged {
                        label: record.label.clone(),
                        direction,
                    },
                );
                discard_file(&file);
                continue;
            }
            if content && from.blob.is_none() {
                let (path, print) = (from.path.clone(), from.fingerprint);
                match self.snapshot_diagnostic(&path, print) {
                    // Updated once the restore shows whether it cloned.
                    Ok(blob) => file.sides(backward).0.blob = Some(blob),
                    Err(error) => {
                        push_error(
                            &mut replay.errors,
                            &mut diagnostics,
                            display(&path),
                            BatchError::HistoryFailed {
                                direction,
                                cause: error,
                            },
                        );
                        discard_file(&file);
                        continue;
                    }
                }
            }
            files.push(file);
        }
        if files.is_empty() && record.directories.is_empty() && record.side_files.is_empty() {
            return (
                None,
                Detailed {
                    result: replay,
                    error_diagnostics: diagnostics,
                },
                moved,
                directory_changes,
            );
        }
        // Publish every reverse copy before the first folder, side-file or
        // audio mutation. A process dying during snapshot preparation has not
        // started replay; a process dying after this point has durable copies
        // for both eligible sides.
        if let Some(intent) = self.intent.as_mut() {
            intent.record = record.clone();
            intent.record.files = files.clone();
        }
        if let Err(error) = self.checkpoint(true) {
            self.store_failed = true;
            self.notice = Some(format!(
                "Undo history could not retain its replay copies: {error}"
            ));
            push_error(
                &mut replay.errors,
                &mut diagnostics,
                record.label.clone(),
                BatchError::HistoryFailed {
                    direction,
                    cause: ErrorCause::external(error),
                },
            );
            record.files = files;
            return (
                Some(record),
                Detailed {
                    result: replay,
                    error_diagnostics: diagnostics,
                },
                moved,
                directory_changes,
            );
        }
        // 2. Replay folder/side-file epochs together, rolling back completed steps on failure.
        let moves: Vec<(PathBuf, PathBuf)> = if backward {
            record
                .directories
                .iter()
                .rev()
                .map(|(from, to)| (to.clone(), from.clone()))
                .collect()
        } else {
            record.directories.clone()
        };
        let directory_outcome = store::check_guards(&record.directory_guards)
            .map_err(ErrorCause::external)
            .and_then(|()| {
                if record.directories.is_empty() {
                    Ok(Vec::new())
                } else {
                    replay_side_files_and_directories(
                        &mut record.side_files,
                        &record.directories,
                        backward,
                    )
                }
            });
        let changes = match directory_outcome {
            Ok(changes) => changes,
            Err(error) => {
                push_error(
                    &mut replay.errors,
                    &mut diagnostics,
                    record.label.clone(),
                    BatchError::HistoryFoldersFailed {
                        direction,
                        cause: error,
                    },
                );
                files.iter().for_each(discard_file);
                discard_record(&record);
                return (
                    None,
                    Detailed {
                        result: replay,
                        error_diagnostics: diagnostics,
                    },
                    moved,
                    directory_changes,
                );
            }
        };
        record.rebase_directory_ids(&changes);
        for file in &mut files {
            store::replace_guard_ids(&mut file.before.parents, &changes);
            store::replace_guard_ids(&mut file.after.parents, &changes);
        }
        directory_changes.extend(changes);
        replay.directories = moves
            .iter()
            .map(|(from, to)| (display(from), display(to)))
            .collect();
        // 3. Rename each file and restore its contents.
        for mut file in files {
            let (content, cost) = (file.content, file.cost);
            // A full rewrite shares nothing, so each snapshot keeps its whole file.
            let cheap = cost < file.before.fingerprint.len;
            let (from, to) = file.sides(backward);
            let current = rebase(&from.path, &moves);
            let outcome = (|| {
                store::check_guards(&to.parents).map_err(ErrorCause::external)?;
                if fingerprint_diagnostic(&current)? != from.fingerprint {
                    return Err(ErrorCause::owned(
                        "The file changed during restore; reload it and try again",
                    ));
                }
                if current != to.path {
                    crate::operations::rename_no_clobber_cause(&current, &to.path)?;
                }
                if content {
                    let restored = (|| {
                        let blob = to
                            .blob
                            .as_ref()
                            .ok_or_else(|| ErrorCause::owned("The undo snapshot is missing"))?;
                        let expected = fingerprint_diagnostic(&to.path)?;
                        if !expected.same_content(&from.fingerprint) {
                            return Err(ErrorCause::owned(
                                "The file changed during restore; reload it and try again",
                            ));
                        }
                        restore(blob, &to.path, &self.options, expected)
                    })();
                    match restored {
                        // The replaced file now shares all but the rewritten region
                        // with the snapshot it came from.
                        Ok(cloned) => {
                            if let Some(left) = from.blob.as_mut() {
                                left.cost = if cloned && left.linked && cheap {
                                    cost.min(left.fingerprint.len)
                                } else {
                                    left.fingerprint.len
                                };
                            }
                        }
                        Err(error) => {
                            if current != to.path {
                                let _ = crate::operations::rename_no_clobber(&to.path, &current);
                            }
                            return Err(error);
                        }
                    }
                }
                fingerprint_diagnostic(&to.path)
            })();
            match outcome {
                Ok(print) => {
                    moved.push((to.path.clone(), to.fingerprint, print));
                    to.fingerprint = print;
                    if from.path != to.path {
                        replay
                            .renamed
                            .push((display(&from.path), display(&to.path)));
                    }
                    replay.paths.push(display(&to.path));
                    record.files.push(file);
                }
                Err(error) => {
                    push_error(
                        &mut replay.errors,
                        &mut diagnostics,
                        display(&current),
                        BatchError::HistoryFailed {
                            direction,
                            cause: error,
                        },
                    );
                    discard_file(&file);
                }
            }
        }
        // Without directory moves, retain independent per-side failure handling.
        if record.directories.is_empty() {
            let mut sides = std::mem::take(&mut record.side_files);
            if backward {
                sides.reverse();
            }
            let mut sides = sides.into_iter();
            while let Some(mut side) = sides.next() {
                match replay_side_file(&mut side, backward) {
                    Ok(changes) => {
                        record.side_files.push(side);
                        record.rebase_directory_ids(&changes);
                        for pending in sides.as_mut_slice() {
                            pending.rebase_directory_ids(&changes);
                        }
                        directory_changes.extend(changes);
                    }
                    Err(error) => {
                        push_error(
                            &mut replay.errors,
                            &mut diagnostics,
                            display(&side.path),
                            BatchError::HistoryFailed {
                                direction,
                                cause: error,
                            },
                        );
                        [side.before.as_ref(), side.after.as_ref()]
                            .into_iter()
                            .flatten()
                            .for_each(discard);
                    }
                }
            }
            if backward {
                record.side_files.reverse();
            }
        }
        match store::directory_guards(&record.directories) {
            Ok(guards) => record.directory_guards = guards,
            Err(error) => {
                self.store_failed = true;
                self.notice = Some(format!(
                    "Undo directory identities could not be saved: {error}"
                ));
            }
        }
        if record.files.is_empty() && record.directories.is_empty() && record.side_files.is_empty()
        {
            return (
                None,
                Detailed {
                    result: replay,
                    error_diagnostics: diagnostics,
                },
                moved,
                directory_changes,
            );
        }
        (
            Some(record),
            Detailed {
                result: replay,
                error_diagnostics: diagnostics,
            },
            moved,
            directory_changes,
        )
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        if let Some(owner) = self.active_capture {
            unwatch_owner(owner);
        }
        // Completed and interrupted on-disk state remains available on reopen.
    }
}

/// A side file's path: existing folder aliases resolved, with absent parents
/// retained until the operation creates them; the final entry is not followed.
fn side_path(path: &Path) -> Result<PathBuf, ErrorCause> {
    let name = path.file_name().ok_or_else(|| {
        ErrorCause::message("{value} is not a file name", [("value", display(path))])
    })?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let absolute = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(ErrorCause::external)?
            .join(parent)
    };
    let mut folder = PathBuf::new();
    // Resolve existing ancestors before '..', retaining missing components
    // without creating directories just to capture an absent output.
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                folder.pop();
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                folder.push(component.as_os_str());
            }
            std::path::Component::Normal(_) => {
                folder.push(component.as_os_str());
                match fs::symlink_metadata(&folder) {
                    Ok(_) => {
                        folder = dunce::canonicalize(&folder).map_err(ErrorCause::external)?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(ErrorCause::external(error)),
                }
            }
        }
    }
    Ok(folder.join(name))
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    matches!((fingerprint(a), fingerprint(b)), (Ok(a), Ok(b)) if a.same_content(&b))
}

/// Whether `path` is exactly `blob` (or absent for `None`).
fn holds(blob: Option<&Blob>, path: &Path) -> bool {
    match blob {
        None => {
            matches!(fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        }
        Some(blob) => {
            if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
                return false;
            }
            match &blob.symlink_target {
                Some(target) => fs::read_link(path).is_ok_and(|current| current == *target),
                None => {
                    fingerprint(path).is_ok_and(|current| current.same_content(&blob.fingerprint))
                }
            }
        }
    }
}

/// Make `path` hold `blob` (written through a sibling staging file), or remove it.
fn put(blob: Option<&Blob>, path: &Path) -> Result<(), ErrorCause> {
    store::safe_parents(path).map_err(ErrorCause::external)?;
    let Some(blob) = blob else {
        return fs::remove_file(path).map_err(ErrorCause::external);
    };
    if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
        return Err(ErrorCause::owned("The undo snapshot was modified"));
    }
    let folder = path
        .parent()
        .ok_or_else(|| ErrorCause::owned("The file has no folder"))?;
    if let Some(target) = &blob.symlink_target {
        if fs::read(&blob.path).map_err(ErrorCause::external)?
            != target.as_os_str().as_encoded_bytes()
        {
            return Err(ErrorCause::owned("The undo link snapshot was modified"));
        }
        let (_directory, staging) = stage_side_link_diagnostic(target, folder)?;
        return fs::rename(staging, path).map_err(ErrorCause::external);
    }
    let mut staging = tempfile::Builder::new()
        .prefix(".opentag-")
        .tempfile_in(folder)
        .map_err(ErrorCause::external)?;
    let mut source = fs::File::open(&blob.path).map_err(ErrorCause::external)?;
    use std::io::Read;
    std::io::copy(
        &mut (&mut source).take(blob.fingerprint.len.saturating_add(1)),
        staging.as_file_mut(),
    )
    .map_err(ErrorCause::external)?;
    staging.as_file().sync_all().map_err(ErrorCause::external)?;
    if !fingerprint_diagnostic(staging.path())?.same_content(&blob.fingerprint)
        || fingerprint_diagnostic(&blob.path)? != blob.fingerprint
    {
        return Err(ErrorCause::owned("The undo snapshot was modified"));
    }
    staging
        .persist(path)
        .map_err(|e| ErrorCause::external(e.error))?;
    Ok(())
}

/// A private sibling directory owns the temporary link until its atomic rename.
/// Capture uses the same helper as a capability probe before any replacement.
#[cfg(all(test, unix))]
fn stage_side_link(target: &Path, folder: &Path) -> Result<(tempfile::TempDir, PathBuf), String> {
    stage_side_link_diagnostic(target, folder).map_err(|cause| cause.to_string())
}
fn stage_side_link_diagnostic(
    target: &Path,
    folder: &Path,
) -> Result<(tempfile::TempDir, PathBuf), ErrorCause> {
    let directory = tempfile::Builder::new()
        .prefix(".opentag-link-")
        .tempdir_in(folder)
        .map_err(ErrorCause::external)?;
    let staging = directory.path().join("entry");
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(target, &staging);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_file(target, &staging);
    #[cfg(not(any(unix, windows)))]
    let created: std::io::Result<()> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Side-file symlink undo is unavailable on this platform",
    ));
    created.map_err(|error| {
        #[cfg(any(unix, windows))]
        let cause = ErrorCause::external(error);
        #[cfg(not(any(unix, windows)))]
        let cause = {
            let _ = error;
            ErrorCause::owned("Side-file symlink undo is unavailable on this platform")
        };
        ErrorCause::wrap("Cannot prepare symlink undo: {e}", [], "e", cause)
    })?;
    if fs::read_link(&staging).map_err(ErrorCause::external)? != target {
        return Err(ErrorCause::owned(
            "The undo symlink target was not preserved",
        ));
    }
    Ok((directory, staging))
}

/// The device a path lives on, where the platform exposes it.
fn volume_id(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(path).ok().map(|m| m.dev())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

/// Bytes available to this user on the volume holding `path`, where known.
// statvfs field widths differ between platforms.
#[allow(clippy::useless_conversion)]
fn available_space(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let path = CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `path` is NUL-terminated and `stats` is written by the call on success.
        if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return None;
        }
        // SAFETY: statvfs returned 0, so the structure is initialised.
        let stats = unsafe { stats.assume_init() };
        Some(u64::from(stats.f_bavail).saturating_mul(u64::from(stats.f_frsize)))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn rebase(path: &Path, moves: &[(PathBuf, PathBuf)]) -> PathBuf {
    let mut current = path.to_path_buf();
    for (from, to) in moves {
        if let Ok(suffix) = current.strip_prefix(from) {
            current = if suffix.as_os_str().is_empty() {
                to.clone()
            } else {
                to.join(suffix)
            };
        }
    }
    current
}

fn missing_parents(path: &Path) -> Result<Vec<PathBuf>, ErrorCause> {
    let mut missing = Vec::new();
    let mut current = path.parent();
    while let Some(parent) = current {
        match fs::metadata(parent) {
            Ok(metadata) if metadata.is_dir() => break,
            Ok(_) => {
                return Err(ErrorCause::message(
                    "{value} is not a folder",
                    [("value", display(parent))],
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(parent.to_path_buf())
            }
            Err(error) => return Err(ErrorCause::external(error)),
        }
        current = parent.parent();
    }
    Ok(missing)
}

fn clean_side_parents(side: &SideFile) {
    for parent in &side.created_parents {
        // Only an empty folder known to have been absent at capture is ours.
        if store::check_guards(std::slice::from_ref(parent)).is_err()
            || fs::remove_dir(&parent.path).is_err()
        {
            break;
        }
    }
}

fn replay_side_file(
    side: &mut SideFile,
    backward: bool,
) -> Result<store::DirectoryChanges, ErrorCause> {
    store::check_guards(&side.parents).map_err(ErrorCause::external)?;
    store::safe_parents(&side.path).map_err(ErrorCause::external)?;
    let (from, to) = if backward {
        (&side.after, &side.before)
    } else {
        (&side.before, &side.after)
    };
    if !holds(from.as_ref(), &side.path) {
        return Err(ErrorCause::message(
            "Changed outside TagGroove: {value}",
            [("value", display(&side.path))],
        ));
    }
    if to.is_some() {
        let mut created = Vec::new();
        let result = (|| {
            for parent in side.created_parents.iter().rev() {
                match fs::create_dir(&parent.path) {
                    Ok(()) => created.push(parent),
                    Err(error)
                        if error.kind() == std::io::ErrorKind::AlreadyExists
                            && parent.path.is_dir() => {}
                    Err(error) => return Err(ErrorCause::external(error)),
                }
            }
            put(to.as_ref(), &side.path)
        })();
        if result.is_err() {
            for parent in created.into_iter().rev() {
                let _ = fs::remove_dir(&parent.path);
            }
        }
        result?;
    } else {
        put(None, &side.path)?;
        clean_side_parents(side);
    }
    let changes = if to.is_some() {
        store::refresh_created_parents(&mut side.created_parents).map_err(ErrorCause::external)?
    } else {
        Vec::new()
    };
    side.parents = store::parent_guards(&side.path).map_err(ErrorCause::external)?;
    Ok(changes)
}

fn replay_side_files_and_directories(
    sides: &mut [SideFile],
    directories: &[(PathBuf, PathBuf)],
    backward: bool,
) -> Result<store::DirectoryChanges, ErrorCause> {
    enum Done {
        Side(usize),
        Move(PathBuf, PathBuf),
    }
    let mut done = Vec::new();
    let mut changes = Vec::new();
    let result = (|| {
        for offset in 0..=directories.len() {
            let epoch = if backward {
                directories.len() - offset
            } else {
                offset
            };
            for offset in 0..sides.len() {
                let index = if backward {
                    sides.len() - 1 - offset
                } else {
                    offset
                };
                let side = &mut sides[index];
                if side.epoch != epoch {
                    continue;
                }
                let created = replay_side_file(side, backward)?;
                for other in &mut *sides {
                    other.rebase_directory_ids(&created);
                }
                changes.extend(created);
                done.push(Done::Side(index));
            }
            let movement = if backward {
                epoch
                    .checked_sub(1)
                    .map(|i| (directories[i].1.clone(), directories[i].0.clone()))
            } else {
                directories.get(epoch).cloned()
            };
            if let Some((from, to)) = movement {
                move_directories(&[(from.clone(), to.clone())])?;
                done.push(Done::Move(from, to));
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        for step in done.into_iter().rev() {
            let outcome = match step {
                Done::Side(index) => {
                    replay_side_file(&mut sides[index], !backward).map(|created| {
                        for other in &mut *sides {
                            other.rebase_directory_ids(&created);
                        }
                    })
                }
                Done::Move(from, to) => move_directories(&[(to, from)]),
            };
            if let Err(error) = outcome {
                rollback_errors.push(error);
            }
        }
        return Err(if rollback_errors.is_empty() {
            error
        } else {
            ErrorCause::children(
                "{error}; rollback incomplete: {value}",
                [
                    ("error", error),
                    ("value", ErrorCause::joined(rollback_errors, "; ")),
                ],
            )
        });
    }
    Ok(changes)
}

fn move_directories(moves: &[(PathBuf, PathBuf)]) -> Result<(), ErrorCause> {
    let mut done: Vec<&(PathBuf, PathBuf)> = Vec::new();
    let result = (|| {
        for change in moves {
            let (from, to) = change;
            if !from.is_dir() {
                return Err(ErrorCause::message(
                    "{value} no longer exists",
                    [("value", display(from))],
                ));
            }
            crate::operations::rename_directory_no_clobber_diagnostic(from, to)?;
            done.push(change);
        }
        Ok(())
    })();
    if result.is_err() {
        for (from, to) in done.into_iter().rev() {
            let _ = crate::operations::rename_directory_no_clobber(to, from);
        }
    }
    result
}

/// Replace `target` with the snapshot bytes through a verified staging file.
/// Returns whether the staging file was a clone of the snapshot.
fn restore(
    blob: &Blob,
    target: &Path,
    options: &TagOptions,
    expected: Fingerprint,
) -> Result<bool, ErrorCause> {
    store::safe_parents(target).map_err(ErrorCause::external)?;
    if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
        return Err(ErrorCause::owned("The undo snapshot was modified"));
    }
    if fingerprint_diagnostic(target)? != expected {
        return Err(ErrorCause::owned(
            "The file changed during restore; reload it and try again",
        ));
    }
    let original = fs::metadata(target).map_err(ErrorCause::external)?;
    if original.permissions().readonly() {
        return Err(ErrorCause::owned("The audio file is read-only"));
    }
    let (staging, cloned) = crate::staging_clone(
        &blob.path,
        target
            .parent()
            .ok_or_else(|| ErrorCause::owned("Audio path has no parent directory"))?,
        ".opentag-",
        "",
    )
    .map_err(ErrorCause::external)?;
    staging.as_file().sync_all().map_err(ErrorCause::external)?;
    if !fingerprint_diagnostic(staging.path())?.same_content(&blob.fingerprint)
        || fingerprint_diagnostic(&blob.path)? != blob.fingerprint
    {
        return Err(ErrorCause::owned("The undo snapshot was modified"));
    }
    crate::read_entry_summary_with_options_diagnostic(staging.path(), &[], options)?;
    fs::set_permissions(staging.path(), original.permissions()).map_err(ErrorCause::external)?;
    if fingerprint_diagnostic(target)? != expected {
        return Err(ErrorCause::owned(
            "The file changed during restore; reload it and try again",
        ));
    }
    staging
        .persist(target)
        .map_err(|e| ErrorCause::external(e.error))?;
    Ok(cloned)
}

fn discard(_blob: &Blob) {
    // Removing a reference retires a blob logically. The store collects physical
    // files only after the replacement index has been synchronized and published;
    // the previous on-disk index must remain complete across a failed commit.
}
fn discard_file(file: &FileRecord) {
    [file.before.blob.as_ref(), file.after.blob.as_ref()]
        .into_iter()
        .flatten()
        .for_each(discard);
}
fn discard_record(record: &Record) {
    record.blobs().for_each(discard);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn legacy_windows_identity_tracks_handles_hard_links_and_replacements() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.wav");
        fs::write(&path, b"original").unwrap();
        let original = fs::File::open(&path).unwrap();
        let expected = windows_legacy_identity(&original).unwrap();
        assert_eq!(
            windows_legacy_identity(&fs::File::open(&path).unwrap()).unwrap(),
            expected
        );
        let link = dir.path().join("linked.wav");
        fs::hard_link(&path, &link).unwrap();
        assert_eq!(
            windows_legacy_identity(&fs::File::open(&link).unwrap()).unwrap(),
            expected
        );
        let replacement = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        fs::write(replacement.path(), b"external").unwrap();
        replacement.persist(&path).unwrap();
        assert_ne!(
            windows_legacy_identity(&fs::File::open(&path).unwrap()).unwrap(),
            expected
        );
        assert_eq!(windows_legacy_identity(&original).unwrap(), expected);
    }

    #[cfg(windows)]
    #[test]
    fn unavailable_windows_fingerprint_reports_capture_failure() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
        // Opening a directory without backup semantics cannot supply the audio-file
        // fingerprint. The capture must report a failure rather than silently skip it.
        let capture = journal.begin("Save tags", None, &[dir.path().to_string_lossy().into()]);
        journal.commit(capture, &[], &[]);
        let state = journal.state();
        assert!(state.undo.is_none());
        assert!(state.notice.unwrap().contains("an undo snapshot failed"));
    }

    fn diagnostic_wav(path: &Path) {
        let samples: Vec<u8> = (0..8000)
            .flat_map(|i| ((i % 251) as i16 * 31).to_le_bytes())
            .collect();
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend((36 + samples.len() as u32).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((samples.len() as u32).to_le_bytes());
        bytes.extend(samples);
        fs::write(path, bytes).unwrap();
    }
    #[test]
    fn diagnostic_history_changed_and_failed_frames_keep_raw_labels_directions_and_bytes() {
        let label = "Help / action 1: Title; it was not undone";
        for backward in [true, false] {
            for changed in [true, false] {
                let dir = tempfile::tempdir().unwrap();
                let root = dunce::canonicalize(dir.path()).unwrap();
                let from = root.join("Help; source.wav");
                let to = root.join("Title; moved.wav");
                diagnostic_wav(&from);
                let original = fs::read(&from).unwrap();
                let mut journal = Journal::open(&root.join("undo"), Limits::default()).unwrap();
                let capture = journal.begin(label, None, &[display(&from)]);
                crate::operations::rename_no_clobber(&from, &to).unwrap();
                journal.commit(capture, &[(display(&from), display(&to))], &[]);
                if !backward {
                    assert!(journal.undo().unwrap().errors.is_empty());
                }
                let (current, target) = if backward { (&to, &from) } else { (&from, &to) };
                if changed {
                    fs::write(current, b"External changed bytes").unwrap();
                } else {
                    fs::write(target, b"External target bytes").unwrap();
                }
                let report = if backward {
                    journal.undo_diagnostic()
                } else {
                    journal.redo_diagnostic()
                }
                .unwrap();
                assert_eq!(report.error_diagnostics.len(), 1);
                assert_eq!(report.error_diagnostics[0].index, 0);
                assert_eq!(report.result.errors.len(), 1);
                let diagnostic = &report.error_diagnostics[0].diagnostic;
                assert_eq!(report.result.errors[0].error, diagnostic.to_string());
                let direction = if backward { "undo" } else { "redo" };
                let json = serde_json::to_value(diagnostic).unwrap();
                assert_eq!(json["direction"], direction);
                if changed {
                    assert_eq!(json["kind"], "history_changed");
                    assert_eq!(json["label"], label);
                    assert_eq!(fs::read(current).unwrap(), b"External changed bytes");
                } else {
                    assert_eq!(json["kind"], "history_failed");
                    assert_eq!(json["cause"]["kind"], "rename");
                    assert_eq!(fs::read(current).unwrap(), original);
                    assert_eq!(fs::read(target).unwrap(), b"External target bytes");
                }
            }
        }
    }
    #[test]
    fn diagnostic_history_missing_snapshot_is_an_owned_cause_without_touching_audio() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let path = root.join("Help.wav");
        diagnostic_wav(&path);
        let mut journal = Journal::open(&root.join("undo"), Limits::default()).unwrap();
        let capture = journal.begin("Title", None, &[display(&path)]);
        crate::write_fields_summary(
            &path,
            &crate::Fields::from([("TITLE".into(), vec!["Help".into()])]),
        )
        .unwrap();
        journal.commit(capture, &[], &[]);
        journal.undo.last_mut().unwrap().files[0].before.blob = None;
        let bytes = fs::read(&path).unwrap();
        let report = journal.undo_diagnostic().unwrap();
        let json = serde_json::to_value(&report.error_diagnostics).unwrap();
        assert_eq!(json[0]["diagnostic"]["kind"], "history_failed");
        assert_eq!(json[0]["diagnostic"]["cause"]["kind"], "owned");
        assert_eq!(
            json[0]["diagnostic"]["cause"]["key"],
            "The undo snapshot is missing"
        );
        assert_eq!(
            report.result.errors[0].error,
            "It was not undone: The undo snapshot is missing"
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn diagnostic_history_folder_failure_retains_raw_label_side_path_and_rollback_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let from = root.join("Album Help");
        let to = root.join("Album Title");
        fs::create_dir(&from).unwrap();
        let audio = from.join("Help.wav");
        diagnostic_wav(&audio);
        let cover = from.join("Title; cover.jpg");
        fs::write(&cover, b"Original cover").unwrap();
        let label = "Help / action 1: Title";
        let mut journal = Journal::open(&root.join("undo"), Limits::default()).unwrap();
        let mut capture = journal.begin(label, None, &[display(&audio)]);
        journal.capture_side_file(&mut capture, &cover).unwrap();
        fs::write(&cover, b"Written cover").unwrap();
        crate::operations::rename_directory_no_clobber(&from, &to).unwrap();
        journal.commit(capture, &[], &[(display(&from), display(&to))]);
        let current = to.join("Title; cover.jpg");
        fs::write(&current, b"External cover").unwrap();
        let report = journal.undo_diagnostic().unwrap();
        let json = serde_json::to_value(&report.error_diagnostics).unwrap();
        assert_eq!(json[0]["diagnostic"]["kind"], "history_folders_failed");
        assert_eq!(json[0]["diagnostic"]["direction"], "undo");
        assert_eq!(report.result.errors[0].path, label);
        assert_eq!(
            report.result.errors[0].error,
            report.error_diagnostics[0].diagnostic.to_string()
        );
        assert_eq!(json[0]["diagnostic"]["cause"]["kind"], "owned");
        assert_eq!(
            json[0]["diagnostic"]["cause"]["key"],
            "Changed outside TagGroove: {value}"
        );
        assert_eq!(fs::read(current).unwrap(), b"External cover");
        assert!(!from.exists());
        assert!(to.join("Help.wav").exists());
    }

    const MB: u64 = 1024 * 1024;

    #[test]
    fn retained_bytes_follow_clones_and_rewritten_regions() {
        let partial = |count, rewritten| Some(Writes { count, rewritten });
        // A cloned padding write keeps its region, rounded to blocks, plus overhead.
        assert_eq!(
            retained(40 * MB, true, partial(1, Some(10))),
            BLOCK + OVERHEAD
        );
        assert_eq!(
            retained(40 * MB, true, partial(2, Some(MB + 1))),
            MB + BLOCK + OVERHEAD
        );
        // Never more than the file itself.
        assert_eq!(retained(1000, true, partial(1, Some(900))), 1000);
        // Full rewrites, unreported writes and copied snapshots keep everything.
        assert_eq!(retained(40 * MB, true, partial(1, None)), 40 * MB);
        assert_eq!(retained(40 * MB, true, partial(0, Some(0))), 40 * MB);
        assert_eq!(retained(40 * MB, true, None), 40 * MB);
        assert_eq!(retained(40 * MB, false, partial(1, Some(10))), 40 * MB);
    }

    #[test]
    fn writes_accumulate_only_for_watched_files_and_any_full_write_is_sticky() {
        let fixture = tempfile::NamedTempFile::new().unwrap();
        let baseline = fingerprint(fixture.path()).unwrap();
        let path = PathBuf::from("/opentag-test/watched.flac");
        let other = PathBuf::from("/opentag-test/unwatched.flac");
        watch([(path.clone(), baseline)]);
        note_write(&path, true, Some(8192), baseline);
        note_write(&path, true, Some(100), baseline);
        note_write(&other, true, Some(1), baseline);
        let writes = unwatch([&path, &other]);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[&path].writes.count, 2);
        assert_eq!(writes[&path].writes.rewritten, Some(8292));
        for (cloned, rewritten) in [(false, Some(10)), (true, None)] {
            watch([(path.clone(), baseline)]);
            note_write(&path, true, Some(10), baseline);
            note_write(&path, cloned, rewritten, baseline);
            note_write(&path, true, Some(10), baseline);
            assert_eq!(unwatch([&path])[&path].writes.rewritten, None);
        }
        assert!(unwatch([&path]).is_empty(), "commit stops watching");
    }

    #[test]
    fn moved_watches_keep_original_identity_and_copies_keep_full_cost() {
        let fixture = tempfile::NamedTempFile::new().unwrap();
        let baseline = fingerprint(fixture.path()).unwrap();
        let original = PathBuf::from("/opentag-move-test/album/one.flac");
        let sibling = PathBuf::from("/opentag-move-test/album-other/two.flac");
        let renamed = PathBuf::from("/opentag-move-test/album/new.flac");
        let moved = PathBuf::from("/opentag-move-test/moved/new.flac");
        let copied = PathBuf::from("/opentag-move-test/copied/new.flac");
        watch([(original.clone(), baseline), (sibling.clone(), baseline)]);
        note_move(&original, &renamed, false);
        note_write(&renamed, true, Some(10), baseline);
        note_move(
            Path::new("/opentag-move-test/album"),
            Path::new("/opentag-move-test/moved"),
            false,
        );
        note_write(&moved, true, Some(20), baseline);
        note_write(&original, true, Some(999), baseline);
        note_write(&sibling, true, Some(7), baseline);
        note_move(&moved, &copied, true);
        note_write(&copied, true, Some(1), baseline);
        let writes = unwatch([&original, &sibling]);
        assert_eq!(writes[&original].writes.count, 3);
        assert_eq!(
            writes[&original].writes.rewritten, None,
            "copied moves cannot claim shared snapshot blocks"
        );
        assert_eq!(
            writes[&sibling].writes.count, 1,
            "a sibling with a similar prefix stays separate"
        );
        assert_eq!(writes[&sibling].writes.rewritten, Some(7));
        note_write(&copied, true, Some(100), baseline);
        assert!(
            unwatch([&original]).is_empty(),
            "moved aliases stop being watched at commit"
        );
    }

    #[test]
    fn side_file_copy_checks_byte_budget_before_allocating() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = Journal::open(
            &dir.path().join("undo"),
            Limits {
                bytes: 100,
                min_free: 0,
                ..Limits::default()
            },
        )
        .unwrap();
        let a = dir.path().join("a.dat");
        let b = dir.path().join("b.dat");
        fs::write(&a, [1; 60]).unwrap();
        fs::write(&b, [2; 50]).unwrap();
        let mut capture = history.begin("Side outputs", None, &[]);
        history.capture_side_file(&mut capture, &a).unwrap();
        assert_eq!(history.next, 1);
        assert!(history
            .capture_side_file(&mut capture, &b)
            .unwrap_err()
            .contains("byte limit"));
        assert_eq!(
            history.next, 1,
            "refuse before allocating a second snapshot"
        );
        assert_eq!(fs::read(&b).unwrap(), [2; 50]);
        fs::write(&a, [3; 60]).unwrap();
        history.commit(capture, &[], &[]);
        assert_eq!(
            history.next, 1,
            "After copy also checks the retained Before cost"
        );
        assert_eq!(history.retained_bytes(), 0);
        assert!(history.state().notice.unwrap().contains("cannot be undone"));
        assert_eq!(fs::read(&a).unwrap(), [3; 60]);
        assert!(!history.dir.join("1.snapshot").exists());
    }

    #[test]
    #[cfg(unix)]
    fn side_file_copy_checks_free_space_before_allocating() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = Journal::open(
            &dir.path().join("undo"),
            Limits {
                min_free: 0,
                ..Limits::default()
            },
        )
        .unwrap();
        let mut capture = history.begin("Side output", None, &[]);
        history.limits.min_free = u64::MAX;
        let path = dir.path().join("cover.dat");
        fs::write(&path, [4; 20]).unwrap();
        assert!(history
            .capture_side_file(&mut capture, &path)
            .unwrap_err()
            .contains("free disk space"));
        assert_eq!(history.next, 0, "refuse before allocating a snapshot");
        assert_eq!(fs::read(&path).unwrap(), [4; 20]);
    }

    #[test]
    fn free_space_and_volume_are_known_on_unix() {
        let dir = std::env::temp_dir();
        assert_eq!(
            available_space(&dir).is_some_and(|free| free > 0),
            cfg!(unix)
        );
        assert_eq!(volume_id(&dir).is_some(), cfg!(unix));
    }
}

#[cfg(all(test, unix))]
mod side_link_tests {
    #[test]
    fn failed_link_creation_removes_its_private_staging_directory() {
        let folder = tempfile::tempdir().unwrap();
        // The OS rejects NUL inside a link target after creating the private
        // staging directory; that failure must leave no sibling probe behind.
        assert!(
            super::stage_side_link(std::path::Path::new("invalid\0target"), folder.path()).is_err()
        );
        assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 0);
    }
}
