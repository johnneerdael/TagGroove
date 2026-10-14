use std::{fs, path::Path, process::Command};
use tag_core::{add_cover, read_entry, write_fields, Fields};

fn fields(values: &[(&str, &str)]) -> Fields {
    values
        .iter()
        .map(|(key, value)| (key.to_string(), vec![value.to_string()]))
        .collect()
}

// ffmpeg's FLAC muxer reserves an 8 KiB PADDING block.
fn flac(path: &Path) -> bool {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("Skipping FLAC padding test: ffmpeg is required");
        return false;
    }
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i"])
        .arg("sine=frequency=440:sample_rate=8000:duration=0.5")
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

/// Byte offset where the audio frames start.
fn audio_start(bytes: &[u8]) -> usize {
    assert_eq!(&bytes[..4], b"fLaC");
    let mut at = 4;
    loop {
        let header = &bytes[at..at + 4];
        at += 4
            + (usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]));
        if header[0] & 0x80 != 0 {
            return at;
        }
    }
}

/// File size and a digest of the audio frames.
fn audio(path: &Path) -> (usize, u64) {
    use std::hash::{Hash, Hasher};
    let bytes = fs::read(path).unwrap();
    let start = audio_start(&bytes);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes[start..].hash(&mut hasher);
    (bytes.len(), hasher.finish())
}

fn blocks(path: &Path) -> Vec<(u8, usize)> {
    let bytes = fs::read(path).unwrap();
    let mut at = 4;
    let mut out = Vec::new();
    loop {
        let header = &bytes[at..at + 4];
        let len =
            usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]);
        out.push((header[0] & 0x7f, len));
        at += 4 + len;
        if header[0] & 0x80 != 0 {
            return out;
        }
    }
}

#[test]
fn tag_edits_reuse_padding_and_never_change_audio_frames() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("padded.flac");
    if !flac(&path) {
        return;
    }
    let cover = dir.path().join("cover.png");
    image::RgbImage::new(8, 8).save(&cover).unwrap();
    add_cover(&path, &cover).unwrap();
    let (size, frames) = audio(&path);

    // Growing and shrinking within the padding keeps the file size and audio offset.
    for title in ["A considerably longer title than before", "S"] {
        write_fields(&path, &fields(&[("TITLE", title), ("COMMENT", "kept")])).unwrap();
        assert_eq!(audio(&path), (size, frames), "{title}");
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.fields["TITLE"], [title]);
        assert_eq!(entry.covers.len(), 1);
        assert_eq!(
            blocks(&path).last().map(|b| b.0),
            Some(1),
            "padding stays last"
        );
    }

    // Exceeding the padding falls back to a full rewrite with the same audio.
    let long = "x".repeat(20_000);
    write_fields(&path, &fields(&[("COMMENT", &long)])).unwrap();
    let (grown, moved) = audio(&path);
    assert!(grown > size);
    assert_eq!(moved, frames);
    assert_eq!(read_entry(&path).unwrap().fields["COMMENT"], [long]);

    // A shorter value after the rewrite fits the enlarged region again.
    write_fields(&path, &fields(&[("COMMENT", "short")])).unwrap();
    assert_eq!(audio(&path), (grown, frames));
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["COMMENT"], ["short"]);
    assert_eq!(entry.fields["TITLE"], ["S"]);
    assert_eq!(entry.covers.len(), 1);
}

#[cfg(unix)]
#[test]
fn in_place_tag_edits_never_touch_a_hard_linked_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("linked.flac");
    if !flac(&path) {
        return;
    }
    // The undo journal snapshots files by hard-linking them before a write.
    let snapshot = dir.path().join("snapshot.flac");
    fs::hard_link(&path, &snapshot).unwrap();
    let before = fs::read(&snapshot).unwrap();
    let (size, frames) = audio(&path);
    write_fields(&path, &fields(&[("TITLE", "Linked")])).unwrap();
    // The fast path ran (same size and audio) on a new inode renamed over the path.
    assert_eq!(audio(&path), (size, frames));
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["Linked"]);
    assert_eq!(fs::read(&snapshot).unwrap(), before);
    use std::os::unix::fs::MetadataExt;
    assert_ne!(
        fs::metadata(&path).unwrap().ino(),
        fs::metadata(&snapshot).unwrap().ino()
    );
    assert_eq!(fs::metadata(&snapshot).unwrap().nlink(), 1);
    if cfg!(target_os = "macos") {
        assert!(
            tag_core::clones_in(dir.path()),
            "temporary directories on APFS clone"
        );
    }
}
