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
use crate::{
    batch_errors::{push_error, BatchError, Detailed, Direction, ErrorCause},
    FileError, TagOptions,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::SystemTime,
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
    current: PathBuf,
    writes: Writes,
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
pub(crate) fn note_write(path: &Path, cloned: bool, rewritten: Option<u64>) {
    if let Ok(mut tracking) = WRITES.lock() {
        let WriteTracking { watched, paths } = &mut *tracking;
        if let Some(entry) = paths
            .get(path)
            .and_then(|original| watched.get_mut(original))
        {
            entry.writes.count += 1;
            entry.writes.rewritten = match (entry.writes.rewritten, cloned, rewritten) {
                (Some(total), true, Some(bytes)) => Some(total.saturating_add(bytes)),
                _ => None,
            };
        }
    }
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
                entry.current = destination;
                if copied {
                    entry.writes.rewritten = None;
                }
            }
        }
    }
}

fn watch(paths: impl IntoIterator<Item = PathBuf>) {
    if let Ok(mut tracking) = WRITES.lock() {
        for path in paths {
            if let Some(old) = tracking.watched.remove(&path) {
                tracking.paths.remove(&old.current);
            }
            tracking.paths.insert(path.clone(), path.clone());
            tracking.watched.insert(
                path.clone(),
                Watched {
                    current: path,
                    writes: Writes {
                        count: 0,
                        rewritten: Some(0),
                    },
                },
            );
        }
    }
}

fn unwatch<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> BTreeMap<PathBuf, Writes> {
    let Ok(mut tracking) = WRITES.lock() else {
        return BTreeMap::new();
    };
    paths
        .into_iter()
        .filter_map(|path| {
            tracking.watched.remove(path).map(|entry| {
                tracking.paths.remove(&entry.current);
                (path.clone(), entry.writes)
            })
        })
        .collect()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fingerprint {
    len: u64,
    modified: Option<SystemTime>,
    identity: Option<(u64, u64)>,
}

fn fingerprint(path: &Path) -> Result<Fingerprint, String> {
    fingerprint_diagnostic(path).map_err(|cause| cause.to_string())
}
fn fingerprint_diagnostic(path: &Path) -> Result<Fingerprint, ErrorCause> {
    let metadata = fs::metadata(path).map_err(ErrorCause::external)?;
    if !metadata.is_file() {
        return Err(ErrorCause::owned("Not a regular file"));
    }
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    };
    #[cfg(not(unix))]
    let identity = None;
    Ok(Fingerprint {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        identity,
    })
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

#[derive(Clone, Debug)]
struct Blob {
    path: PathBuf,
    fingerprint: Fingerprint,
    /// A hard link to the file's previous inode rather than a copy.
    linked: bool,
    /// Bytes this snapshot keeps on disk.
    cost: u64,
    /// Side-file links retain the entry target verbatim, not its referent.
    symlink_target: Option<PathBuf>,
}
#[derive(Clone, Debug)]
struct Side {
    path: PathBuf,
    blob: Option<Blob>,
    fingerprint: Fingerprint,
}
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
struct SideFile {
    /// Path at the capture epoch, before subsequent directory moves.
    path: PathBuf,
    epoch: usize,
    created_parents: Vec<PathBuf>,
    before: Option<Blob>,
    after: Option<Blob>,
}
#[derive(Clone, Debug)]
struct Record {
    label: String,
    group: Option<String>,
    files: Vec<FileRecord>,
    directories: Vec<(PathBuf, PathBuf)>,
    side_files: Vec<SideFile>,
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
}

type Moved = Vec<(PathBuf, Fingerprint, Fingerprint)>;

struct Captured {
    raw: String,
    path: PathBuf,
    fingerprint: Fingerprint,
    blob: Blob,
}
struct SideCapture {
    path: PathBuf,
    before: Option<Blob>,
    epoch: usize,
    created_parents: Vec<PathBuf>,
}
/// Snapshots taken before one operation; pass it back to [`Journal::commit`].
pub struct Capture {
    label: String,
    group: Option<String>,
    files: Vec<Captured>,
    side_files: Vec<SideCapture>,
    planned_bytes: u64,
    abandoned: Option<String>,
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
    limits: Limits,
    undo: Vec<Record>,
    redo: Vec<Record>,
    next: u64,
    notice: Option<String>,
    abandoned_group: Option<String>,
    options: TagOptions,
    _lock: fs::File,
}

const LOCK: &str = ".session-lock";

impl Journal {
    /// Open a session journal below `root`. Each process owns a locked session
    /// directory; directories of sessions that are no longer running are removed.
    pub fn open(root: &Path, limits: Limits) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !entry.file_name().to_string_lossy().starts_with("session-") {
                    continue;
                }
                let stale = match fs::File::open(path.join(LOCK)) {
                    Ok(file) => file.try_lock().is_ok(),
                    Err(_) => true,
                };
                if stale {
                    let _ = fs::remove_dir_all(&path);
                }
            }
        }
        let dir = root.join(format!(
            "session-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let lock = fs::File::create(dir.join(LOCK)).map_err(|e| e.to_string())?;
        lock.lock().map_err(|e| e.to_string())?;
        Ok(Self {
            dir,
            limits,
            undo: Vec::new(),
            redo: Vec::new(),
            next: 0,
            notice: None,
            abandoned_group: None,
            options: TagOptions::default(),
            _lock: lock,
        })
    }

    /// Tag preferences used to verify that a restored copy still reads as audio.
    /// Restores put back the exact previous bytes, so no tag is rewritten.
    pub fn set_tag_options(&mut self, options: TagOptions) {
        self.options = options;
    }

    pub fn state(&self) -> HistoryState {
        let step = |r: &Record| Step {
            label: r.label.clone(),
            files: r.files.len(),
        };
        HistoryState {
            undo: self.undo.last().map(step),
            redo: self.redo.last().map(step),
            notice: self.notice.clone(),
        }
    }

    /// Bytes currently retained by audio and side-file snapshots.
    pub fn retained_bytes(&self) -> u64 {
        self.undo.iter().chain(&self.redo).map(Record::bytes).sum()
    }

    fn blob_path(&mut self) -> PathBuf {
        self.next += 1;
        self.dir.join(format!("{}.snapshot", self.next))
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
        let path = self.blob_path();
        let linked = fs::hard_link(source, &path).is_ok();
        if !linked {
            if let Err(error) = fs::copy(source, &path) {
                let _ = fs::remove_file(&path);
                return Err(ErrorCause::external(error));
            }
            if fingerprint_diagnostic(source)? != expected {
                let _ = fs::remove_file(&path);
                return Err(ErrorCause::owned(
                    "The file changed while its undo snapshot was taken",
                ));
            }
        }
        let blob = Blob {
            fingerprint: fingerprint_diagnostic(&path)?,
            path,
            linked,
            cost: expected.len,
            symlink_target: None,
        };
        if blob.fingerprint.len != expected.len {
            discard(&blob);
            return Err(ErrorCause::owned("The undo snapshot is incomplete"));
        }
        Ok(blob)
    }

    /// Snapshot `paths` before an operation writes them. Operations sharing a
    /// `group` (one Save click writing files one by one) merge into one step.
    pub fn begin(&mut self, label: &str, group: Option<&str>, paths: &[String]) -> Capture {
        let mut capture = Capture {
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
            let Ok(print) = fingerprint(&path) else {
                continue;
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
        for (raw, path, print) in targets {
            match self.snapshot(&path, print) {
                Ok(blob) => capture.files.push(Captured {
                    raw,
                    path,
                    fingerprint: print,
                    blob,
                }),
                Err(error) => {
                    capture.files.iter().for_each(|f| discard(&f.blob));
                    capture.files.clear();
                    capture.abandoned = Some(format!("an undo snapshot failed: {error}"));
                    return capture;
                }
            }
        }
        watch(capture.files.iter().map(|f| f.path.clone()));
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
        });
        Ok(())
    }

    fn copy_side_file(&mut self, source: &Path, retained: u64) -> Result<Option<Blob>, ErrorCause> {
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
                let path = self.blob_path();
                if let Err(error) = fs::write(&path, target.as_os_str().as_encoded_bytes()) {
                    let _ = fs::remove_file(&path);
                    return Err(ErrorCause::external(error));
                }
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
        let path = self.blob_path();
        let result = (|| {
            let mut input = fs::File::open(source).map_err(ErrorCause::external)?;
            let mut output = fs::File::create(&path).map_err(ErrorCause::external)?;
            let copied = std::io::copy(&mut (&mut input).take(bytes), &mut output)
                .map_err(ErrorCause::external)?;
            let mut extra = [0];
            if copied != bytes || input.read(&mut extra).map_err(ErrorCause::external)? != 0 {
                return Err(ErrorCause::owned(
                    "The side file changed size while its undo snapshot was taken",
                ));
            }
            Ok(Blob {
                fingerprint: fingerprint_diagnostic(&path)?,
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
            } = capture;
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
            });
        }
        (recorded, lost)
    }

    /// Record what the operation changed. `renamed` maps an input path (as
    /// passed to `begin`, or canonical) to its final path; `directories` lists
    /// directory moves in the order they happened.
    pub fn commit(
        &mut self,
        mut capture: Capture,
        renamed: &[(String, String)],
        directories: &[(String, String)],
    ) {
        let directories: Vec<(PathBuf, PathBuf)> = directories
            .iter()
            .map(|(from, to)| (PathBuf::from(from), PathBuf::from(to)))
            .collect();
        let writes = unwatch(capture.files.iter().map(|f| &f.path));
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
            return;
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
            let Ok(after) = fingerprint(&to) else {
                // The operation removed or lost the file; there is nothing to restore onto.
                discard(&captured.blob);
                lost += 1;
                continue;
            };
            // A confirmed staged replacement still needs its snapshot when size
            // and mtime are preserved and the platform supplies no file identity.
            let content = after != captured.fingerprint
                || writes
                    .get(&captured.path)
                    .is_some_and(|write| write.count > 0);
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
            let cost = retained(len, true, writes.get(&captured.path).copied());
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
                },
                after: Side {
                    path: to,
                    blob: None,
                    fingerprint: after,
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
            return;
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
    }

    fn enforce_limits(&mut self) {
        while self.undo.len() > 1
            && (self.undo.len() + self.redo.len() > self.limits.operations
                || self.retained_bytes() > self.limits.bytes)
        {
            let record = self.undo.remove(0);
            discard_record(&record);
        }
    }

    fn clear_redo(&mut self) {
        for record in self.redo.drain(..) {
            discard_record(&record);
        }
    }

    /// Restore the most recent operation. Errors when there is nothing to undo.
    pub fn undo(&mut self) -> Result<Replay, String> {
        self.undo_diagnostic().map(|report| report.result)
    }

    /// Undo with additive presentation companions; bookkeeping is shared with undo.
    pub fn undo_diagnostic(&mut self) -> Result<Detailed<Replay>, String> {
        let record = self.undo.pop().ok_or("There is nothing to undo")?;
        self.notice = None;
        let (record, replay, moved) = self.replay(record, true);
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
        Ok(replay)
    }

    /// Re-apply the most recently undone operation.
    pub fn redo(&mut self) -> Result<Replay, String> {
        self.redo_diagnostic().map(|report| report.result)
    }

    /// Redo with additive presentation companions; bookkeeping is shared with redo.
    pub fn redo_diagnostic(&mut self) -> Result<Detailed<Replay>, String> {
        let record = self.redo.pop().ok_or("There is nothing to redo")?;
        self.notice = None;
        let (record, replay, moved) = self.replay(record, false);
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
        Ok(replay)
    }

    /// Move one record's files to its other side. Also returns, per restored
    /// file, its path with the fingerprint that side had before and has now.
    fn replay(
        &mut self,
        mut record: Record,
        backward: bool,
    ) -> (Option<Record>, Detailed<Replay>, Moved) {
        let mut diagnostics = Vec::new();
        let mut moved = Vec::new();
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
            if fingerprint(&from.path).ok() != Some(from.fingerprint) {
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
        let directory_outcome = if record.directories.is_empty() {
            Ok(())
        } else {
            replay_side_files_and_directories(&record.side_files, &record.directories, backward)
        };
        if let Err(error) = directory_outcome {
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
            );
        }
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
                if current != to.path {
                    crate::operations::rename_no_clobber_cause(&current, &to.path)?;
                }
                if content {
                    let blob = to
                        .blob
                        .as_ref()
                        .ok_or_else(|| ErrorCause::owned("The undo snapshot is missing"))?;
                    match restore(blob, &to.path, &self.options) {
                        // The replaced file now shares all but the rewritten region
                        // with the snapshot it came from.
                        Ok(cloned) => {
                            if let Some(left) = from.blob.as_mut() {
                                left.cost = if cloned && left.linked && cheap {
                                    cost
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
            for side in sides {
                match replay_side_file(&side, backward) {
                    Ok(()) => record.side_files.push(side),
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
        if record.files.is_empty() && record.directories.is_empty() && record.side_files.is_empty()
        {
            return (
                None,
                Detailed {
                    result: replay,
                    error_diagnostics: diagnostics,
                },
                moved,
            );
        }
        (
            Some(record),
            Detailed {
                result: replay,
                error_diagnostics: diagnostics,
            },
            moved,
        )
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
    matches!((fs::read(a), fs::read(b)), (Ok(a), Ok(b)) if a == b)
}

/// Whether `path` is exactly `blob` (or absent for `None`).
fn holds(blob: Option<&Blob>, path: &Path) -> bool {
    match blob {
        None => {
            matches!(fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        }
        Some(blob) => match &blob.symlink_target {
            Some(target) => fs::read_link(path).is_ok_and(|current| current == *target),
            None => {
                fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
                    && same_bytes(&blob.path, path)
            }
        },
    }
}

/// Make `path` hold `blob` (written through a sibling staging file), or remove it.
fn put(blob: Option<&Blob>, path: &Path) -> Result<(), ErrorCause> {
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
    std::io::copy(&mut source, staging.as_file_mut()).map_err(ErrorCause::external)?;
    staging.as_file().sync_all().map_err(ErrorCause::external)?;
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

impl Drop for Journal {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
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
        if fs::remove_dir(parent).is_err() {
            break;
        }
    }
}

fn replay_side_file(side: &SideFile, backward: bool) -> Result<(), ErrorCause> {
    let (from, to) = if backward {
        (&side.after, &side.before)
    } else {
        (&side.before, &side.after)
    };
    if !holds(from.as_ref(), &side.path) {
        return Err(ErrorCause::message(
            "Changed outside OpenTag: {value}",
            [("value", display(&side.path))],
        ));
    }
    if to.is_some() {
        let mut created = Vec::new();
        let result = (|| {
            for parent in side.created_parents.iter().rev() {
                match fs::create_dir(parent) {
                    Ok(()) => created.push(parent),
                    Err(error)
                        if error.kind() == std::io::ErrorKind::AlreadyExists && parent.is_dir() => {
                    }
                    Err(error) => return Err(ErrorCause::external(error)),
                }
            }
            put(to.as_ref(), &side.path)
        })();
        if result.is_err() {
            for parent in created.into_iter().rev() {
                let _ = fs::remove_dir(parent);
            }
        }
        result
    } else {
        put(None, &side.path)?;
        clean_side_parents(side);
        Ok(())
    }
}

fn replay_side_files_and_directories(
    sides: &[SideFile],
    directories: &[(PathBuf, PathBuf)],
    backward: bool,
) -> Result<(), ErrorCause> {
    enum Done {
        Side(usize),
        Move(PathBuf, PathBuf),
    }
    let mut done = Vec::new();
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
                let side = &sides[index];
                if side.epoch != epoch {
                    continue;
                }
                replay_side_file(side, backward)?;
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
                Done::Side(index) => replay_side_file(&sides[index], !backward),
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
    Ok(())
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
fn restore(blob: &Blob, target: &Path, options: &TagOptions) -> Result<bool, ErrorCause> {
    if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
        return Err(ErrorCause::owned("The undo snapshot was modified"));
    }
    let target = dunce::canonicalize(target).map_err(ErrorCause::external)?;
    let original = fs::metadata(&target).map_err(ErrorCause::external)?;
    if original.permissions().readonly() {
        return Err(ErrorCause::owned("The audio file is read-only"));
    }
    let expected = fingerprint_diagnostic(&target)?;
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
    if fs::metadata(staging.path())
        .map_err(ErrorCause::external)?
        .len()
        != blob.fingerprint.len
    {
        return Err(ErrorCause::owned("The restored copy is incomplete"));
    }
    crate::read_entry_summary_with_options_diagnostic(staging.path(), &[], options)?;
    fs::set_permissions(staging.path(), original.permissions()).map_err(ErrorCause::external)?;
    if fingerprint_diagnostic(&target)? != expected {
        return Err(ErrorCause::owned(
            "The file changed during restore; reload it and try again",
        ));
    }
    staging
        .persist(&target)
        .map_err(|e| ErrorCause::external(e.error))?;
    Ok(cloned)
}

fn discard(blob: &Blob) {
    let _ = fs::remove_file(&blob.path);
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
            "Changed outside OpenTag: {value}"
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
        let path = PathBuf::from("/opentag-test/watched.flac");
        let other = PathBuf::from("/opentag-test/unwatched.flac");
        watch([path.clone()]);
        note_write(&path, true, Some(8192));
        note_write(&path, true, Some(100));
        note_write(&other, true, Some(1));
        let writes = unwatch([&path, &other]);
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[&path].count, 2);
        assert_eq!(writes[&path].rewritten, Some(8292));
        for (cloned, rewritten) in [(false, Some(10)), (true, None)] {
            watch([path.clone()]);
            note_write(&path, true, Some(10));
            note_write(&path, cloned, rewritten);
            note_write(&path, true, Some(10));
            assert_eq!(unwatch([&path])[&path].rewritten, None);
        }
        assert!(unwatch([&path]).is_empty(), "commit stops watching");
    }

    #[test]
    fn moved_watches_keep_original_identity_and_copies_keep_full_cost() {
        let original = PathBuf::from("/opentag-move-test/album/one.flac");
        let sibling = PathBuf::from("/opentag-move-test/album-other/two.flac");
        let renamed = PathBuf::from("/opentag-move-test/album/new.flac");
        let moved = PathBuf::from("/opentag-move-test/moved/new.flac");
        let copied = PathBuf::from("/opentag-move-test/copied/new.flac");
        watch([original.clone(), sibling.clone()]);
        note_move(&original, &renamed, false);
        note_write(&renamed, true, Some(10));
        note_move(
            Path::new("/opentag-move-test/album"),
            Path::new("/opentag-move-test/moved"),
            false,
        );
        note_write(&moved, true, Some(20));
        note_write(&original, true, Some(999));
        note_write(&sibling, true, Some(7));
        note_move(&moved, &copied, true);
        note_write(&copied, true, Some(1));
        let writes = unwatch([&original, &sibling]);
        assert_eq!(writes[&original].count, 3);
        assert_eq!(
            writes[&original].rewritten, None,
            "copied moves cannot claim shared snapshot blocks"
        );
        assert_eq!(
            writes[&sibling].count, 1,
            "a sibling with a similar prefix stays separate"
        );
        assert_eq!(writes[&sibling].rewritten, Some(7));
        note_write(&copied, true, Some(100));
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
