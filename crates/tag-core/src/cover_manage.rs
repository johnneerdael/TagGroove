//! Whole-list cover edits behind Manage Covers and the cover window: reorder,
//! retype, describe, replace and remove covers, and embed covers that only
//! some of the selected files contain.
use super::*;
use std::hash::{DefaultHasher, Hasher};
use std::io::Cursor;

/// Content identity of embedded image bytes (stable for one build).
pub fn cover_digest(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(data);
    format!("{:016x}-{}", hasher.finish(), data.len())
}

/// One distinct embedded cover: image content, picture type and description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverRef {
    pub digest: String,
    pub picture_type: String,
    pub description: String,
}
impl CoverRef {
    pub fn of(cover: &CoverInfo) -> Self {
        Self {
            digest: cover.digest.clone(),
            picture_type: cover.picture_type.clone(),
            description: cover.description.clone(),
        }
    }
    fn matches(&self, picture: &Picture, digest: &str) -> bool {
        self.digest == digest
            && self.picture_type == format!("{:?}", picture.pic_type())
            && self.description == picture.description().unwrap_or_default()
    }
}

/// One entry of the desired cover list, in display order. Covers of a file
/// that no entry refers to are removed.
#[derive(Debug, Clone, Default)]
pub struct CoverChange {
    /// The existing cover this entry keeps; `None` embeds `image` as a new cover.
    pub original: Option<CoverRef>,
    /// Replacement image bytes (always present for new covers).
    pub image: Option<Vec<u8>>,
    pub picture_type: String,
    pub description: String,
    /// Embed in every file, including files that do not contain `original`.
    pub all_files: bool,
    /// Which copy of identical covers (same image, type and description)
    /// this entry keeps; `None` keeps every copy.
    pub occurrence: Option<usize>,
}

/// Build a picture from image bytes. Lofty does not sniff WebP, which is
/// stored with its MIME type like Mp3tag does.
pub fn picture_from_bytes(data: Vec<u8>) -> Result<Picture, String> {
    if data.len() >= 8 {
        if let Ok(picture) = Picture::from_reader(&mut Cursor::new(&data)) {
            return Ok(picture);
        }
    }
    match image::guess_format(&data) {
        Ok(image::ImageFormat::WebP) => Ok(Picture::unchecked(data)
            .mime_type(lofty::picture::MimeType::Unknown("image/webp".into()))
            .build()),
        _ => Err("The image format is not supported (use JPEG, PNG, GIF, BMP or WebP)".into()),
    }
}

fn current_pictures(file: &TaggedFile, options: &TagOptions) -> Vec<Picture> {
    if tag_policy::policy(file.file_type()).is_some() {
        return tag_policy::displayed_pictures(file, options);
    }
    ordered_tags(file)
        .into_iter()
        .flat_map(|tag| tag.pictures().to_vec())
        .collect()
}

/// Image bytes of `cover` in `path`, if the file contains it.
pub fn find_cover_data(
    path: &Path,
    cover: &CoverRef,
    options: &TagOptions,
) -> Result<Option<Vec<u8>>, String> {
    read_description(path, true, &[], options)?
        .covers
        .into_iter()
        .find(|info| &CoverRef::of(info) == cover)
        .map(|info| STANDARD.decode(info.data_base64).map_err(|e| e.to_string()))
        .transpose()
}

/// ASF, DSD and Matroska artwork is not read by lofty; rebuild the pictures
/// from a full read.
fn described_pictures(path: &Path, options: &TagOptions) -> Result<Vec<Picture>, String> {
    read_description(path, true, &[], options)?
        .covers
        .into_iter()
        .map(|info| {
            let mut picture = Picture::unchecked(
                STANDARD
                    .decode(info.data_base64)
                    .map_err(|e| e.to_string())?,
            )
            .mime_type(lofty::picture::MimeType::from_str(&info.mime))
            .pic_type(picture_type_from_name(&info.picture_type)?)
            .build();
            picture.set_description((!info.description.is_empty()).then_some(info.description));
            Ok(picture)
        })
        .collect()
}

fn planned_pictures(current: &[Picture], changes: &[CoverChange]) -> Result<Vec<Picture>, String> {
    let digests: Vec<String> = current.iter().map(|p| cover_digest(p.data())).collect();
    let mut planned = Vec::new();
    for change in changes {
        let kind = picture_type_from_name(&change.picture_type)?;
        let mut matching: Vec<&Picture> = match &change.original {
            Some(original) => current
                .iter()
                .zip(&digests)
                .filter(|(picture, digest)| original.matches(picture, digest))
                .map(|(picture, _)| picture)
                .collect(),
            None => Vec::new(),
        };
        if let Some(occurrence) = change.occurrence {
            matching = matching.into_iter().skip(occurrence).take(1).collect();
        }
        let count = if change.original.is_none() || change.all_files {
            matching.len().max(1)
        } else {
            matching.len()
        };
        for index in 0..count {
            let mut picture = match (&change.image, matching.get(index)) {
                (Some(data), _) => picture_from_bytes(data.clone())?,
                (None, Some(existing)) => (*existing).clone(),
                (None, None) => {
                    return Err(
                        "A cover to embed is no longer available; reopen Manage covers".into(),
                    )
                }
            };
            picture.set_pic_type(kind);
            picture.set_description(
                (!change.description.is_empty()).then(|| change.description.clone()),
            );
            planned.push(picture);
        }
    }
    Ok(planned)
}

fn identity(picture: &Picture) -> (String, PictureType, String) {
    (
        cover_digest(picture.data()),
        picture.pic_type(),
        picture.description().unwrap_or_default().to_owned(),
    )
}

/// MP4 artwork has neither picture types nor descriptions.
fn normalize_for(file_type: FileType, pictures: &mut [Picture]) {
    if file_type == FileType::Mp4 {
        for picture in pictures {
            picture.set_pic_type(PictureType::Other);
            picture.set_description(None);
        }
    }
}

/// Replace the displayed cover list of `path` with `changes`. Files whose
/// covers already match are left untouched.
pub fn apply_cover_changes_with_options(
    path: &Path,
    changes: &[CoverChange],
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let asf = asf::sniff(path);
    let container = container_formats::detect(path);
    let (current, file_type) = if asf || container.is_some() {
        (
            described_pictures(path, options)?,
            FileType::Custom("container"),
        )
    } else {
        let file = load(path)?;
        (current_pictures(&file, options), file.file_type())
    };
    let mut planned = planned_pictures(&current, changes)?;
    normalize_for(file_type, &mut planned);
    let unchanged = current.len() == planned.len()
        && current
            .iter()
            .zip(&planned)
            .all(|(a, b)| identity(a) == identity(b));
    if unchanged {
        return read_entry_summary_with_options(path, &options.mappings, options);
    }
    if asf {
        return asf::change_pictures(
            path,
            false,
            options,
            |pictures| {
                *pictures = planned;
                Ok(())
            },
            DUPLICATE_COVERS,
        );
    }
    if let Some(kind) = container {
        return container_formats::set_covers(path, kind, options, planned, DUPLICATE_COVERS);
    }
    write_pictures(path, planned, options)
}

/// Append one cover built from image bytes to the displayed tag.
pub fn add_cover_bytes_with_options(
    path: &Path,
    data: Vec<u8>,
    label: &str,
    picture_type: Option<&str>,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    let mut picture = picture_from_bytes(data)?;
    picture.set_pic_type(
        picture_type
            .map(picture_type_from_name)
            .transpose()?
            .unwrap_or(PictureType::CoverFront),
    );
    add_picture_result(path, picture, label, false, options)
}

const DUPLICATE_COVERS: &str =
    "The tag format cannot store these covers; give covers of the same type distinct descriptions";

fn write_pictures(
    path: &Path,
    pictures: Vec<Picture>,
    options: &TagOptions,
) -> Result<AudioEntry, String> {
    staged_mutation(path, false, options, |staging, mut file, staged_path| {
        if tag_policy::policy(file.file_type()).is_some() {
            let written = tag_policy::write(
                staging,
                &file,
                &options.mappings,
                options,
                tag_policy::Change::Pictures(pictures.clone()),
            )?;
            tag_policy::verify_pictures(
                staged_path,
                options,
                written,
                &pictures,
                DUPLICATE_COVERS,
            )?;
        } else {
            let primary = file.primary_tag_type();
            let file_type = file.file_type();
            let others: Vec<TagType> = file
                .tags()
                .iter()
                .filter(|tag| tag.tag_type() != primary && !tag.pictures().is_empty())
                .map(Tag::tag_type)
                .collect();
            for kind in others {
                if !file.tag_support(kind).is_writable() {
                    return Err("Cover tag is read-only".into());
                }
                let tag = file.tag_mut(kind).ok_or("Cover tag disappeared")?;
                while !tag.pictures().is_empty() {
                    tag.remove_picture(0);
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
            if file.primary_tag().is_none() {
                file.insert_tag(Tag::new(primary));
            }
            let tag = file
                .primary_tag_mut()
                .ok_or("The audio format has no writable tag")?;
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
            for picture in &pictures {
                tag.push_picture(picture.clone());
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
        let written = read_entry_summary_with_options(staged_path, &[], options)?.covers;
        let expected: Vec<_> = pictures.iter().map(identity).collect();
        let matches = written.len() == expected.len()
            && written
                .iter()
                .zip(&expected)
                .all(|(cover, (digest, kind, description))| {
                    &cover.digest == digest
                        && cover.picture_type == format!("{kind:?}")
                        && &cover.description == description
                });
        if matches {
            Ok(())
        } else {
            Err(DUPLICATE_COVERS.into())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbImage};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(width, height))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn webp() -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(3, 2))
            .write_to(&mut bytes, ImageFormat::WebP)
            .unwrap();
        bytes.into_inner()
    }

    fn wav(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
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
        path
    }

    fn keep(cover: &CoverInfo) -> CoverChange {
        CoverChange {
            original: Some(CoverRef::of(cover)),
            picture_type: cover.picture_type.clone(),
            description: cover.description.clone(),
            ..Default::default()
        }
    }

    fn covers(path: &Path) -> Vec<CoverInfo> {
        read_entry_summary(path).unwrap().covers
    }

    #[test]
    fn digest_identifies_content() {
        assert_eq!(cover_digest(b"abc"), cover_digest(b"abc"));
        assert_ne!(cover_digest(b"abc"), cover_digest(b"abd"));
        assert!(cover_digest(b"abc").ends_with("-3"));
    }

    #[test]
    fn reorders_retypes_describes_and_removes_covers() {
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path(), "song.wav");
        add_cover_bytes_with_options(&path, png(4, 4), "a.png", None, &TagOptions::default())
            .unwrap();
        add_cover_bytes_with_options(
            &path,
            png(8, 2),
            "b.png",
            Some("CoverBack"),
            &TagOptions::default(),
        )
        .unwrap();
        let before = covers(&path);
        assert_eq!(before.len(), 2);
        assert_eq!(before[0].width, Some(4));
        assert_eq!(before[1].picture_type, "CoverBack");

        // Move the back cover first and describe it; drop nothing.
        let mut back = keep(&before[1]);
        back.description = "Rear".into();
        let entry = apply_cover_changes_with_options(
            &path,
            &[back, keep(&before[0])],
            &TagOptions::default(),
        )
        .unwrap();
        assert_eq!(entry.covers.len(), 2);
        assert_eq!(entry.covers[0].digest, before[1].digest);
        assert_eq!(entry.covers[0].description, "Rear");
        assert_eq!(entry.covers[1].digest, before[0].digest);

        // Retype the remaining front cover and remove the back cover.
        let now = covers(&path);
        let mut media = keep(&now[1]);
        media.picture_type = "Media".into();
        let entry =
            apply_cover_changes_with_options(&path, &[media], &TagOptions::default()).unwrap();
        assert_eq!(entry.covers.len(), 1);
        assert_eq!(entry.covers[0].picture_type, "Media");
        assert_eq!(entry.duration, 0.1);
    }

    #[test]
    fn unchanged_plans_do_not_rewrite_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path(), "song.wav");
        add_cover_bytes_with_options(&path, png(4, 4), "a.png", None, &TagOptions::default())
            .unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        let current = covers(&path);
        std::thread::sleep(std::time::Duration::from_millis(20));
        apply_cover_changes_with_options(&path, &[keep(&current[0])], &TagOptions::default())
            .unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn replace_keeps_position_type_and_description() {
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path(), "song.wav");
        add_cover_bytes_with_options(&path, png(4, 4), "a.png", None, &TagOptions::default())
            .unwrap();
        add_cover_bytes_with_options(
            &path,
            png(5, 5),
            "b.png",
            Some("Artist"),
            &TagOptions::default(),
        )
        .unwrap();
        let before = covers(&path);
        let mut replaced = keep(&before[0]);
        replaced.image = Some(png(9, 3));
        let entry = apply_cover_changes_with_options(
            &path,
            &[replaced, keep(&before[1])],
            &TagOptions::default(),
        )
        .unwrap();
        assert_eq!(entry.covers[0].picture_type, "CoverFront");
        assert_eq!(
            (entry.covers[0].width, entry.covers[0].height),
            (Some(9), Some(3))
        );
        assert_eq!(entry.covers[1].digest, before[1].digest);
    }

    #[test]
    fn apply_to_all_embeds_covers_missing_from_some_files() {
        let dir = tempfile::tempdir().unwrap();
        let first = wav(dir.path(), "one.wav");
        let second = wav(dir.path(), "two.wav");
        add_cover_bytes_with_options(&first, png(6, 6), "a.png", None, &TagOptions::default())
            .unwrap();
        let shared = covers(&first)[0].clone();
        let reference = CoverRef::of(&shared);
        let data = find_cover_data(&first, &reference, &TagOptions::default())
            .unwrap()
            .unwrap();
        assert_eq!(cover_digest(&data), shared.digest);
        // Without the image bytes a missing cover cannot be embedded.
        let mut change = keep(&shared);
        change.all_files = true;
        assert!(apply_cover_changes_with_options(
            &second,
            std::slice::from_ref(&change),
            &TagOptions::default()
        )
        .is_err());
        change.image = Some(data);
        for path in [&first, &second] {
            let entry = apply_cover_changes_with_options(
                path,
                std::slice::from_ref(&change),
                &TagOptions::default(),
            )
            .unwrap();
            assert_eq!(entry.covers.len(), 1);
            assert_eq!(entry.covers[0].digest, shared.digest);
        }
        // Without "all files", a missing cover stays absent.
        let third = wav(dir.path(), "three.wav");
        let entry =
            apply_cover_changes_with_options(&third, &[keep(&shared)], &TagOptions::default())
                .unwrap();
        assert!(entry.covers.is_empty());
    }

    #[test]
    fn new_covers_accept_webp_and_reject_other_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = wav(dir.path(), "song.wav");
        let change = CoverChange {
            image: Some(webp()),
            picture_type: "CoverFront".into(),
            ..Default::default()
        };
        let entry =
            apply_cover_changes_with_options(&path, &[change], &TagOptions::default()).unwrap();
        assert_eq!(entry.covers[0].mime, "image/webp");
        assert!(picture_from_bytes(b"not an image at all".to_vec()).is_err());
        let duplicate = CoverChange {
            image: Some(png(2, 2)),
            picture_type: "CoverFront".into(),
            ..Default::default()
        };
        let before = fs::read(&path).unwrap();
        assert!(apply_cover_changes_with_options(
            &path,
            &[duplicate.clone(), duplicate],
            &TagOptions::default()
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn mp3_policy_formats_follow_the_plan() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mp3");
        // A bare MPEG frame stream is enough for lofty to probe the file.
        let mut data = Vec::new();
        for _ in 0..20 {
            data.extend([0xFF, 0xFB, 0x90, 0x64]);
            data.extend(vec![0u8; 413]);
        }
        fs::write(&path, data).unwrap();
        let options = TagOptions::default();
        add_cover_bytes_with_options(&path, png(4, 4), "a.png", None, &options).unwrap();
        add_cover_bytes_with_options(&path, png(3, 3), "b.png", Some("CoverBack"), &options)
            .unwrap();
        let before = read_entry_summary_with_options(&path, &[], &options)
            .unwrap()
            .covers;
        let entry = apply_cover_changes_with_options(
            &path,
            &[keep(&before[1]), keep(&before[0])],
            &options,
        )
        .unwrap();
        assert_eq!(entry.covers[0].picture_type, "CoverBack");
        assert_eq!(entry.covers[1].picture_type, "CoverFront");
    }
}
