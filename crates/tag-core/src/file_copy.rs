//! Exclusive copies with content/identity receipts for caller-owned rollback.
//! No association policy, history, or source-file mutation belongs here.
use crate::file_baseline::{
    fingerprint_diagnostic, metadata_stamp, metadata_stamp_for_handle, Fingerprint,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Identifies a destination folder across preview and publication. Unlike a
/// directory timestamp, identity remains valid after our own files are added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryIdentity {
    resolved: PathBuf,
    #[cfg(unix)]
    identity: (u64, u64),
    #[cfg(windows)]
    identity: crate::file_baseline::WindowsIdentity,
}
impl DirectoryIdentity {
    pub fn read(path: &Path) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !metadata.is_dir() {
            return Err("The reviewed destination folder is not a regular directory".into());
        }
        let resolved = dunce::canonicalize(path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino())
        };
        #[cfg(windows)]
        let identity = {
            use std::os::windows::fs::OpenOptionsExt;
            let directory = fs::File::options()
                .read(true)
                .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
                .open(path)
                .map_err(|e| e.to_string())?;
            crate::file_baseline::windows_identity(&directory).map_err(|e| e.to_string())?
        };
        #[cfg(not(any(unix, windows)))]
        return Err("Destination folder identity is unavailable on this platform".into());
        #[cfg(any(unix, windows))]
        Ok(Self { resolved, identity })
    }
}

#[derive(Debug)]
pub struct CopyReceipt {
    path: PathBuf,
    fingerprint: Fingerprint,
}
impl CopyReceipt {
    pub fn matches_digest(&self, digest: &[u8; 32]) -> bool {
        self.fingerprint.has_digest(digest)
    }
    /// Refuse observed edits, replacement, symlinks or missing destinations.
    /// The last check and unlink are not an adversarial filesystem CAS.
    pub fn rollback(&self) -> Result<(), String> {
        let current = fingerprint_diagnostic(&self.path).map_err(|e| e.to_string())?;
        if current != self.fingerprint {
            return Err(format!(
                "{} changed after copying; retained the destination",
                self.path.display()
            ));
        }
        #[cfg(windows)]
        {
            let metadata = fs::symlink_metadata(&self.path).map_err(|e| e.to_string())?;
            if metadata.permissions().readonly() {
                let mut permissions = metadata.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                permissions.set_readonly(false);
                fs::set_permissions(&self.path, permissions).map_err(|e| e.to_string())?;
            }
        }
        fs::remove_file(&self.path).map_err(|e| e.to_string())
    }
    pub fn destination(&self) -> &Path {
        &self.path
    }
}

/// Copy into an exclusively created leaf in an existing directory. Return a
/// receipt bound to the created handle and bytes actually written, not to a
/// later path lookup that could adopt an unrelated replacement.
pub fn copy_with_receipt(source: &Path, destination: &Path) -> Result<CopyReceipt, String> {
    let before = metadata_stamp(source).map_err(|e| e.to_string())?;
    let mut input = fs::File::open(source).map_err(|e| e.to_string())?;
    let metadata = input.metadata().map_err(|e| e.to_string())?;
    if metadata_stamp_for_handle(&input).map_err(|e| e.to_string())? != before {
        return Err("The source changed before copying".into());
    }
    let mut output = fs::File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| format!("Cannot create {}: {e}", destination.display()))?;
    let mut hasher = Sha256::new();
    let mut written = 0u64;
    let result = (|| {
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = match input.read(&mut buffer) {
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                result => result.map_err(|e| e.to_string())?,
            };
            if count == 0 {
                break;
            }
            let mut position = 0;
            while position < count {
                let count = match output.write(&buffer[position..count]) {
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    result => result.map_err(|e| e.to_string())?,
                };
                if count == 0 {
                    return Err("The destination stopped accepting copied bytes".into());
                }
                hasher.update(&buffer[position..position + count]);
                written += count as u64;
                position += count;
            }
        }
        if written != metadata.len()
            || metadata_stamp_for_handle(&input).map_err(|e| e.to_string())? != before
            || metadata_stamp(source).map_err(|e| e.to_string())? != before
        {
            return Err("Source changed while copying; original was retained".into());
        }
        let mut times = fs::FileTimes::new();
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
    let receipt = metadata_stamp_for_handle(&output)
        .map_err(|e| e.to_string())
        .and_then(|stamp| {
            if stamp.len != written {
                return Err("Copied destination changed before its receipt was recorded".into());
            }
            Ok(CopyReceipt {
                path: destination.into(),
                fingerprint: Fingerprint::from_stamp(stamp, hasher.finalize().into()),
            })
        });
    drop(output);
    match (result, receipt) {
        (Ok(()), Ok(receipt)) => Ok(receipt),
        (Err(error), Ok(receipt)) => Err(match receipt.rollback() {
            Ok(()) => format!("{error}; incomplete copy was removed"),
            Err(cleanup) => format!("{error}; copy cleanup incomplete: {cleanup}"),
        }),
        (result, Err(receipt)) => Err(format!(
            "{}; copy cleanup could not establish ownership: {receipt}; retained {}",
            result
                .err()
                .unwrap_or_else(|| "Copy could not be verified".into()),
            destination.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipt_removes_only_unchanged_created_copy_and_preserves_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.bin");
        let target = root.path().join("target.bin");
        let bytes = vec![17; 150_003];
        fs::write(&source, &bytes).unwrap();
        let before = fs::metadata(&source).unwrap();
        let receipt = copy_with_receipt(&source, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), bytes);
        assert!(receipt.matches_digest(&Sha256::digest(&bytes).into()));
        assert_eq!(
            fs::metadata(&target).unwrap().modified().unwrap(),
            before.modified().unwrap()
        );
        assert!(copy_with_receipt(&source, &target).is_err());
        receipt.rollback().unwrap();
        assert!(!target.exists());
        assert_eq!(fs::read(&source).unwrap(), bytes);
    }
    #[test]
    fn receipt_refuses_edits_and_equal_byte_replacements() {
        for replace in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("source");
            let target = root.path().join("target");
            fs::write(&source, b"unchanged").unwrap();
            let receipt = copy_with_receipt(&source, &target).unwrap();
            if replace {
                let prior = fs::metadata(&target).unwrap();
                fs::rename(&target, root.path().join("retained-created-copy")).unwrap();
                fs::write(&target, b"unchanged").unwrap();
                fs::File::options()
                    .write(true)
                    .open(&target)
                    .unwrap()
                    .set_times(fs::FileTimes::new().set_modified(prior.modified().unwrap()))
                    .unwrap();
            } else {
                fs::write(&target, b"outside change").unwrap();
            }
            assert!(receipt.rollback().is_err());
            assert!(target.exists());
            assert_eq!(fs::read(&source).unwrap(), b"unchanged");
        }
    }
    #[cfg(unix)]
    #[test]
    fn receipt_and_directory_identity_refuse_symlink_substitution() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        let outside = root.path().join("outside");
        fs::write(&source, b"source").unwrap();
        fs::write(&outside, b"outside").unwrap();
        let receipt = copy_with_receipt(&source, &target).unwrap();
        fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink(&outside, &target).unwrap();
        assert!(receipt.rollback().is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"outside");
        let directory = root.path().join("destination");
        fs::create_dir(&directory).unwrap();
        let identity = DirectoryIdentity::read(&directory).unwrap();
        fs::rename(&directory, root.path().join("original-directory")).unwrap();
        fs::create_dir(&directory).unwrap();
        assert_ne!(DirectoryIdentity::read(&directory).unwrap(), identity);
        fs::remove_dir(&directory).unwrap();
        std::os::unix::fs::symlink(root.path().join("original-directory"), &directory).unwrap();
        assert!(DirectoryIdentity::read(&directory).is_err());
    }
}
