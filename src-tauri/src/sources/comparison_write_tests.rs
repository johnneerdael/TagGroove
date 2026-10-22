#![cfg(test)]
use super::*;
use serde_json::json;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

static SERIAL: Mutex<()> = Mutex::new(());
static TICKET: AtomicU64 = AtomicU64::new(10);
fn ticket() -> u64 {
    TICKET.fetch_add(10, Ordering::SeqCst)
}

fn wav(path: &Path) {
    let samples = vec![0u8; 8820];
    let mut data = b"RIFF".to_vec();
    data.extend((36 + samples.len() as u32).to_le_bytes());
    data.extend(b"WAVEfmt ");
    data.extend(16u32.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(44100u32.to_le_bytes());
    data.extend(88200u32.to_le_bytes());
    data.extend(2u16.to_le_bytes());
    data.extend(16u16.to_le_bytes());
    data.extend(b"data");
    data.extend((samples.len() as u32).to_le_bytes());
    data.extend(samples);
    std::fs::write(path, data).unwrap();
}
fn options() -> ImportCover {
    ImportCover {
        replace: false,
        preserve: false,
        save_as: None,
    }
}
fn item(path: &Path, updates: Fields, remove_covers: bool, token: Option<&str>) -> ApplyItem {
    serde_json::from_value(json!({"path":path,"updates":updates,"remove_covers":remove_covers,"comparison_snapshot":token})).unwrap()
}
fn write(items: &[ApplyItem]) -> ScanResult {
    write_import(items, None, &options(), &EvaluationSettings::default()).unwrap()
}

#[test]
fn none_removes_a_text_field_and_existing_artwork_in_the_same_import() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    tag_core::write_fields_summary(
        &path,
        &Fields::from([("COMMENT".into(), vec!["My note".into()])]),
    )
    .unwrap();
    let image = dir.path().join("cover.png");
    image::DynamicImage::new_rgb8(2, 2).save(&image).unwrap();
    tag_core::add_cover_summary(&path, &image).unwrap();
    let result = write(&[item(
        &path,
        Fields::from([("COMMENT".into(), vec![])]),
        true,
        None,
    )]);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let read = tag_core::read_entry_summary(&path).unwrap();
    assert!(!read.fields.contains_key("COMMENT"));
    assert!(read.covers.is_empty());
}

#[test]
fn cover_only_none_is_a_valid_write_but_inherit_without_changes_is_not() {
    let path = Path::new("/fixture.wav");
    assert!(validate_items(&[item(path, Fields::new(), true, None)], false).is_ok());
    assert!(validate_items(&[item(path, Fields::new(), false, None)], false).is_err());
    assert!(validate_items(&[item(path, Fields::new(), true, None)], true).is_err());
}

#[test]
fn ordinary_source_imports_preserve_artwork_without_an_explicit_cover_action() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    let image = dir.path().join("cover.png");
    image::DynamicImage::new_rgb8(2, 2).save(&image).unwrap();
    tag_core::add_cover_summary(&path, &image).unwrap();
    let before = tag_core::read_entry_summary(&path).unwrap().covers[0]
        .digest
        .clone();
    let old_item: ApplyItem =
        serde_json::from_value(json!({"path":path,"updates":{"TITLE":["Track"]}})).unwrap();
    let result = write(&[old_item]);
    assert!(result.errors.is_empty());
    assert_eq!(
        tag_core::read_entry_summary(&path).unwrap().covers[0].digest,
        before
    );
}

#[test]
fn snapshots_reject_changed_files_and_paths_outside_the_reviewed_selection() {
    use super::comparison::{Snapshot, SnapshotStore};
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    wav(&a);
    wav(&b);
    let mut store = SnapshotStore::default();
    let token = store
        .install(Snapshot::read(&[a.to_string_lossy().into_owned()], 10).unwrap())
        .unwrap();
    assert!(store.begin("unknown-token", &a).is_err());
    assert!(store.begin(&token, &b).is_err());
    tag_core::write_fields_summary(
        &a,
        &Fields::from([("TITLE".into(), vec!["Externally edited".into()])]),
    )
    .unwrap();
    let baseline = store.begin(&token, &a).unwrap();
    assert!(baseline.verify(&a).is_err());
    store.finish(&token, &a, false);
}

#[test]
fn snapshots_reserve_writes_and_only_keep_failed_targets_retryable() {
    use super::comparison::{Snapshot, SnapshotStore};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    let mut store = SnapshotStore::default();
    let selected = vec![path.to_string_lossy().into_owned()];
    let token = store
        .install(Snapshot::read(&selected, 10).unwrap())
        .unwrap();
    store.begin(&token, &path).unwrap().verify(&path).unwrap();
    assert!(store.begin(&token, &path).is_err());
    assert!(store
        .install(Snapshot::read(&selected, 11).unwrap())
        .is_err());
    store.finish(&token, &path, false);
    assert!(store.begin(&token, &path).is_ok());
    store.finish(&token, &path, true);
    assert!(store.begin(&token, &path).is_err());
}

#[test]
fn cancelled_snapshot_reads_cannot_resurrect_an_old_comparison() {
    use super::comparison::{Snapshot, SnapshotStore};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    let selected = vec![path.to_string_lossy().into_owned()];
    let mut store = SnapshotStore::default();
    let pending = Snapshot::read(&selected, 10).unwrap();
    store.cancel(11);
    assert!(store.install(pending).is_err());
    let token = store
        .install(Snapshot::read(&selected, 12).unwrap())
        .unwrap();
    store.cancel(13);
    assert!(store.begin(&token, &path).is_err());
    assert!(Snapshot::read(&[], 14).is_err());
    assert!(Snapshot::read(&[selected[0].clone(), selected[0].clone()], 14).is_err());
}

#[test]
fn snapshot_import_refuses_newer_metadata_without_overwriting_it() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    wav(&path);
    let token =
        super::comparison::capture(&[path.to_string_lossy().into_owned()], ticket()).unwrap();
    tag_core::write_fields_summary(
        &path,
        &Fields::from([("TITLE".into(), vec!["Newer value".into()])]),
    )
    .unwrap();
    let result = write(&[item(
        &path,
        Fields::from([("TITLE".into(), vec!["Stale import".into()])]),
        false,
        Some(&token),
    )]);
    assert_eq!(result.errors.len(), 1);
    assert!(result.entries.is_empty());
    assert_eq!(
        tag_core::read_entry_summary(&path).unwrap().fields["TITLE"],
        ["Newer value"]
    );
}

#[test]
fn partial_album_import_retries_only_the_failed_file_and_cannot_repeat_successes() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    let saved = dir.path().join("b.saved");
    wav(&a);
    wav(&b);
    let token = super::comparison::capture(
        &[
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ],
        ticket(),
    )
    .unwrap();
    std::fs::rename(&b, &saved).unwrap();
    let update = |path: &Path| {
        item(
            path,
            Fields::from([("ALBUMARTIST".into(), vec!["Various Artists".into()])]),
            false,
            Some(&token),
        )
    };
    let first = write(&[update(&a), update(&b)]);
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.errors.len(), 1);
    std::fs::rename(&saved, &b).unwrap();
    let retry = write(&[update(&b)]);
    assert_eq!(retry.entries.len(), 1);
    assert!(retry.errors.is_empty(), "{:?}", retry.errors);
    let duplicate = write(&[update(&a)]);
    assert!(duplicate.entries.is_empty());
    assert_eq!(duplicate.errors.len(), 1);
    assert_eq!(
        tag_core::read_entry_summary(&a).unwrap().fields["ALBUMARTIST"],
        ["Various Artists"]
    );
    assert_eq!(
        tag_core::read_entry_summary(&b).unwrap().fields["ALBUMARTIST"],
        ["Various Artists"]
    );
}

#[test]
fn provider_document_changes_invalidate_the_comparison_without_bypassing_save_back() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("provider.wav");
    wav(&original);
    let session = crate::documents::command_fixture::Session::import(
        &dir.path().join("documents"),
        &original,
        "provider.wav",
        true,
    );
    let managed = Path::new(&session.path);
    let token = super::comparison::capture(std::slice::from_ref(&session.path), ticket()).unwrap();
    crate::mutate_document(managed, |working| {
        crate::settings::write_summary(
            working,
            &Fields::from([("TITLE".into(), vec!["Changed after review".into()])]),
        )
    })
    .unwrap();
    let result = write(&[item(
        managed,
        Fields::from([("TITLE".into(), vec!["Stale import".into()])]),
        false,
        Some(&token),
    )]);
    assert_eq!(result.errors.len(), 1);
    assert!(result.entries.is_empty());
    assert_eq!(
        tag_core::read_entry_summary(&original).unwrap().fields["TITLE"],
        ["Changed after review"]
    );
}

#[test]
fn captured_view_returns_the_actual_existing_metadata_in_selection_order() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    wav(&a);
    wav(&b);
    tag_core::write_fields_summary(
        &a,
        &Fields::from([("COMMENT".into(), vec!["Actual current note".into()])]),
    )
    .unwrap();
    let view = super::comparison::capture_view(
        &[
            b.to_string_lossy().into_owned(),
            a.to_string_lossy().into_owned(),
        ],
        ticket(),
    )
    .unwrap();
    assert_eq!(view.files[0].path, b.to_string_lossy());
    assert_eq!(view.files[1].fields["COMMENT"], ["Actual current note"]);
    assert!(!view.token.is_empty());
    let json = serde_json::to_value(view).unwrap();
    assert_eq!(
        json["files"][1]["fields"]["COMMENT"],
        json!(["Actual current note"])
    );
    assert!(json["files"][1].get("identity").is_none());
}
