//! Cover plans must verify the written tag, while retaining unread/unwritten kinds.
use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{
    config::WriteOptions,
    id3::v2::{AttachedPictureFrame, Frame, Id3v2Tag},
    picture::{Picture, PictureType},
    prelude::TagExt,
    TextEncoding,
};
use std::{fs, io::Cursor, ops::Range, path::Path};
use tag_core::{
    apply_cover_changes_with_options, cover_digest,
    history::{Journal, Limits},
    read_entry_summary_with_options, read_entry_with_options, CoverChange, CoverRef, TagOptions,
};

fn png(color: [u8; 3]) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(2, 2, image::Rgb(color))
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

fn le32(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

fn tail_end(bytes: &[u8]) -> usize {
    if bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG" {
        bytes.len() - 128
    } else {
        bytes.len()
    }
}

/// Structural observers compare the encoded carrier and complete retained APE tag.
fn ape_span(bytes: &[u8]) -> Option<Range<usize>> {
    let end = tail_end(bytes);
    if end < 32 || &bytes[end - 32..end - 24] != b"APETAGEX" {
        return None;
    }
    let size = le32(bytes, end - 20);
    assert!(size >= 32 && size <= end);
    let header = if le32(bytes, end - 12) & 0x8000_0000 != 0 {
        32
    } else {
        0
    };
    Some(end - size - header..end)
}

fn audio(bytes: &[u8]) -> &[u8] {
    let start = if bytes.starts_with(b"ID3") {
        10 + bytes[6..10]
            .iter()
            .fold(0usize, |n, b| (n << 7) | usize::from(*b))
    } else {
        0
    };
    let end = ape_span(bytes).map_or_else(|| tail_end(bytes), |span| span.start);
    &bytes[start..end]
}

fn raw_ape(image: &[u8]) -> Vec<u8> {
    let mut items = Vec::new();
    for (key, flags, value) in [
        ("Title", 0u32, b"Retained APE title".to_vec()),
        ("Opaque record", 2, b"\x00\xffpreserve me".to_vec()),
        (
            "Cover Art (Front)",
            2,
            [b"ape.png\0".as_slice(), image].concat(),
        ),
    ] {
        items.extend((value.len() as u32).to_le_bytes());
        items.extend(flags.to_le_bytes());
        items.extend(key.as_bytes());
        items.push(0);
        items.extend(value);
    }
    let block = |flags: u32| {
        let mut bytes = b"APETAGEX".to_vec();
        for value in [2000, items.len() as u32 + 32, 3, flags] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0u8; 8]);
        bytes
    };
    [block(0xa000_0000), items.clone(), block(0x8000_0000)].concat()
}

fn only_id3(options: &TagOptions) -> TagOptions {
    let mut options = options.clone();
    options.mpeg.read_id3v1 = false;
    options.mpeg.read_id3v2 = true;
    options.mpeg.read_ape = false;
    options
}

fn seed(path: &Path, id3_image: &[u8], ape_image: &[u8]) {
    let carrier = include_bytes!("fixtures/save-corpus/tone.mp3");
    fs::write(path, audio(carrier)).unwrap();
    let mut picture = Picture::from_reader(&mut Cursor::new(id3_image)).unwrap();
    picture.set_pic_type(PictureType::CoverFront);
    picture.set_description(Some("ID3 original".into()));
    let mut tag = Id3v2Tag::new();
    tag.insert(Frame::Picture(AttachedPictureFrame::new(
        TextEncoding::UTF16,
        picture,
    )));
    tag.save_to_path(path, WriteOptions::default().use_id3v23(true))
        .unwrap();
    let mut bytes = fs::read(path).unwrap();
    bytes.extend(raw_ape(ape_image));
    fs::write(path, bytes).unwrap();
}

fn run_plan(replacement: bool, remove: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("covers.mp3");
    let id3_image = png([255, 0, 0]);
    let ape_image = png([0, 0, 255]);
    let new_image = png([0, 255, 0]);
    assert_ne!(cover_digest(&id3_image), cover_digest(&ape_image));
    assert_ne!(cover_digest(&new_image), cover_digest(&ape_image));
    seed(&path, &id3_image, &ape_image);
    let mut options = TagOptions::default();
    options.mpeg.read_ape = true;
    assert!(options.mpeg.write_id3v1 && options.mpeg.write_id3v2);
    assert!(!options.mpeg.write_ape);
    let before = fs::read(&path).unwrap();
    let displayed = read_entry_summary_with_options(&path, &[], &options).unwrap();
    assert_eq!(displayed.covers.len(), 1);
    let original = &displayed.covers[0];
    assert_eq!(original.digest, cover_digest(&ape_image));
    let id3_before = read_entry_summary_with_options(&path, &[], &only_id3(&options)).unwrap();
    assert_eq!(id3_before.covers.len(), 1);
    assert_eq!(id3_before.covers[0].digest, cover_digest(&id3_image));
    let changes = if remove {
        Vec::new()
    } else {
        vec![CoverChange {
            original: Some(CoverRef::of(original)),
            image: replacement.then(|| new_image.clone()),
            picture_type: original.picture_type.clone(),
            // An image-only replacement also exercises equal type/description.
            description: if replacement {
                original.description.clone()
            } else {
                "Edited description".into()
            },
            ..Default::default()
        }]
    };
    let mut history = Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
    history.set_tag_options(options.clone());
    let capture = history.begin(
        "Manage covers",
        None,
        &[path.to_string_lossy().into_owned()],
    );
    let result = apply_cover_changes_with_options(&path, &changes, &options);
    assert!(
        result.is_ok(),
        "cover plan must verify ID3 artwork independently of retained APE: {result:?}"
    );
    history.commit(capture, &[], &[]);
    let after = fs::read(&path).unwrap();
    assert_ne!(after, before);
    assert_eq!(audio(&after), audio(&before), "encoded audio changed");
    assert_eq!(
        &after[ape_span(&after).unwrap()],
        &before[ape_span(&before).unwrap()],
        "unwritten APE tag changed"
    );
    let id3_after = read_entry_with_options(&path, &[], &only_id3(&options)).unwrap();
    if remove {
        assert!(id3_after.covers.is_empty());
    } else {
        assert_eq!(id3_after.covers.len(), 1);
        let written = &id3_after.covers[0];
        let expected = if replacement { &new_image } else { &ape_image };
        assert_eq!(written.digest, cover_digest(expected));
        assert_eq!(STANDARD.decode(&written.data_base64).unwrap(), *expected);
        assert_eq!(written.picture_type, changes[0].picture_type);
        assert_eq!(written.description, changes[0].description);
    }
    // Normal reads continue to show the retained, higher-priority APE artwork.
    let visible = read_entry_summary_with_options(&path, &[], &options).unwrap();
    assert_eq!(visible.covers.len(), 1);
    assert_eq!(visible.covers[0].digest, original.digest);
    assert_eq!(visible.covers[0].description, original.description);
    assert!(history.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(history.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(&path).unwrap(), after);
}

#[test]
fn replace_cover_verifies_image_bytes_in_written_id3_while_retaining_ape() {
    run_plan(true, false);
}

#[test]
fn set_cover_description_verifies_written_id3_while_retaining_ape() {
    run_plan(false, false);
}

#[test]
fn remove_displayed_cover_verifies_written_id3_while_retaining_ape() {
    run_plan(false, true);
}
