//! Locale/header preservation uses an observer independent of Lofty's model.
#[allow(dead_code)]
#[path = "support/save_corpus.rs"]
mod corpus;
use corpus::mp4_atoms;
use mp4_atoms::{data, freeform, item, items, media, with_items, Identity};
use std::fs;
use tag_core::{write_fields_with_options, Fields, TagOptions};
const CARRIER: &[u8] = include_bytes!("fixtures/save-corpus/tone.m4a");
fn title(path: &std::path::Path, single: bool) {
    let mut options = TagOptions::default();
    options.advanced.mp4_single_atom = single;
    write_fields_with_options(
        path,
        &Fields::from([("TITLE".into(), vec!["Changed title".into()])]),
        false,
        &[],
        &options,
    )
    .unwrap();
}
fn retained(bytes: &[u8]) -> Vec<mp4_atoms::Item> {
    items(bytes)
        .into_iter()
        .filter(|item| item.identity != Identity::Fourcc(*b"\xa9nam"))
        .collect()
}

#[test]
fn unrelated_title_retains_each_locale_and_raw_data_envelope() {
    let unknown = data(0, 1033, b"opaque\0\xff");
    let utf16 = "日本"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect::<Vec<_>>();
    let raw = vec![
        item(*b"xQA!", &[unknown.clone(), data(0, 1043, b"opaque\0\xff")]),
        item(*b"xQA!", &[unknown]),
        item(
            *b"\xa9ART",
            &[data(1, 1031, b"Artist"), data(2, 1041, &utf16)],
        ),
        freeform(
            b"com.example",
            b"custom:name",
            &[data(1, 1025, b"same"), data(1, 1033, b"same")],
        ),
        item(*b"plID", &[data(21, 1036, &0x01020304u64.to_be_bytes())]),
    ];
    for single in [true, false] {
        for padding in [0, 8192] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("locale.m4a");
            let before = with_items(CARRIER, &raw, padding);
            fs::write(&path, &before).unwrap();
            let visible_before = tag_core::read_entry(&path).unwrap();
            assert_eq!(visible_before.fields["ARTIST"], ["Artist", "日本"]);
            let expected = retained(&before)
                .into_iter()
                .flat_map(|item| {
                    item.values
                        .into_iter()
                        .map(move |value| (item.identity.clone(), value.raw))
                })
                .collect::<Vec<_>>();
            title(&path, single);
            let after = fs::read(&path).unwrap();
            assert_eq!(
                tag_core::read_entry(&path).unwrap().fields["ARTIST"],
                visible_before.fields["ARTIST"]
            );
            let actual = retained(&after)
                .into_iter()
                .flat_map(|item| {
                    item.values
                        .into_iter()
                        .map(move |value| (item.identity.clone(), value.raw))
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "single={single},padding={padding}");
            assert_eq!(media(&after), media(&before));
        }
    }
}

#[test]
fn independent_observer_detects_locale_that_native_debug_omits() {
    use lofty::file::AudioFile;
    let root = tempfile::tempdir().unwrap();
    let mut native = Vec::new();
    let mut observed = Vec::new();
    for locale in [0, 1033] {
        let path = root.path().join(format!("{locale}.m4a"));
        let raw = with_items(CARRIER, &[item(*b"xQA!", &[data(0, locale, b"opaque")])], 0);
        fs::write(&path, &raw).unwrap();
        native.push(
            lofty::mp4::Mp4File::read_from(
                &mut fs::File::open(path).unwrap(),
                lofty::config::ParseOptions::new(),
            )
            .unwrap()
            .ilst()
            .cloned(),
        );
        observed.push(items(&raw));
    }
    assert_eq!(native[0], native[1]);
    assert_ne!(observed[0], observed[1]);
    assert_eq!(observed[1][0].values[0].locale, 1033);
}

#[test]
fn current_duplicate_grouping_baseline_is_observed_for_both_options() {
    for single in [true, false] {
        for (label, raw, expected) in [
            (
                "repeated",
                vec![
                    item(*b"xQA!", &[data(0, 0, b"A")]),
                    item(*b"xQA!", &[data(0, 0, b"B")]),
                ],
                vec![1, 1],
            ),
            (
                "multi",
                vec![item(*b"xQA!", &[data(0, 0, b"A"), data(0, 0, b"B")])],
                if single { vec![2] } else { vec![1, 1] },
            ),
            (
                "legacy-repeated",
                vec![
                    item(*b"plID", &[data(21, 0, &[1, 2, 3, 4])]),
                    item(*b"plID", &[data(21, 0, &[0, 0, 0, 0, 5, 6, 7, 8])]),
                ],
                if single { vec![2] } else { vec![1, 1] },
            ),
        ] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("grouping.m4a");
            fs::write(&path, with_items(CARRIER, &raw, 0)).unwrap();
            title(&path, single);
            let grouped = retained(&fs::read(path).unwrap())
                .into_iter()
                .map(|item| item.values.len())
                .collect::<Vec<_>>();
            assert_eq!(grouped, expected, "{label},single={single}");
        }
    }
}

#[test]
fn explicit_edits_removals_and_mapping_changes_do_not_resurrect_raw_groups() {
    let raw = vec![
        item(*b"\xa9ART", &[data(1, 1033, b"Old"), data(1, 1043, b"Old")]),
        item(*b"\xa9ART", &[data(1, 1031, b"Third")]),
        item(*b"xQA!", &[data(0, 1041, b"keep")]),
    ];
    for single in [true, false] {
        for remove in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("edit.m4a");
            let before = with_items(CARRIER, &raw, 0);
            fs::write(&path, &before).unwrap();
            let mut options = TagOptions::default();
            options.advanced.mp4_single_atom = single;
            write_fields_with_options(
                &path,
                &Fields::from([(
                    "ARTIST".into(),
                    if remove {
                        vec![]
                    } else {
                        vec!["New A".into(), "New B".into()]
                    },
                )]),
                false,
                &[],
                &options,
            )
            .unwrap();
            let after = fs::read(&path).unwrap();
            let edited = items(&after)
                .into_iter()
                .filter(|i| i.identity == Identity::Fourcc(*b"\xa9ART"))
                .collect::<Vec<_>>();
            assert_eq!(
                edited.iter().map(|i| i.values.len()).sum::<usize>(),
                if remove { 0 } else { 2 }
            );
            assert!(edited.iter().flat_map(|i| &i.values).all(|v| v.locale == 0));
            assert_eq!(media(&before), media(&after));
            assert_eq!(
                items(&after)
                    .into_iter()
                    .find(|i| i.identity == Identity::Fourcc(*b"xQA!"))
                    .unwrap()
                    .values[0]
                    .locale,
                1041
            );
        }
    }
    for reuse in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("remap.m4a");
        fs::write(
            &path,
            with_items(
                CARRIER,
                &[freeform(
                    b"com.apple.iTunes",
                    b"COMPOSER",
                    &[data(1, 1033, b"Bach")],
                )],
                0,
            ),
        )
        .unwrap();
        let mut options = TagOptions::default();
        options.advanced.reuse_mp4_atoms = reuse;
        write_fields_with_options(
            &path,
            &Fields::from([("TITLE".into(), vec!["Changed".into()])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let atoms = items(&fs::read(&path).unwrap());
        assert_eq!(atoms.iter().any(|i|matches!(&i.identity,Identity::Freeform(mean,name) if mean==b"com.apple.iTunes"&&name==b"COMPOSER")),reuse);
        let composer = atoms
            .iter()
            .find(|i| {
                if reuse {
                    matches!(&i.identity,Identity::Freeform(_,name) if name==b"COMPOSER")
                } else {
                    i.identity == Identity::Fourcc(*b"\xa9wrt")
                }
            })
            .unwrap();
        assert_eq!(composer.values[0].locale, if reuse { 1033 } else { 0 });
    }
}

fn widened(bytes: &[u8], ids: &[[u8; 4]]) -> Vec<u8> {
    let chain = mp4_atoms::path(bytes, ids);
    let target = chain.last().unwrap();
    assert_eq!(target.body - target.start, 8);
    let mut header = 1u32.to_be_bytes().to_vec();
    header.extend(target.kind);
    header.extend(((target.end - target.start + 8) as u64).to_be_bytes());
    let mut result = [&bytes[..target.start], &header, &bytes[target.body..]].concat();
    for b in &chain[..chain.len() - 1] {
        if b.body - b.start == 8 {
            result[b.start..b.start + 4]
                .copy_from_slice(&((b.end - b.start + 8) as u32).to_be_bytes());
        } else {
            result[b.start + 8..b.start + 16]
                .copy_from_slice(&((b.end - b.start + 8) as u64).to_be_bytes());
        }
    }
    result
}

#[test]
fn extended_container_full_fallback_preserves_raw_values_and_media() {
    for id in [
        vec![*b"moov"],
        vec![*b"moov", *b"udta"],
        vec![*b"moov", *b"udta", *b"meta"],
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("wide.m4a");
        let before = widened(
            &with_items(CARRIER, &[item(*b"xQA!", &[data(0, 1033, b"keep")])], 0),
            &id,
        );
        fs::write(&path, &before).unwrap();
        title(&path, true);
        let after = fs::read(path).unwrap();
        assert_eq!(retained(&after), retained(&before));
        assert_eq!(media(&after), media(&before));
        assert_eq!(
            mp4_atoms::chunk_offsets(&after),
            mp4_atoms::chunk_offsets(&before)
        );
    }
}

#[test]
fn absent_tag_fallback_and_complete_removal_use_normal_encoding() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("empty.m4a");
    let chain = mp4_atoms::path(CARRIER, &[*b"moov", *b"udta"]);
    let udta = chain.last().unwrap();
    let mut before = [&CARRIER[..udta.start], &CARRIER[udta.end..]].concat();
    let moov = &chain[0];
    before[moov.start..moov.start + 4]
        .copy_from_slice(&((moov.end - moov.start - (udta.end - udta.start)) as u32).to_be_bytes());
    fs::write(&path, &before).unwrap();
    title(&path, true);
    assert_eq!(
        tag_core::read_entry(&path).unwrap().fields["TITLE"],
        ["Changed title"]
    );
    assert_eq!(media(&fs::read(&path).unwrap()), media(&before));
    write_fields_with_options(
        &path,
        &Fields::from([("TITLE".into(), vec![])]),
        false,
        &[],
        &TagOptions::default(),
    )
    .unwrap();
    assert!(!tag_core::read_entry(&path)
        .unwrap()
        .fields
        .contains_key("TITLE"));
    assert_eq!(media(&fs::read(path).unwrap()), media(&before));
}

#[test]
fn accepted_terminal_zero_size_items_are_not_relocated_with_open_ended_headers() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("zero.m4a");
    let mut raw = item(*b"xQA!", &[data(0, 1033, b"keep")]);
    raw[..4].fill(0);
    let before = with_items(CARRIER, &[raw], 0);
    fs::write(&path, &before).unwrap();
    // Do not assume this is accepted by the pinned reader: qualify it first.
    match tag_core::read_entry(&path) {
        Ok(_) => {
            title(&path, true);
            assert_eq!(
                tag_core::read_entry(&path).unwrap().fields["TITLE"],
                ["Changed title"]
            );
        }
        Err(_) => {
            assert!(tag_core::write_fields(
                &path,
                &Fields::from([("TITLE".into(), vec!["Changed title".into()])])
            )
            .is_err());
            assert_eq!(fs::read(path).unwrap(), before);
        }
    }
}

#[test]
fn ilst_padding_payload_is_not_required_to_contain_child_atoms() {
    for kind in [*b"free", *b"skip"] {
        for single in [true, false] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("padding.m4a");
            let before = with_items(
                CARRIER,
                &[
                    mp4_atoms::boxed(kind, b"abcd"),
                    item(*b"\xa9nam", &[data(1, 1033, b"Old")]),
                ],
                0,
            );
            fs::write(&path, &before).unwrap();
            assert_eq!(
                tag_core::read_entry(&path).unwrap().fields["TITLE"],
                ["Old"]
            );
            title(&path, single);
            assert_eq!(
                tag_core::read_entry(&path).unwrap().fields["TITLE"],
                ["Changed title"]
            );
            assert_eq!(media(&fs::read(&path).unwrap()), media(&before));
        }
    }
}

#[test]
fn pinned_lofty_extended_header_control_exposes_invalid_output() {
    use lofty::{file::AudioFile, tag::TagExt};
    let before = widened(
        &with_items(CARRIER, &[item(*b"xQA!", &[data(0, 1033, b"keep")])], 0),
        &[*b"moov"],
    );
    let mut cursor = std::io::Cursor::new(before);
    let original =
        lofty::mp4::Mp4File::read_from(&mut cursor, lofty::config::ParseOptions::new()).unwrap();
    let mut tag = original.ilst().cloned().unwrap();
    tag.insert(lofty::mp4::Atom::new(
        lofty::mp4::AtomIdent::Fourcc(*b"\xa9nam"),
        lofty::mp4::AtomData::UTF8("Changed title".into()),
    ));
    cursor.set_position(0);
    tag.save_to(&mut cursor, lofty::config::WriteOptions::default())
        .unwrap();
    cursor.set_position(0);
    assert!(
        lofty::mp4::Mp4File::read_from(&mut cursor, lofty::config::ParseOptions::new()).is_err()
    );
    // The old extended-size word remains where a child header must start.
    let b = cursor.into_inner();
    let moov = mp4_atoms::path(&b, &[*b"moov"])[0].clone();
    assert_ne!(&b[moov.body + 4..moov.body + 8], b"trak");
}

#[test]
fn localized_values_and_cover_changes_survive_exact_journal_replay() {
    use tag_core::history::{Journal, Limits};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("journal.m4a");
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(2, 2, image::Rgb([255, 0, 0]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let png = png.into_inner();
    let before = with_items(
        CARRIER,
        &[
            item(*b"xQA!", &[data(0, 1033, b"keep")]),
            item(*b"covr", &[data(14, 1043, &png)]),
        ],
        0,
    );
    fs::write(&path, &before).unwrap();
    let mut journal = Journal::open(&root.path().join("history"), Limits::default()).unwrap();
    let capture = journal.begin(
        "Localized save",
        None,
        &[path.to_string_lossy().into_owned()],
    );
    title(&path, true);
    journal.commit(capture, &[], &[]);
    let saved = fs::read(&path).unwrap();
    assert_eq!(retained(&saved), retained(&before));
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), saved);
    tag_core::remove_cover(&path, 0).unwrap();
    assert!(!items(&fs::read(&path).unwrap())
        .iter()
        .any(|i| i.identity == Identity::Fourcc(*b"covr")));
    tag_core::add_cover_bytes_with_options(&path, png, "new", None, &TagOptions::default())
        .unwrap();
    let after = fs::read(&path).unwrap();
    let covers = items(&after)
        .into_iter()
        .filter(|i| i.identity == Identity::Fourcc(*b"covr"))
        .collect::<Vec<_>>();
    assert_eq!(covers.len(), 1);
    assert_eq!(covers[0].values[0].locale, 0);
    assert_eq!(
        items(&after)
            .into_iter()
            .find(|i| i.identity == Identity::Fourcc(*b"xQA!"))
            .unwrap()
            .values[0]
            .locale,
        1033
    );
    assert_eq!(media(&after), media(&before));
}

#[test]
fn exact_freeform_identity_and_user_mapping_keep_only_untouched_locales() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("mapping.m4a");
    let before = with_items(
        CARRIER,
        &[
            freeform(b"com:a", b"b", &[data(1, 1033, b"left")]),
            freeform(b"com", b"a:b", &[data(1, 1043, b"right")]),
        ],
        0,
    );
    fs::write(&path, &before).unwrap();
    title(&path, true);
    let after = items(&fs::read(&path).unwrap());
    for (mean, name, locale) in [
        (b"com:a".as_slice(), b"b".as_slice(), 1033),
        (b"com".as_slice(), b"a:b".as_slice(), 1043),
    ] {
        assert_eq!(
            after
                .iter()
                .find(|i| i.identity == Identity::Freeform(mean.to_vec(), name.to_vec()))
                .unwrap()
                .values[0]
                .locale,
            locale
        );
    }
    let rules = [tag_core::UserFieldMapping {
        tag_type: "MP4".into(),
        source: "----:com:a:b".into(),
        target: "DISPLAY".into(),
    }];
    // The existing field-name model cannot distinguish colon-colliding display
    // identities for an explicit edit. Its refusal must never publish changes.
    let checkpoint = fs::read(&path).unwrap();
    assert!(write_fields_with_options(
        &path,
        &Fields::from([("DISPLAY".into(), vec!["edited".into()])]),
        false,
        &rules,
        &TagOptions::default()
    )
    .is_err());
    assert_eq!(fs::read(&path).unwrap(), checkpoint);
    fs::write(
        &path,
        with_items(
            CARRIER,
            &[
                freeform(b"com", b"unique", &[data(1, 1043, b"old")]),
                item(*b"xQA!", &[data(0, 1033, b"keep")]),
            ],
            0,
        ),
    )
    .unwrap();
    let rules = [tag_core::UserFieldMapping {
        tag_type: "MP4".into(),
        source: "----:com:unique".into(),
        target: "DISPLAY".into(),
    }];
    write_fields_with_options(
        &path,
        &Fields::from([("DISPLAY".into(), vec!["edited".into()])]),
        false,
        &rules,
        &TagOptions::default(),
    )
    .unwrap();
    let after = items(&fs::read(&path).unwrap());
    assert_eq!(
        after
            .iter()
            .find(|i| i.identity == Identity::Freeform(b"com".to_vec(), b"unique".to_vec()))
            .unwrap()
            .values[0]
            .locale,
        0
    );
    assert_eq!(
        after
            .iter()
            .find(|i| i.identity == Identity::Fourcc(*b"xQA!"))
            .unwrap()
            .values[0]
            .locale,
        1033
    );
}

#[test]
fn unsupported_zero_size_data_refuses_without_mutating_original() {
    for single in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("zero-data.m4a");
        let mut last = data(21, 1043, &[0, 0, 0, 0, 1, 2, 3, 4]);
        last[..4].fill(0);
        let before = with_items(
            CARRIER,
            &[
                item(*b"plID", &[data(21, 1033, &[0, 0, 0, 1])]),
                item(*b"plID", &[last]),
            ],
            0,
        );
        fs::write(&path, &before).unwrap();
        assert!(tag_core::read_entry(&path).is_err());
        let mut options = TagOptions::default();
        options.advanced.mp4_single_atom = single;
        assert!(write_fields_with_options(
            &path,
            &Fields::from([("TITLE".into(), vec!["Changed".into()])]),
            false,
            &[],
            &options
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn unsupported_non_data_item_children_refuse_without_changing_original() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("prefix.m4a");
    let before = with_items(
        CARRIER,
        &[mp4_atoms::boxed(
            *b"plID",
            &[
                mp4_atoms::boxed(*b"xtra", b"firstxxx"),
                data(21, 1033, &[0, 0, 0, 1]),
            ]
            .concat(),
        )],
        0,
    );
    fs::write(&path, &before).unwrap();
    assert!(tag_core::read_entry(&path).is_err());
    assert!(tag_core::write_fields(
        &path,
        &Fields::from([("TITLE".into(), vec!["Changed".into()])])
    )
    .is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

fn fragment_target(bytes: &[u8]) -> (usize, usize) {
    let moof = mp4_atoms::path(bytes, &[*b"moof"])[0].clone();
    let tfhd = mp4_atoms::path(bytes, &[*b"moof", *b"traf", *b"tfhd"])
        .last()
        .unwrap()
        .clone();
    let flags = u32::from_be_bytes(bytes[tfhd.body..tfhd.body + 4].try_into().unwrap()) & 0xffffff;
    assert_ne!(flags & 0x020000, 0, "fixture uses default-base-is-moof");
    assert_eq!(flags & 1, 0);
    let trun = mp4_atoms::path(bytes, &[*b"moof", *b"traf", *b"trun"])
        .last()
        .unwrap()
        .clone();
    let flags = u32::from_be_bytes(bytes[trun.body..trun.body + 4].try_into().unwrap()) & 0xffffff;
    assert_ne!(flags & 1, 0);
    let offset = i32::from_be_bytes(bytes[trun.body + 8..trun.body + 12].try_into().unwrap());
    let target = usize::try_from(moof.start as i64 + i64::from(offset)).unwrap();
    let mdat = mp4_atoms::path(bytes, &[*b"mdat"])[0].clone();
    assert_eq!(target, mdat.body);
    (target, mdat.end)
}
#[test]
fn real_fragmented_fallback_preserves_locales_audio_and_relative_sample_offset() {
    const FRAGMENTED: &[u8] = include_bytes!("fixtures/save-corpus/tone-fragmented.m4a");
    assert_eq!(media(FRAGMENTED).concat(), media(CARRIER).concat());
    fragment_target(FRAGMENTED);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fragmented.m4a");
    fs::write(&path, FRAGMENTED).unwrap();
    tag_core::write_fields(
        &path,
        &Fields::from([("PRIVATE".into(), vec!["retained".into()])]),
    )
    .unwrap();
    let mut before = fs::read(&path).unwrap();
    let chain = mp4_atoms::path(&before, &[*b"moov", *b"udta", *b"meta", *b"ilst"]);
    let ilst = chain.last().unwrap();
    for i in mp4_atoms::boxes(&before, ilst.body..ilst.end) {
        if i.kind != *b"----" {
            continue;
        }
        let children = mp4_atoms::boxes(&before, i.body..i.end);
        if children
            .iter()
            .any(|a| a.kind == *b"name" && &before[a.body + 4..a.end] == b"PRIVATE")
        {
            let d = children.iter().find(|a| a.kind == *b"data").unwrap();
            before[d.body + 4..d.body + 8].copy_from_slice(&1033u32.to_be_bytes());
        }
    }
    fs::write(&path, &before).unwrap();
    fragment_target(&before);
    tag_core::read_entry(&path).unwrap();
    write_fields_with_options(
        &path,
        &Fields::from([("TITLE".into(), vec!["growth".repeat(2048)])]),
        false,
        &[],
        &TagOptions::default(),
    )
    .unwrap();
    let after = fs::read(&path).unwrap();
    assert_eq!(retained(&after), retained(&before));
    let (a, b) = fragment_target(&after);
    assert_eq!(&after[a..b], media(CARRIER).concat());
    assert_eq!(media(&after), media(&before));
}

#[test]
fn extended_quicktime_meta_without_flags_retains_descendant_boundaries() {
    let before = with_items(CARRIER, &[item(*b"xQA!", &[data(0, 1033, b"keep")])], 0);
    let chain = mp4_atoms::path(&before, &[*b"moov", *b"udta", *b"meta"]);
    let meta = chain.last().unwrap();
    let mut quicktime = [&before[..meta.body], &before[meta.body + 4..]].concat();
    for b in &chain {
        quicktime[b.start..b.start + 4]
            .copy_from_slice(&((b.end - b.start - 4) as u32).to_be_bytes());
    }
    let before = widened(&quicktime, &[*b"moov", *b"udta", *b"meta"]);
    // Direct pinned-Lofty control: keep the original wide-header slot and
    // every original child address while presenting a recognized ignored child.
    use lofty::{file::AudioFile, tag::TagExt};
    let original_meta = mp4_atoms::path(&before, &[*b"moov", *b"udta", *b"meta"])
        .last()
        .unwrap()
        .clone();
    let mut adapted = before.clone();
    adapted[original_meta.start..original_meta.start + 4]
        .copy_from_slice(&((original_meta.end - original_meta.start) as u32).to_be_bytes());
    adapted[original_meta.start + 8..original_meta.start + 16]
        .copy_from_slice(&[0, 0, 0, 8, b'm', b'h', b'd', b'r']);
    assert_eq!(
        &adapted[original_meta.body..original_meta.end],
        &before[original_meta.body..original_meta.end]
    );
    let mut cursor = std::io::Cursor::new(adapted);
    let file =
        lofty::mp4::Mp4File::read_from(&mut cursor, lofty::config::ParseOptions::new()).unwrap();
    let mut tag = file.ilst().cloned().unwrap();
    tag.insert(lofty::mp4::Atom::new(
        lofty::mp4::AtomIdent::Fourcc(*b"\xa9nam"),
        lofty::mp4::AtomData::UTF8("Changed title".into()),
    ));
    cursor.set_position(0);
    tag.save_to(&mut cursor, lofty::config::WriteOptions::default())
        .unwrap();
    let mut direct = cursor.into_inner();
    let generated = mp4_atoms::path(&direct, &[*b"moov", *b"udta", *b"meta"])
        .last()
        .unwrap()
        .clone();
    assert_eq!(
        &direct[generated.body..generated.body + 8],
        &[0, 0, 0, 8, b'm', b'h', b'd', b'r']
    );
    direct[generated.start..generated.start + 4].copy_from_slice(&1u32.to_be_bytes());
    direct[generated.start + 8..generated.start + 16]
        .copy_from_slice(&((generated.end - generated.start) as u64).to_be_bytes());
    let mut cursor = std::io::Cursor::new(direct);
    lofty::mp4::Mp4File::read_from(&mut cursor, lofty::config::ParseOptions::new()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("quicktime.m4a");
    fs::write(&path, &before).unwrap();
    tag_core::read_entry(&path).unwrap();
    title(&path, true);
    let after = fs::read(&path).unwrap();
    assert_eq!(retained(&after), retained(&before));
    assert_eq!(media(&after), media(&before));
    let restored = mp4_atoms::path(&after, &[*b"moov", *b"udta", *b"meta"])
        .last()
        .unwrap()
        .clone();
    assert_eq!(restored.body - restored.start, 16);
    assert_eq!(
        mp4_atoms::chunk_offsets(&after),
        mp4_atoms::chunk_offsets(&before)
    );
    let children = mp4_atoms::boxes(&after, restored.body..restored.end);
    assert!(children.iter().all(|a| a.kind != *b"mhdr"));
    let original_children = mp4_atoms::boxes(&before, original_meta.body..original_meta.end);
    for original in original_children.into_iter().filter(|a| a.kind != *b"ilst") {
        let actual = children.iter().find(|a| a.kind == original.kind).unwrap();
        assert_eq!(
            &after[actual.start..actual.end],
            &before[original.start..original.end]
        );
    }
}

#[test]
fn corpus_snapshot_detects_locale_loss_while_fields_and_native_model_match() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("corpus.m4a");
    corpus::seed(&path);
    let before = corpus::snapshot(&path);
    let mut bytes = fs::read(&path).unwrap();
    let chain = mp4_atoms::path(&bytes, &[*b"moov", *b"udta", *b"meta", *b"ilst"]);
    let ilst = chain.last().unwrap();
    let item = mp4_atoms::boxes(&bytes, ilst.body..ilst.end)
        .into_iter()
        .find(|i| i.kind == *b"xQA!")
        .unwrap();
    let d = mp4_atoms::boxes(&bytes, item.body..item.end)
        .into_iter()
        .find(|a| a.kind == *b"data")
        .unwrap();
    bytes[d.body + 4..d.body + 8].fill(0);
    fs::write(&path, bytes).unwrap();
    let after = corpus::snapshot(&path);
    assert_eq!(after["fields"], before["fields"]);
    assert_eq!(after["opaque_native"], before["opaque_native"]);
    assert_eq!(
        after["audio_payload_base64"],
        before["audio_payload_base64"]
    );
    assert_ne!(after["native_mp4"], before["native_mp4"]);
}

#[test]
fn extended_quicktime_adapter_allows_last_field_removal() {
    for single in [true, false] {
        let original = with_items(
            CARRIER,
            &[item(*b"\xa9nam", &[data(1, 1033, b"Last field")])],
            0,
        );
        let chain = mp4_atoms::path(&original, &[*b"moov", *b"udta", *b"meta"]);
        let meta = chain.last().unwrap();
        let mut quicktime = [&original[..meta.body], &original[meta.body + 4..]].concat();
        for b in &chain {
            quicktime[b.start..b.start + 4]
                .copy_from_slice(&((b.end - b.start - 4) as u32).to_be_bytes());
        }
        let before = widened(&quicktime, &[*b"moov", *b"udta", *b"meta"]);
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("last-field.m4a");
        fs::write(&path, &before).unwrap();
        assert_eq!(
            tag_core::read_entry(&path).unwrap().fields["TITLE"],
            ["Last field"]
        );
        let mut options = TagOptions::default();
        options.advanced.mp4_single_atom = single;
        write_fields_with_options(
            &path,
            &Fields::from([("TITLE".into(), vec![])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        assert!(!tag_core::read_entry(&path)
            .unwrap()
            .fields
            .contains_key("TITLE"));
        let after = fs::read(&path).unwrap();
        let chain = mp4_atoms::path(&after, &[*b"moov", *b"udta", *b"meta"]);
        let meta = chain.last().unwrap();
        assert_eq!(meta.body - meta.start, 16);
        let children = mp4_atoms::boxes(&after, meta.body..meta.end);
        assert!(children
            .iter()
            .all(|a| a.kind != *b"mhdr" && a.kind != *b"ilst"));
        assert_eq!(media(&after), media(&before));
        assert_eq!(
            mp4_atoms::chunk_offsets(&after),
            mp4_atoms::chunk_offsets(&before)
        );
    }
}
