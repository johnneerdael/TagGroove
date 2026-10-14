use tag_core::{numbering::*, Fields};

fn item(path: &str, album: &[&str], disc: &str) -> NumberingItem {
    NumberingItem {
        path: path.into(),
        fields: Fields::from([
            ("album".into(), album.iter().map(|s| (*s).into()).collect()),
            ("DISCNUMBER".into(), vec![disc.into()]),
        ]),
    }
}

fn tracks(plan: &[NumberingUpdate]) -> Vec<&str> {
    plan.iter()
        .map(|row| row.fields["TRACK"][0].as_str())
        .collect()
}

fn discs(plan: &[NumberingUpdate]) -> Vec<&str> {
    plan.iter()
        .map(|row| row.fields["DISCNUMBER"][0].as_str())
        .collect()
}

#[test]
fn selection_order_and_explicit_totals_are_preserved() {
    let input = [item("b.flac", &["A"], "7"), item("a.mp3", &["A"], "8")];
    let options = NumberingOptions {
        track_start: 7,
        track_digits: 3,
        track_total: Some(24),
        ..Default::default()
    };
    let plan = plan_numbering(&input, &options).unwrap();
    assert_eq!(tracks(&plan), ["007/24", "008/24"]);
    assert_eq!(plan[0].path, "b.flac");
    assert_eq!(plan[1].path, "a.mp3");
    assert_eq!(plan[0].fields.len(), 1);
    assert_eq!(input[0].fields["DISCNUMBER"], ["7"]);
}

#[test]
fn directory_transitions_increment_once_and_reset_tracks_to_configured_start() {
    let input = [
        item("album/disc-a/2.flac", &["A"], "1"),
        item("album/disc-a/1.flac", &["A"], "1"),
        item("album/disc-b/1.flac", &["B"], "2"),
        item("album/disc-a/3.flac", &["A"], "1"),
    ];
    let options = NumberingOptions {
        track_start: 3,
        track_total: Some(10),
        disc_enabled: true,
        disc_start: 2,
        disc_digits: 2,
        disc_total: Some(4),
        next_disc_on_directory: true,
        next_disc_on_field: Some("ALBUM".into()),
        reset_track_on_next_disc: true,
        ..Default::default()
    };
    let plan = plan_numbering(&input, &options).unwrap();
    assert_eq!(tracks(&plan), ["03/10", "04/10", "03/10", "03/10"]);
    assert_eq!(discs(&plan), ["02/4", "02/4", "03/4", "04/4"]);
}

#[test]
fn next_disc_can_keep_a_continuous_track_counter() {
    let input = [item("a/1.mp3", &["A"], "1"), item("b/2.mp3", &["A"], "1")];
    let options = NumberingOptions {
        disc_enabled: true,
        next_disc_on_directory: true,
        ..Default::default()
    };
    let plan = plan_numbering(&input, &options).unwrap();
    assert_eq!(tracks(&plan), ["01", "02"]);
    assert_eq!(discs(&plan), ["1", "2"]);
}

#[test]
fn album_reset_takes_precedence_and_implicitly_resets_tracks() {
    let input = [
        item("album-a/disc-a/1.mp3", &["A"], "1"),
        item("album-a/disc-b/2.mp3", &["A"], "2"),
        item("album-b/disc-a/3.mp3", &["B"], "1"),
        item("album-b/disc-a/4.mp3", &["B"], "1"),
    ];
    let options = NumberingOptions {
        track_start: 5,
        disc_enabled: true,
        disc_start: 3,
        next_disc_on_directory: true,
        next_disc_on_field: Some("DISCNUMBER".into()),
        reset_disc_on_field: Some("ALBUM".into()),
        ..Default::default()
    };
    let plan = plan_numbering(&input, &options).unwrap();
    assert_eq!(tracks(&plan), ["05", "06", "05", "06"]);
    assert_eq!(discs(&plan), ["3", "4", "3", "3"]);
}

#[test]
fn field_changes_use_original_multivalues_and_case_insensitive_names() {
    let mut input = [
        item("1.flac", &["A", "B"], "1"),
        item("2.flac", &["A", "C"], "1"),
        item("3.flac", &["A", "C"], "2"),
    ];
    let album = input[1].fields.remove("album").unwrap();
    input[1].fields.insert("ALBUM".into(), album);
    let options = NumberingOptions {
        disc_enabled: true,
        next_disc_on_field: Some("album".into()),
        reset_track_on_next_disc: true,
        ..Default::default()
    };
    let plan = plan_numbering(&input, &options).unwrap();
    assert_eq!(tracks(&plan), ["01", "01", "02"]);
    assert_eq!(discs(&plan), ["1", "2", "2"]);
}

#[test]
fn track_only_mode_can_reset_by_existing_disc_field_without_changing_disc_tags() {
    let input = [item("1.flac", &["A"], "1"), item("2.flac", &["A"], "2")];
    let options = NumberingOptions {
        next_disc_on_field: Some("DISCNUMBER".into()),
        reset_track_on_next_disc: true,
        ..Default::default()
    };
    let plan = plan_numbering(&input, &options).unwrap();
    assert_eq!(tracks(&plan), ["01", "01"]);
    assert!(plan
        .iter()
        .all(|row| !row.fields.contains_key("DISCNUMBER")));
}

#[test]
fn custom_field_names_and_absent_values_remain_available_for_grouping() {
    let mut input = [
        item("1.flac", &[], "1"),
        item("2.flac", &[], "1"),
        item("3.flac", &[], "1"),
    ];
    input[1]
        .fields
        .insert("_CUSTOM 猫".into(), vec!["First".into()]);
    input[2]
        .fields
        .insert("_CUSTOM 猫".into(), vec!["Second".into()]);
    let options = NumberingOptions {
        next_disc_on_field: Some("_CUSTOM 猫".into()),
        reset_track_on_next_disc: true,
        ..Default::default()
    };
    assert_eq!(
        tracks(&plan_numbering(&input, &options).unwrap()),
        ["01", "01", "01"]
    );
}

#[test]
fn invalid_settings_and_overflow_return_no_partial_plan() {
    let input = [item("a/1.mp3", &["A"], "1"), item("b/2.mp3", &["A"], "1")];
    for options in [
        NumberingOptions {
            track_start: 0,
            ..Default::default()
        },
        NumberingOptions {
            track_digits: 11,
            ..Default::default()
        },
        NumberingOptions {
            track_total: Some(0),
            ..Default::default()
        },
        NumberingOptions {
            disc_enabled: true,
            disc_start: 0,
            ..Default::default()
        },
        NumberingOptions {
            next_disc_on_field: Some(" ".into()),
            ..Default::default()
        },
        NumberingOptions {
            reset_disc_on_field: Some("ALBUM\0".into()),
            ..Default::default()
        },
        NumberingOptions {
            track_start: u32::MAX,
            ..Default::default()
        },
        NumberingOptions {
            disc_enabled: true,
            disc_start: u32::MAX,
            next_disc_on_directory: true,
            ..Default::default()
        },
    ] {
        assert!(plan_numbering(&input, &options).is_err(), "{options:?}");
    }
    let one = plan_numbering(
        &input[..1],
        &NumberingOptions {
            track_start: u32::MAX,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(tracks(&one), ["4294967295"]);
    assert!(plan_numbering(&[], &NumberingOptions::default())
        .unwrap()
        .is_empty());
}
