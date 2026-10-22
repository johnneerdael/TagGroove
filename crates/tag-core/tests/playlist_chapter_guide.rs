//! Written playlist/CUE examples against actual core readers/writers and row model.
mod guide_examples;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tag_core::{
    evaluate, evaluate_filename_with_context,
    playlist::{
        decode_playlist, load_playlist, parse_entries, render_playlist, PlaylistItem, PlaylistKind,
        PlaylistWriteOptions,
    },
    read_entry, subsong_file, write_chapter_rows, write_fields, EvalContext, Fields, TagOptions,
};
const GUIDE: &str = include_str!("../../../docs/user-guide/export/playlists.md");
const CHAPTER: &str = include_str!("../../../docs/user-guide/tagging/chapters.md");
const RECIPE: &str = include_str!("../../../docs/user-guide/export/recipes/playlist-per-album.md");
fn forms(page: &str) -> [String; 2] {
    let lf = page.replace("\r\n", "\n");
    [lf.clone(), lf.replace('\n', "\r\n")]
}
fn section<'a>(page: &'a str, marker: &str) -> &'a str {
    page.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
}
fn block(page: &str, marker: &str) -> String {
    let blocks = guide_examples::text_blocks(section(page, marker));
    assert_eq!(blocks.len(), 1);
    blocks[0].clone()
}
fn source_block(page: &str, marker: &str) -> String {
    let text = block(page, marker);
    if page.contains("\r\n") {
        text.replace('\n', "\r\n")
    } else {
        text
    }
}
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    section(page, marker)
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn marked<'a>(page: &'a str, marker: &str) -> &'a str {
    page.split(&format!("<!-- {marker} -->"))
        .nth(1)
        .unwrap()
        .split('`')
        .nth(1)
        .unwrap()
}
fn fields(items: &[(&str, &str)]) -> Fields {
    items
        .iter()
        .map(|(k, v)| ((*k).into(), vec![(*v).into()]))
        .collect()
}
fn wav(path: &Path, seconds: u32) {
    let audio = vec![0u8; seconds as usize * 8000 * 2];
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + audio.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((audio.len() as u32).to_le_bytes());
    bytes.extend(audio);
    fs::write(path, bytes).unwrap();
}
fn playlist_fixture(page: &str, dir: &Path) -> Vec<PlaylistItem> {
    let cases = rows(page, "playlist-write-items");
    assert_eq!(cases.len(), 2);
    cases
        .into_iter()
        .map(|row| {
            let path = dir.join(code(row[0]));
            wav(&path, code(row[1]).parse().unwrap());
            write_fields(
                &path,
                &fields(&[("ARTIST", code(row[2])), ("TITLE", code(row[3]))]),
            )
            .unwrap();
            let entry = read_entry(&path).unwrap();
            assert_eq!(entry.duration, code(row[1]).parse::<f64>().unwrap());
            PlaylistItem {
                path,
                duration: entry.duration,
                title: evaluate(marked(page, "playlist-extinf-format"), &entry.fields).unwrap(),
            }
        })
        .collect()
}
#[test]
fn written_m3u8_and_pls_outputs_roundtrip_actual_practice_wave_files_without_changing_them() {
    for page in forms(GUIDE) {
        let dir = tempfile::tempdir().unwrap();
        let items = playlist_fixture(&page, dir.path());
        let original: Vec<_> = items
            .iter()
            .map(|item| fs::read(&item.path).unwrap())
            .collect();
        let options = PlaylistWriteOptions {
            extended: true,
            relative: true,
            root: Some(dir.path().into()),
            ..Default::default()
        };
        for (extension, marker) in [
            ("m3u8", "playlist-write-m3u8"),
            ("pls", "playlist-write-pls"),
        ] {
            let target = dir.path().join(format!("practice.{extension}"));
            let bytes = render_playlist(&target, &items, &options).unwrap();
            let expected = block(&page, marker)
                .lines()
                .collect::<Vec<_>>()
                .join("\r\n")
                + "\r\n";
            assert_eq!(bytes, expected.as_bytes());
            fs::write(&target, bytes).unwrap();
            let loaded = load_playlist(&target, None).unwrap();
            assert!(loaded.missing.is_empty());
            assert_eq!(
                loaded.files,
                items
                    .iter()
                    .map(|item| item.path.clone())
                    .collect::<Vec<_>>()
            );
        }
        for (item, before) in items.iter().zip(original) {
            assert_eq!(fs::read(&item.path).unwrap(), before);
        }
    }
}
#[test]
fn written_loaded_playlist_preserves_first_order_and_reports_missing_and_remote_entries() {
    for page in forms(GUIDE) {
        let dir = tempfile::tempdir().unwrap();
        let items = playlist_fixture(&page, dir.path());
        let target = dir.path().join("list.m3u8");
        fs::write(&target, source_block(&page, "playlist-load-example")).unwrap();
        let loaded = load_playlist(&target, None).unwrap();
        assert_eq!(loaded.files, [items[1].path.clone(), items[0].path.clone()]);
        assert_eq!(loaded.missing.len(), 2);
        assert_eq!(loaded.missing[0].0, 5);
        assert!(loaded.missing[0].1.contains("file not found"));
        assert_eq!(loaded.missing[1].0, 6);
        assert!(loaded.missing[1].1.contains("only local files"));
        let reordered = "[playlist]\nFile2=second.wav\nFile1=first.wav\n";
        assert_eq!(
            parse_entries(reordered, PlaylistKind::Pls)
                .iter()
                .map(|entry| entry.location.as_str())
                .collect::<Vec<_>>(),
            ["first.wav", "second.wav"]
        );
    }
}
#[test]
fn written_no_system_codepage_m3u_names_choose_lossless_readback_and_explicit_ansi_can_reject() {
    for page in forms(GUIDE) {
        let cases = rows(&page, "playlist-encoding-cases");
        assert_eq!(cases.len(), 3);
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("list.m3u");
        let options = PlaylistWriteOptions {
            relative: true,
            ..Default::default()
        };
        for row in cases {
            let name = code(row[0]);
            let items = [PlaylistItem {
                path: dir.path().join(name),
                duration: 1.0,
                title: String::new(),
            }];
            let bytes = render_playlist(&target, &items, &options).unwrap();
            let text = decode_playlist(&bytes, PlaylistKind::M3u, None);
            assert_eq!(text, format!("{name}\r\n"));
            match row[1] {
                "UTF-8" => assert!(std::str::from_utf8(&bytes).is_ok()),
                "Windows-1252" => assert!(std::str::from_utf8(&bytes).is_err()),
                other => panic!("Unknown written encoding {other}"),
            }
        }
        let items = [PlaylistItem {
            path: dir.path().join("港.wav"),
            duration: 1.0,
            title: String::new(),
        }];
        let explicit = PlaylistWriteOptions {
            ansi_codepage: Some("windows-1252".into()),
            ..options
        };
        assert!(render_playlist(&target, &items, &explicit)
            .unwrap_err()
            .contains(".m3u8"));
    }
}
#[test]
fn written_external_cue_rows_read_titles_artists_and_timing_then_edit_sheet_not_audio() {
    for page in forms(CHAPTER) {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("image.wav");
        wav(&audio, 3);
        write_fields(
            &audio,
            &fields(&[("TITLE", "Audio file title"), ("ARTIST", "File artist")]),
        )
        .unwrap();
        let before_audio = fs::read(&audio).unwrap();
        let sheet = dir.path().join("practice.cue");
        let text = source_block(&page, "cue-guide-source");
        fs::write(&sheet, &text).unwrap();
        let loaded = load_playlist(&sheet, None).unwrap();
        assert!(loaded.missing.is_empty());
        assert_eq!(loaded.files.len(), 2);
        let expected = rows(&page, "cue-guide-rows");
        assert_eq!(expected.len(), 2);
        for (path, row) in loaded.files.iter().zip(expected) {
            let entry = read_entry(path).unwrap();
            for (name, value) in [
                ("TRACK", row[0]),
                ("TITLE", row[1]),
                ("ARTIST", row[2]),
                ("ALBUM", row[3]),
            ] {
                assert_eq!(entry.fields[name], [code(value)]);
            }
            assert_eq!(entry.technical["_SUBSONG"], [code(row[4])]);
            assert_eq!(entry.technical["_SUBSONG_COUNT"], [code(row[5])]);
            assert_eq!(entry.duration, code(row[6]).parse::<f64>().unwrap());
            assert_eq!(subsong_file(path).unwrap(), audio);
        }
        write_fields(
            &loaded.files[0],
            &fields(&[("TITLE", marked(&page, "cue-title-edit"))]),
        )
        .unwrap();
        let edited = fs::read_to_string(&sheet).unwrap();
        assert_eq!(
            edited.replace(
                &format!("TITLE \"{}\"", marked(&page, "cue-title-edit")),
                &format!("TITLE \"{}\"", code(rows(&page, "cue-guide-rows")[0][1]))
            ),
            text
        );
        assert_eq!(
            read_entry(&loaded.files[1]).unwrap().fields["TITLE"],
            ["Low Tide"]
        );
        write_fields(
            &loaded.files[0],
            &fields(&[("ALBUM", marked(&page, "cue-album-edit"))]),
        )
        .unwrap();
        for path in &loaded.files {
            assert_eq!(
                read_entry(path).unwrap().fields["ALBUM"],
                [marked(&page, "cue-album-edit")]
            );
        }
        assert_eq!(fs::read(&audio).unwrap(), before_audio);
    }
}
#[test]
fn cue_track_number_unsupported_fields_and_conflicting_shared_values_keep_sheet_unchanged() {
    for page in forms(CHAPTER) {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("image.wav");
        wav(&audio, 3);
        let sheet = dir.path().join("practice.cue");
        fs::write(&sheet, source_block(&page, "cue-guide-source")).unwrap();
        let loaded = load_playlist(&sheet, None).unwrap();
        let before = fs::read(&sheet).unwrap();
        for (changes, wanted) in [
            (fields(&[("TRACK", "7")]), "TRACK lines"),
            (fields(&[("CUSTOM_FIELD", "text")]), "cannot store"),
        ] {
            assert!(write_fields(&loaded.files[0], &changes)
                .unwrap_err()
                .contains(wanted));
            assert_eq!(fs::read(&sheet).unwrap(), before);
        }
        let request = [
            (1, fields(&[("ALBUM", "One")])),
            (2, fields(&[("ALBUM", "Two")])),
        ];
        assert!(
            write_chapter_rows(&sheet, &request, false, &TagOptions::default())
                .unwrap_err()
                .contains("stored once")
        );
        assert_eq!(fs::read(&sheet).unwrap(), before);
    }
}
#[test]
fn written_album_playlist_partition_and_filename_expressions_use_each_members_actual_folder() {
    for page in forms(RECIPE) {
        let blocks = guide_examples::text_blocks(&page);
        assert_eq!(blocks.len(), 4);
        let members = rows(&page, "album-playlist-members");
        assert_eq!(members.len(), 3);
        let outputs = rows(&page, "album-playlist-outputs");
        let disc_outputs = rows(&page, "album-playlist-disc-outputs");
        assert_eq!(disc_outputs.len(), 3);
        assert_eq!(outputs.len(), 2);
        let root = tempfile::tempdir().unwrap();
        let mut seen = Vec::new();
        for row in members {
            let folder = root.path().join(code(row[1]));
            let mut folder_text = folder.to_string_lossy().into_owned();
            folder_text.push(std::path::MAIN_SEPARATOR);
            let snapshot = fields(&[("_FOLDERPATH", &folder_text), ("DISCNUMBER", code(row[3]))]);
            let context = EvalContext::new(&snapshot);
            assert_eq!(evaluate(&blocks[0], &snapshot).unwrap(), folder_text);
            let target =
                PathBuf::from(evaluate_filename_with_context(&blocks[1], &context).unwrap());
            let expected = root.path().join(code(
                outputs[if code(row[1]) == "Harbour Lights" {
                    0
                } else {
                    1
                }][0],
            ));
            assert_eq!(target, expected);
            assert_eq!(
                evaluate(&blocks[2], &snapshot).unwrap(),
                format!("{folder_text}-{}", code(row[3]))
            );
            let disc_target =
                PathBuf::from(evaluate_filename_with_context(&blocks[3], &context).unwrap());
            assert_eq!(
                disc_target,
                root.path().join(code(
                    disc_outputs[code(row[0]).parse::<usize>().unwrap() - 1][1]
                ))
            );
            if !seen.contains(&target) {
                seen.push(target);
            }
        }
        assert_eq!(seen.len(), 2);
    }
}
