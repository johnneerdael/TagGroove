//! Actual written first-use folder scanning, not a native picker/install test.
mod guide_examples;
use std::{fs, path::Path};
use tag_core::{scan_paths_with_options, TagOptions};
const PAGE: &str = include_str!("../../../docs/user-guide/getting-started/opening-files.md");
fn section<'a>(page: &'a str, marker: &str) -> &'a str {
    page.split(&format!("<!-- {marker}:start -->"))
        .nth(1)
        .unwrap()
        .split(&format!("<!-- {marker}:end -->"))
        .next()
        .unwrap()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn audio(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    if path.extension().unwrap() == "mp3" {
        let mut bytes = Vec::new();
        for frame in 0..20u32 {
            bytes.extend([0xff, 0xfb, 0x90, 0x64]);
            bytes.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
        }
        fs::write(path, bytes).unwrap();
    } else {
        let data = vec![0u8; 16000];
        let mut bytes = b"RIFF".to_vec();
        bytes.extend(16036u32.to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(data);
        fs::write(path, bytes).unwrap();
    }
}
#[test]
fn written_folder_recursion_and_extension_cases_read_actual_generated_files_without_changes() {
    let lf = PAGE.replace("\r\n", "\n");
    for page in [lf.clone(), lf.replace('\n', "\r\n")] {
        let dir = tempfile::tempdir().unwrap();
        let files = guide_examples::text_blocks(section(&page, "first-use-scan-files"));
        assert_eq!(files.len(), 1);
        let paths: Vec<_> = files[0].lines().map(|name| dir.path().join(name)).collect();
        assert_eq!(paths.len(), 3);
        for path in &paths {
            audio(path);
        }
        let before: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
        let cases: Vec<_> = section(&page, "first-use-scan-cases")
            .lines()
            .filter(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| {
                line.trim_matches('|')
                    .split('|')
                    .map(str::trim)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(cases.len(), 4);
        // The production scanner returns canonical paths (e.g. /private/var on macOS).
        let root = dunce::canonicalize(dir.path().join("Practice")).unwrap();
        for row in cases {
            let input = if row[0] == "Folder" {
                root.clone()
            } else {
                root.join(code(row[0].strip_prefix("Explicit file ").unwrap()))
            };
            let allowed = if row[2] == "(empty)" {
                None
            } else {
                Some(vec![code(row[2]).to_string()])
            };
            let scan = scan_paths_with_options(
                &[input.to_string_lossy().into_owned()],
                row[1] == "Yes",
                &TagOptions::default(),
                allowed.as_deref(),
            );
            assert!(scan.errors.is_empty(), "{:?}", scan.errors);
            let mut found: Vec<_> = scan
                .entries
                .iter()
                .map(|entry| {
                    Path::new(&entry.path)
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .collect();
            let mut expected: Vec<_> = row[3]
                .split(';')
                .map(|v| code(v.trim()).to_string())
                .collect();
            found.sort();
            expected.sort();
            assert_eq!(found, expected);
        }
        for (path, original) in paths.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), original);
        }
    }
}
