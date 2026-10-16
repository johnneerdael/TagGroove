use std::path::{Path, PathBuf};

/// Structured error provenance for the no-clobber rename helper. The existing
/// String API remains the default; converter rollback UI uses these parts.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RenameFailure {
    External {
        error: String,
    },
    OnlyRegular,
    CannotCreate {
        path: String,
        error: String,
    },
    CopyRollback {
        error: CopyFailure,
        rollback: String,
    },
    SourceChangedRollback {
        rollback: String,
    },
    RemoveOriginalRollback {
        error: String,
        rollback: String,
    },
    NoName,
    NoTemporaryName,
    CannotRename {
        path: String,
        error: String,
    },
    AliasRollback {
        source: String,
        destination: String,
        error: String,
        rollback: String,
    },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopyFailure {
    External { error: String },
    SourceChanged,
}
impl std::fmt::Display for RenameFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::External { error } => write!(f, "{error}"),
            Self::OnlyRegular => write!(f, "Only regular files can be renamed"),
            Self::CannotCreate { path, error } => write!(f, "Cannot create {path}: {error}"),
            Self::CopyRollback { error, rollback } => {
                match error {
                    CopyFailure::External { error } => write!(f, "{error}")?,
                    CopyFailure::SourceChanged => write!(f, "Source changed while copying; original was retained")?,
                }
                write!(f, "; destination rollback: {rollback}")
            }
            Self::SourceChangedRollback { rollback } => write!(f, "Source changed before move completed; original was retained; destination rollback: {rollback}"),
            Self::RemoveOriginalRollback { error, rollback } => write!(f, "Cannot remove original: {error}; destination rollback: {rollback}"),
            Self::NoName => write!(f, "The source has no name"),
            Self::NoTemporaryName => write!(f, "No free temporary name for the case-only rename"),
            Self::CannotRename { path, error } => write!(f, "Cannot rename {path}: {error}"),
            Self::AliasRollback { source, destination, error, rollback } => write!(f, "Cannot rename {source} to {destination}: {error}; rollback: {rollback}"),
        }
    }
}
fn reported(failure: &mut Option<RenameFailure>, error: RenameFailure) -> String {
    let message = error.to_string();
    *failure = Some(error);
    message
}

fn same_source(before: &std::fs::Metadata, after: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return false;
        }
    }
    before.len() == after.len() && before.modified().ok() == after.modified().ok()
}

fn copy_new(
    source: &mut std::fs::File,
    destination: &Path,
    metadata: &std::fs::Metadata,
) -> Result<(), String> {
    copy_reported(source, destination, metadata, &mut None)
}
fn copy_reported(
    source: &mut std::fs::File,
    destination: &Path,
    metadata: &std::fs::Metadata,
    failure: &mut Option<RenameFailure>,
) -> Result<(), String> {
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| {
            reported(
                failure,
                RenameFailure::CannotCreate {
                    path: destination.to_string_lossy().into_owned(),
                    error: e.to_string(),
                },
            )
        })?;
    let mut source_changed = false;
    let result = (|| {
        let copied = std::io::copy(source, &mut output).map_err(|e| e.to_string())?;
        if copied != metadata.len()
            || !same_source(metadata, &source.metadata().map_err(|e| e.to_string())?)
        {
            source_changed = true;
            return Err("Source changed while copying; original was retained".into());
        }
        let mut times = std::fs::FileTimes::new();
        if let Ok(time) = metadata.modified() {
            times = times.set_modified(time);
        }
        if let Ok(time) = metadata.accessed() {
            times = times.set_accessed(time);
        }
        output.set_times(times).map_err(|e| e.to_string())?;
        output
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())
    })();
    drop(output);
    if let Err(error) = result {
        let rollback = remove_copy(destination);
        return Err(reported(
            failure,
            RenameFailure::CopyRollback {
                error: if source_changed {
                    CopyFailure::SourceChanged
                } else {
                    CopyFailure::External { error }
                },
                rollback: format!("{rollback:?}"),
            },
        ));
    }
    Ok(())
}

fn remove_copy(path: &Path) -> std::io::Result<()> {
    // A copied Windows read-only attribute must not prevent rollback. This file
    // has an independent inode; never apply this to a hard-linked destination.
    #[cfg(windows)]
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            // This statement is Windows-only: it clears the DOS read-only
            // attribute, never Unix mode bits or access-control permissions.
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    std::fs::remove_file(path)
}

/// Whether two existing paths reach the same file or folder.
fn same_identity(
    a: &Path,
    a_meta: &std::fs::Metadata,
    b: &Path,
    b_meta: &std::fs::Metadata,
) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = (a, b);
        a_meta.dev() == b_meta.dev() && a_meta.ino() == b_meta.ino()
    }
    #[cfg(not(unix))]
    {
        // Windows resolves both through their handles to the stored spelling.
        let _ = (a_meta, b_meta);
        matches!(
            (std::fs::canonicalize(a), std::fs::canonicalize(b)),
            (Ok(a), Ok(b)) if a == b
        )
    }
}

/// How a rename destination relates to its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Nothing exists at the destination.
    Free,
    /// The destination is the source itself, spelled in another letter case
    /// or Unicode normalization on a case-insensitive volume (macOS, Windows).
    Alias,
    /// The destination spells every name like the source except the case of a
    /// folder above it, so renaming it changes nothing.
    Unchanged,
    /// A different file or folder exists at the destination.
    Taken,
}

fn parent_of(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// Classify `destination` for a rename of `source`. `identity` tells whether
/// two existing paths reach the same entry and `listed` whether a folder lists
/// a name byte for byte. A second hard link to the same file is listed under
/// its own name, so it stays taken; a case-insensitive alias is never listed.
fn classify(
    source: &Path,
    destination: &Path,
    exists: impl Fn(&Path) -> bool,
    identity: impl Fn(&Path, &Path) -> bool,
    listed: impl Fn(&Path, &std::ffi::OsStr) -> bool,
) -> Destination {
    if !exists(destination) {
        return Destination::Free;
    }
    if source == destination {
        return Destination::Unchanged;
    }
    let (Some(name), Some(_)) = (destination.file_name(), source.file_name()) else {
        return Destination::Taken;
    };
    let parent = parent_of(destination);
    // Hard links in two folders share an identity but are distinct entries.
    if !identity(source, destination) || !identity(parent_of(source), parent) {
        return Destination::Taken;
    }
    if source.file_name() == Some(name) {
        Destination::Unchanged
    } else if listed(parent, name) {
        Destination::Taken
    } else {
        Destination::Alias
    }
}

/// Inspect a rename destination on the real file system.
pub fn destination_state(source: &Path, destination: &Path) -> Destination {
    classify(
        source,
        destination,
        |path| std::fs::symlink_metadata(path).is_ok(),
        |a, b| match (std::fs::symlink_metadata(a), std::fs::symlink_metadata(b)) {
            (Ok(a_meta), Ok(b_meta)) => same_identity(a, &a_meta, b, &b_meta),
            _ => false,
        },
        |parent, name| {
            std::fs::read_dir(parent)
                .map(|entries| entries.flatten().any(|entry| entry.file_name() == name))
                // Unreadable folders are treated as listing the name: never clobber.
                .unwrap_or(true)
        },
    )
}

/// Whether `destination` is `source` spelled with another letter case.
pub fn is_case_alias(source: &Path, destination: &Path) -> bool {
    matches!(
        destination_state(source, destination),
        Destination::Alias | Destination::Unchanged
    )
}

/// Move one entry to a path that does not exist, without replacing anything
/// that appears there meanwhile where the platform allows it.
fn move_entry(from: &Path, to: &Path, is_file: bool) -> std::io::Result<()> {
    if is_file {
        match std::fs::hard_link(from, to) {
            Ok(()) => return std::fs::remove_file(from),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err(e),
            // File systems without hard links (FAT, exFAT) fall back to rename.
            Err(_) => {}
        }
    }
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", to.display()),
        ));
    }
    std::fs::rename(from, to)
}

/// Change only the spelling of an entry on a case-insensitive volume. A direct
/// rename can be refused or ignored there, so the entry moves through a unique
/// temporary name in its own folder and back to the new spelling.
fn rename_alias(source: &Path, destination: &Path) -> Result<(), String> {
    rename_alias_reported(source, destination, &mut None)
}
fn rename_alias_reported(
    source: &Path,
    destination: &Path,
    failure: &mut Option<RenameFailure>,
) -> Result<(), String> {
    let is_file = std::fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .is_file();
    let parent = parent_of(source);
    let name = source
        .file_name()
        .ok_or_else(|| reported(failure, RenameFailure::NoName))?;
    let staging = (0..1000)
        .map(|n| {
            let mut staged = std::ffi::OsString::from(".");
            staged.push(name);
            staged.push(format!(".opentag-case-{}-{n}", std::process::id()));
            parent.join(staged)
        })
        .find(|path| {
            matches!(std::fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        })
        .ok_or_else(|| reported(failure, RenameFailure::NoTemporaryName))?;
    move_entry(source, &staging, is_file).map_err(|e| {
        reported(
            failure,
            RenameFailure::CannotRename {
                path: source.to_string_lossy().into_owned(),
                error: e.to_string(),
            },
        )
    })?;
    if let Err(error) = move_entry(&staging, destination, is_file) {
        let rollback = move_entry(&staging, source, is_file);
        return Err(reported(
            failure,
            RenameFailure::AliasRollback {
                source: source.to_string_lossy().into_owned(),
                destination: destination.to_string_lossy().into_owned(),
                error: error.to_string(),
                rollback: format!("{rollback:?}"),
            },
        ));
    }
    Ok(())
}

fn record_moved_path(source: Option<&Path>, destination: &Path, copied: bool) {
    if let (Some(source), Ok(destination)) = (source, dunce::canonicalize(destination)) {
        crate::history::note_move(source, &destination, copied);
    }
}

/// Rename a folder without replacing an existing one. Changing only the letter
/// case of the folder's name works on case-insensitive volumes too.
pub fn rename_directory_no_clobber(source: &Path, destination: &Path) -> Result<(), String> {
    let tracked_source = dunce::canonicalize(source).ok();
    if !std::fs::symlink_metadata(source)
        .map_err(|e| format!("{}: {e}", source.display()))?
        .is_dir()
    {
        return Err(format!("{} is not a folder", source.display()));
    }
    match destination_state(source, destination) {
        Destination::Unchanged => return Ok(()),
        Destination::Alias => {
            rename_alias(source, destination)?;
            record_moved_path(tracked_source.as_deref(), destination, false);
            return Ok(());
        }
        Destination::Taken => return Err(format!("{} already exists", destination.display())),
        Destination::Free => {}
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    move_entry(source, destination, false).map_err(|e| e.to_string())?;
    record_moved_path(tracked_source.as_deref(), destination, false);
    Ok(())
}

pub fn rename_no_clobber(source: &Path, destination: &Path) -> Result<(), String> {
    rename_reported(source, destination, &mut None)
}
/// The same operation and guards, retaining owned scaffolds and raw IO/path parts.
pub fn rename_no_clobber_diagnostic(
    source: &Path,
    destination: &Path,
) -> Result<(), RenameFailure> {
    let mut failure = None;
    rename_reported(source, destination, &mut failure)
        .map_err(|error| failure.unwrap_or(RenameFailure::External { error }))
}
fn rename_reported(
    source: &Path,
    destination: &Path,
    failure: &mut Option<RenameFailure>,
) -> Result<(), String> {
    let tracked_source = dunce::canonicalize(source).ok();
    if source == destination {
        return Ok(());
    }
    if !std::fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err(reported(failure, RenameFailure::OnlyRegular));
    }
    match destination_state(source, destination) {
        Destination::Unchanged => return Ok(()),
        Destination::Alias => {
            rename_alias_reported(source, destination, failure)?;
            record_moved_path(tracked_source.as_deref(), destination, false);
            return Ok(());
        }
        Destination::Taken | Destination::Free => {}
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // hard_link creates the destination atomically and fails if it already exists.
    // It retains every byte and the source's permissions without a rename-overwrite race.
    let mut input = std::fs::File::open(source).map_err(|e| e.to_string())?;
    let metadata = input.metadata().map_err(|e| e.to_string())?;
    let copied = match std::fs::hard_link(source, destination) {
        Ok(()) => false,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(reported(
                failure,
                RenameFailure::CannotCreate {
                    path: destination.to_string_lossy().into_owned(),
                    error: e.to_string(),
                },
            ))
        }
        // Cross-volume moves and filesystems without hard links use an exclusive
        // copy. Do not delete the source until the complete copy is durable.
        Err(_) => {
            copy_reported(&mut input, destination, &metadata, failure)?;
            true
        }
    };
    let unchanged = std::fs::symlink_metadata(source)
        .is_ok_and(|current| current.is_file() && same_source(&metadata, &current));
    if !unchanged {
        let cleanup = if copied {
            remove_copy(destination)
        } else {
            std::fs::remove_file(destination)
        };
        return Err(reported(
            failure,
            RenameFailure::SourceChangedRollback {
                rollback: format!("{cleanup:?}"),
            },
        ));
    }
    if let Err(e) = std::fs::remove_file(source) {
        let cleanup = if copied {
            remove_copy(destination)
        } else {
            std::fs::remove_file(destination)
        };
        return Err(reported(
            failure,
            RenameFailure::RemoveOriginalRollback {
                error: e.to_string(),
                rollback: format!("{cleanup:?}"),
            },
        ));
    }
    record_moved_path(tracked_source.as_deref(), destination, copied);
    Ok(())
}

/// Copy a regular file to a new path. The destination is created exclusively,
/// so an existing file is never replaced, and an incomplete copy is removed.
pub fn copy_no_clobber(source: &Path, destination: &Path) -> Result<(), String> {
    if !std::fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("Only regular files can be copied".into());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut input = std::fs::File::open(source).map_err(|e| e.to_string())?;
    let metadata = input.metadata().map_err(|e| e.to_string())?;
    copy_new(&mut input, destination, &metadata)
}

pub fn render_csv(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|row| {
            let values: Vec<_> = row
                .iter()
                .map(|s| {
                    if s.contains([';', '"', '\r', '\n']) {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.clone()
                    }
                })
                .collect();
            format!("{}\r\n", values.join(";"))
        })
        .collect()
}

pub fn playlist_path(source: &Path, playlist: &Path) -> PathBuf {
    let parent = playlist.parent().unwrap_or(Path::new("."));
    let src: Vec<_> = source.components().collect();
    let base: Vec<_> = parent.components().collect();
    if src.first() != base.first() {
        return source.to_path_buf();
    }
    let shared = src.iter().zip(&base).take_while(|(a, b)| a == b).count();
    let mut result = PathBuf::new();
    for _ in &base[shared..] {
        result.push("..");
    }
    for part in &src[shared..] {
        result.push(part.as_os_str());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_preserves_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("first.mp3");
        let dst = dir.path().join("second.mp3");
        std::fs::write(&src, b"first").unwrap();
        std::fs::write(&dst, b"second").unwrap();
        let legacy = rename_no_clobber(&src, &dst).unwrap_err();
        let diagnostic = rename_no_clobber_diagnostic(&src, &dst).unwrap_err();
        let RenameFailure::CannotCreate { path, error } = &diagnostic else {
            panic!("missing owned create provenance: {diagnostic:?}");
        };
        assert_eq!(path, &dst.to_string_lossy());
        assert!(!error.is_empty());
        assert_eq!(legacy, diagnostic.to_string());
        assert_eq!(
            serde_json::to_value(&diagnostic).unwrap()["kind"],
            "cannot_create"
        );
        assert_eq!(std::fs::read(src).unwrap(), b"first");
        assert_eq!(std::fs::read(dst).unwrap(), b"second");
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// A simulated folder holding `entries`, case-insensitive or not.
    fn simulate(
        entries: &[&str],
        insensitive: bool,
        source: &str,
        destination: &str,
    ) -> Destination {
        let key = |path: &Path| {
            let text = path.to_string_lossy().into_owned();
            if insensitive {
                text.to_lowercase()
            } else {
                text
            }
        };
        let stored = |path: &Path| {
            entries
                .iter()
                .map(Path::new)
                .find(|entry| key(entry) == key(path))
                .or_else(|| (key(path) == key(Path::new("/m"))).then_some(Path::new("/m")))
        };
        classify(
            Path::new(source),
            Path::new(destination),
            |path| stored(path).is_some(),
            // `link` and `linked` are hard links to one file.
            |a, b| {
                let inode =
                    |p: &Path| stored(p).map(|s| s.to_string_lossy().replace("linked", "link"));
                inode(a).is_some() && inode(a) == inode(b)
            },
            |parent, name| {
                entries
                    .iter()
                    .any(|entry| Path::new(entry) == parent.join(name))
            },
        )
    }

    #[test]
    fn destinations_are_classified_for_both_kinds_of_volume() {
        let folder = [
            "/m/Song.mp3",
            "/m/Other.mp3",
            "/m/link.mp3",
            "/m/linked.mp3",
        ];
        for insensitive in [true, false] {
            let state = |a, b| simulate(&folder, insensitive, a, b);
            assert_eq!(state("/m/Song.mp3", "/m/New.mp3"), Destination::Free);
            assert_eq!(state("/m/Song.mp3", "/m/Other.mp3"), Destination::Taken);
            assert_eq!(state("/m/link.mp3", "/m/linked.mp3"), Destination::Taken);
            assert_eq!(state("/m/Song.mp3", "/m/Song.mp3"), Destination::Unchanged);
        }
        let insensitive = |a, b| simulate(&folder, true, a, b);
        assert_eq!(
            insensitive("/m/Song.mp3", "/m/song.mp3"),
            Destination::Alias
        );
        assert_eq!(
            insensitive("/m/Song.mp3", "/m/SONG.MP3"),
            Destination::Alias
        );
        assert_eq!(
            insensitive("/m/Song.mp3", "/m/other.mp3"),
            Destination::Taken
        );
        assert_eq!(
            insensitive("/m/Song.mp3", "/M/Song.mp3"),
            Destination::Unchanged
        );
        let sensitive = |a, b| simulate(&folder, false, a, b);
        assert_eq!(sensitive("/m/Song.mp3", "/m/song.mp3"), Destination::Free);
        assert_eq!(sensitive("/m/Song.mp3", "/m/other.mp3"), Destination::Free);
    }

    #[test]
    fn case_only_renames_work_and_never_replace_another_file() {
        let dir = tempfile::tempdir().unwrap();
        let song = dir.path().join("Song.mp3");
        let other = dir.path().join("Other.mp3");
        std::fs::write(&song, b"song").unwrap();
        std::fs::write(&other, b"other").unwrap();
        let insensitive = dir.path().join("song.mp3").exists();
        rename_no_clobber(&song, &dir.path().join("song.mp3")).unwrap();
        assert_eq!(names(dir.path()), ["Other.mp3", "song.mp3"]);
        rename_no_clobber(&dir.path().join("song.mp3"), &dir.path().join("SONG.MP3")).unwrap();
        assert_eq!(names(dir.path()), ["Other.mp3", "SONG.MP3"]);
        assert_eq!(std::fs::read(dir.path().join("SONG.MP3")).unwrap(), b"song");
        // On a case-insensitive volume other.mp3 is Other.mp3 and must survive.
        let result = rename_no_clobber(&dir.path().join("SONG.MP3"), &dir.path().join("other.mp3"));
        assert_eq!(result.is_err(), insensitive);
        assert_eq!(std::fs::read(&other).unwrap(), b"other");
        assert!(!names(dir.path()).iter().any(|n| n.contains("opentag-case")));
    }

    #[test]
    fn hard_links_are_distinct_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.mp3");
        let second = dir.path().join("second.mp3");
        std::fs::write(&first, b"audio").unwrap();
        std::fs::hard_link(&first, &second).unwrap();
        assert_eq!(destination_state(&first, &second), Destination::Taken);
        assert!(rename_no_clobber(&first, &second).is_err());
        assert_eq!(names(dir.path()), ["first.mp3", "second.mp3"]);
    }

    #[test]
    fn folders_change_case_and_keep_other_folders() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("Album");
        std::fs::create_dir(&album).unwrap();
        std::fs::write(album.join("track.mp3"), b"audio").unwrap();
        std::fs::create_dir(dir.path().join("Other")).unwrap();
        rename_directory_no_clobber(&album, &dir.path().join("ALBUM")).unwrap();
        assert_eq!(names(dir.path()), ["ALBUM", "Other"]);
        assert_eq!(
            std::fs::read(dir.path().join("ALBUM/track.mp3")).unwrap(),
            b"audio"
        );
        assert!(
            rename_directory_no_clobber(&dir.path().join("ALBUM"), &dir.path().join("Other"))
                .is_err()
        );
        rename_directory_no_clobber(&dir.path().join("ALBUM"), &dir.path().join("New/Album"))
            .unwrap();
        assert_eq!(names(&dir.path().join("New")), ["Album"]);
    }

    #[test]
    fn rename_creates_nested_destination_and_preserves_contents() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("first.mp3");
        let dst = dir.path().join("album/second.mp3");
        std::fs::write(&src, b"audio").unwrap();
        rename_no_clobber(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_eq!(std::fs::read(dst).unwrap(), b"audio");
    }

    #[test]
    fn exclusive_copy_preserves_bytes_times_permissions_and_existing_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.flac");
        let target = dir.path().join("copied.flac");
        let bytes = (0..131_073).map(|i| (i % 251) as u8).collect::<Vec<_>>();
        std::fs::write(&source, &bytes).unwrap();
        // Windows requires a writable handle to set fixture timestamps.
        let input = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&source)
            .unwrap();
        input
            .set_times(std::fs::FileTimes::new().set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
            ))
            .unwrap();
        let metadata = input.metadata().unwrap();
        copy_new(
            &mut std::fs::File::open(&source).unwrap(),
            &target,
            &metadata,
        )
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert_eq!(
            std::fs::metadata(&target).unwrap().modified().unwrap(),
            metadata.modified().unwrap()
        );
        assert_eq!(
            std::fs::metadata(&target).unwrap().permissions(),
            metadata.permissions()
        );
        assert!(copy_new(
            &mut std::fs::File::open(&source).unwrap(),
            &target,
            &metadata
        )
        .is_err());
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }

    #[test]
    fn copy_keeps_source_and_never_replaces_destination() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("first.mp3");
        let dst = dir.path().join("copies/first.mp3");
        std::fs::write(&src, b"audio").unwrap();
        copy_no_clobber(&src, &dst).unwrap();
        assert_eq!(std::fs::read(&src).unwrap(), b"audio");
        assert_eq!(std::fs::read(&dst).unwrap(), b"audio");
        std::fs::write(&dst, b"existing").unwrap();
        assert!(copy_no_clobber(&src, &dst).is_err());
        assert_eq!(std::fs::read(&dst).unwrap(), b"existing");
        assert!(copy_no_clobber(dir.path(), &dir.path().join("dir-copy")).is_err());
    }

    #[test]
    fn csv_quotes_delimiters_quotes_and_linebreaks() {
        assert_eq!(
            render_csv(&[
                vec!["Title".into(), "Artist".into()],
                vec!["one;two".into(), "say \"hi\"\nthere".into()]
            ]),
            "Title;Artist\r\n\"one;two\";\"say \"\"hi\"\"\nthere\"\r\n"
        );
    }

    #[test]
    fn playlist_paths_are_relative_to_output_directory() {
        assert_eq!(
            playlist_path(
                Path::new("/music/album/track.flac"),
                Path::new("/music/list.m3u8")
            ),
            PathBuf::from("album/track.flac")
        );
        assert_eq!(
            playlist_path(
                Path::new("/music/track.flac"),
                Path::new("/playlists/list.m3u8")
            ),
            PathBuf::from("../music/track.flac")
        );
    }
}
