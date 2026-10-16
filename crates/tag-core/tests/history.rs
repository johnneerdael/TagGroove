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

fn preserved_timestamp_edit_after_move(rename_file: bool, move_folder: bool) {
    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap();
    let album = root.join("Album");
    fs::create_dir(&album).unwrap();
    let original_path = album.join("01.wav");
    wav(&original_path);
    let options = tag_core::TagOptions {
        preserve_modified_time: true,
        ..Default::default()
    };
    tag_core::write_fields_with_options(
        &original_path,
        &fields(&[("TITLE", &["One"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    let original = fs::read(&original_path).unwrap();
    let modified = fs::metadata(&original_path).unwrap().modified().unwrap();
    let original_key = canonical(&original_path);
    let mut history = journal(&root);
    let capture = history.begin("Move and save", None, std::slice::from_ref(&original_key));
    let mut edited_path = original_path.clone();
    if rename_file {
        edited_path = album.join("One.wav");
        tag_core::operations::rename_no_clobber(&original_path, &edited_path).unwrap();
    }
    let moved = root.join("Moved album");
    if move_folder {
        tag_core::operations::rename_directory_no_clobber(&album, &moved).unwrap();
        edited_path = moved.join(edited_path.file_name().unwrap());
    }
    tag_core::write_fields_with_options(
        &edited_path,
        &fields(&[("TITLE", &["Uno"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    let edited = fs::read(&edited_path).unwrap();
    assert_ne!(edited, original);
    assert_eq!(edited.len(), original.len());
    assert_eq!(
        fs::metadata(&edited_path).unwrap().modified().unwrap(),
        modified
    );
    let renames = if rename_file {
        vec![(original_key, key(&edited_path))]
    } else {
        Vec::new()
    };
    let directories = if move_folder {
        vec![(key(&album), key(&moved))]
    } else {
        Vec::new()
    };
    history.commit(capture, &renames, &directories);
    let replay = history.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert!(
        fs::read(&original_path).unwrap() == original,
        "undo restores exact original bytes"
    );
    assert!(!edited_path.exists());
    let replay = history.redo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert!(
        fs::read(&edited_path).unwrap() == edited,
        "redo restores exact edited bytes"
    );
    assert!(!original_path.exists());
}

#[test]
fn renamed_same_size_timestamp_preserving_save_restores_tags_and_name() {
    preserved_timestamp_edit_after_move(true, false);
}

#[test]
fn moved_folder_same_size_timestamp_preserving_save_restores_tags_and_folder() {
    preserved_timestamp_edit_after_move(false, true);
}

#[test]
fn renamed_file_and_folder_timestamp_preserving_save_restores_all_changes() {
    preserved_timestamp_edit_after_move(true, true);
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "journal")
        .collect();
    names.sort();
    names
}

#[test]
fn case_only_renames_of_files_and_folders_are_undone_and_redone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap();
    let album = root.join("album");
    fs::create_dir(&album).unwrap();
    let first = album.join("one.wav");
    wav(&first);
    let mut history = journal(&root);
    let capture = history.begin("Case Conversion", None, &[key(&first)]);
    let renamed = album.join("One.wav");
    tag_core::operations::rename_no_clobber(&first, &renamed).unwrap();
    let moved = root.join("ALBUM");
    tag_core::operations::rename_directory_no_clobber(&album, &moved).unwrap();
    let last = moved.join("One.wav");
    history.commit(
        capture,
        &[(key(&first), key(&last))],
        &[(key(&album), key(&moved))],
    );
    assert_eq!(names(&root), ["ALBUM"]);
    assert_eq!(names(&moved), ["One.wav"]);

    let replay = history.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(names(&root), ["album"]);
    assert_eq!(names(&album), ["one.wav"]);

    let replay = history.redo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(names(&root), ["ALBUM"]);
    assert_eq!(names(&moved), ["One.wav"]);
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
fn side_files_created_or_replaced_are_removed_or_restored() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("track.wav");
    let created = dir.path().join("folder.jpg");
    let replaced = dir.path().join("cover.png");
    wav(&audio);
    fs::write(&replaced, b"old image").unwrap();
    let original = fs::read(&audio).unwrap();
    let mut history = journal(dir.path());
    let mut capture = history.begin("Tag source import", None, &[key(&audio)]);
    write_fields(&audio, &fields(&[("TITLE", &["Imported"])])).unwrap();
    for path in [&created, &replaced] {
        history.capture_side_file(&mut capture, path).unwrap();
        fs::write(path, b"new image").unwrap();
    }
    history.commit(capture, &[], &[]);
    assert_eq!(history.state().undo.unwrap().files, 1, "only audio counts");

    let replay = history.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(fs::read(&audio).unwrap(), original);
    assert!(!created.exists(), "a created image is removed");
    assert_eq!(fs::read(&replaced).unwrap(), b"old image");

    let replay = history.redo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(fs::read(&created).unwrap(), b"new image");
    assert_eq!(fs::read(&replaced).unwrap(), b"new image");

    // An image changed by another program is reported and left alone.
    fs::write(&created, b"edited elsewhere").unwrap();
    let replay = history.undo().unwrap();
    assert_eq!(replay.errors.len(), 1, "{:?}", replay.errors);
    assert!(replay.errors[0].error.contains("Changed outside OpenTag"));
    assert_eq!(fs::read(&created).unwrap(), b"edited elsewhere");
    assert_eq!(fs::read(&replaced).unwrap(), b"old image");
    assert_eq!(fs::read(&audio).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn side_file_symlinks_restore_relative_and_dangling_targets_verbatim() {
    use std::io::Write;
    for link_target in ["original.jpg", "missing.jpg"] {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.jpg");
        fs::write(&original, b"original target").unwrap();
        let output = dir.path().join("cover.jpg");
        std::os::unix::fs::symlink(link_target, &output).unwrap();
        let mut history = journal(dir.path());
        let mut capture = history.begin("Cover link", None, &[]);
        history.capture_side_file(&mut capture, &output).unwrap();
        let mut staged = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        staged.write_all(b"exported image").unwrap();
        staged.persist(&output).unwrap();
        history.commit(capture, &[], &[]);
        let undo = history.undo().unwrap();
        assert!(undo.errors.is_empty(), "{:?}", undo.errors);
        assert_eq!(fs::read_link(&output).unwrap(), Path::new(link_target));
        assert_eq!(fs::read(&original).unwrap(), b"original target");
        assert!(!dir.path().join("missing.jpg").exists());
        let redo = history.redo().unwrap();
        assert!(redo.errors.is_empty(), "{:?}", redo.errors);
        assert!(fs::symlink_metadata(&output).unwrap().is_file());
        assert_eq!(fs::read(output).unwrap(), b"exported image");
        assert_eq!(fs::read(original).unwrap(), b"original target");
        assert!(!fs::read_dir(dir.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".opentag-")));
    }
}

#[test]
fn side_file_captures_across_directory_moves_keep_the_earliest_image() {
    let dir = tempfile::tempdir().unwrap();
    // Directory records use the same canonical roots as the command path
    // and the existing directory-move journal fixtures.
    let root = dunce::canonicalize(dir.path()).unwrap();
    let old = root.join("Album");
    let middle = root.join("Renamed");
    let final_folder = root.join("Final");
    fs::create_dir(&old).unwrap();
    fs::write(old.join("cover.png"), b"original image").unwrap();
    let mut history = journal(dir.path());
    let mut capture = history.begin("Move cover", None, &[]);
    history
        .capture_side_file(&mut capture, &old.join("cover.png"))
        .unwrap();
    fs::write(old.join("cover.png"), b"intermediate image").unwrap();
    fs::rename(&old, &middle).unwrap();
    history
        .capture_side_file(&mut capture, &middle.join("cover.png"))
        .unwrap();
    fs::write(middle.join("cover.png"), b"final image").unwrap();
    fs::rename(&middle, &final_folder).unwrap();
    history.commit(
        capture,
        &[],
        &[
            (key(&old), key(&middle)),
            (key(&middle), key(&final_folder)),
        ],
    );
    let undo = history.undo().unwrap();
    assert!(undo.errors.is_empty(), "{:?}", undo.errors);
    assert_eq!(fs::read(old.join("cover.png")).unwrap(), b"original image");
    assert!(!middle.exists() && !final_folder.exists());
    let redo = history.redo().unwrap();
    assert!(redo.errors.is_empty(), "{:?}", redo.errors);
    assert_eq!(
        fs::read(final_folder.join("cover.png")).unwrap(),
        b"final image"
    );
    assert!(!old.exists() && !middle.exists());
}

#[cfg(unix)]
#[test]
fn absent_side_file_capture_resolves_symlink_ancestors_without_creating_parents() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    let alias = dir.path().join("alias");
    fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    let output = alias.join("new/missing/../cover.png");
    let mut history = journal(dir.path());
    let mut capture = history.begin("New nested cover", None, &[]);
    history.capture_side_file(&mut capture, &output).unwrap();
    assert!(
        !real.join("new").exists(),
        "capture only records the absent entry"
    );
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(&output, b"new cover image").unwrap();
    history.commit(capture, &[], &[]);
    assert!(history.undo().unwrap().errors.is_empty());
    assert!(!real.join("new/cover.png").exists());
    assert!(history.redo().unwrap().errors.is_empty());
    assert_eq!(
        fs::read(real.join("new/cover.png")).unwrap(),
        b"new cover image"
    );
    assert_eq!(fs::read_link(alias).unwrap(), real);
}

#[test]
fn unchanged_side_files_are_not_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("folder.jpg");
    fs::write(&image, b"same").unwrap();
    let mut history = journal(dir.path());
    let mut capture = history.begin("Tag source import", None, &[]);
    history.capture_side_file(&mut capture, &image).unwrap();
    fs::write(&image, b"same").unwrap();
    history
        .capture_side_file(&mut capture, &dir.path().join("never-written.jpg"))
        .unwrap();
    history.commit(capture, &[], &[]);
    assert!(history.state().undo.is_none());
    assert_eq!(snapshots(dir.path()), 0, "no copies are kept");
    // A folder in place of the image cannot be captured.
    let mut capture = history.begin("Tag source import", None, &[]);
    assert!(history.capture_side_file(&mut capture, dir.path()).is_err());
    history.commit(capture, &[], &[]);
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

#[test]
fn same_size_preserved_timestamp_edits_keep_every_undo_and_redo_step() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("same-size.wav");
    wav(&path);
    let options = tag_core::TagOptions {
        preserve_modified_time: true,
        ..Default::default()
    };
    tag_core::write_fields_with_options(&path, &fields(&[("TITLE", &["0"])]), false, &[], &options)
        .unwrap();
    let original = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let mut history = journal(dir.path());
    for index in 1..=3 {
        let before = fs::read(&path).unwrap();
        let capture = history.begin(&format!("Save {index}"), None, &[key(&path)]);
        tag_core::write_fields_with_options(
            &path,
            &fields(&[("TITLE", &[&index.to_string()])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let after = fs::read(&path).unwrap();
        assert_ne!(after, before, "each save changes the stored title");
        assert_eq!(after.len(), original.len());
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        history.commit(capture, &[], &[]);
        assert_eq!(history.state().undo.unwrap().label, format!("Save {index}"));
    }
    let latest = fs::read(&path).unwrap();
    for index in (1..=3).rev() {
        assert_eq!(history.state().undo.unwrap().label, format!("Save {index}"));
        history.undo().unwrap();
        assert_eq!(
            read_entry(&path).unwrap().fields["TITLE"],
            [(index - 1).to_string()]
        );
    }
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(history.state().undo.is_none());
    for index in 1..=3 {
        assert_eq!(history.state().redo.unwrap().label, format!("Save {index}"));
        history.redo().unwrap();
        assert_eq!(
            read_entry(&path).unwrap().fields["TITLE"],
            [index.to_string()]
        );
    }
    assert_eq!(fs::read(&path).unwrap(), latest);
    assert!(history.state().redo.is_none());
}

#[test]
fn an_empty_write_request_adds_no_step_and_keeps_redo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-write.wav");
    wav(&path);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_500_000_000))
        .unwrap();
    let original = fs::read(&path).unwrap();
    let mut history = journal(dir.path());
    let capture = history.begin("Changed title", None, &[key(&path)]);
    write_fields(&path, &fields(&[("TITLE", &["Changed"])])).unwrap();
    history.commit(capture, &[], &[]);
    history.undo().unwrap();
    let capture = history.begin("Empty write", None, &[key(&path)]);
    write_fields(&path, &Fields::new()).unwrap();
    history.commit(capture, &[], &[]);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(history.state().undo.is_none());
    assert_eq!(history.state().redo.unwrap().label, "Changed title");
    history.redo().unwrap();
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["Changed"]);
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
