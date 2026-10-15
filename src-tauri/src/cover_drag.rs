//! Dragging the displayed cover out of the cover window (to the desktop or a
//! file manager folder). The image is written to a temporary file that the
//! native drag session hands over as a file; the drop target copies it.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Temporary drag files older than this are removed before the next drag.
const KEEP: Duration = Duration::from_secs(60 * 60);
const IMAGE_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct DragOut {
    /// The temporary image file offered to the drop target.
    pub path: String,
    /// `dropped`, `cancelled`, or `prepared` when no native drag was started.
    pub result: &'static str,
}

/// A file name for the dragged cover from `hint` (album or audio file name)
/// with the image's extension. Characters that file systems or file URIs
/// reject are replaced.
pub fn file_name(hint: &str, mime: &str) -> String {
    let unsafe_char = |c: char| {
        c.is_control()
            || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            // The Linux drag source writes file URIs without escaping them.
            || (cfg!(target_os = "linux")
                && !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '(' | ')')))
    };
    let trimmed = hint.trim_matches(|c: char| c == '.' || c.is_whitespace());
    let cleaned: String = trimmed
        .chars()
        .map(|c| if unsafe_char(c) { '_' } else { c })
        .take(100)
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
    let stem = if cleaned.chars().all(|c| c == '_' || c == '.') {
        "Cover"
    } else {
        cleaned
    };
    format!("{stem}.{}", crate::covers::extension_for(mime))
}

/// Write `data` as `name` into a new folder below `root`, after removing
/// drag folders that are older than an hour at `now`.
pub fn prepare(root: &Path, data: &[u8], name: &str, now: SystemTime) -> Result<PathBuf, String> {
    if data.is_empty() || data.len() > IMAGE_LIMIT {
        return Err("The cover cannot be dragged".into());
    }
    if image::guess_format(data).is_err() {
        return Err("The cover is not an image".into());
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > KEEP);
            if old {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let folder = root.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(name);
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(path)
}

/// A small drag image: the platform shows the drag image at its own size.
fn preview(data: &[u8]) -> Vec<u8> {
    let Ok(image) = image::load_from_memory(data) else {
        return data.to_vec();
    };
    let mut bytes = std::io::Cursor::new(Vec::new());
    match image
        .thumbnail(96, 96)
        .write_to(&mut bytes, image::ImageFormat::Png)
    {
        Ok(()) => bytes.into_inner(),
        Err(_) => data.to_vec(),
    }
}

fn drag_root() -> PathBuf {
    std::env::temp_dir().join("opentag-cover-drag")
}

/// Start dragging the displayed cover out of the window. Resolves when the
/// drag ends. Scripted test runs only prepare the file: a native drag needs
/// a real pointer.
#[tauri::command]
pub async fn drag_cover_out(
    window: tauri::WebviewWindow,
    data_base64: String,
    mime: String,
    name: String,
) -> Result<DragOut, String> {
    let data = STANDARD.decode(data_base64).map_err(|e| e.to_string())?;
    let target = file_name(&name, &mime);
    let (path, data) = tauri::async_runtime::spawn_blocking(move || {
        prepare(&drag_root(), &data, &target, SystemTime::now()).map(|path| (path, data))
    })
    .await
    .map_err(|e| e.to_string())??;
    let shown = path.to_string_lossy().into_owned();
    if crate::e2e::enabled() {
        return Ok(DragOut {
            path: shown,
            result: "prepared",
        });
    }
    start(window, path, preview(&data))
        .await
        .map(|result| DragOut {
            path: shown,
            result,
        })
}

#[cfg(desktop)]
async fn start(
    window: tauri::WebviewWindow,
    path: PathBuf,
    image: Vec<u8>,
) -> Result<&'static str, String> {
    use std::sync::Mutex;
    let (sender, receiver) = tokio::sync::oneshot::channel::<Result<&'static str, String>>();
    let sender = std::sync::Arc::new(Mutex::new(Some(sender)));
    let send = move |result: Result<&'static str, String>| {
        if let Some(sender) = sender.lock().ok().and_then(|mut s| s.take()) {
            let _ = sender.send(result);
        }
    };
    let finished = send.clone();
    let handle = window.clone();
    window
        .run_on_main_thread(move || {
            #[cfg(target_os = "linux")]
            let source = handle.gtk_window();
            #[cfg(not(target_os = "linux"))]
            let source = tauri::Result::Ok(handle.clone());
            let started = match source {
                Ok(source) => drag::start_drag(
                    &source,
                    drag::DragItem::Files(vec![path]),
                    drag::Image::Raw(image),
                    move |result, _| {
                        finished(Ok(match result {
                            drag::DragResult::Dropped => "dropped",
                            drag::DragResult::Cancel => "cancelled",
                        }))
                    },
                    drag::Options::default(),
                )
                .map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            };
            if let Err(error) = started {
                send(Err(format!("The cover could not be dragged: {error}")));
            }
        })
        .map_err(|e| e.to_string())?;
    // A drag source that never reports its end must not keep the window waiting.
    match tokio::time::timeout(Duration::from_secs(600), receiver).await {
        Ok(Ok(result)) => result,
        _ => Ok("cancelled"),
    }
}

#[cfg(mobile)]
async fn start(_: tauri::WebviewWindow, _: PathBuf, _: Vec<u8>) -> Result<&'static str, String> {
    Err("Dragging covers out is not available here".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(300, 200, image::Rgb([200, 10, 10]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn dragged_covers_get_safe_names_with_the_image_extension() {
        assert_eq!(file_name("Album", "image/png"), "Album.png");
        assert_eq!(file_name("Album", "image/jpeg"), "Album.jpg");
        assert_eq!(file_name("", "image/webp"), "Cover.webp");
        assert_eq!(file_name("  ..  ", "image/gif"), "Cover.gif");
        assert_eq!(file_name("a/b:c", "image/jpeg"), "a_b_c.jpg");
        assert_eq!(file_name("\u{0}\u{7}", "image/jpeg"), "Cover.jpg");
        assert!(file_name(&"x".repeat(300), "image/png").len() <= 104);
        if cfg!(target_os = "linux") {
            assert_eq!(file_name("Björk Live", "image/png"), "Bj_rk_Live.png");
        } else {
            assert_eq!(file_name("Björk Live", "image/png"), "Björk Live.png");
        }
    }

    #[test]
    fn prepared_files_hold_the_image_and_old_folders_are_removed() {
        let root = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let data = png();
        let path = prepare(root.path(), &data, "Album.png", now).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), data);
        assert_eq!(path.file_name().unwrap(), "Album.png");
        assert!(path.starts_with(root.path()));
        // A second drag of the same name gets its own folder; recent ones stay.
        let again = prepare(root.path(), &data, "Album.png", now).unwrap();
        assert_ne!(again, path);
        assert!(path.exists());
        // Two hours later the earlier folders are removed.
        let later = now + Duration::from_secs(2 * 60 * 60);
        let last = prepare(root.path(), &data, "Album.png", later).unwrap();
        assert!(!path.exists() && !again.exists());
        assert!(last.exists());
        assert!(prepare(root.path(), b"not an image", "x.jpg", now).is_err());
        assert!(prepare(root.path(), b"", "x.jpg", now).is_err());
    }

    #[test]
    fn drag_previews_are_small_pngs() {
        let small = preview(&png());
        let image = image::load_from_memory(&small).unwrap();
        assert!(image.width() <= 96 && image.height() <= 96);
        assert_eq!(preview(b"junk"), b"junk");
    }
}
