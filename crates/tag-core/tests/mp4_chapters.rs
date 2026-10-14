//! MP4 chapters written by ffmpeg (a Nero `chpl` box and a QuickTime chapter
//! track): read as `_CHAPTERS`, and removed by Utils → Strip chapters MP4
//! without moving or changing any media byte. Skipped without ffmpeg.
use std::{fs, path::Path, process::Command};
use tag_core::{read_entry, strip_mp4_chapters_with_options, write_fields, Fields, TagOptions};

fn tools() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok()
}

fn encode(dir: &Path, name: &str, codec: &str) -> std::path::PathBuf {
    let metadata = dir.join("chapters.txt");
    fs::write(
        &metadata,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=400\ntitle=One\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=400\nEND=1000\ntitle=Two – ü\n",
    )
    .unwrap();
    let path = dir.join(name);
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=1", "-i"])
        .arg(&metadata)
        .args(["-map", "0", "-map_chapters", "1", "-c:a", codec])
        .args(["-metadata", "title=Song"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
}

fn probe(path: &Path, entries: &str, extra: &[&str]) -> String {
    let output = Command::new("ffprobe")
        .args(["-v", "error"])
        .args(extra)
        .args(["-show_entries", entries, "-of", "csv=p=0"])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}
fn packets(path: &Path) -> String {
    probe(
        path,
        "packet=pts,size",
        &["-select_streams", "a", "-show_data_hash", "MD5"],
    ) + &probe(
        path,
        "packet=data_hash",
        &["-select_streams", "a", "-show_data_hash", "MD5"],
    )
}

#[test]
fn nero_and_quicktime_chapters_are_read_and_stripped() {
    if !tools() {
        eprintln!("Skipping MP4 chapter test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, codec) in [("a.mp4", "aac"), ("b.m4b", "aac"), ("c.m4a", "alac")] {
        let path = encode(dir.path(), name, codec);
        let entry = read_entry(&path).unwrap();
        assert_eq!(
            entry.technical["_CHAPTERS"],
            ["00:00:00.000 One", "00:00:00.400 Two – ü"],
            "{name}"
        );
        assert_eq!(entry.technical["_SUBSONG_COUNT"], ["2"], "{name}");
        assert!(!entry.fields.contains_key("_CHAPTERS"));
        assert_eq!(
            entry.technical["_CODEC"],
            [if codec == "alac" { "ALAC" } else { "AAC" }]
        );

        // Without the Nero box the QuickTime chapter track is read.
        let mut bytes = fs::read(&path).unwrap();
        let chpl = bytes.windows(4).position(|w| w == b"chpl").unwrap();
        bytes[chpl..chpl + 4].copy_from_slice(b"free");
        let track_only = dir.path().join(format!("track-{name}"));
        fs::write(&track_only, &bytes).unwrap();
        let entry = read_entry(&track_only).unwrap();
        assert_eq!(
            entry.technical["_CHAPTERS"],
            ["00:00:00.000 One", "00:00:00.400 Two – ü"],
            "{name}"
        );

        let before = fs::read(&path).unwrap();
        let audio = packets(&path);
        let stripped =
            strip_mp4_chapters_with_options(&path, false, &TagOptions::default()).unwrap();
        assert!(!stripped.technical.contains_key("_CHAPTERS"), "{name}");
        assert!(!stripped.technical.contains_key("_SUBSONG_COUNT"));
        assert_eq!(stripped.fields["TITLE"], ["Song"]);
        let after = fs::read(&path).unwrap();
        assert_eq!(after.len(), before.len(), "{name}: nothing moves");
        assert_eq!(probe(&path, "chapter=start_time", &[]), "", "{name}");
        assert_eq!(packets(&path), audio, "{name}");
        // Tags can still be written afterwards.
        let written = write_fields(&path, &Fields::from([("ALBUM".into(), vec!["X".into()])]));
        assert_eq!(written.unwrap().fields["ALBUM"], ["X"]);
        assert_eq!(packets(&path), audio, "{name}");

        // A second strip, and files without chapters, change nothing.
        let unchanged = fs::read(&path).unwrap();
        strip_mp4_chapters_with_options(&path, false, &TagOptions::default()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), unchanged);
    }
    let flac = dir.path().join("plain.flac");
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=0.2"])
        .arg(&flac)
        .output()
        .unwrap();
    assert!(output.status.success());
    let before = fs::read(&flac).unwrap();
    strip_mp4_chapters_with_options(&flac, false, &TagOptions::default()).unwrap();
    assert_eq!(fs::read(&flac).unwrap(), before);
}
