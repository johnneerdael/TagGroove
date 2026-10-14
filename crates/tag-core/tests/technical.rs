use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, UNIX_EPOCH},
};
use tag_core::{add_cover_summary, read_entry, read_entry_summary, write_fields_summary, Fields};
fn wav(path: &Path) {
    let mut b = b"RIFF".to_vec();
    b.extend(16036u32.to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(8000u32.to_le_bytes());
    b.extend(16000u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(16000u32.to_le_bytes());
    b.extend(vec![0u8; 16000]);
    fs::write(path, b).unwrap();
}
#[test]
fn wav_properties_dates_and_paths_are_measured_without_polluting_raw_fields() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.wav");
    wav(&p);
    fs::OpenOptions::new()
        .write(true)
        .open(&p)
        .unwrap()
        .set_times(
            fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
        )
        .unwrap();
    let a = read_entry_summary(&p).unwrap();
    let b = read_entry(&p).unwrap();
    assert_eq!(a.technical, b.technical);
    for (key, value) in [
        ("_CODEC", "PCM"),
        ("_BITSPERSAMPLE", "16"),
        ("_SAMPLERATE", "8000"),
        ("_BITRATE", "128"),
        ("_MODE", "Mono"),
        ("_LENGTH_SECONDS", "1"),
        ("_LENGTH", "0:01"),
        ("_COVERS", "0"),
        ("_FILE_MOD_DATETIME_RAW", "1700000000"),
        ("_FILENAME", "source"),
        ("_FILENAME_EXT", "source.wav"),
        ("_EXTENSION", "wav"),
    ] {
        assert_eq!(a.technical[key], [value], "{key}");
    }
    assert_eq!(
        a.technical["_FILE_SIZE_BYTES"],
        [fs::metadata(&p).unwrap().len().to_string()]
    );
    assert!(a.fields.is_empty());
    assert!(!a.technical.contains_key("_VBR"));
    assert!(!a.technical.contains_key("_MD5AUDIO"));
    assert!(!a.technical.contains_key("_SUBSONG"));
    let json = serde_json::to_value(&a).unwrap();
    let mut old = json.as_object().unwrap().clone();
    old.remove("technical");
    let old: tag_core::AudioEntry = serde_json::from_value(old.into()).unwrap();
    assert!(old.technical.is_empty());
}
#[test]
fn cover_headers_and_mutation_summary_technical_match_full_reads() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("source.wav");
    wav(&p);
    let image = d.path().join("image.png");
    image::RgbImage::new(17, 11).save(&image).unwrap();
    let a = add_cover_summary(&p, &image).unwrap();
    let b = read_entry(&p).unwrap();
    assert_eq!(a.technical, b.technical);
    assert!(a.covers[0].data_base64.is_empty());
    assert_eq!(a.covers[0].size, fs::metadata(&image).unwrap().len());
    assert_eq!(a.covers[0].width, Some(17));
    assert_eq!(a.covers[0].height, Some(11));
    assert_eq!(a.technical["_COVERS"], ["1"]);
    assert_eq!(a.technical["_COVER_WIDTH"], ["17"]);
    assert_eq!(a.technical["_COVER_HEIGHT"], ["11"]);
    assert_eq!(a.technical["_COVER_MIMETYPE"], ["image/png"]);
    assert_eq!(a.technical["_COVER_TYPE"], ["Front Cover"]);
    assert_eq!(
        a.technical["_PATH"],
        [p.canonicalize().unwrap().to_string_lossy()]
    );
    assert_eq!(a.technical["_FILENAME_EXT"], ["source.wav"]);
    let fields: Fields = [("TITLE".into(), vec!["Edited".into()])]
        .into_iter()
        .collect();
    let c = write_fields_summary(&p, &fields).unwrap();
    assert_eq!(c.technical, read_entry(&p).unwrap().technical);
    assert!(c.covers[0].data_base64.is_empty());
}
#[test]
fn encoded_properties_identify_actual_codec_and_keep_raw_fields() {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("ffmpeg absent: skipping encoded technical fixtures");
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("source.wav");
    wav(&source);
    for (ext, codec, expected) in [
        ("flac", "flac", "FLAC"),
        ("m4a", "aac", "AAC"),
        ("alac.m4a", "alac", "ALAC"),
        ("mp3", "libmp3lame", "MPEG-2.5 Layer III"),
        ("ogg", "vorbis", "Vorbis"),
        ("opus", "libopus", "Opus"),
        ("aiff", "pcm_s16be", "PCM"),
    ] {
        let p = d.path().join(format!("test.{ext}"));
        let r = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&source)
            .args(["-c:a", codec])
            .args(["-strict", "-2", "-ac", "2"])
            .arg(&p)
            .output()
            .unwrap();
        assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
        let a = read_entry_summary(&p).unwrap();
        let b = read_entry(&p).unwrap();
        assert_eq!(a.technical, b.technical, "{codec}");
        assert_eq!(a.technical["_CODEC"], [expected], "{codec}");
        assert!(!a.fields.keys().any(|k| k.starts_with('_')));
        if ext == "flac" {
            use lofty::file::AudioFile;
            let native = lofty::flac::FlacFile::read_from(
                &mut fs::File::open(&p).unwrap(),
                lofty::config::ParseOptions::new(),
            )
            .unwrap();
            assert_eq!(
                a.technical["_TOOL"],
                [native.vorbis_comments().unwrap().vendor()]
            );
        }
    }
}

#[test]
fn native_tag_version_encoding_and_opaque_identifiers_are_measured() {
    use lofty::{
        config::WriteOptions,
        id3::v2::{BinaryFrame, Frame, FrameId, Id3v2Tag, TextInformationFrame},
        tag::TagExt,
        TextEncoding,
    };
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("v23.wav");
    wav(&p);
    let mut native = Id3v2Tag::new();
    native.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF16,
        "Title",
    )));
    native.insert(Frame::Binary(BinaryFrame::new(
        FrameId::new("XABC").unwrap(),
        vec![1, 2, 3],
    )));
    native
        .save_to_path(&p, WriteOptions::default().use_id3v23(true))
        .unwrap();
    let a = read_entry_summary(&p).unwrap();
    assert_eq!(a.technical, read_entry(&p).unwrap().technical);
    assert_eq!(a.technical["_TAG"], ["ID3v2.3"]);
    assert_eq!(a.technical["_TAG_READ"], ["ID3v2.3"]);
    assert_eq!(a.technical["_ID3V2_CHARACTER_ENCODING"], ["UTF-16"]);
    assert_eq!(a.technical["_ID3V2_UNKNOWN_FRAMES"], ["XABC"]);
    assert_eq!(a.fields["TITLE"], ["Title"]);
    assert!(!a.fields.contains_key("_TAG"));
}

#[test]
fn jpeg_headers_supply_dimensions_but_unsupported_image_headers_do_not_invent_zeroes() {
    let d = tempfile::tempdir().unwrap();
    for (extension, dimensions) in [("jpg", Some((31, 19))), ("gif", None)] {
        let p = d.path().join(format!("{extension}.wav"));
        wav(&p);
        let image = d.path().join(format!("art.{extension}"));
        image::RgbImage::new(31, 19).save(&image).unwrap();
        let a = add_cover_summary(&p, &image).unwrap();
        assert_eq!(a.technical, read_entry(&p).unwrap().technical);
        assert_eq!(a.covers[0].width, dimensions.map(|d| d.0));
        assert_eq!(a.covers[0].height, dimensions.map(|d| d.1));
        assert!(a.covers[0].data_base64.is_empty());
        assert_eq!(
            a.technical.contains_key("_COVER_WIDTH"),
            dimensions.is_some()
        );
        assert_eq!(a.covers[0].size, fs::metadata(image).unwrap().len());
    }
}

#[test]
fn raw_modified_timestamp_is_directly_usable_by_fmtdate() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("date.wav");
    wav(&p);
    let time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    fs::OpenOptions::new()
        .write(true)
        .open(&p)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(time))
        .unwrap();
    let a = read_entry_summary(&p).unwrap();
    let context = tag_core::EvalContext::new(&a.technical);
    let expected: chrono::DateTime<chrono::Local> = time.into();
    assert_eq!(
        tag_core::evaluate_with_context(
            "$fmtDate(yyyy-MM-dd HH:mm:ss,%_file_mod_datetime_raw%)",
            &context
        )
        .unwrap(),
        expected.format("%Y-%m-%d %H:%M:%S").to_string()
    );
}

#[test]
fn malformed_embedded_jpeg_header_is_an_absent_dimension_not_a_panic() {
    use lofty::{
        config::WriteOptions,
        picture::{MimeType, Picture},
        tag::{Tag, TagExt, TagType},
    };
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("malformed-art.wav");
    wav(&p);
    let mut tag = Tag::new(TagType::Id3v2);
    // Valid JPEG prefix, then a segment length smaller than its own size word.
    tag.push_picture(
        Picture::unchecked(vec![255, 216, 255, 224, 0, 2, 255, 225, 0, 1])
            .mime_type(MimeType::Jpeg)
            .build(),
    );
    tag.save_to_path(&p, WriteOptions::default()).unwrap();
    let entry = read_entry_summary(&p).unwrap();
    assert_eq!(entry.covers[0].size, 10);
    assert_eq!(entry.covers[0].width, None);
    assert!(!entry.technical.contains_key("_COVER_WIDTH"));
}
