//! File List behaviours that need the backend: the first image of a file's
//! folder for the cover pane (read only, nothing is written), and the artwork
//! half of Copy/Cut/Paste tags. Copied cover images stay here, keyed by
//! content digest, until a save writes the pasted covers.
use super::{documents, io, settings};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tag_core::{CoverChange, FileError};

const IMAGE_EXTENSIONS: [&str; 6] = ["jpg", "jpeg", "png", "gif", "bmp", "webp"];
const FOLDER_IMAGE_LIMIT: u64 = 32 * 1024 * 1024;
/// Copied artwork is held in memory; larger selections copy their tags only.
const CLIPBOARD_BYTES: usize = 256 * 1024 * 1024;

#[derive(Serialize)]
pub struct FolderImage {
    name: String,
    mime: String,
    size: u64,
    width: Option<u32>,
    height: Option<u32>,
    data_base64: String,
}

fn mime_and_size(data: &[u8]) -> Option<(String, Option<u32>, Option<u32>)> {
    let picture = tag_core::picture_from_bytes(data.to_vec()).ok()?;
    let mime = picture.mime_type()?.as_str().to_owned();
    let (width, height) = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
        .map_or((None, None), |(w, h)| (Some(w), Some(h)));
    Some((mime, width, height))
}

/// Image files of `dir` in name order, ignoring case like Explorer and Finder.
pub(crate) fn folder_images(dir: &Path) -> Vec<PathBuf> {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut images: Vec<(String, PathBuf)> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|e| {
                IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_ascii_lowercase().as_str())
            }) && path.is_file()
        })
        .map(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            (name.to_lowercase(), path)
        })
        .collect();
    images.sort();
    images.into_iter().map(|(_, path)| path).collect()
}

/// The first readable image next to `audio`, or `None`.
fn read_folder_image(audio: &Path) -> Option<FolderImage> {
    if cfg!(target_os = "android")
        || audio.to_string_lossy().starts_with("content://")
        || documents::is_managed(audio)
    {
        return None;
    }
    let dir = audio.parent()?;
    folder_images(dir).into_iter().find_map(|path| {
        let size = std::fs::metadata(&path).ok()?.len();
        if size == 0 || size > FOLDER_IMAGE_LIMIT {
            return None;
        }
        let data = std::fs::read(&path).ok()?;
        let (mime, width, height) = mime_and_size(&data)?;
        Some(FolderImage {
            name: path.file_name()?.to_string_lossy().into_owned(),
            mime,
            size,
            width,
            height,
            data_base64: STANDARD.encode(&data),
        })
    })
}

/// Options → Tags → "Don't display first image from file directory as cover
/// art" off: the cover pane shows this image for files without embedded art.
#[tauri::command]
pub async fn folder_image(path: String) -> Result<Option<FolderImage>, String> {
    // Read-only and independent of tag writes, so it skips the file operation lock.
    tauri::async_runtime::spawn_blocking(move || read_folder_image(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())
}

/// Serializes tests that replace the shared clipboard store.
#[cfg(test)]
pub static TEST_CLIPBOARD: Mutex<()> = Mutex::new(());

fn clipboard() -> &'static Mutex<HashMap<String, Vec<u8>>> {
    static CLIPBOARD: std::sync::OnceLock<Mutex<HashMap<String, Vec<u8>>>> =
        std::sync::OnceLock::new();
    CLIPBOARD.get_or_init(Default::default)
}

/// One copied cover; the bytes stay in the backend under `digest`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClipCover {
    digest: String,
    picture_type: String,
    description: String,
    mime: String,
    size: u64,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Serialize, Default)]
pub struct CopiedCovers {
    /// Per requested path; `None` when its covers could not be copied.
    covers: Vec<Option<Vec<ClipCover>>>,
    /// Image data by digest, once per distinct image, for the cover pane.
    images: HashMap<String, String>,
    errors: Vec<FileError>,
}

fn copy_covers(paths: &[String], keep: &[String]) -> CopiedCovers {
    let mut copied = CopiedCovers::default();
    let mut store: HashMap<String, Vec<u8>> = HashMap::new();
    {
        let previous = clipboard().lock().map(|g| g.clone()).unwrap_or_default();
        let keep: HashSet<&String> = keep.iter().collect();
        for (digest, data) in previous {
            if keep.contains(&digest) {
                store.insert(digest, data);
            }
        }
    }
    let mut bytes: usize = store.values().map(Vec::len).sum();
    for path in paths {
        let read = super::require_audio_handle(Path::new(path))
            .and_then(|_| settings::read_full(Path::new(path)));
        let entry = match read {
            Ok(entry) => entry,
            Err(error) => {
                copied.covers.push(None);
                copied.errors.push(FileError {
                    path: path.clone(),
                    error,
                });
                continue;
            }
        };
        let mut covers = Vec::new();
        let mut failed = None;
        for cover in &entry.covers {
            let data = match STANDARD.decode(&cover.data_base64) {
                Ok(data) => data,
                Err(error) => {
                    failed = Some(error.to_string());
                    break;
                }
            };
            let digest = if cover.digest.is_empty() {
                tag_core::cover_digest(&data)
            } else {
                cover.digest.clone()
            };
            if !store.contains_key(&digest) {
                if bytes + data.len() > CLIPBOARD_BYTES {
                    failed = Some("The copied artwork is larger than 256 MB".into());
                    break;
                }
                bytes += data.len();
                copied
                    .images
                    .insert(digest.clone(), cover.data_base64.clone());
                store.insert(digest.clone(), data);
            } else if !copied.images.contains_key(&digest) {
                copied
                    .images
                    .insert(digest.clone(), cover.data_base64.clone());
            }
            covers.push(ClipCover {
                digest,
                picture_type: cover.picture_type.clone(),
                description: cover.description.clone(),
                mime: cover.mime.clone(),
                size: cover.size,
                width: cover.width,
                height: cover.height,
            });
        }
        match failed {
            None => copied.covers.push(Some(covers)),
            Some(error) => {
                copied.covers.push(None);
                copied.errors.push(FileError {
                    path: path.clone(),
                    error,
                });
            }
        }
    }
    if let Ok(mut guard) = clipboard().lock() {
        *guard = store;
    }
    copied
}

/// Copy tags: read the covers of `paths` and hold their images until a save
/// writes them. Images of earlier copies are dropped unless `keep` names them
/// (covers pasted but not saved yet).
#[tauri::command]
pub async fn copy_tag_covers(
    paths: Vec<String>,
    keep: Vec<String>,
) -> Result<CopiedCovers, String> {
    io(move || Ok(copy_covers(&paths, &keep))).await
}

/// A pasted cover list entry, saved with the file's pending tags.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PendingCover {
    digest: String,
    picture_type: String,
    description: String,
}

/// The cover changes that replace a file's covers with `covers`.
pub fn cover_changes(covers: &[PendingCover]) -> Result<Vec<CoverChange>, String> {
    let store = clipboard()
        .lock()
        .map_err(|_| "The tag clipboard is busy".to_string())?;
    covers
        .iter()
        .map(|cover| {
            let data = store.get(&cover.digest).cloned().ok_or(
                "The pasted artwork is no longer available; copy and paste the tags again",
            )?;
            Ok(CoverChange {
                original: None,
                image: Some(data),
                picture_type: cover.picture_type.clone(),
                description: cover.description.clone(),
                all_files: true,
                occurrence: None,
            })
        })
        .collect()
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

    #[test]
    fn folder_image_is_the_first_image_by_name_and_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("song.mp3");
        std::fs::write(&audio, b"not read").unwrap();
        assert!(read_folder_image(&audio).is_none());
        std::fs::write(dir.path().join("notes.txt"), b"text").unwrap();
        std::fs::write(dir.path().join("Folder.png"), png(3, 2)).unwrap();
        std::fs::write(dir.path().join("cover.png"), png(4, 4)).unwrap();
        std::fs::write(dir.path().join("broken.jpg"), b"not an image").unwrap();
        let before: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        let image = read_folder_image(&audio).unwrap();
        // broken.jpg sorts first but is skipped; cover.png precedes Folder.png ignoring case.
        assert_eq!(image.name, "cover.png");
        assert_eq!((image.width, image.height), (Some(4), Some(4)));
        assert_eq!(image.mime, "image/png");
        assert_eq!(STANDARD.decode(image.data_base64).unwrap(), png(4, 4));
        let after: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(before.len(), after.len());
    }

    #[test]
    fn pending_covers_resolve_from_the_clipboard_store() {
        let _serial = TEST_CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
        let data = png(2, 2);
        let digest = tag_core::cover_digest(&data);
        clipboard()
            .lock()
            .unwrap()
            .insert(digest.clone(), data.clone());
        let changes = cover_changes(&[PendingCover {
            digest: digest.clone(),
            picture_type: "CoverFront".into(),
            description: String::new(),
        }])
        .unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].image.as_deref(), Some(data.as_slice()));
        assert!(changes[0].original.is_none() && changes[0].all_files);
        assert!(cover_changes(&[PendingCover {
            digest: "missing".into(),
            picture_type: "CoverFront".into(),
            description: String::new(),
        }])
        .is_err());
        assert!(cover_changes(&[]).unwrap().is_empty());
    }
}
