use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tag_core::{Action, AudioEntry, Fields, FileError, ScanResult};
mod action_host;
mod converters;
mod evaluation;
mod file_hashes;
mod rename_plan;

#[tauri::command]
async fn load_action_library(
    app: tauri::AppHandle,
) -> Result<tag_core::actions::ActionLibrary, String> {
    let path = action_host::library_path(&app)?;
    io(move || action_host::load_library(&path)).await
}
#[tauri::command]
async fn save_action_library(
    app: tauri::AppHandle,
    library: tag_core::actions::ActionLibrary,
) -> Result<(), String> {
    let path = action_host::library_path(&app)?;
    io(move || action_host::save_library(&path, &library)).await
}
#[tauri::command]
async fn import_action_group(
    path: String,
    legacy_encoding: Option<String>,
) -> Result<tag_core::actions::MtaImport, String> {
    io(move || action_host::import_group(Path::new(&path), legacy_encoding.as_deref())).await
}
#[tauri::command]
async fn execute_action_groups(
    app: tauri::AppHandle,
    paths: Vec<String>,
    groups: Vec<tag_core::actions::ActionGroup>,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<action_host::ActionResult, String> {
    let settings = evaluation::EvaluationSettings::for_app(&app, columns)?;
    io(move || Ok(action_host::execute(&paths, &groups, settings))).await
}

// Serialize operations to prevent two UI commands replacing each other's edits.
static FILE_OPERATIONS: Mutex<()> = Mutex::new(());
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
fn evaluation_settings(
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<evaluation::EvaluationSettings, String> {
    match APP_HANDLE.get() {
        Some(app) => evaluation::EvaluationSettings::for_app(app, columns),
        None => Ok(evaluation::EvaluationSettings {
            columns: columns.unwrap_or_default(),
            ..Default::default()
        }),
    }
}

#[derive(Serialize)]
struct RuntimePlatform {
    family: &'static str,
    folder_picker_supported: bool,
}

#[tauri::command]
fn runtime_platform() -> RuntimePlatform {
    let family = if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else {
        "desktop"
    };
    RuntimePlatform {
        family,
        folder_picker_supported: family == "desktop",
    }
}

#[tauri::command]
async fn get_covers(path: String) -> Result<Vec<tag_core::CoverInfo>, String> {
    io(move || tag_core::read_entry(Path::new(&path)).map(|e| e.covers)).await
}

#[derive(Deserialize)]
struct FilterItem {
    path: String,
    fields: Fields,
    raw_fields: Option<Fields>,
}

#[tauri::command]
async fn filter_matches(
    query: String,
    items: Vec<FilterItem>,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<Vec<String>, String> {
    let settings = evaluation_settings(columns)?;
    tauri::async_runtime::spawn_blocking(move || {
        let filter = tag_core::compile_filter(&query)?;
        let mut matching = Vec::new();
        for item in items {
            if settings.matches(
                &filter,
                &item.fields,
                item.raw_fields.as_ref().unwrap_or(&item.fields),
                &item.path,
            )? {
                matching.push(item.path);
            }
        }
        Ok(matching)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn io<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = FILE_OPERATIONS
            .lock()
            .map_err(|_| "File operation lock failed".to_string())?;
        work()
    })
    .await
    .map_err(|e| e.to_string())?
}

fn batch(
    paths: &[String],
    mut work: impl FnMut(&Path) -> Result<AudioEntry, String>,
) -> ScanResult {
    let mut result = ScanResult {
        entries: vec![],
        errors: vec![],
    };
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        if !seen.insert(path) {
            continue;
        }
        match work(Path::new(path)) {
            Ok(entry) => result.entries.push(entry),
            Err(error) => result.errors.push(FileError {
                path: path.clone(),
                error,
            }),
        }
    }
    result
}

#[tauri::command]
async fn scan_files(paths: Vec<String>, recursive: bool) -> Result<ScanResult, String> {
    io(move || Ok(tag_core::scan_paths(&paths, recursive))).await
}

#[tauri::command]
async fn save_fields(paths: Vec<String>, updates: Fields) -> Result<ScanResult, String> {
    io(move || {
        Ok(batch(&paths, |p| {
            tag_core::write_fields_summary(p, &updates)
        }))
    })
    .await
}

#[tauri::command]
async fn remove_tags(paths: Vec<String>) -> Result<ScanResult, String> {
    io(move || Ok(batch(&paths, tag_core::remove_tags_summary))).await
}

#[tauri::command]
async fn add_cover(paths: Vec<String>, image_path: String) -> Result<ScanResult, String> {
    io(move || {
        Ok(batch(&paths, |p| {
            tag_core::add_cover_summary(p, Path::new(&image_path))
        }))
    })
    .await
}

#[tauri::command]
async fn remove_cover(paths: Vec<String>, index: usize) -> Result<ScanResult, String> {
    io(move || Ok(batch(&paths, |p| tag_core::remove_cover_summary(p, index)))).await
}

#[derive(Serialize)]
struct Preview {
    path: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_token: Option<String>,
}

#[tauri::command]
async fn preview_format(
    paths: Vec<String>,
    template: String,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<Vec<Preview>, String> {
    let settings = evaluation_settings(columns)?;
    io(move || {
        paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| {
                let entry = tag_core::read_entry_summary(Path::new(&path))?;
                let value = settings.evaluate(
                    &template,
                    &entry.fields,
                    &evaluation::technical_fields(&entry, index + 1),
                    &entry.path,
                    true,
                )?;
                Ok(Preview {
                    path,
                    value,
                    plan_token: None,
                })
            })
            .collect()
    })
    .await
}

#[derive(Serialize)]
struct ParsedPreview {
    path: String,
    fields: Fields,
}

#[tauri::command]
async fn preview_filename(
    paths: Vec<String>,
    pattern: String,
) -> Result<Vec<ParsedPreview>, String> {
    io(move || {
        paths
            .into_iter()
            .map(|path| {
                let fields = tag_core::parse_filename(&path, &pattern)?;
                Ok(ParsedPreview { path, fields })
            })
            .collect()
    })
    .await
}

#[tauri::command]
async fn filename_to_tags(paths: Vec<String>, pattern: String) -> Result<ScanResult, String> {
    io(move || {
        Ok(batch(&paths, |p| {
            let updates = tag_core::parse_filename(&p.to_string_lossy(), &pattern)?;
            tag_core::write_fields_summary(p, &updates)
        }))
    })
    .await
}

#[tauri::command]
async fn apply_action(
    paths: Vec<String>,
    action: Action,
) -> Result<action_host::ActionResult, String> {
    let settings = evaluation_settings(None)?;
    let group = tag_core::actions::ActionGroup {
        name: "Quick action".into(),
        enabled: true,
        actions: vec![tag_core::actions::from_quick_action(&action)?],
    };
    io(move || Ok(action_host::execute(&paths, &[group], settings))).await
}

#[cfg(test)]
fn rename_target(entry: &AudioEntry, template: &str) -> Result<PathBuf, String> {
    rename_target_with_settings(
        entry,
        template,
        1,
        &evaluation::EvaluationSettings::default(),
    )
}

fn rename_target_with_settings(
    entry: &AudioEntry,
    template: &str,
    index: usize,
    settings: &evaluation::EvaluationSettings,
) -> Result<PathBuf, String> {
    let generated = settings.filename(template, entry, index)?;
    filename_target(entry, generated)
}

fn filename_target(entry: &AudioEntry, generated: String) -> Result<PathBuf, String> {
    if generated.trim().is_empty() {
        return Err("Filename evaluates to an empty string".into());
    }
    let generated = if cfg!(windows) && generated.starts_with(r"\\?\") {
        generated
    } else {
        generated.replace('\\', "/")
    };
    let relative = Path::new(&generated);
    let source = Path::new(&entry.path);
    let mut target = if relative.is_absolute() {
        relative.to_path_buf()
    } else {
        source.parent().unwrap_or(Path::new(".")).join(relative)
    };
    if let Some(ext) = source.extension() {
        if target.extension() != Some(ext) {
            let mut name = target.into_os_string();
            name.push(".");
            name.push(ext);
            target = PathBuf::from(name);
        }
    }
    Ok(target)
}

#[tauri::command]
async fn preview_rename(
    paths: Vec<String>,
    template: String,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<Vec<Preview>, String> {
    let settings = evaluation_settings(columns)?;
    io(move || {
        let mut targets = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let entry = tag_core::read_entry_summary(Path::new(path))?;
            targets.push(rename_target_with_settings(
                &entry,
                &template,
                index + 1,
                &settings,
            )?);
        }
        let token = rename_plan::record(&paths, &template, targets.clone())?;
        Ok(paths
            .into_iter()
            .zip(targets)
            .map(|(path, target)| Preview {
                path,
                value: target.to_string_lossy().into_owned(),
                plan_token: Some(token.clone()),
            })
            .collect())
    })
    .await
}

#[derive(Serialize)]
struct Renamed {
    from: String,
    to: String,
}
#[derive(Serialize)]
struct RenameResult {
    #[serde(flatten)]
    result: ScanResult,
    renamed: Vec<Renamed>,
}

#[tauri::command]
async fn rename_files(
    paths: Vec<String>,
    template: String,
    plan_token: Option<String>,
) -> Result<RenameResult, String> {
    let settings = evaluation_settings(None)?;
    io(move || {
        let mut result = ScanResult {
            entries: vec![],
            errors: vec![],
        };
        let mut plans = vec![];
        let mut destinations = std::collections::HashSet::new();
        let candidates = if let Some(token) = plan_token {
            rename_plan::consume(&token, &paths, &template)?
                .into_iter()
                .map(|(path, target)| (path, Ok(target)))
                .collect::<Vec<_>>()
        } else {
            paths
                .into_iter()
                .enumerate()
                .map(|(index, path)| {
                    let plan = tag_core::read_entry_summary(Path::new(&path)).and_then(|entry| {
                        rename_target_with_settings(&entry, &template, index + 1, &settings)
                    });
                    (path, plan)
                })
                .collect()
        };
        for (path, plan) in candidates {
            match plan {
                Ok(target)
                    if destinations.insert(target.clone())
                        && (std::fs::canonicalize(&path).ok().as_ref() == Some(&target)
                            || !target.exists()) =>
                {
                    plans.push((path, target))
                }
                Ok(target) => result.errors.push(FileError {
                    path,
                    error: format!(
                        "Destination exists or another file maps to {}",
                        target.display()
                    ),
                }),
                Err(error) => result.errors.push(FileError { path, error }),
            }
        }
        let mut renamed = vec![];
        // Validate the entire plan before touching any file.
        if !result.errors.is_empty() {
            return Ok(RenameResult { result, renamed });
        }
        for (path, target) in plans {
            let operation = if std::fs::canonicalize(&path).ok().as_ref() == Some(&target) {
                Ok(())
            } else {
                tag_core::operations::rename_no_clobber(Path::new(&path), &target)
            };
            match operation {
                Ok(()) => {
                    let to = target.to_string_lossy().into_owned();
                    renamed.push(Renamed {
                        from: path,
                        to: to.clone(),
                    });
                    match tag_core::read_entry_summary(&target) {
                        Ok(entry) => result.entries.push(entry),
                        Err(error) => result.errors.push(FileError { path: to, error }),
                    }
                }
                Err(error) => result.errors.push(FileError { path, error }),
            }
        }
        Ok(RenameResult { result, renamed })
    })
    .await
}

fn export_entries(paths: &[String], output: &Path) -> Result<Vec<AudioEntry>, String> {
    if paths.iter().any(|p| Path::new(p) == output) {
        return Err("Export cannot replace an input audio file".into());
    }
    paths
        .iter()
        .map(|p| tag_core::read_entry_summary(Path::new(p)))
        .collect()
}

fn write_export(output: &Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

fn export_field(fields: &Fields, format: &str, key: &str, separator: &str) -> String {
    let lookup = |name: &str| {
        fields
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, values)| values.join(separator))
            .unwrap_or_default()
    };
    let format = format.to_ascii_lowercase();
    if key == "TRACK"
        && ["flac", "ogg", "vorbis", "opus", "speex"]
            .iter()
            .any(|name| format.contains(name))
    {
        return lookup("TRACKNUMBER");
    }
    let value = lookup(key);
    if value.is_empty() {
        match key {
            "TRACK" => lookup("TRACKNUMBER"),
            "YEAR" => lookup("DATE"),
            _ => value,
        }
    } else {
        value
    }
}

#[tauri::command]
async fn export_csv(paths: Vec<String>, output_path: String) -> Result<(), String> {
    io(move || {
        let entries = export_entries(&paths, Path::new(&output_path))?;
        let names = [
            "TITLE", "ARTIST", "ALBUM", "TRACK", "YEAR", "GENRE", "COMMENT",
        ];
        let mut rows = vec![names
            .iter()
            .map(|s| s.to_string())
            .chain(["PATH".to_string()])
            .collect()];
        for entry in entries {
            rows.push(
                names
                    .iter()
                    .map(|key| export_field(&entry.fields, &entry.format, key, "\\\\"))
                    .chain([entry.path])
                    .collect(),
            );
        }
        write_export(
            Path::new(&output_path),
            &tag_core::operations::render_csv(&rows),
        )
    })
    .await
}

#[tauri::command]
async fn export_playlist(paths: Vec<String>, output_path: String) -> Result<(), String> {
    io(move || {
        let output = Path::new(&output_path);
        let entries = export_entries(&paths, output)?;
        let mut text = String::from("#EXTM3U\n");
        for entry in entries {
            let get = |key: &str| {
                export_field(&entry.fields, &entry.format, key, " / ").replace(['\r', '\n'], " ")
            };
            let relative = tag_core::operations::playlist_path(Path::new(&entry.path), output);
            let path = relative.to_string_lossy();
            if path.contains(['\r', '\n']) {
                return Err("Playlist paths cannot contain line breaks".into());
            }
            text.push_str(&format!(
                "#EXTINF:{},{} - {}\n{}\n",
                entry.duration.round() as u64,
                get("ARTIST"),
                get("TITLE"),
                path.replace('\\', "/")
            ));
        }
        write_export(output, &text)
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_document_access::init())
        .invoke_handler(tauri::generate_handler![
            load_action_library,
            save_action_library,
            import_action_group,
            execute_action_groups,
            runtime_platform,
            get_covers,
            filter_matches,
            scan_files,
            save_fields,
            remove_tags,
            add_cover,
            remove_cover,
            preview_format,
            preview_filename,
            preview_rename,
            filename_to_tags,
            apply_action,
            rename_files,
            converters::preview_filename_conversion,
            converters::apply_filename_conversion,
            converters::preview_text_import,
            converters::apply_text_import,
            export_csv,
            export_playlist
        ])
        .run(tauri::generate_context!())
        .expect("Unable to run OpenTag");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path) {
        let samples = vec![0u8; 8820];
        let mut data = b"RIFF".to_vec();
        data.extend((36 + samples.len() as u32).to_le_bytes());
        data.extend(b"WAVEfmt ");
        data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(44100u32.to_le_bytes());
        data.extend(88200u32.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(16u16.to_le_bytes());
        data.extend(b"data");
        data.extend((samples.len() as u32).to_le_bytes());
        data.extend(samples);
        std::fs::write(path, data).unwrap();
    }

    #[test]
    fn rename_batch_collision_aborts_every_source() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        wav(&a);
        wav(&b);
        let result = tauri::async_runtime::block_on(rename_files(
            vec![
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            "same".into(),
            None,
        ))
        .unwrap();
        assert!(!result.result.errors.is_empty());
        assert!(result.renamed.is_empty());
        assert!(a.exists() && b.exists());
        assert!(!dir.path().join("same.wav").exists());
    }

    #[test]
    fn rename_preview_and_result_use_identical_sanitized_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["A/B".into()]);
        tag_core::write_fields(&path, &fields).unwrap();
        let paths = vec![path.to_string_lossy().into_owned()];
        let preview = tauri::async_runtime::block_on(preview_rename(
            paths.clone(),
            "Album/%title%".into(),
            None,
        ))
        .unwrap();
        let result = tauri::async_runtime::block_on(rename_files(
            paths,
            "Album/%title%".into(),
            preview[0].plan_token.clone(),
        ))
        .unwrap();
        assert!(result.result.errors.is_empty());
        assert_eq!(preview[0].value, result.renamed[0].to);
        assert!(result.renamed[0].to.ends_with("A_B.wav"));
        assert_eq!(result.result.entries[0].fields["TITLE"], ["A/B"]);
    }

    #[test]
    fn dotted_filename_preview_and_import_preserve_complete_title() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Artist - title.live.wav");
        wav(&path);
        let paths = vec![path.to_string_lossy().into_owned()];
        let previews = tauri::async_runtime::block_on(preview_filename(
            paths.clone(),
            "%artist% - %title%".into(),
        ))
        .unwrap();
        assert_eq!(previews[0].fields["TITLE"], ["title.live"]);
        let result =
            tauri::async_runtime::block_on(filename_to_tags(paths, "%artist% - %title%".into()))
                .unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.entries[0].fields["TITLE"], ["title.live"]);
    }

    #[test]
    fn remove_action_really_removes_key_from_saved_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["Keep".into()]);
        fields.insert("COMMENT".into(), vec!["Remove".into()]);
        tag_core::write_fields(&path, &fields).unwrap();
        let result = tauri::async_runtime::block_on(apply_action(
            vec![path.to_string_lossy().into_owned()],
            Action::Remove {
                fields: vec!["COMMENT".into()],
            },
        ))
        .unwrap();
        assert!(result.errors.is_empty());
        assert!(!result.entries[0].fields.contains_key("COMMENT"));
        assert_eq!(result.entries[0].fields["TITLE"], ["Keep"]);
    }

    #[test]
    fn export_refuses_input_and_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let before = std::fs::read(&path).unwrap();
        let result = tauri::async_runtime::block_on(export_csv(
            vec![path.to_string_lossy().into_owned()],
            path.to_string_lossy().into_owned(),
        ));
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let out = dir.path().join("old.csv");
        std::fs::write(&out, b"keep").unwrap();
        assert!(write_export(&out, "new").is_err());
        assert_eq!(std::fs::read(out).unwrap(), b"keep");
    }

    #[test]
    fn exports_preserve_native_tag_casing_aliases_and_multivalues() {
        use std::process::Command;
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            eprintln!("Skipping encoded export regression: ffmpeg is required");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("Café 猫.flac");
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=0.1",
                "-metadata",
                "title=Café; 猫 \"live\"",
                "-metadata",
                "album=Native album",
                "-metadata",
                "date=2020-04-18",
                "-metadata",
                "track=03/06",
                "-metadata",
                "genre=Ambient",
                "-metadata",
                "comment=first line\nsecond line",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        tag_core::write_fields_summary(
            &path,
            &Fields::from([
                ("ARTIST".into(), vec!["Mémoire".into(), "Guest 猫".into()]),
                ("COMMENT".into(), vec!["first line\nsecond line".into()]),
            ]),
        )
        .unwrap();
        let entry = tag_core::read_entry_summary(&path).unwrap();
        assert!(entry.fields.contains_key("title"));
        assert!(entry.fields.contains_key("date"));
        let before = std::fs::read(&path).unwrap();
        let paths = vec![path.to_string_lossy().into_owned()];
        let csv = root.join("tags.csv");
        tauri::async_runtime::block_on(export_csv(
            paths.clone(),
            csv.to_string_lossy().into_owned(),
        ))
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(csv).unwrap(),
            format!("TITLE;ARTIST;ALBUM;TRACK;YEAR;GENRE;COMMENT;PATH\r\n\"Café; 猫 \"\"live\"\"\";Mémoire\\\\Guest 猫;Native album;03/06;2020-04-18;Ambient;\"first line\nsecond line\";{}\r\n", entry.path),
        );
        let playlist = root.join("list.m3u8");
        tauri::async_runtime::block_on(export_playlist(
            paths,
            playlist.to_string_lossy().into_owned(),
        ))
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(playlist).unwrap(),
            "#EXTM3U\n#EXTINF:0,Mémoire / Guest 猫 - Café; 猫 \"live\"\nCafé 猫.flac\n",
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    #[test]
    fn export_lookup_keeps_comment_description_and_explicit_aliases_distinct() {
        let fields = Fields::from([
            ("comment".into(), vec!["Unicode 猫".into()]),
            ("DESCRIPTION".into(), vec!["Independent description".into()]),
            ("year".into(), vec!["1998".into()]),
            ("DATE".into(), vec!["2020-04-18".into()]),
            ("track".into(), vec!["2/9".into()]),
            ("TRACKNUMBER".into(), vec!["3/9".into()]),
        ]);
        assert_eq!(
            export_field(&fields, "WAV", "COMMENT", "\\\\"),
            "Unicode 猫"
        );
        assert_eq!(
            export_field(&fields, "WAV", "DESCRIPTION", "\\\\"),
            "Independent description"
        );
        assert_eq!(export_field(&fields, "WAV", "YEAR", "\\\\"), "1998");
        assert_eq!(export_field(&fields, "WAV", "TRACK", "\\\\"), "2/9");
        for format in ["FLAC", "Vorbis", "Opus", "Speex", "Ogg"] {
            assert_eq!(export_field(&fields, format, "TRACK", "\\\\"), "3/9");
        }
    }

    #[test]
    fn rename_functions_receive_original_unsanitized_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["A:B".into()]);
        let entry = tag_core::write_fields(&path, &fields).unwrap();
        let target = rename_target(&entry, "$replace(%title%,':','-')").unwrap();
        assert_eq!(target.file_name().unwrap(), "A-B.wav");
    }
}
