//! Explicit copy-on-write file cloning.
//!
//! Staged writes start from a copy of the original. On APFS (`clonefile`) and
//! on Linux filesystems with reflinks (`FICLONE`: Btrfs, XFS, bcachefs) that
//! copy is a clone sharing every block with the original until it is
//! rewritten. Reporting whether a copy really is a clone lets the undo journal
//! count only the blocks a write replaced. Everywhere else, including Windows,
//! the copy is a plain full copy and is reported as such.
use std::{
    collections::HashMap,
    fs, io,
    path::Path,
    sync::{Mutex, OnceLock},
};

/// Create `destination`, which must not exist, with the contents of `source`.
/// Returns `true` when it is a copy-on-write clone, `false` for a full copy.
pub fn clone_or_copy(source: &Path, destination: &Path) -> io::Result<bool> {
    if clone(source, destination)? {
        return Ok(true);
    }
    fs::copy(source, destination)?;
    Ok(false)
}

#[cfg(target_vendor = "apple")]
fn clone(source: &Path, destination: &Path) -> io::Result<bool> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    // Ownership follows the creating user like any other new file.
    const CLONE_NOOWNERCOPY: u32 = 0x0002;
    let source = CString::new(source.as_os_str().as_bytes())?;
    let target = CString::new(destination.as_os_str().as_bytes())?;
    // SAFETY: both arguments are valid NUL-terminated paths for the call.
    if unsafe { libc::clonefile(source.as_ptr(), target.as_ptr(), CLONE_NOOWNERCOPY) } == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    match error.raw_os_error() {
        // Unsupported by this volume or across volumes: fall back to a copy.
        Some(libc::ENOTSUP | libc::EXDEV | libc::EPERM | libc::EINVAL) => Ok(false),
        _ => Err(error),
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn clone(source: &Path, destination: &Path) -> io::Result<bool> {
    use std::os::fd::AsRawFd;
    let input = fs::File::open(source)?;
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    // SAFETY: both descriptors are open for the duration of the call.
    let cloned = unsafe { libc::ioctl(output.as_raw_fd(), libc::FICLONE, input.as_raw_fd()) } == 0;
    if cloned {
        output.set_permissions(input.metadata()?.permissions())?;
        return Ok(true);
    }
    drop(output);
    fs::remove_file(destination)?;
    Ok(false)
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
fn clone(_: &Path, _: &Path) -> io::Result<bool> {
    Ok(false)
}

/// Whether files in `dir` can be cloned, probed once per volume with a small
/// temporary file that is removed again.
pub fn supports_clone(dir: &Path) -> bool {
    static PROBED: OnceLock<Mutex<HashMap<u64, bool>>> = OnceLock::new();
    let Some(volume) = volume_id(dir) else {
        return false;
    };
    let cache = PROBED.get_or_init(Default::default);
    if let Some(known) = cache.lock().ok().and_then(|c| c.get(&volume).copied()) {
        return known;
    }
    let probe = (|| {
        let source = tempfile::Builder::new()
            .prefix(".opentag-clone-probe-")
            .tempfile_in(dir)
            .ok()?;
        std::io::Write::write_all(&mut source.as_file(), b"OpenTag clone probe").ok()?;
        let target = source.path().with_extension("clone");
        let cloned = clone(source.path(), &target).unwrap_or(false);
        let _ = fs::remove_file(&target);
        Some(cloned)
    })()
    .unwrap_or(false);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(volume, probe);
    }
    probe
}

/// The device a path lives on, where the platform exposes it.
pub fn volume_id(path: &Path) -> Option<u64> {
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
pub fn available_space(path: &Path) -> Option<u64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_or_copy_reproduces_bytes_and_refuses_existing_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.bin");
        fs::write(&source, vec![7u8; 300_000]).unwrap();
        let target = dir.path().join("target.bin");
        let cloned = clone_or_copy(&source, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), fs::read(&source).unwrap());
        assert_eq!(cloned, supports_clone(dir.path()));
        // Writing the copy never changes the source.
        fs::write(&target, b"changed").unwrap();
        assert_eq!(fs::read(&source).unwrap().len(), 300_000);
        assert!(clone_or_copy(&source, &target).is_err());
        assert!(available_space(dir.path()).is_some_and(|free| free > 0) || cfg!(not(unix)));
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn apfs_temporary_directories_clone() {
        // macOS temporary directories live on APFS.
        assert!(supports_clone(&std::env::temp_dir()));
    }
}
