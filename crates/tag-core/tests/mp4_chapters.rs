//! MP4 chapters written by ffmpeg (a Nero `chpl` box and a QuickTime chapter
//! track): read as CHAPTERS, written back as both kinds (checked with
//! ffprobe), listed as rows, and removed by Utils → Strip chapters MP4
//! without moving or changing any media byte. Skipped without ffmpeg.
use std::{fs, path::Path, process::Command};
use tag_core::{read_entry, strip_mp4_chapters_with_options, write_fields, Fields, TagOptions};

fn tools() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok()
}

fn encode(dir: &Path, name: &str, codec: &str) -> std::path::PathBuf {
    let metadata = dir.join("chapters.txt");
    fs::write(
        &metadata,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=400\ntitle=One\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=400\nEND=1000\ntitle=Two – ü\n",
    )
    .unwrap();
    let path = dir.join(name);
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=1", "-i"])
        .arg(&metadata)
        .args(["-map", "0", "-map_chapters", "1", "-c:a", codec])
        .args(["-metadata", "title=Song"])
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

fn probe(path: &Path, entries: &str, extra: &[&str]) -> String {
    let output = Command::new("ffprobe")
        .args(["-v", "error"])
        .args(extra)
        .args(["-show_entries", entries, "-of", "csv=p=0"])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Windows builds of ffprobe end lines with CR LF.
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}
fn packets(path: &Path) -> String {
    probe(
        path,
        "packet=pts,size",
        &["-select_streams", "a", "-show_data_hash", "MD5"],
    ) + &probe(
        path,
        "packet=data_hash",
        &["-select_streams", "a", "-show_data_hash", "MD5"],
    )
}

#[test]
fn nero_and_quicktime_chapters_are_read_and_stripped() {
    if !tools() {
        eprintln!("Skipping MP4 chapter test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, codec) in [("a.mp4", "aac"), ("b.m4b", "aac"), ("c.m4a", "alac")] {
        let path = encode(dir.path(), name, codec);
        let entry = read_entry(&path).unwrap();
        assert_eq!(
            entry.fields["CHAPTERS"],
            ["00:00:00.000 One", "00:00:00.400 Two – ü"],
            "{name}"
        );
        assert_eq!(entry.technical["_SUBSONG_COUNT"], ["2"], "{name}");
        assert_eq!(entry.chapters.len(), 2);
        assert_eq!(
            entry.technical["_CODEC"],
            [if codec == "alac" { "ALAC" } else { "AAC" }]
        );

        // Without the Nero box the QuickTime chapter track is read.
        let mut bytes = fs::read(&path).unwrap();
        let chpl = bytes.windows(4).position(|w| w == b"chpl").unwrap();
        bytes[chpl..chpl + 4].copy_from_slice(b"free");
        let track_only = dir.path().join(format!("track-{name}"));
        fs::write(&track_only, &bytes).unwrap();
        let entry = read_entry(&track_only).unwrap();
        assert_eq!(
            entry.fields["CHAPTERS"],
            ["00:00:00.000 One", "00:00:00.400 Two – ü"],
            "{name}"
        );

        let before = fs::read(&path).unwrap();
        let audio = packets(&path);
        let stripped =
            strip_mp4_chapters_with_options(&path, false, &TagOptions::default()).unwrap();
        assert!(!stripped.fields.contains_key("CHAPTERS"), "{name}");
        assert!(!stripped.technical.contains_key("_SUBSONG_COUNT"));
        assert_eq!(stripped.fields["TITLE"], ["Song"]);
        let after = fs::read(&path).unwrap();
        assert_eq!(after.len(), before.len(), "{name}: nothing moves");
        assert_eq!(probe(&path, "chapter=start_time", &[]), "", "{name}");
        assert_eq!(packets(&path), audio, "{name}");
        // Tags can still be written afterwards.
        let written = write_fields(&path, &Fields::from([("ALBUM".into(), vec!["X".into()])]));
        assert_eq!(written.unwrap().fields["ALBUM"], ["X"]);
        assert_eq!(packets(&path), audio, "{name}");

        // A second strip, and files without chapters, change nothing.
        let unchanged = fs::read(&path).unwrap();
        strip_mp4_chapters_with_options(&path, false, &TagOptions::default()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), unchanged);
    }
    let flac = dir.path().join("plain.flac");
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=0.2"])
        .arg(&flac)
        .output()
        .unwrap();
    assert!(output.status.success());
    let before = fs::read(&flac).unwrap();
    strip_mp4_chapters_with_options(&flac, false, &TagOptions::default()).unwrap();
    assert_eq!(fs::read(&flac).unwrap(), before);
}

#[test]
fn m4b_and_m4v_files_are_scanned_with_their_codec() {
    if !tools() {
        eprintln!("Skipping MP4 scan test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, extra) in [
        ("book.m4b", vec!["-c:a", "aac"]),
        (
            "clip.m4v",
            vec![
                "-f",
                "lavfi",
                "-i",
                "color=blue:s=16x16:d=0.3",
                "-c:a",
                "aac",
                "-c:v",
                "mpeg4",
                "-f",
                "mp4",
            ],
        ),
    ] {
        let output = Command::new("ffmpeg")
            .args(["-v", "error", "-f", "lavfi", "-i", "sine=duration=0.3"])
            .args(&extra)
            .arg(dir.path().join(name))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let scanned = tag_core::scan_paths(&[dir.path().to_string_lossy().into_owned()], false);
    assert!(scanned.errors.is_empty(), "{:?}", scanned.errors);
    let found: Vec<_> = scanned
        .entries
        .iter()
        .map(|e| {
            (
                e.filename.as_str(),
                e.format.as_str(),
                e.technical["_CODEC"][0].as_str(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [("book.m4b", "MP4", "AAC"), ("clip.m4v", "MP4", "AAC")]
    );
}

fn fields(pairs: &[(&str, &[&str])]) -> Fields {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

/// ffprobe's chapter list: start time and title per chapter.
fn chapters(path: &Path) -> String {
    probe(path, "chapter=start_time:chapter_tags=title", &[])
}

/// Read the QuickTime chapter track alone, with the Nero box hidden.
fn track_chapters(path: &Path, dir: &Path) -> Vec<String> {
    let mut bytes = fs::read(path).unwrap();
    let chpl = bytes.windows(4).position(|w| w == b"chpl").unwrap();
    bytes[chpl..chpl + 4].copy_from_slice(b"free");
    let copy = dir.join("track-only.m4a");
    fs::write(&copy, &bytes).unwrap();
    read_entry(&copy).unwrap().fields["CHAPTERS"].clone()
}

#[test]
fn chapters_are_written_edited_and_removed_as_nero_and_quicktime() {
    if !tools() {
        eprintln!("Skipping MP4 chapter writing test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, codec) in [("a.mp4", "aac"), ("b.m4b", "aac"), ("c.m4a", "alac")] {
        let path = encode(dir.path(), name, codec);
        let audio = packets(&path);
        let before = fs::read(&path).unwrap();
        // ffmpeg writes moov last: only the tail of the file changes.
        let moov = before.windows(4).rposition(|w| w == b"moov").unwrap() - 4;

        let written = write_fields(
            &path,
            &fields(&[
                (
                    "CHAPTERS",
                    &["0:00 Intro", "0:00.250 Middle part", "00:00:00.700 End – é"],
                ),
                ("ALBUM", &["Book"]),
            ]),
        )
        .unwrap();
        assert_eq!(
            written.fields["CHAPTERS"],
            [
                "00:00:00.000 Intro",
                "00:00:00.250 Middle part",
                "00:00:00.700 End – é"
            ],
            "{name}"
        );
        assert_eq!(written.fields["ALBUM"], ["Book"]);
        assert_eq!(written.technical["_SUBSONG_COUNT"], ["3"]);
        assert_eq!(packets(&path), audio, "{name}: audio unchanged");
        let after = fs::read(&path).unwrap();
        assert_eq!(
            after[..moov],
            before[..moov],
            "{name}: media stays in place"
        );
        assert_eq!(
            chapters(&path),
            "0.000000,Intro\n0.250000,Middle part\n0.700000,End – é\n",
            "{name}"
        );
        assert_eq!(
            track_chapters(&path, dir.path()),
            [
                "00:00:00.000 Intro",
                "00:00:00.250 Middle part",
                "00:00:00.700 End – é"
            ],
            "{name}: QuickTime chapter track"
        );

        // A title edit and a repeated write keep the file in place.
        let length = fs::metadata(&path).unwrap().len();
        let edited = write_fields(
            &path,
            &fields(&[(
                "CHAPTERS",
                &[
                    "00:00:00.000 Intro",
                    "00:00:00.250 Middle",
                    "00:00:00.700 End – é",
                ],
            )]),
        )
        .unwrap();
        assert_eq!(edited.fields["CHAPTERS"][1], "00:00:00.250 Middle");
        assert!(fs::metadata(&path).unwrap().len() <= length);
        assert_eq!(packets(&path), audio, "{name}");
        assert_eq!(
            fs::read(&path).unwrap()[..moov],
            before[..moov],
            "{name}: media stays in place"
        );

        // Equal starts and over-long titles are refused without changes.
        let unchanged = fs::read(&path).unwrap();
        assert!(write_fields(&path, &fields(&[("CHAPTERS", &["0:00 A", "0:00 B"])])).is_err());
        let long = format!("0:00 {}", "x".repeat(256));
        assert!(write_fields(&path, &fields(&[("CHAPTERS", &[long.as_str()])])).is_err());
        assert_eq!(fs::read(&path).unwrap(), unchanged);

        // An empty CHAPTERS removes both kinds in place.
        let length = fs::metadata(&path).unwrap().len();
        let cleared = write_fields(&path, &fields(&[("CHAPTERS", &[])])).unwrap();
        assert!(!cleared.fields.contains_key("CHAPTERS"), "{name}");
        assert_eq!(cleared.fields["ALBUM"], ["Book"]);
        assert_eq!(fs::metadata(&path).unwrap().len(), length);
        assert_eq!(chapters(&path), "", "{name}");
        assert_eq!(packets(&path), audio, "{name}");

        // Chapters can be added again after removal.
        let late = write_fields(&path, &fields(&[("CHAPTERS", &["0:00.5 Late start"])])).unwrap();
        assert_eq!(late.fields["CHAPTERS"], ["00:00:00.500 Late start"]);
        // Like ffmpeg's own files, the QuickTime track (which ffprobe prefers)
        // starts at zero; the Nero box keeps the start.
        assert_eq!(chapters(&path), "0.000000,Late start\n", "{name}");
        assert_eq!(packets(&path), audio, "{name}");
    }
}

#[test]
fn fast_start_files_are_rewritten_once_then_edited_in_place() {
    if !tools() {
        eprintln!("Skipping MP4 fast start test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fast.m4a");
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=duration=1",
            "-c:a",
            "aac",
        ])
        .args(["-movflags", "+faststart", "-metadata", "title=Song"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let audio = packets(&path);
    let moov_first = |bytes: &[u8]| {
        bytes.windows(4).position(|w| w == b"moov") < bytes.windows(4).position(|w| w == b"mdat")
    };
    assert!(moov_first(&fs::read(&path).unwrap()));
    // No chapters and no free space: the media moves once, behind room for later edits.
    let first = write_fields(&path, &fields(&[("CHAPTERS", &["0:00 One", "0:00.5 Two"])])).unwrap();
    assert_eq!(first.fields["CHAPTERS"].len(), 2);
    assert_eq!(first.fields["TITLE"], ["Song"]);
    assert_eq!(packets(&path), audio);
    assert!(moov_first(&fs::read(&path).unwrap()), "moov stays first");
    assert_eq!(chapters(&path), "0.000000,One\n0.500000,Two\n");
    let length = fs::metadata(&path).unwrap().len();
    let media = fs::read(&path).unwrap();
    let mdat = media.windows(4).rposition(|w| w == b"mdat").unwrap();
    // Later edits fit the space after moov: nothing after it moves.
    write_fields(
        &path,
        &fields(&[("CHAPTERS", &["0:00 One", "0:00.3 Two", "0:00.6 Three"])]),
    )
    .unwrap();
    let edited = fs::read(&path).unwrap();
    assert_eq!(edited.len() as u64, length);
    assert_eq!(edited[mdat..], media[mdat..]);
    assert_eq!(
        chapters(&path),
        "0.000000,One\n0.300000,Two\n0.600000,Three\n"
    );
    assert_eq!(packets(&path), audio);
}

#[test]
fn chapters_are_listed_and_edited_as_rows() {
    if !tools() {
        eprintln!("Skipping MP4 chapter row test: ffmpeg and ffprobe are required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = encode(dir.path(), "book.m4b", "aac");
    write_fields(&path, &fields(&[("ARTIST", &["Author"])])).unwrap();
    let mut options = TagOptions::default();
    let folder = dir.path().to_string_lossy().into_owned();
    let plain =
        tag_core::scan_paths_with_options(std::slice::from_ref(&folder), false, &options, None);
    assert_eq!(plain.entries.len(), 1);
    options.advanced.list_chapters = true;
    let listed = tag_core::scan_paths_with_options(&[folder], false, &options, None);
    assert!(listed.errors.is_empty(), "{:?}", listed.errors);
    let rows: Vec<_> = listed.entries.iter().map(|e| e.path.clone()).collect();
    let file = dunce::canonicalize(&path)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        rows,
        [
            tag_core::subsong_path(&file, 1),
            tag_core::subsong_path(&file, 2)
        ]
    );
    let second = &listed.entries[1];
    assert_eq!(second.filename, "book.m4b");
    assert_eq!(second.fields["TITLE"], ["Two – ü"]);
    assert_eq!(second.fields["ARTIST"], ["Author"]);
    assert!(!second.fields.contains_key("CHAPTERS"));
    assert_eq!(second.technical["_SUBSONG"], ["1"]);
    assert_eq!(second.technical["_SUBSONG_COUNT"], ["2"]);
    assert_eq!(second.technical["_LENGTH_SECONDS"], ["1"]);
    assert!((second.duration - 0.6).abs() < 0.05, "{}", second.duration);
    // Sub-tracks without numbers are numbered, as in Mp3tag.
    assert_eq!(second.fields["TRACK"], ["2"]);
    assert_eq!(second.fields["TOTALTRACKS"], ["2"]);

    // A row edit renames its chapter. A field only this row has is stored
    // with the chapter in Nero tags (udta/tags), not in the file's tag.
    let audio = packets(&path);
    let row = tag_core::write_fields_with_options(
        Path::new(&rows[1]),
        &fields(&[("TITLE", &["Renamed"]), ("ALBUM", &["Part two"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    assert_eq!(row.path, rows[1]);
    assert_eq!(row.fields["TITLE"], ["Renamed"]);
    assert_eq!(row.fields["ALBUM"], ["Part two"]);
    let file_entry = read_entry(&path).unwrap();
    assert_eq!(
        file_entry.fields["CHAPTERS"],
        ["00:00:00.000 One", "00:00:00.400 Renamed"]
    );
    assert!(!file_entry.fields.contains_key("ALBUM"));
    assert_eq!(file_entry.chapters[1].fields["ALBUM"], ["Part two"]);
    assert!(file_entry.chapters[0].fields.is_empty());
    assert_eq!(file_entry.fields["TITLE"], ["Song"]);
    assert_eq!(chapters(&path), "0.000000,One\n0.400000,Renamed\n");
    assert_eq!(packets(&path), audio);
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.windows(4).any(|w| w == b"tseg"));
    assert!(bytes.windows(8).any(|w| w == b"\x80\0\x05album"));

    // Rows saved together with one value: a file field. With different
    // values: one per chapter, and the file loses the field.
    let together = tag_core::write_chapter_rows(
        &path,
        &[
            (
                1,
                fields(&[("TITLE", &["First"]), ("GENRE", &["Audiobook"])]),
            ),
            (2, fields(&[("GENRE", &["Audiobook"])])),
        ],
        false,
        &options,
    )
    .unwrap();
    assert_eq!(together[0].fields["TITLE"], ["First"]);
    assert_eq!(together[1].fields["GENRE"], ["Audiobook"]);
    assert_eq!(read_entry(&path).unwrap().fields["GENRE"], ["Audiobook"]);
    let split = tag_core::write_chapter_rows(
        &path,
        &[
            (1, fields(&[("GENRE", &["A"]), ("ALBUM", &["Part two"])])),
            (2, fields(&[("GENRE", &["B"])])),
        ],
        false,
        &options,
    )
    .unwrap();
    assert_eq!(split[0].fields["GENRE"], ["A"]);
    assert_eq!(split[1].fields["GENRE"], ["B"]);
    let file_entry = read_entry(&path).unwrap();
    assert!(!file_entry.fields.contains_key("GENRE"));
    assert_eq!(file_entry.chapters[0].fields["GENRE"], ["A"]);
    // ALBUM is now the same on both rows: back in the file, out of Nero.
    assert_eq!(file_entry.fields["ALBUM"], ["Part two"]);
    assert!(!file_entry.chapters[1].fields.contains_key("ALBUM"));
    assert_eq!(file_entry.chapters[0].title, "First");
    assert_eq!(packets(&path), audio);
    let unchanged = fs::read(&path).unwrap();
    assert!(tag_core::write_chapter_rows(
        &path,
        &[(3, fields(&[("TITLE", &["x"])]))],
        false,
        &options
    )
    .is_err());
    assert!(tag_core::write_chapter_rows(
        &path,
        &[(1, fields(&[("CHAPTERS", &["0:00 x"])]))],
        false,
        &options
    )
    .is_err());
    assert_eq!(fs::read(&path).unwrap(), unchanged);
    // Remove tag on the file removes the Nero tags with the iTunes tag; the
    // chapters stay.
    let removed = tag_core::remove_tags(&path).unwrap();
    assert!(!removed.fields.contains_key("ALBUM"));
    assert_eq!(removed.chapters.len(), 2);
    assert!(removed.chapters.iter().all(|c| c.fields.is_empty()));
    assert!(!fs::read(&path).unwrap().windows(4).any(|w| w == b"tseg"));
    assert_eq!(chapters(&path), "0.000000,First\n0.400000,Renamed\n");
    assert_eq!(packets(&path), audio);
    // Whole-file operations take the file, not a row.
    assert!(tag_core::remove_tags(Path::new(&rows[0])).is_err());
    assert!(strip_mp4_chapters_with_options(Path::new(&rows[0]), false, &options).is_err());
}
