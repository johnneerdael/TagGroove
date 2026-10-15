//! Chapters listed as rows (Options → Tags → Advanced → List chapters as
//! separate files) for ID3v2 `CHAP` frames, cue sheets embedded in a
//! CUESHEET field and loaded cue sheets, with edits written back into the
//! chapter. Audio fixtures come from ffmpeg; the tests are skipped without it.
use std::{fs, path::Path, process::Command};
use tag_core::{
    playlist::load_playlist, read_entry, scan_paths_with_options, subsong_path, write_chapter_rows,
    write_fields, write_fields_with_options, Fields, TagOptions,
};

fn tools() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok()
}

fn fields(pairs: &[(&str, &[&str])]) -> Fields {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

fn ffmpeg(args: &[&str], output: &Path) {
    let result = Command::new("ffmpeg")
        .args(["-v", "error", "-y"])
        .args(args)
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn probe(path: &Path, entries: &str, extra: &[&str]) -> String {
    let output = Command::new("ffprobe")
        .args(["-v", "error"])
        .args(extra)
        .args(["-show_entries", entries, "-of", "csv=p=0"])
        .arg(path)
        .output()
        .unwrap();
    // Windows builds of ffprobe end lines with CR LF.
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

fn audio(path: &Path) -> String {
    probe(
        path,
        "packet=pts,size,data_hash",
        &["-select_streams", "a", "-show_data_hash", "MD5"],
    )
}

fn listing() -> TagOptions {
    let mut options = TagOptions::default();
    options.advanced.list_chapters = true;
    options
}

#[test]
fn id3v2_chapters_are_rows_and_titles_are_written_into_chap_frames() {
    if !tools() {
        eprintln!("Skipping ID3v2 chapter rows: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let meta = dir.path().join("chapters.txt");
    fs::write(
        &meta,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=400\ntitle=One\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=400\nEND=1000\ntitle=Two\n",
    )
    .unwrap();
    for version in ["3", "4"] {
        let path = dir.path().join(format!("podcast{version}.mp3"));
        let meta = meta.to_string_lossy();
        ffmpeg(
            &[
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1",
                "-i",
                &meta,
                "-map",
                "0",
                "-map_chapters",
                "1",
                "-c:a",
                "libmp3lame",
                "-id3v2_version",
                version,
                "-metadata",
                "title=Episode",
                "-metadata",
                "artist=Host",
            ],
            &path,
        );
        let sound = audio(&path);
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.chapters.len(), 2, "v2.{version}");
        assert_eq!(entry.technical["_SUBSONG_COUNT"], ["2"]);
        assert!(!entry.fields.contains_key("CHAPTERS"));

        let options = listing();
        let listed = scan_paths_with_options(
            &[path.to_string_lossy().into_owned()],
            false,
            &options,
            None,
        );
        let file = dunce::canonicalize(&path)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let rows: Vec<_> = listed.entries.iter().map(|e| e.path.clone()).collect();
        assert_eq!(rows, [subsong_path(&file, 1), subsong_path(&file, 2)]);
        assert_eq!(listed.entries[1].fields["TITLE"], ["Two"]);
        assert_eq!(listed.entries[1].fields["ARTIST"], ["Host"]);
        assert_eq!(listed.entries[1].technical["_SUBSONG"], ["1"]);

        let row = write_fields_with_options(
            Path::new(&rows[0]),
            &fields(&[("TITLE", &["Opening – é"]), ("ALBUM", &["Show"])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        assert_eq!(row.fields["TITLE"], ["Opening – é"]);
        assert_eq!(row.fields["ALBUM"], ["Show"]);
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.fields["TITLE"], ["Episode"], "the file title is kept");
        // Only the first row has ALBUM: it is a sub-frame of that chapter.
        assert!(!entry.fields.contains_key("ALBUM"));
        assert_eq!(entry.chapters[0].fields["ALBUM"], ["Show"]);
        assert_eq!(entry.chapters[0].title, "Opening – é");
        assert_eq!(entry.chapters[1].title, "Two");
        assert_eq!(
            probe(&path, "chapter=start_time:chapter_tags=title", &[]),
            "0.000000,Opening – é\n0.400000,Two\n",
            "v2.{version}"
        );
        // MPEG saves use the configured ID3v2 revision; the chapters follow.
        assert!(
            entry.technical["_TAG"][0].contains("ID3v2.3"),
            "{:?}",
            entry.technical["_TAG"]
        );
        assert_eq!(audio(&path), sound);
        // Clearing a chapter title keeps the chapter.
        write_chapter_rows(&path, &[(2, fields(&[("TITLE", &[])]))], false, &options).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.chapters.len(), 2);
        assert_eq!(entry.chapters[1].title, "");

        // Differing fields go into the CHAP sub-frames (TPE1, TIT3, WXXX),
        // equal ones into the tag; the chapters keep their times.
        let rows = write_chapter_rows(
            &path,
            &[
                (
                    1,
                    fields(&[
                        ("ARTIST", &["Guest"]),
                        ("SUBTITLE", &["Part A"]),
                        ("WWW", &["https://example.com/1"]),
                        ("ALBUM", &["Show"]),
                    ]),
                ),
                (
                    2,
                    fields(&[("SUBTITLE", &["Part B"]), ("ALBUM", &["Show"])]),
                ),
            ],
            false,
            &options,
        )
        .unwrap();
        assert_eq!(rows[0].fields["ARTIST"], ["Guest"]);
        assert_eq!(rows[1].fields["ARTIST"], ["Host"]);
        assert_eq!(rows[1].fields["SUBTITLE"], ["Part B"]);
        assert!(!rows[1].fields.contains_key("WWW"));
        let entry = read_entry(&path).unwrap();
        assert!(!entry.fields.contains_key("ARTIST"));
        assert!(!entry.fields.contains_key("SUBTITLE"));
        assert_eq!(entry.fields["ALBUM"], ["Show"]);
        assert_eq!(
            entry.chapters[0].fields,
            fields(&[
                ("ARTIST", &["Guest"]),
                ("SUBTITLE", &["Part A"]),
                ("WWW", &["https://example.com/1"]),
            ])
        );
        assert_eq!(
            entry.chapters[1].fields,
            fields(&[("ARTIST", &["Host"]), ("SUBTITLE", &["Part B"])])
        );
        assert_eq!(entry.chapters[0].title, "Opening – é");
        assert_eq!(
            probe(&path, "chapter=start_time,end_time", &[]),
            "0.000000,0.400000\n0.400000,1.000000\n"
        );
        // One value on every row again: back in the tag, out of the chapters.
        write_chapter_rows(
            &path,
            &[(2, fields(&[("ARTIST", &["Guest"])]))],
            false,
            &options,
        )
        .unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.fields["ARTIST"], ["Guest"]);
        assert!(!entry.chapters[0].fields.contains_key("ARTIST"));
        assert!(!entry.chapters[1].fields.contains_key("ARTIST"));
        assert_eq!(entry.chapters[1].fields["SUBTITLE"], ["Part B"]);
        assert_eq!(audio(&path), sound);
    }
}

const SHEET: &str = "REM GENRE Jazz\r\nPERFORMER \"The Trio\"\r\nTITLE \"Live\"\r\nFILE \"image.flac\" WAVE\r\n  TRACK 01 AUDIO\r\n    TITLE \"Opening\"\r\n    INDEX 01 00:00:00\r\n  TRACK 02 AUDIO\r\n    TITLE \"Ballad\"\r\n    PERFORMER \"Guest\"\r\n    INDEX 01 00:01:00\r\n";

#[test]
fn embedded_cue_sheets_list_their_tracks_and_rows_edit_the_sheet() {
    if !tools() {
        eprintln!("Skipping embedded cue sheet rows: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("image.flac");
    ffmpeg(
        &["-f", "lavfi", "-i", "sine=duration=2", "-c:a", "flac"],
        &path,
    );
    write_fields(
        &path,
        &fields(&[
            ("CUESHEET", &[SHEET]),
            ("ALBUM", &["Live"]),
            ("ARTIST", &["The Trio"]),
        ]),
    )
    .unwrap();
    let sound = audio(&path);
    let options = listing();
    let listed = scan_paths_with_options(
        &[path.to_string_lossy().into_owned()],
        false,
        &options,
        None,
    );
    assert_eq!(listed.entries.len(), 2);
    let (first, second) = (&listed.entries[0], &listed.entries[1]);
    assert_eq!(first.fields["TITLE"], ["Opening"]);
    assert_eq!(first.fields["ARTIST"], ["The Trio"]);
    assert_eq!(first.fields["TRACK"], ["1"]);
    assert!(!first.fields.contains_key("CUESHEET"));
    assert_eq!(second.fields["ARTIST"], ["Guest"]);
    assert_eq!(second.fields["ALBUM"], ["Live"]);
    assert_eq!(second.technical["_SUBSONG"], ["1"]);
    assert!((first.duration - 1.0).abs() < 0.01, "{}", first.duration);

    let rows = write_chapter_rows(
        &path,
        &[
            (1, fields(&[("TITLE", &["Intro"]), ("ARTIST", &["Solo"])])),
            (
                2,
                fields(&[("ISRC", &["USXXX2400001"]), ("GENRE", &["Bop"])]),
            ),
        ],
        false,
        &options,
    )
    .unwrap();
    assert_eq!(rows[0].fields["TITLE"], ["Intro"]);
    assert_eq!(rows[0].fields["ARTIST"], ["Solo"]);
    assert_eq!(rows[1].fields["ISRC"], ["USXXX2400001"]);
    assert_eq!(rows[1].fields["GENRE"], ["Bop"]);
    let entry = read_entry(&path).unwrap();
    let sheet = &entry.fields["CUESHEET"][0];
    assert!(sheet.contains("    TITLE \"Intro\"\r\n"), "{sheet}");
    assert!(sheet.contains("    PERFORMER \"Solo\"\r\n"), "{sheet}");
    assert!(sheet.contains("    ISRC USXXX2400001\r\n"), "{sheet}");
    assert!(
        sheet.contains("REM GENRE Jazz\r\n"),
        "album lines are kept: {sheet}"
    );
    assert_eq!(
        entry.fields["GENRE"],
        ["Bop"],
        "other fields are the file's"
    );
    assert_eq!(audio(&path), sound);
    // Track numbers come from the sheet.
    assert!(
        write_chapter_rows(&path, &[(1, fields(&[("TRACK", &["5"])]))], false, &options).is_err()
    );
}

#[test]
fn loaded_cue_sheets_list_tracks_and_rows_write_the_sheet() {
    if !tools() {
        eprintln!("Skipping cue sheet rows: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("image.flac");
    ffmpeg(
        &["-f", "lavfi", "-i", "sine=duration=2", "-c:a", "flac"],
        &image,
    );
    write_fields(&image, &fields(&[("TITLE", &["Whole image"])])).unwrap();
    let cue = dir.path().join("live.cue");
    fs::write(&cue, SHEET).unwrap();
    let loaded = load_playlist(&cue, None).unwrap();
    let rows: Vec<String> = loaded
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let base = cue.to_string_lossy().into_owned();
    assert_eq!(rows, [subsong_path(&base, 1), subsong_path(&base, 2)]);
    let options = TagOptions::default();
    let scanned = scan_paths_with_options(&rows, false, &options, None);
    assert!(scanned.errors.is_empty(), "{:?}", scanned.errors);
    let second = &scanned.entries[1];
    assert_eq!(second.filename, "image.flac");
    assert_eq!(second.fields["TITLE"], ["Ballad"]);
    assert_eq!(second.fields["ARTIST"], ["Guest"]);
    assert_eq!(second.fields["ALBUM"], ["Live"]);
    assert_eq!(second.fields["ALBUMARTIST"], ["The Trio"]);
    assert_eq!(second.fields["GENRE"], ["Jazz"]);
    assert_eq!(second.fields["TRACK"], ["2"]);
    assert_eq!(scanned.entries[0].fields["ARTIST"], ["The Trio"]);
    assert!((second.duration - 1.0).abs() < 0.01, "{}", second.duration);
    assert_eq!(second.technical["_SUBSONG_COUNT"], ["2"]);

    let image_bytes = fs::read(&image).unwrap();
    let written = write_fields(
        Path::new(&rows[1]),
        &fields(&[("TITLE", &["Slow one"]), ("ALBUM", &["Live at home"])]),
    )
    .unwrap();
    assert_eq!(written.fields["TITLE"], ["Slow one"]);
    assert_eq!(written.fields["ALBUM"], ["Live at home"]);
    assert_eq!(
        fs::read_to_string(&cue).unwrap(),
        SHEET
            .replace("TITLE \"Ballad\"", "TITLE \"Slow one\"")
            .replace("TITLE \"Live\"", "TITLE \"Live at home\"")
    );
    assert_eq!(
        fs::read(&image).unwrap(),
        image_bytes,
        "the audio is untouched"
    );
    // Fields a cue sheet cannot hold are refused; whole-file operations too.
    assert!(write_fields(Path::new(&rows[0]), &fields(&[("LYRICIST", &["x"])])).is_err());
    assert!(tag_core::remove_tags(Path::new(&rows[0])).is_err());
    assert_eq!(read_entry(&image).unwrap().fields["TITLE"], ["Whole image"]);
}

#[test]
fn matroska_rows_keep_differing_fields_in_chapter_tags() {
    if !tools() {
        eprintln!("Skipping Matroska chapter fields: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let meta = dir.path().join("chapters.txt");
    fs::write(
        &meta,
        ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=Opening\n\
         [CHAPTER]\nTIMEBASE=1/1000\nSTART=1000\nEND=2000\ntitle=Second part\n",
    )
    .unwrap();
    let path = dir.path().join("film.mka");
    let meta = meta.to_string_lossy();
    ffmpeg(
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=duration=2",
            "-i",
            &meta,
            "-map",
            "0",
            "-map_chapters",
            "1",
            "-c:a",
            "flac",
            "-metadata",
            "title=Film",
            "-metadata",
            "artist=Crew",
        ],
        &path,
    );
    let sound = audio(&path);
    let options = listing();
    let rows = write_chapter_rows(
        &path,
        &[(1, fields(&[("COMMENT", &["Intro scene"])]))],
        false,
        &options,
    )
    .unwrap();
    assert_eq!(rows[0].fields["COMMENT"], ["Intro scene"]);
    let entry = read_entry(&path).unwrap();
    assert!(!entry.fields.contains_key("COMMENT"));
    assert_eq!(entry.chapters[0].fields["COMMENT"], ["Intro scene"]);
    assert!(entry.chapters[1].fields.is_empty());
    // ffprobe still reads both chapters and the tags; the audio is unchanged.
    assert_eq!(
        probe(&path, "chapter=start_time:chapter_tags=title", &[]),
        "0.000000,Opening\n1.000000,Second part\n"
    );
    assert_eq!(audio(&path), sound);
    // The same value on both rows: a file tag again.
    write_chapter_rows(
        &path,
        &[(2, fields(&[("COMMENT", &["Intro scene"])]))],
        false,
        &options,
    )
    .unwrap();
    let entry = read_entry(&path).unwrap();
    assert_eq!(entry.fields["COMMENT"], ["Intro scene"]);
    assert!(entry.chapters.iter().all(|c| c.fields.is_empty()));
    assert_eq!(audio(&path), sound);
}

#[test]
fn cue_tracks_play_from_the_file_of_their_index_01() {
    if !tools() {
        eprintln!("Skipping cue sheets across files: ffmpeg is required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for name in ["disc-1.flac", "disc-2.flac"] {
        ffmpeg(
            &["-f", "lavfi", "-i", "sine=duration=2", "-c:a", "flac"],
            &dir.path().join(name),
        );
    }
    let cue = dir.path().join("gaps.cue");
    fs::write(
        &cue,
        "PERFORMER \"The Duo\"\r\nFILE \"disc-1.flac\" WAVE\r\n  TRACK 01 AUDIO\r\n    TITLE \"First\"\r\n    INDEX 01 00:00:00\r\n  TRACK 02 AUDIO\r\n    TITLE \"Second\"\r\n    INDEX 00 00:01:50\r\nFILE \"disc-2.flac\" WAVE\r\n    INDEX 01 00:00:00\r\n",
    )
    .unwrap();
    let loaded = load_playlist(&cue, None).unwrap();
    let base = cue.to_string_lossy().into_owned();
    let rows: Vec<String> = loaded
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(rows, [subsong_path(&base, 1), subsong_path(&base, 2)]);
    let scanned = scan_paths_with_options(&rows, false, &TagOptions::default(), None);
    assert!(scanned.errors.is_empty(), "{:?}", scanned.errors);
    let (first, second) = (&scanned.entries[0], &scanned.entries[1]);
    assert_eq!(first.filename, "disc-1.flac");
    assert_eq!(second.filename, "disc-2.flac");
    // Track 1 keeps the pregap of track 2 at the end of its file.
    assert!((first.duration - 2.0).abs() < 0.01, "{}", first.duration);
    assert!((second.duration - 2.0).abs() < 0.01, "{}", second.duration);
    assert_eq!(second.fields["TITLE"], ["Second"]);
    assert_eq!(second.technical["_SUBSONG"], ["1"]);
    assert_eq!(
        dunce::canonicalize(tag_core::subsong_file(Path::new(&rows[1])).unwrap()).unwrap(),
        dunce::canonicalize(dir.path().join("disc-2.flac")).unwrap()
    );
}
