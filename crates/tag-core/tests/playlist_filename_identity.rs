//! Authored playlist data must select the intended member even when a legacy
//! trimmed or percent-decoded spelling names a different existing file.
use std::{
    cell::Cell,
    path::{Path, PathBuf},
};
use tag_core::playlist::{
    decode_playlist, entry_candidates, load_playlist, parse_entries, render_playlist,
    resolve_entries, PlaylistItem, PlaylistKind, PlaylistWriteOptions, RawEntry,
};

#[test]
fn parsers_preserve_location_data_after_format_syntax() {
    for kind in [PlaylistKind::M3u, PlaylistKind::M3u8] {
        let parsed = parse_entries("\u{feff}#EXTM3U\n\u{feff}\u{feff}  Leading.wav \n \u{feff}literal.wav \n   \n #comment\n", kind);
        assert_eq!(
            parsed,
            [
                RawEntry {
                    line: 2,
                    location: "  Leading.wav ".into()
                },
                RawEntry {
                    line: 3,
                    location: " \u{feff}literal.wav ".into()
                },
            ]
        );
    }
    let pls = parse_entries("[playlist]\n File2 = Trailing.wav \n\u{feff} File1 =\u{feff}Intro.wav \nfile2= Other.wav\nFile3=   \n", PlaylistKind::Pls);
    assert_eq!(
        pls,
        [
            RawEntry {
                line: 3,
                location: "\u{feff}Intro.wav ".into()
            },
            RawEntry {
                line: 2,
                location: " Trailing.wav ".into()
            },
            RawEntry {
                line: 4,
                location: " Other.wav".into()
            },
        ]
    );
    let cue = "\u{feff} FILE \" \u{feff}Disc.wav \" WAVE\nFILE track two.wav WAVE\n";
    assert_eq!(
        parse_entries(cue, PlaylistKind::Cue)
            .iter()
            .map(|e| e.location.as_str())
            .collect::<Vec<_>>(),
        [" \u{feff}Disc.wav ", "track two.wav"]
    );
}

#[test]
fn explicit_uri_classification_precedes_all_local_probes() {
    for kind in [PlaylistKind::M3u, PlaylistKind::M3u8] {
        let raw = parse_entries(
            "#EXTM3U\n\u{feff}Title://song.wav\n\u{feff}file:///Help%ZZ{error}\n",
            kind,
        );
        let calls = Cell::new(0);
        let loaded = resolve_entries(&raw, Path::new("virtual"), |_| {
            calls.set(calls.get() + 1);
            true
        });
        assert_eq!(calls.get(), 0);
        assert!(loaded.files.is_empty());
        assert_eq!(
            loaded.missing,
            [
                (
                    2,
                    "Title://song.wav: only local files can be loaded, not Title URLs".into()
                ),
                (3, "file:///Help%ZZ{error}: invalid file URL".into()),
            ]
        );
    }
    let raw = [RawEntry {
        line: 7,
        location: "  Title://song.wav  ".into(),
    }];
    let loaded = resolve_entries(&raw, Path::new("virtual"), |_| {
        panic!("URI reached local probe")
    });
    assert_eq!(
        loaded.missing,
        [(
            7,
            "Title://song.wav: only local files can be loaded, not Title URLs".into()
        )]
    );
    // A BOM in a PLS value or quoted CUE FILE is data, not M3U syntax.
    let location = "\u{feff}Title://song.wav";
    let expected = Path::new("virtual").join("\u{feff}Title:").join("song.wav");
    for raw in [
        parse_entries(&format!("File1={location}\n"), PlaylistKind::Pls),
        parse_entries(&format!("FILE \"{location}\" WAVE\n"), PlaylistKind::Cue),
    ] {
        assert_eq!(raw[0].location, location);
        assert_eq!(
            resolve_entries(&raw, Path::new("virtual"), |p| p == expected)
                .files
                .as_slice(),
            std::slice::from_ref(&expected)
        );
    }
}

#[test]
fn candidate_families_preserve_raw_data_before_legacy_trimmed_fallback() {
    let directory = Path::new("virtual");
    let spelling = " Album\\Percent%20Track.wav ";
    let names: &[&str] = if cfg!(windows) {
        &[
            " Album\\Percent%20Track.wav ",
            " Album\\Percent Track.wav ",
            "Album\\Percent%20Track.wav",
            "Album\\Percent Track.wav",
        ]
    } else {
        &[
            " Album\\Percent%20Track.wav ",
            " Album/Percent%20Track.wav ",
            " Album\\Percent Track.wav ",
            " Album/Percent Track.wav ",
            "Album\\Percent%20Track.wav",
            "Album/Percent%20Track.wav",
            "Album\\Percent Track.wav",
            "Album/Percent Track.wav",
        ]
    };
    assert_eq!(
        entry_candidates(spelling, directory).unwrap(),
        names
            .iter()
            .map(|name| directory.join(name))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        entry_candidates("./Album/../Plain.wav", directory).unwrap(),
        [directory.join("Plain.wav")]
    );
    assert_eq!(
        entry_candidates("Percent%20Intro.wav", directory).unwrap(),
        [
            directory.join("Percent%20Intro.wav"),
            directory.join("Percent Intro.wav")
        ]
    );
    let raw = [RawEntry {
        line: 1,
        location: " Missing.wav ".into(),
    }];
    let expected = directory.join("Missing.wav");
    assert_eq!(
        resolve_entries(&raw, directory, |p| p == expected).files,
        [expected]
    );
}

#[test]
fn relative_writers_escape_only_ambiguous_location_syntax() {
    let directory = Path::new("virtual");
    for (name, m3u_location, pls_location) in [
        ("#Intro.wav", "./#Intro.wav", "#Intro.wav"),
        (" #Intro.wav", "./ #Intro.wav", " #Intro.wav"),
        (
            "\u{feff}Intro.wav",
            "./\u{feff}Intro.wav",
            "\u{feff}Intro.wav",
        ),
        ("   ", "./   ", "./   "),
        (" Leading.wav", " Leading.wav", " Leading.wav"),
        ("Trailing.wav ", "Trailing.wav ", "Trailing.wav "),
    ] {
        for extension in ["m3u", "m3u8", "pls"] {
            for extended in [false, true] {
                for bom in [false, true] {
                    let target = directory.join(format!("list.{extension}"));
                    let options = PlaylistWriteOptions {
                        relative: true,
                        extended,
                        bom,
                        ..Default::default()
                    };
                    let items = [PlaylistItem {
                        path: directory.join(name),
                        duration: 1.0,
                        title: "Owned".into(),
                    }];
                    let bytes = render_playlist(&target, &items, &options).unwrap();
                    let kind = if extension == "pls" {
                        PlaylistKind::Pls
                    } else if extension == "m3u" {
                        PlaylistKind::M3u
                    } else {
                        PlaylistKind::M3u8
                    };
                    let text = decode_playlist(&bytes, kind, None);
                    let expected = if extension == "pls" {
                        pls_location
                    } else {
                        m3u_location
                    };
                    assert_eq!(
                        parse_entries(&text, kind)
                            .iter()
                            .map(|e| e.location.as_str())
                            .collect::<Vec<_>>(),
                        [expected],
                        "{name:?}/{extension}/{extended}/{bom}: {text:?}"
                    );
                    if extension == "pls" {
                        assert!(text.contains(&format!("File1={expected}\r\n")));
                    } else {
                        assert!(text.ends_with(&format!("{expected}\r\n")));
                    }
                }
            }
        }
    }
}

#[test]
fn line_break_paths_still_refuse_every_writable_format() {
    for extension in ["m3u", "m3u8", "pls"] {
        for relative in [false, true] {
            for name in ["bad\nname.wav", "bad\rname.wav"] {
                let options = PlaylistWriteOptions {
                    relative,
                    ..Default::default()
                };
                let items = [PlaylistItem {
                    path: Path::new("virtual").join(name),
                    duration: 1.0,
                    title: "".into(),
                }];
                assert_eq!(
                    render_playlist(
                        &Path::new("virtual").join(format!("list.{extension}")),
                        &items,
                        &options
                    )
                    .unwrap_err(),
                    "Playlist paths cannot contain line breaks"
                );
            }
        }
    }
}

#[cfg(unix)]
mod unix_identity {
    use super::*;
    use std::{collections::HashSet, fs, io::Write, os::unix::fs::MetadataExt};

    fn create_members(directory: &Path, names: &[&str]) -> Vec<(PathBuf, Vec<u8>)> {
        let mut identities = HashSet::new();
        names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let path = directory.join(name);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                let bytes = format!("independent member {index}: {name}").into_bytes();
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .unwrap()
                    .write_all(&bytes)
                    .unwrap();
                let metadata = fs::metadata(&path).unwrap();
                assert!(
                    identities.insert((metadata.dev(), metadata.ino())),
                    "fixture names alias: {name:?}"
                );
                (path, bytes)
            })
            .collect()
    }

    fn assert_member(actual: &Path, expected: &Path) {
        assert_eq!(actual, expected);
        let actual = fs::metadata(actual).unwrap();
        let expected = fs::metadata(expected).unwrap();
        assert_eq!(
            (actual.dev(), actual.ino()),
            (expected.dev(), expected.ino())
        );
    }

    #[test]
    fn writer_reader_selects_authored_member_with_distinct_existing_siblings() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let sources = create_members(
            root,
            &[
                "#Intro.wav",
                "#Album/Track.wav",
                " #Intro.wav",
                " Leading.wav",
                "Leading.wav",
                "Trailing.wav ",
                "Trailing.wav",
                "\u{feff}Intro.wav",
                "Intro.wav",
                "   ",
                "Percent%20Intro.wav",
                "Percent Intro.wav",
                "file:Intro.wav",
                "C:\\Intro.wav",
                "Plain.wav",
            ],
        );
        // Expected identities are authored independently of the rendered text.
        let expected_names = [
            "#Intro.wav",
            "#Album/Track.wav",
            " #Intro.wav",
            " Leading.wav",
            "Trailing.wav ",
            "\u{feff}Intro.wav",
            "   ",
            "Percent%20Intro.wav",
            "file:Intro.wav",
            "C:\\Intro.wav",
            "Plain.wav",
        ];
        let expected: Vec<_> = expected_names.iter().map(|name| root.join(name)).collect();
        for extension in ["m3u", "m3u8", "pls"] {
            for relative in [false, true] {
                for extended in [false, true] {
                    let target = root.join(format!("roundtrip.{extension}"));
                    let items: Vec<_> = expected
                        .iter()
                        .map(|path| PlaylistItem {
                            path: path.clone(),
                            duration: 1.0,
                            title: "Owned".into(),
                        })
                        .collect();
                    let bytes = render_playlist(
                        &target,
                        &items,
                        &PlaylistWriteOptions {
                            relative,
                            extended,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    fs::write(&target, &bytes).unwrap();
                    let loaded = load_playlist(&target, None).unwrap();
                    assert!(
                        loaded.missing.is_empty(),
                        "{extension}/{relative}/{extended}: {:?}",
                        loaded.missing
                    );
                    assert_eq!(
                        loaded.files.len(),
                        expected.len(),
                        "{extension}/{relative}/{extended}: {:?}",
                        loaded.files
                    );
                    for (actual, expected) in loaded.files.iter().zip(&expected) {
                        assert_member(actual, expected);
                    }
                }
            }
        }
        for (path, bytes) in sources {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn raw_transformed_family_beats_distinct_legacy_siblings_for_playlist_and_cue_rows() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let sources = create_members(
            root,
            &[
                " Album/Track.wav ",
                "Album\\Track.wav",
                "Album/Track.wav",
                " Percent Track.wav ",
                "Percent%20Track.wav",
                "Percent Track.wav",
                " Album/Percent Track.wav ",
                "Album\\Percent%20Track.wav",
                "Album/Percent%20Track.wav",
                "Album\\Percent Track.wav",
                "Album/Percent Track.wav",
                "\u{feff}Title:/song.wav",
                "Title:/song.wav",
            ],
        );
        for (spelling, expected_name) in [
            (" Album\\Track.wav ", " Album/Track.wav "),
            (" Percent%20Track.wav ", " Percent Track.wav "),
            (" Album\\Percent%20Track.wav ", " Album/Percent Track.wav "),
            ("\u{feff}Title://song.wav", "\u{feff}Title:/song.wav"),
        ] {
            let expected = root.join(expected_name);
            let raw = [RawEntry {
                line: 1,
                location: spelling.into(),
            }];
            let resolved = resolve_entries(&raw, root, |path| path.exists());
            assert!(resolved.missing.is_empty());
            assert_eq!(resolved.files.len(), 1);
            assert_member(&resolved.files[0], &expected);
            let cue = root.join("identity.cue");
            fs::write(
                &cue,
                format!("FILE \"{spelling}\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n"),
            )
            .unwrap();
            let loaded = load_playlist(&cue, None).unwrap();
            assert!(loaded.missing.is_empty());
            assert_eq!(
                loaded.files,
                [PathBuf::from(tag_core::subsong_path(
                    &cue.to_string_lossy(),
                    1
                ))]
            );
            assert_member(
                &tag_core::subsong_file(&loaded.files[0]).unwrap(),
                &expected,
            );
        }
        for (path, bytes) in sources {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
}
