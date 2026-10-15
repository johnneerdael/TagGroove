//! Manage Covers and the cover window context menu: aggregated cover sets of
//! the selection, whole-list cover plans, extraction, and clipboard images.
//! Every mutation is journaled for undo like the other tag commands.
use super::{auxiliary_input, history, io, mutate_document, settings, whole_file_batch};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tag_core::{CoverChange, CoverRef, FileError, ScanResult};

/// Image bytes from files or the clipboard, kept for later plan steps so the
/// webview never sends large images back. Oldest entries are dropped first.
static STAGED: Mutex<VecDeque<(String, Vec<u8>)>> = Mutex::new(VecDeque::new());
const STAGED_LIMIT: usize = 16;
const STAGED_BYTES: usize = 128 * 1024 * 1024;
const IMAGE_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Serialize)]
pub struct StagedImage {
    token: String,
    mime: String,
    size: u64,
    width: Option<u32>,
    height: Option<u32>,
    data_base64: String,
    label: String,
}

fn stage(data: Vec<u8>, label: &str) -> Result<StagedImage, String> {
    let picture = tag_core::picture_from_bytes(data.clone())?;
    let mime = picture
        .mime_type()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| "application/octet-stream".into());
    let (width, height) = image::ImageReader::new(std::io::Cursor::new(&data))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .map_or((None, None), |(w, h)| (Some(w), Some(h)));
    let token = uuid::Uuid::new_v4().to_string();
    let mut staged = STAGED.lock().map_err(|_| "Staged image lock failed")?;
    staged.push_back((token.clone(), data.clone()));
    while staged.len() > STAGED_LIMIT
        || staged.iter().map(|(_, bytes)| bytes.len()).sum::<usize>() > STAGED_BYTES
    {
        staged.pop_front();
    }
    Ok(StagedImage {
        token,
        mime,
        size: data.len() as u64,
        width,
        height,
        data_base64: STANDARD.encode(&data),
        label: label.into(),
    })
}

fn read_image(path: &Path) -> Result<Vec<u8>, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > IMAGE_LIMIT {
        return Err("The image is larger than 32 MB".into());
    }
    std::fs::read(path).map_err(|e| e.to_string())
}

/// Where a new or replacement image comes from.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    Path(String),
    Staged(String),
}
impl ImageSource {
    fn load(&self) -> Result<(Vec<u8>, String), String> {
        match self {
            Self::Path(path) => {
                let local = auxiliary_input(path)?;
                let label = local
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                Ok((read_image(&local)?, label))
            }
            Self::Staged(token) => STAGED
                .lock()
                .map_err(|_| "Staged image lock failed")?
                .iter()
                .find(|(t, _)| t == token)
                .map(|(_, bytes)| (bytes.clone(), "Pasted image".into()))
                .ok_or_else(|| "The image is no longer available; add it again".into()),
        }
    }
}

#[derive(Serialize)]
pub struct CoverSetItem {
    #[serde(flatten)]
    cover: CoverRef,
    mime: String,
    size: u64,
    width: Option<u32>,
    height: Option<u32>,
    /// Indexes into the requested paths of the files containing this cover.
    files: Vec<usize>,
    /// Position of the cover in the first containing file.
    index: usize,
    data_base64: String,
}

#[derive(Serialize)]
pub struct CoverSet {
    files: usize,
    covers: Vec<CoverSetItem>,
    errors: Vec<FileError>,
}

/// Distinct covers of the selection in first-seen order, with the files that
/// contain each one.
#[tauri::command]
pub async fn cover_set(paths: Vec<String>) -> Result<CoverSet, String> {
    io(move || Ok(collect_set(&paths))).await
}

fn collect_set(paths: &[String]) -> CoverSet {
    let mut set = CoverSet {
        files: paths.len(),
        covers: Vec::new(),
        errors: Vec::new(),
    };
    let mut first: HashMap<usize, (String, usize)> = HashMap::new();
    for (file, path) in paths.iter().enumerate() {
        let entry = match super::require_audio_handle(Path::new(path))
            .and_then(|_| settings::read_summary(Path::new(path)))
        {
            Ok(entry) => entry,
            Err(error) => {
                set.errors.push(FileError {
                    path: path.clone(),
                    error,
                });
                continue;
            }
        };
        for (index, cover) in entry.covers.iter().enumerate() {
            let reference = CoverRef::of(cover);
            match set.covers.iter_mut().find(|item| item.cover == reference) {
                Some(item) => {
                    if item.files.last() != Some(&file) {
                        item.files.push(file);
                    }
                }
                None => {
                    first.insert(set.covers.len(), (path.clone(), index));
                    set.covers.push(CoverSetItem {
                        cover: reference,
                        mime: cover.mime.clone(),
                        size: cover.size,
                        width: cover.width,
                        height: cover.height,
                        files: vec![file],
                        index,
                        data_base64: String::new(),
                    });
                }
            }
        }
    }
    // Image data is read once per file that first contains a cover.
    let mut full: HashMap<String, Vec<tag_core::CoverInfo>> = HashMap::new();
    for (position, item) in set.covers.iter_mut().enumerate() {
        let (path, index) = &first[&position];
        if !full.contains_key(path) {
            let covers = settings::read_full(Path::new(path))
                .map(|e| e.covers)
                .unwrap_or_default();
            full.insert(path.clone(), covers);
        }
        if let Some(cover) = full[path].get(*index) {
            item.data_base64 = cover.data_base64.clone();
        }
    }
    set
}

/// One entry of a desired cover list (see [`tag_core::CoverChange`]).
#[derive(Deserialize)]
pub struct PlanItem {
    original: Option<CoverRef>,
    image: Option<ImageSource>,
    picture_type: String,
    description: String,
    #[serde(default)]
    all_files: bool,
    #[serde(default)]
    occurrence: Option<usize>,
}

fn resolve_plan(paths: &[String], items: Vec<PlanItem>) -> Result<Vec<CoverChange>, String> {
    let options = settings::tag_options();
    items
        .into_iter()
        .map(|item| {
            let image =
                match (&item.image, &item.original) {
                    (Some(source), _) => Some(source.load()?.0),
                    // A cover embedded in every file needs its bytes for the files
                    // that lack it; take them from the first file that has it.
                    (None, Some(original)) if item.all_files => {
                        let mut found = None;
                        for path in paths {
                            if let Ok(Some(data)) =
                                tag_core::find_cover_data(Path::new(path), original, &options)
                            {
                                found = Some(data);
                                break;
                            }
                        }
                        Some(found.ok_or(
                            "A cover to embed is no longer available; reopen Manage covers",
                        )?)
                    }
                    _ => None,
                };
            if item.original.is_none() && image.is_none() {
                return Err("A new cover needs an image".to_string());
            }
            Ok(CoverChange {
                original: item.original,
                image,
                picture_type: item.picture_type,
                description: item.description,
                all_files: item.all_files,
                occurrence: item.occurrence,
            })
        })
        .collect()
}

fn history_label(label: &str) -> &str {
    match label {
        "Manage covers"
        | "Replace cover"
        | "Remove cover"
        | "Cut cover"
        | "Set cover type"
        | "Set cover description" => label,
        _ => "Edit covers",
    }
}

/// Replace the cover list of every path with `items`, in order.
#[tauri::command]
pub async fn apply_cover_plan(
    paths: Vec<String>,
    items: Vec<PlanItem>,
    label: String,
) -> Result<ScanResult, String> {
    io(move || {
        let changes = resolve_plan(&paths, items)?;
        history::record(history_label(&label), None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::apply_cover_changes_with_options(
                        p,
                        &changes,
                        &settings::tag_options(),
                    )
                })
            }))
        })
    })
    .await
}

/// Append one image as a cover of every path (Add cover, paste and drop).
#[tauri::command]
pub async fn add_cover_image(
    paths: Vec<String>,
    image: ImageSource,
    picture_type: Option<String>,
) -> Result<ScanResult, String> {
    io(move || {
        let (data, label) = image.load()?;
        history::record("Add cover", None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::add_cover_bytes_with_options(
                        p,
                        data.clone(),
                        &label,
                        picture_type.as_deref(),
                        &settings::tag_options(),
                    )
                })
            }))
        })
    })
    .await
}

/// Read an image file for preview and later use in a plan.
#[tauri::command]
pub async fn stage_cover_file(path: String) -> Result<StagedImage, String> {
    io(move || {
        let local = auxiliary_input(&path)?;
        let label = local
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        stage(read_image(&local)?, &label)
    })
    .await
}

/// Stage image bytes the webview received from a drop or paste event.
#[tauri::command]
pub async fn stage_cover_data(data_base64: String, label: String) -> Result<StagedImage, String> {
    io(move || {
        if data_base64.len() as u64 > IMAGE_LIMIT * 4 / 3 + 4 {
            return Err("The image is larger than 32 MB".into());
        }
        let data = STANDARD.decode(data_base64).map_err(|e| e.to_string())?;
        stage(data, &label)
    })
    .await
}

/// Settings of the Adjust Cover action, as the action editor sends them.
#[derive(Deserialize)]
pub struct AdjustSettings {
    format: tag_core::actions::CoverImageFormat,
    quality: u8,
    resize: Option<tag_core::actions::CoverResize>,
}

/// Where Manage Covers takes an image to adjust from.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustSource {
    /// A cover of an audio file, by its position in that file.
    Cover { path: String, index: usize },
    /// An image staged earlier in the dialog.
    Staged(String),
}

/// Manage Covers → Adjust: resize or convert one cover and stage the result.
/// An image that would stay as it is is staged unchanged.
#[tauri::command]
pub async fn stage_adjusted_cover(
    source: AdjustSource,
    settings: AdjustSettings,
) -> Result<StagedImage, String> {
    io(move || {
        let data = match source {
            AdjustSource::Cover { path, index } => cover_bytes(Path::new(&path), index)?.0,
            AdjustSource::Staged(token) => ImageSource::Staged(token).load()?.0,
        };
        let adjusted = tag_core::adjust_cover_image(
            &data,
            settings.format,
            settings.quality,
            settings.resize.as_ref(),
        )?;
        stage(adjusted.unwrap_or(data), "Adjusted cover")
    })
    .await
}

/// Add cover's start folder (Options → Directories → Cover art directory): the
/// option evaluated for the first selected file, relative to its folder; else
/// the file's folder when it holds an image, else the working folder.
#[tauri::command]
pub async fn cover_start_directory(path: String, template: String) -> Result<String, String> {
    let settings = super::evaluation_settings(None)?;
    io(move || start_directory(Path::new(&path), &template, &settings)).await
}

fn start_directory(
    audio: &Path,
    template: &str,
    settings: &crate::evaluation::EvaluationSettings,
) -> Result<String, String> {
    let folder = audio.parent().unwrap_or(Path::new("."));
    if !template.trim().is_empty() {
        let entry = settings::read_summary(audio)?;
        let technical = super::preview_technical_fields(&entry, 1);
        let value = settings.evaluate(template, &entry.fields, &technical, &entry.path, true)?;
        let candidate = folder.join(value.trim());
        if !value.trim().is_empty() && candidate.is_dir() {
            return Ok(candidate.to_string_lossy().into_owned());
        }
    }
    if !crate::list_behaviour::folder_images(folder).is_empty() {
        return Ok(folder.to_string_lossy().into_owned());
    }
    Ok(settings
        .working_directory
        .as_deref()
        .filter(|working| Path::new(working).is_dir())
        .map(str::to_owned)
        .unwrap_or_else(|| folder.to_string_lossy().into_owned()))
}

fn cover_bytes(path: &Path, index: usize) -> Result<(Vec<u8>, String), String> {
    super::require_audio_handle(path)?;
    let entry = settings::read_full(path)?;
    let cover = entry
        .covers
        .get(index)
        .ok_or_else(|| format!("Cover {} does not exist", index + 1))?;
    Ok((
        STANDARD
            .decode(&cover.data_base64)
            .map_err(|e| e.to_string())?,
        cover.mime.clone(),
    ))
}

pub fn extension_for(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tif",
        "image/webp" => "webp",
        _ => "jpg",
    }
}

/// Where Extract cover writes: `output` with the image's extension, relative
/// names resolved against the audio file's folder.
fn extract_target(audio: &Path, output: &str, mime: &str) -> Result<PathBuf, String> {
    let output = output.trim();
    if output.is_empty() || output.contains('\0') {
        return Err("Enter a file name for the cover".into());
    }
    let mut target = PathBuf::from(output);
    if target.is_relative() {
        target = audio
            .parent()
            .ok_or("The audio file has no folder")?
            .join(target);
    }
    let extension = extension_for(mime);
    let matches = target.extension().is_some_and(|e| {
        let e = e.to_string_lossy().to_ascii_lowercase();
        e == extension || (extension == "jpg" && e == "jpeg")
    });
    if !matches {
        let mut name = target.file_name().ok_or("Enter a file name")?.to_owned();
        name.push(format!(".{extension}"));
        target.set_file_name(name);
    }
    if target == audio {
        return Err("Choose a different file than the audio file".into());
    }
    Ok(target)
}

#[derive(Serialize)]
pub struct Extracted {
    path: String,
    exists: bool,
}

/// Save one embedded cover as an image file. Existing files are only
/// replaced with `overwrite`; otherwise `exists` reports the conflict.
#[tauri::command]
pub async fn extract_cover(
    path: String,
    index: usize,
    output: String,
    overwrite: bool,
) -> Result<Extracted, String> {
    io(move || {
        super::reject_provider_operation(std::slice::from_ref(&path), "Extract cover")?;
        let audio = Path::new(&path);
        let (data, mime) = cover_bytes(audio, index)?;
        let target = extract_target(audio, &output, &mime)?;
        let shown = target.to_string_lossy().into_owned();
        if target.exists() && !overwrite {
            return Ok(Extracted {
                path: shown,
                exists: true,
            });
        }
        if let Some(parent) = target.parent() {
            if !parent.is_dir() {
                return Err(format!("The folder {} does not exist", parent.display()));
            }
        }
        let mut staging =
            tempfile::NamedTempFile::new_in(target.parent().unwrap_or(Path::new(".")))
                .map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut staging, &data).map_err(|e| e.to_string())?;
        staging.persist(&target).map_err(|e| e.error.to_string())?;
        Ok(Extracted {
            path: shown,
            exists: false,
        })
    })
    .await
}

/// Put one embedded cover on the system clipboard as image data.
#[tauri::command]
pub async fn copy_cover_to_clipboard(path: String, index: usize) -> Result<(), String> {
    io(move || {
        let (data, _) = cover_bytes(Path::new(&path), index)?;
        let image = image::load_from_memory(&data)
            .map_err(|e| format!("The cover could not be decoded: {e}"))?
            .to_rgba8();
        clipboard::set_image(image.width(), image.height(), image.into_raw())
    })
    .await
}

/// Stage the clipboard image (or a copied image file) for Paste cover.
#[tauri::command]
pub async fn paste_cover_from_clipboard() -> Result<StagedImage, String> {
    io(move || match clipboard::get()? {
        clipboard::Content::File(path) => {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            stage(read_image(&path)?, &label)
        }
        clipboard::Content::Image(width, height, rgba) => {
            let image = image::RgbaImage::from_raw(width, height, rgba)
                .ok_or("The clipboard image is malformed")?;
            let mut png = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut png, image::ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            stage(png.into_inner(), "Pasted image")
        }
    })
    .await
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod clipboard {
    use std::borrow::Cow;
    use std::path::PathBuf;

    pub enum Content {
        File(PathBuf),
        Image(u32, u32, Vec<u8>),
    }

    /// One clipboard for the app's lifetime: on X11 and Wayland the owning
    /// object serves the copied data, so dropping it would empty the clipboard.
    fn with<T>(
        use_clipboard: impl FnOnce(&mut arboard::Clipboard) -> Result<T, String>,
    ) -> Result<T, String> {
        static CLIPBOARD: std::sync::Mutex<Option<arboard::Clipboard>> =
            std::sync::Mutex::new(None);
        let mut guard = CLIPBOARD
            .lock()
            .map_err(|_| "The clipboard is busy".to_string())?;
        if guard.is_none() {
            *guard = Some(
                arboard::Clipboard::new()
                    .map_err(|e| format!("The clipboard is unavailable: {e}"))?,
            );
        }
        use_clipboard(guard.as_mut().expect("clipboard was opened"))
    }

    pub fn set_image(width: u32, height: u32, rgba: Vec<u8>) -> Result<(), String> {
        with(|clipboard| {
            clipboard
                .set_image(arboard::ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(rgba),
                })
                .map_err(|e| format!("The cover could not be copied: {e}"))
        })
    }

    /// Copied image files keep their original encoding; Finder also puts the
    /// file icon on the clipboard, so files take precedence over image data.
    pub fn get() -> Result<Content, String> {
        with(read)
    }

    fn read(clipboard: &mut arboard::Clipboard) -> Result<Content, String> {
        if let Ok(files) = clipboard.get().file_list() {
            if let Some(file) = files.into_iter().find(|f| f.is_file()) {
                return Ok(Content::File(file));
            }
        }
        if let Ok(image) = clipboard.get_image() {
            return Ok(Content::Image(
                image.width as u32,
                image.height as u32,
                image.bytes.into_owned(),
            ));
        }
        if let Ok(text) = clipboard.get_text() {
            let text = text.trim();
            let path = PathBuf::from(text.strip_prefix("file://").unwrap_or(text));
            if path.is_absolute() && path.is_file() {
                return Ok(Content::File(path));
            }
        }
        Err("The clipboard does not contain an image".into())
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
mod clipboard {
    pub enum Content {
        File(std::path::PathBuf),
        Image(u32, u32, Vec<u8>),
    }
    pub fn set_image(_: u32, _: u32, _: Vec<u8>) -> Result<(), String> {
        Err("Copy cover is available in the desktop app".into())
    }
    pub fn get() -> Result<Content, String> {
        Err("Paste cover is available in the desktop app".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(RgbImage::new(width, height))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn wav(dir: &Path, name: &str) -> String {
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
        std::fs::write(&path, data).unwrap();
        dunce::canonicalize(&path)
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    fn run<T>(future: impl std::future::Future<Output = T>) -> T {
        tauri::async_runtime::block_on(future)
    }

    #[test]
    fn manage_covers_adjust_stages_a_resized_copy_and_keeps_fitting_images() {
        let dir = tempfile::tempdir().unwrap();
        let song = wav(dir.path(), "song.wav");
        let image = dir.path().join("big.png");
        std::fs::write(&image, png(400, 200)).unwrap();
        run(add_cover_image(
            vec![song.clone()],
            ImageSource::Path(image.to_string_lossy().into_owned()),
            None,
        ))
        .unwrap();
        let settings = |format, pixels| AdjustSettings {
            format,
            quality: 90,
            resize: Some(tag_core::actions::CoverResize {
                pixels,
                size_mode: tag_core::actions::CoverSizeMode::Maximum,
                scale_mode: tag_core::actions::CoverScaleMode::Proportional,
            }),
        };
        use tag_core::actions::CoverImageFormat::{Jpeg, Original};
        let resized = run(stage_adjusted_cover(
            AdjustSource::Cover {
                path: song.clone(),
                index: 0,
            },
            settings(Original, 100),
        ))
        .unwrap();
        assert_eq!(
            (resized.mime.as_str(), resized.width, resized.height),
            ("image/png", Some(100), Some(50))
        );
        // A staged image can be adjusted again, here to JPEG.
        let jpeg = run(stage_adjusted_cover(
            AdjustSource::Staged(resized.token.clone()),
            settings(Jpeg, 100),
        ))
        .unwrap();
        assert_eq!(
            (jpeg.mime.as_str(), jpeg.width, jpeg.height),
            ("image/jpeg", Some(100), Some(50))
        );
        // Within the size and format, the bytes are staged unchanged.
        let same = run(stage_adjusted_cover(
            AdjustSource::Cover {
                path: song.clone(),
                index: 0,
            },
            settings(Original, 800),
        ))
        .unwrap();
        assert_eq!(STANDARD.decode(&same.data_base64).unwrap(), png(400, 200));
        // The file itself is untouched until a plan is applied.
        assert_eq!(
            settings::read_full(Path::new(&song)).unwrap().covers[0].width,
            Some(400)
        );
        assert!(run(stage_adjusted_cover(
            AdjustSource::Cover {
                path: song,
                index: 3
            },
            settings(Original, 100),
        ))
        .is_err());
    }

    #[test]
    fn add_cover_starts_in_the_cover_art_directory_then_an_image_folder_then_the_working_folder() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("Album");
        let art = album.join("Artwork");
        std::fs::create_dir_all(&art).unwrap();
        let song = wav(&album, "song.wav");
        let album = Path::new(&song).parent().unwrap().to_path_buf();
        let working = dunce::canonicalize(dir.path()).unwrap();
        let settings = crate::evaluation::EvaluationSettings {
            working_directory: Some(working.to_string_lossy().into_owned()),
            ..Default::default()
        };
        let start =
            |template: &str| start_directory(Path::new(&song), template, &settings).unwrap();
        // Relative names and placeholders resolve against the file's folder.
        assert_eq!(start("Artwork"), album.join("Artwork").to_string_lossy());
        assert_eq!(
            start("$left(Artworks,7)"),
            album.join("Artwork").to_string_lossy()
        );
        assert_eq!(
            start(&dunce::canonicalize(&art).unwrap().to_string_lossy()),
            dunce::canonicalize(&art).unwrap().to_string_lossy()
        );
        // No option, or a folder that does not exist: the working folder,
        // unless the file's folder holds an image.
        assert_eq!(start(""), working.to_string_lossy());
        assert_eq!(start("Missing"), working.to_string_lossy());
        std::fs::write(album.join("front.JPG"), png(2, 2)).unwrap();
        assert_eq!(start("Missing"), album.to_string_lossy());
        let without = crate::evaluation::EvaluationSettings::default();
        std::fs::remove_file(album.join("front.JPG")).unwrap();
        assert_eq!(
            start_directory(Path::new(&song), "", &without).unwrap(),
            album.to_string_lossy()
        );
    }

    #[test]
    fn cover_set_aggregates_and_plans_apply_to_all_files() {
        let dir = tempfile::tempdir().unwrap();
        let one = wav(dir.path(), "one.wav");
        let two = wav(dir.path(), "two.wav");
        let image = dir.path().join("a.png");
        std::fs::write(&image, png(5, 4)).unwrap();
        let staged = run(stage_cover_file(image.to_string_lossy().into_owned())).unwrap();
        assert_eq!((staged.width, staged.height), (Some(5), Some(4)));
        assert_eq!(
            (staged.mime.as_str(), staged.label.as_str()),
            ("image/png", "a.png")
        );
        let paths = vec![one.clone(), two.clone()];
        run(add_cover_image(
            vec![one.clone()],
            ImageSource::Path(image.to_string_lossy().into_owned()),
            None,
        ))
        .unwrap();
        let set = run(cover_set(paths.clone())).unwrap();
        assert_eq!(set.files, 2);
        assert_eq!(set.covers.len(), 1);
        assert_eq!(set.covers[0].files, vec![0]);
        assert!(!set.covers[0].data_base64.is_empty());

        let item = |all_files| PlanItem {
            original: Some(set.covers[0].cover.clone()),
            image: None,
            picture_type: "CoverBack".into(),
            description: "Shared".into(),
            all_files,
            occurrence: None,
        };
        let result = run(apply_cover_plan(
            paths.clone(),
            vec![item(true)],
            "Manage covers".into(),
        ))
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let set = run(cover_set(paths.clone())).unwrap();
        assert_eq!(set.covers.len(), 1);
        assert_eq!(set.covers[0].files, vec![0, 1]);
        assert_eq!(set.covers[0].cover.picture_type, "CoverBack");

        // An empty plan removes every cover.
        run(apply_cover_plan(
            paths.clone(),
            vec![],
            "Remove cover".into(),
        ))
        .unwrap();
        assert!(run(cover_set(paths)).unwrap().covers.is_empty());
    }

    #[test]
    fn extract_adds_extension_and_reports_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let one = wav(dir.path(), "one.wav");
        let image = dir.path().join("a.png");
        std::fs::write(&image, png(3, 3)).unwrap();
        run(add_cover_image(
            vec![one.clone()],
            ImageSource::Path(image.to_string_lossy().into_owned()),
            None,
        ))
        .unwrap();
        let first = run(extract_cover(one.clone(), 0, "folder".into(), false)).unwrap();
        assert!(!first.exists);
        assert!(first.path.ends_with("folder.png"));
        assert_eq!(std::fs::read(&first.path).unwrap(), png(3, 3));
        let again = run(extract_cover(one.clone(), 0, "folder".into(), false)).unwrap();
        assert!(again.exists);
        assert!(run(extract_cover(one.clone(), 0, "folder".into(), true)).is_ok_and(|e| !e.exists));
        assert!(run(extract_cover(one.clone(), 4, "x".into(), false)).is_err());
        assert!(run(extract_cover(one, 0, " ".into(), false)).is_err());
    }

    #[test]
    fn staged_images_reject_non_images_and_expire() {
        assert!(stage(b"plain text, not an image".to_vec(), "x").is_err());
        let tokens: Vec<_> = (0..STAGED_LIMIT + 1)
            .map(|_| stage(png(1, 1), "x").unwrap().token)
            .collect();
        assert!(ImageSource::Staged(tokens[0].clone()).load().is_err());
        assert!(ImageSource::Staged(tokens[STAGED_LIMIT].clone())
            .load()
            .is_ok());
        assert_eq!(extension_for("image/jpeg"), "jpg");
        assert_eq!(history_label("anything"), "Edit covers");
    }
}
