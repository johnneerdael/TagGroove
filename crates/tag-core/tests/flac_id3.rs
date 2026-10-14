//! FLAC files with a prepended ID3v2 tag: Mp3tag reads only the Vorbis
//! comments and removes the ID3v2 tag (file-formats: FLAC removes Vorbis
//! Comments and ID3v2). The FLAC stream after the tag is kept byte for byte.
use lofty::{
    config::WriteOptions,
    id3::v2::{AttachedPictureFrame, Frame, FrameId, Id3v2Tag, TextInformationFrame},
    picture::{MimeType, Picture, PictureType},
    prelude::TagExt,
    TextEncoding,
};
use std::fs;
use tag_core::{add_cover, read_entry, remove_tags, write_fields, Fields};

fn fields(values: &[(&str, &[&str])]) -> Fields {
    values
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

/// fLaC, a STREAMINFO block (44.1 kHz, stereo, 16 bits, one second), a
/// Vorbis comment block with TITLE, and opaque frame bytes.
fn flac(title: &str) -> Vec<u8> {
    let mut bytes = b"fLaC".to_vec();
    let mut info = Vec::new();
    info.extend(4096u16.to_be_bytes());
    info.extend(4096u16.to_be_bytes());
    info.extend([0; 6]);
    let packed: u64 = (44_100u64 << 44) | (1 << 41) | (15 << 36) | 44_100;
    info.extend(packed.to_be_bytes());
    info.extend([0; 16]);
    bytes.push(0);
    bytes.extend(&(info.len() as u32).to_be_bytes()[1..]);
    bytes.extend(info);
    let mut comment = Vec::new();
    comment.extend(6u32.to_le_bytes());
    comment.extend(b"vendor");
    comment.extend(1u32.to_le_bytes());
    let item = format!("TITLE={title}");
    comment.extend((item.len() as u32).to_le_bytes());
    comment.extend(item.as_bytes());
    bytes.push(0x84);
    bytes.extend(&(comment.len() as u32).to_be_bytes()[1..]);
    bytes.extend(comment);
    bytes.extend([0xFF, 0xF8]);
    bytes.extend((0..3000u32).map(|i| (i * 13 % 97) as u8));
    bytes
}

fn id3_prefix() -> Vec<u8> {
    let mut tag = Id3v2Tag::new();
    for (id, value) in [("TIT2", "ID3 title"), ("TPE1", "ID3 artist")] {
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new(id).unwrap(),
            TextEncoding::UTF8,
            value.to_owned(),
        )));
    }
    let picture = Picture::unchecked(vec![1, 2, 3])
        .pic_type(PictureType::CoverFront)
        .mime_type(MimeType::Png)
        .build();
    tag.insert(Frame::Picture(AttachedPictureFrame::new(
        TextEncoding::UTF8,
        picture,
    )));
    let mut bytes = Vec::new();
    tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
    bytes
}

#[test]
fn prepended_id3v2_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prefixed.flac");
    let mut bytes = id3_prefix();
    bytes.extend(flac("Vorbis title"));
    fs::write(&path, &bytes).unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["TITLE"], ["Vorbis title"]);
    assert!(!entry.fields.contains_key("ARTIST"), "{:?}", entry.fields);
    assert!(entry.covers.is_empty());
    assert_eq!(entry.technical["_TAG"], ["VorbisComment"]);
    assert_eq!(entry.sample_rate, 44_100);
}

#[test]
fn saving_and_removing_strip_the_prepended_id3v2() {
    let dir = tempfile::tempdir().unwrap();
    let stream = flac("Vorbis title");
    let image = dir.path().join("cover.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([9, 9, 9]))
        .save(&image)
        .unwrap();
    for operation in ["write", "cover", "remove"] {
        let path = dir.path().join(format!("{operation}.flac"));
        let mut bytes = id3_prefix();
        bytes.extend(&stream);
        fs::write(&path, &bytes).unwrap();
        let entry = match operation {
            "write" => write_fields(&path, &fields(&[("ALBUM", &["Album"])])).unwrap(),
            "cover" => add_cover(&path, &image).unwrap(),
            _ => remove_tags(&path).unwrap(),
        };
        let after = fs::read(&path).unwrap();
        assert!(after.starts_with(b"fLaC"), "{operation}: ID3v2 kept");
        // The audio frames follow the metadata unchanged.
        let frames = stream.len() - 3002;
        assert!(after.ends_with(&stream[frames..]), "{operation}");
        match operation {
            "write" => {
                assert_eq!(entry.fields["TITLE"], ["Vorbis title"]);
                assert_eq!(entry.fields["ALBUM"], ["Album"]);
            }
            "cover" => {
                assert_eq!(entry.covers.len(), 1);
                assert_eq!(entry.fields["TITLE"], ["Vorbis title"]);
            }
            _ => assert!(entry.fields.is_empty()),
        }
        assert_eq!(read_entry(&path).unwrap().fields, entry.fields);
    }
}
