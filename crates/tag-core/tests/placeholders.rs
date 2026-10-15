//! Placeholders measured from encoded files: bitrate type and encoder,
//! tag sizes, MP4 header times, video properties, chapter counts and the
//! audio MD5 source. Files are made with ffmpeg; the tests are skipped
//! without it.
use std::{fs, path::Path, process::Command};
use tag_core::{audio_digest, format_length, read_entry, write_fields, AudioDigest, Fields};

fn ffmpeg() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
}

fn encode(dir: &Path, name: &str, args: &[&str]) -> std::path::PathBuf {
    let path = dir.join(name);
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=1"])
        .args(args)
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
}

fn value<'a>(fields: &'a Fields, name: &str) -> Option<&'a str> {
    fields.get(name).and_then(|v| v.first()).map(String::as_str)
}

#[test]
fn length_has_hours_from_one_hour_on() {
    assert_eq!(format_length(0), "0:00");
    assert_eq!(format_length(59), "0:59");
    assert_eq!(format_length(3599), "59:59");
    assert_eq!(format_length(3600), "1:00:00");
    assert_eq!(format_length(4503), "1:15:03");
}

#[test]
fn mpeg_bitrate_type_encoder_and_tag_sizes() {
    if !ffmpeg() {
        eprintln!("Skipping: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let vbr = encode(dir.path(), "vbr.mp3", &["-c:a", "libmp3lame", "-q:a", "4"]);
    let cbr = encode(
        dir.path(),
        "cbr.mp3",
        &["-c:a", "libmp3lame", "-b:a", "128k"],
    );
    let vbr_entry = read_entry(&vbr).unwrap();
    assert_eq!(value(&vbr_entry.technical, "_VBR"), Some("VBR"));
    assert!(
        value(&vbr_entry.technical, "_TOOL").is_some_and(|tool| !tool.is_empty()),
        "{:?}",
        vbr_entry.technical
    );
    assert_eq!(
        value(&read_entry(&cbr).unwrap().technical, "_VBR"),
        Some("CBR")
    );

    let updates = Fields::from([("TITLE".into(), vec!["Measured".into()])]);
    let tagged = write_fields(&cbr, &updates).unwrap();
    let prepended: u64 = value(&tagged.technical, "_TAG_SIZE_PREPENDED")
        .unwrap()
        .parse()
        .unwrap();
    let appended: u64 = value(&tagged.technical, "_TAG_SIZE_APPENDED")
        .unwrap()
        .parse()
        .unwrap();
    let total: u64 = value(&tagged.technical, "_TAG_SIZE")
        .unwrap()
        .parse()
        .unwrap();
    let bytes = fs::read(&cbr).unwrap();
    assert!(bytes.starts_with(b"ID3"));
    assert!(prepended >= 10);
    assert_eq!(total, prepended + appended);
    // The audio range starts at the first frame after the tags.
    match audio_digest(&cbr).unwrap() {
        AudioDigest::Range(range) => {
            assert_eq!(range.start, prepended);
            assert_eq!(range.end, bytes.len() as u64 - appended);
            assert_eq!(bytes[range.start as usize], 0xFF);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn flac_audio_md5_is_the_stored_checksum() {
    if !ffmpeg() {
        eprintln!("Skipping: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let flac = encode(dir.path(), "a.flac", &["-c:a", "flac"]);
    let stored = &fs::read(&flac).unwrap()[26..42];
    match audio_digest(&flac).unwrap() {
        AudioDigest::Stored(md5) => {
            assert_eq!(md5.len(), 32);
            assert_eq!(
                md5,
                stored
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
        }
        other => panic!("{other:?}"),
    }
    let entry = read_entry(&flac).unwrap();
    assert!(value(&entry.technical, "_TAG_SIZE").is_some());
    let wav = encode(dir.path(), "a.wav", &["-c:a", "pcm_s16le"]);
    assert_eq!(audio_digest(&wav).unwrap(), AudioDigest::Unavailable);
}

#[test]
fn mp4_header_times_and_video_properties() {
    if !ffmpeg() {
        eprintln!("Skipping: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let clip = encode(
        dir.path(),
        "clip.m4v",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=blue:s=32x16:d=1",
            "-c:a",
            "aac",
            "-c:v",
            "mpeg4",
            "-metadata",
            "creation_time=2024-01-02T03:04:05Z",
            "-f",
            "mp4",
        ],
    );
    let technical = read_entry(&clip).unwrap().technical;
    assert_eq!(
        value(&technical, "_MP4_CREATION_TIME_RAW"),
        Some("1704164645")
    );
    assert!(value(&technical, "_MP4_CREATION_TIME").is_some_and(|t| !t.is_empty()));
    assert_eq!(value(&technical, "_VIDEO_CODEC"), Some("MPEG-4 Visual"));
    assert_eq!(value(&technical, "_VIDEO_WIDTH"), Some("32"));
    assert_eq!(value(&technical, "_VIDEO_HEIGHT"), Some("16"));
    assert!(
        value(&technical, "_VIDEO_BITRATE").is_some_and(|b| b.parse::<u64>().unwrap() > 0),
        "{technical:?}"
    );
    // Audio-only MP4 files have no video placeholders.
    let audio = encode(dir.path(), "a.m4a", &["-c:a", "aac"]);
    let technical = read_entry(&audio).unwrap().technical;
    assert!(!technical.contains_key("_VIDEO_CODEC"));
    assert_eq!(audio_digest(&audio).unwrap(), AudioDigest::Unavailable);
}

#[test]
fn matroska_video_properties() {
    if !ffmpeg() {
        eprintln!("Skipping: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let clip = encode(
        dir.path(),
        "clip.mkv",
        &[
            "-f",
            "lavfi",
            "-i",
            "color=red:s=48x32:d=1",
            "-c:a",
            "flac",
            "-c:v",
            "mpeg4",
        ],
    );
    let technical = read_entry(&clip).unwrap().technical;
    assert_eq!(value(&technical, "_VIDEO_CODEC"), Some("MPEG-4 Visual"));
    assert_eq!(value(&technical, "_VIDEO_WIDTH"), Some("48"));
    assert_eq!(value(&technical, "_VIDEO_HEIGHT"), Some("32"));
}

#[test]
fn id3_chapters_are_counted_as_subsongs() {
    if !ffmpeg() {
        eprintln!("Skipping: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let metadata = dir.path().join("chapters.txt");
    fs::write(
        &metadata,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=400\ntitle=One\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=400\nEND=1000\ntitle=Two\n",
    )
    .unwrap();
    let path = dir.path().join("chapters.mp3");
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=1", "-i"])
        .arg(&metadata)
        .args(["-map", "0", "-map_chapters", "1", "-c:a", "libmp3lame"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let technical = read_entry(&path).unwrap().technical;
    assert_eq!(value(&technical, "_SUBSONG_COUNT"), Some("2"));
}
