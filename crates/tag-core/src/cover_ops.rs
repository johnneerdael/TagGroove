use super::*;
use crate::actions::{CoverImageFormat, CoverResize, CoverScaleMode, CoverSizeMode};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

pub fn picture_type_from_name(name: &str) -> Result<PictureType, String> {
    let name = name.trim();
    if let Ok(number) = name.parse::<u8>() {
        return Ok(PictureType::from_u8(number));
    }
    (0..=20)
        .map(PictureType::from_u8)
        .find(|kind| format!("{kind:?}").eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("Unknown cover type: {name}"))
}

fn transform_covers(
    path: &Path,
    options: &TagOptions,
    mut transform: impl FnMut(&Picture) -> Result<Option<Picture>, String>,
) -> Result<AudioEntry, String> {
    if read_entry_summary_with_options(path, &[], options)?
        .covers
        .is_empty()
    {
        return read_entry_summary_with_options(path, &options.mappings, options);
    }
    if asf::sniff(path) {
        return asf::change_pictures(
            path,
            false,
            options,
            |pictures| {
                let mut edited = Vec::new();
                for picture in pictures.iter() {
                    edited.extend(transform(picture)?);
                }
                *pictures = edited;
                Ok(())
            },
            "The edited covers could not be stored",
        );
    }
    if let Some(kind) = container_formats::detect(path) {
        return container_formats::transform_covers(path, kind, options, transform);
    }
    staged_mutation(path, false, options, |staging, mut file, staged_path| {
        if tag_policy::policy(file.file_type()).is_some() {
            let mut pictures = Vec::new();
            for picture in tag_policy::displayed_pictures(&file, options) {
                if let Some(picture) = transform(&picture)? {
                    pictures.push(picture);
                }
            }
            let written = tag_policy::write(
                staging,
                &file,
                &options.mappings,
                options,
                tag_policy::Change::Pictures(pictures.clone()),
            )?;
            return tag_policy::verify_pictures(
                staged_path,
                options,
                written,
                &pictures,
                "The tag format cannot preserve all edited covers (duplicate type/description)",
            );
        }
        let kinds: Vec<_> = ordered_tags(&file)
            .into_iter()
            .filter(|tag| !tag.pictures().is_empty())
            .map(Tag::tag_type)
            .collect();
        let mut expected = Vec::new();
        for kind in kinds {
            if !file.tag_support(kind).is_writable() {
                return Err("Cover tag is read-only".into());
            }
            let file_type = file.file_type();
            let tag = file.tag_mut(kind).ok_or("Cover tag disappeared")?;
            let original = tag.pictures().to_vec();
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
            for picture in original {
                if let Some(picture) = transform(&picture)? {
                    expected.push((
                        format!("{:?}", picture.pic_type()),
                        picture.description().unwrap_or_default().to_owned(),
                    ));
                    tag.push_picture(picture);
                }
            }
            staging
                .seek(SeekFrom::Start(0))
                .map_err(|e| e.to_string())?;
            save_preserving_native(
                staging,
                file_type,
                tag,
                &Fields::new(),
                true,
                Id3Write::preserving(options),
            )?;
        }
        let actual = read_entry_summary_with_options(staged_path, &[], options)?;
        if actual.covers.len() != expected.len() {
            return Err(
                "The tag format cannot preserve all edited covers (duplicate type/description)"
                    .into(),
            );
        }
        for (cover, (kind, description)) in actual.covers.iter().zip(expected) {
            if cover.picture_type != kind || cover.description != description {
                return Err(
                    "The tag format does not support the requested cover properties".into(),
                );
            }
        }
        Ok(())
    })
}

pub fn remove_all_covers_summary(path: &Path) -> Result<AudioEntry, String> {
    remove_all_covers_with_options(path, &TagOptions::default())
}
pub fn remove_all_covers_with_options(
    path: &Path,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    transform_covers(path, options, |_| Ok(None))
}
pub fn set_cover_properties_summary(
    path: &Path,
    picture_type: Option<&str>,
    description: Option<&str>,
    only_picture_type: Option<&str>,
) -> Result<AudioEntry, String> {
    set_cover_properties_with_options(
        path,
        picture_type,
        description,
        only_picture_type,
        &TagOptions::default(),
    )
}
pub fn set_cover_properties_with_options(
    path: &Path,
    picture_type: Option<&str>,
    description: Option<&str>,
    only_picture_type: Option<&str>,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let kind = picture_type.map(picture_type_from_name).transpose()?;
    let filter = only_picture_type.map(picture_type_from_name).transpose()?;
    transform_covers(path, options, |original| {
        let mut picture = original.clone();
        if filter.is_none_or(|f| f == picture.pic_type()) {
            if let Some(kind) = kind {
                picture.set_pic_type(kind);
            }
            if let Some(description) = description {
                picture.set_description(if description.is_empty() {
                    None
                } else {
                    Some(description.into())
                });
            }
        }
        Ok(Some(picture))
    })
}

fn resize_image(image: DynamicImage, options: &CoverResize) -> Result<DynamicImage, String> {
    if options.pixels == 0 || options.pixels > 32768 {
        return Err("Cover size must be between 1 and 32768 pixels".into());
    }
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return Err("Cover dimensions are empty".into());
    }
    let dimension = match options.size_mode {
        CoverSizeMode::Maximum => width.max(height),
        CoverSizeMode::Minimum => width.min(height),
    };
    // max. size: a cover within the size keeps its size in every scale mode.
    // min. size: the smaller side is matched, up or down.
    let unchanged = match options.size_mode {
        CoverSizeMode::Maximum => dimension <= options.pixels,
        CoverSizeMode::Minimum => {
            dimension == options.pixels
                && (matches!(options.scale_mode, CoverScaleMode::Proportional) || width == height)
        }
    };
    if unchanged {
        return Ok(image);
    }
    let filter = image::imageops::FilterType::Lanczos3;
    Ok(match options.scale_mode {
        CoverScaleMode::Proportional => {
            let scale = options.pixels as f64 / dimension as f64;
            image.resize_exact(
                ((width as f64 * scale).round() as u32).max(1),
                ((height as f64 * scale).round() as u32).max(1),
                filter,
            )
        }
        CoverScaleMode::Square => image.resize_exact(options.pixels, options.pixels, filter),
        CoverScaleMode::CropSquare => image.resize_to_fill(options.pixels, options.pixels, filter),
    })
}

pub fn adjust_covers_summary(
    path: &Path,
    format: CoverImageFormat,
    quality: u8,
    resize: Option<&CoverResize>,
) -> Result<AudioEntry, String> {
    adjust_covers_with_options(path, format, quality, resize, &TagOptions::default())
}
pub fn adjust_covers_with_options(
    path: &Path,
    format: CoverImageFormat,
    quality: u8,
    resize: Option<&CoverResize>,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    check_adjust_parameters(quality, resize)?;
    transform_covers(path, options, |original| {
        let Some(data) = adjust_image(
            original.data(),
            original.mime_type(),
            format,
            quality,
            resize,
        )?
        else {
            // A cover that keeps its size and format is left byte for byte.
            return Ok(Some(original.clone()));
        };
        let mut picture =
            Picture::from_reader(&mut Cursor::new(data)).map_err(|e| e.to_string())?;
        picture.set_pic_type(original.pic_type());
        picture.set_description(original.description().map(str::to_owned));
        Ok(Some(picture))
    })
}

fn check_adjust_parameters(quality: u8, resize: Option<&CoverResize>) -> Result<(), String> {
    if quality > 100 {
        return Err("JPEG quality must be between 0 and 100".into());
    }
    if let Some(options) = resize {
        if options.pixels == 0 || options.pixels > 32768 {
            return Err("Cover size must be between 1 and 32768 pixels".into());
        }
    }
    Ok(())
}

/// The image file Import Cover uses for `path`. A `*` or `?` in the file
/// name (`*.jpg`, `cover?.png`) selects the first matching file in that
/// folder, in natural, case-insensitive name order, that the cover import
/// accepts (JPEG, PNG, GIF, BMP, TIFF or WebP). Other names are used as they
/// are.
pub fn resolve_cover_source(path: &Path) -> Result<PathBuf, String> {
    let wildcard = |text: &str| text.contains(['*', '?']);
    let Some(pattern) = path.file_name().map(|name| name.to_string_lossy()) else {
        return Ok(path.to_path_buf());
    };
    if !wildcard(&pattern) {
        return Ok(path.to_path_buf());
    }
    let folder = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if wildcard(&folder.to_string_lossy()) {
        return Err("Wildcards are only supported in the image file name".into());
    }
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let mut names: Vec<(String, PathBuf)> = fs::read_dir(folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| !kind.is_dir()))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let lower: Vec<char> = name.to_lowercase().chars().collect();
            wildcard_match(&pattern, &lower).then(|| (name.to_lowercase(), entry.path()))
        })
        .collect();
    names.sort_by(|(a, a_path), (b, b_path)| {
        crate::export::natural_compare(a, b).then_with(|| a_path.cmp(b_path))
    });
    names
        .into_iter()
        .map(|(_, path)| path)
        .find(|path| importable_image(path))
        .ok_or_else(|| format!("No image matches {}", path.display()))
}

/// `*` matches any run of characters and `?` one character.
fn wildcard_match(pattern: &[char], name: &[char]) -> bool {
    let (mut p, mut n) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, n));
            p += 1;
        } else if let Some((star_p, star_n)) = star {
            p = star_p + 1;
            n = star_n + 1;
            star = Some((star_p, star_n + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

/// A file the cover import accepts. The first bytes must look like an image
/// before the whole file is read and checked the way the import checks it.
fn importable_image(path: &Path) -> bool {
    use std::io::Read;
    let mut header = [0u8; 64];
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut length = 0;
    while length < header.len() {
        match file.read(&mut header[length..]) {
            Ok(0) | Err(_) => break,
            Ok(read) => length += read,
        }
    }
    image::guess_format(&header[..length]).is_ok()
        && fs::read(path).is_ok_and(|data| picture_from_bytes(data).is_ok())
}

/// Adjust Cover applied to one image outside a file (Manage Covers). `None`
/// means the image keeps its size and format and stays as it is.
pub fn adjust_cover_image(
    data: &[u8],
    format: CoverImageFormat,
    quality: u8,
    resize: Option<&CoverResize>,
) -> Result<Option<Vec<u8>>, String> {
    check_adjust_parameters(quality, resize)?;
    let mime = picture_from_bytes(data.to_vec())?.mime_type().cloned();
    adjust_image(data, mime.as_ref(), format, quality, resize)
}

fn adjust_image(
    data: &[u8],
    mime: Option<&lofty::picture::MimeType>,
    format: CoverImageFormat,
    quality: u8,
    resize: Option<&CoverResize>,
) -> Result<Option<Vec<u8>>, String> {
    let mut image = image::load_from_memory(data).map_err(|e| e.to_string())?;
    let size = (image.width(), image.height());
    if let Some(options) = resize {
        image = resize_image(image, options)?;
    }
    let original_format = match mime {
        Some(lofty::picture::MimeType::Jpeg) => Some(ImageFormat::Jpeg),
        Some(lofty::picture::MimeType::Png) => Some(ImageFormat::Png),
        Some(lofty::picture::MimeType::Gif) => Some(ImageFormat::Gif),
        Some(lofty::picture::MimeType::Bmp) => Some(ImageFormat::Bmp),
        Some(lofty::picture::MimeType::Unknown(mime)) if mime == "image/webp" => {
            Some(ImageFormat::WebP)
        }
        _ => image::guess_format(data).ok(),
    };
    let output_format = match format {
        CoverImageFormat::Jpeg => ImageFormat::Jpeg,
        CoverImageFormat::Png => ImageFormat::Png,
        CoverImageFormat::Original => original_format
            .filter(|f| {
                matches!(
                    f,
                    ImageFormat::Jpeg
                        | ImageFormat::Png
                        | ImageFormat::Gif
                        | ImageFormat::Bmp
                        | ImageFormat::WebP
                )
            })
            .ok_or("The original cover format cannot be written; select JPEG or PNG")?,
    };
    if (image.width(), image.height()) == size && original_format == Some(output_format) {
        return Ok(None);
    }
    let mut encoded = Cursor::new(Vec::new());
    if output_format == ImageFormat::Jpeg {
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, quality)
            .encode_image(&image.to_rgb8())
            .map_err(|e| e.to_string())?;
    } else if output_format == ImageFormat::Png {
        image
            .write_to(&mut encoded, output_format)
            .map_err(|e| e.to_string())?;
    } else {
        // GIF, BMP and WebP encoders take 8-bit RGBA.
        DynamicImage::ImageRgba8(image.to_rgba8())
            .write_to(&mut encoded, output_format)
            .map_err(|e| e.to_string())?;
    }
    Ok(Some(encoded.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{CoverScaleMode, CoverSizeMode};
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

    #[test]
    fn wildcards_match_whole_names_case_insensitively() {
        let matches = |pattern: &str, name: &str| {
            let chars = |text: &str| text.to_lowercase().chars().collect::<Vec<_>>();
            wildcard_match(&chars(pattern), &chars(name))
        };
        assert!(matches("*.jpg", "Front.JPG"));
        assert!(matches("cover?.png", "cover2.png"));
        assert!(matches("*front*", "Album front side.webp"));
        assert!(matches("*", "anything"));
        assert!(!matches("*.jpg", "cover.jpeg"));
        assert!(!matches("cover?.png", "cover.png"));
        assert!(!matches("*.jpg", "cover.jpg.txt"));
    }

    #[test]
    fn wildcard_cover_source_is_the_first_image_in_natural_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut png = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(1, 1))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        for name in ["scan10.png", "scan2.png", "Scan3.png"] {
            fs::write(dir.path().join(name), &png).unwrap();
        }
        // Not an image despite its name, so it is skipped, and so is an
        // image format the cover import does not take (QOI).
        fs::write(dir.path().join("scan1.png"), b"text").unwrap();
        fs::write(
            dir.path().join("scan01.png"),
            b"qoif\0\0\0\x01\0\0\0\x01\x03\0",
        )
        .unwrap();
        fs::create_dir(dir.path().join("scan0.png")).unwrap();
        assert_eq!(
            resolve_cover_source(&dir.path().join("scan*.png")).unwrap(),
            dir.path().join("scan2.png")
        );
        assert_eq!(
            resolve_cover_source(&dir.path().join("SCAN?.PNG")).unwrap(),
            dir.path().join("scan2.png")
        );
        // A name without wildcards is used as given, even when missing.
        let missing = dir.path().join("missing.jpg");
        assert_eq!(resolve_cover_source(&missing).unwrap(), missing);
        assert!(resolve_cover_source(&dir.path().join("*.gif"))
            .unwrap_err()
            .contains("No image matches"));
        assert!(resolve_cover_source(&dir.path().join("*/*.png"))
            .unwrap_err()
            .contains("only supported in the image file name"));
    }

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        let audio = vec![0u8; 8820];
        let mut data = b"RIFF".to_vec();
        data.extend((36 + audio.len() as u32).to_le_bytes());
        data.extend(b"WAVEfmt ");
        data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(44100u32.to_le_bytes());
        data.extend(88200u32.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(16u16.to_le_bytes());
        data.extend(b"data");
        data.extend((audio.len() as u32).to_le_bytes());
        data.extend(audio);
        fs::write(&path, data).unwrap();
        let image = dir.path().join("cover.png");
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(400, 200))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        fs::write(&image, bytes.into_inner()).unwrap();
        add_cover(&path, &image).unwrap();
        (dir, path, image)
    }

    #[test]
    fn cover_property_changes_and_bulk_removal_preserve_metadata_and_audio() {
        let (_dir, path, image) = fixture();
        add_cover(&path, &image).unwrap();
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["Keep".into()]);
        write_fields(&path, &fields).unwrap();
        let entry =
            set_cover_properties_summary(&path, Some("CoverBack"), None, Some("CoverFront"))
                .unwrap();
        assert_eq!(entry.covers.len(), 2);
        assert!(entry
            .covers
            .iter()
            .all(|c| c.picture_type == "CoverBack" && c.data_base64.is_empty()));
        let before = fs::read(&path).unwrap();
        assert!(set_cover_properties_summary(&path, None, Some("Same description"), None).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        let entry = remove_all_covers_summary(&path).unwrap();
        assert!(entry.covers.is_empty());
        assert_eq!(entry.fields["TITLE"], ["Keep"]);
        assert_eq!(entry.duration, 0.1);
    }

    #[test]
    fn proportional_square_and_crop_resizes_apply_actual_pixels() {
        for (mode, width, height) in [
            (CoverScaleMode::Proportional, 100, 50),
            (CoverScaleMode::Square, 100, 100),
            (CoverScaleMode::CropSquare, 100, 100),
        ] {
            let (_dir, path, _) = fixture();
            let resize = CoverResize {
                pixels: 100,
                size_mode: CoverSizeMode::Maximum,
                scale_mode: mode,
            };
            adjust_covers_summary(&path, CoverImageFormat::Jpeg, 85, Some(&resize)).unwrap();
            let entry = read_entry(&path).unwrap();
            assert_eq!(entry.covers[0].mime, "image/jpeg");
            let image =
                image::load_from_memory(&STANDARD.decode(&entry.covers[0].data_base64).unwrap())
                    .unwrap();
            assert_eq!((image.width(), image.height()), (width, height));
        }
    }

    #[test]
    fn maximum_size_never_enlarges_and_minimum_size_matches_the_smaller_side() {
        let image = || DynamicImage::ImageRgb8(RgbImage::new(400, 200));
        let size = |pixels, size_mode, scale_mode| {
            let resized = resize_image(
                image(),
                &CoverResize {
                    pixels,
                    size_mode,
                    scale_mode,
                },
            )
            .unwrap();
            (resized.width(), resized.height())
        };
        use CoverScaleMode::{CropSquare, Proportional, Square};
        use CoverSizeMode::{Maximum, Minimum};
        for mode in [Proportional, Square, CropSquare] {
            assert_eq!(size(500, Maximum, mode), (400, 200), "{mode:?}");
            assert_eq!(size(400, Maximum, mode), (400, 200), "{mode:?}");
        }
        assert_eq!(size(200, Maximum, Proportional), (200, 100));
        assert_eq!(size(200, Maximum, Square), (200, 200));
        assert_eq!(size(200, Maximum, CropSquare), (200, 200));
        assert_eq!(size(100, Minimum, Proportional), (200, 100));
        assert_eq!(size(300, Minimum, Proportional), (600, 300));
        assert_eq!(size(200, Minimum, Proportional), (400, 200));
        assert_eq!(size(200, Minimum, CropSquare), (200, 200));
        assert_eq!(size(300, Minimum, Square), (300, 300));
    }

    #[test]
    fn covers_that_keep_size_and_format_are_not_re_encoded() {
        let (_dir, path, image) = fixture();
        let original = fs::read(&image).unwrap();
        let resize = CoverResize {
            pixels: 800,
            size_mode: CoverSizeMode::Maximum,
            scale_mode: CoverScaleMode::CropSquare,
        };
        for format in [CoverImageFormat::Original, CoverImageFormat::Png] {
            adjust_covers_summary(&path, format, 85, Some(&resize)).unwrap();
            let entry = read_entry(&path).unwrap();
            assert_eq!(
                STANDARD.decode(&entry.covers[0].data_base64).unwrap(),
                original
            );
        }
        // A new format re-encodes at the same size.
        adjust_covers_summary(&path, CoverImageFormat::Jpeg, 85, Some(&resize)).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.covers[0].mime, "image/jpeg");
        assert_eq!(
            (entry.covers[0].width, entry.covers[0].height),
            (Some(400), Some(200))
        );
    }

    #[test]
    fn original_format_resizes_gif_covers_as_gif() {
        let (dir, path, _) = fixture();
        remove_all_covers_summary(&path).unwrap();
        let gif = dir.path().join("cover.gif");
        DynamicImage::ImageRgb8(RgbImage::new(300, 300))
            .save_with_format(&gif, ImageFormat::Gif)
            .unwrap();
        add_cover(&path, &gif).unwrap();
        let resize = CoverResize {
            pixels: 100,
            size_mode: CoverSizeMode::Maximum,
            scale_mode: CoverScaleMode::Proportional,
        };
        adjust_covers_summary(&path, CoverImageFormat::Original, 85, Some(&resize)).unwrap();
        let entry = read_entry(&path).unwrap();
        assert_eq!(entry.covers[0].mime, "image/gif");
        let bytes = STANDARD.decode(&entry.covers[0].data_base64).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Gif);
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (100, 100));
    }

    #[test]
    fn single_images_adjust_like_the_action_and_keep_bmp_and_webp() {
        let encode = |format| {
            let mut bytes = Cursor::new(Vec::new());
            DynamicImage::ImageRgb8(RgbImage::new(400, 200))
                .write_to(&mut bytes, format)
                .unwrap();
            bytes.into_inner()
        };
        let max = |pixels| CoverResize {
            pixels,
            size_mode: CoverSizeMode::Maximum,
            scale_mode: CoverScaleMode::Proportional,
        };
        for format in [ImageFormat::Png, ImageFormat::Bmp, ImageFormat::WebP] {
            let data = encode(format);
            // Within the size and in its own format: nothing to do.
            assert_eq!(
                adjust_cover_image(&data, CoverImageFormat::Original, 90, Some(&max(800))).unwrap(),
                None,
                "{format:?}"
            );
            let resized =
                adjust_cover_image(&data, CoverImageFormat::Original, 90, Some(&max(100)))
                    .unwrap()
                    .unwrap();
            assert_eq!(image::guess_format(&resized).unwrap(), format);
            let image = image::load_from_memory(&resized).unwrap();
            assert_eq!((image.width(), image.height()), (100, 50), "{format:?}");
        }
        let jpeg = adjust_cover_image(&encode(ImageFormat::Png), CoverImageFormat::Jpeg, 80, None)
            .unwrap()
            .unwrap();
        assert_eq!(image::guess_format(&jpeg).unwrap(), ImageFormat::Jpeg);
        assert!(
            adjust_cover_image(&encode(ImageFormat::Png), CoverImageFormat::Jpeg, 101, None)
                .is_err()
        );
        assert!(adjust_cover_image(b"not an image", CoverImageFormat::Png, 80, None).is_err());
    }

    #[test]
    fn invalid_cover_parameters_leave_the_original_unchanged() {
        let (_dir, path, _) = fixture();
        let before = fs::read(&path).unwrap();
        assert!(set_cover_properties_summary(&path, Some("Unknown"), None, None).is_err());
        assert!(adjust_covers_summary(&path, CoverImageFormat::Jpeg, 101, None).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }
}
