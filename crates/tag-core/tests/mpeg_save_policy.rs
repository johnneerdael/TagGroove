//! Real owned MPEG carrier controls for the bounded native seed/mask contract.
#[allow(dead_code)]
#[path = "support/save_corpus.rs"]
mod corpus;
use lofty::{
    config::WriteOptions,
    file::AudioFile,
    id3::{
        v1::Id3v1Tag,
        v2::{
            AttachedPictureFrame, BinaryFrame, ChapterFrame, Frame, FrameId, Id3v2Tag,
            PrivateFrame, TextInformationFrame,
        },
    },
    prelude::TagExt,
    TextEncoding,
};
use std::{fs, ops::Range, path::Path};
use tag_core::{
    history::{Journal, Limits},
    read_entry, read_entry_with_options, remove_tags_with_options, write_fields_with_options,
    Fields, TagOptions,
};

fn updates(name: &str, values: &[&str]) -> Fields {
    [(name.into(), values.iter().map(|s| (*s).into()).collect())]
        .into_iter()
        .collect()
}
fn le32(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}
/// Independent structural tail observer, including the optional APE header.
fn ape_span(bytes: &[u8]) -> Option<Range<usize>> {
    let end = if bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG" {
        bytes.len() - 128
    } else {
        bytes.len()
    };
    if end < 32 || &bytes[end - 32..end - 24] != b"APETAGEX" {
        return None;
    }
    let footer = end - 32;
    let size = le32(bytes, footer + 12);
    assert!(size >= 32 && size <= end);
    let start = end
        - size
        - if le32(bytes, footer + 20) & 0x8000_0000 != 0 {
            32
        } else {
            0
        };
    if start < end - size {
        assert_eq!(&bytes[start..start + 8], b"APETAGEX");
    }
    Some(start..end)
}
fn ape_bytes(bytes: &[u8]) -> Vec<u8> {
    ape_span(bytes)
        .map(|r| bytes[r].to_vec())
        .unwrap_or_default()
}
fn v1(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG" {
        bytes[bytes.len() - 128..].to_vec()
    } else {
        vec![]
    }
}
fn prefix(bytes: &[u8]) -> Vec<u8> {
    if bytes.starts_with(b"ID3") {
        let len = bytes[6..10]
            .iter()
            .fold(0usize, |n, b| (n << 7) | usize::from(*b));
        bytes[..10 + len].to_vec()
    } else {
        vec![]
    }
}
fn raw_ape(opaque: bool) -> Vec<u8> {
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(2, 2, image::Rgb([7, 11, 13]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let cover_value = [b"fixture.png\0".as_slice(), png.get_ref().as_slice()].concat();
    let records: Vec<(&str, u32, &[u8])> = [
        ("Title", 0, b"APE title".as_slice()),
        ("Album", 0, b"APE album".as_slice()),
        ("REPLAYGAIN_TRACK_GAIN", 0, b"-3.00 dB".as_slice()),
        ("MP3GAIN_UNDO", 0, b"+001,+001,N".as_slice()),
        ("MP3GAIN_MINMAX", 0, b"094,205".as_slice()),
        ("Odd binary key", 3, b"\x01\x00\xffopaque".as_slice()),
        ("Cover Art (Front)", 2, cover_value.as_slice()),
    ]
    .into_iter()
    .filter(|(name, _, _)| opaque || *name == "Title" || *name == "Album")
    .collect();
    raw_ape_records(&records)
}
fn raw_ape_records(records: &[(&str, u32, &[u8])]) -> Vec<u8> {
    let mut items = Vec::new();
    for (name, flags, value) in records {
        items.extend((value.len() as u32).to_le_bytes());
        items.extend(flags.to_le_bytes());
        items.extend(name.as_bytes());
        items.push(0);
        items.extend(*value);
    }
    let count = records.len() as u32;
    let block = |flags: u32| {
        let mut v = b"APETAGEX".to_vec();
        for n in [2000, items.len() as u32 + 32, count, flags] {
            v.extend(n.to_le_bytes());
        }
        v.extend([0u8; 8]);
        v
    };
    let mut out = block(0xa000_0000);
    out.extend(&items);
    out.extend(block(0x8000_0000));
    out
}
fn seed(path: &Path, id3: bool, ape: bool, legacy: bool, opaque: bool) {
    fs::write(path, include_bytes!("fixtures/save-corpus/tone.mp3")).unwrap();
    // Start from the encoded frames, independent of the fixture's original tags.
    let audio = corpus::payload(path);
    fs::write(path, audio).unwrap();
    if id3 {
        let mut tag = Id3v2Tag::new();
        for (id, text) in [("TIT2", "ID3 title"), ("TALB", "ID3 album")] {
            tag.insert(Frame::Text(TextInformationFrame::new(
                FrameId::new(id).unwrap(),
                TextEncoding::UTF16,
                text.to_owned(),
            )));
        }
        tag.insert_user_text("REPLAYGAIN_TRACK_GAIN".into(), "-7.25 dB".into());
        tag.save_to_path(path, WriteOptions::default().use_id3v23(true))
            .unwrap();
    }
    if ape {
        let mut b = fs::read(path).unwrap();
        b.extend(raw_ape(opaque));
        fs::write(path, b).unwrap();
    }
    if legacy {
        let mut tag = Id3v1Tag::new();
        tag.title = Some("Old v1".into());
        tag.album = Some("Old album".into());
        tag.save_to_path(path, WriteOptions::default()).unwrap();
    }
}
fn only(kind: &str) -> TagOptions {
    let mut o = TagOptions::default();
    o.mpeg.read_id3v1 = kind == "v1";
    o.mpeg.read_id3v2 = kind == "v2";
    o.mpeg.read_ape = kind == "ape";
    o
}
fn field(path: &Path, kind: &str, key: &str) -> Vec<String> {
    read_entry_with_options(path, &[], &only(kind))
        .unwrap()
        .fields
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
        .unwrap_or_default()
}
fn options(read_ape: bool, write: [bool; 3]) -> TagOptions {
    let mut o = TagOptions::default();
    o.mpeg.read_ape = read_ape;
    [o.mpeg.write_id3v1, o.mpeg.write_id3v2, o.mpeg.write_ape] = write;
    o
}

#[test]
fn fresh_seed_reads_id3_and_explicit_ape_opt_in_is_read_only() {
    assert!(!TagOptions::default().mpeg.read_ape);
    assert!(TagOptions::default().ape.read_ape);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("read.mp3");
    seed(&path, true, true, true, true);
    let bytes = fs::read(&path).unwrap();
    assert_eq!(read_entry(&path).unwrap().fields["TITLE"], ["ID3 title"]);
    assert_eq!(field(&path, "ape", "TITLE"), ["APE title"]);
    assert_eq!(field(&path, "v1", "TITLE"), ["Old v1"]);
    let explicit: TagOptions =
        serde_json::from_value(serde_json::json!({"mpeg":{"read_ape":true}})).unwrap();
    assert!(explicit.mpeg.read_ape);
    assert_eq!(
        read_entry_with_options(&path, &[], &explicit)
            .unwrap()
            .fields["Title"],
        ["APE title"]
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
#[test]
fn ordinary_mpeg_save_preserves_entire_unwritten_ape_and_kind_specific_gain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.mp3");
    for read in [false, true] {
        for remove in [false, true] {
            seed(&path, true, true, true, true);
            let before = fs::read(&path).unwrap();
            let payload = corpus::payload(&path);
            let mut o = options(read, [true, true, false]);
            o.mpeg.remove_ape = remove;
            write_fields_with_options(&path, &updates("TITLE", &["New ID3 title"]), false, &[], &o)
                .unwrap();
            assert_eq!(
                ape_bytes(&fs::read(&path).unwrap()),
                ape_bytes(&before),
                "read={read}/remove={remove}"
            );
            assert_eq!(field(&path, "v2", "TITLE"), ["New ID3 title"]);
            assert_eq!(field(&path, "ape", "TITLE"), ["APE title"]);
            assert_eq!(
                field(&path, "v2", "REPLAYGAIN_TRACK_GAIN"),
                [if read { "-3.00 dB" } else { "-7.25 dB" }]
            );
            if !read {
                assert!(field(&path, "v2", "MP3GAIN_UNDO").is_empty());
            }
            assert_eq!(corpus::payload(&path), payload);
        }
    }
}
#[test]
fn ape_only_hidden_values_and_covers_are_not_imported_or_deleted_by_mpeg_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hidden.mp3");
    seed(&path, false, true, false, true);
    let before = fs::read(&path).unwrap();
    let visible_ape = read_entry_with_options(&path, &[], &only("ape")).unwrap();
    assert_eq!(
        visible_ape.covers.len(),
        1,
        "seed must contain real APE artwork"
    );
    let o = options(false, [true, true, false]);
    let read = read_entry_with_options(&path, &[], &o).unwrap();
    assert!(read.fields.is_empty() && read.covers.is_empty());
    assert!(read.technical["_TAG"].iter().any(|v| v.contains("APE")));
    write_fields_with_options(&path, &updates("ARTIST", &["Edited"]), false, &[], &o).unwrap();
    assert_eq!(ape_bytes(&fs::read(&path).unwrap()), ape_bytes(&before));
    assert!(read_entry_with_options(&path, &[], &only("v2"))
        .unwrap()
        .covers
        .is_empty());
    assert!(field(&path, "v2", "TITLE").is_empty());
    assert!(field(&path, "v2", "MP3GAIN_UNDO").is_empty());
    assert_eq!(field(&path, "ape", "TITLE"), ["APE title"]);
}
#[test]
fn ape_only_writes_preserve_all_unwritten_v1_bytes_and_use_written_kind_verification() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("write8.mp3");
    for read in [false, true] {
        for legacy in [false, true] {
            seed(&path, true, true, legacy, true);
            let before = fs::read(&path).unwrap();
            let payload = corpus::payload(&path);
            let o = options(read, [false, false, true]);
            write_fields_with_options(&path, &updates("TITLE", &["New APE title"]), false, &[], &o)
                .unwrap();
            let after = fs::read(&path).unwrap();
            assert_eq!(v1(&after), v1(&before));
            assert_eq!(prefix(&after), prefix(&before));
            assert_eq!(field(&path, "ape", "TITLE"), ["New APE title"]);
            assert_eq!(corpus::payload(&path), payload);
            let shown = read_entry_with_options(&path, &[], &o).unwrap();
            assert_eq!(
                shown
                    .fields
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case("TITLE"))
                    .unwrap()
                    .1,
                &vec![if read {
                    "New APE title".to_owned()
                } else {
                    "ID3 title".to_owned()
                }]
            );
        }
    }
}
#[test]
fn v1_plus_ape_writes_update_only_the_two_selected_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("write9.mp3");
    seed(&path, true, true, true, true);
    let before = fs::read(&path).unwrap();
    let o = options(false, [true, false, true]);
    write_fields_with_options(&path, &updates("TITLE", &["New both"]), false, &[], &o).unwrap();
    assert_eq!(field(&path, "v1", "TITLE"), ["New both"]);
    assert_eq!(field(&path, "ape", "TITLE"), ["New both"]);
    assert_eq!(prefix(&fs::read(&path).unwrap()), prefix(&before));
}
#[test]
fn no_op_and_true_empty_writes_preserve_unwritten_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.mp3");
    for write in [
        [true, true, false],
        [false, false, true],
        [true, false, false],
    ] {
        seed(&path, true, true, true, false);
        let before = fs::read(&path).unwrap();
        let payload = corpus::payload(&path);
        let o = options(write[2], write);
        let existing = read_entry_with_options(&path, &[], &o).unwrap().fields;
        let empty_update =
            write_fields_with_options(&path, &Fields::new(), false, &[], &o).unwrap();
        assert_eq!(
            empty_update.fields, existing,
            "empty UPDATE keeps the nonempty supplied model"
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            before,
            "empty UPDATE changed tag presence/raw bytes"
        );
        assert_eq!(corpus::payload(&path), payload);
        write_fields_with_options(&path, &existing, false, &[], &o).unwrap();
        let no_op = fs::read(&path).unwrap();
        if !write[2] {
            assert_eq!(ape_bytes(&no_op), ape_bytes(&before));
        }
        if !write[0] {
            assert_eq!(v1(&no_op), v1(&before));
        }
        let clear = read_entry_with_options(&path, &[], &o)
            .unwrap()
            .fields
            .into_keys()
            .map(|k| (k, vec![]))
            .collect();
        write_fields_with_options(&path, &clear, false, &[], &o).unwrap();
        let after = fs::read(&path).unwrap();
        if write[2] {
            assert!(ape_span(&after).is_none());
            assert_eq!(field(&path, "v1", "TITLE"), ["Old v1"]);
        } else {
            assert_eq!(ape_bytes(&after), ape_bytes(&before));
        }
        if write[0] {
            assert!(v1(&after).is_empty());
        }
        if write[1] {
            assert!(prefix(&after).is_empty());
        }
        assert_eq!(corpus::payload(&path), payload);
    }
}
#[test]
fn explicit_remove_and_undo_redo_remain_independent_of_read_and_save_masks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.mp3");
    seed(&path, true, true, true, true);
    let before = fs::read(&path).unwrap();
    let payload = corpus::payload(&path);
    let o = options(false, [true, true, false]);
    let mut journal = Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
    let capture = journal.begin(
        "MPEG preservation",
        None,
        &[path.to_string_lossy().into_owned()],
    );
    write_fields_with_options(&path, &updates("ARTIST", &["New artist"]), false, &[], &o).unwrap();
    journal.commit(capture, &[], &[]);
    assert!(journal.state().notice.is_none());
    assert_eq!(
        journal.state().undo.as_ref().unwrap().label,
        "MPEG preservation"
    );
    let saved = fs::read(&path).unwrap();
    assert_eq!(ape_bytes(&saved), ape_bytes(&before));
    let mut bad = saved.clone();
    let span = ape_span(&bad).unwrap();
    bad[span.start + 40] ^= 1;
    assert_ne!(ape_bytes(&bad), ape_bytes(&before));
    let mut audio_bad = saved.clone();
    let audio_start = prefix(&saved).len();
    audio_bad[audio_start + 30] ^= 1;
    let audio_end = ape_span(&audio_bad).unwrap().start;
    assert_ne!(&audio_bad[audio_start..audio_end], payload.as_slice());
    assert_eq!(
        fs::read(&path).unwrap(),
        saved,
        "in-memory mutation changed the file"
    );
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), saved);
    let mut remove = o.clone();
    remove.mpeg.remove_id3v1 = false;
    remove.mpeg.remove_id3v2 = false;
    remove.mpeg.remove_ape = true;
    remove_tags_with_options(&path, false, &remove).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(ape_span(&after).is_none());
    assert_eq!(prefix(&after), prefix(&saved));
    assert_eq!(v1(&after), v1(&saved));
    assert_eq!(corpus::payload(&path), payload);
}

#[test]
fn nonempty_custom_only_model_writes_blank_v1_without_removing_other_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("custom-only.mp3");
    for legacy in [false, true] {
        seed(&path, false, false, legacy, false);
        let mut tag = Id3v2Tag::new();
        tag.insert_user_text("CUSTOM".into(), "Retained".into());
        tag.save_to_path(&path, WriteOptions::default().use_id3v23(true))
            .unwrap();
        let original = fs::read(&path).unwrap();
        let audio = corpus::payload(&path);
        let o = options(false, [true, false, false]);
        write_fields_with_options(&path, &updates("CUSTOM", &["Changed"]), false, &[], &o).unwrap();
        let saved = fs::read(&path).unwrap();
        assert_eq!(v1(&saved).len(), 128);
        assert!(field(&path, "v1", "TITLE").is_empty());
        assert_eq!(prefix(&saved), prefix(&original));
        assert_eq!(corpus::payload(&path), audio);
    }
}

#[test]
fn disabled_writers_invalid_values_and_v1_artwork_still_refuse_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("refusal.mp3");
    seed(&path, true, true, true, true);
    let original = fs::read(&path).unwrap();
    let disabled = options(false, [false, false, false]);
    assert!(write_fields_with_options(
        &path,
        &updates("TITLE", &["Rejected"]),
        false,
        &[],
        &disabled
    )
    .is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(write_fields_with_options(
        &path,
        &updates("TITLE", &["bad\0value"]),
        false,
        &[],
        &options(false, [true, true, false])
    )
    .is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(1, 1, image::Rgb([1, 2, 3]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let error = tag_core::add_cover_bytes_with_options(
        &path,
        png.into_inner(),
        "",
        None,
        &options(false, [true, false, false]),
    )
    .unwrap_err();
    assert!(error.contains("ID3v1 cannot store cover art"));
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn plain_id3_empty_model_removes_hidden_written_ape_art_and_binary_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hidden-ape.mp3");
    seed(&path, true, true, true, true);
    let o = options(false, [false, false, true]);
    let before = fs::read(&path).unwrap();
    let audio = corpus::payload(&path);
    assert!(read_entry_with_options(&path, &[], &o)
        .unwrap()
        .covers
        .is_empty());
    assert_eq!(
        read_entry_with_options(&path, &[], &only("ape"))
            .unwrap()
            .covers
            .len(),
        1
    );
    let clear = read_entry_with_options(&path, &[], &o)
        .unwrap()
        .fields
        .into_keys()
        .map(|k| (k, vec![]))
        .collect();
    write_fields_with_options(&path, &clear, false, &[], &o).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(ape_span(&after).is_none(),"known-empty plain source must clear the entire written APE, including hidden artwork/binary");
    assert_eq!(prefix(&after), prefix(&before));
    assert_eq!(v1(&after), v1(&before));
    assert_eq!(corpus::payload(&path), audio);
}
#[test]
fn plain_ape_empty_model_removes_hidden_written_id3_art_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hidden-id3.mp3");
    seed(&path, false, true, true, false);
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(2, 2, image::Rgb([19, 23, 29]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let picture =
        lofty::picture::Picture::from_reader(&mut std::io::Cursor::new(png.into_inner())).unwrap();
    let mut tag = Id3v2Tag::new();
    tag.insert(Frame::Picture(AttachedPictureFrame::new(
        TextEncoding::UTF16,
        picture,
    )));
    tag.save_to_path(&path, WriteOptions::default().use_id3v23(true))
        .unwrap();
    let o = options(true, [false, true, false]);
    let before = fs::read(&path).unwrap();
    let audio = corpus::payload(&path);
    assert!(read_entry_with_options(&path, &[], &o)
        .unwrap()
        .covers
        .is_empty());
    assert_eq!(
        read_entry_with_options(&path, &[], &only("v2"))
            .unwrap()
            .covers
            .len(),
        1
    );
    let clear = read_entry_with_options(&path, &[], &o)
        .unwrap()
        .fields
        .into_keys()
        .map(|k| (k, vec![]))
        .collect();
    write_fields_with_options(&path, &clear, false, &[], &o).unwrap();
    let after = fs::read(&path).unwrap();
    assert!(
        prefix(&after).is_empty(),
        "known-empty plain source must clear written ID3 artwork"
    );
    assert_eq!(ape_bytes(&after), ape_bytes(&before));
    assert_eq!(v1(&after), v1(&before));
    assert_eq!(corpus::payload(&path), audio);
}
#[test]
fn displayed_ape_binary_records_survive_last_text_field_clear() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source-binary.mp3");
    seed(&path, false, false, true, false);
    let binary = ("Associated binary", 3, b"\x00\xffopaque-source".as_slice());
    let mut original = fs::read(&path).unwrap();
    let legacy = v1(&original);
    original.truncate(original.len() - legacy.len());
    original.extend(raw_ape_records(&[("Title", 0, b"Displayed"), binary]));
    original.extend(&legacy);
    fs::write(&path, &original).unwrap();
    let o = options(true, [false, false, true]);
    let before = read_entry_with_options(&path, &[], &o).unwrap();
    assert_eq!(before.fields["Title"], ["Displayed"]);
    assert!(before.covers.is_empty());
    let audio = corpus::payload(&path);
    write_fields_with_options(&path, &updates("TITLE", &[]), false, &[], &o).unwrap();
    let after = fs::read(&path).unwrap();
    // The selected writer may normalize the tag envelope. The complete
    // remaining item (length, flags, key and data) must remain identical.
    let actual = ape_bytes(&after);
    let expected = raw_ape_records(&[binary]);
    assert_eq!(
        &actual[32..actual.len() - 32],
        &expected[32..expected.len() - 32]
    );
    assert_eq!(v1(&after), legacy);
    assert_eq!(corpus::payload(&path), audio);
}
#[test]
fn displayed_id3_opaque_and_chapter_source_is_not_whole_tag_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("opaque-source.mp3");
    for chapter in [false, true] {
        seed(&path, false, true, true, false);
        let mut tag = Id3v2Tag::new();
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new("TIT2").unwrap(),
            TextEncoding::UTF16,
            "Displayed",
        )));
        if chapter {
            tag.insert(Frame::Chapter(ChapterFrame::new(
                "chapter1",
                0..1000,
                u32::MAX..u32::MAX,
                Default::default(),
            )));
        } else {
            tag.insert(Frame::Binary(BinaryFrame::new(
                FrameId::new("XABC").unwrap(),
                vec![1, 0, 255, 17],
            )));
            tag.insert(Frame::Private(PrivateFrame::new(
                "independent.owner",
                vec![3, 0, 7],
            )));
        }
        tag.save_to_path(&path, WriteOptions::default().use_id3v23(true))
            .unwrap();
        let before = fs::read(&path).unwrap();
        let audio = corpus::payload(&path);
        let o = options(false, [false, true, false]);
        write_fields_with_options(&path, &updates("TITLE", &[]), false, &[], &o).unwrap();
        let after = fs::read(&path).unwrap();
        assert!(!prefix(&after).is_empty());
        assert!(field(&path, "v2", "TITLE").is_empty());
        assert_eq!(ape_bytes(&after), ape_bytes(&before));
        assert_eq!(v1(&after), v1(&before));
        assert_eq!(corpus::payload(&path), audio);
        let native = lofty::mpeg::MpegFile::read_from(
            &mut fs::File::open(&path).unwrap(),
            lofty::config::ParseOptions::new(),
        )
        .unwrap();
        let tag = native.id3v2().unwrap();
        if chapter {
            assert!(tag.into_iter().any(|f| matches!(f, Frame::Chapter(_))));
        } else {
            assert!(tag.into_iter().any(|f|matches!(f,Frame::Binary(b) if b.id().as_str()=="XABC" && b.data.as_ref()==[1,0,255,17])));
            assert!(tag.into_iter().any(|f| matches!(f, Frame::Private(_))));
        }
    }
}
