//! Session undo journal for operations that write audio files.
//!
//! Before an operation writes, every target file is snapshotted into the
//! journal directory. Staged writes replace files atomically, so a hard link
//! retains the previous inode without copying audio; other volumes fall back
//! to a bounded copy. Undo and redo first verify that each file is still
//! exactly as the journal left it, then restore through a verified sibling
//! staging file. Files changed elsewhere are reported and never overwritten.
use crate::{FileError, TagOptions};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

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

/// Bounds for retained snapshots. An operation whose files exceed `bytes`
/// is not recorded, so the journal never retains more than this much audio.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub operations: usize,
    pub bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            operations: 50,
            bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug)]
struct Blob {
    path: PathBuf,
    fingerprint: Fingerprint,
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
#[derive(Clone, Debug)]
struct Record {
    label: String,
    group: Option<String>,
    files: Vec<FileRecord>,
    directories: Vec<(PathBuf, PathBuf)>,
}
impl Record {
    fn blobs(&self) -> impl Iterator<Item = &Blob> {
        self.files
            .iter()
            .flat_map(|f| [f.before.blob.as_ref(), f.after.blob.as_ref()])
            .flatten()
    }
    fn bytes(&self) -> u64 {
        self.blobs().map(|b| b.fingerprint.len).sum()
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
        if fs::hard_link(source, &path).is_err() {
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
            abandoned: None,
        };
        if group.is_some() && self.abandoned_group.as_deref() == group {
            capture.abandoned = Some("the operation is too large to undo".into());
            return capture;
        }
        let mut seen = std::collections::HashSet::new();
        let mut targets = Vec::new();
        for raw in paths {
            let Ok(path) = Path::new(raw).canonicalize() else {
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
        let total: u64 = grouped + targets.iter().map(|(_, _, f)| f.len).sum::<u64>();
        if total > self.limits.bytes {
            capture.abandoned = Some("the operation is too large to undo".into());
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
        capture
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
        if let Some(reason) = capture.abandoned {
            if capture.group.is_some() {
                if self
                    .undo
                    .last()
                    .is_some_and(|top| top.group == capture.group)
                {
                    let record = self.undo.pop().expect("checked top record");
                    discard_record(&record);
                }
                self.abandoned_group = capture.group.clone();
            }
            self.clear_redo();
            self.notice = Some(format!("{} cannot be undone: {reason}.", capture.label));
            return;
        }
        let mut files = Vec::new();
        let mut lost = 0;
        for captured in capture.files {
            let to = renamed
                .iter()
                .find(|(from, _)| {
                    from == &captured.raw || Path::new(from) == captured.path.as_path()
                })
                .map(|(_, to)| PathBuf::from(to))
                .unwrap_or_else(|| rebase(&captured.path, &directories));
            let to = to.canonicalize().unwrap_or(to);
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
            let blob = if content {
                Some(captured.blob)
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
            });
        }
        if files.is_empty() && directories.is_empty() {
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
                    } else if let Some(blob) = &file.before.blob {
                        discard(blob);
                    }
                    existing.content |= file.content;
                    existing.after = file.after;
                } else {
                    top.files.push(file);
                }
            }
        } else {
            self.undo.push(Record {
                label: capture.label,
                group: capture.group,
                files,
                directories,
            });
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
            let content = file.content;
            let (from, to) = file.sides(backward);
            let current = rebase(&from.path, &moves);
            let outcome = (|| {
                if current != to.path {
                    crate::operations::rename_no_clobber(&current, &to.path)?;
                }
                if content {
                    let blob = to.blob.as_ref().ok_or("The undo snapshot is missing")?;
                    if let Err(error) = restore(blob, &to.path, &self.options) {
                        if current != to.path {
                            let _ = crate::operations::rename_no_clobber(&to.path, &current);
                        }
                        return Err(error);
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
        if record.files.is_empty() && record.directories.is_empty() {
            return (None, replay, moved);
        }
        (Some(record), replay, moved)
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
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
            if fs::symlink_metadata(to).is_ok() {
                return Err(format!("{} already exists", to.display()));
            }
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::rename(from, to).map_err(|e| e.to_string())?;
            done.push(change);
        }
        Ok(())
    })();
    if result.is_err() {
        for (from, to) in done.into_iter().rev() {
            let _ = fs::rename(to, from);
        }
    }
    result
}

/// Replace `target` with the snapshot bytes through a verified staging file.
fn restore(blob: &Blob, target: &Path, options: &TagOptions) -> Result<(), String> {
    if fingerprint(&blob.path).ok() != Some(blob.fingerprint) {
        return Err("The undo snapshot was modified".into());
    }
    let target = target.canonicalize().map_err(|e| e.to_string())?;
    let original = fs::metadata(&target).map_err(|e| e.to_string())?;
    if original.permissions().readonly() {
        return Err("The audio file is read-only".into());
    }
    let expected = fingerprint(&target)?;
    let staging = tempfile::Builder::new()
        .prefix(".opentag-")
        .tempfile_in(
            target
                .parent()
                .ok_or("Audio path has no parent directory")?,
        )
        .map_err(|e| e.to_string())?;
    fs::copy(&blob.path, staging.path()).map_err(|e| e.to_string())?;
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
    Ok(())
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
