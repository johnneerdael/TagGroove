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
use crate::{FileError, TagOptions};
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

/// Files an open capture is watching, with what their staged writes replaced.
static WRITES: Mutex<BTreeMap<PathBuf, Writes>> = Mutex::new(BTreeMap::new());

/// Called by staged writes after replacing `path`: whether the staging copy was
/// a clone, and the bytes rewritten in place (`None` for a full rewrite).
pub(crate) fn note_write(path: &Path, cloned: bool, rewritten: Option<u64>) {
    if let Ok(mut writes) = WRITES.lock() {
        if let Some(entry) = writes.get_mut(path) {
            entry.count += 1;
            entry.rewritten = match (entry.rewritten, cloned, rewritten) {
                (Some(total), true, Some(bytes)) => Some(total.saturating_add(bytes)),
                _ => None,
            };
        }
    }
}

fn watch(paths: impl IntoIterator<Item = PathBuf>) {
    if let Ok(mut writes) = WRITES.lock() {
        for path in paths {
            writes.insert(
                path,
                Writes {
                    count: 0,
                    rewritten: Some(0),
                },
            );
        }
    }
}

fn unwatch<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> BTreeMap<PathBuf, Writes> {
    let Ok(mut writes) = WRITES.lock() else {
        return BTreeMap::new();
    };
    paths
        .into_iter()
        .filter_map(|path| writes.remove(path).map(|w| (path.clone(), w)))
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
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("Not a regular file".into());
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
/// this much audio. Journaling is also refused when it would leave less than
/// `min_free` bytes available on the journal's volume.
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
/// complete bytes on each side, or `None` where it did not exist.
#[derive(Clone, Debug)]
struct SideFile {
    path: PathBuf,
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
/// Snapshots taken before one operation; pass it back to [`Journal::commit`].
pub struct Capture {
    label: String,
    group: Option<String>,
    files: Vec<Captured>,
    side_files: Vec<(PathBuf, Option<Blob>)>,
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

    /// Bytes of audio currently retained by snapshots.
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
        let path = self.blob_path();
        let linked = fs::hard_link(source, &path).is_ok();
        if !linked {
            if let Err(error) = fs::copy(source, &path) {
                let _ = fs::remove_file(&path);
                return Err(error.to_string());
            }
            if fingerprint(source)? != expected {
                let _ = fs::remove_file(&path);
                return Err("The file changed while its undo snapshot was taken".into());
            }
        }
        let blob = Blob {
            fingerprint: fingerprint(&path)?,
            path,
            linked,
            cost: expected.len,
        };
        if blob.fingerprint.len != expected.len {
            discard(&blob);
            return Err("The undo snapshot is incomplete".into());
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
        if capture.abandoned.is_some() {
            return Ok(());
        }
        let path = side_path(path)?;
        if capture.side_files.iter().any(|(known, _)| *known == path) {
            return Ok(());
        }
        let before = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => Some(self.copy_blob(&path)?),
            Ok(_) => return Err(format!("{} is not a regular file", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        capture.side_files.push((path, before));
        Ok(())
    }

    /// A complete copy: side files are small and may be rewritten in place.
    fn copy_blob(&mut self, source: &Path) -> Result<Blob, String> {
        let path = self.blob_path();
        if let Err(error) = fs::copy(source, &path) {
            let _ = fs::remove_file(&path);
            return Err(error.to_string());
        }
        let fingerprint = fingerprint(&path)?;
        Ok(Blob {
            path,
            cost: fingerprint.len,
            fingerprint,
            linked: false,
        })
    }

    /// Side files as the operation left them; unchanged ones are dropped.
    /// Also returns how many could not be recorded.
    fn commit_side_files(
        &mut self,
        captured: Vec<(PathBuf, Option<Blob>)>,
    ) -> (Vec<SideFile>, usize) {
        let mut recorded = Vec::new();
        let mut lost = 0;
        for (path, before) in captured {
            let after = if path.is_file() {
                match self.copy_blob(&path) {
                    Ok(blob) => Some(blob),
                    Err(_) => {
                        before.iter().for_each(discard);
                        lost += 1;
                        continue;
                    }
                }
            } else {
                None
            };
            let unchanged = match (&before, &after) {
                (None, None) => true,
                (Some(old), Some(new)) => same_bytes(&old.path, &new.path),
                _ => false,
            };
            if unchanged {
                [before.as_ref(), after.as_ref()]
                    .into_iter()
                    .flatten()
                    .for_each(discard);
                continue;
            }
            recorded.push(SideFile {
                path,
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
        capture: Capture,
        renamed: &[(String, String)],
        directories: &[(String, String)],
    ) {
        let directories: Vec<(PathBuf, PathBuf)> = directories
            .iter()
            .map(|(from, to)| (PathBuf::from(from), PathBuf::from(to)))
            .collect();
        let writes = unwatch(capture.files.iter().map(|f| &f.path));
        let (side_files, side_lost) = if capture.abandoned.is_some() {
            capture
                .side_files
                .iter()
                .filter_map(|(_, blob)| blob.as_ref())
                .for_each(discard);
            (Vec::new(), 0)
        } else {
            self.commit_side_files(capture.side_files)
        };
        if let Some(reason) = capture.abandoned {
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
            let content = after != captured.fingerprint;
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
    fn replay(&mut self, mut record: Record, backward: bool) -> (Option<Record>, Replay, Moved) {
        let mut moved = Vec::new();
        let mut replay = Replay {
            label: record.label.clone(),
            ..Default::default()
        };
        let verb = if backward { "undone" } else { "redone" };
        // 1. Verify every file is exactly as this journal left it.
        let mut files = Vec::new();
        for mut file in std::mem::take(&mut record.files) {
            let content = file.content;
            let (from, _) = file.sides(backward);
            if fingerprint(&from.path).ok() != Some(from.fingerprint) {
                replay.errors.push(FileError {
                    path: display(&from.path),
                    error: format!(
                        "Changed outside OpenTag after {}; it was not {verb}",
                        record.label
                    ),
                });
                discard_file(&file);
                continue;
            }
            if content && from.blob.is_none() {
                let (path, print) = (from.path.clone(), from.fingerprint);
                match self.snapshot(&path, print) {
                    // Updated once the restore shows whether it cloned.
                    Ok(blob) => file.sides(backward).0.blob = Some(blob),
                    Err(error) => {
                        replay.errors.push(FileError {
                            path: display(&path),
                            error: format!("It was not {verb}: {error}"),
                        });
                        discard_file(&file);
                        continue;
                    }
                }
            }
            files.push(file);
        }
        // 2. Move directories as one unit; any failure rolls back and changes nothing.
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
        if let Err(error) = move_directories(&moves) {
            replay.errors.push(FileError {
                path: record.label.clone(),
                error: format!("Folders could not be moved, so nothing was {verb}: {error}"),
            });
            files.iter().for_each(discard_file);
            discard_record(&record);
            return (None, replay, moved);
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
                    crate::operations::rename_no_clobber(&current, &to.path)?;
                }
                if content {
                    let blob = to.blob.as_ref().ok_or("The undo snapshot is missing")?;
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
                fingerprint(&to.path)
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
                    replay.errors.push(FileError {
                        path: display(&current),
                        error: format!("It was not {verb}: {error}"),
                    });
                    discard_file(&file);
                }
            }
        }
        // 4. Put back or remove the side files the operation created or replaced.
        for mut side in std::mem::take(&mut record.side_files) {
            let path = rebase(&side.path, &moves);
            let (from, to) = if backward {
                (&side.after, &side.before)
            } else {
                (&side.before, &side.after)
            };
            let outcome = if !holds(from.as_ref(), &path) {
                Err(format!(
                    "Changed outside OpenTag after {}; it was not {verb}",
                    record.label
                ))
            } else {
                put(to.as_ref(), &path).map_err(|error| format!("It was not {verb}: {error}"))
            };
            match outcome {
                Ok(()) => {
                    side.path = path;
                    record.side_files.push(side);
                }
                Err(error) => {
                    replay.errors.push(FileError {
                        path: display(&path),
                        error,
                    });
                    [side.before.as_ref(), side.after.as_ref()]
                        .into_iter()
                        .flatten()
                        .for_each(discard);
                }
            }
        }
        if record.files.is_empty() && record.directories.is_empty() && record.side_files.is_empty()
        {
            return (None, replay, moved);
        }
        (Some(record), replay, moved)
    }
}

/// A side file's path: its folder resolved, the file itself possibly absent.
fn side_path(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} is not a file name", path.display()))?;
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
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
        Some(blob) => path.is_file() && same_bytes(&blob.path, path),
    }
}

/// Make `path` hold `blob` (written through a sibling staging file), or remove it.
fn put(blob: Option<&Blob>, path: &Path) -> Result<(), String> {
    let Some(blob) = blob else {
        return fs::remove_file(path).map_err(|e| e.to_string());
    };
    if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
        return Err("The undo snapshot was modified".into());
    }
    let folder = path.parent().ok_or("The file has no folder")?;
    let mut staging = tempfile::Builder::new()
        .prefix(".opentag-")
        .tempfile_in(folder)
        .map_err(|e| e.to_string())?;
    let mut source = fs::File::open(&blob.path).map_err(|e| e.to_string())?;
    std::io::copy(&mut source, staging.as_file_mut()).map_err(|e| e.to_string())?;
    staging.as_file().sync_all().map_err(|e| e.to_string())?;
    staging.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
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

fn move_directories(moves: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    let mut done: Vec<&(PathBuf, PathBuf)> = Vec::new();
    let result = (|| {
        for change in moves {
            let (from, to) = change;
            if !from.is_dir() {
                return Err(format!("{} no longer exists", from.display()));
            }
            crate::operations::rename_directory_no_clobber(from, to)?;
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
fn restore(blob: &Blob, target: &Path, options: &TagOptions) -> Result<bool, String> {
    if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
        return Err("The undo snapshot was modified".into());
    }
    let target = dunce::canonicalize(target).map_err(|e| e.to_string())?;
    let original = fs::metadata(&target).map_err(|e| e.to_string())?;
    if original.permissions().readonly() {
        return Err("The audio file is read-only".into());
    }
    let expected = fingerprint(&target)?;
    let (staging, cloned) = crate::staging_clone(
        &blob.path,
        target
            .parent()
            .ok_or("Audio path has no parent directory")?,
        ".opentag-",
        "",
    )?;
    staging.as_file().sync_all().map_err(|e| e.to_string())?;
    if fs::metadata(staging.path())
        .map_err(|e| e.to_string())?
        .len()
        != blob.fingerprint.len
    {
        return Err("The restored copy is incomplete".into());
    }
    crate::read_entry_summary_with_options(staging.path(), &[], options)?;
    fs::set_permissions(staging.path(), original.permissions()).map_err(|e| e.to_string())?;
    if fingerprint(&target)? != expected {
        return Err("The file changed during restore; reload it and try again".into());
    }
    staging.persist(&target).map_err(|e| e.error.to_string())?;
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
    fn free_space_and_volume_are_known_on_unix() {
        let dir = std::env::temp_dir();
        assert_eq!(
            available_space(&dir).is_some_and(|free| free > 0),
            cfg!(unix)
        );
        assert_eq!(volume_id(&dir).is_some(), cfg!(unix));
    }
}
