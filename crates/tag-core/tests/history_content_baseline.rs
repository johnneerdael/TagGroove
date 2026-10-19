use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use tag_core::{history::Journal, history::Limits, write_fields, Fields};

struct FixtureDir(Option<tempfile::TempDir>);
impl FixtureDir {
    fn path(&self) -> &Path {
        self.0.as_ref().unwrap().path()
    }
}
impl Drop for FixtureDir {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let retained = self.0.take().unwrap().keep();
            eprintln!(
                "Retained failing fixture and independent before/external/replay bytes: {}",
                retained.display()
            );
        }
    }
}
fn fixture_dir() -> FixtureDir {
    FixtureDir(Some(tempfile::tempdir().unwrap()))
}
fn checkpoint(path: &Path, label: &str) {
    fs::copy(
        path,
        path.parent().unwrap().join(format!("evidence-{label}")),
    )
    .unwrap();
}

fn wav(path: &Path) {
    let samples: Vec<_> = (0..8000)
        .flat_map(|i| ((i % 251) as i16 * 31).to_le_bytes())
        .collect();
    let mut bytes = b"RIFF".to_vec();
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
    bytes.extend(samples);
    fs::write(path, bytes).unwrap();
    checkpoint(path, "input.wav");
}

/// Mutate the existing object, retaining its exact length and modification time.
/// This deliberately defeats the old metadata-only guard without replacing it.
fn concealed_edit(path: &Path, offset: u64) -> Vec<u8> {
    let before = fs::metadata(path).unwrap();
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(&[byte[0] ^ 1]).unwrap();
    file.sync_all().unwrap();
    file.set_modified(before.modified().unwrap()).unwrap();
    let after = fs::metadata(path).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(after.modified().unwrap(), before.modified().unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
    }
    fs::read(path).unwrap()
}

fn pcm_offset(path: &Path) -> u64 {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    let mut start = 12;
    while start + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()) as usize;
        if &bytes[start..start + 4] == b"data" {
            assert!(len > 0 && start + 8 + len <= bytes.len());
            return (start + 8) as u64;
        }
        start += 8 + len + len % 2;
    }
    panic!("Generated WAV lost its PCM data chunk");
}

fn saved(history: &mut Journal, path: &Path) {
    let capture = history.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    let fields: Fields = [("TITLE".into(), vec!["Our edit".into()])].into();
    write_fields(path, &fields).unwrap();
    history.commit(capture, &[], &[]);
    assert!(history.state().notice.is_none());
    assert!(history.state().undo.is_some());
    checkpoint(path, "saved.wav");
}

fn audio_edit_survives_replay(redo: bool) {
    let dir = fixture_dir();
    let path = dir.path().join("audio.wav");
    wav(&path);
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    saved(&mut history, &path);
    if redo {
        assert!(history.undo().unwrap().errors.is_empty());
    }
    let external = concealed_edit(&path, pcm_offset(&path));
    checkpoint(&path, "external.wav");
    let replay = if redo { history.redo() } else { history.undo() }.unwrap();
    checkpoint(&path, "replay.wav");
    assert!(
        fs::read(&path).unwrap() == external,
        "external audio overwritten; redo={redo}"
    );
    assert_eq!(replay.errors.len(), 1, "redo={redo}");
    assert!(replay.paths.is_empty());
    assert!(history.state().undo.is_none() && history.state().redo.is_none());
}

#[test]
fn concealed_same_object_audio_edits_survive_undo() {
    audio_edit_survives_replay(false);
}

#[test]
fn concealed_same_object_audio_edits_survive_redo() {
    audio_edit_survives_replay(true);
}

#[test]
fn concealed_snapshot_corruption_is_refused_before_audio_replacement() {
    let dir = fixture_dir();
    let path = dir.path().join("audio.wav");
    wav(&path);
    let original = fs::read(&path).unwrap();
    let root = dir.path().join("history");
    let mut history = Journal::open(&root, Limits::default()).unwrap();
    saved(&mut history, &path);
    let current = fs::read(&path).unwrap();
    let snapshots: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .flat_map(|session| fs::read_dir(session.path()).unwrap().flatten())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|value| value == "snapshot"))
        .filter(|path| fs::read(path).unwrap() == original)
        .collect();
    assert_eq!(snapshots.len(), 1);
    concealed_edit(&snapshots[0], pcm_offset(&snapshots[0]));
    checkpoint(&snapshots[0], "tampered-snapshot.wav");
    fs::copy(
        snapshots[0]
            .parent()
            .unwrap()
            .join("evidence-tampered-snapshot.wav"),
        dir.path().join("evidence-tampered-snapshot.wav"),
    )
    .unwrap();
    let replay = history.undo().unwrap();
    checkpoint(&path, "replay.wav");
    assert!(
        fs::read(&path).unwrap() == current,
        "corrupt snapshot was published"
    );
    assert_eq!(replay.errors.len(), 1);
    assert!(replay.paths.is_empty());
}

#[test]
fn concealed_side_file_edits_survive_replay() {
    for redo in [false, true] {
        let dir = fixture_dir();
        let path = dir.path().join("cover.bin");
        fs::write(&path, b"Original cover").unwrap();
        let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
        let mut capture = history.begin("Save cover", None, &[]);
        history.capture_side_file(&mut capture, &path).unwrap();
        fs::write(&path, b"Our new cover!").unwrap();
        history.commit(capture, &[], &[]);
        assert!(history.state().notice.is_none());
        if redo {
            assert!(history.undo().unwrap().errors.is_empty());
        }
        let external = concealed_edit(&path, 3);
        let replay = if redo { history.redo() } else { history.undo() }.unwrap();
        assert_eq!(fs::read(&path).unwrap(), external);
        assert_eq!(replay.errors.len(), 1);
    }
}

#[test]
fn concealed_audio_edit_also_prevents_a_rename_only_replay() {
    let dir = fixture_dir();
    let source = dir.path().join("before.wav");
    let target = dir.path().join("after.wav");
    wav(&source);
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    let capture = history.begin("Rename", None, &[source.to_string_lossy().into_owned()]);
    tag_core::operations::rename_no_clobber(&source, &target).unwrap();
    history.commit(
        capture,
        &[(
            source.to_string_lossy().into_owned(),
            target.to_string_lossy().into_owned(),
        )],
        &[],
    );
    let external = concealed_edit(&target, pcm_offset(&target));
    checkpoint(&target, "external.wav");
    let replay = history.undo().unwrap();
    checkpoint(
        if target.exists() { &target } else { &source },
        "replay.wav",
    );
    assert!(
        target.exists() && !source.exists(),
        "changed audio was renamed by Undo"
    );
    assert_eq!(fs::read(&target).unwrap(), external);
    assert_eq!(replay.errors.len(), 1);
}

#[test]
fn concealed_side_snapshot_corruption_is_refused() {
    let dir = fixture_dir();
    let path = dir.path().join("cover.bin");
    fs::write(&path, b"Original cover").unwrap();
    let root = dir.path().join("history");
    let mut history = Journal::open(&root, Limits::default()).unwrap();
    let mut capture = history.begin("Save cover", None, &[]);
    history.capture_side_file(&mut capture, &path).unwrap();
    fs::write(&path, b"Our new cover!").unwrap();
    history.commit(capture, &[], &[]);
    let current = fs::read(&path).unwrap();
    let snapshot = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .flat_map(|session| fs::read_dir(session.path()).unwrap().flatten())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|value| value == "snapshot")
                && fs::read(path).unwrap() == b"Original cover"
        })
        .unwrap();
    concealed_edit(&snapshot, 2);
    fs::copy(&snapshot, dir.path().join("evidence-tampered-side.bin")).unwrap();
    let replay = history.undo().unwrap();
    checkpoint(&path, "replay.bin");
    assert!(
        fs::read(&path).unwrap() == current,
        "corrupt side-file snapshot was published"
    );
    assert_eq!(replay.errors.len(), 1);
}

#[cfg(unix)]
#[test]
fn replaced_audio_leaf_symlink_is_refused_without_following_its_referent() {
    let dir = fixture_dir();
    let path = dir.path().join("audio.wav");
    let referent = dir.path().join("moved.wav");
    wav(&path);
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    saved(&mut history, &path);
    let current = fs::read(&path).unwrap();
    fs::rename(&path, &referent).unwrap();
    std::os::unix::fs::symlink(&referent, &path).unwrap();
    let replay = history.undo().unwrap();
    checkpoint(&referent, "replay.wav");
    assert!(
        fs::read(&referent).unwrap() == current,
        "replaced symlink authorized a write to its referent"
    );
    assert_eq!(fs::read_link(&path).unwrap(), referent);
    assert_eq!(replay.errors.len(), 1);
}

#[cfg(unix)]
#[test]
fn refused_history_admission_does_not_read_audio_contents() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fixture_dir();
    let path = dir.path().join("write-only.wav");
    wav(&path);
    let mut history = Journal::open(
        &dir.path().join("history"),
        Limits {
            min_free: u64::MAX,
            ..Limits::default()
        },
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o200)).unwrap();
    let capture = history.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    history.commit(capture, &[], &[]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let state = history.state();
    assert!(state.undo.is_none());
    assert!(state.notice.unwrap().contains("not enough free disk space"));
}

#[test]
fn a_replacement_after_capture_is_not_accepted_as_our_save() {
    let dir = fixture_dir();
    let path = dir.path().join("audio.wav");
    wav(&path);
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    let capture = history.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    let before = fs::metadata(&path).unwrap();
    let mut external = fs::read(&path).unwrap();
    external[44] ^= 7;
    let mut replacement = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    replacement.write_all(&external).unwrap();
    replacement
        .as_file()
        .set_modified(before.modified().unwrap())
        .unwrap();
    replacement.persist(&path).unwrap();
    checkpoint(&path, "external.wav");
    let result = write_fields(&path, &[("TITLE".into(), vec!["Ours".into()])].into());
    history.commit(capture, &[], &[]);
    checkpoint(&path, "replay.wav");
    assert!(
        result.is_err(),
        "a changed capture baseline was accepted for Save"
    );
    assert!(fs::read(&path).unwrap() == external);
    assert!(history.state().undo.is_none());
    assert!(history.state().notice.is_some());
}

#[test]
fn an_external_change_after_our_save_is_not_recorded_as_our_work() {
    let dir = fixture_dir();
    let path = dir.path().join("audio.wav");
    wav(&path);
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    let capture = history.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    write_fields(&path, &[("TITLE".into(), vec!["Ours".into()])].into()).unwrap();
    let external = concealed_edit(&path, pcm_offset(&path));
    checkpoint(&path, "external.wav");
    history.commit(capture, &[], &[]);
    assert!(
        history.state().undo.is_none(),
        "post-write external data became the journal baseline"
    );
    assert!(history.state().notice.is_some());
    assert!(fs::read(&path).unwrap() == external);
}

#[cfg(unix)]
#[test]
fn a_leaf_symlink_substituted_after_capture_cannot_bypass_the_save_guard() {
    let dir = fixture_dir();
    let path = dir.path().join("audio.wav");
    let referent = dir.path().join("moved.wav");
    wav(&path);
    let original = fs::read(&path).unwrap();
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    let capture = history.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    fs::rename(&path, &referent).unwrap();
    std::os::unix::fs::symlink(&referent, &path).unwrap();
    let result = write_fields(&path, &[("TITLE".into(), vec!["Ours".into()])].into());
    history.commit(capture, &[], &[]);
    checkpoint(&referent, "replay.wav");
    assert!(
        result.is_err(),
        "canonicalization bypassed the captured-path guard"
    );
    assert!(fs::read(&referent).unwrap() == original);
    assert_eq!(fs::read_link(&path).unwrap(), referent);
    assert!(history.state().undo.is_none());
}

#[test]
fn an_audio_edit_beyond_the_first_read_buffer_is_refused() {
    let dir = fixture_dir();
    let path = dir.path().join("long.wav");
    wav(&path);
    let mut bytes = fs::read(&path).unwrap();
    bytes.resize(44 + 262_144, 0x5a);
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
    bytes[40..44].copy_from_slice(&262_144u32.to_le_bytes());
    fs::write(&path, bytes).unwrap();
    checkpoint(&path, "input.wav");
    let mut history = Journal::open(&dir.path().join("history"), Limits::default()).unwrap();
    saved(&mut history, &path);
    let external = concealed_edit(&path, pcm_offset(&path) + 200_000);
    checkpoint(&path, "external.wav");
    let result = history.undo().unwrap();
    checkpoint(&path, "replay.wav");
    assert!(fs::read(&path).unwrap() == external);
    assert_eq!(result.errors.len(), 1);
}
