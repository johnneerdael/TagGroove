use std::{fs, path::Path};
use tag_core::{
    history::{Journal, Limits},
    write_fields, Fields,
};

struct EvidenceDir(Option<tempfile::TempDir>);
impl EvidenceDir {
    fn new() -> Self {
        Self(Some(tempfile::tempdir().unwrap()))
    }
    fn path(&self) -> &Path {
        self.0.as_ref().unwrap().path()
    }
}
impl Drop for EvidenceDir {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "Retained restart regression fixture: {}",
                self.0.take().unwrap().keep().display()
            );
        }
    }
}
fn key(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}
fn wav(path: &Path) {
    let samples: Vec<u8> = (0..8000)
        .flat_map(|i| ((i % 251) as i16 * 31).to_le_bytes())
        .collect();
    let mut b = b"RIFF".to_vec();
    b.extend((36 + samples.len() as u32).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(8000u32.to_le_bytes());
    b.extend(16000u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((samples.len() as u32).to_le_bytes());
    b.extend(samples);
    fs::write(path, &b).unwrap();
    fs::write(path.with_extension("independent-input"), b).unwrap();
}

fn external_pcm_edit(path: &Path) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};
    let metadata = fs::metadata(path).unwrap();
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.seek(SeekFrom::Start(44)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    file.seek(SeekFrom::Start(44)).unwrap();
    file.write_all(&[byte[0] ^ 1]).unwrap();
    file.sync_all().unwrap();
    file.set_modified(metadata.modified().unwrap()).unwrap();
    let bytes = fs::read(path).unwrap();
    fs::write(path.with_extension("independent-external"), &bytes).unwrap();
    bytes
}

fn pcm(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let length = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(at + 8 + length <= bytes.len());
        if &bytes[at..at + 4] == b"data" {
            return bytes[at + 8..at + 8 + length].to_vec();
        }
        at += 8 + length + length % 2;
    }
    panic!("Independent WAV observer did not find PCM");
}
fn open(root: &Path) -> Journal {
    Journal::open(root, Limits::default()).unwrap()
}
fn save(journal: &mut Journal, audio: &Path, title: &str) -> Vec<u8> {
    let capture = journal.begin("Save tags", None, &[key(audio)]);
    let fields: Fields = [("TITLE".into(), vec![title.into()])].into();
    write_fields(audio, &fields).unwrap();
    journal.commit(capture, &[], &[]);
    assert!(journal.state().notice.is_none(), "{:?}", journal.state());
    assert!(journal.state().undo.is_some());
    let bytes = fs::read(audio).unwrap();
    fs::write(audio.with_extension(format!("independent-{title}")), &bytes).unwrap();
    bytes
}

#[test]
fn committed_save_and_cover_outputs_survive_reopening() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let original = fs::read(&audio).unwrap();
    let root = dir.path().join("history");
    let cover = dir.path().join("existing-cover.bin");
    let added = dir.path().join("added-cover.bin");
    fs::write(&cover, b"Independent existing cover bytes").unwrap();
    let mut j = open(&root);
    let mut c = j.begin("Save with cover outputs", None, &[key(&audio)]);
    j.capture_side_file(&mut c, &cover).unwrap();
    j.capture_side_file(&mut c, &added).unwrap();
    let fields: Fields = [("TITLE".into(), vec!["Saved title".into()])].into();
    write_fields(&audio, &fields).unwrap();
    fs::write(&cover, b"New cover").unwrap();
    fs::write(&added, b"Added cover").unwrap();
    j.commit(c, &[], &[]);
    assert!(j.state().notice.is_none());
    let saved = fs::read(&audio).unwrap();
    fs::write(audio.with_extension("independent-saved"), &saved).unwrap();
    drop(j);
    let mut j = open(&root);
    assert_eq!(
        j.state().undo.as_ref().map(|s| s.label.as_str()),
        Some("Save with cover outputs")
    );
    let replay = j.undo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(fs::read(&audio).unwrap(), original);
    assert_eq!(
        fs::read(&cover).unwrap(),
        b"Independent existing cover bytes"
    );
    assert!(!added.exists());
    drop(j);
    let mut j = open(&root);
    let replay = j.redo().unwrap();
    assert!(replay.errors.is_empty(), "{:?}", replay.errors);
    assert_eq!(fs::read(&audio).unwrap(), saved);
    assert_eq!(fs::read(&cover).unwrap(), b"New cover");
    assert_eq!(fs::read(&added).unwrap(), b"Added cover");
}

#[test]
fn redo_keeps_exact_saved_bytes_after_restart() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let original = fs::read(&audio).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved = save(&mut j, &audio, "saved");
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), original);
    drop(j);
    let mut j = open(&root);
    assert_eq!(
        j.state().redo.as_ref().map(|s| s.label.as_str()),
        Some("Save tags")
    );
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), saved);
}

#[test]
fn repeated_reopen_keeps_all_restored_file_identity_epochs() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let original = fs::read(&audio).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let one = save(&mut j, &audio, "one");
    let two = save(&mut j, &audio, "two");
    drop(j);
    for expected in [&one, &original] {
        let mut j = open(&root);
        assert!(j.undo().unwrap().errors.is_empty());
        assert_eq!(&fs::read(&audio).unwrap(), expected);
    }
    for expected in [&one, &two] {
        let mut j = open(&root);
        assert!(j.redo().unwrap().errors.is_empty());
        assert_eq!(&fs::read(&audio).unwrap(), expected);
    }
}

#[test]
fn pure_rename_reopens_without_rewriting_the_audio() {
    let dir = EvidenceDir::new();
    let before = dir.path().join("before.wav");
    let after = dir.path().join("after.wav");
    wav(&before);
    let original = fs::read(&before).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let c = j.begin("Rename file", None, &[key(&before)]);
    fs::rename(&before, &after).unwrap();
    j.commit(c, &[(key(&before), key(&after))], &[]);
    drop(j);
    let mut j = open(&root);
    assert!(j.undo().unwrap().errors.is_empty());
    assert!(!after.exists());
    assert_eq!(fs::read(&before).unwrap(), original);
    drop(j);
    let mut j = open(&root);
    assert!(j.redo().unwrap().errors.is_empty());
    assert!(!before.exists());
    assert_eq!(fs::read(&after).unwrap(), original);
}

#[test]
fn a_second_writer_cannot_open_the_same_durable_store() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let first = open(&root);
    assert!(
        Journal::open(&root, Limits::default()).is_err(),
        "Concurrent writers must not publish competing operation indexes"
    );
    drop(first);
    let reopened = Journal::open(&root, Limits::default());
    assert!(
        reopened.is_ok(),
        "Writer release did not reopen: {:?}",
        reopened.err()
    );
}

#[test]
fn dropping_a_capture_releases_its_write_guard_but_keeps_the_interruption_intent() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let c = j.begin("Abandoned", None, &[key(&audio)]);
    external_pcm_edit(&audio);
    let expected_pcm = pcm(&audio);
    drop(c);
    let result = write_fields(
        &audio,
        &Fields::from([("TITLE".into(), vec!["Later unjournaled save".into()])]),
    );
    assert!(
        result.is_ok(),
        "A dropped capture kept a stale staged-write guard: {result:?}"
    );
    assert_eq!(pcm(&audio), expected_pcm);
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("session-store/index.json")).unwrap()).unwrap();
    assert!(index["pending"] == true || index["pending"].is_object());
}

#[test]
fn dropping_a_journal_releases_only_its_own_capture_watchers() {
    let dir = EvidenceDir::new();
    let one = dir.path().join("one.wav");
    let two = dir.path().join("two.wav");
    wav(&one);
    wav(&two);
    let mut first = open(&dir.path().join("first-history"));
    let mut second = open(&dir.path().join("second-history"));
    let first_capture = first.begin("First", None, &[key(&one)]);
    let second_capture = second.begin("Second", None, &[key(&two)]);
    drop(first);
    external_pcm_edit(&one);
    let expected_one = pcm(&one);
    let external_two = external_pcm_edit(&two);
    let fields = Fields::from([("TITLE".into(), vec!["Later".into()])]);
    let result = write_fields(&one, &fields);
    assert!(
        result.is_ok(),
        "A dropped journal kept its capture guard: {result:?}"
    );
    assert_eq!(pcm(&one), expected_one);
    drop(first_capture);
    assert!(
        write_fields(&two, &fields).is_err(),
        "Another journal's active guard was removed"
    );
    assert!(fs::read(&two).unwrap() == external_two);
    drop(second_capture);
    assert!(write_fields(&two, &fields).is_ok());
}

#[test]
fn failed_replay_preparation_reports_unavailable_history_without_touching_audio() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved = save(&mut j, &audio, "saved");
    let index = root.join("session-store/index.json");
    let held = root.join("session-store/independent-index-before-prepare.json");
    fs::rename(&index, &held).unwrap();
    fs::create_dir(&index).unwrap();
    assert!(j.undo().is_err());
    assert!(fs::read(&audio).unwrap() == saved);
    let state = j.state();
    assert!(
        state.notice.is_some() && state.undo.is_none() && state.redo.is_none(),
        "Failed preparation still offered history as if ready: {state:?}"
    );
    assert!(held.is_file());
}

#[cfg(unix)]
#[test]
fn replacing_the_lock_leaf_does_not_admit_a_competing_writer() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let first = open(&root);
    let store = root.join("session-store");
    fs::rename(
        store.join(".session-lock"),
        store.join("independent-held-lock"),
    )
    .unwrap();
    fs::write(store.join(".session-lock"), b"").unwrap();
    assert!(
        Journal::open(&root, Limits::default()).is_err(),
        "A replacement lock leaf bypassed the live owner's lock"
    );
    drop(first);
}

#[test]
fn snapshot_collision_never_overwrites_an_existing_hard_link() {
    let dir = EvidenceDir::new();
    let one = dir.path().join("one.wav");
    let two = dir.path().join("two.wav");
    wav(&one);
    wav(&two);
    let original = fs::read(&one).unwrap();
    let mut different = fs::read(&two).unwrap();
    different[44] ^= 1;
    fs::write(&two, &different).unwrap();
    fs::write(two.with_extension("independent-different"), &different).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let snapshot = root.join("session-store/1.snapshot");
    fs::hard_link(&one, &snapshot).unwrap();
    let capture = j.begin("Save tags", None, &[key(&two)]);
    assert!(
        fs::read(&one).unwrap() == original,
        "Colliding snapshot allocation overwrote another live file"
    );
    assert!(fs::read(&snapshot).unwrap() == original);
    j.commit(capture, &[], &[]);
    assert!(j.state().notice.is_some());
    assert!(fs::read(&one).unwrap() == original);
}

#[test]
fn swapped_audio_blobs_are_refused_before_any_audio_is_restored() {
    let dir = EvidenceDir::new();
    let one = dir.path().join("one.wav");
    let two = dir.path().join("two.wav");
    wav(&one);
    wav(&two);
    let mut different = fs::read(&two).unwrap();
    different[44] ^= 1;
    fs::write(&two, &different).unwrap();
    fs::write(two.with_extension("independent-different"), &different).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved_one = save(&mut j, &one, "one");
    let saved_two = save(&mut j, &two, "two");
    drop(j);
    let index = root.join("session-store/index.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
    let a = value["undo"][0]["files"][0]["before"]["blob"].clone();
    let b = value["undo"][1]["files"][0]["before"]["blob"].clone();
    value["undo"][0]["files"][0]["before"]["blob"] = b;
    value["undo"][1]["files"][0]["before"]["blob"] = a;
    fs::write(&index, serde_json::to_vec(&value).unwrap()).unwrap();
    if let Ok(mut j) = Journal::open(&root, Limits::default()) {
        let _ = j.undo();
        assert!(
            fs::read(&two).unwrap() == saved_two,
            "An admitted mismatched audio blob overwrote the other track's data"
        );
        panic!("A mismatched audio snapshot was admitted");
    }
    assert!(fs::read(&one).unwrap() == saved_one && fs::read(&two).unwrap() == saved_two);
}

#[test]
fn empty_records_are_rejected_instead_of_becoming_zero_file_undo_steps() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    drop(open(&root));
    let index = root.join("session-store/index.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
    value["undo"] = serde_json::json!([{"label":"Empty step","group":null,"files":[],"directories":[],"side_files":[]}]);
    fs::write(&index, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(Journal::open(&root, Limits::default()).is_err());
}

#[test]
fn lazy_redo_snapshots_stay_within_the_same_byte_budget_after_undo_and_reopen() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let original = fs::read(&audio).unwrap();
    let limit = original.len() as u64 + 1000;
    let limits = Limits {
        bytes: limit,
        ..Limits::default()
    };
    let root = dir.path().join("history");
    let mut j = Journal::open(&root, limits).unwrap();
    save(&mut j, &audio, "saved");
    assert!(j.retained_bytes() <= limit);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), original);
    assert!(
        j.retained_bytes() <= limit,
        "Lazy reverse snapshots exceeded the retained byte cap: {} > {limit}",
        j.retained_bytes()
    );
    drop(j);
    let j = Journal::open(&root, limits).unwrap();
    assert!(j.retained_bytes() <= limit);
}

#[test]
fn a_cloned_shrink_never_charges_more_than_the_complete_reverse_image() {
    use std::io::Write;
    let dir = EvidenceDir::new();
    if !tag_core::clones_in(dir.path()) {
        eprintln!("Cloned shrinking replay needs a clone-capable local filesystem");
        return;
    }
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let after = fs::read(&audio).unwrap();
    let mut before = after.clone();
    before.extend(b"JUNK");
    before.extend(65536u32.to_le_bytes());
    before.extend(std::iter::repeat_n(0x5a, 65536));
    let riff_size = u32::try_from(before.len() - 8).unwrap();
    before[4..8].copy_from_slice(&riff_size.to_le_bytes());
    fs::write(&audio, &before).unwrap();
    fs::write(audio.with_extension("independent-large"), &before).unwrap();
    assert!(tag_core::read_entry(&audio).is_ok());
    let root = dir.path().join("history");
    let mut j = open(&root);
    let c = j.begin("Whole-file shrink", None, &[key(&audio)]);
    let mut staging = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
    staging.write_all(&after).unwrap();
    staging.as_file().sync_all().unwrap();
    staging.persist(&audio).unwrap();
    j.commit(c, &[], &[]);
    drop(j);
    let index = root.join("session-store/index.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
    assert_eq!(
        value["undo"][0]["files"][0]["before"]["blob"]["cost"],
        before.len() as u64
    );
    assert!(value["undo"][0]["files"][0]["after"]["blob"].is_null());
    value["undo"][0]["files"][0]["cost"] = (20u64 * 1024).into();
    fs::write(&index, serde_json::to_vec(&value).unwrap()).unwrap();
    let mut j = open(&root);
    assert!(j.undo().unwrap().errors.is_empty());
    assert!(fs::read(&audio).unwrap() == before);
    assert!(
        j.retained_bytes() <= (before.len() + after.len()) as u64,
        "Reverse cost exceeds the complete independent file images"
    );
    drop(j);
    let mut j = open(&root);
    assert!(j.redo().unwrap().errors.is_empty());
    assert!(fs::read(&audio).unwrap() == after);
}

#[test]
fn committing_a_refused_overlapping_capture_cannot_clear_the_first_intent_or_blob() {
    let dir = EvidenceDir::new();
    let one = dir.path().join("one.wav");
    let two = dir.path().join("two.wav");
    wav(&one);
    wav(&two);
    let original = fs::read(&one).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let first = j.begin("First save", None, &[key(&one)]);
    let fields: Fields = [("TITLE".into(), vec!["First saved".into()])].into();
    write_fields(&one, &fields).unwrap();
    let refused = j.begin("Overlapping save", None, &[key(&two)]);
    j.commit(refused, &[], &[]);
    let store = root.join("session-store");
    assert!(
        store.join("1.snapshot").is_file(),
        "The active operation's original bytes were collected"
    );
    assert_eq!(fs::read(store.join("1.snapshot")).unwrap(), original);
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(store.join("index.json")).unwrap()).unwrap();
    assert!(index["pending"].is_object());
    j.commit(first, &[], &[]);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&one).unwrap(), original);
}

#[test]
fn index_publication_failure_keeps_previous_and_new_recovery_bytes() {
    let dir = EvidenceDir::new();
    let one = dir.path().join("one.wav");
    let two = dir.path().join("two.wav");
    wav(&one);
    wav(&two);
    let original_one = fs::read(&one).unwrap();
    let original_two = fs::read(&two).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved_one = save(&mut j, &one, "first");
    let c = j.begin("Second save", None, &[key(&two)]);
    let store = root.join("session-store");
    let index = store.join("index.json");
    let frozen_index = store.join("independent-index-before-failure.json");
    let before_index = fs::read(&index).unwrap();
    fs::rename(&index, &frozen_index).unwrap();
    fs::create_dir(&index).unwrap();
    let fields: Fields = [("TITLE".into(), vec!["Second saved title".into()])].into();
    write_fields(&two, &fields).unwrap();
    let saved_two = fs::read(&two).unwrap();
    j.commit(c, &[], &[]);
    assert!(j.state().notice.is_some());
    assert!(j.state().undo.is_none() && j.state().redo.is_none());
    assert_eq!(fs::read(&frozen_index).unwrap(), before_index);
    let snapshots: Vec<_> = fs::read_dir(&store)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "snapshot"))
        .map(|path| fs::read(path).unwrap())
        .collect();
    assert!(snapshots.contains(&original_one));
    assert!(snapshots.contains(&original_two));
    assert_eq!(fs::read(&two).unwrap(), saved_two);
    fs::remove_dir(&index).unwrap();
    fs::rename(&frozen_index, &index).unwrap();
    drop(j);
    let recovered = open(&root);
    assert!(recovered.state().undo.is_none() && recovered.state().redo.is_none());
    assert!(recovered.state().notice.is_some());
    assert_eq!(recovered.state().recovery.unwrap().copies, 2);
    assert_eq!(fs::read(&one).unwrap(), saved_one);
    assert_eq!(fs::read(&two).unwrap(), saved_two);
    let retained: Vec<_> = fs::read_dir(&store)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "snapshot")
        })
        .map(|path| fs::read(path).unwrap())
        .collect();
    assert!(retained.contains(&original_one));
    assert!(retained.contains(&original_two));
}

#[cfg(unix)]
#[test]
fn replacing_the_live_store_with_a_symlink_never_writes_or_collects_there() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    save(&mut j, &audio, "first");
    let store = root.join("session-store");
    let retained = root.join("retained-original-store");
    fs::rename(&store, &retained).unwrap();
    let unrelated = dir.path().join("unrelated-cache");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("index.json"), b"Independent unrelated index").unwrap();
    fs::write(
        unrelated.join("99.snapshot"),
        b"Independent unrelated snapshot",
    )
    .unwrap();
    // Keep the prior blob's actual identity, defeating metadata-only checks of
    // that blob. The directory entry itself must still be rejected.
    fs::hard_link(retained.join("1.snapshot"), unrelated.join("1.snapshot")).unwrap();
    std::os::unix::fs::symlink(&unrelated, &store).unwrap();
    let c = j.begin("Save tags", None, &[key(&audio)]);
    assert_eq!(
        fs::read(unrelated.join("index.json")).unwrap(),
        b"Independent unrelated index"
    );
    assert_eq!(fs::read_dir(&unrelated).unwrap().count(), 3);
    let fields: Fields = [("TITLE".into(), vec!["Second".into()])].into();
    write_fields(&audio, &fields).unwrap();
    j.commit(c, &[], &[]);
    assert!(j.state().notice.is_some());
    assert_eq!(
        fs::read(unrelated.join("99.snapshot")).unwrap(),
        b"Independent unrelated snapshot"
    );
    assert_eq!(
        fs::read(unrelated.join("index.json")).unwrap(),
        b"Independent unrelated index"
    );
    assert_eq!(fs::read_dir(&unrelated).unwrap().count(), 3);
}

#[cfg(unix)]
#[test]
fn an_audio_parent_replaced_by_a_symlink_refuses_replay_without_writing() {
    let dir = EvidenceDir::new();
    let folder = dir.path().join("album");
    let moved = dir.path().join("moved-outside-the-recorded-location");
    fs::create_dir(&folder).unwrap();
    let audio = folder.join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved = save(&mut j, &audio, "saved");
    fs::rename(&folder, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &folder).unwrap();
    let report = j.undo().unwrap();
    assert_eq!(fs::read(moved.join("audio.wav")).unwrap(), saved);
    assert!(report.paths.is_empty());
    assert_eq!(report.errors.len(), 1);
}

#[test]
fn pre_epoch_snapshot_timestamps_survive_the_index() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let time = std::time::UNIX_EPOCH
        .checked_sub(std::time::Duration::from_secs(60))
        .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&audio)
        .unwrap()
        .set_modified(time)
        .unwrap();
    let original = fs::read(&audio).unwrap();
    let root = dir.path().join("history");
    let mut j = open(&root);
    save(&mut j, &audio, "saved");
    drop(j);
    let mut j = open(&root);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), original);
}

#[test]
fn loaded_index_rejects_invalid_ids_costs_versions_and_epochs() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let mut c = j.begin("Audio and side", None, &[key(&audio)]);
    let side = dir.path().join("cover.bin");
    j.capture_side_file(&mut c, &side).unwrap();
    let fields: Fields = [("TITLE".into(), vec!["Saved".into()])].into();
    write_fields(&audio, &fields).unwrap();
    fs::write(&side, b"Independent side-file bytes").unwrap();
    j.commit(c, &[], &[]);
    drop(j);
    let saved = fs::read(&audio).unwrap();
    let index = root.join("session-store/index.json");
    let original = fs::read(&index).unwrap();
    let clean: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let mut cases = Vec::new();
    let mut bad = clean.clone();
    bad["version"] = 999.into();
    cases.push(bad);
    let mut bad = clean.clone();
    bad["platform"] = "Unknown platform".into();
    cases.push(bad);
    let mut bad = clean.clone();
    bad["undo"][0]["files"][0]["before"]["blob"]["path"] = "../../escape.snapshot".into();
    cases.push(bad);
    let mut bad = clean.clone();
    bad["undo"][0]["files"][0]["before"]["blob"]["cost"] = 0.into();
    cases.push(bad);
    let mut bad = clean.clone();
    bad["undo"][0]["files"][0]["cost"] = 0.into();
    cases.push(bad);
    let mut bad = clean.clone();
    bad["undo"][0]["side_files"][0]["epoch"] = 100.into();
    cases.push(bad);
    let mut bad = clean.clone();
    let duplicate = bad["undo"][0]["files"][0].clone();
    bad["undo"][0]["files"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    cases.push(bad);
    for (i, bad) in cases.into_iter().enumerate() {
        let bytes = serde_json::to_vec(&bad).unwrap();
        fs::write(&index, &bytes).unwrap();
        assert!(
            Journal::open(&root, Limits::default()).is_err(),
            "Invalid index case {i} admitted"
        );
        assert_eq!(
            fs::read(&index).unwrap(),
            bytes,
            "Invalid input must be preserved"
        );
        assert_eq!(fs::read(&audio).unwrap(), saved);
    }
    fs::write(&index, original).unwrap();
    assert!(open(&root).state().undo.is_some());
}

#[test]
fn an_exhausted_snapshot_counter_is_refused_without_panicking_or_writing_audio() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved = save(&mut j, &audio, "saved");
    drop(j);
    let index = root.join("session-store/index.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
    value["next"] = u64::MAX.into();
    fs::write(&index, serde_json::to_vec(&value).unwrap()).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Ok(mut j) = Journal::open(&root, Limits::default()) {
            let c = j.begin("Save", None, &[key(&audio)]);
            j.commit(c, &[], &[]);
            assert!(j.state().notice.is_some());
        }
    }));
    assert!(result.is_ok(), "An invalid counter must not panic");
    assert_eq!(fs::read(&audio).unwrap(), saved);
}

#[test]
fn an_interrupted_capture_is_visible_and_never_replayed_automatically() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let capture = j.begin("Interrupted save", None, &[key(&audio)]);
    let fields: Fields = [("TITLE".into(), vec!["Uncommitted result".into()])].into();
    write_fields(&audio, &fields).unwrap();
    let uncommitted = fs::read(&audio).unwrap();
    drop(capture);
    drop(j);
    match Journal::open(&root, Limits::default()) {
        Ok(j) => {
            assert!(
                j.state().notice.is_some(),
                "An interrupted operation must be reported"
            );
            assert!(j.state().undo.is_none() && j.state().redo.is_none());
        }
        Err(error) => assert!(error.to_lowercase().contains("interrupt"), "{error}"),
    }
    assert_eq!(fs::read(&audio).unwrap(), uncommitted);
}

#[test]
fn corrupt_index_is_refused_without_deleting_evidence() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let store = root.join("session-store");
    fs::create_dir_all(&store).unwrap();
    let index = store.join("index.json");
    let bad = b"{\"version\":1,\"undo\":[truncated";
    fs::write(&index, bad).unwrap();
    assert!(Journal::open(&root, Limits::default()).is_err());
    assert_eq!(fs::read(&index).unwrap(), bad);
}

#[cfg(unix)]
#[test]
fn index_symlink_is_refused_without_following_it() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let store = root.join("session-store");
    fs::create_dir_all(&store).unwrap();
    let outside = dir.path().join("unrelated-index");
    fs::write(&outside, b"Unrelated bytes").unwrap();
    std::os::unix::fs::symlink(&outside, store.join("index.json")).unwrap();
    assert!(Journal::open(&root, Limits::default()).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"Unrelated bytes");
    assert!(fs::symlink_metadata(store.join("index.json"))
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn durable_store_symlink_is_refused_without_following_it() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    fs::create_dir(&root).unwrap();
    let outside = dir.path().join("unrelated-folder");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"Unrelated bytes").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("session-store")).unwrap();
    assert!(Journal::open(&root, Limits::default()).is_err());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"Unrelated bytes");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn side_link_native_target_bytes_survive_restart() {
    use std::os::unix::ffi::OsStringExt;
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let cover = dir.path().join("cover-link");
    // APFS rejects non-UTF8 leaf names, but a symlink's literal target is data.
    // Keep relative, dangling and non-UTF8 target bytes without following them.
    let before = std::path::PathBuf::from(std::ffi::OsString::from_vec(
        b"../covers/before-\xff.png".to_vec(),
    ));
    let after = std::path::PathBuf::from(std::ffi::OsString::from_vec(
        b"../covers/after-\xfe.png".to_vec(),
    ));
    std::os::unix::fs::symlink(&before, &cover).unwrap();
    let mut j = open(&root);
    let mut c = j.begin("Native path", None, &[key(&audio)]);
    j.capture_side_file(&mut c, &cover).unwrap();
    let fields: Fields = [("TITLE".into(), vec!["New title".into()])].into();
    write_fields(&audio, &fields).unwrap();
    fs::remove_file(&cover).unwrap();
    std::os::unix::fs::symlink(&after, &cover).unwrap();
    j.commit(c, &[], &[]);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read_link(&cover).unwrap(), before);
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read_link(&cover).unwrap(), after);
    eprintln!("NATIVE_LINK_IN_SESSION_CONTROL passed");
    drop(j);
    let mut j = open(&root);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read_link(&cover).unwrap(), before);
    drop(j);
    let mut j = open(&root);
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read_link(&cover).unwrap(), after);
}

#[test]
fn a_replaced_audio_parent_is_refused_even_when_the_file_inode_is_preserved() {
    let dir = EvidenceDir::new();
    let album = dir.path().join("album");
    fs::create_dir(&album).unwrap();
    let audio = album.join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let mut j = open(&root);
    let saved = save(&mut j, &audio, "saved");
    drop(j);
    let external = dir.path().join("external-album");
    fs::rename(&album, &external).unwrap();
    fs::create_dir(&album).unwrap();
    fs::rename(external.join("audio.wav"), &audio).unwrap();
    let mut j = open(&root);
    let result = j.undo().unwrap();
    assert!(
        !result.errors.is_empty(),
        "Undo accepted a replaced parent directory"
    );
    assert_eq!(fs::read(&audio).unwrap(), saved);
    assert!(album.is_dir());
    assert!(external.is_dir());
}

#[test]
fn a_replaced_created_side_parent_is_not_removed_by_undo() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let root = dir.path().join("history");
    let parent = dir.path().join("covers");
    let side = parent.join("cover.bin");
    let mut j = open(&root);
    let mut c = j.begin("Export cover", None, &[key(&audio)]);
    j.capture_side_file(&mut c, &side).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::write(&side, b"Independent cover bytes").unwrap();
    j.commit(c, &[], &[]);
    drop(j);
    let external = dir.path().join("external-covers");
    fs::rename(&parent, &external).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::rename(external.join("cover.bin"), &side).unwrap();
    let mut j = open(&root);
    let result = j.undo().unwrap();
    assert!(
        !result.errors.is_empty(),
        "Undo deleted an outside replacement folder"
    );
    assert_eq!(fs::read(&side).unwrap(), b"Independent cover bytes");
    assert!(parent.is_dir());
}

#[test]
fn redo_refuses_an_outside_created_folder_instead_of_adopting_it() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let parent = dir.path().join("covers");
    let side = parent.join("cover.bin");
    let mut j = open(&root);
    let mut c = j.begin("Export cover", None, &[]);
    j.capture_side_file(&mut c, &side).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::write(&side, b"Cover").unwrap();
    j.commit(c, &[], &[]);
    assert!(j.undo().unwrap().errors.is_empty());
    assert!(!parent.exists());
    drop(j);
    fs::create_dir(&parent).unwrap();
    let mut j = open(&root);
    let result = j.redo().unwrap();
    assert!(
        !result.errors.is_empty(),
        "Redo adopted a folder created outside the app"
    );
    assert!(!side.exists());
    assert!(parent.is_dir());
}

#[test]
fn an_empty_directory_move_refuses_a_replaced_source_on_reopen() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let before = dir.path().join("before");
    let after = dir.path().join("after");
    fs::create_dir(&before).unwrap();
    let mut j = open(&root);
    let c = j.begin("Move folder", None, &[]);
    fs::rename(&before, &after).unwrap();
    j.commit(c, &[], &[(key(&before), key(&after))]);
    drop(j);
    fs::rename(&after, dir.path().join("external-folder")).unwrap();
    fs::create_dir(&after).unwrap();
    let mut j = open(&root);
    let result = j.undo().unwrap();
    assert!(
        !result.errors.is_empty(),
        "Undo moved a replacement directory"
    );
    assert!(!before.exists());
    assert!(after.is_dir());
}

#[test]
fn side_parent_identities_follow_owned_recreation_across_restarts() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let parent = dir.path().join("covers").join("nested");
    let side = parent.join("cover.bin");
    let mut j = open(&root);
    let mut c = j.begin("Export cover", None, &[]);
    j.capture_side_file(&mut c, &side).unwrap();
    fs::create_dir_all(&parent).unwrap();
    fs::write(&side, b"Cover").unwrap();
    j.commit(c, &[], &[]);
    drop(j);
    for _ in 0..3 {
        let mut j = open(&root);
        assert!(j.undo().unwrap().errors.is_empty());
        assert!(!parent.exists());
        drop(j);
        let mut j = open(&root);
        assert!(j.redo().unwrap().errors.is_empty());
        assert_eq!(fs::read(&side).unwrap(), b"Cover");
    }
}

#[test]
fn durable_capture_child_process() {
    let Some(root) = std::env::var_os("TAGGROOVE_HISTORY_CAPTURE_CHILD_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let audio = root.join("audio.wav");
    let cover = root.join("cover.bin");
    let mut j = open(&root.join("history"));
    let mut capture = j.begin("Interrupted save and cover", None, &[key(&audio)]);
    j.capture_side_file(&mut capture, &cover).unwrap();
    write_fields(
        &audio,
        &Fields::from([("TITLE".into(), vec!["Interrupted result".into()])]),
    )
    .unwrap();
    fs::write(&cover, b"Interrupted new cover").unwrap();
    // Exit a real child process without running Capture or Journal destructors.
    // This exercises process death, not a hardware power-loss guarantee.
    std::process::exit(91);
}

#[test]
fn abrupt_capture_keeps_current_results_and_allows_new_history_after_reopening() {
    let dir = EvidenceDir::new();
    let audio = dir.path().join("audio.wav");
    let cover = dir.path().join("cover.bin");
    wav(&audio);
    let original = fs::read(&audio).unwrap();
    fs::write(&cover, b"Original cover").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "durable_capture_child_process",
            "--test-threads=1",
        ])
        .env("TAGGROOVE_HISTORY_CAPTURE_CHILD_ROOT", dir.path())
        .output()
        .unwrap();
    fs::write(dir.path().join("child-stdout.log"), &result.stdout).unwrap();
    fs::write(dir.path().join("child-stderr.log"), &result.stderr).unwrap();
    assert_eq!(
        result.status.code(),
        Some(91),
        "Child did not reach the intended interruption boundary"
    );
    let interrupted = fs::read(&audio).unwrap();
    fs::write(
        dir.path().join("independent-interrupted-audio"),
        &interrupted,
    )
    .unwrap();
    assert_ne!(interrupted, original);
    let root = dir.path().join("history");
    let mut j = Journal::open(&root, Limits::default()).unwrap_or_else(|error| {
        panic!("Interrupted history left all later Undo permanently unavailable: {error}")
    });
    assert!(j.state().notice.is_some());
    assert!(j.state().undo.is_none() && j.state().redo.is_none());
    assert_eq!(fs::read(&audio).unwrap(), interrupted);
    assert_eq!(fs::read(&cover).unwrap(), b"Interrupted new cover");
    let fresh = save(&mut j, &audio, "Fresh save");
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), interrupted);
    drop(j);
    let mut j = open(&root);
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), fresh);
    let blobs: Vec<Vec<u8>> = fs::read_dir(root.join("session-store"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "snapshot"))
        .map(|path| fs::read(path).unwrap())
        .collect();
    assert!(
        blobs.contains(&original),
        "Fresh history garbage-collected the interrupted audio recovery copy"
    );
    assert!(
        blobs.iter().any(|bytes| bytes == b"Original cover"),
        "Fresh history garbage-collected the interrupted cover recovery copy"
    );
}

#[test]
fn interrupted_recovery_respects_a_smaller_reopen_byte_limit() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let original = fs::read(&audio).unwrap();
    let mut j = open(&root);
    let capture = j.begin("Interrupted save", None, &[key(&audio)]);
    write_fields(
        &audio,
        &Fields::from([("TITLE".into(), vec!["Interrupted".into()])]),
    )
    .unwrap();
    let current = fs::read(&audio).unwrap();
    drop(capture);
    drop(j);
    let j = Journal::open(
        &root,
        Limits {
            bytes: original.len() as u64 - 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(j.retained_bytes(), 0);
    assert!(j.state().recovery.is_none());
    assert!(j.state().notice.is_some());
    assert_eq!(fs::read(&audio).unwrap(), current);
    assert!(!fs::read_dir(root.join("session-store"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .any(|path| path
            .extension()
            .is_some_and(|extension| extension == "snapshot")));
}

#[test]
fn recovery_and_new_undo_share_the_same_operation_limit() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let mut j = open(&root);
    let capture = j.begin("Interrupted save", None, &[key(&audio)]);
    write_fields(
        &audio,
        &Fields::from([("TITLE".into(), vec!["Interrupted".into()])]),
    )
    .unwrap();
    let interrupted = fs::read(&audio).unwrap();
    drop(capture);
    drop(j);
    let mut j = Journal::open(
        &root,
        Limits {
            operations: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(j.state().recovery.unwrap().operations, 1);
    let capture = j.begin("New save", None, &[key(&audio)]);
    write_fields(
        &audio,
        &Fields::from([("TITLE".into(), vec!["New saved title".into()])]),
    )
    .unwrap();
    j.commit(capture, &[], &[]);
    assert!(j.state().undo.is_some());
    assert!(
        j.state().recovery.is_none(),
        "Recovery and Undo silently exceeded the one-operation cap"
    );
    assert!(
        j.state().notice.is_some(),
        "Recovery eviction was not reported"
    );
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&audio).unwrap(), interrupted);
}

#[test]
fn recovery_export_copies_preimages_without_overwriting_or_consuming_them() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let audio = dir.path().join("audio.wav");
    let cover = dir.path().join("cover.bin");
    wav(&audio);
    let original_audio = fs::read(&audio).unwrap();
    fs::write(&cover, b"Independent original cover").unwrap();
    let mut j = open(&root);
    let mut capture = j.begin("Interrupted save with cover", None, &[key(&audio)]);
    j.capture_side_file(&mut capture, &cover).unwrap();
    write_fields(
        &audio,
        &Fields::from([("TITLE".into(), vec!["Interrupted title".into()])]),
    )
    .unwrap();
    fs::write(&cover, b"Current cover").unwrap();
    let current_audio = fs::read(&audio).unwrap();
    drop(capture);
    drop(j);
    let j = open(&root);
    let summary = j.state().recovery.unwrap();
    let destination = dir.path().join("exports");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("user-file.bin"), b"User-owned file").unwrap();
    let exported = j.export_recovery(&destination).unwrap();
    assert!(exported.is_dir());
    // The exporter uses a compatible Windows path; std canonicalization can
    // add a verbatim prefix. Compare physical locations in the same form.
    let canonical_export = fs::canonicalize(&exported).unwrap();
    let canonical_destination = fs::canonicalize(&destination).unwrap();
    assert_eq!(
        canonical_export.parent(),
        Some(canonical_destination.as_path())
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(exported.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["version"], 1);
    assert_eq!(
        manifest["operations"][0]["label"],
        "Interrupted save with cover"
    );
    let entries = manifest["operations"][0]["entries"].as_array().unwrap();
    let mut output_bytes = Vec::new();
    for entry in entries {
        let name = entry["copy"].as_str().unwrap();
        assert_eq!(Path::new(name).components().count(), 1);
        let path = exported.join(name);
        assert!(fs::symlink_metadata(&path).unwrap().file_type().is_file());
        output_bytes.push(fs::read(path).unwrap());
    }
    assert_eq!(output_bytes.len(), 2);
    assert!(output_bytes.contains(&original_audio));
    assert!(output_bytes
        .iter()
        .any(|bytes| bytes == b"Independent original cover"));
    let again = j.export_recovery(&destination).unwrap();
    assert_ne!(again, exported);
    assert_eq!(j.state().recovery.unwrap(), summary);
    assert_eq!(fs::read(&audio).unwrap(), current_audio);
    assert_eq!(fs::read(&cover).unwrap(), b"Current cover");
    assert_eq!(
        fs::read(destination.join("user-file.bin")).unwrap(),
        b"User-owned file"
    );
    // A later in-place cache edit must not be exported as verified recovery.
    let blob = fs::read_dir(root.join("session-store"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "snapshot")
                && fs::read(path).unwrap() == b"Independent original cover"
        })
        .unwrap();
    let stamp = fs::metadata(&blob).unwrap().modified().unwrap();
    fs::write(&blob, b"Independent modified cover").unwrap();
    fs::File::options()
        .write(true)
        .open(&blob)
        .unwrap()
        .set_modified(stamp)
        .unwrap();
    assert!(j.export_recovery(&destination).is_err());
    assert_eq!(fs::read(&audio).unwrap(), current_audio);
    assert_eq!(fs::read(&cover).unwrap(), b"Current cover");
}

#[cfg(unix)]
#[test]
fn recovery_export_preserves_native_link_target_data_without_creating_links() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let side = dir.path().join("cover-link");
    let target = std::path::PathBuf::from(std::ffi::OsString::from_vec(
        b"../missing-\xff-cover".to_vec(),
    ));
    std::os::unix::fs::symlink(&target, &side).unwrap();
    let mut j = open(&root);
    let mut capture = j.begin("Interrupted link replacement", None, &[]);
    j.capture_side_file(&mut capture, &side).unwrap();
    fs::remove_file(&side).unwrap();
    fs::write(&side, b"Current regular cover").unwrap();
    drop(capture);
    drop(j);
    let j = open(&root);
    let destination = dir.path().join("exports");
    fs::create_dir(&destination).unwrap();
    let exported = j.export_recovery(&destination).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(exported.join("manifest.json")).unwrap()).unwrap();
    let entry = &manifest["operations"][0]["entries"][0];
    assert_eq!(entry["symlink_target"]["encoding"], "Unix");
    assert_eq!(
        entry["symlink_target"]["units"],
        serde_json::json!(target.as_os_str().as_bytes())
    );
    for entry in fs::read_dir(&exported).unwrap() {
        assert!(
            entry.unwrap().file_type().unwrap().is_file(),
            "Export recreated a recovery symlink"
        );
    }
    assert_eq!(fs::read(&side).unwrap(), b"Current regular cover");
}

#[cfg(unix)]
#[test]
fn recovery_export_refuses_an_insufficient_destination_reserve() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let audio = dir.path().join("audio.wav");
    wav(&audio);
    let mut j = open(&root);
    let capture = j.begin("Interrupted", None, &[key(&audio)]);
    drop(capture);
    drop(j);
    let j = Journal::open(
        &root,
        Limits {
            min_free: u64::MAX / 2,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(j.state().recovery.is_some());
    let destination = dir.path().join("exports");
    fs::create_dir(&destination).unwrap();
    assert!(j.export_recovery(&destination).is_err());
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
    assert!(j.state().recovery.is_some());
}

#[test]
fn recovery_export_identifies_the_capture_epoch_of_folder_action_outputs() {
    let dir = EvidenceDir::new();
    let base = fs::canonicalize(dir.path()).unwrap();
    let root = base.join("history");
    let album = base.join("Album");
    let moved = base.join("Moved");
    let side = album.join("nested/cover.bin");
    fs::create_dir(&album).unwrap();
    fs::write(album.join("original.txt"), b"Original folder member").unwrap();
    let mut j = open(&root);
    let mut capture = j.begin("Folder action", None, &[]);
    fs::rename(&album, &moved).unwrap();
    j.capture_side_file_after_moves(&mut capture, &side, 1)
        .unwrap();
    fs::create_dir_all(side.parent().unwrap()).unwrap();
    fs::write(&side, b"After-move cover").unwrap();
    j.commit(capture, &[], &[(key(&album), key(&moved))]);
    let interruption = j.begin("Interrupted later work", None, &[]);
    drop(interruption);
    drop(j);
    let j = open(&root);
    let destination = base.join("exports");
    fs::create_dir(&destination).unwrap();
    let exported = j.export_recovery(&destination).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(exported.join("manifest.json")).unwrap()).unwrap();
    let operation = &manifest["operations"][0];
    assert_eq!(operation["label"], "Folder action");
    let entry = operation["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["copy"].is_string())
        .unwrap();
    assert_eq!(
        entry["epoch"], 1,
        "Recovery lost which folder-move phase owns this side-output path"
    );
    assert_eq!(
        fs::read(exported.join(entry["copy"].as_str().unwrap())).unwrap(),
        b"After-move cover"
    );
    assert_eq!(fs::read(&side).unwrap(), b"After-move cover");
    assert_eq!(
        fs::read(moved.join("original.txt")).unwrap(),
        b"Original folder member"
    );
}

#[test]
fn owned_parent_recreation_updates_later_redo_records_after_reopening() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let parent = dir.path().join("covers");
    let first = parent.join("first.bin");
    let second = parent.join("second.bin");
    let mut j = open(&root);
    let mut capture = j.begin("First cover", None, &[]);
    j.capture_side_file(&mut capture, &first).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::write(&first, b"First").unwrap();
    j.commit(capture, &[], &[]);
    let mut capture = j.begin("Second cover", None, &[]);
    j.capture_side_file(&mut capture, &second).unwrap();
    fs::write(&second, b"Second").unwrap();
    j.commit(capture, &[], &[]);
    assert!(j.undo().unwrap().errors.is_empty());
    assert!(j.undo().unwrap().errors.is_empty());
    assert!(!parent.exists());
    drop(j);
    let mut j = open(&root);
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&first).unwrap(), b"First");
    drop(j);
    let mut j = open(&root);
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&second).unwrap(), b"Second");
}

#[test]
fn a_reused_group_identifier_after_restart_does_not_merge_separate_save_clicks() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let first = dir.path().join("first.wav");
    let second = dir.path().join("second.wav");
    wav(&first);
    wav(&second);
    let first_original = fs::read(&first).unwrap();
    let second_original = fs::read(&second).unwrap();
    let fields = Fields::from([("TITLE".into(), vec!["Saved".into()])]);
    let mut j = open(&root);
    let capture = j.begin("Save tags", Some("same-process-local-id"), &[key(&first)]);
    write_fields(&first, &fields).unwrap();
    j.commit(capture, &[], &[]);
    let first_saved = fs::read(&first).unwrap();
    drop(j);
    let mut j = open(&root);
    let capture = j.begin("Save tags", Some("same-process-local-id"), &[key(&second)]);
    write_fields(&second, &fields).unwrap();
    j.commit(capture, &[], &[]);
    assert_eq!(j.state().undo.unwrap().files, 1);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&second).unwrap(), second_original);
    assert_eq!(fs::read(&first).unwrap(), first_saved);
    assert!(j.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&first).unwrap(), first_original);
}

#[test]
fn a_partial_undo_reopens_with_redo_only_for_its_successfully_restored_files() {
    let dir = EvidenceDir::new();
    let root = dir.path().join("history");
    let first = dir.path().join("first.wav");
    let second = dir.path().join("second.wav");
    wav(&first);
    wav(&second);
    let second_original = fs::read(&second).unwrap();
    let mut j = open(&root);
    let capture = j.begin("Batch save", None, &[key(&first), key(&second)]);
    let fields = Fields::from([("TITLE".into(), vec!["Saved".into()])]);
    write_fields(&first, &fields).unwrap();
    write_fields(&second, &fields).unwrap();
    let second_saved = fs::read(&second).unwrap();
    j.commit(capture, &[], &[]);
    let external = external_pcm_edit(&first);
    let replay = j.undo().unwrap();
    assert_eq!(replay.errors.len(), 1);
    assert_eq!(fs::read(&first).unwrap(), external);
    assert_eq!(fs::read(&second).unwrap(), second_original);
    drop(j);
    let mut j = open(&root);
    assert_eq!(j.state().redo.unwrap().files, 1);
    assert!(j.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&first).unwrap(), external);
    assert_eq!(fs::read(&second).unwrap(), second_saved);
}
