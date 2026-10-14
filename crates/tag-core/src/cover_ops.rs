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
        return read_entry_summary_with_options(path, &[], options);
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
                &[],
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
    if matches!(options.scale_mode, CoverScaleMode::Proportional)
        && match options.size_mode {
            CoverSizeMode::Maximum => dimension <= options.pixels,
            CoverSizeMode::Minimum => dimension >= options.pixels,
        }
    {
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
    if quality > 100 {
        return Err("JPEG quality must be between 0 and 100".into());
    }
    if let Some(options) = resize {
        if options.pixels == 0 || options.pixels > 32768 {
            return Err("Cover size must be between 1 and 32768 pixels".into());
        }
    }
    transform_covers(path, options, |original| {
        let mut image = image::load_from_memory(original.data()).map_err(|e| e.to_string())?;
        if let Some(options) = resize {
            image = resize_image(image, options)?;
        }
        let output_format=match format {
            CoverImageFormat::Jpeg=>ImageFormat::Jpeg,CoverImageFormat::Png=>ImageFormat::Png,
            CoverImageFormat::Original=>match original.mime_type(){Some(lofty::picture::MimeType::Jpeg)=>ImageFormat::Jpeg,Some(lofty::picture::MimeType::Png)=>ImageFormat::Png,_=>return Err("Original-format cover adjustment supports JPEG and PNG; select an output format".into())},
        };
        let mut encoded = Cursor::new(Vec::new());
        if output_format == ImageFormat::Jpeg {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, quality)
                .encode_image(&image.to_rgb8())
                .map_err(|e| e.to_string())?;
        } else {
            image
                .write_to(&mut encoded, output_format)
                .map_err(|e| e.to_string())?;
        }
        encoded.set_position(0);
        let mut picture = Picture::from_reader(&mut encoded).map_err(|e| e.to_string())?;
        picture.set_pic_type(original.pic_type());
        picture.set_description(original.description().map(str::to_owned));
        Ok(Some(picture))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{CoverScaleMode, CoverSizeMode};
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

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
    fn invalid_cover_parameters_leave_the_original_unchanged() {
        let (_dir, path, _) = fixture();
        let before = fs::read(&path).unwrap();
        assert!(set_cover_properties_summary(&path, Some("Unknown"), None, None).is_err());
        assert!(adjust_covers_summary(&path, CoverImageFormat::Jpeg, 101, None).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }
}
