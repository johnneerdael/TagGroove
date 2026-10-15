//! ID3v2 and MP4 tag edits reuse padding and never move the audio data.
use std::{fs, path::Path, process::Command};
use tag_core::{read_entry, write_fields, write_fields_with_options, Fields, TagOptions};

fn fields(values: &[(&str, &str)]) -> Fields {
    values
        .iter()
        .map(|(key, value)| (key.to_string(), vec![value.to_string()]))
        .collect()
}

fn ffmpeg(args: &[&str], path: &Path) -> bool {
    ffmpeg_seconds(1, args, path)
}

fn ffmpeg_seconds(seconds: u32, args: &[&str], path: &Path) -> bool {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("Skipping in-place tag test: ffmpeg is required");
        return false;
    }
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i"])
        .arg(format!(
            "sine=frequency=440:sample_rate=44100:duration={seconds}"
        ))
        .args(args)
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

fn syncsafe(bytes: &[u8]) -> usize {
    bytes.iter().fold(0, |v, b| (v << 7) | usize::from(*b))
}

/// File size and the bytes after the ID3v2 tag (the MPEG stream).
fn mpeg(path: &Path) -> (usize, Vec<u8>) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..3], b"ID3");
    let end = 10 + syncsafe(&bytes[6..10]);
    (bytes.len(), bytes[end..bytes.len() - 128].to_vec())
}

fn decodes(path: &Path) {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .unwrap();
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn mp3_edits_reuse_id3v2_padding_and_rewrites_reserve_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("song.mp3");
    let args = [
        "-c:a",
        "libmp3lame",
        "-id3v2_version",
        "3",
        "-write_id3v1",
        "1",
        "-metadata",
        "title=Original",
    ];
    if !ffmpeg(&args, &path) {
        return;
    }
    // ffmpeg reserves little padding: the first edit that outgrows it
    // rewrites the file and reserves 4 KiB.
    let (size, stream) = mpeg(&path);
    let long = "A title much longer than the padding ffmpeg reserved ".repeat(4);
    write_fields(&path, &fields(&[("TITLE", &long)])).unwrap();
    let (grown, moved) = mpeg(&path);
    assert!(grown >= size + 4096, "{size} -> {grown}");
    assert_eq!(moved, stream);

    // Later edits, growing or shrinking, fit and keep every audio byte in place.
    for (title, comment) in [("S", "kept"), (long.as_str(), "x".repeat(2000).as_str())] {
        write_fields(&path, &fields(&[("TITLE", title), ("COMMENT", comment)])).unwrap();
        assert_eq!(mpeg(&path), (grown, stream.clone()), "{title}");
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.fields["TITLE"], [title]);
        assert_eq!(entry.fields["COMMENT"], [comment]);
    }
    decodes(&path);

    // ID3v2.4 is written in place as well.
    let mut options = TagOptions::default();
    options.mpeg.id3v2_format = tag_core::Id3v2Format::V24Utf8;
    write_fields_with_options(&path, &fields(&[("TITLE", "Vier")]), false, &[], &options).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes[3], 4);
    assert_eq!(mpeg(&path), (grown, stream.clone()));
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["Vier"]);
}

#[test]
fn id3v2_padding_writes_never_modify_a_hard_linked_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("linked.mp3");
    if !ffmpeg(&["-c:a", "libmp3lame", "-metadata", "title=T"], &path) {
        return;
    }
    write_fields(&path, &fields(&[("TITLE", "Padded")])).unwrap();
    let snapshot = dir.path().join("snapshot.mp3");
    fs::hard_link(&path, &snapshot).unwrap();
    let before = fs::read(&snapshot).unwrap();
    write_fields(&path, &fields(&[("TITLE", "Edited in padding")])).unwrap();
    assert_eq!(fs::read(&snapshot).unwrap(), before, "snapshot changed");
    assert_eq!(fs::read(&path).unwrap().len(), before.len(), "in place");
    assert_eq!(
        read_entry(&path).unwrap().fields["TITLE"],
        ["Edited in padding"]
    );
}

#[test]
fn wav_id3_chunk_is_rewritten_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("song.wav");
    // Without the encoder INFO chunk, and with a field that has no RIFF id,
    // only the ID3 chunk holds tags.
    if !ffmpeg(&["-fflags", "+bitexact", "-flags:a", "+bitexact"], &path) {
        return;
    }
    write_fields(&path, &fields(&[("COMPOSER", "First")])).unwrap();
    let before = fs::read(&path).unwrap();
    write_fields(&path, &fields(&[("COMPOSER", "A longer second composer")])).unwrap();
    let after = fs::read(&path).unwrap();
    assert_eq!(after.len(), before.len());
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["COMPOSER"], ["A longer second composer"]);
    assert_eq!(entry.technical["_TAG"], ["ID3v2.4"]);
    decodes(&path);
}

/// File size, `mdat` payload and the position of `moov`.
fn mp4(path: &Path) -> (usize, Vec<u8>, usize) {
    let bytes = fs::read(path).unwrap();
    let (mut at, mut mdat, mut moov) = (0, None, 0);
    while at + 8 <= bytes.len() {
        let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        match &bytes[at + 4..at + 8] {
            b"mdat" => mdat = Some(bytes[at + 8..at + size].to_vec()),
            b"moov" => moov = at,
            _ => {}
        }
        at += size;
    }
    (bytes.len(), mdat.unwrap(), moov)
}

#[test]
fn mp4_edits_reuse_free_atoms_and_rewrites_reserve_them() {
    for faststart in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.m4a");
        let mut args = vec!["-c:a", "aac", "-metadata", "title=Original"];
        if faststart {
            args.extend(["-movflags", "+faststart"]);
        }
        if !ffmpeg(&args, &path) {
            return;
        }
        let (size, media, moov) = mp4(&path);
        // ffmpeg reserves no `free` atom beside `ilst`: the first growing edit
        // splices one in, moving the media only when `moov` precedes it.
        write_fields(&path, &fields(&[("TITLE", "A longer title")])).unwrap();
        let (grown, moved, _) = mp4(&path);
        assert!(grown >= size + 4096, "{size} -> {grown}");
        assert_eq!(moved, media);
        decodes(&path);

        for title in ["S", &"y".repeat(3000), "Back"] {
            write_fields(&path, &fields(&[("TITLE", title), ("COMMENT", "c")])).unwrap();
            assert_eq!(mp4(&path), (grown, media.clone(), moov), "{title}");
            let entry = read_entry(&path).unwrap();
            assert_eq!(entry.fields["TITLE"], [title]);
            assert_eq!(entry.fields["COMMENT"], ["c"]);
        }
        decodes(&path);
        let probe = Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format_tags=title", "-of"])
            .args(["default=nw=1:nk=1"])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "Back");
    }
}

/// The undo journal counts an in-place write by its region, not the file.
#[test]
fn in_place_writes_are_journaled_by_region() {
    let dir = tempfile::tempdir().unwrap();
    if !tag_core::clones_in(dir.path()) {
        return;
    }
    let mp3 = dir.path().join("song.mp3");
    let m4a = dir.path().join("song.m4a");
    let lame = ["-c:a", "libmp3lame", "-metadata", "title=T"];
    if !ffmpeg_seconds(10, &lame, &mp3) || !ffmpeg_seconds(10, &["-c:a", "aac"], &m4a) {
        return;
    }
    let mut history = tag_core::history::Journal::open(
        &dir.path().join("journal"),
        tag_core::history::Limits::default(),
    )
    .unwrap();
    for path in [&mp3, &m4a] {
        // The first edit reserves padding with a full rewrite.
        write_fields(path, &fields(&[("TITLE", "Padded")])).unwrap();
        let before = history.retained_bytes();
        let key = path.to_string_lossy().into_owned();
        let capture = history.begin("Save tags", None, &[key]);
        write_fields(path, &fields(&[("TITLE", "Edited in place")])).unwrap();
        history.commit(capture, &[], &[]);
        let cost = history.retained_bytes() - before;
        assert!(
            cost <= 64 * 1024 && cost * 2 < fs::metadata(path).unwrap().len(),
            "{cost} bytes for an in-place edit of {}",
            path.display()
        );
    }
}
