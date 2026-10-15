use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tag_core::history::{Journal, Limits};
use tag_core::{add_cover, read_entry, remove_cover, remove_tags, write_fields, Fields};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(key, values)| {
            (
                key.to_string(),
                values.iter().map(|v| v.to_string()).collect(),
            )
        })
        .collect()
}

// Independently generated PCM audio; no vendor media is used.
fn wav(path: &Path) {
    let samples: Vec<u8> = (0..8000)
        .flat_map(|i| ((i % 251) as i16 * 31).to_le_bytes())
        .collect();
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend((36 + samples.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32).to_le_bytes());
    bytes.extend(&samples);
    fs::write(path, bytes).unwrap();
}

// Silent MPEG-1 Layer III frames (128 kbit/s, 44.1 kHz, 417 bytes each).
fn mp3(path: &Path) {
    let mut bytes = Vec::new();
    for _ in 0..40 {
        bytes.extend([0xFF, 0xFB, 0x90, 0x64]);
        bytes.extend(std::iter::repeat_n(0u8, 413));
    }
    fs::write(path, bytes).unwrap();
}

fn flac(path: &Path) -> bool {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("Skipping FLAC undo fixture: ffmpeg is required");
        return false;
    }
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=8000:duration=0.2",
        ])
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

fn png(dir: &Path) -> PathBuf {
    let image = dir.join("cover.png");
    fs::write(&image, STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap()).unwrap();
    image
}

fn journal(dir: &Path) -> Journal {
    Journal::open(&dir.join("journal"), Limits::default()).unwrap()
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn canonical(path: &Path) -> String {
    key(&dunce::canonicalize(path).unwrap())
}

#[test]
fn undo_and_redo_restore_exact_fields_and_covers_across_formats() {
    let dir = tempfile::tempdir().unwrap();
    let image = png(dir.path());
    let mut files = vec![dir.path().join("song.wav"), dir.path().join("song.mp3")];
    wav(&files[0]);
    mp3(&files[1]);
    let flac_path = dir.path().join("song.flac");
    if flac(&flac_path) {
        files.push(flac_path);
    }
    let mut history = journal(dir.path());
    for path in &files {
        write_fields(
            path,
            &fields(&[("TITLE", &["Original"]), ("ARTIST", &["A", "B"])]),
        )
        .unwrap();
        add_cover(path, &image).unwrap();
    }
    let paths: Vec<String> = files.iter().map(|p| key(p)).collect();
    let before: Vec<Vec<u8>> = files.iter().map(|p| fs::read(p).unwrap()).collect();
    let originals: Vec<_> = files.iter().map(|p| read_entry(p).unwrap()).collect();

    let capture = history.begin("Save tags", None, &paths);
    for path in &files {
        write_fields(path, &fields(&[("TITLE", &["Changed"]), ("ARTIST", &[])])).unwrap();
        remove_cover(path, 0).unwrap();
    }
    history.commit(capture, &[], &[]);
    let after: Vec<Vec<u8>> = files.iter().map(|p| fs::read(p).unwrap()).collect();
    let state = history.state();
    assert_eq!(state.undo.as_ref().unwrap().label, "Save tags");
    assert_eq!(state.undo.unwrap().files, files.len());
    assert!(state.redo.is_none());

    let replay = history.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(replay.paths.len(), files.len());
    for (index, path) in files.iter().enumerate() {
        assert_eq!(fs::read(path).unwrap(), before[index], "{}", path.display());
        let entry = read_entry(path).unwrap();
        assert_eq!(entry.fields, originals[index].fields);
        assert_eq!(entry.covers.len(), 1);
        assert_eq!(
            entry.covers[0].data_base64,
            originals[index].covers[0].data_base64
        );
    }
    assert!(history.state().undo.is_none());
    assert_eq!(history.state().redo.unwrap().label, "Save tags");

    let replay = history.redo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    for (index, path) in files.iter().enumerate() {
        assert_eq!(fs::read(path).unwrap(), after[index]);
        assert!(read_entry(path).unwrap().covers.is_empty());
    }
    // Undo after redo still reaches the original bytes.
    history.undo().unwrap();
    for (index, path) in files.iter().enumerate() {
        assert_eq!(fs::read(path).unwrap(), before[index]);
    }
}

#[test]
fn remove_tags_and_add_cover_are_reversible_and_preserve_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let image = png(dir.path());
    let path = dir.path().join("song.mp3");
    mp3(&path);
    write_fields(
        &path,
        &fields(&[("TITLE", &["Keep me"]), ("COMMENT", &["Long\ncomment"])]),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let original = fs::read(&path).unwrap();
    let mut history = journal(dir.path());
    let capture = history.begin("Remove tags", None, &[key(&path)]);
    remove_tags(&path).unwrap();
    history.commit(capture, &[], &[]);
    let capture = history.begin("Add cover", None, &[key(&path)]);
    add_cover(&path, &image).unwrap();
    history.commit(capture, &[], &[]);
    assert_eq!(history.state().undo.unwrap().label, "Add cover");
    history.undo().unwrap();
    let entry = read_entry(&path).unwrap();
    assert!(entry.fields.is_empty() && entry.covers.is_empty());
    assert_eq!(history.state().undo.unwrap().label, "Remove tags");
    history.undo().unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }
}

#[test]
fn renames_and_directory_moves_reverse_and_reapply() {
    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap();
    let dir = root.as_path();
    let album = dir.join("Album");
    fs::create_dir(&album).unwrap();
    let first = album.join("01.wav");
    let second = album.join("02.wav");
    let bystander = album.join("cover.jpg");
    wav(&first);
    wav(&second);
    fs::write(&bystander, b"not audio").unwrap();
    write_fields(&first, &fields(&[("TITLE", &["One"])])).unwrap();
    let original = fs::read(&first).unwrap();
    let (first_key, second_key) = (canonical(&first), canonical(&second));
    let mut history = journal(dir);

    // An action renames the first file, edits it, then renames the folder.
    let capture = history.begin("Actions", None, &[first_key.clone(), second_key.clone()]);
    let renamed = album.join("One.wav");
    tag_core::operations::rename_no_clobber(&first, &renamed).unwrap();
    write_fields(&renamed, &fields(&[("TITLE", &["Uno"])])).unwrap();
    let moved = dir.join("Renamed album");
    fs::rename(&album, &moved).unwrap();
    let final_first = moved.join("One.wav");
    let final_second = moved.join("02.wav");
    history.commit(
        capture,
        &[
            (first_key.clone(), key(&final_first)),
            (second_key.clone(), key(&final_second)),
        ],
        &[(key(&album), key(&moved))],
    );
    let edited = fs::read(&final_first).unwrap();

    let replay = history.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert!(!moved.exists());
    assert_eq!(fs::read(&first).unwrap(), original);
    assert!(second.exists() && bystander.exists());
    assert_eq!(replay.directories, vec![(key(&moved), key(&album))]);
    assert!(replay
        .renamed
        .contains(&(key(&final_first), first_key.clone())));
    assert!(replay
        .renamed
        .contains(&(key(&final_second), second_key.clone())));

    let replay = history.redo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert!(!album.exists());
    assert_eq!(fs::read(&final_first).unwrap(), edited);
    assert!(final_second.exists() && moved.join("cover.jpg").exists());
}

#[test]
fn pure_rename_keeps_no_snapshot_and_reverses() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("a.wav");
    wav(&source);
    let target = dir.path().join("b.wav");
    let mut history = journal(dir.path());
    let capture = history.begin("Rename", None, &[key(&source)]);
    tag_core::operations::rename_no_clobber(&source, &target).unwrap();
    history.commit(capture, &[(key(&source), key(&target))], &[]);
    assert_eq!(history.retained_bytes(), 0, "pure renames retain no audio");
    let pair = (
        canonical(&target),
        key(&dunce::canonicalize(dir.path()).unwrap().join("a.wav")),
    );
    let replay = history.undo().unwrap();
    assert_eq!(replay.renamed, vec![pair]);
    assert!(source.exists() && !target.exists());
    history.redo().unwrap();
    assert!(!source.exists() && target.exists());
}

#[test]
fn external_changes_are_reported_and_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let changed = dir.path().join("changed.wav");
    let kept = dir.path().join("kept.wav");
    wav(&changed);
    wav(&kept);
    let kept_original = fs::read(&kept).unwrap();
    let mut history = journal(dir.path());
    let capture = history.begin("Save tags", None, &[key(&changed), key(&kept)]);
    write_fields(&changed, &fields(&[("TITLE", &["Ours"])])).unwrap();
    write_fields(&kept, &fields(&[("TITLE", &["Ours"])])).unwrap();
    history.commit(capture, &[], &[]);
    // Another program edits one file after the operation.
    write_fields(&changed, &fields(&[("TITLE", &["Theirs"])])).unwrap();
    let external = fs::read(&changed).unwrap();
    let replay = history.undo().unwrap();
    assert_eq!(replay.errors.len(), 1);
    assert!(replay.errors[0].error.contains("Changed outside OpenTag"));
    assert_eq!(
        fs::read(&changed).unwrap(),
        external,
        "external edit must survive"
    );
    assert_eq!(fs::read(&kept).unwrap(), kept_original);
    // Only the restored file remains redoable.
    assert_eq!(history.state().redo.unwrap().files, 1);

    // An external change before redo is detected the same way.
    write_fields(&kept, &fields(&[("TITLE", &["Elsewhere"])])).unwrap();
    let elsewhere = fs::read(&kept).unwrap();
    let replay = history.redo().unwrap();
    assert_eq!(replay.errors.len(), 1);
    assert_eq!(fs::read(&kept).unwrap(), elsewhere);
    assert!(history.state().undo.is_none() && history.state().redo.is_none());
    assert!(history.undo().is_err());
}

#[test]
fn deleted_or_recreated_files_and_occupied_rename_sources_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("a.wav");
    wav(&source);
    let target = dir.path().join("b.wav");
    let mut history = journal(dir.path());
    let capture = history.begin("Rename", None, &[key(&source)]);
    tag_core::operations::rename_no_clobber(&source, &target).unwrap();
    write_fields(&target, &fields(&[("TITLE", &["Renamed"])])).unwrap();
    history.commit(capture, &[(key(&source), key(&target))], &[]);
    // Something new now occupies the original name.
    fs::write(&source, b"new file").unwrap();
    let replay = history.undo().unwrap();
    assert_eq!(replay.errors.len(), 1);
    assert_eq!(fs::read(&source).unwrap(), b"new file");
    assert_eq!(read_entry(&target).unwrap().fields["TITLE"], ["Renamed"]);
}

#[test]
fn grouped_saves_merge_and_unchanged_files_are_not_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    let untouched = dir.path().join("c.wav");
    for path in [&a, &b, &untouched] {
        wav(path);
    }
    let (a_original, b_original) = (fs::read(&a).unwrap(), fs::read(&b).unwrap());
    let mut history = journal(dir.path());
    for (path, title) in [(&a, "One"), (&b, "Two"), (&a, "Again")] {
        let capture = history.begin("Save tags", Some("save-1"), &[key(path)]);
        write_fields(path, &fields(&[("TITLE", &[title])])).unwrap();
        history.commit(capture, &[], &[]);
    }
    // A failed write leaves the file untouched and adds nothing.
    let capture = history.begin("Save tags", Some("save-1"), &[key(&untouched)]);
    assert!(write_fields(&untouched, &fields(&[("BAD\0", &["x"])])).is_err());
    history.commit(capture, &[], &[]);
    let state = history.state();
    assert_eq!(state.undo.unwrap().files, 2);
    history.undo().unwrap();
    assert_eq!(fs::read(&a).unwrap(), a_original);
    assert_eq!(fs::read(&b).unwrap(), b_original);
    assert!(history.state().undo.is_none());
    // An operation that changes nothing records nothing and keeps redo.
    let capture = history.begin("Remove tags", None, &[key(&untouched)]);
    history.commit(capture, &[], &[]);
    assert!(history.state().redo.is_some());
    // A new change clears redo.
    let capture = history.begin("Save tags", None, &[key(&untouched)]);
    write_fields(&untouched, &fields(&[("TITLE", &["New"])])).unwrap();
    history.commit(capture, &[], &[]);
    assert!(history.state().redo.is_none());
}

fn snapshots(dir: &Path) -> usize {
    fs::read_dir(dir.join("journal"))
        .unwrap()
        .flatten()
        .flat_map(|session| fs::read_dir(session.path()).unwrap().flatten())
        .filter(|f| f.file_name().to_string_lossy().ends_with(".snapshot"))
        .count()
}

#[test]
fn limits_bound_operations_and_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    let size = fs::metadata(&path).unwrap().len();
    let mut history = Journal::open(
        &dir.path().join("journal"),
        Limits {
            operations: 3,
            bytes: size * 10,
            ..Limits::default()
        },
    )
    .unwrap();
    for index in 0..5 {
        let capture = history.begin(&format!("Save {index}"), None, &[key(&path)]);
        write_fields(&path, &fields(&[("TITLE", &[&index.to_string()])])).unwrap();
        history.commit(capture, &[], &[]);
    }
    assert_eq!(snapshots(dir.path()), 3, "evicted snapshots are deleted");
    for expected in ["Save 4", "Save 3", "Save 2"] {
        assert_eq!(history.state().undo.unwrap().label, expected);
        history.undo().unwrap();
    }
    assert!(history.state().undo.is_none());
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["1"]);

    // An operation larger than the byte bound is applied but not recorded.
    let mut small = Journal::open(
        &dir.path().join("journal"),
        Limits {
            operations: 10,
            bytes: size - 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let capture = small.begin("Save tags", None, &[key(&path)]);
    write_fields(&path, &fields(&[("TITLE", &["Too big"])])).unwrap();
    small.commit(capture, &[], &[]);
    let state = small.state();
    assert!(state.undo.is_none());
    assert!(state.notice.unwrap().contains("too large"));
}

#[test]
fn stale_sessions_are_removed_and_live_sessions_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("journal");
    let stale = root.join("session-1-stale");
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("1.snapshot"), b"old").unwrap();
    let live = Journal::open(&root, Limits::default()).unwrap();
    assert!(!stale.exists());
    let second = Journal::open(&root, Limits::default()).unwrap();
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        2,
        "a running session is retained"
    );
    drop(live);
    drop(second);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}

#[test]
fn consecutive_steps_undo_and_redo_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.mp3");
    mp3(&path);
    let mut history = journal(dir.path());
    let mut states = vec![fs::read(&path).unwrap()];
    for title in ["One", "Two", "Three"] {
        let capture = history.begin(title, None, &[key(&path)]);
        write_fields(&path, &fields(&[("TITLE", &[title])])).unwrap();
        history.commit(capture, &[], &[]);
        states.push(fs::read(&path).unwrap());
    }
    for _ in 0..2 {
        for expected in states[..3].iter().rev() {
            let replay = history.undo().unwrap();
            assert!(replay.errors.is_empty(), "{:?}", replay.errors);
            assert_eq!(&fs::read(&path).unwrap(), expected);
        }
        assert!(history.undo().is_err());
        for expected in &states[1..] {
            let replay = history.redo().unwrap();
            assert!(replay.errors.is_empty(), "{:?}", replay.errors);
            assert_eq!(&fs::read(&path).unwrap(), expected);
        }
        assert!(history.redo().is_err());
    }
}

fn noise_flac(path: &Path, seconds: u32) -> bool {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("Skipping clone accounting test: ffmpeg is required");
        return false;
    }
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i"])
        .arg(format!("anoisesrc=d={seconds}:r=44100:a=0.5"))
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

#[test]
fn full_rewrites_count_whole_files_and_padding_writes_count_their_region() {
    let dir = tempfile::tempdir().unwrap();
    let flac = dir.path().join("padded.flac");
    if !noise_flac(&flac, 2) {
        return;
    }
    let wav_path = dir.path().join("plain.wav");
    wav(&wav_path);
    let (flac_len, wav_len) = (
        fs::metadata(&flac).unwrap().len(),
        fs::metadata(&wav_path).unwrap().len(),
    );
    let mut history = journal(dir.path());
    let clones = tag_core::clones_in(dir.path());

    // WAV tags are rewritten as a whole: the snapshot keeps the whole file.
    let capture = history.begin("Save tags", None, &[key(&wav_path)]);
    write_fields(&wav_path, &fields(&[("TITLE", &["Whole"])])).unwrap();
    history.commit(capture, &[], &[]);
    assert_eq!(history.retained_bytes(), wav_len);

    // A FLAC edit that fits the padding rewrites only the metadata region.
    let capture = history.begin("Save tags", None, &[key(&flac)]);
    write_fields(&flac, &fields(&[("TITLE", &["Small"])])).unwrap();
    history.commit(capture, &[], &[]);
    let flac_cost = history.retained_bytes() - wav_len;
    if clones {
        assert!(
            flac_cost <= 64 * 1024,
            "{flac_cost} bytes for a padding write"
        );
    } else {
        assert_eq!(flac_cost, flac_len, "full copies keep the whole file");
    }

    // Overflowing the padding shifts the audio: the whole file is kept.
    let before = fs::metadata(&flac).unwrap().len();
    let capture = history.begin("Save tags", None, &[key(&flac)]);
    write_fields(&flac, &fields(&[("COMMENT", &[&"x".repeat(64 * 1024)])])).unwrap();
    history.commit(capture, &[], &[]);
    assert_eq!(history.retained_bytes() - wav_len - flac_cost, before);

    // Undo keeps both versions: a full rewrite retains both whole files, a
    // clone-backed padding write retains about its region twice.
    let after = fs::metadata(&flac).unwrap().len();
    history.undo().unwrap();
    history.undo().unwrap();
    let (undo_cost, redo) = (history.retained_bytes(), history.state().redo.unwrap());
    assert_eq!(redo.label, "Save tags");
    let small = undo_cost - wav_len - before - after;
    if clones {
        assert!(small <= 2 * 64 * 1024, "{small}");
    } else {
        assert_eq!(small, 2 * flac_len);
    }
}

#[test]
fn many_padded_flac_saves_stay_under_the_cap_and_undo_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let template = dir.path().join("template.flac");
    if !noise_flac(&template, 6) || !tag_core::clones_in(dir.path()) {
        eprintln!("Skipping: needs ffmpeg and a cloning volume (APFS, Btrfs, XFS)");
        return;
    }
    let size = fs::metadata(&template).unwrap().len();
    let count = 48;
    let files: Vec<PathBuf> = (0..count)
        .map(|index| {
            let path = dir.path().join(format!("{index:02}.flac"));
            fs::copy(&template, &path).unwrap();
            write_fields(&path, &fields(&[("TITLE", &[&format!("Track {index}")])])).unwrap();
            path
        })
        .collect();
    let originals: Vec<Vec<u8>> = files.iter().map(|p| fs::read(p).unwrap()).collect();
    // The whole selection is many times the cap; only rewritten regions count.
    let cap = size * 2;
    assert!(size * count > 20 * cap);
    let mut history = Journal::open(
        &dir.path().join("journal"),
        Limits {
            bytes: cap,
            ..Limits::default()
        },
    )
    .unwrap();
    let paths: Vec<String> = files.iter().map(|p| key(p)).collect();
    let capture = history.begin("Save tags", None, &paths);
    let options = tag_core::TagOptions::default();
    let written =
        tag_core::parallel::map_ordered_on(8, &files, &tag_core::parallel::Silent, |path| {
            tag_core::write_fields_with_options(
                path,
                &fields(&[("ALBUM", &["One album for all"])]),
                false,
                &[],
                &options,
            )
        });
    assert!(written.iter().all(|w| matches!(w, Some(Ok(_)))));
    history.commit(capture, &[], &[]);
    let state = history.state();
    assert_eq!(state.notice, None);
    assert_eq!(state.undo.unwrap().files, count as usize);
    assert!(
        history.retained_bytes() <= count * 64 * 1024,
        "{} retained for {count} padding writes",
        history.retained_bytes()
    );
    let replay = history.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    for (path, original) in files.iter().zip(&originals) {
        assert_eq!(&fs::read(path).unwrap(), original);
    }
    history.redo().unwrap();
    assert!(files
        .iter()
        .all(|p| read_entry(p).unwrap().fields["ALBUM"] == ["One album for all"]));
}

// The free-space guard reads statvfs and exists on Unix only.
#[cfg(unix)]
#[test]
fn journaling_is_refused_when_free_space_would_run_low() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    let mut history = Journal::open(
        &dir.path().join("journal"),
        Limits {
            min_free: u64::MAX / 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let capture = history.begin("Save tags", None, &[key(&path)]);
    write_fields(&path, &fields(&[("TITLE", &["Applied anyway"])])).unwrap();
    history.commit(capture, &[], &[]);
    let state = history.state();
    assert!(state.undo.is_none());
    if cfg!(unix) {
        assert!(state.notice.unwrap().contains("free disk space"));
    }
    assert_eq!(
        read_entry(&path).unwrap().fields["TITLE"],
        ["Applied anyway"]
    );
}
