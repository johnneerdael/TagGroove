//! Bounded bundle intake and replacement. Private source text never leaves the machine.
use super::{extension, load, plain_name, relative_name, schema, MAX_DEPTH, MAX_FILE_BYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(super) static LOCK: Mutex<()> = Mutex::new(());
const MAX_ENTRIES: usize = 4096;
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXISTING_DEPTH: usize = 64;
const BACKUP: &str = ".opentag-sources-previous";
const MARKER: &str = ".opentag-source-install";
const BACKUP_MARKER: &[u8] = b"source installer backup v1\n";
const ROTATION: &str = ".opentag-sources-rotation";
const ROTATION_MARKER: &[u8] = b"source installer rotation v1\n";
const INTENT: &str = "intent.json";
const INTENT_LIMIT: u64 = 2 * 1024 * 1024;
const PUBLICATION: &str = "publication";
const PUBLICATION_MARKER: &[u8] = b"source installer publication v1\n";

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PublicationTarget {
    Intent,
    BackupMarker,
    ActiveMarker,
}

impl PublicationTarget {
    fn limit(self) -> u64 {
        match self {
            Self::Intent => INTENT_LIMIT,
            Self::BackupMarker => BACKUP_MARKER.len() as u64,
            Self::ActiveMarker => MAX_BYTES,
        }
    }
    fn checkpoint(self, persisted: bool) -> &'static str {
        match (self, persisted) {
            (Self::Intent, false) => "publication-intent",
            (Self::Intent, true) => "publication-persisted-intent",
            (Self::BackupMarker, false) => "publication-backup-marker",
            (Self::BackupMarker, true) => "publication-persisted-backup-marker",
            (Self::ActiveMarker, false) => "publication-active-marker",
            (Self::ActiveMarker, true) => "publication-persisted-active-marker",
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PublicationState {
    version: u32,
    target: PublicationTarget,
    length: u64,
    sha256: String,
}

fn recover_publication(rotation: &Path) -> Result<(), String> {
    let slot = rotation.join(PUBLICATION);
    if std::fs::symlink_metadata(&slot).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        return Ok(());
    }
    owned_marker(&slot, PUBLICATION_MARKER)?;
    for entry in std::fs::read_dir(&slot).map_err(|e| e.to_string())? {
        let name = entry.map_err(|e| e.to_string())?.file_name();
        if ![MARKER, "metadata.json", "data"]
            .iter()
            .any(|known| name == *known)
        {
            return Err(
                "Unexpected source publication entry; retained files were not changed.".into(),
            );
        }
    }
    let state: PublicationState = serde_json::from_slice(&read(&slot.join("metadata.json"), 4096)?)
        .map_err(|e| format!("Invalid source publication state: {e}"))?;
    if state.version != 1
        || state.length > state.target.limit()
        || state.sha256.len() != 64
        || !state.sha256.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(
            "Unsupported source publication state; retained files were not changed.".into(),
        );
    }
    let data = slot.join("data");
    match std::fs::symlink_metadata(&data) {
        Ok(_) => {
            let bytes = read(&data, state.target.limit())?;
            if bytes.len() as u64 != state.length
                || format!("{:x}", Sha256::digest(&bytes)) != state.sha256
            {
                return Err(
                    "Source publication data changed; retained files were not changed.".into(),
                );
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    // Retire the complete marked directory before deleting any of its members.
    // A cleanup interruption leaves no half-owned slot in rotation.
    let retired = tempfile::Builder::new()
        .prefix(".opentag-source-publication-retired-")
        .tempdir_in(
            rotation
                .parent()
                .ok_or("The sources folder has no parent.")?,
        )
        .map_err(|e| e.to_string())?;
    std::fs::rename(&slot, retired.path().join(PUBLICATION)).map_err(|e| e.to_string())?;
    Ok(())
}

fn publish_file(
    rotation: &Path,
    kind: PublicationTarget,
    target: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    if bytes.len() as u64 > kind.limit() {
        return Err("Source publication exceeds its limit.".into());
    }
    let slot = rotation.join(PUBLICATION);
    match std::fs::symlink_metadata(&slot) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
        Ok(_) => {
            return Err(
                "Source publication is already pending; retained files were not changed.".into(),
            )
        }
    }
    // Byte writes happen outside hashed trees and the rotation whitelist.
    // Admit the complete bounded, marked package as one directory rename.
    let stage = tempfile::Builder::new()
        .prefix(".opentag-source-publication-stage-")
        .tempdir_in(
            rotation
                .parent()
                .ok_or("The sources folder has no parent.")?,
        )
        .map_err(|e| e.to_string())?;
    let state = PublicationState {
        version: 1,
        target: kind,
        length: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    };
    for (name, payload) in [
        (MARKER, PUBLICATION_MARKER.to_vec()),
        (
            "metadata.json",
            serde_json::to_vec(&state).map_err(|e| e.to_string())?,
        ),
        ("data", bytes.to_vec()),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(stage.path().join(name))
            .map_err(|e| e.to_string())?;
        file.write_all(&payload)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    std::fs::rename(stage.path(), &slot).map_err(|e| e.to_string())?;
    checkpoint(kind.checkpoint(false))?;
    std::fs::rename(slot.join("data"), target).map_err(|e| e.to_string())?;
    checkpoint(kind.checkpoint(true))?;
    recover_publication(rotation)
}

type Files = BTreeMap<String, Vec<u8>>;

#[derive(Clone, Default)]
struct ExistingTree {
    files: BTreeMap<PathBuf, Vec<u8>>,
    directories: BTreeSet<PathBuf>,
    bytes: u64,
    entries: usize,
    internal_marker_budget: bool,
}

/// Source identifiers use Unicode and forward slashes; unrelated native paths need not.
fn source_name(path: &Path) -> Option<String> {
    path.iter()
        .map(|part| part.to_str())
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
}

fn supported(name: &str) -> bool {
    matches!(extension(name).as_str(), "src" | "inc" | "settings")
}

fn add(files: &mut Files, name: &str, bytes: Vec<u8>, total: &mut u64) -> Result<(), String> {
    relative_name(name)?;
    if files
        .keys()
        .any(|old| old.to_lowercase() == name.to_lowercase())
    {
        return Err(format!(
            "The bundle repeats {name} (file names ignore case)."
        ));
    }
    *total += bytes.len() as u64;
    if bytes.len() as u64 > MAX_FILE_BYTES || *total > MAX_BYTES || files.len() >= MAX_ENTRIES {
        return Err(
            "Source bundle exceeds the limit: 4 MB per source file, 128 MB total, 4096 files."
                .into(),
        );
    }
    files.insert(name.into(), bytes);
    Ok(())
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.len() > limit {
        return Err(format!(
            "{} must be a regular file of at most {limit} bytes (no symbolic links).",
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
        return Err("File grew beyond the installation limit.".into());
    }
    Ok(bytes)
}

fn directory(
    path: &Path,
    prefix: &str,
    depth: usize,
    files: &mut Files,
    total: &mut u64,
    entries: &mut usize,
) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        *entries += 1;
        if *entries > MAX_ENTRIES {
            return Err("Source folder exceeds 4096 entries.".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Source folder has a non-Unicode file name.")?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "{name}: symbolic links are not accepted in source folders."
            ));
        }
        // Download metadata and documentation are never installed.
        if name.starts_with('.') {
            continue;
        }
        let relative = format!("{prefix}{name}");
        if kind.is_dir() {
            let depth_limit = MAX_DEPTH;
            if depth >= depth_limit {
                return Err(format!(
                    "{relative}: source folders can be at most {depth_limit} levels deep."
                ));
            }
            plain_name(&name)?;
            directory(
                &entry.path(),
                &format!("{relative}/"),
                depth + 1,
                files,
                total,
                entries,
            )?;
        } else if supported(&name) {
            plain_name(&name)?;
            let bytes = read(&entry.path(), MAX_FILE_BYTES)?;
            *total += bytes.len() as u64;
            if *total > MAX_BYTES {
                return Err("Sources folder exceeds the 128 MB staging limit.".into());
            }
            *total -= bytes.len() as u64;
            add(files, &relative, bytes, total)?;
        }
    }
    Ok(())
}

/// Snapshot local files and directories without converting their native names.
fn existing_directory(
    path: &Path,
    prefix: &Path,
    depth: usize,
    tree: &mut ExistingTree,
) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let internal_marker = tree.internal_marker_budget && depth == 0 && name == MARKER;
        if !internal_marker {
            tree.entries += 1;
        }
        if tree.entries > MAX_ENTRIES {
            return Err("Source folder exceeds 4096 entries.".into());
        }
        let relative = prefix.join(&name);
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            let name = name.to_string_lossy();
            return Err(format!(
                "{name}: symbolic links are not accepted in source folders."
            ));
        }
        if kind.is_dir() {
            if depth >= MAX_EXISTING_DEPTH {
                let relative = relative.display();
                let depth_limit = MAX_EXISTING_DEPTH;
                return Err(format!(
                    "{relative}: source folders can be at most {depth_limit} levels deep."
                ));
            }
            tree.directories.insert(relative.clone());
            existing_directory(&entry.path(), &relative, depth + 1, tree)?;
        } else {
            let bytes = read(&entry.path(), MAX_BYTES)?;
            if !internal_marker {
                tree.bytes += bytes.len() as u64;
            }
            if tree.bytes > MAX_BYTES {
                return Err("Sources folder exceeds the 128 MB staging limit.".into());
            }
            tree.files.insert(relative, bytes);
        }
    }
    Ok(())
}

fn archive(
    path: &Path,
    files: &mut Files,
    total: &mut u64,
    entries: &mut usize,
) -> Result<(), String> {
    let bytes = read(path, MAX_BYTES)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    *entries += zip.len();
    if *entries > MAX_ENTRIES {
        return Err("Source ZIP exceeds 4096 entries.".into());
    }
    let mut seen = BTreeSet::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("ZIP entry {i}: {e}"))?;
        let name = entry.name().replace('\\', "/");
        let name = name.trim_end_matches('/');
        // Validate every archive entry, including ignored documentation. Never
        // extract any paths before this check.
        if name.starts_with('/')
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
        {
            return Err(format!("ZIP contains an unsafe path: {name}"));
        }
        if name.contains(':') {
            return Err(format!("ZIP contains an unsafe path: {name}"));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(format!("The ZIP repeats {name} (file names ignore case)."));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!(
                "{name}: symbolic links are not accepted in ZIP files."
            ));
        }
        if entry.is_dir()
            || name
                .split('/')
                .any(|part| part.starts_with('.') || part == "__MACOSX")
            || !supported(name)
        {
            continue;
        }
        relative_name(name)?;
        if entry.size() > MAX_FILE_BYTES {
            return Err(format!("{name} is larger than 4 MB."));
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("{name}: {e}"))?;
        add(files, name, bytes, total)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RotationIntent {
    version: u32,
    phase: String,
    active: Option<String>,
    active_marked: Option<String>,
    previous: Option<String>,
    candidate: String,
    cleanup: BTreeMap<String, BTreeMap<String, Option<String>>>,
}

fn cleanup_manifest(tree: &ExistingTree) -> BTreeMap<String, Option<String>> {
    let key = |kind: u8, path: &Path| {
        let mut hash = Sha256::new();
        hash.update([kind]);
        hash.update(path.as_os_str().as_encoded_bytes());
        format!("{:x}", hash.finalize())
    };
    tree.directories
        .iter()
        .map(|path| (key(b'd', path), None))
        .chain(tree.files.iter().map(|(path, bytes)| {
            (
                key(b'f', path),
                Some(format!("{:x}", Sha256::digest(bytes))),
            )
        }))
        .collect()
}

fn snapshot_tree(folder: &Path, internal_marker_budget: bool) -> Result<ExistingTree, String> {
    let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.is_symlink() {
        return Err("Source recovery requires real directories without symbolic links.".into());
    }
    let mut tree = ExistingTree {
        internal_marker_budget,
        ..ExistingTree::default()
    };
    existing_directory(folder, Path::new(""), 0, &mut tree)?;
    Ok(tree)
}

fn tree_hash(tree: &ExistingTree) -> String {
    let mut hash = Sha256::new();
    for (kind, path, bytes) in tree
        .directories
        .iter()
        .map(|path| (b'd', path, &[][..]))
        .chain(
            tree.files
                .iter()
                .map(|(path, bytes)| (b'f', path, bytes.as_slice())),
        )
    {
        let name = path.as_os_str().as_encoded_bytes();
        hash.update([kind]);
        hash.update(name.len().to_le_bytes());
        hash.update(name);
        hash.update(bytes.len().to_le_bytes());
        hash.update(bytes);
    }
    format!("{hash:x}", hash = hash.finalize())
}

fn identity(folder: &Path, internal_marker_budget: bool) -> Result<Option<String>, String> {
    let meta = match std::fs::symlink_metadata(folder) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if !meta.is_dir() || meta.is_symlink() {
        return Err("Source recovery requires real directories without symbolic links.".into());
    }
    Ok(Some(tree_hash(&snapshot_tree(
        folder,
        internal_marker_budget,
    )?)))
}

fn owned_marker(folder: &Path, expected: &[u8]) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
    if !meta.is_dir()
        || meta.is_symlink()
        || read(&folder.join(MARKER), 128)?.as_slice() != expected
    {
        return Err(format!(
            "Installation recovery folder {} has no marker; move it aside before installing.",
            folder.display()
        ));
    }
    Ok(())
}

fn rotation_intent(rotation: &Path) -> Result<Option<RotationIntent>, String> {
    owned_marker(rotation, ROTATION_MARKER)?;
    for entry in std::fs::read_dir(rotation).map_err(|e| e.to_string())? {
        let name = entry.map_err(|e| e.to_string())?.file_name();
        if ![MARKER, INTENT, "candidate", "previous"]
            .iter()
            .any(|known| name == *known)
        {
            return Err(
                "Unexpected source installation recovery entry; move it aside before installing."
                    .into(),
            );
        }
    }
    let path = rotation.join(INTENT);
    if std::fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) {
        if std::fs::symlink_metadata(rotation.join("candidate")).is_ok()
            || std::fs::symlink_metadata(rotation.join("previous")).is_ok()
        {
            return Err(
                "Source installation recovery intent is missing; retained files were not changed."
                    .into(),
            );
        }
        return Ok(None);
    }
    let intent: RotationIntent = serde_json::from_slice(&read(&path, INTENT_LIMIT)?)
        .map_err(|e| format!("Invalid source installation recovery intent: {e}"))?;
    let digest = |text: &str| text.len() == 64 && text.bytes().all(|c| c.is_ascii_hexdigit());
    if intent.version != 1
        || !["prepared", "rolled-back", "committed"].contains(&intent.phase.as_str())
        || !digest(&intent.candidate)
        || [&intent.active, &intent.active_marked, &intent.previous]
            .into_iter()
            .flatten()
            .any(|s| !digest(s))
        || intent.active.is_some() != intent.active_marked.is_some()
        || intent
            .cleanup
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != BTreeSet::from(["candidate", "previous"])
        || intent.cleanup.values().any(|entries| {
            entries.len() > MAX_ENTRIES + 1
                || entries.iter().any(|(key, value)| {
                    !digest(key) || value.as_ref().is_some_and(|text| !digest(text))
                })
        })
    {
        return Err(
            "Unsupported source installation recovery intent; retained files were not changed."
                .into(),
        );
    }
    Ok(Some(intent))
}

fn save_intent(rotation: &Path, intent: &RotationIntent) -> Result<(), String> {
    let bytes = serde_json::to_vec(intent).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > INTENT_LIMIT {
        return Err("Source installation recovery intent exceeds its limit.".into());
    }
    publish_file(
        rotation,
        PublicationTarget::Intent,
        &rotation.join(INTENT),
        &bytes,
    )
}

#[cfg(test)]
thread_local! {
    static BOOTSTRAP_COLLISION: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
    static INSTALL_FAULT: std::cell::RefCell<Option<(&'static str, bool)>> = const { std::cell::RefCell::new(None) };
    static INTERRUPTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static INSTALL_KILL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
fn checkpoint(point: &str) -> Result<(), String> {
    #[cfg(test)]
    {
        tests::checkpoint(point)
    }
    #[cfg(not(test))]
    {
        let _ = point;
        Ok(())
    }
}

fn restore_active_marker(folder: &Path, candidate: &Path) -> Result<(), String> {
    let original = candidate.join(MARKER);
    let current = folder.join(MARKER);
    match std::fs::symlink_metadata(&original) {
        Ok(meta) if meta.is_file() && !meta.is_symlink() => {
            let bytes = read(&original, MAX_BYTES)?;
            if std::fs::read(&current).ok().as_deref() != Some(bytes.as_slice()) {
                publish_file(
                    candidate
                        .parent()
                        .ok_or("The sources folder has no parent.")?,
                    PublicationTarget::ActiveMarker,
                    &current,
                    &bytes,
                )?;
            }
        }
        Ok(meta) if meta.is_dir() && !meta.is_symlink() => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if std::fs::symlink_metadata(&current).is_ok() {
                if read(&current, 128)?.as_slice() != BACKUP_MARKER {
                    return Err(
                        "Source recovery marker changed; retained files were not changed.".into(),
                    );
                }
                std::fs::remove_file(current).map_err(|e| e.to_string())?;
            }
        }
        _ => return Err("Source recovery marker is not a regular original entry.".into()),
    }
    Ok(())
}

fn finish_rotation(rotation: &Path, intent: &RotationIntent) -> Result<(), String> {
    // The ownership marker remains permanently outside the retired trees, so a
    // failed recursive cleanup remains recognizable on the next read.
    for name in ["previous", "candidate"] {
        let path = rotation.join(name);
        if identity(&path, name == "previous")?.is_some() {
            let remaining = cleanup_manifest(&snapshot_tree(&path, name == "previous")?);
            let expected = &intent.cleanup[name];
            if remaining
                .iter()
                .any(|(key, value)| expected.get(key) != Some(value))
            {
                return Err(
                    "Retired source installation files changed; retained files were not deleted."
                        .into(),
                );
            }
            checkpoint("cleanup")?;
            std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
        }
    }
    std::fs::remove_file(rotation.join(INTENT)).map_err(|e| e.to_string())
}

fn recover_rotation(folder: &Path) -> Result<(), String> {
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    let rotation = parent.join(ROTATION);
    if std::fs::symlink_metadata(&rotation).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    owned_marker(&rotation, ROTATION_MARKER)?;
    recover_publication(&rotation)?;
    let Some(mut intent) = rotation_intent(&rotation)? else {
        return Ok(());
    };
    if intent.phase != "prepared" {
        return finish_rotation(&rotation, &intent);
    }
    let backup = parent.join(BACKUP);
    let candidate = rotation.join("candidate");
    let previous = rotation.join("previous");
    let active_id = identity(folder, false)?;
    let backup_id = identity(&backup, true)?;
    let previous_id = identity(&previous, true)?;
    let candidate_id = identity(&candidate, false)?;
    let unchanged =
        active_id == intent.active && backup_id == intent.previous && previous_id.is_none();
    if candidate_id.is_none() {
        if unchanged {
            intent.phase = "rolled-back".into();
        } else if active_id.as_ref() == Some(&intent.candidate)
            && backup_id == intent.active_marked
            && previous_id == intent.previous
        {
            intent.phase = "committed".into();
        } else {
            return Err("Source installation recovery conflicts with changed files; retained files were not changed.".into());
        }
        save_intent(&rotation, &intent)?;
        return finish_rotation(&rotation, &intent);
    }
    if candidate_id.as_ref() != Some(&intent.candidate)
        || (previous_id.is_some() && previous_id != intent.previous)
    {
        return Err("Source installation recovery conflicts with changed files; retained files were not changed.".into());
    }
    if active_id.is_none() && intent.active.is_some() {
        if backup_id != intent.active && backup_id != intent.active_marked {
            return Err(
                "The previous active source folder changed; retained files were not changed."
                    .into(),
            );
        }
        if intent.previous.is_some() && previous_id != intent.previous {
            return Err(
                "The previous source snapshot is missing; retained files were not changed.".into(),
            );
        }
        checkpoint("restore-active")?;
        std::fs::rename(&backup, folder).map_err(|e| e.to_string())?;
    } else if active_id != intent.active && active_id != intent.active_marked {
        return Err("The active source folder changed; retained files were not changed.".into());
    } else if !unchanged && backup_id.is_some() {
        return Err(
            "A source recovery destination is occupied; retained files were not changed.".into(),
        );
    }
    if intent.active.is_some() {
        restore_active_marker(folder, &candidate)?;
        if identity(folder, false)? != intent.active {
            return Err("The restored source folder differs from its original snapshot.".into());
        }
    }
    if previous_id.is_some() {
        if identity(&backup, true)?.is_some() {
            return Err(
                "A source recovery destination is occupied; retained files were not changed."
                    .into(),
            );
        }
        std::fs::rename(&previous, &backup).map_err(|e| e.to_string())?;
    }
    if identity(&backup, true)? != intent.previous {
        return Err(
            "The previous source snapshot changed; retained files were not changed.".into(),
        );
    }
    intent.phase = "rolled-back".into();
    save_intent(&rotation, &intent)?;
    finish_rotation(&rotation, &intent)
}

fn publish(
    folder: &Path,
    stage: &Path,
    existing: &ExistingTree,
    had_active: bool,
    previous_before: &Option<String>,
) -> Result<(), String> {
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    let backup = parent.join(BACKUP);
    let rotation = parent.join(ROTATION);
    let active = identity(folder, false)?;
    if active.is_some() != had_active
        || active
            .as_ref()
            .is_some_and(|hash| *hash != tree_hash(existing))
        || identity(&backup, true)? != *previous_before
    {
        return Err("Installed sources changed during staging; try again.".into());
    }
    if std::fs::symlink_metadata(&rotation).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        // The fixed recovery name must never expose a partially bootstrapped
        // marker. Prepare it outside the recognized namespace, then admit it.
        let bootstrap = tempfile::Builder::new()
            .prefix(".opentag-sources-rotation-stage-")
            .tempdir_in(parent)
            .map_err(|e| e.to_string())?;
        checkpoint("bootstrap-created")?;
        let mut marker = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(bootstrap.path().join(MARKER))
            .map_err(|e| e.to_string())?;
        marker
            .write_all(ROTATION_MARKER)
            .and_then(|_| marker.sync_all())
            .map_err(|e| e.to_string())?;
        drop(marker);
        checkpoint("bootstrap-before-admission")?;
        match std::fs::symlink_metadata(&rotation) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
            Ok(_) => return Err("Source installation recovery is still pending.".into()),
        }
        std::fs::rename(bootstrap.path(), &rotation).map_err(|e| e.to_string())?;
        checkpoint("bootstrap-after-admission")?;
    }
    if rotation_intent(&rotation)?.is_some() {
        return Err("Source installation recovery is still pending.".into());
    }
    let mut marked = existing.clone();
    marked
        .files
        .insert(PathBuf::from(MARKER), BACKUP_MARKER.to_vec());
    let intent = RotationIntent {
        version: 1,
        phase: "prepared".into(),
        active: active.clone(),
        active_marked: active.map(|_| tree_hash(&marked)),
        previous: identity(&backup, true)?,
        candidate: identity(stage, false)?.ok_or("The staged source folder disappeared.")?,
        cleanup: BTreeMap::from([
            (
                "candidate".into(),
                cleanup_manifest(&snapshot_tree(stage, false)?),
            ),
            (
                "previous".into(),
                if backup.exists() {
                    cleanup_manifest(&snapshot_tree(&backup, true)?)
                } else {
                    BTreeMap::new()
                },
            ),
        ]),
    };
    save_intent(&rotation, &intent)?;
    std::fs::rename(stage, rotation.join("candidate")).map_err(|e| e.to_string())?;
    checkpoint("after-prepare")?;
    if backup.exists() {
        std::fs::rename(&backup, rotation.join("previous")).map_err(|e| e.to_string())?;
    }
    checkpoint("after-previous")?;
    if folder.exists() {
        if identity(&backup, true)?.is_some() {
            return Err(
                "A source installation destination is occupied; retained files were not changed."
                    .into(),
            );
        }
        checkpoint("active-rename")?;
        std::fs::rename(folder, &backup).map_err(|e| e.to_string())?;
        checkpoint("after-active")?;
        if identity(&backup, true)? != intent.active {
            return Err("Installed sources changed during staging; try again.".into());
        }
        publish_file(
            &rotation,
            PublicationTarget::BackupMarker,
            &backup.join(MARKER),
            BACKUP_MARKER,
        )?;
        checkpoint("after-marker")?;
    }
    if identity(folder, false)?.is_some()
        || identity(&backup, true)? != intent.active_marked
        || identity(&rotation.join("previous"), true)? != intent.previous
        || identity(&rotation.join("candidate"), false)?.as_ref() != Some(&intent.candidate)
    {
        return Err("Source installation recovery conflicts with changed files; retained files were not changed.".into());
    }
    checkpoint("candidate-publication")?;
    std::fs::rename(rotation.join("candidate"), folder).map_err(|e| e.to_string())?;
    checkpoint("after-publication")?;
    // Publication is the commit point. Retired cleanup may be retried without
    // rolling back or reporting refusal for the successfully installed bundle.
    let _ = recover_rotation(folder);
    Ok(())
}

/// An interrupted directory swap retains the old complete folder beside it.
/// Only restore/delete a backup carrying our marker, never an unrelated folder.
pub(super) fn recover(folder: &Path) -> Result<(), String> {
    recover_rotation(folder)?;
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    let backup = parent.join(BACKUP);
    if !backup.exists() {
        return Ok(());
    }
    if std::fs::symlink_metadata(&backup)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
        || std::fs::read(backup.join(MARKER)).ok().as_deref() != Some(BACKUP_MARKER)
    {
        return Err(format!(
            "Installation recovery folder {} has no marker; move it aside before installing.",
            backup.display()
        ));
    }
    if !folder.exists() {
        std::fs::rename(&backup, folder)
            .map_err(|e| format!("Restore {} to {}: {e}", backup.display(), folder.display()))?;
        let _ = std::fs::remove_file(folder.join(MARKER));
    }
    Ok(())
}

/// Canonicalize source identifiers while retaining every native metadata path.
fn prepare_incoming(existing: &ExistingTree, incoming: Files) -> Result<Files, String> {
    let mut spellings = BTreeMap::new();
    for name in existing
        .files
        .keys()
        .chain(existing.directories.iter())
        .filter_map(|path| source_name(path))
    {
        let mut prefix = String::new();
        for part in name.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(previous) = spellings.insert(prefix.to_lowercase(), prefix.clone()) {
                if previous != prefix {
                    return Err(format!("Installed paths differ only in case: {previous} and {prefix}. Rename one before installing."));
                }
            }
        }
    }
    let incoming = incoming
        .into_iter()
        .map(|(name, bytes)| {
            let mut canonical = String::new();
            for part in name.split('/') {
                if !canonical.is_empty() {
                    canonical.push('/');
                }
                canonical.push_str(part);
                let known = spellings
                    .entry(canonical.to_lowercase())
                    .or_insert_with(|| canonical.clone());
                canonical = known.clone();
            }
            (canonical, bytes)
        })
        .collect::<Files>();
    let incoming_paths = incoming.keys().map(PathBuf::from).collect::<BTreeSet<_>>();
    let mut combined_entries = BTreeSet::new();
    for path in existing
        .files
        .keys()
        .chain(existing.directories.iter())
        .chain(incoming_paths.iter())
    {
        for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
            combined_entries.insert(ancestor.to_path_buf());
        }
    }
    if combined_entries.len() > MAX_ENTRIES {
        return Err("Source folder exceeds 4096 entries.".into());
    }
    let combined_bytes = existing
        .files
        .iter()
        .filter(|(path, _)| !incoming_paths.contains(*path))
        .map(|(_, bytes)| bytes.len() as u64)
        .chain(incoming.values().map(|bytes| bytes.len() as u64))
        .sum::<u64>();
    if combined_bytes > MAX_BYTES {
        return Err("Combined sources exceed the 128 MB staging limit.".into());
    }
    Ok(incoming)
}

fn write_all<P: AsRef<Path>>(folder: &Path, files: &BTreeMap<P, Vec<u8>>) -> Result<(), String> {
    for (name, bytes) in files {
        let relative = name.as_ref();
        let path = folder.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| {
            let name = relative.display();
            format!("{name}: {e}")
        })?;
    }
    Ok(())
}

pub(super) fn prepare_folder_open(folder: &Path) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    recover(folder)?;
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())
}

pub(super) fn install(folder: &Path, paths: &[String]) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    recover(folder)?;
    let previous_before = identity(
        &folder
            .parent()
            .ok_or("The sources folder has no parent.")?
            .join(BACKUP),
        true,
    )?;
    if paths.len() > MAX_ENTRIES {
        return Err("Choose at most 4096 source paths.".into());
    }
    if paths.is_empty() {
        return Err("Choose a source folder, ZIP, or .src, .inc and .settings files.".into());
    }
    let mut incoming = Files::new();
    let mut complete = false;
    let mut total = 0;
    let mut entries = 0;
    for path in paths {
        let path = Path::new(path);
        let meta =
            std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.is_symlink() {
            return Err("Symbolic links cannot be installed as source bundles.".into());
        }
        if meta.is_dir() {
            complete = true;
            directory(path, "", 0, &mut incoming, &mut total, &mut entries)?;
        } else if extension(&path.to_string_lossy()) == "zip" {
            complete = true;
            archive(path, &mut incoming, &mut total, &mut entries)?;
        } else {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Source file has no usable name.")?;
            if !supported(name) {
                return Err(format!(
                    "{name} is not a .src, .inc or .settings file or ZIP."
                ));
            }
            add(&mut incoming, name, read(path, MAX_FILE_BYTES)?, &mut total)?;
        }
    }
    if incoming.is_empty() {
        return Err("The bundle contains no .src, .inc or .settings files.".into());
    }
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".opentag-sources-stage-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let mut existing = ExistingTree::default();
    let had_active = folder.exists();
    if had_active {
        let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.is_symlink() {
            return Err("The sources folder must be a real directory.".into());
        }
        existing_directory(folder, Path::new(""), 0, &mut existing)?;
    }
    let incoming = prepare_incoming(&existing, incoming)?;
    for name in &existing.directories {
        std::fs::create_dir_all(stage.path().join(name)).map_err(|e| e.to_string())?;
    }
    write_all(stage.path(), &existing.files)?;
    write_all(stage.path(), &incoming)?;
    // Folder/ZIP bundles must be self-contained. Explicit-file selections also
    // support incremental updates using validated installed companions.
    let check = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    write_all(check.path(), &incoming)?;
    let checked = if complete { check.path() } else { stage.path() };
    for name in incoming.keys() {
        match extension(name).as_str() {
            "src" => {
                let file = load(checked, name)
                    .map_err(|e| format!("{name}: {e}. Choose the complete bundle."))?;
                super::validate_scripts(&file)?;
                if let Some(settings) = file.settings {
                    schema(checked, &settings)
                        .map_err(|e| format!("{name}: [Settings]={settings}: {e}"))?;
                }
            }
            "settings" => {
                schema(checked, name)?;
            }
            "inc" => {
                super::source::validate_includes(name, &super::reader(checked))?;
            }
            _ => {}
        }
    }
    // A shared include/settings update must not break a previously working
    // source outside the incoming collection. Keep pre-existing problems visible
    // without making an unrelated old broken source block every new install.
    let validate_source = |root: &Path, name: &str| -> Result<(), String> {
        let source = load(root, name)?;
        super::validate_scripts(&source)?;
        if let Some(settings) = source.settings {
            schema(root, &settings)?;
        }
        Ok(())
    };
    for name in existing
        .files
        .keys()
        .filter_map(|path| source_name(path))
        .filter(|name| extension(name) == "src")
    {
        if validate_source(folder, &name).is_ok() {
            validate_source(stage.path(), &name)
                .map_err(|e| format!("{name}: {e}. Choose the complete bundle."))?;
        }
    }
    match publish(
        folder,
        stage.path(),
        &existing,
        had_active,
        &previous_before,
    ) {
        Ok(()) => Ok(()),
        Err(error) => {
            #[cfg(test)]
            if INTERRUPTED.with(|state| state.replace(false)) {
                return Err(error);
            }
            recover(folder).map_err(|recovery| {
                format!("Install failed ({error}); source recovery is retained: {recovery}")
            })?;
            Err(format!("Sources were not replaced: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    pub(super) fn checkpoint(point: &str) -> Result<(), String> {
        if point == "bootstrap-before-admission" {
            if let Some(path) = BOOTSTRAP_COLLISION.with(|state| state.borrow_mut().take()) {
                std::fs::create_dir(path).unwrap();
            }
        }
        if INSTALL_FAULT.with(|fault| {
            fault
                .borrow()
                .as_ref()
                .is_some_and(|(wanted, _)| *wanted == point)
        }) {
            let (_, interrupted) = INSTALL_FAULT.with(|fault| fault.borrow_mut().take().unwrap());
            if INSTALL_KILL.with(|state| state.replace(false)) {
                eprintln!("INSTALL_PUBLICATION_KILL {point}");
                std::process::exit(73);
            }
            INTERRUPTED.with(|state| state.set(interrupted));
            return Err(format!("Test filesystem fault: {point}"));
        }
        Ok(())
    }
    const SOURCE: &str = "[Settings]=Options.settings\n[Include]=Common.inc\n";
    const INCLUDE: &str =
        "[AlbumUrl]=https://example.invalid/%s\n[ParserScriptAlbum]=...\nSayRest\n";
    const SETTINGS: &str =
        r#"{"key":"example","settings":[{"type":"string","key":"language","default":"en"}]}"#;
    fn collection(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("Shop#&Album.src"), SOURCE).unwrap();
        std::fs::write(path.join("Common.inc"), INCLUDE).unwrap();
        std::fs::write(path.join("Options.settings"), SETTINGS).unwrap();
    }
    fn selected(path: &Path) -> Vec<String> {
        vec![path.to_string_lossy().into_owned()]
    }
    fn zip(path: &Path, entries: &[(&str, &str)]) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, text) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    fn snapshot(folder: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn collect(root: &Path, folder: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    collect(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root).unwrap().into(),
                        std::fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut files = BTreeMap::new();
        collect(folder, folder, &mut files);
        files
    }

    #[cfg(unix)]
    #[test]
    fn refused_folder_and_zip_marker_publication_keep_active_and_previous_bytes() {
        use std::os::unix::fs::PermissionsExt;
        struct RestoreMode(PathBuf, u32);
        impl Drop for RestoreMode {
            fn drop(&mut self) {
                if self.0.exists() {
                    std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(self.1))
                        .unwrap();
                }
            }
        }
        for route in ["folder", "zip"] {
            let dir = tempfile::tempdir().unwrap();
            let bundle = dir.path().join("download");
            let target = dir.path().join("sources");
            collection(&bundle);
            install(&target, &selected(&bundle)).unwrap();
            std::fs::write(
                target.join("settings.json"),
                br#"{"example":{"language":"de"}}"#,
            )
            .unwrap();
            std::fs::write(
                bundle.join("Common.inc"),
                INCLUDE.replace("example.invalid", "second.invalid"),
            )
            .unwrap();
            install(&target, &selected(&bundle)).unwrap();
            let active_before = snapshot(&target);
            let backup = dir.path().join(BACKUP);
            let previous_before = snapshot(&backup);
            let next_include = INCLUDE.replace("example.invalid", "third.invalid");
            std::fs::write(bundle.join("Common.inc"), &next_include).unwrap();
            let incoming = if route == "zip" {
                let path = dir.path().join("update.zip");
                zip(
                    &path,
                    &[
                        ("Shop#&Album.src", SOURCE),
                        ("Common.inc", &next_include),
                        ("Options.settings", SETTINGS),
                    ],
                );
                path
            } else {
                bundle
            };
            let mode = std::fs::metadata(&target).unwrap().permissions().mode();
            let restore_mode = RestoreMode(target.clone(), mode);
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o500)).unwrap();
            let probe = target.join(".permission-control");
            if std::fs::write(&probe, b"probe").is_ok() {
                std::fs::remove_file(probe).unwrap();
                eprintln!("permission control unavailable: this runtime permits writes to a0500 directory");
                return;
            }
            let error = install(&target, &selected(&incoming)).unwrap_err();
            drop(restore_mode);
            assert!(error.contains("Permission denied"), "{route}: {error}");
            assert_eq!(
                snapshot(&target),
                active_before,
                "{route}: active bundle changed on refusal"
            );
            assert!(
                backup.is_dir(),
                "{route}: refusal deleted the previous snapshot"
            );
            assert_eq!(
                snapshot(&backup),
                previous_before,
                "{route}: previous snapshot changed on refusal"
            );
        }
    }

    fn prepared_update(root: &Path) -> (PathBuf, PathBuf) {
        let bundle = root.join("download");
        let target = root.join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::write(
            target.join("settings.json"),
            br#"{"example":{"language":"de"}}"#,
        )
        .unwrap();
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("example.invalid", "second.invalid"),
        )
        .unwrap();
        install(&target, &selected(&bundle)).unwrap();
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("example.invalid", "third.invalid"),
        )
        .unwrap();
        (bundle, target)
    }

    fn fault(point: &'static str, interrupt: bool) {
        INSTALL_FAULT.with(|fault| {
            assert!(fault.borrow().is_none());
            *fault.borrow_mut() = Some((point, interrupt));
        });
    }

    #[test]
    fn precommit_rename_and_publication_faults_restore_exact_active_and_previous() {
        for point in ["active-rename", "candidate-publication"] {
            for original_marker in [None, Some(b"original active metadata".as_slice())] {
                let root = tempfile::tempdir().unwrap();
                let (bundle, target) = prepared_update(root.path());
                if let Some(bytes) = original_marker {
                    std::fs::write(target.join(MARKER), bytes).unwrap();
                }
                let before = snapshot(&target);
                let previous = snapshot(&root.path().join(BACKUP));
                fault(point, false);
                assert!(install(&target, &selected(&bundle))
                    .unwrap_err()
                    .contains(point));
                assert_eq!(snapshot(&target), before, "{point}");
                assert_eq!(snapshot(&root.path().join(BACKUP)), previous, "{point}");
                assert!(rotation_intent(&root.path().join(ROTATION))
                    .unwrap()
                    .is_none());
                recover(&target).unwrap();
                assert_eq!(snapshot(&target), before);
                assert_eq!(snapshot(&root.path().join(BACKUP)), previous);
            }
        }
    }

    #[test]
    #[ignore = "fresh-process source recovery worker"]
    fn source_rotation_recovery_worker() {
        let target = PathBuf::from(std::env::var_os("TAGGROOVE_ROTATION_TEST_FOLDER").unwrap());
        recover(&target).unwrap();
    }

    fn fresh_recovery(target: &Path) {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sources::web::installer::tests::source_rotation_recovery_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("TAGGROOVE_ROTATION_TEST_FOLDER", target)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    #[ignore = "fresh-process publication interruption worker"]
    fn source_publication_kill_worker() {
        let target = PathBuf::from(std::env::var_os("TAGGROOVE_ROTATION_TEST_FOLDER").unwrap());
        let wanted = std::env::var("TAGGROOVE_PUBLICATION_TEST_POINT").unwrap();
        let point = match wanted.as_str() {
            "publication-intent" => "publication-intent",
            "publication-persisted-intent" => "publication-persisted-intent",
            "publication-backup-marker" => "publication-backup-marker",
            "publication-persisted-backup-marker" => "publication-persisted-backup-marker",
            "publication-active-marker" => "publication-active-marker",
            "publication-persisted-active-marker" => "publication-persisted-active-marker",
            _ => panic!("unknown publication test point"),
        };
        let bundle = target.parent().unwrap().join("download");
        if wanted.contains("active-marker") {
            fault("after-marker", true);
            install(&target, &selected(&bundle)).unwrap_err();
            fault(point, false);
            INSTALL_KILL.with(|state| state.set(true));
            recover(&target).unwrap();
        } else {
            fault(point, false);
            INSTALL_KILL.with(|state| state.set(true));
            install(&target, &selected(&bundle)).unwrap();
        }
        panic!("publication worker did not terminate at its checkpoint");
    }

    #[test]
    #[ignore = "fresh-process rotation bootstrap interruption worker"]
    fn source_bootstrap_kill_worker() {
        let target = PathBuf::from(std::env::var_os("TAGGROOVE_ROTATION_TEST_FOLDER").unwrap());
        let wanted = std::env::var("TAGGROOVE_BOOTSTRAP_TEST_POINT").unwrap();
        let point = match wanted.as_str() {
            "bootstrap-created" => "bootstrap-created",
            "bootstrap-before-admission" => "bootstrap-before-admission",
            "bootstrap-after-admission" => "bootstrap-after-admission",
            _ => panic!("unknown bootstrap test point"),
        };
        fault(point, false);
        INSTALL_KILL.with(|state| state.set(true));
        install(
            &target,
            &selected(&target.parent().unwrap().join("download")),
        )
        .unwrap();
        panic!("bootstrap worker did not terminate at its checkpoint");
    }

    #[test]
    fn rotation_bootstrap_interruption_never_publishes_unmarked_fixed_state() {
        for point in [
            "bootstrap-created",
            "bootstrap-before-admission",
            "bootstrap-after-admission",
        ] {
            let root = tempfile::tempdir().unwrap();
            let (bundle, target) = prepared_update(root.path());
            let rotation = root.path().join(ROTATION);
            std::fs::remove_dir_all(&rotation).unwrap();
            let active = snapshot(&target);
            let previous = snapshot(&root.path().join(BACKUP));
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sources::web::installer::tests::source_bootstrap_kill_worker",
                    "--ignored",
                    "--nocapture",
                ])
                .env("TAGGROOVE_ROTATION_TEST_FOLDER", &target)
                .env("TAGGROOVE_BOOTSTRAP_TEST_POINT", point)
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(73),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            fresh_recovery(&target);
            assert_eq!(snapshot(&target), active, "{point}");
            assert_eq!(snapshot(&root.path().join(BACKUP)), previous, "{point}");
            if point == "bootstrap-after-admission" {
                owned_marker(&rotation, ROTATION_MARKER).unwrap();
            } else {
                assert!(std::fs::symlink_metadata(&rotation).is_err());
            }
            install(&target, &selected(&bundle)).unwrap();
            assert!(std::fs::read_to_string(target.join("Common.inc"))
                .unwrap()
                .contains("third.invalid"));
        }
    }

    #[test]
    fn rotation_bootstrap_refuses_an_unowned_empty_admission_collision() {
        let root = tempfile::tempdir().unwrap();
        let (bundle, target) = prepared_update(root.path());
        let rotation = root.path().join(ROTATION);
        std::fs::remove_dir_all(&rotation).unwrap();
        let active = snapshot(&target);
        let previous = snapshot(&root.path().join(BACKUP));
        BOOTSTRAP_COLLISION.with(|state| *state.borrow_mut() = Some(rotation.clone()));
        assert!(install(&target, &selected(&bundle)).is_err());
        assert!(rotation.is_dir());
        assert_eq!(std::fs::read_dir(&rotation).unwrap().count(), 0);
        assert_eq!(snapshot(&target), active);
        assert_eq!(snapshot(&root.path().join(BACKUP)), previous);
    }

    #[test]
    fn atomic_publication_windows_recover_after_fresh_process_termination() {
        for kind in ["intent", "backup-marker", "active-marker"] {
            for persisted in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let (_bundle, target) = prepared_update(root.path());
                if kind == "active-marker" {
                    std::fs::write(target.join(MARKER), b"original active marker bytes").unwrap();
                }
                let active = snapshot(&target);
                let previous = snapshot(&root.path().join(BACKUP));
                let point = format!(
                    "publication-{}{kind}",
                    if persisted { "persisted-" } else { "" }
                );
                let result = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "sources::web::installer::tests::source_publication_kill_worker",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("TAGGROOVE_ROTATION_TEST_FOLDER", &target)
                    .env("TAGGROOVE_PUBLICATION_TEST_POINT", &point)
                    .output()
                    .unwrap();
                assert_eq!(
                    result.status.code(),
                    Some(73),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                let slot = root.path().join(ROTATION).join(PUBLICATION);
                assert_eq!(read(&slot.join(MARKER), 128).unwrap(), PUBLICATION_MARKER);
                assert_eq!(slot.join("data").exists(), !persisted, "{point}");
                fresh_recovery(&target);
                assert_eq!(snapshot(&target), active, "{point}");
                assert_eq!(snapshot(&root.path().join(BACKUP)), previous, "{point}");
                assert!(std::fs::symlink_metadata(&slot).is_err());
                assert!(rotation_intent(&root.path().join(ROTATION))
                    .unwrap()
                    .is_none());
                recover(&target).unwrap();
                assert_eq!(snapshot(&target), active);
                assert_eq!(snapshot(&root.path().join(BACKUP)), previous);
            }
        }
    }

    #[test]
    fn publication_ownership_unknown_entries_and_changed_payloads_refuse_unchanged() {
        for damage in [
            "unmarked",
            "unknown-entry",
            "changed-data",
            "unknown-version",
            "random-temp",
        ] {
            let root = tempfile::tempdir().unwrap();
            let (_bundle, target) = prepared_update(root.path());
            let rotation = root.path().join(ROTATION);
            let slot = rotation.join(PUBLICATION);
            if damage == "random-temp" {
                std::fs::write(rotation.join(".tmp-unrecognized"), b"not installer owned").unwrap();
            } else {
                std::fs::create_dir(&slot).unwrap();
                std::fs::write(slot.join(MARKER), PUBLICATION_MARKER).unwrap();
                let state = PublicationState {
                    version: if damage == "unknown-version" { 2 } else { 1 },
                    target: PublicationTarget::Intent,
                    length: 2,
                    sha256: format!("{:x}", Sha256::digest(b"{}")),
                };
                std::fs::write(
                    slot.join("metadata.json"),
                    serde_json::to_vec(&state).unwrap(),
                )
                .unwrap();
                std::fs::write(slot.join("data"), b"{}").unwrap();
                match damage {
                    "unmarked" => std::fs::remove_file(slot.join(MARKER)).unwrap(),
                    "unknown-entry" => std::fs::write(slot.join("extra"), b"user data").unwrap(),
                    "changed-data" => std::fs::write(slot.join("data"), b"changed").unwrap(),
                    _ => {}
                }
            }
            let before = snapshot(root.path());
            assert!(recover(&target).is_err(), "{damage}");
            assert_eq!(snapshot(root.path()), before, "{damage}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn publication_slot_and_payload_symlinks_refuse_without_following_or_deleting() {
        use std::os::unix::fs::symlink;
        for which in ["slot", "data", "metadata.json"] {
            let root = tempfile::tempdir().unwrap();
            let (_bundle, target) = prepared_update(root.path());
            let rotation = root.path().join(ROTATION);
            let slot = rotation.join(PUBLICATION);
            std::fs::create_dir(&slot).unwrap();
            std::fs::write(slot.join(MARKER), PUBLICATION_MARKER).unwrap();
            let state = PublicationState {
                version: 1,
                target: PublicationTarget::Intent,
                length: 2,
                sha256: format!("{:x}", Sha256::digest(b"{}")),
            };
            std::fs::write(
                slot.join("metadata.json"),
                serde_json::to_vec(&state).unwrap(),
            )
            .unwrap();
            std::fs::write(slot.join("data"), b"{}").unwrap();
            let original = if which == "slot" {
                slot.clone()
            } else {
                slot.join(which)
            };
            let retained = root.path().join("retained-publication");
            std::fs::rename(&original, &retained).unwrap();
            symlink(&retained, &original).unwrap();
            let before = snapshot(root.path());
            assert!(recover(&target).is_err());
            assert_eq!(snapshot(root.path()), before);
            assert!(std::fs::symlink_metadata(original).unwrap().is_symlink());
        }
    }

    #[test]
    fn interrupted_rotation_cutpoints_recover_in_fresh_processes() {
        for point in [
            "after-prepare",
            "after-previous",
            "after-active",
            "after-marker",
            "after-publication",
        ] {
            let root = tempfile::tempdir().unwrap();
            let (bundle, target) = prepared_update(root.path());
            let before = snapshot(&target);
            let previous = snapshot(&root.path().join(BACKUP));
            fault(point, true);
            assert!(install(&target, &selected(&bundle))
                .unwrap_err()
                .contains(point));
            fresh_recovery(&target);
            if point == "after-publication" {
                let mut expected = before.clone();
                expected.insert(
                    PathBuf::from("Common.inc"),
                    INCLUDE
                        .replace("example.invalid", "third.invalid")
                        .into_bytes(),
                );
                assert_eq!(snapshot(&target), expected);
                let mut expected_previous = before;
                expected_previous.insert(PathBuf::from(MARKER), BACKUP_MARKER.to_vec());
                assert_eq!(snapshot(&root.path().join(BACKUP)), expected_previous);
            } else {
                assert_eq!(snapshot(&target), before, "{point}");
                assert_eq!(snapshot(&root.path().join(BACKUP)), previous, "{point}");
            }
            assert!(rotation_intent(&root.path().join(ROTATION))
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn committed_cleanup_failure_retries_without_rolling_back_published_sources() {
        let root = tempfile::tempdir().unwrap();
        let (bundle, target) = prepared_update(root.path());
        let before = snapshot(&target);
        fault("cleanup", false);
        install(&target, &selected(&bundle)).unwrap();
        let after = snapshot(&target);
        let previous = snapshot(&root.path().join(BACKUP));
        assert_eq!(
            after[Path::new("Common.inc")],
            INCLUDE
                .replace("example.invalid", "third.invalid")
                .into_bytes()
        );
        assert_eq!(
            previous[Path::new("Common.inc")],
            before[Path::new("Common.inc")]
        );
        assert_eq!(
            rotation_intent(&root.path().join(ROTATION))
                .unwrap()
                .unwrap()
                .phase,
            "committed"
        );
        fresh_recovery(&target);
        assert_eq!(snapshot(&target), after);
        assert_eq!(snapshot(&root.path().join(BACKUP)), previous);
        assert!(rotation_intent(&root.path().join(ROTATION))
            .unwrap()
            .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn partial_retired_cleanup_permission_failure_retries_only_known_unchanged_entries() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let (bundle, target) = prepared_update(root.path());
        let backup = root.path().join(BACKUP);
        std::fs::create_dir(backup.join("Locked")).unwrap();
        std::fs::write(
            backup.join("Locked/retained.txt"),
            b"original retired metadata",
        )
        .unwrap();
        fault("cleanup", false);
        install(&target, &selected(&bundle)).unwrap();
        let active = snapshot(&target);
        let previous = snapshot(&backup);
        let rotation = root.path().join(ROTATION);
        let retired = rotation.join("previous");
        // A completed unlink followed by an ordinary directory permission error
        // models partial recursive cleanup, without any publication mutation.
        std::fs::remove_file(retired.join("Common.inc")).unwrap();
        let locked = retired.join("Locked");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).unwrap();
        let probe = locked.join(".permission-control");
        if std::fs::write(&probe, b"probe").is_ok() {
            std::fs::remove_file(probe).unwrap();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
            eprintln!(
                "permission control unavailable: this runtime permits writes to a0500 directory"
            );
            return;
        }
        let result = recover(&target);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.unwrap_err().contains("Permission denied"));
        assert_eq!(snapshot(&target), active);
        assert_eq!(snapshot(&backup), previous);
        // New or changed retired data still refuses, even after partial cleanup.
        std::fs::write(
            locked.join("retained.txt"),
            b"external change after failure",
        )
        .unwrap();
        let retained = snapshot(&rotation);
        assert!(recover(&target).unwrap_err().contains("changed"));
        assert_eq!(snapshot(&rotation), retained);
        std::fs::write(locked.join("retained.txt"), b"original retired metadata").unwrap();
        fresh_recovery(&target);
        assert_eq!(snapshot(&target), active);
        assert_eq!(snapshot(&backup), previous);
        assert!(rotation_intent(&rotation).unwrap().is_none());
    }

    #[test]
    fn recovery_refuses_changed_candidate_previous_and_occupied_destinations() {
        for changed in [
            "candidate",
            "previous",
            "active-destination",
            "backup-destination",
        ] {
            let root = tempfile::tempdir().unwrap();
            let (bundle, target) = prepared_update(root.path());
            fault("after-active", true);
            install(&target, &selected(&bundle)).unwrap_err();
            let rotation = root.path().join(ROTATION);
            let path = match changed {
                "candidate" => rotation.join("candidate/Common.inc"),
                "previous" => rotation.join("previous/Common.inc"),
                "active-destination" => {
                    std::fs::create_dir(&target).unwrap();
                    target.join("new-user-file.txt")
                }
                "backup-destination" => root.path().join(BACKUP).join("new-user-file.txt"),
                _ => unreachable!(),
            };
            std::fs::write(path, b"external change").unwrap();
            let before = snapshot(root.path());
            assert!(recover(&target).is_err(), "{changed}");
            assert_eq!(
                snapshot(root.path()),
                before,
                "{changed}: recovery changed retained data"
            );
        }
    }

    #[test]
    fn failed_active_restoration_retains_both_snapshots_for_a_fresh_retry() {
        let root = tempfile::tempdir().unwrap();
        let (bundle, target) = prepared_update(root.path());
        let active = snapshot(&target);
        let previous = snapshot(&root.path().join(BACKUP));
        fault("after-active", true);
        install(&target, &selected(&bundle)).unwrap_err();
        let retained = snapshot(root.path());
        fault("restore-active", false);
        assert!(recover(&target).unwrap_err().contains("restore-active"));
        assert_eq!(snapshot(root.path()), retained);
        fresh_recovery(&target);
        assert_eq!(snapshot(&target), active);
        assert_eq!(snapshot(&root.path().join(BACKUP)), previous);
    }

    #[test]
    fn invalid_rotation_ownership_and_intent_refuse_without_changes() {
        for changed in ["unmarked", "malformed", "unknown-version"] {
            let root = tempfile::tempdir().unwrap();
            let (bundle, target) = prepared_update(root.path());
            fault("after-prepare", true);
            install(&target, &selected(&bundle)).unwrap_err();
            let rotation = root.path().join(ROTATION);
            match changed {
                "unmarked" => std::fs::remove_file(rotation.join(MARKER)).unwrap(),
                "malformed" => std::fs::write(rotation.join(INTENT), b"broken intent").unwrap(),
                "unknown-version" => {
                    let mut intent = rotation_intent(&rotation).unwrap().unwrap();
                    intent.version = 99;
                    save_intent(&rotation, &intent).unwrap();
                }
                _ => unreachable!(),
            }
            let before = snapshot(root.path());
            assert!(recover(&target).is_err(), "{changed}");
            assert_eq!(snapshot(root.path()), before);
        }
    }

    #[cfg(unix)]
    #[test]
    fn rotation_symlink_ownership_or_intent_is_not_followed() {
        use std::os::unix::fs::symlink;
        for kind in ["container", "intent"] {
            let root = tempfile::tempdir().unwrap();
            let (bundle, target) = prepared_update(root.path());
            fault("after-prepare", true);
            install(&target, &selected(&bundle)).unwrap_err();
            let rotation = root.path().join(ROTATION);
            let original = if kind == "container" {
                rotation.clone()
            } else {
                rotation.join(INTENT)
            };
            let saved = root.path().join("retained-original");
            std::fs::rename(&original, &saved).unwrap();
            symlink(&saved, &original).unwrap();
            let active = snapshot(&target);
            let previous = snapshot(&root.path().join(BACKUP));
            assert!(recover(&target).is_err());
            assert!(std::fs::symlink_metadata(&original).unwrap().is_symlink());
            assert_eq!(snapshot(&target), active);
            assert_eq!(snapshot(&root.path().join(BACKUP)), previous);
        }
    }

    #[test]
    fn capability_preflight_rejects_parser_errors_before_replacing_sources_or_backup() {
        for (section, command, reason) in [
            (
                "ParserScriptIndex",
                "FutureCommand \"x\"",
                "not a known command",
            ),
            (
                "ParserScriptAlbum",
                "FutureCommand \"x\"",
                "not a known command",
            ),
            ("ParserScriptAlbum", "If \"x\"", "no Endif"),
            (
                "ParserScriptAlbum",
                "SayRest unexpected",
                "takes 0 parameters",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let bundle = dir.path().join("download");
            let target = dir.path().join("sources");
            collection(&bundle);
            install(&target, &selected(&bundle)).unwrap();
            std::fs::write(
                target.join("settings.json"),
                r#"{"example":{"language":"de"}}"#,
            )
            .unwrap();
            install(&target, &selected(&bundle)).unwrap();
            let backup = target.parent().unwrap().join(BACKUP);
            let before = snapshot(&target);
            let previous = snapshot(&backup);
            let script = if section == "ParserScriptIndex" {
                format!("[IndexUrl]=https://example.invalid/%s\n[{section}]=...\n{command}\n[ParserScriptAlbum]=...\nSayRest\n")
            } else {
                format!("[MinAppVersionWin]=1.0\n[{section}]=...\n{command}\n")
            };
            std::fs::write(bundle.join("Common.inc"), script).unwrap();
            let error = install(&target, &selected(&bundle)).unwrap_err();
            assert!(
                error.contains(&format!("Common.inc [{section}] line 3 ({command})")),
                "{error}"
            );
            assert!(error.contains(reason), "{error}");
            assert_eq!(snapshot(&target), before);
            assert_eq!(snapshot(&backup), previous);
        }
    }

    #[test]
    fn capability_preflight_rejects_an_include_update_that_breaks_a_working_source() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let before = snapshot(&target);
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("SayRest", "FutureCommand"),
        )
        .unwrap();
        let error = install(&target, &selected(&bundle.join("Common.inc"))).unwrap_err();
        assert!(error.contains("Shop#&Album.src"), "{error}");
        assert!(
            error.contains("Common.inc [ParserScriptAlbum] line 3 (FutureCommand)"),
            "{error}"
        );
        assert_eq!(snapshot(&target), before);
    }

    #[test]
    fn capability_preflight_accepts_supported_scripts_with_unrelated_minimum_versions() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::fs::write(
            bundle.join("Common.inc"),
            format!("[MinAppVersionWin]=999.0\n[MinAppVersionMac]=999.0\n{INCLUDE}"),
        )
        .unwrap();
        install(&target, &selected(&bundle)).unwrap();
        let source = load(&target, "Shop#&Album.src").unwrap();
        assert_eq!(source.min_app_version_win, "999.0");
        assert_eq!(source.min_app_version_mac, "999.0");
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn capability_preflight_keeps_existing_incompatible_sources_without_blocking_an_install() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::fs::create_dir_all(&target).unwrap();
        let old = "[AlbumUrl]=https://example.invalid/%s\n[ParserScriptAlbum]=...\nFutureCommand\n";
        std::fs::write(target.join("Old.src"), old).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Old.src")).unwrap(),
            old
        );
        assert!(target.join("Shop#&Album.src").is_file());
    }
    #[test]
    fn folder_updates_preserve_values_other_files_and_previous_complete_install() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("data/sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let values = br#"{"example":{"language":"de"}}"#;
        std::fs::write(target.join("settings.json"), values).unwrap();
        std::fs::write(target.join("notes.txt"), "local file").unwrap();
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("example.invalid", "changed.invalid"),
        )
        .unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(std::fs::read(target.join("settings.json")).unwrap(), values);
        assert_eq!(
            std::fs::read_to_string(target.join("notes.txt")).unwrap(),
            "local file"
        );
        assert!(
            std::fs::read_to_string(target.parent().unwrap().join(BACKUP).join("Common.inc"))
                .unwrap()
                .contains("example.invalid")
        );
        let listing = super::super::list(&target);
        assert_eq!(listing.sources.len(), 1);
        assert_eq!(listing.sources[0].menu, "Shop#&Album");
        assert!(listing.problems.is_empty());
    }
    #[test]
    fn existing_hidden_directories_survive_an_update() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("data/sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let metadata = target.join(".git/logs/refs/remotes/origin");
        std::fs::create_dir_all(&metadata).unwrap();
        std::fs::write(metadata.join("main"), b"retained local history").unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(
            std::fs::read(metadata.join("main")).unwrap(),
            b"retained local history"
        );
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn existing_empty_directories_survive_consecutive_updates_and_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let directories = [
            "Empty",
            ".git/objects",
            ".git/refs/heads",
            "Cache/Nested/Empty",
        ];
        for name in directories {
            std::fs::create_dir_all(target.join(name)).unwrap();
        }
        for _ in 0..2 {
            install(&target, &selected(&bundle)).unwrap();
            for name in directories {
                assert!(
                    target.join(name).is_dir(),
                    "active directory {name} was lost"
                );
                assert!(
                    dir.path().join(BACKUP).join(name).is_dir(),
                    "snapshot directory {name} was lost"
                );
            }
        }
        std::fs::remove_dir_all(&target).unwrap();
        prepare_folder_open(&target).unwrap();
        for name in directories {
            assert!(
                target.join(name).is_dir(),
                "recovered directory {name} was lost"
            );
        }
    }

    #[test]
    fn existing_empty_directories_retain_the_64_level_bound() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let mut deepest = target.clone();
        for _ in 0..MAX_EXISTING_DEPTH {
            deepest.push("e");
        }
        std::fs::create_dir_all(&deepest).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert!(deepest.is_dir());
        let too_deep = deepest.join("e");
        std::fs::create_dir(&too_deep).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("64 levels deep"));
        assert!(
            too_deep.is_dir(),
            "rejected update must leave existing directories intact"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
    }

    #[test]
    fn existing_empty_directories_cannot_be_replaced_by_incoming_files() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        std::fs::create_dir(target.join("Reserved.src")).unwrap();
        std::fs::write(bundle.join("Reserved.src"), SOURCE).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        assert!(target.join("Reserved.src").is_dir());
        assert!(!dir.path().join(BACKUP).exists());
    }

    #[test]
    fn existing_empty_directories_keep_their_case_for_new_children() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle.join("keptcase"));
        collection(&target);
        std::fs::create_dir(target.join("KeptCase")).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        let names = std::fs::read_dir(&target)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>();
        assert!(names.iter().any(|name| name == "KeptCase"));
        assert!(!names.iter().any(|name| name == "keptcase"));
        assert!(target.join("KeptCase/Shop#&Album.src").is_file());
    }

    #[test]
    fn existing_empty_directories_count_toward_the_combined_entry_limit() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::fs::create_dir(&target).unwrap();
        for index in 0..MAX_ENTRIES {
            std::fs::create_dir(target.join(format!("empty-{index:04}"))).unwrap();
        }
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("4096"));
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), MAX_ENTRIES);
        assert!(!dir.path().join(BACKUP).exists());
    }

    #[cfg(unix)]
    #[test]
    fn native_metadata_paths_do_not_alias_unicode_source_identifiers() {
        use std::os::unix::ffi::OsStringExt;
        let opaque = PathBuf::from(std::ffi::OsString::from_vec(b"Upper-\xff".to_vec()));
        assert!(
            opaque.as_os_str().to_os_string().into_string().is_err(),
            "legacy conversion rejects this native name"
        );
        let mut existing = ExistingTree::default();
        existing.directories.insert(opaque.clone());
        existing
            .files
            .insert(opaque.join("notes.bin"), b"native metadata".to_vec());
        existing.directories.insert(PathBuf::from("Known"));
        existing.files.insert(
            Path::new("Known").join(std::ffi::OsString::from_vec(b"entry-\xfe".to_vec())),
            Vec::new(),
        );
        let incoming = Files::from([
            ("upper-\u{fffd}/Child.inc".into(), b"new source".to_vec()),
            ("known/Child.inc".into(), b"new child".to_vec()),
        ]);
        let prepared = prepare_incoming(&existing, incoming).unwrap();
        assert!(prepared.contains_key("upper-\u{fffd}/Child.inc"));
        assert!(!prepared.contains_key("Upper-\u{fffd}/Child.inc"));
        assert!(prepared.contains_key("Known/Child.inc"));
        assert!(existing.files.contains_key(&opaque.join("notes.bin")));
        assert!(source_name(&opaque).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn native_metadata_paths_remain_in_the_combined_entry_budget() {
        use std::os::unix::ffi::OsStringExt;
        let mut existing = ExistingTree::default();
        for index in 0..MAX_ENTRIES {
            let mut name = format!("metadata-{index}-").into_bytes();
            name.push(0xff);
            existing.files.insert(
                PathBuf::from(std::ffi::OsString::from_vec(name)),
                Vec::new(),
            );
        }
        let incoming = Files::from([("New.inc".into(), b"new source".to_vec())]);
        assert!(prepare_incoming(&existing, incoming)
            .unwrap_err()
            .contains("4096"));
    }

    // APFS rejects these names. Protected Linux CI exercises the real filesystem case.
    #[cfg(unix)]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "requires a Linux filesystem accepting non-UTF-8 names"
    )]
    #[test]
    fn existing_non_unicode_entries_survive_updates_snapshots_and_recovery() {
        use std::os::unix::{ffi::OsStringExt, fs::symlink};
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let first = PathBuf::from(std::ffi::OsString::from_vec(b"metadata-\xff".to_vec()));
        let second = PathBuf::from(std::ffi::OsString::from_vec(b"metadata-\xfe".to_vec()));
        let native_dir = PathBuf::from(std::ffi::OsString::from_vec(b".cache-\xfd".to_vec()));
        let empty = native_dir.join(std::ffi::OsString::from_vec(b"empty-\xfc".to_vec()));
        let nested = native_dir.join(std::ffi::OsString::from_vec(b"value-\xfb".to_vec()));
        std::fs::create_dir_all(target.join(&empty)).unwrap();
        for (name, bytes) in [
            (&first, b"first".as_slice()),
            (&second, b"second"),
            (&nested, b"nested"),
        ] {
            std::fs::write(target.join(name), bytes).unwrap();
        }
        std::fs::write(target.join("metadata-\u{fffd}"), b"Unicode peer").unwrap();
        for _ in 0..2 {
            install(&target, &selected(&bundle)).unwrap();
            for root in [&target, &dir.path().join(BACKUP)] {
                assert_eq!(std::fs::read(root.join(&first)).unwrap(), b"first");
                assert_eq!(std::fs::read(root.join(&second)).unwrap(), b"second");
                assert_eq!(std::fs::read(root.join(&nested)).unwrap(), b"nested");
                assert_eq!(
                    std::fs::read(root.join("metadata-\u{fffd}")).unwrap(),
                    b"Unicode peer"
                );
                assert!(root.join(&empty).is_dir());
            }
        }
        std::fs::remove_dir_all(&target).unwrap();
        prepare_folder_open(&target).unwrap();
        assert_eq!(std::fs::read(target.join(&first)).unwrap(), b"first");
        assert!(target.join(&empty).is_dir());
        let outside = dir.path().join("outside.txt");
        std::fs::write(&outside, b"outside stays private").unwrap();
        let link = std::ffi::OsString::from_vec(b"link-\xfa".to_vec());
        symlink(&outside, target.join(link)).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("symbolic links"));
        assert_eq!(std::fs::read(&outside).unwrap(), b"outside stays private");
    }

    #[cfg(unix)]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "requires a Linux filesystem accepting non-UTF-8 names"
    )]
    #[test]
    fn incoming_non_unicode_source_names_still_reject_without_replacement() {
        use std::os::unix::ffi::OsStringExt;
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let name = std::ffi::OsString::from_vec(b"incoming-\xff.src".to_vec());
        std::fs::write(bundle.join(name), SOURCE).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("non-Unicode"));
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert!(!dir.path().join(BACKUP).exists());
    }

    #[test]
    fn folder_open_restores_an_interrupted_installation_before_creating_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        let backup = dir.path().join(BACKUP);
        collection(&backup);
        std::fs::write(backup.join(MARKER), b"source installer backup v1\n").unwrap();
        prepare_folder_open(&target).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert!(!backup.exists());
    }

    #[test]
    fn folder_open_waits_for_an_installation_swap() {
        use std::sync::mpsc;
        use std::time::Duration;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        let backup = dir.path().join(BACKUP);
        let stage = dir.path().join("stage");
        collection(&target);
        collection(&stage);
        std::fs::write(stage.join("notes.txt"), "new complete installation").unwrap();
        let guard = LOCK.lock().unwrap();
        std::fs::write(target.join(MARKER), b"source installer backup v1\n").unwrap();
        std::fs::rename(&target, &backup).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let folder = target.clone();
        let opener = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let result = prepare_folder_open(&folder);
            done_tx.send(result).unwrap();
        });
        started_rx.recv().unwrap();
        assert!(matches!(
            done_rx.recv_timeout(Duration::from_millis(250)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert!(
            !target.exists(),
            "folder opening must not recreate the swap destination"
        );
        std::fs::rename(&stage, &target).unwrap();
        drop(guard);
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        opener.join().unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("notes.txt")).unwrap(),
            "new complete installation"
        );
        assert!(backup.join("Common.inc").is_file());
    }

    #[test]
    fn second_file_and_dependency_errors_leave_every_installed_file_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::write(bundle.join("Common.inc"), "updated").unwrap();
        std::fs::write(bundle.join("Options.settings"), "invalid JSON").unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        std::fs::write(bundle.join("Options.settings"), SETTINGS).unwrap();
        std::fs::remove_file(bundle.join("Common.inc")).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("complete bundle"));
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
    }
    #[test]
    fn explicit_file_update_reuses_installed_companions_without_weakening_bundle_preflight() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::remove_file(bundle.join("Common.inc")).unwrap();
        std::fs::remove_file(bundle.join("Options.settings")).unwrap();
        install(&target, &selected(&bundle.join("Shop#&Album.src"))).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        std::fs::write(
            bundle.join("Common.inc"),
            format!("{INCLUDE}\n[Settings]=Absent.settings\n"),
        )
        .unwrap();
        assert!(install(&target, &selected(&bundle.join("Common.inc"))).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn nested_zip_preflights_all_companions_and_rejects_duplicate_and_escape_paths() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bundle.zip");
        let target = dir.path().join("sources");
        zip(
            &archive,
            &[
                ("Shop/Album.src", SOURCE),
                ("Shop/Common.inc", INCLUDE),
                ("Shop/Options.settings", SETTINGS),
                (".DS_Store", "ignored download metadata"),
                ("__MACOSX/._Shop", "ignored download metadata"),
            ],
        );
        install(&target, &selected(&archive)).unwrap();
        assert!(target.join("Shop/Album.src").exists());
        for entries in [
            vec![("../outside.src", INCLUDE)],
            vec![("A.src", INCLUDE), ("a.SRC", INCLUDE)],
            vec![("C:/evil.src", INCLUDE)],
            vec![("foo/../../evil.src", INCLUDE)],
        ] {
            zip(&archive, &entries);
            assert!(install(&target, &selected(&archive)).is_err());
            assert!(target.join("Shop/Album.src").exists());
        }
        assert!(!dir.path().join("outside.src").exists());
    }
    #[test]
    fn updates_reuse_installed_directory_spelling_and_preserve_ids() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle.join("Shop"));
        install(&target, &selected(&bundle)).unwrap();
        std::fs::rename(bundle.join("Shop"), bundle.join("shop")).unwrap();
        std::fs::rename(
            bundle.join("shop/Shop#&Album.src"),
            bundle.join("shop/shop#&album.SRC"),
        )
        .unwrap();
        std::fs::write(bundle.join("shop/Extra.inc"), INCLUDE).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert!(target.join("Shop/Extra.inc").is_file());
        let directories = std::fs::read_dir(&target)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(directories, ["Shop"]);
        assert_eq!(
            super::super::list(&target).sources[0].id,
            "Shop/Shop#&Album.src"
        );
    }

    #[test]
    fn interrupted_swap_restores_previous_complete_sources_on_next_read() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        std::fs::remove_dir_all(&target).unwrap();
        assert_eq!(super::super::list(&target).sources.len(), 1);
        assert!(!dir.path().join(BACKUP).exists());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_folder_and_zip_symlinks_and_bounded_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::os::unix::fs::symlink(bundle.join("Common.inc"), bundle.join("Link.inc")).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("symbolic links"));
        std::fs::remove_file(bundle.join("Link.inc")).unwrap();
        std::fs::write(bundle.join("Big.inc"), vec![0; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        let archive = dir.path().join("links.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        writer
            .add_symlink(
                "Link.src",
                "elsewhere",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        assert!(install(&target, &selected(&archive))
            .unwrap_err()
            .contains("symbolic links"));
        assert!(!target.exists());
    }
    #[test]
    #[ignore = "read-only qualification of user-owned local source files; requires OPENTAG_SOURCE_BUNDLE"]
    fn qualify_private_bundle() {
        let bundle = std::env::var("OPENTAG_SOURCE_BUNDLE").expect("OPENTAG_SOURCE_BUNDLE");
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        install(&target, &[bundle]).expect("bundle preflight/install failed");
        let listing = super::super::list(&target);
        assert!(listing.problems.is_empty());
        let mut scripts = 0;
        for source in &listing.sources {
            let file = load(&target, &source.id).unwrap();
            for block in [file.index_script, file.album_script].into_iter().flatten() {
                super::super::engine::Script::compile(&block.lines)
                    .expect("parser compilation failed");
                scripts += 1;
            }
        }
        println!("qualification: {} sources, {} settings schemas, {scripts} parser blocks compile; no provider requests, metadata changes or source text recorded", listing.sources.len(), listing.settings.len());
    }
}
