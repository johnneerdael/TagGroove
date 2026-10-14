//! Utils → Remove Unsupported Fields and Optimize (FLAC, MP4).
//! FLAC and MP4 checks use ffmpeg-encoded files and compare decoded packets
//! when ffmpeg is installed.
use lofty::{
    config::WriteOptions,
    id3::v2::{BinaryFrame, Frame, FrameId, Id3v2Tag, PrivateFrame, UniqueFileIdentifierFrame},
    prelude::{Accessor, TagExt},
};
use std::{fs, path::Path, process::Command};
use tag_core::{
    optimize_with_options, read_entry_with_options, remove_unsupported_fields_with_options,
    write_fields_with_options, Fields, TagOptions, FLAC_PADDING,
};

fn mp3(path: &Path) -> Vec<u8> {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xFF, 0xFB, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
    audio
}

/// Bytes after a leading ID3v2 tag.
fn after_id3(bytes: &[u8]) -> &[u8] {
    assert_eq!(&bytes[..3], b"ID3");
    let size = bytes[6..10]
        .iter()
        .fold(0usize, |v, b| (v << 7) | usize::from(*b));
    &bytes[10 + size..]
}

fn technical(path: &Path, key: &str) -> String {
    read_entry_with_options(path, &[], &TagOptions::default())
        .unwrap()
        .technical
        .get(key)
        .map(|v| v.join(" "))
        .unwrap_or_default()
}

#[test]
fn unsupported_id3_frames_are_removed_and_everything_else_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("song.mp3");
    let audio = mp3(&path);
    let mut tag = Id3v2Tag::new();
    tag.set_title("Kept".into());
    tag.insert(Frame::Private(PrivateFrame::new(
        "WM/MediaClassPrimaryID",
        vec![1, 2, 3],
    )));
    tag.insert(Frame::Binary(BinaryFrame::new(
        FrameId::new("GEOB").unwrap(),
        vec![0, b'x', 0, 0, 0, 9],
    )));
    tag.insert(Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame::new(
        "http://musicbrainz.org",
        b"mbid".to_vec(),
    )));
    tag.insert(Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame::new(
        "http://example.com",
        b"other".to_vec(),
    )));
    tag.save_to_path(&path, WriteOptions::default().use_id3v23(true))
        .unwrap();
    let options = TagOptions::default();
    let cover = dir.path().join("cover.png");
    image::DynamicImage::new_rgb8(2, 2)
        .save_with_format(&cover, image::ImageFormat::Png)
        .unwrap();
    tag_core::add_cover_with_options(&path, &cover, false, &options).unwrap();
    let unknown = technical(&path, "_ID3V2_UNKNOWN_FRAMES");
    assert_eq!(unknown, "GEOB, PRIV, UFID");
    let before = read_entry_with_options(&path, &[], &options).unwrap();
    let rest = after_id3(&fs::read(&path).unwrap()).to_vec();
    assert!(rest.starts_with(&audio));

    let entry = remove_unsupported_fields_with_options(&path, false, &options).unwrap();
    assert!(!entry.technical.contains_key("_ID3V2_UNKNOWN_FRAMES"));
    assert_eq!(entry.technical["_TAG"], before.technical["_TAG"]);
    assert_eq!(
        entry.fields, before.fields,
        "displayed fields are unchanged"
    );
    assert_eq!(entry.covers.len(), 1);
    assert_eq!(entry.fields["TITLE"], ["Kept"]);
    assert!(entry.fields.contains_key("MUSICBRAINZ_TRACKID"));
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[3..5], [3, 0], "ID3v2.3 is kept");
    assert_eq!(
        after_id3(&bytes),
        rest.as_slice(),
        "audio and ID3v1 are untouched"
    );

    // Nothing left to remove: the file is not rewritten.
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    remove_unsupported_fields_with_options(&path, false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
}

fn ffmpeg() -> bool {
    let ok = Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok();
    if !ok {
        eprintln!("Skipping ffmpeg-encoded Optimize checks: ffmpeg and ffprobe are required");
    }
    ok
}

fn encode(args: &[&str], path: &Path) {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-y"])
        .args(args)
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Hashes of every audio packet, independent of the container layout.
fn packets(path: &Path) -> String {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:a", "-c", "copy", "-f", "framemd5", "-"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn flac_blocks(path: &Path) -> Vec<(u8, usize)> {
    let bytes = fs::read(path).unwrap();
    let mut at = 4;
    let mut out = Vec::new();
    loop {
        let header = &bytes[at..at + 4];
        let len =
            usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]);
        out.push((header[0] & 0x7f, len));
        at += 4 + len;
        if header[0] & 0x80 != 0 {
            return out;
        }
    }
}

#[test]
fn optimize_flac_leaves_exactly_four_kilobytes_of_padding() {
    if !ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("song.flac");
    encode(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=8000:duration=1",
        ],
        &path,
    );
    let options = TagOptions::default();
    write_fields_with_options(
        &path,
        &Fields::from([("TITLE".into(), vec!["Padded".into()])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    // ffmpeg reserves 8 KiB of padding; Optimize shrinks it to 4 KiB.
    assert!(flac_blocks(&path)
        .iter()
        .any(|&(kind, len)| kind == 1 && len > FLAC_PADDING));
    let audio = packets(&path);
    let entry = optimize_with_options(&path, false, &options).unwrap();
    assert_eq!(entry.fields["TITLE"], ["Padded"]);
    let blocks = flac_blocks(&path);
    assert_eq!(blocks.last(), Some(&(1, FLAC_PADDING)));
    assert_eq!(blocks.iter().filter(|(kind, _)| *kind == 1).count(), 1);
    assert_eq!(packets(&path), audio);

    // A large cover leaves no padding; Optimize adds it back.
    let cover = dir.path().join("cover.png");
    image::DynamicImage::new_rgb8(64, 64)
        .save_with_format(&cover, image::ImageFormat::Png)
        .unwrap();
    let mut bytes = fs::read(&path).unwrap();
    // Remove the padding block by hand: mark the block before it as last.
    let padding_at = 4 + blocks[..blocks.len() - 1]
        .iter()
        .map(|(_, len)| 4 + len)
        .sum::<usize>();
    let previous = padding_at - 4 - blocks[blocks.len() - 2].1;
    bytes[previous] |= 0x80;
    bytes.drain(padding_at..padding_at + 4 + FLAC_PADDING);
    fs::write(&path, &bytes).unwrap();
    assert!(!flac_blocks(&path).iter().any(|(kind, _)| *kind == 1));
    tag_core::add_cover_with_options(&path, &cover, false, &options).unwrap();
    optimize_with_options(&path, false, &options).unwrap();
    assert_eq!(flac_blocks(&path).last(), Some(&(1, FLAC_PADDING)));
    assert_eq!(packets(&path), audio);
    assert_eq!(
        read_entry_with_options(&path, &[], &options)
            .unwrap()
            .covers
            .len(),
        1
    );

    // Already optimized: untouched.
    let before = fs::read(&path).unwrap();
    optimize_with_options(&path, false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
}

/// Top-level box names in order.
fn top_boxes(path: &Path) -> Vec<String> {
    let bytes = fs::read(path).unwrap();
    let mut at = 0;
    let mut out = Vec::new();
    while at < bytes.len() {
        let size = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        out.push(String::from_utf8_lossy(&bytes[at + 4..at + 8]).into_owned());
        at += if size == 0 { bytes.len() - at } else { size };
    }
    out
}

fn chapters(path: &Path) -> usize {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-show_chapters"])
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| *line == "[CHAPTER]")
        .count()
}

#[test]
fn optimize_rewrites_ffmpeg_mp4_files_losslessly() {
    if !ffmpeg() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let metadata = dir.path().join("chapters.txt");
    fs::write(
        &metadata,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=One\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=1000\nEND=2000\ntitle=Two\n",
    )
    .unwrap();
    let path = dir.path().join("book.m4a");
    encode(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-i",
            metadata.to_str().unwrap(),
            "-map_chapters",
            "1",
            "-c:a",
            "aac",
        ],
        &path,
    );
    let options = TagOptions::default();
    write_fields_with_options(
        &path,
        &Fields::from([("TITLE".into(), vec!["Book".into()])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(top_boxes(&path), ["ftyp", "free", "mdat", "moov"]);
    assert_eq!(chapters(&path), 2);
    let audio = packets(&path);

    let entry = optimize_with_options(&path, false, &options).unwrap();
    assert_eq!(entry.fields["TITLE"], ["Book"]);
    assert_eq!(top_boxes(&path), ["ftyp", "moov", "mdat"]);
    assert!(!fs::read(&path).unwrap().windows(4).any(|w| w == b"free"));
    assert_eq!(packets(&path), audio);
    assert_eq!(chapters(&path), 2, "Optimize keeps chapters");

    let before = fs::read(&path).unwrap();
    optimize_with_options(&path, false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before, "already optimized");
}

#[test]
fn utils_reject_formats_they_do_not_apply_to() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("song.mp3");
    mp3(&path);
    let options = TagOptions::default();
    assert!(optimize_with_options(&path, false, &options)
        .unwrap_err()
        .contains("FLAC and MP4"));
    // An MP3 without any ID3v2 tag has nothing to remove.
    let bytes = fs::read(&path).unwrap();
    remove_unsupported_fields_with_options(&path, false, &options).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
}
