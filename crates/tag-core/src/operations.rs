use std::path::{Path, PathBuf};

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
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| format!("Cannot create {}: {e}", destination.display()))?;
    let result = (|| {
        let copied = std::io::copy(source, &mut output).map_err(|e| e.to_string())?;
        if copied != metadata.len()
            || !same_source(metadata, &source.metadata().map_err(|e| e.to_string())?)
        {
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
        return Err(format!(
            "{error}; destination rollback: {:?}",
            remove_copy(destination)
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
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    std::fs::remove_file(path)
}

pub fn rename_no_clobber(source: &Path, destination: &Path) -> Result<(), String> {
    if source == destination {
        return Ok(());
    }
    if !std::fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("Only regular files can be renamed".into());
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
            return Err(format!("Cannot create {}: {e}", destination.display()))
        }
        // Cross-volume moves and filesystems without hard links use an exclusive
        // copy. Do not delete the source until the complete copy is durable.
        Err(_) => {
            copy_new(&mut input, destination, &metadata)?;
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
        return Err(format!("Source changed before move completed; original was retained; destination rollback: {cleanup:?}"));
    }
    if let Err(e) = std::fs::remove_file(source) {
        let cleanup = if copied {
            remove_copy(destination)
        } else {
            std::fs::remove_file(destination)
        };
        return Err(format!(
            "Cannot remove original: {e}; destination rollback: {cleanup:?}"
        ));
    }
    Ok(())
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
        assert!(rename_no_clobber(&src, &dst).is_err());
        assert_eq!(std::fs::read(src).unwrap(), b"first");
        assert_eq!(std::fs::read(dst).unwrap(), b"second");
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
