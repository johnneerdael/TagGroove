//! Session-only identity tracking for explicitly loaded file roots.
//! Never enumerate a parent directory to guess the location of a missing file.
use crate::action_host::PathChange;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Identity {
    #[cfg(unix)]
    Unix(u64, u64),
    #[cfg(windows)]
    Windows(u64, [u8; 16]),
}

#[cfg(unix)]
fn identity(file: &File) -> io::Result<Identity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(Identity::Unix(metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn identity(file: &File) -> io::Result<Identity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
    };
    let mut info = FILE_ID_INFO::default();
    // SAFETY: live owned handle; correctly sized writable FileIdInfo buffer.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            std::ptr::from_mut(&mut info).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(Identity::Windows(
        info.VolumeSerialNumber,
        info.FileId.Identifier,
    ))
}

#[cfg(unix)]
fn links(file: &File) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(file.metadata()?.nlink())
}

#[cfg(windows)]
fn links(file: &File) -> io::Result<u64> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: live owned handle and correctly sized writable buffer.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(u64::from(info.nNumberOfLinks))
}

#[cfg(target_os = "macos")]
fn current_path(file: &File) -> io::Result<PathBuf> {
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStringExt;
    let mut buffer = [0u8; libc::PATH_MAX as usize];
    // SAFETY: owned live descriptor and PATH_MAX writable bytes, as required by F_GETPATH.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let length = buffer
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| io::Error::other("Source path is not terminated"))?;
    Ok(std::ffi::OsString::from_vec(buffer[..length].to_vec()).into())
}

#[cfg(target_os = "linux")]
fn current_path(file: &File) -> io::Result<PathBuf> {
    use std::os::fd::AsRawFd;
    // A returned " (deleted)" suffix is never stripped: identity verification
    // below rejects a deleted object's stale name, including forged lookalikes.
    std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

#[cfg(windows)]
fn current_path(file: &File) -> io::Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFinalPathNameByHandleW, FILE_NAME_NORMALIZED, VOLUME_NAME_DOS,
    };
    let mut buffer = vec![0u16; 256];
    loop {
        // SAFETY: live owned handle and writable buffer of the declared length.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                FILE_NAME_NORMALIZED | VOLUME_NAME_DOS,
            )
        } as usize;
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        if length < buffer.len() {
            return Ok(std::ffi::OsString::from_wide(&buffer[..length]).into());
        }
        if length > 32768 {
            return Err(io::Error::other(
                "Source path exceeds the native path limit",
            ));
        }
        buffer.resize(length + 1, 0);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn current_path(_: &File) -> io::Result<PathBuf> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Source tracking is unavailable on this platform",
    ))
}

struct TrackedFile {
    file: File,
    identity: Identity,
    canonical: PathBuf,
    address: PathBuf,
    requested: PathBuf,
}
fn open_regular(path: &Path) -> io::Result<File> {
    if !std::fs::metadata(path)?.is_file() {
        return Err(io::Error::other("Source is not a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // The metadata check can race a replacement with a FIFO. Never block
        // the global file-operation/tracking locks waiting for a pipe writer.
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("Source is not a regular file"));
    }
    Ok(file)
}
impl TrackedFile {
    fn open(path: &Path) -> io::Result<Self> {
        let file = open_regular(path)?;
        let expected = identity(&file)?;
        let canonical = dunce::canonicalize(path)?;
        if identity(&open_regular(&canonical)?)? != expected {
            return Err(io::Error::other(
                "Source changed while tracking was being registered",
            ));
        }
        Ok(Self {
            file,
            identity: expected,
            requested: path.into(),
            address: canonical.clone(),
            canonical,
        })
    }

    fn resolve(&mut self, requested: &Path) -> io::Result<PathBuf> {
        if links(&self.file)? == 0 {
            *self = Self::open(requested)?;
            return Ok(requested.into());
        }
        let candidate = match current_path(&self.file).and_then(dunce::canonicalize) {
            Ok(path) => path,
            Err(error) => {
                if open_regular(requested)
                    .and_then(|file| identity(&file))
                    .ok()
                    .as_ref()
                    == Some(&self.identity)
                {
                    self.address = dunce::canonicalize(requested)?;
                    return Ok(requested.into());
                }
                return Err(io::Error::other(format!(
                    "The source location could not be verified: {error}"
                )));
            }
        };
        if identity(&open_regular(&candidate)?)? != self.identity {
            return Err(io::Error::other("The moved source could not be verified"));
        }
        match open_regular(requested) {
            Ok(file) => {
                if identity(&file)? != self.identity {
                    return Err(io::Error::other("Another file now uses the source name or its alias was retargeted; reopen the intended file"));
                }
                // Keep valid logical aliases and unambiguous existing hard-link
                // names. A single-link case-only rename publishes its native name.
                if candidate == self.canonical || links(&self.file)? != 1 {
                    self.address = dunce::canonicalize(requested)?;
                    return Ok(requested.into());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if links(&self.file)? != 1 {
            return Err(io::Error::other(
                "The moved source has multiple hard-link names; reopen the intended file",
            ));
        }
        if candidate.to_str().is_none() {
            return Err(io::Error::other(
                "The moved source name cannot be represented by this workspace",
            ));
        }
        self.address = candidate.clone();
        Ok(candidate)
    }
}

/// Opaque session-only references. Paths remain the user's chosen logical roots.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceRef {
    path: String,
    token: Option<String>,
    error: Option<String>,
}
#[derive(Serialize)]
pub struct Prepared {
    key: String,
    sources: Vec<SourceRef>,
    acquired: Vec<String>,
}
#[derive(Serialize)]
pub struct Resolution {
    key: String,
    paths: Vec<String>,
    renamed: Vec<PathChange>,
    sources: Vec<SourceRef>,
    acquired: Vec<String>,
}
#[derive(Default)]
struct Tracker {
    session: String,
    // All captures have one lifetime and the same mutation bookkeeping. Native
    // membership never depends on an acknowledgement from the webview.
    files: HashMap<String, TrackedFile>,
}
#[derive(Default)]
pub struct MutationEpoch(HashMap<(String, Identity), u64>);
fn is_playlist(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            ["m3u", "m3u8", "pls", "cue", "xspf", "wpl", "asx"]
                .iter()
                .any(|item| extension.eq_ignore_ascii_case(item))
        })
}
fn operation_addresses(paths: &[String]) -> HashMap<PathBuf, PathBuf> {
    let mut addresses = HashMap::new();
    for path in paths {
        let path =
            tag_core::subsong_parts(path).map_or(Path::new(path), |(file, _)| Path::new(file));
        addresses.insert(path.to_path_buf(), path.to_path_buf());
        if let Ok(canonical) = dunce::canonicalize(path) {
            addresses.insert(canonical, path.to_path_buf());
        }
    }
    addresses
}
impl Tracker {
    fn capture(&mut self, path: String) -> SourceRef {
        let mut source = SourceRef {
            path,
            token: None,
            error: None,
        };
        if !cfg!(any(target_os = "macos", target_os = "linux", windows))
            || Path::new(&source.path).is_dir()
            || is_playlist(Path::new(&source.path))
        {
            return source;
        }
        match TrackedFile::open(Path::new(&source.path)) {
            Ok(file) => {
                let token = uuid::Uuid::new_v4().to_string();
                self.files.insert(token.clone(), file);
                source.token = Some(token);
            }
            Err(error) => source.error = Some(error.to_string()),
        }
        source
    }
    fn prepare(&mut self, paths: Vec<String>) -> Prepared {
        let sources: Vec<_> = paths.into_iter().map(|path| self.capture(path)).collect();
        let acquired = sources
            .iter()
            .filter_map(|source| source.token.clone())
            .collect();
        Prepared {
            key: uuid::Uuid::new_v4().to_string(),
            sources,
            acquired,
        }
    }
    fn release(&mut self, tokens: &[String]) {
        for token in tokens {
            self.files.remove(token);
        }
    }
    fn resolve_one(&mut self, mut source: SourceRef) -> Result<(SourceRef, bool), String> {
        if let Some(token) = &source.token {
            let file = self
                .files
                .get_mut(token)
                .ok_or("Source tracking is unavailable for this captured file; reopen it")?;
            match file.resolve(Path::new(&source.path)) {
                Ok(path) => {
                    source.path = path
                        .to_str()
                        .ok_or("The moved source name cannot be represented by this workspace")?
                        .to_owned()
                }
                // Keep the old token alive until the host accepts this missing
                // source result or cancels it. A stale Refresh cannot close it.
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound
                        && links(&file.file).ok() == Some(0) =>
                {
                    source.token = None;
                    source.error = None;
                }
                Err(error) => return Err(format!("{}: {error}", source.path)),
            }
            return Ok((source, false));
        }
        if !Path::new(&source.path).exists() {
            if let Some(error) = &source.error {
                return Err(format!(
                    "{}: Cannot follow an external rename: {error}",
                    source.path
                ));
            }
            return Ok((source, false));
        }
        let source = self.capture(source.path);
        let acquired = source.token.is_some();
        Ok((source, acquired))
    }
    fn resolve(&mut self, sources: Vec<SourceRef>) -> Result<Resolution, String> {
        let mut result = Resolution {
            key: uuid::Uuid::new_v4().to_string(),
            paths: Vec::with_capacity(sources.len()),
            renamed: Vec::new(),
            sources: Vec::with_capacity(sources.len()),
            acquired: Vec::new(),
        };
        for original in sources {
            let from = original.path.clone();
            let (source, acquired) = match self.resolve_one(original) {
                Ok(source) => source,
                Err(error) => {
                    self.release(&result.acquired);
                    return Err(error);
                }
            };
            if acquired {
                result.acquired.extend(source.token.clone());
            }
            if from != source.path {
                result.renamed.push(PathChange {
                    from,
                    to: source.path.clone(),
                });
            }
            result.paths.push(source.path.clone());
            result.sources.push(source);
        }
        Ok(result)
    }
    fn before_change(&self, paths: Option<&[String]>) -> MutationEpoch {
        let attempted = paths.map(operation_addresses);
        let mut epoch = MutationEpoch::default();
        for (token, file) in &self.files {
            let path = match &attempted {
                Some(paths) => paths
                    .get(&file.address)
                    .or_else(|| paths.get(&file.requested))
                    .cloned(),
                None => Some(file.address.clone()),
            };
            let Some(path) = path else {
                continue;
            };
            let Ok(count) = links(&file.file) else {
                continue;
            };
            if open_regular(&path)
                .and_then(|file| identity(&file))
                .ok()
                .as_ref()
                == Some(&file.identity)
            {
                epoch
                    .0
                    .insert((token.clone(), file.identity.clone()), count);
            }
        }
        epoch
    }
    fn changed(
        &mut self,
        paths: &[String],
        renamed: &[(String, String)],
        directories: &[(String, String)],
        epoch: &MutationEpoch,
        completed: &[String],
    ) {
        self.update(paths, renamed, directories, epoch, completed, false);
    }
    fn update(
        &mut self,
        paths: &[String],
        renamed: &[(String, String)],
        directories: &[(String, String)],
        epoch: &MutationEpoch,
        completed: &[String],
        replay: bool,
    ) {
        let attempted = operation_addresses(paths);
        let completed = operation_addresses(completed);
        let renamed: HashMap<_, _> = renamed
            .iter()
            .map(|(from, to)| (PathBuf::from(from), PathBuf::from(to)))
            .collect();
        for (token, file) in &mut self.files {
            let old = file.address.clone();
            let mut current = old.clone();
            for (from, to) in directories {
                if let Ok(tail) = current.strip_prefix(from) {
                    current = Path::new(to).join(tail);
                }
            }
            if let Some(to) = renamed.get(&old).or_else(|| renamed.get(&file.requested)) {
                current = to.clone();
            }
            if current == old
                && !attempted.contains_key(&old)
                && !attempted.contains_key(&file.requested)
            {
                continue;
            }
            if let Ok(mut rebound) = TrackedFile::open(&current) {
                if rebound.identity != file.identity {
                    let after = links(&file.file).ok();
                    let successful_replacement = epoch
                        .0
                        .get(&(token.clone(), file.identity.clone()))
                        .is_some_and(|before| {
                            replay
                                || after.is_some_and(|after| after.checked_add(1) == Some(*before))
                        })
                        && completed.contains_key(&current);
                    if after != Some(0) && !successful_replacement {
                        continue;
                    }
                }
                rebound.canonical = file.canonical.clone();
                rebound.requested = file.requested.clone();
                *file = rebound;
            }
        }
    }
}
fn store() -> &'static Mutex<Tracker> {
    static TRACKER: OnceLock<Mutex<Tracker>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(Tracker::default()))
}
fn lock() -> std::sync::MutexGuard<'static, Tracker> {
    store().lock().unwrap_or_else(|error| error.into_inner())
}
#[tauri::command]
pub async fn begin_source_tracking() -> Result<String, String> {
    crate::io(|| {
        let session = uuid::Uuid::new_v4().to_string();
        *lock() = Tracker {
            session: session.clone(),
            ..Tracker::default()
        };
        Ok(session)
    })
    .await
}
#[tauri::command]
pub async fn prepare_source_roots(session: String, paths: Vec<String>) -> Result<Prepared, String> {
    crate::io(move || {
        crate::reject_provider_operation(&paths, "Source tracking")?;
        let mut tracker = lock();
        if tracker.session != session {
            return Err("Source tracking session changed".into());
        }
        Ok(tracker.prepare(paths))
    })
    .await
}
#[tauri::command]
pub async fn resolve_source_roots(
    session: String,
    sources: Vec<SourceRef>,
) -> Result<Resolution, String> {
    crate::io(move || {
        crate::reject_provider_operation(
            &sources
                .iter()
                .map(|source| source.path.clone())
                .collect::<Vec<_>>(),
            "Source tracking",
        )?;
        let mut tracker = lock();
        if tracker.session != session {
            return Err("Source tracking session changed".into());
        }
        tracker.resolve(sources)
    })
    .await
}
#[tauri::command]
pub async fn release_source_roots(session: String, tokens: Vec<String>) -> Result<(), String> {
    crate::io(move || {
        let mut tracker = lock();
        if tracker.session == session {
            tracker.release(&tokens);
        }
        Ok(())
    })
    .await
}
pub fn before_change(paths: Option<&[String]>) -> MutationEpoch {
    lock().before_change(paths)
}
pub fn changed(
    paths: &[String],
    renamed: &[(String, String)],
    directories: &[(String, String)],
    epoch: &MutationEpoch,
    completed: &[String],
) {
    lock().changed(paths, renamed, directories, epoch, completed);
}

/// Successful journal replay may retain the former live object as a snapshot.
pub fn replayed(
    paths: &[String],
    renamed: &[(String, String)],
    directories: &[(String, String)],
    epoch: &MutationEpoch,
    completed: &[String],
) {
    lock().update(paths, renamed, directories, epoch, completed, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rebinding_requires_the_verified_preoperation_object_and_successful_path() {
        for (verified_before, completed) in [(true, true), (false, true), (true, false)] {
            let (temp, old) = fixture();
            let mut tracker = Tracker::default();
            let prepared = tracker.prepare(vec![text(&old)]);
            let paths = vec![text(&old)];
            let epoch = if verified_before {
                tracker.before_change(None)
            } else {
                MutationEpoch::default()
            };
            // History retains the old live object rather than unlinking it.
            std::fs::rename(&old, temp.path().join("snapshot")).unwrap();
            std::fs::write(&old, b"replayed").unwrap();
            tracker.update(
                &paths,
                &[],
                &[],
                &epoch,
                if completed { &paths } else { &[] },
                true,
            );
            if verified_before && completed {
                let later = temp.path().join("later.wav");
                std::fs::rename(&old, &later).unwrap();
                assert_eq!(
                    tracker.resolve(prepared.sources).unwrap().paths,
                    vec![text(&dunce::canonicalize(later).unwrap())]
                );
            } else {
                assert!(tracker.resolve(prepared.sources).is_err());
            }
        }
    }
    #[test]
    fn followed_external_moves_keep_save_and_second_move_bound_to_the_saved_object() {
        for hard_linked in [false, true] {
            let (temp, old) = fixture();
            let first = temp.path().join("first.wav");
            let second = temp.path().join("second.wav");
            let mut tracker = Tracker::default();
            let prepared = tracker.prepare(vec![text(&old)]);
            std::fs::rename(&old, &first).unwrap();
            let resolved = tracker.resolve(prepared.sources).unwrap();
            if hard_linked {
                std::fs::hard_link(&first, temp.path().join("other.wav")).unwrap();
            }
            let paths = resolved.paths.clone();
            let epoch = tracker.before_change(Some(&paths));
            let replacement = temp.path().join("replacement");
            std::fs::write(&replacement, b"saved").unwrap();
            std::fs::rename(replacement, &first).unwrap();
            tracker.changed(&paths, &[], &[], &epoch, &paths);
            std::fs::rename(&first, &second).unwrap();
            assert_eq!(
                tracker.resolve(resolved.sources).unwrap().paths,
                vec![text(&dunce::canonicalize(&second).unwrap())],
                "hard_linked={hard_linked}"
            );
            assert_eq!(std::fs::read(second).unwrap(), b"saved");
            if hard_linked {
                assert_eq!(
                    std::fs::read(temp.path().join("other.wav")).unwrap(),
                    b"original"
                );
            }
        }
    }
    fn text(path: &Path) -> String {
        path.to_str().unwrap().to_owned()
    }
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let path = dunce::canonicalize(temp.path()).unwrap().join("old.wav");
        std::fs::write(&path, b"original").unwrap();
        (temp, path)
    }
    #[test]
    fn captured_objects_follow_same_folder_cross_folder_and_second_moves_before_ui_acceptance() {
        for cross in [false, true] {
            let (temp, old) = fixture();
            let parent = if cross {
                let p = temp.path().join("elsewhere");
                std::fs::create_dir(&p).unwrap();
                p
            } else {
                temp.path().to_path_buf()
            };
            let first = parent.join("first.wav");
            let second = temp.path().join("second.wav");
            let mut tracker = Tracker::default();
            let prepared = tracker.prepare(vec![text(&old)]);
            let token = prepared.sources[0].token.clone();
            std::fs::rename(&old, &first).unwrap();
            let mut resolved = tracker.resolve(prepared.sources).unwrap();
            assert_eq!(
                resolved.paths,
                vec![text(&dunce::canonicalize(&first).unwrap())]
            );
            assert_eq!(resolved.sources[0].token, token);
            std::fs::rename(&first, &second).unwrap();
            resolved = tracker.resolve(resolved.sources).unwrap();
            assert_eq!(
                resolved.paths,
                vec![text(&dunce::canonicalize(second).unwrap())]
            );
            assert_eq!(resolved.sources[0].token, token);
            assert_eq!(tracker.files.len(), 1);
        }
    }
    #[test]
    fn cancelled_preparations_release_only_their_own_objects_and_missing_tokens_never_reopen_names()
    {
        let (_temp, old) = fixture();
        let mut tracker = Tracker::default();
        let active = tracker.prepare(vec![text(&old)]);
        let cancelled = tracker.prepare(vec![text(&old)]);
        tracker.release(&cancelled.acquired);
        assert_eq!(tracker.files.len(), 1);
        assert!(tracker.resolve(active.sources).is_ok());
        assert!(tracker.resolve(cancelled.sources).is_err());
    }
    #[test]
    fn failed_operations_and_reused_names_cannot_adopt_another_object() {
        let (temp, old) = fixture();
        let moved = temp.path().join("moved.wav");
        let mut tracker = Tracker::default();
        let prepared = tracker.prepare(vec![text(&old)]);
        std::fs::rename(&old, &moved).unwrap();
        std::fs::write(&old, b"replacement").unwrap();
        let paths = vec![text(&old)];
        let epoch = tracker.before_change(Some(&paths));
        tracker.changed(&paths, &[], &[], &epoch, &[]);
        assert!(tracker
            .resolve(prepared.sources)
            .err()
            .unwrap()
            .contains("Another file"));
        assert_eq!(std::fs::read(&old).unwrap(), b"replacement");
        assert_eq!(std::fs::read(moved).unwrap(), b"original");
    }
    #[test]
    fn moving_back_to_original_name_does_not_adopt_a_reused_intermediate_name() {
        let (temp, old) = fixture();
        let intermediate = temp.path().join("intermediate.wav");
        let mut tracker = Tracker::default();
        let prepared = tracker.prepare(vec![text(&old)]);
        std::fs::rename(&old, &intermediate).unwrap();
        let resolved = tracker.resolve(prepared.sources).unwrap();
        std::fs::rename(&intermediate, &old).unwrap();
        std::fs::write(&intermediate, b"unrelated").unwrap();
        assert!(tracker.resolve(resolved.sources).is_err());
        assert_eq!(std::fs::read(old).unwrap(), b"original");
        assert_eq!(std::fs::read(intermediate).unwrap(), b"unrelated");
    }
    #[test]
    fn all_captures_rebind_successful_atomic_saves_including_before_ui_acceptance_and_hard_links() {
        for hard_linked in [false, true] {
            let (temp, old) = fixture();
            let other = temp.path().join("other.wav");
            if hard_linked {
                std::fs::hard_link(&old, &other).unwrap();
            }
            let mut tracker = Tracker::default();
            let prepared = tracker.prepare(vec![text(&old)]);
            let paths = vec![text(&old)];
            let epoch = tracker.before_change(Some(&paths));
            let replacement = temp.path().join("replacement");
            std::fs::write(&replacement, b"saved").unwrap();
            std::fs::rename(replacement, &old).unwrap();
            tracker.changed(&paths, &[], &[], &epoch, &paths);
            let moved = temp.path().join("moved.wav");
            std::fs::rename(&old, &moved).unwrap();
            let resolved = tracker.resolve(prepared.sources).unwrap();
            assert_eq!(
                resolved.paths,
                vec![text(&dunce::canonicalize(&moved).unwrap())]
            );
            assert_eq!(std::fs::read(moved).unwrap(), b"saved");
            if hard_linked {
                assert_eq!(std::fs::read(other).unwrap(), b"original");
            }
        }
    }
    #[test]
    fn known_rename_then_atomic_save_then_external_move_keeps_the_same_token() {
        let (temp, old) = fixture();
        let first = temp.path().join("first.wav");
        let final_path = temp.path().join("final.wav");
        let mut tracker = Tracker::default();
        let mut prepared = tracker.prepare(vec![text(&old)]);
        std::fs::rename(&old, &first).unwrap();
        tracker.changed(
            &[text(&old)],
            &[(text(&old), text(&first))],
            &[],
            &MutationEpoch::default(),
            &[text(&first)],
        );
        prepared.sources[0].path = text(&first);
        let epoch = tracker.before_change(Some(&[text(&first)]));
        let replacement = temp.path().join("replacement");
        std::fs::write(&replacement, b"saved").unwrap();
        std::fs::rename(replacement, &first).unwrap();
        tracker.changed(&[text(&first)], &[], &[], &epoch, &[text(&first)]);
        std::fs::rename(first, &final_path).unwrap();
        assert_eq!(
            tracker.resolve(prepared.sources).unwrap().paths,
            vec![text(&dunce::canonicalize(final_path).unwrap())]
        );
    }
    #[test]
    fn external_atomic_replacement_rebinds_without_losing_a_later_move() {
        let (temp, old) = fixture();
        let mut tracker = Tracker::default();
        let prepared = tracker.prepare(vec![text(&old)]);
        let replacement = temp.path().join("replacement");
        std::fs::write(&replacement, b"outside saved").unwrap();
        std::fs::rename(replacement, &old).unwrap();
        let resolved = tracker.resolve(prepared.sources).unwrap();
        let moved = temp.path().join("moved.wav");
        std::fs::rename(&old, &moved).unwrap();
        assert_eq!(
            tracker.resolve(resolved.sources).unwrap().paths,
            vec![text(&dunce::canonicalize(moved).unwrap())]
        );
    }
    #[test]
    fn unlinked_source_tokens_remain_valid_until_the_host_accepts_missing_membership() {
        let (_temp, old) = fixture();
        let mut tracker = Tracker::default();
        let prepared = tracker.prepare(vec![text(&old)]);
        std::fs::remove_file(old).unwrap();
        let resolved = tracker.resolve(prepared.sources.clone()).unwrap();
        assert!(resolved.sources[0].token.is_none());
        assert_eq!(tracker.files.len(), 1);
        assert!(tracker.resolve(prepared.sources).is_ok());
        tracker.release(&prepared.acquired);
        assert!(tracker.files.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn aliases_preserve_logical_paths_follow_parent_moves_and_refuse_retargeting() {
        let (temp, old) = fixture();
        let alias = temp.path().join("alias.wav");
        std::os::unix::fs::symlink(&old, &alias).unwrap();
        let mut tracker = Tracker::default();
        let prepared = tracker.prepare(vec![text(&alias)]);
        assert_eq!(
            tracker.resolve(prepared.sources.clone()).unwrap().paths,
            vec![text(&alias)]
        );
        let moved = temp.path().join("moved.wav");
        std::fs::rename(&old, &moved).unwrap();
        assert_eq!(
            tracker.resolve(prepared.sources.clone()).unwrap().paths,
            vec![text(&dunce::canonicalize(&moved).unwrap())]
        );
        std::fs::remove_file(&alias).unwrap();
        let other = temp.path().join("other.wav");
        std::fs::write(&other, b"other").unwrap();
        std::os::unix::fs::symlink(other, &alias).unwrap();
        assert!(tracker.resolve(prepared.sources).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn fifo_refusal_does_not_wait_for_a_writer() {
        use std::os::unix::ffi::OsStrExt;
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pipe.wav");
        let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: owned temporary path, terminated CString and owner-only permissions.
        assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "source_tracking::tests::fifo_worker",
                "--ignored",
            ])
            .env("TAGGROOVE_OWNED_FIFO", path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Source tracking waited for a FIFO writer");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[cfg(unix)]
    #[test]
    #[ignore = "Owned FIFO refusal worker"]
    fn fifo_worker() {
        let path = std::env::var_os("TAGGROOVE_OWNED_FIFO").unwrap();
        assert!(TrackedFile::open(Path::new(&path))
            .err()
            .unwrap()
            .to_string()
            .contains("not a regular file"));
    }
}
