//! Actual written cover recipes on generated images and independent MPEG fixtures.
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path};
use tag_core::{
    actions::{
        apply_action_spec, ActionEffect, ActionSpec, ActionState, CoverImageFormat, CoverResize,
        CoverScaleMode, CoverSizeMode,
    },
    add_cover_bytes_with_options, adjust_cover_image, apply_cover_changes_with_options,
    import_cover_with_options, read_entry_summary, resolve_cover_source, CoverChange, CoverRef,
    Fields, TagOptions,
};
const EMBED: &str = include_str!("../../../docs/user-guide/tagging/recipes/embed-folder-jpg.md");
const RESIZE: &str = include_str!("../../../docs/user-guide/tagging/recipes/resize-covers.md");
const FRONT: &str =
    include_str!("../../../docs/user-guide/tagging/recipes/keep-only-front-cover.md");
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
fn rows<'a>(page: &'a str, marker: &str) -> Vec<Vec<&'a str>> {
    section(page, marker)
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect()
}
fn code(value: &str) -> &str {
    value.strip_prefix('`').unwrap().strip_suffix('`').unwrap()
}
fn image(width: u32, height: u32, format: ImageFormat, colour: u8) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(RgbImage::from_pixel(width, height, Rgb([colour, 30, 180])))
        .write_to(&mut out, format)
        .unwrap();
    out.into_inner()
}
fn mp3(path: &Path) -> Vec<u8> {
    let mut audio = Vec::new();
    for frame in 0..20u32 {
        audio.extend([0xff, 0xfb, 0x90, 0x64]);
        audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
    }
    fs::write(path, &audio).unwrap();
    audio
}
fn filename(template: &str, folder: &Path) -> String {
    let mut state = ActionState {
        fields: Fields::new(),
        context: Fields::from([(
            "_FOLDERPATH".into(),
            vec![format!("{}{}", folder.display(), std::path::MAIN_SEPARATOR)],
        )]),
    };
    match apply_action_spec(
        &mut state,
        &ActionSpec::ImportCover {
            mta_source: None,
            filename_template: template.into(),
            picture_type: None,
            delete_existing: false,
        },
    )
    .unwrap()
    .as_slice()
    {
        [ActionEffect::ImportCover { filename, .. }] => filename.clone(),
        other => panic!("Unexpected effect {other:?}"),
    }
}
fn role(label: &str) -> &str {
    match label {
        "Front Cover" => "CoverFront",
        "Back Cover" => "CoverBack",
        "Leaflet" => "Leaflet",
        other => panic!("Unknown written role {other}"),
    }
}
#[test]
fn embed_settings_and_count_cases_run_on_actual_mp3_writes_or_invalid_source_refusal() {
    for page in forms(EMBED) {
        let settings = rows(&page, "embed-settings")
            .into_iter()
            .map(|r| (r[0], r[1]))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(settings["Import cover as"], "Front Cover");
        assert_eq!(settings["Delete existing covers"], "Off");
        let cases = rows(&page, "embed-count-cases");
        assert_eq!(cases.len(), 5);
        for row in cases {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("practice.mp3");
            let audio = mp3(&path);
            let count = code(row[0]).parse::<usize>().unwrap();
            for i in 0..count {
                add_cover_bytes_with_options(
                    &path,
                    image(2, 2, ImageFormat::Png, 40 + i as u8),
                    "old.png",
                    Some(if i == 0 { "CoverBack" } else { "Leaflet" }),
                    &TagOptions::default(),
                )
                .unwrap();
            }
            assert_eq!(read_entry_summary(&path).unwrap().covers.len(), count);
            let target = filename(code(settings["Image filename format"]), dir.path());
            fs::write(
                &target,
                if row[2] == "Valid" {
                    image(3, 2, ImageFormat::Jpeg, 210)
                } else {
                    b"not an image".to_vec()
                },
            )
            .unwrap();
            let before = fs::read(&path).unwrap();
            let result = import_cover_with_options(
                &path,
                &resolve_cover_source(Path::new(&target)).unwrap(),
                Some("CoverFront"),
                row[1] == "On",
                &TagOptions::default(),
            );
            if row[2] == "Valid" {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
                assert_eq!(fs::read(&path).unwrap(), before);
            }
            assert_eq!(
                read_entry_summary(&path).unwrap().covers.len(),
                code(row[3]).parse::<usize>().unwrap()
            );
            assert!(fs::read(&path)
                .unwrap()
                .windows(audio.len())
                .any(|w| w == audio));
        }
    }
}
#[test]
fn exact_written_wildcard_candidates_choose_the_first_importable_natural_name() {
    for page in forms(EMBED) {
        let dir = tempfile::tempdir().unwrap();
        let cases = rows(&page, "embed-wildcard-cases");
        assert_eq!(cases.len(), 4);
        for row in &cases {
            let path = dir.path().join(code(row[0]));
            match row[1] {
                "Plain text, not an image" => fs::write(path, b"plain text").unwrap(),
                "Valid PNG" => fs::write(path, image(2, 2, ImageFormat::Png, 100)).unwrap(),
                "A folder" => fs::create_dir(path).unwrap(),
                other => panic!("Unknown written candidate {other}"),
            }
        }
        let template = section(&page, "embed-wildcard-format")
            .split('`')
            .nth(1)
            .unwrap();
        let actual = resolve_cover_source(Path::new(&filename(template, dir.path()))).unwrap();
        let expected = code(cases.iter().find(|r| r[2] == "Chosen").unwrap()[0]);
        assert_eq!(actual.file_name().unwrap().to_string_lossy(), expected);
    }
}
fn size_mode(text: &str) -> CoverSizeMode {
    match text {
        "Maximum size" => CoverSizeMode::Maximum,
        "Minimum size" => CoverSizeMode::Minimum,
        other => panic!("Unknown written policy {other}"),
    }
}
fn scale(text: &str) -> CoverScaleMode {
    match text {
        "Scale proportionally" => CoverScaleMode::Proportional,
        "Scale to square" => CoverScaleMode::Square,
        "Crop to square" => CoverScaleMode::CropSquare,
        other => panic!("Unknown written scaling {other}"),
    }
}
#[test]
fn actual_resize_settings_and_all_dimension_examples_preserve_small_original_bytes() {
    for page in forms(RESIZE) {
        let settings = rows(&page, "resize-settings")
            .into_iter()
            .map(|r| (r[0], r[1]))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(settings["Image format"], "Original");
        assert_eq!(settings["Resize cover"], "On");
        let quality = settings["Quality"]
            .split('`')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let pixels = code(settings["Size in pixels"]).parse().unwrap();
        let main = rows(&page, "resize-main-cases");
        assert_eq!(main.len(), 4);
        for row in main {
            let width = code(row[0]).parse().unwrap();
            let height = code(row[1]).parse().unwrap();
            let bytes = image(width, height, ImageFormat::Png, 100);
            let changed = adjust_cover_image(
                &bytes,
                CoverImageFormat::Original,
                quality,
                Some(&CoverResize {
                    native_geometry: true,
                    size_enabled: true,
                    pixels,
                    size_mode: size_mode(settings["Size policy"]),
                    scale_mode: scale(settings["Scaling"]),
                }),
            )
            .unwrap();
            let result = image::load_from_memory(changed.as_ref().unwrap_or(&bytes)).unwrap();
            assert_eq!(
                (result.width(), result.height()),
                (code(row[2]).parse().unwrap(), code(row[3]).parse().unwrap())
            );
            if width <= pixels && height <= pixels {
                assert!(changed.is_none());
            }
        }
        let cases = rows(&page, "resize-policy-cases");
        assert_eq!(cases.len(), 6);
        for row in cases {
            let bytes = image(
                code(row[0]).parse().unwrap(),
                code(row[1]).parse().unwrap(),
                ImageFormat::Png,
                110,
            );
            let changed = adjust_cover_image(
                &bytes,
                CoverImageFormat::Original,
                quality,
                Some(&CoverResize {
                    native_geometry: true,
                    size_enabled: true,
                    pixels,
                    size_mode: size_mode(row[2]),
                    scale_mode: scale(row[3]),
                }),
            )
            .unwrap();
            let result = image::load_from_memory(changed.as_ref().unwrap_or(&bytes)).unwrap();
            assert_eq!(
                (result.width(), result.height()),
                (code(row[4]).parse().unwrap(), code(row[5]).parse().unwrap())
            );
        }
    }
}
#[test]
fn front_only_plan_retains_each_files_membership_and_apply_to_all_is_different() {
    for page in forms(FRONT) {
        let dir = tempfile::tempdir().unwrap();
        let catalog = rows(&page, "front-catalog");
        assert_eq!(catalog.len(), 4);
        let images = catalog
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    code(r[0]).to_owned(),
                    image(2, 2, ImageFormat::Png, 30 + i as u8 * 30),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let cases = rows(&page, "front-membership");
        assert_eq!(cases.len(), 3);
        let mut paths = Vec::new();
        let mut refs = BTreeMap::<String, CoverRef>::new();
        for row in &cases {
            let path = dir.path().join(code(row[0]));
            mp3(&path);
            let labels: Vec<String> = serde_json::from_str(code(row[1])).unwrap();
            let changes = labels
                .iter()
                .map(|label| {
                    let entry = catalog.iter().find(|r| code(r[0]) == label).unwrap();
                    CoverChange {
                        image: Some(images[label].clone()),
                        picture_type: role(entry[1]).into(),
                        description: code(entry[2]).into(),
                        all_files: true,
                        ..Default::default()
                    }
                })
                .collect::<Vec<_>>();
            let entry =
                apply_cover_changes_with_options(&path, &changes, &TagOptions::default()).unwrap();
            for (label, cover) in labels.iter().zip(&entry.covers) {
                refs.insert(label.clone(), CoverRef::of(cover));
            }
            paths.push(path);
        }
        let kept = catalog
            .iter()
            .filter(|r| r[1] == "Front Cover")
            .map(|r| code(r[0]))
            .collect::<Vec<_>>();
        let changes = kept
            .iter()
            .map(|label| {
                let original = refs[*label].clone();
                CoverChange {
                    picture_type: original.picture_type.clone(),
                    description: original.description.clone(),
                    original: Some(original),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        for (path, row) in paths.iter().zip(&cases) {
            let entry =
                apply_cover_changes_with_options(path, &changes, &TagOptions::default()).unwrap();
            let actual = entry
                .covers
                .iter()
                .map(|c| {
                    refs.iter()
                        .find(|(_, reference)| **reference == CoverRef::of(c))
                        .unwrap()
                        .0
                        .clone()
                })
                .collect::<Vec<_>>();
            let expected: Vec<String> = serde_json::from_str(code(row[2])).unwrap();
            assert_eq!(actual, expected);
        }
        let mut redistribute = changes.clone();
        redistribute[0].all_files = true;
        redistribute[0].image = Some(images[kept[0]].clone());
        let expected = [vec![kept[0]], vec![kept[0], kept[1]], vec![kept[0]]];
        for (path, wanted) in paths.iter().zip(expected) {
            let entry =
                apply_cover_changes_with_options(path, &redistribute, &TagOptions::default())
                    .unwrap();
            let actual = entry
                .covers
                .iter()
                .map(|c| {
                    refs.iter()
                        .find(|(_, reference)| **reference == CoverRef::of(c))
                        .unwrap()
                        .0
                        .as_str()
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, wanted);
        }
    }
}
