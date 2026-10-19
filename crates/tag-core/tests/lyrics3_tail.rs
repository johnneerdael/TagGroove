//! Independent physical Lyrics3v2 boundaries; no original-program execution.
use std::{fs, path::Path};
use tag_core::{
    history::{Journal, Limits},
    read_entry_with_options, remove_tag_kinds_with_options, remove_tags_with_options,
    write_fields_with_options, Fields, RemovableTag, TagOptions,
};

fn audio() -> Vec<u8> {
    (0..20u32)
        .flat_map(|frame| {
            let mut bytes = vec![0xff, 0xfb, 0x90, 0x64];
            bytes.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
            bytes
        })
        .collect()
}
fn lyrics() -> Vec<u8> {
    // Unknown, non-record body is deliberately opaque, including non-UTF8.
    let body = b"LYRICSBEGINopaque\xffTAGnot-a-field";
    [
        body.as_slice(),
        format!("{:06}", body.len()).as_bytes(),
        b"LYRICS200",
    ]
    .concat()
}
fn v1() -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes[..3].copy_from_slice(b"TAG");
    bytes[3..11].copy_from_slice(b"V1 title");
    bytes[93..97].copy_from_slice(b"2001");
    bytes[127] = 255;
    bytes
}
fn ape(header: bool, collision: bool) -> Vec<u8> {
    ape_named(header, collision, "Title")
}
fn ape_named(header: bool, collision: bool, name: &str) -> Vec<u8> {
    let value = if collision {
        let mut value = vec![b'x'; 180];
        // The final 96 item bytes start with TAG, hence EOF - 128.
        value[84..87].copy_from_slice(b"TAG");
        value
    } else {
        b"APE title".to_vec()
    };
    let mut items = (value.len() as u32).to_le_bytes().to_vec();
    items.extend(0u32.to_le_bytes());
    items.extend(name.as_bytes());
    items.push(0);
    items.extend(value);
    let block = |flags: u32| {
        let mut b = b"APETAGEX".to_vec();
        for n in [2000, items.len() as u32 + 32, 1, flags] {
            b.extend(n.to_le_bytes());
        }
        b.extend([0; 8]);
        b
    };
    let mut out = if header { block(0xa000_0000) } else { vec![] };
    out.extend(&items);
    out.extend(block(if header { 0x8000_0000 } else { 0 }));
    out
}
fn seed(path: &Path, has_ape: bool, has_v1: bool) -> Vec<u8> {
    let mut bytes = audio();
    if has_ape {
        bytes.extend(ape(true, false));
    }
    bytes.extend(lyrics());
    if has_v1 {
        bytes.extend(v1());
    }
    fs::write(path, &bytes).unwrap();
    bytes
}
fn options(mask: u8) -> TagOptions {
    let mut o = TagOptions::default();
    o.mpeg.read_ape = true;
    o.mpeg.remove_id3v1 = mask & 1 != 0;
    o.mpeg.remove_id3v2 = mask & 2 != 0;
    o.mpeg.remove_ape = mask & 8 != 0;
    o
}
fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle)
}

/// Independently authored Monkey's Audio 3.99 descriptor/header carrier.
/// Its bytes are a preservation control, with no decoder acceptance claim.
fn ape_carrier() -> Vec<u8> {
    let mut bytes = b"MAC ".to_vec();
    bytes.extend(3990u16.to_le_bytes());
    bytes.extend(0u16.to_le_bytes());
    bytes.extend(52u32.to_le_bytes());
    bytes.extend(24u32.to_le_bytes());
    bytes.extend([0; 36]);
    bytes.extend(2000u16.to_le_bytes());
    bytes.extend(0u16.to_le_bytes());
    for value in [1152u32, 1152, 1] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(44_100u32.to_le_bytes());
    bytes
}

fn assert_final_v1_title(bytes: &[u8], carrier: &[u8], title: &[u8]) {
    assert_eq!(
        bytes.len(),
        carrier.len() + 128,
        "unexpected residual physical tail"
    );
    assert_eq!(&bytes[..carrier.len()], carrier, "carrier bytes changed");
    let tail = &bytes[carrier.len()..];
    assert_eq!(&tail[..3], b"TAG");
    let mut expected = [0; 30];
    expected[..title.len()].copy_from_slice(title);
    assert_eq!(&tail[3..33], expected, "final physical V1 title");
    assert!(!contains(bytes, b"LYRICS200"));
    assert!(!contains(bytes, b"APETAGEX"));
}

#[test]
fn ape_family_save_removing_ape_retains_the_newly_written_physical_v1() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.ape");
    let carrier = ape_carrier();
    fs::write(
        &path,
        [carrier.clone(), ape(false, false), lyrics(), v1()].concat(),
    )
    .unwrap();
    let mut o = TagOptions::default();
    o.ape.write_id3v1 = true;
    o.ape.write_ape = false;
    o.ape.remove_id3v1 = false;
    o.ape.remove_id3v2 = false;
    o.ape.remove_ape = true;
    let fields: Fields = [("TITLE".into(), vec!["New V1 edit".into()])]
        .into_iter()
        .collect();
    write_fields_with_options(&path, &fields, false, &[], &o).unwrap();
    assert_final_v1_title(&fs::read(&path).unwrap(), &carrier, b"New V1 edit");
}

#[test]
fn ape_family_remove8_without_a_v1_write_retains_the_original_v1_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("remove.ape");
    let carrier = ape_carrier();
    fs::write(
        &path,
        [carrier.clone(), ape(false, false), lyrics(), v1()].concat(),
    )
    .unwrap();
    let mut o = TagOptions::default();
    o.ape.remove_id3v1 = false;
    o.ape.remove_id3v2 = false;
    o.ape.remove_ape = true;
    remove_tags_with_options(&path, false, &o).unwrap();
    let after = fs::read(&path).unwrap();
    assert_final_v1_title(&after, &carrier, b"V1 title");
    assert_eq!(&after[carrier.len() + 93..carrier.len() + 97], b"2001");
}

#[test]
fn custom_only_v1_save_replaces_owned_phantom_without_an_internal_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blank-v1.mp3");
    let ape = ape_named(false, true, "CUSTOM");
    let expected_prefix = [audio(), ape, lyrics()].concat();
    fs::write(&path, &expected_prefix).unwrap();
    let mut o = options(1);
    o.mpeg.write_id3v1 = true;
    o.mpeg.write_id3v2 = false;
    o.mpeg.write_ape = false;
    let fields: Fields = [("CUSTOM".into(), vec!["Changed custom".into()])]
        .into_iter()
        .collect();
    write_fields_with_options(&path, &fields, false, &[], &o).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        bytes.len(),
        expected_prefix.len() + 128,
        "owned phantom remained inside the physical tail"
    );
    assert!(
        bytes.starts_with(&expected_prefix),
        "encoded carrier, original APE bytes or Lyrics position changed"
    );
    assert_eq!(
        &bytes[expected_prefix.len()..expected_prefix.len() + 3],
        b"TAG"
    );
    assert_eq!(
        &bytes[expected_prefix.len() + 3..expected_prefix.len() + 33],
        &[0; 30]
    );
    let mut phantom = [0; 128];
    phantom[..3].copy_from_slice(b"TAG");
    phantom[32] = 255;
    phantom[93..97].copy_from_slice(b"0000");
    phantom[127] = 255;
    assert!(
        !contains(&bytes, &phantom),
        "owned placeholder leaked into the published file"
    );
    let entry = read_entry_with_options(&path, &[], &o).unwrap();
    assert!(entry.technical["_TAG"].join(" ").contains("APE"));
    assert!(entry.technical["_TAG"].join(" ").contains("ID3v1"));
}

#[test]
fn opaque_v2_read_exposes_ape_without_inventing_v1_or_changing_source() {
    let dir = tempfile::tempdir().unwrap();
    for header in [false, true] {
        for collision in [false, true] {
            for real_v1 in [false, true] {
                let path = dir.path().join("read.mp3");
                let mut bytes = audio();
                bytes.extend(ape(header, collision));
                bytes.extend(lyrics());
                if real_v1 {
                    bytes.extend(v1());
                }
                fs::write(&path, &bytes).unwrap();
                let entry = read_entry_with_options(&path, &[], &options(1)).unwrap();
                let inventory = entry
                    .technical
                    .get("_TAG")
                    .cloned()
                    .unwrap_or_default()
                    .join(" ");
                assert!(
                    inventory.contains("APE"),
                    "missing APE: header={header} collision={collision} v1={real_v1}: {inventory}"
                );
                assert_eq!(
                    inventory.contains("ID3v1"),
                    real_v1,
                    "fabricated or lost V1: {inventory}"
                );
                assert_eq!(fs::read(&path).unwrap(), bytes);
            }
        }
    }
}

#[test]
fn public_file_remove_and_utils_remove_orphan_without_v1_inventory() {
    let dir = tempfile::tempdir().unwrap();
    for utils in [false, true] {
        let path = dir.path().join("orphan.mp3");
        seed(&path, false, false);
        let before = read_entry_with_options(&path, &[], &options(1)).unwrap();
        assert!(!before
            .technical
            .get("_TAG")
            .is_some_and(|v| v.iter().any(|s| s.contains("ID3v1"))));
        if utils {
            remove_tag_kinds_with_options(&path, &[RemovableTag::Id3v1], false, &options(1))
                .unwrap();
        } else {
            remove_tags_with_options(&path, false, &options(1)).unwrap();
        }
        assert_eq!(fs::read(&path).unwrap(), audio(), "utils={utils}");
    }
}

#[test]
fn selected_v1_and_ape_removals_drop_v2_and_preserve_encoded_audio() {
    let dir = tempfile::tempdir().unwrap();
    for mask in [1, 9, 8] {
        let path = dir.path().join("remove.mp3");
        seed(&path, true, true);
        remove_tags_with_options(&path, false, &options(mask)).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(!contains(&bytes, b"LYRICS200"), "mask={mask}");
        assert!(
            bytes.starts_with(&audio()),
            "encoded frames changed: mask={mask}"
        );
        assert_eq!(contains(&bytes, b"APETAGEX"), mask == 1);
        let has_v1 = bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG";
        assert_eq!(has_v1, mask == 8);
        if mask == 8 {
            assert!(contains(&bytes[bytes.len() - 128..], b"V1 title"));
        }
    }
}

#[test]
fn utils_order_does_not_change_dependent_tail_removal() {
    let dir = tempfile::tempdir().unwrap();
    for header in [false, true] {
        for kinds in [
            [RemovableTag::Id3v1, RemovableTag::Ape],
            [RemovableTag::Ape, RemovableTag::Id3v1],
        ] {
            let path = dir.path().join("order.mp3");
            fs::write(&path, [audio(), ape(header, true), lyrics(), v1()].concat()).unwrap();
            remove_tag_kinds_with_options(&path, &kinds, false, &options(9)).unwrap();
            assert_eq!(fs::read(&path).unwrap(), audio());
        }
    }
}

#[test]
fn ordinary_nonempty_ape_write_preserves_raw_lyrics_and_unwritten_v1_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("write.mp3");
    seed(&path, true, true);
    let suffix = [lyrics(), v1()].concat();
    let mut o = options(8);
    o.mpeg.write_id3v1 = false;
    o.mpeg.write_id3v2 = false;
    o.mpeg.write_ape = true;
    let fields: Fields = [("TITLE".into(), vec!["Changed".into()])]
        .into_iter()
        .collect();
    for _ in 0..2 {
        write_fields_with_options(&path, &fields, false, &[], &o).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.ends_with(&suffix));
        assert!(bytes.starts_with(&audio()));
        assert_eq!(
            bytes
                .windows(lyrics().len())
                .filter(|w| *w == lyrics())
                .count(),
            1
        );
    }
}

#[test]
fn malformed_legacy_and_unselected_orphans_remain_exact() {
    let dir = tempfile::tempdir().unwrap();
    for tail in [
        b"LYRICSBEGINlegacyLYRICSEND".to_vec(),
        b"LYRICSBEGINopaque000001LYRICS200".to_vec(),
        lyrics(),
    ] {
        let path = dir.path().join("preserve.mp3");
        let bytes = [audio(), tail].concat();
        fs::write(&path, &bytes).unwrap();
        remove_tags_with_options(&path, false, &options(8)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn known_empty_save_removes_the_selected_dependent_lyrics_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.mp3");
    seed(&path, true, true);
    let mut o = options(8);
    o.mpeg.write_id3v1 = true;
    o.mpeg.write_id3v2 = false;
    o.mpeg.write_ape = true;
    let fields: Fields = [("TITLE".into(), vec![])].into_iter().collect();
    write_fields_with_options(&path, &fields, false, &[], &o).unwrap();
    assert_eq!(fs::read(&path).unwrap(), audio());
}

#[test]
fn admitted_remove_has_exact_whole_file_undo_and_redo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.mp3");
    let before = seed(&path, true, true);
    let mut journal = Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
    let capture = journal.begin("Remove tags", None, &[path.to_string_lossy().into_owned()]);
    remove_tags_with_options(&path, false, &options(9)).unwrap();
    journal.commit(capture, &[], &[]);
    assert!(
        journal.state().notice.is_none(),
        "current operation was not admitted"
    );
    assert_eq!(journal.state().undo.as_ref().unwrap().files, 1);
    let after = fs::read(&path).unwrap();
    assert_eq!(after, audio());
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), after);
}

#[test]
fn lyrics_skip_composes_with_physical_id3_extended_and_encrypted_overlays() {
    let dir = tempfile::tempdir().unwrap();
    for encrypted in [false, true] {
        let path = dir.path().join("overlay.mp3");
        let frame = |id: &[u8], flags: u8, value: &[u8]| {
            let mut bytes = id.to_vec();
            bytes.extend((value.len() as u32).to_be_bytes());
            bytes.extend([0, flags]);
            bytes.extend(value);
            bytes
        };
        let mut body = 6u32.to_be_bytes().to_vec();
        body.extend([0; 6]);
        body.extend(frame(b"TIT2", 0, b"\0ID3 title"));
        if encrypted {
            body.extend(frame(b"ENCR", 0, b"example.invalid\0\x81fixture"));
            body.extend(frame(b"TXXX", 0x40, b"\x81opaque cipher"));
        }
        body.extend([0; 32]);
        let mut prefix = b"ID3\x03\0\x40".to_vec();
        prefix.extend([21, 14, 7, 0].map(|shift| ((body.len() >> shift) & 127) as u8));
        prefix.extend(body);
        let bytes = [prefix.clone(), audio(), ape(false, true), lyrics()].concat();
        fs::write(&path, &bytes).unwrap();
        let read = read_entry_with_options(&path, &[], &options(1)).unwrap();
        assert!(read.technical["_TAG"].join(" ").contains("APE"));
        assert!(!read.technical["_TAG"].join(" ").contains("ID3v1"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let mut o = options(8);
        o.mpeg.write_id3v1 = false;
        o.mpeg.write_id3v2 = false;
        o.mpeg.write_ape = true;
        let fields: Fields = [("TITLE".into(), vec!["Changed".into()])]
            .into_iter()
            .collect();
        write_fields_with_options(&path, &fields, false, &[], &o).unwrap();
        let after = fs::read(&path).unwrap();
        assert!(
            after.starts_with(&prefix),
            "physical ID3 provenance changed"
        );
        assert!(after.ends_with(&lyrics()));
        assert!(contains(&after, &audio()));
    }
}

#[test]
fn raw_stream_stand_in_restores_the_guard_before_reassembly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owned.tta");
    let mut carrier = b"TTA1".to_vec();
    for value in [1u16, 2, 16] {
        carrier.extend(value.to_le_bytes());
    }
    carrier.extend(44_100u32.to_le_bytes());
    carrier.extend(44_100u32.to_le_bytes());
    let mut crc = !0u32;
    for byte in &carrier {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    carrier.extend((!crc).to_le_bytes());
    carrier.extend((0..1024).map(|i| (i * 37 % 89 + 1) as u8));
    let suffix = [lyrics(), v1()].concat();
    fs::write(
        &path,
        [carrier.clone(), ape(false, true), suffix.clone()].concat(),
    )
    .unwrap();
    let read = read_entry_with_options(&path, &[], &options(9)).unwrap();
    assert!(read.technical["_TAG"].join(" ").contains("APE"));
    let mut o = options(8);
    o.mpeg.write_id3v1 = false;
    o.mpeg.write_id3v2 = false;
    o.mpeg.write_ape = true;
    let fields: Fields = [("TITLE".into(), vec!["Changed".into()])]
        .into_iter()
        .collect();
    write_fields_with_options(&path, &fields, false, &[], &o).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(after.starts_with(&carrier));
    assert!(after.ends_with(&suffix));
    remove_tags_with_options(&path, false, &options(9)).unwrap();
    assert_eq!(fs::read(&path).unwrap(), carrier);
}
