use tag_core::history::{Journal, Limits};
#[test]
fn oversized_side_file_is_refused_before_a_copy() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("existing.png");
    std::fs::write(&output, [7; 64]).unwrap();
    let mut history = Journal::open(
        &dir.path().join("undo"),
        Limits {
            bytes: 32,
            min_free: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    let mut capture = history.begin("Export cover", None, &[]);
    assert!(
        history.capture_side_file(&mut capture, &output).is_err(),
        "must refuse before copying an over-limit existing output"
    );
    assert_eq!(std::fs::read(output).unwrap(), [7; 64]);
}

#[test]
fn lost_after_snapshot_does_not_offer_an_incomplete_folder_undo() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let album = root.join("Album");
    let moved = root.join("Moved");
    std::fs::create_dir(&album).unwrap();
    let mut history = Journal::open(
        &root.join("undo"),
        Limits {
            bytes: 32,
            min_free: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    let mut capture = history.begin("Export then move", None, &[]);
    std::fs::rename(&album, &moved).unwrap();
    let output = album.join("new.png");
    history
        .capture_side_file_after_moves(&mut capture, &output, 1)
        .unwrap();
    std::fs::create_dir(&album).unwrap();
    std::fs::write(&output, [8; 64]).unwrap();
    history.commit(
        capture,
        &[],
        &[(
            album.to_string_lossy().into_owned(),
            moved.to_string_lossy().into_owned(),
        )],
    );
    assert!(history.state().undo.is_none());
    assert!(history.state().notice.unwrap().contains("cannot be undone"));
    assert_eq!(history.retained_bytes(), 0);
    assert_eq!(std::fs::read(output).unwrap(), [8; 64]);
    assert!(moved.is_dir());
}
