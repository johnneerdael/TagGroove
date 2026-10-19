//! Bounded content/identity baselines shared by edit history and staged writes.
use crate::batch_errors::ErrorCause;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path, time::SystemTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MetadataStamp {
    pub(crate) len: u64,
    modified: Option<SystemTime>,
    #[cfg(not(windows))]
    identity: Option<(u64, u64)>,
    #[cfg(windows)]
    identity: Option<WindowsIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    pub(crate) len: u64,
    modified: Option<SystemTime>,
    #[cfg(not(windows))]
    identity: Option<(u64, u64)>,
    #[cfg(windows)]
    identity: Option<WindowsIdentity>,
    digest: [u8; 32],
}
impl Fingerprint {
    pub(crate) fn same_content(&self, other: &Self) -> bool {
        self.len == other.len && self.digest == other.digest
    }
    pub(crate) fn matches_metadata(&self, stamp: &MetadataStamp) -> bool {
        self.len == stamp.len && self.modified == stamp.modified && self.identity == stamp.identity
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowsIdentity {
    FileId128 { volume: u64, file: [u8; 16] },
    FileIndex64 { volume: u32, file: u64 },
}

#[cfg(windows)]
pub(crate) fn windows_legacy_identity(file: &fs::File) -> std::io::Result<WindowsIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the file owns a live handle, and info is the correctly sized writable buffer.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(WindowsIdentity::FileIndex64 {
        volume: info.dwVolumeSerialNumber,
        file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

#[cfg(windows)]
fn windows_identity(file: &fs::File) -> std::io::Result<WindowsIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdInfo, GetFileInformationByHandleEx, FILE_ID_INFO,
    };

    let mut info = FILE_ID_INFO::default();
    // Prefer the full ID: the legacy 64-bit index is not guaranteed unique on ReFS.
    // SAFETY: the file owns a live handle, and the buffer matches FileIdInfo's ABI.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            std::ptr::from_mut(&mut info).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    } != 0
    {
        Ok(WindowsIdentity::FileId128 {
            volume: info.VolumeSerialNumber,
            file: info.FileId.Identifier,
        })
    } else {
        // Older/network filesystems may expose only volume + file index. If neither
        // query works, refuse the fingerprint rather than trusting size/mtime alone.
        windows_legacy_identity(file)
    }
}

fn stamp(
    file: &fs::File,
    metadata: &fs::Metadata,
    require_identity: bool,
) -> Result<MetadataStamp, ErrorCause> {
    if !metadata.is_file() {
        return Err(ErrorCause::owned("Not a regular file"));
    }
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    };
    #[cfg(windows)]
    let identity = match windows_identity(file) {
        Ok(identity) => Some(identity),
        Err(error) if require_identity => return Err(ErrorCause::external(error)),
        Err(_) => None,
    };
    #[cfg(not(windows))]
    let _ = (file, require_identity);
    #[cfg(not(any(unix, windows)))]
    let identity = None;
    Ok(MetadataStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        identity,
    })
}

fn change_time(file: &fs::File, metadata: &fs::Metadata) -> Option<(i64, i64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = file;
        Some((metadata.ctime(), metadata.ctime_nsec()))
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FileBasicInfo, GetFileInformationByHandleEx, FILE_BASIC_INFO,
        };
        let _ = metadata;
        let mut info = FILE_BASIC_INFO::default();
        // SAFETY: the handle is live and the output buffer has FILE_BASIC_INFO's exact ABI.
        let ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileBasicInfo,
                std::ptr::from_mut(&mut info).cast(),
                std::mem::size_of::<FILE_BASIC_INFO>() as u32,
            )
        };
        (ok != 0).then_some((info.ChangeTime, 0))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (file, metadata);
        None
    }
}

/// Metadata-only admission: do not read audio that cannot enter the journal.
pub(crate) fn metadata_stamp(path: &Path) -> Result<MetadataStamp, ErrorCause> {
    metadata_stamp_for(path, true)
}
fn metadata_stamp_for(path: &Path, require_identity: bool) -> Result<MetadataStamp, ErrorCause> {
    let leaf = fs::symlink_metadata(path).map_err(ErrorCause::external)?;
    if !leaf.is_file() {
        return Err(ErrorCause::owned("Not a regular file"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_READ_ATTRIBUTES;
        let file = fs::File::options()
            .access_mode(FILE_READ_ATTRIBUTES)
            .open(path)
            .map_err(ErrorCause::external)?;
        let metadata = file.metadata().map_err(ErrorCause::external)?;
        stamp(&file, &metadata, require_identity)
    }
    #[cfg(not(windows))]
    {
        let _ = require_identity;
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            Some((leaf.dev(), leaf.ino()))
        };
        #[cfg(not(unix))]
        let identity = None;
        Ok(MetadataStamp {
            len: leaf.len(),
            modified: leaf.modified().ok(),
            identity,
        })
    }
}

fn changed() -> ErrorCause {
    ErrorCause::owned("The file changed while its content was being checked")
}

/// Hash exactly one bounded file image, checking both the open handle and leaf.
/// This detects observed changes; the final check and rename are not a filesystem CAS.
pub(crate) fn fingerprint_diagnostic(path: &Path) -> Result<Fingerprint, ErrorCause> {
    fingerprint_for(path, true)
}
/// Keep ordinary Save available when a Windows filesystem cannot expose a file
/// ID. Its full content baseline still applies; strict history capture refuses
/// that filesystem, retaining the existing unavailable-Undo notice policy.
pub(crate) fn fingerprint_for_edit_diagnostic(path: &Path) -> Result<Fingerprint, ErrorCause> {
    fingerprint_for(path, false)
}
fn fingerprint_for(path: &Path, require_identity: bool) -> Result<Fingerprint, ErrorCause> {
    let leaf = metadata_stamp_for(path, require_identity)?;
    let mut file = fs::File::open(path).map_err(ErrorCause::external)?;
    let metadata = file.metadata().map_err(ErrorCause::external)?;
    let before = stamp(&file, &metadata, require_identity)?;
    if before != leaf {
        return Err(changed());
    }
    let change = change_time(&file, &metadata);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut remaining = before.len;
    while remaining != 0 {
        let length = remaining.min(buffer.len() as u64) as usize;
        let n = match file.read(&mut buffer[..length]) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            value => value.map_err(ErrorCause::external)?,
        };
        if n == 0 {
            return Err(changed());
        }
        hasher.update(&buffer[..n]);
        remaining -= n as u64;
    }
    let mut extra = [0u8; 1];
    loop {
        match file.read(&mut extra) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            value => {
                if value.map_err(ErrorCause::external)? != 0 {
                    return Err(changed());
                }
                break;
            }
        }
    }
    let after = file.metadata().map_err(ErrorCause::external)?;
    if stamp(&file, &after, require_identity)? != before
        || change_time(&file, &after) != change
        || metadata_stamp_for(path, require_identity)? != before
    {
        return Err(changed());
    }
    Ok(Fingerprint {
        len: before.len,
        modified: before.modified,
        identity: before.identity,
        digest: hasher.finalize().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_reader_matches_fixed_sha256_vectors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vector.bin");
        for (input, expected) in [
            (
                b"".as_slice(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                b"abc".as_slice(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
        ] {
            fs::write(&path, input).unwrap();
            let fingerprint = fingerprint_diagnostic(&path).unwrap();
            let actual: String = fingerprint
                .digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(actual, expected);
            assert_eq!(fingerprint.len, input.len() as u64);
        }
    }

    #[test]
    #[ignore = "Manual warm-cache streaming hash benchmark; no timing assertion"]
    fn benchmark_streaming_content_baseline() {
        use std::io::Write;
        for size in [10 * 1024 * 1024, 64 * 1024 * 1024] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("hash-input.bin");
            let mut file = fs::File::create(&path).unwrap();
            let chunk = [0x5au8; 64 * 1024];
            for _ in 0..size / chunk.len() {
                file.write_all(&chunk).unwrap();
            }
            file.sync_all().unwrap();
            drop(file);
            let start = std::time::Instant::now();
            let result = fingerprint_diagnostic(&path).unwrap();
            let elapsed = start.elapsed();
            assert_eq!(result.len, size as u64);
            assert_eq!(fingerprint_diagnostic(&path).unwrap(), result);
            eprintln!(
                "content_baseline bytes={size} elapsed_ns={} retained_input={}",
                elapsed.as_nanos(),
                dir.keep().display()
            );
        }
    }
}
