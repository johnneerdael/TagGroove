#[path = "support/save_corpus.rs"]
mod corpus;

use std::fs;
use tag_core::history::{Journal, Limits};

#[test]
fn common_saves_preserve_audio_opaque_metadata_and_exact_undo() {
    for format in corpus::FORMATS {
        for operation in ["edit-one", "edit-many"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("café 日本.{format}"));
            corpus::seed(&path);
            let before_bytes = fs::read(&path).unwrap();
            let before = corpus::snapshot(&path);
            assert_eq!(before["opaque_marker_count"], 1, "{format}");
            assert_eq!(before["covers"].as_array().unwrap().len(), 2);
            let mut journal =
                Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
            let capture = journal.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
            corpus::edit(&path, operation);
            journal.commit(capture, &[], &[]);
            let after_bytes = fs::read(&path).unwrap();
            let after = corpus::snapshot(&path);
            let mut expected = before.clone();
            expected["fields"]["TITLE"] = serde_json::json!(["Changed café 日本"]);
            if operation == "edit-many" {
                expected["fields"].as_object_mut().unwrap().remove("ALBUM");
                expected["fields"]["ARTIST"] = serde_json::json!(["Third", "Fourth"]);
            }
            assert_eq!(
                after, expected,
                "{format} {operation}: unrelated data changed"
            );
            assert!(journal.undo().unwrap().errors.is_empty());
            assert_eq!(fs::read(&path).unwrap(), before_bytes, "{format}: undo");
            assert_eq!(corpus::snapshot(&path), before);
            assert!(journal.redo().unwrap().errors.is_empty());
            assert_eq!(fs::read(&path).unwrap(), after_bytes, "{format}: redo");
            let capture =
                journal.begin("Invalid save", None, &[path.to_string_lossy().into_owned()]);
            assert!(
                tag_core::write_fields(&path, &corpus::fields(&[("TITLE", &["bad\0value"])]))
                    .is_err()
            );
            journal.commit(capture, &[], &[]);
            assert_eq!(
                fs::read(&path).unwrap(),
                after_bytes,
                "{format}: rejected save"
            );
            assert_eq!(journal.state().undo.unwrap().label, "Save tags");
        }
    }
}

#[test]
fn competing_tag_policy_exposes_gain_location_and_can_be_undone() {
    let dir = tempfile::tempdir().unwrap();
    for preserve in [false, true] {
        let result =
            corpus::competing_mpeg(&dir.path().join(format!("gain-{preserve}.mp3")), preserve);
        assert_eq!(
            result["displayed_before"]["Title"],
            serde_json::json!(["APE wins display"])
        );
        assert_eq!(result["audio_preserved"], true);
        assert_eq!(result["undo_exact"], true);
        assert_eq!(result["undo_errors"], serde_json::json!([]));
        if preserve {
            assert_eq!(
                result["native_ape_after"]["mp3gain_undo"],
                "Text(\"+001,+001,N\")"
            );
            assert_eq!(
                result["native_ape_after"]["mp3gain_minmax"],
                "Text(\"094,205\")"
            );
            assert_eq!(
                result["native_ape_after"]["opaque_companion"],
                "Binary([1, 0, 255, 17])"
            );
        } else {
            assert!(
                result["native_ape_after"].is_null(),
                "default save removes unwritten APE"
            );
            assert_eq!(
                result["displayed_after"]["MP3GAIN_UNDO"],
                serde_json::json!(["+001,+001,N"]),
                "displayed text surviving in ID3 does not prove native gain-consumer compatibility"
            );
        }
    }
}

#[test]
fn negative_controls_detect_actual_field_cover_opaque_and_audio_loss() {
    for format in corpus::FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("control.{format}"));
        corpus::seed(&path);
        let bytes = fs::read(&path).unwrap();
        let before = corpus::snapshot(&path);
        tag_core::write_fields(&path, &corpus::fields(&[("CUSTOM FIELD", &[])])).unwrap();
        assert_ne!(corpus::snapshot(&path)["fields"], before["fields"]);
        fs::write(&path, &bytes).unwrap();
        tag_core::remove_cover(&path, 0).unwrap();
        assert_ne!(corpus::snapshot(&path)["covers"], before["covers"]);
        let mut damaged = bytes.clone();
        let marker = b"TGQA\0\x01\x02\xffopaque";
        let at = damaged
            .windows(marker.len())
            .position(|v| v == marker)
            .unwrap();
        damaged[at + marker.len() - 1] ^= 1;
        fs::write(&path, damaged).unwrap();
        assert_ne!(
            corpus::snapshot(&path)["opaque_marker_count"],
            before["opaque_marker_count"]
        );
        assert_ne!(
            corpus::snapshot(&path)["opaque_native"],
            before["opaque_native"]
        );
        let audio = corpus::payload(&path);
        let mut damaged = bytes;
        let at = damaged
            .windows(audio.len())
            .position(|v| v == audio)
            .unwrap();
        damaged[at + audio.len() - 1] ^= 1;
        fs::write(&path, damaged).unwrap();
        assert_ne!(
            corpus::payload(&path),
            audio,
            "{format}: actual payload corruption must be detected"
        );
    }
}

/// A standards-shaped v2.4 frame/footer around the retained MPEG frames.
fn footer_mp3() -> (Vec<u8>, Vec<u8>, usize) {
    let carrier = include_bytes!("fixtures/save-corpus/tone.mp3");
    assert_eq!(&carrier[..10], b"ID3\x04\0\0\0\0\0\x0a");
    let audio = carrier[20..].to_vec();
    let text = b"\x03Footer case";
    let mut frame = b"TIT2".to_vec();
    frame.extend([0, 0, 0, u8::try_from(text.len()).unwrap(), 0, 0]);
    frame.extend_from_slice(text);
    let mut header = b"ID3\x04\0\x10\0\0\0".to_vec();
    header.push(u8::try_from(frame.len()).unwrap());
    let mut bytes = header.clone();
    bytes.extend(frame);
    let footer_at = bytes.len();
    bytes.extend_from_slice(b"3DI");
    bytes.extend_from_slice(&header[3..]);
    bytes.extend_from_slice(&audio);
    (bytes, audio, footer_at)
}

#[test]
fn mpeg_payload_excludes_a_v24_footer_before_and_after_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("footer.mp3");
    let (bytes, audio, _) = footer_mp3();
    fs::write(&path, bytes).unwrap();
    assert_eq!(
        tag_core::read_entry(&path).unwrap().fields["TITLE"],
        ["Footer case"]
    );
    assert_eq!(
        corpus::payload(&path),
        audio,
        "footer is metadata, not MPEG audio"
    );
    corpus::edit(&path, "edit-one");
    assert_eq!(
        corpus::payload(&path),
        audio,
        "an ordinary save preserves MPEG frames"
    );
}

#[test]
fn mpeg_payload_rejects_missing_truncated_or_mismatched_v24_footers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid-footer.mp3");
    let (bytes, _, footer_at) = footer_mp3();
    for variant in ["missing", "truncated", "signature", "header"] {
        let mut invalid = bytes.clone();
        match variant {
            "missing" => {
                invalid.drain(footer_at..footer_at + 10);
            }
            "truncated" => invalid.truncate(footer_at + 5),
            "signature" => invalid[footer_at] = b'X',
            "header" => invalid[footer_at + 5] ^= 0x10,
            _ => unreachable!(),
        }
        fs::write(&path, invalid).unwrap();
        assert!(
            std::panic::catch_unwind(|| corpus::payload(&path)).is_err(),
            "{variant}"
        );
    }
}

/// Move the retained movie box before mdat and update its one real chunk offset.
fn terminal_mdat() -> (Vec<u8>, Vec<u8>, usize) {
    let carrier = include_bytes!("fixtures/save-corpus/tone.m4a");
    let mdat = carrier.windows(4).position(|v| v == b"mdat").unwrap() - 4;
    let size = u32::from_be_bytes(carrier[mdat..mdat + 4].try_into().unwrap()) as usize;
    let moov = mdat + size;
    assert_eq!(&carrier[moov + 4..moov + 8], b"moov");
    let mut movie = carrier[moov..].to_vec();
    assert_eq!(
        u32::from_be_bytes(movie[..4].try_into().unwrap()) as usize,
        movie.len()
    );
    let stco = movie.windows(4).position(|v| v == b"stco").unwrap();
    assert_eq!(&movie[stco + 8..stco + 12], &1u32.to_be_bytes());
    assert_eq!(
        &movie[stco + 12..stco + 16],
        &u32::try_from(mdat + 8).unwrap().to_be_bytes()
    );
    let target = mdat + movie.len();
    movie[stco + 12..stco + 16].copy_from_slice(&u32::try_from(target + 8).unwrap().to_be_bytes());
    let mut bytes = carrier[..mdat].to_vec();
    bytes.extend(movie);
    bytes.extend_from_slice(&carrier[mdat..moov]);
    bytes[target..target + 4].fill(0);
    assert_eq!(bytes.len(), carrier.len());
    (bytes, carrier[mdat + 8..moov].to_vec(), target)
}

#[test]
fn m4a_payload_accepts_a_terminal_eof_mdat_before_and_after_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("terminal.m4a");
    fs::write(&path, include_bytes!("fixtures/save-corpus/tone.m4a")).unwrap();
    let before = tag_core::read_entry(&path).unwrap();
    let (bytes, audio, _) = terminal_mdat();
    fs::write(&path, bytes).unwrap();
    assert_eq!(tag_core::read_entry(&path).unwrap().fields, before.fields);
    assert_eq!(corpus::payload(&path), audio);
    corpus::edit(&path, "edit-one");
    assert_eq!(corpus::payload(&path), audio);
}

#[test]
fn m4a_payload_skips_an_eof_metadata_box_and_rejects_invalid_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boundaries.m4a");
    let (bytes, audio, mdat) = terminal_mdat();
    let mut metadata_eof = include_bytes!("fixtures/save-corpus/tone.m4a").to_vec();
    let moov = metadata_eof.windows(4).position(|v| v == b"moov").unwrap() - 4;
    metadata_eof[moov..moov + 4].fill(0);
    fs::write(&path, metadata_eof).unwrap();
    assert_eq!(corpus::payload(&path), audio, "metadata is not audio");
    for variant in ["truncated", "oversize", "extended", "empty"] {
        let mut invalid = bytes.clone();
        match variant {
            "truncated" => invalid.truncate(mdat + 7),
            "oversize" => invalid[mdat..mdat + 4].copy_from_slice(&u32::MAX.to_be_bytes()),
            "extended" => invalid[mdat..mdat + 4].copy_from_slice(&1u32.to_be_bytes()),
            "empty" => invalid.truncate(mdat + 8),
            _ => unreachable!(),
        }
        fs::write(&path, invalid).unwrap();
        assert!(
            std::panic::catch_unwind(|| corpus::payload(&path)).is_err(),
            "{variant}"
        );
    }
}
