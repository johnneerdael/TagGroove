use crate::evaluation::{technical_fields, EvaluationSettings};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tag_core::actions::{
    ActionEffect, ActionGroup, ActionHost, ActionLibrary, ActionSpec, ActionState,
    BatchActionEffect, MtaImport, NameOrigin, TemplateMode,
};
use tag_core::{AudioEntry, Fields, FileError, ScanResult};
use tauri::Manager;

#[derive(Debug, Serialize)]
pub struct PathChange {
    pub from: String,
    pub to: String,
}
#[derive(Serialize)]
pub struct ActionResult {
    #[serde(flatten)]
    pub result: ScanResult,
    pub renamed: Vec<PathChange>,
    pub moved_directories: Vec<PathChange>,
    /// Playlist and export files written by the actions.
    pub outputs: Vec<String>,
}
impl std::ops::Deref for ActionResult {
    type Target = ScanResult;
    fn deref(&self) -> &Self::Target {
        &self.result
    }
}

pub fn library_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("actions.json"))
}
pub fn load_library(path: &Path) -> Result<ActionLibrary, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read action configuration: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ActionLibrary::default()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn save_library(path: &Path, library: &ActionLibrary) -> Result<(), String> {
    let mut names = HashSet::new();
    for group in &library.groups {
        if group.name.trim().is_empty() || !names.insert(group.name.to_lowercase()) {
            return Err("Action group names must be nonempty and unique".into());
        }
    }
    let bytes = serde_json::to_vec_pretty(library).map_err(|e| e.to_string())?;
    atomic_output(path, &bytes, false)
}
pub fn import_group(path: &Path, legacy_encoding: Option<&str>) -> Result<MtaImport, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    tag_core::actions::import_mta(
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        &bytes,
        legacy_encoding,
    )
}
/// Write a group as a UTF-16LE `.mta` file with BOM and CRLF lines, the
/// layout of the action files shipped with Mp3tag.
pub fn export_group(path: &Path, group: &ActionGroup) -> Result<(), String> {
    let text = tag_core::actions::export_mta(group)?;
    let mut bytes = vec![0xff, 0xfe];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    atomic_output(path, &bytes, false)
}
#[tauri::command]
pub async fn export_action_group(path: String, group: ActionGroup) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || export_group(Path::new(&path), &group))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn atomic_output(path: &Path, bytes: &[u8], append: bool) -> Result<(), String> {
    atomic_output_with_directories(path, bytes, append, true)
}
fn atomic_output_with_directories(
    path: &Path,
    bytes: &[u8],
    append: bool,
    create_directories: bool,
) -> Result<(), String> {
    let parent = path.parent().unwrap_or(Path::new("."));
    if create_directories {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut staging = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    if append && path.exists() {
        let mut original = fs::File::open(path).map_err(|e| e.to_string())?;
        std::io::copy(&mut original, &mut staging).map_err(|e| e.to_string())?;
    }
    staging
        .write_all(bytes)
        .and_then(|_| staging.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    staging.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

fn delta(before: &Fields, after: &Fields) -> Fields {
    let mut updates = Fields::new();
    for key in before.keys() {
        if !after.keys().any(|k| k.eq_ignore_ascii_case(key)) {
            updates.insert(key.clone(), vec![]);
        }
    }
    for (key, values) in after {
        if before.get(key) != Some(values) {
            updates.insert(key.clone(), values.clone());
        }
    }
    updates
}

fn state_path(state: &ActionState) -> Result<PathBuf, String> {
    state
        .context
        .get("_PATH")
        .and_then(|v| v.first())
        .map(PathBuf::from)
        .ok_or_else(|| "Action state has no file path".into())
}
/// The file extension of a cover image, from its MIME type or its bytes.
fn cover_extension(mime: &str, bytes: &[u8]) -> &'static str {
    match mime.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        "image/tiff" => "tif",
        _ => match image::guess_format(bytes) {
            Ok(image::ImageFormat::Jpeg) => "jpg",
            Ok(image::ImageFormat::Png) => "png",
            Ok(image::ImageFormat::Gif) => "gif",
            Ok(image::ImageFormat::Bmp) => "bmp",
            Ok(image::ImageFormat::WebP) => "webp",
            Ok(image::ImageFormat::Tiff) => "tif",
            _ => "img",
        },
    }
}

/// Export Cover appends the image type's extension to the evaluated name
/// (`Vol. 2` → `Vol. 2.jpg`). A name already ending in that extension is
/// kept, and another image extension is replaced (`cover.jpg` holding PNG
/// data → `cover.png`).
fn cover_file_target(mut target: PathBuf, mime: &str, bytes: &[u8]) -> PathBuf {
    let extension = cover_extension(mime, bytes);
    let current = target
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    match current.as_deref() {
        Some(current)
            if current == extension
                || (extension == "jpg" && current == "jpeg")
                || (extension == "tif" && current == "tiff") => {}
        Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "img") => {
            target.set_extension(extension);
        }
        _ => {
            let mut name = target.file_name().unwrap_or_default().to_os_string();
            name.push(".");
            name.push(extension);
            target.set_file_name(name);
        }
    }
    target
}

/// Turn a new file or folder name from an action into a relative path the way
/// Tag → Filename does. In a format string `\` separates folders on every
/// platform; text guessed from values has its separators and invalid
/// characters replaced. Edited names keep their characters: on macOS and Linux
/// a `\` already in a filename must not become a folder.
fn action_name(name: &str, origin: NameOrigin, separator: &str) -> String {
    match origin {
        NameOrigin::Value => tag_core::filename_value(name, separator),
        NameOrigin::Template if !cfg!(windows) => name.replace('\\', "/"),
        NameOrigin::Template | NameOrigin::Edited => name.to_owned(),
    }
}

fn relative_to_file(path: &str, source: &Path) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        source.parent().unwrap_or(Path::new(".")).join(path)
    }
}

struct NativeHost {
    settings: EvaluationSettings,
    directories: Vec<PathChange>,
    exported_covers: HashSet<(String, [u8; 32])>,
    identities: Vec<String>,
    initial_fields: Vec<Fields>,
    inputs: Vec<PathBuf>,
    /// Playlists and export files written, for opening after export.
    outputs: Vec<String>,
}
impl NativeHost {
    fn resolve(&self, path: &Path) -> PathBuf {
        let mut current = path.to_path_buf();
        for change in &self.directories {
            if let Ok(suffix) = current.strip_prefix(&change.from) {
                current = Path::new(&change.to).join(suffix);
            }
        }
        current
    }
    fn synchronize(&self, state: &mut ActionState) -> Result<(), String> {
        let path = self.resolve(&state_path(state)?);
        state
            .context
            .insert("_PATH".into(), vec![path.to_string_lossy().into_owned()]);
        Ok(())
    }
    fn flush(&self, state: &mut ActionState) -> Result<AudioEntry, String> {
        self.synchronize(state)?;
        let path = state_path(state)?;
        let original = crate::settings::read_summary(&path)?;
        let updates = delta(&original.fields, &state.fields);
        let entry = if updates.is_empty() {
            original
        } else {
            crate::settings::write_summary(&path, &updates)?
        };
        let counter = state
            .context
            .get("_COUNTER")
            .and_then(|v| v.first())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        state.fields = entry.fields.clone();
        state.context = technical_fields(&entry, counter);
        Ok(entry)
    }
    fn identity(&self, state: &ActionState) -> (String, bool) {
        let index = state
            .context
            .get("_COUNTER")
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1)
            .saturating_sub(1);
        let identity = self.identities.get(index).cloned().unwrap_or_default();
        let unchanged = self.initial_fields.get(index) == Some(&state.fields)
            && state_path(state).is_ok_and(|p| p == Path::new(&identity));
        (identity, unchanged)
    }
    fn protect_output(&self, target: &Path, states: &[ActionState]) -> Result<(), String> {
        let target = normalized_output(target)?;
        let protected = self.inputs.iter().cloned().chain(
            states
                .iter()
                .map(state_path)
                .collect::<Result<Vec<_>, _>>()?,
        );
        for source in protected {
            let source = self.resolve(&source);
            let source = normalized_output(&source)?;
            if source == target {
                return Err("Export output cannot replace an input audio file".into());
            }
        }
        Ok(())
    }
}
impl NativeHost {
    /// Technical fields of the file's current path, for format strings.
    fn path_context(&self, state: &ActionState) -> Result<Fields, String> {
        let mut context = state.context.clone();
        let path = self.resolve(&state_path(state)?);
        context.insert("_PATH".into(), vec![path.to_string_lossy().into_owned()]);
        context.insert(
            "_FILENAME_EXT".into(),
            vec![path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()],
        );
        context.insert(
            "_FILENAME".into(),
            vec![path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()],
        );
        context.insert(
            "_DIRECTORY".into(),
            vec![path
                .parent()
                .and_then(Path::file_name)
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()],
        );
        let parent = path.parent().unwrap_or(Path::new(""));
        context.insert(
            "_FOLDERPATH".into(),
            vec![format!("{}{}", parent.display(), std::path::MAIN_SEPARATOR)],
        );
        context.insert(
            "_PARENT_DIRECTORY".into(),
            vec![parent
                .parent()
                .and_then(Path::file_name)
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()],
        );
        Ok(context)
    }
}
impl ActionHost for NativeHost {
    fn evaluate(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        let (identity, unchanged) = self.identity(state);
        let context = self.path_context(state)?;
        self.settings
            .evaluate(template, &state.fields, &context, &identity, unchanged)
    }
    fn evaluate_filename(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        let (identity, unchanged) = self.identity(state);
        let context = self.path_context(state)?;
        self.settings
            .evaluate_filename(template, &state.fields, &context, &identity, unchanged)
    }
    fn execute(&mut self, effect: &ActionEffect, state: &mut ActionState) -> Result<(), String> {
        self.flush(state)?;
        let path = state_path(state)?;
        match effect {
            ActionEffect::RenameFile {
                filename,
                includes_extension,
                origin,
            } => {
                let filename = action_name(filename, *origin, &self.settings.folder_separator);
                let mut target = relative_to_file(&filename, &path);
                if !includes_extension {
                    if let Some(extension) = path.extension() {
                        let dotted = format!(".{}", extension.to_string_lossy());
                        // A generated name that already spells the extension keeps one.
                        if *origin == NameOrigin::Edited
                            || !tag_core::ends_with_extension(&filename, &dotted)
                        {
                            let mut name = target.into_os_string();
                            name.push(".");
                            name.push(extension);
                            target = PathBuf::from(name);
                        }
                    }
                }
                tag_core::operations::rename_no_clobber(&path, &target)?;
                let entry = crate::settings::read_summary(&target)?;
                let index: usize = state
                    .context
                    .get("_COUNTER")
                    .and_then(|v| v.first())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                if let Some(input) = self.inputs.get_mut(index.saturating_sub(1)) {
                    *input = target.clone();
                }
                state.context = technical_fields(&entry, index);
            }
            ActionEffect::RenameDirectory { directory, origin } => {
                let directory = action_name(directory, *origin, &self.settings.folder_separator);
                let source = path.parent().ok_or("File has no parent directory")?;
                let target = if Path::new(&directory).is_absolute() {
                    PathBuf::from(&directory)
                } else {
                    source.parent().unwrap_or(Path::new(".")).join(&directory)
                };
                if source != target {
                    if target.starts_with(source) {
                        return Err("Cannot move a directory inside itself".into());
                    }
                    tag_core::operations::rename_directory_no_clobber(source, &target)?;
                    self.directories.push(PathChange {
                        from: source.to_string_lossy().into_owned(),
                        to: target.to_string_lossy().into_owned(),
                    });
                }
                self.synchronize(state)?;
            }
            ActionEffect::RemoveCovers => {
                tag_core::remove_all_covers_with_options(&path, &crate::settings::tag_options())?;
            }
            ActionEffect::ImportCover {
                filename,
                picture_type,
                delete_existing,
            } => {
                tag_core::import_cover_with_options(
                    &path,
                    &relative_to_file(filename, &path),
                    picture_type.as_deref(),
                    *delete_existing,
                    &crate::settings::tag_options(),
                )?;
            }
            ActionEffect::ExportCover {
                filename,
                picture_type,
                export_duplicates,
            } => {
                let entry = crate::settings::read_full(&path)?;
                let filter = picture_type
                    .as_deref()
                    .map(tag_core::picture_type_from_name)
                    .transpose()?;
                let mut count = 0;
                for cover in &entry.covers {
                    if filter.is_some_and(|f| format!("{f:?}") != cover.picture_type) {
                        continue;
                    }
                    let bytes = STANDARD
                        .decode(&cover.data_base64)
                        .map_err(|e| e.to_string())?;
                    let digest: [u8; 32] = Sha256::digest(&bytes).into();
                    let mut target =
                        cover_file_target(relative_to_file(filename, &path), &cover.mime, &bytes);
                    // Duplicate identity is the requested destination plus
                    // content. Take it before per-file multi-cover numbering.
                    let identity = (normalized_output(&target)?, digest);
                    if !export_duplicates && self.exported_covers.contains(&identity) {
                        continue;
                    }
                    if count > 0 {
                        let stem = target.file_stem().unwrap_or_default().to_string_lossy();
                        let extension = target.extension().unwrap_or_default().to_string_lossy();
                        target.set_file_name(format!("{stem} ({count}).{extension}"));
                    }
                    self.protect_output(&target, std::slice::from_ref(state))?;
                    // Cover export never overwrites existing files implicitly.
                    let parent = target.parent().unwrap_or(Path::new("."));
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    let mut staging =
                        tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
                    staging
                        .write_all(&bytes)
                        .and_then(|_| staging.as_file().sync_all())
                        .map_err(|e| e.to_string())?;
                    staging
                        .persist_noclobber(&target)
                        .map_err(|e| e.error.to_string())?;
                    self.exported_covers.insert(identity);
                    count += 1;
                }
            }
            ActionEffect::AdjustCover {
                format,
                quality,
                resize,
            } => {
                tag_core::adjust_covers_with_options(
                    &path,
                    *format,
                    *quality,
                    resize.as_ref(),
                    &crate::settings::tag_options(),
                )?;
            }
            ActionEffect::SetCoverProperties {
                picture_type,
                description,
                only_picture_type,
            } => {
                tag_core::set_cover_properties_with_options(
                    &path,
                    picture_type.as_deref(),
                    description.as_deref(),
                    only_picture_type.as_deref(),
                    &crate::settings::tag_options(),
                )?;
            }
            ActionEffect::ImportTextFile { field, filename } => {
                let bytes =
                    fs::read(relative_to_file(filename, &path)).map_err(|e| e.to_string())?;
                // A BOM decides; text without one is UTF-8 when valid and
                // otherwise ANSI (the system codepage, Windows-1252 without one).
                let text = tag_core::decode_export_template(
                    &bytes,
                    self.settings.ansi_codepage().as_deref(),
                )
                .map_err(|e| format!("Cannot decode the text file: {e}"))?;
                state.fields.insert(field.clone(), vec![text]);
                self.flush(state)?;
            }
        }
        // Refresh properties without losing imported data needed by later actions.
        let entry = crate::settings::read_summary(&self.resolve(&state_path(state)?))?;
        let index = state
            .context
            .get("_COUNTER")
            .and_then(|v| v.first())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        state.context = technical_fields(&entry, index);
        Ok(())
    }
    fn execute_batch(
        &mut self,
        effect: &BatchActionEffect,
        states: &mut [ActionState],
    ) -> Result<(), String> {
        for state in states.iter_mut() {
            self.flush(state)?;
        }
        match effect {
            BatchActionEffect::GeneratePlaylists {
                filename_template,
                partition_template,
            } => {
                let mut groups: BTreeMap<String, Vec<ActionState>> = BTreeMap::new();
                for state in states.iter() {
                    let key = self.evaluate(partition_template, state)?;
                    groups.entry(key).or_default().push(state.clone());
                }
                let options = crate::playlists::PlaylistOptions::current();
                let mut write_options = options.write_options();
                write_options.root = crate::playlists::working_root(None, &self.settings);
                let mut outputs = Vec::new();
                let mut destinations = HashSet::new();
                for group in groups.values() {
                    let Some(first) = group.first() else { continue };
                    let name = self.evaluate(filename_template, first)?;
                    let target =
                        normalized_output_path(&relative_to_file(&name, &state_path(first)?))?;
                    self.protect_output(&target, states)?;
                    if !destinations.insert(normalized_output(&target)?) {
                        return Err(format!(
                            "Multiple playlist partitions use {}",
                            target.display()
                        ));
                    }
                    let mut items = Vec::with_capacity(group.len());
                    for state in group {
                        let path = state_path(state)?;
                        let entry = crate::settings::read_summary(&path)?;
                        let title = if options.extended {
                            self.evaluate(&options.extinf_format, state)?
                        } else {
                            String::new()
                        };
                        items.push(tag_core::playlist::PlaylistItem {
                            path,
                            duration: entry.duration,
                            title,
                        });
                    }
                    let bytes =
                        tag_core::playlist::render_playlist(&target, &items, &write_options)?;
                    outputs.push((target, bytes));
                }
                for (target, bytes) in outputs {
                    crate::playlists::write(&target, &bytes, &options)?;
                    self.outputs.push(target.to_string_lossy().into_owned());
                }
                Ok(())
            }
            BatchActionEffect::Export {
                template,
                filename_template,
                append,
                partition_template,
            } => {
                let saved = if !self.settings.export_template_inline
                    && !self.settings.app_data_directory.is_empty()
                {
                    crate::exports::saved_preferences(
                        &Path::new(&self.settings.app_data_directory).join("exports.json"),
                    )?
                } else {
                    None
                };
                let (write_bom, create_directories) = crate::exports::current_output_options(
                    &saved.unwrap_or(crate::exports::Preferences {
                        write_bom: self.settings.export_write_bom,
                        create_directories: self.settings.export_create_directories,
                        ..Default::default()
                    }),
                );
                let template = if self.settings.export_template_inline {
                    template.clone()
                } else {
                    resolve_export_template(template, &self.settings)?
                };
                let mut rows = Vec::with_capacity(states.len());
                for state in states.iter() {
                    let path = state_path(state)?;
                    let entry = crate::settings::read_summary(&path)?;
                    let mut fields = state.fields.clone();
                    fields.extend(state.context.clone());
                    let (identity, unchanged) = self.identity(state);
                    rows.push(tag_core::export::ExportRow {
                        fields: state.fields.clone(),
                        context_fields: state.context.clone(),
                        path: entry.path,
                        duration: entry.duration,
                        size: entry.size,
                        columns: Some(self.settings.column_values(
                            &fields,
                            &state.fields,
                            &identity,
                            unchanged,
                        )?),
                    });
                }
                let options = tag_core::export::ExportOptions {
                    filename_override: (!filename_template.trim().is_empty())
                        .then(|| filename_template.clone()),
                    base_directory: states
                        .first()
                        .map(state_path)
                        .transpose()?
                        .and_then(|p| p.parent().map(|p| p.to_string_lossy().into_owned()))
                        .unwrap_or_else(|| ".".into()),
                    partition: partition_template.clone(),
                    app_directory: Some(self.settings.app_directory.clone()),
                    app_data_directory: Some(self.settings.app_data_directory.clone()),
                    ansi_codepage: self.settings.ansi_codepage(),
                    timestamp_seconds: Some(self.settings.timestamp_seconds),
                    bom: write_bom,
                    ..Default::default()
                };
                let hashes = tag_core::export::requested_hash_fields(&template, &options)?;
                for row in &mut rows {
                    row.context_fields
                        .extend(crate::file_hashes::compute(Path::new(&row.path), &hashes)?);
                }
                let documents = tag_core::export::render_export(&template, &rows, &options)?;
                // Render, encode and protect the whole batch before writing any
                // destination, so a later partition cannot overwrite input audio.
                let mut outputs = Vec::with_capacity(documents.len());
                let mut seen = HashSet::new();
                for mut document in documents {
                    let target = PathBuf::from(&document.filename);
                    if !create_directories && !target.parent().unwrap_or(Path::new(".")).is_dir() {
                        return Err(format!(
                            "Export directory does not exist: {}",
                            target.display()
                        ));
                    }
                    self.protect_output(&target, states)?;
                    let normalized = normalized_output(&target)?;
                    if !seen.insert(normalized) {
                        return Err(format!(
                            "Multiple export partitions use {}",
                            target.display()
                        ));
                    }
                    if *append && fs::metadata(&target).is_ok_and(|m| m.len() > 0) {
                        document.bom = false;
                    }
                    let bytes = tag_core::export::encode_export_document(
                        &document,
                        options.ansi_codepage.as_deref(),
                    )?;
                    outputs.push((target, bytes));
                }
                for (target, bytes) in outputs {
                    atomic_output_with_directories(&target, &bytes, *append, create_directories)?;
                    self.outputs.push(target.to_string_lossy().into_owned());
                }
                Ok(())
            }
        }
    }
}

use std::collections::BTreeMap;

pub(crate) fn normalized_output(path: &Path) -> Result<String, String> {
    let value = normalized_output_path(path)?.to_string_lossy().into_owned();
    Ok(if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    })
}

/// Like [`normalized_output_path`] for a rename destination: when the final
/// name already exists as a plain entry (on a case-insensitive volume, the
/// source itself in another case), keep the requested spelling so a case-only
/// rename is not folded back to the stored name.
pub(crate) fn normalized_rename_target(path: &Path) -> Result<PathBuf, String> {
    let normalized = normalized_output_path(path)?;
    match (path.file_name(), fs::symlink_metadata(path)) {
        (Some(name), Ok(metadata)) if !metadata.file_type().is_symlink() => {
            Ok(normalized.with_file_name(name))
        }
        _ => Ok(normalized),
    }
}

/// Resolve destination aliases without creating parents or folding the case of
/// the path that a native file operation will use.
pub(crate) fn normalized_output_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut resolved = PathBuf::new();
    // Resolve existing symlinks before processing '..', while normalizing
    // nonexistent components without creating them. A missing/../audio alias
    // must compare equal to audio before the output parent is created.
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            // A Windows drive/verbatim prefix is incomplete until its root is
            // appended. Querying `D:` as a filesystem object can fail with
            // ERROR_INVALID_FUNCTION; only inspect complete normal components.
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                resolved.push(component.as_os_str());
            }
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            std::path::Component::Normal(_) => {
                resolved.push(component.as_os_str());
                match fs::symlink_metadata(&resolved) {
                    Ok(_) => {
                        resolved = resolved.canonicalize().map_err(|e| e.to_string())?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(format!(
                            "Cannot inspect output path {}: {error}",
                            resolved.display()
                        ))
                    }
                }
            }
        }
    }
    Ok(resolved)
}

fn resolve_export_template(source: &str, settings: &EvaluationSettings) -> Result<String, String> {
    // Action configurations traditionally reference an .mte name or path. The
    // editor also accepts a complete scripted configuration directly.
    if source.contains(['$', '%', '\r', '\n']) {
        return Ok(source.to_owned());
    }
    let requested = Path::new(source);
    let mut candidates = vec![requested.to_path_buf()];
    if !requested.is_absolute() {
        let directory = Path::new(&settings.app_data_directory).join("export");
        candidates.push(directory.join(requested));
        if requested.extension().is_none() {
            candidates.push(directory.join(format!("{source}.mte")));
        }
    }
    if let Some(path) = candidates.into_iter().find(|p| p.is_file()) {
        return tag_core::export::decode_export_template(
            &fs::read(&path).map_err(|e| e.to_string())?,
            settings.ansi_codepage().as_deref(),
        );
    }
    let library_path = Path::new(&settings.app_data_directory).join("exports.json");
    if !settings.app_data_directory.is_empty() && library_path.is_file() {
        let library = crate::exports::load(&library_path)?;
        if let Some(configuration) = library
            .configurations
            .into_iter()
            .find(|c| c.name.eq_ignore_ascii_case(source) || c.id == source)
        {
            return Ok(configuration.template);
        }
    }
    Err(format!("Export configuration not found: {source}"))
}

pub fn execute(
    paths: &[String],
    groups: &[ActionGroup],
    settings: EvaluationSettings,
) -> ActionResult {
    let mut result = ScanResult::default();
    let mut states = Vec::new();
    let mut initial = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        if !seen.insert(path.clone()) {
            continue;
        }
        match crate::settings::read_summary(Path::new(path)) {
            Ok(entry) => {
                initial.push(entry.path.clone());
                states.push(ActionState {
                    fields: entry.fields.clone(),
                    context: technical_fields(&entry, states.len() + 1),
                });
            }
            Err(error) => result.errors.push(FileError {
                path: path.clone(),
                error,
            }),
        }
    }
    let mut host = NativeHost {
        settings,
        directories: Vec::new(),
        exported_covers: HashSet::new(),
        identities: initial.clone(),
        inputs: initial.iter().map(PathBuf::from).collect(),
        initial_fields: states.iter().map(|s| s.fields.clone()).collect(),
        outputs: Vec::new(),
    };
    let mut failed = HashSet::new();
    let mut preflight = None;
    for group in groups.iter().filter(|g| g.enabled) {
        for action in &group.actions {
            if let Err(error) = tag_core::actions::validate_action(action) {
                preflight = Some(format!("{}: {error}", group.name));
                break;
            }
        }
        if preflight.is_some() {
            break;
        }
    }
    if let Some(error) = preflight {
        for (index, path) in initial.iter().enumerate() {
            failed.insert(index);
            result.errors.push(FileError {
                path: path.clone(),
                error: error.clone(),
            });
        }
    } else {
        for group in groups.iter().filter(|g| g.enabled) {
            let mut deferred = Vec::new();
            for (action_index, action) in group.actions.iter().enumerate() {
                let batch = match action {
                    ActionSpec::Export {
                        template,
                        filename_template,
                        append,
                        partition_template,
                    } => Some(BatchActionEffect::Export {
                        template: template.clone(),
                        filename_template: filename_template.clone(),
                        append: *append,
                        partition_template: partition_template.clone(),
                    }),
                    ActionSpec::GeneratePlaylists {
                        filename_template,
                        partition_template,
                    } => Some(BatchActionEffect::GeneratePlaylists {
                        filename_template: filename_template.clone(),
                        partition_template: partition_template.clone(),
                    }),
                    _ => None,
                };
                if let Some(batch) = batch {
                    deferred.push(batch);
                    continue;
                }
                for (index, state) in states.iter_mut().enumerate() {
                    if failed.contains(&index) {
                        continue;
                    }
                    let applied = tag_core::actions::apply_action_spec_with_modes(
                        state,
                        action,
                        &mut |template, state, mode| match mode {
                            TemplateMode::Text => host.evaluate(template, state),
                            TemplateMode::Filename => host.evaluate_filename(template, state),
                        },
                    );
                    let outcome = applied.and_then(|effects| {
                        for effect in effects {
                            host.execute(&effect, state)?;
                        }
                        Ok(())
                    });
                    if let Err(error) = outcome {
                        failed.insert(index);
                        let path = host.resolve(
                            &state_path(state).unwrap_or_else(|_| PathBuf::from(&initial[index])),
                        );
                        result.errors.push(FileError {
                            path: path.to_string_lossy().into_owned(),
                            error: format!("{} / action {}: {error}", group.name, action_index + 1),
                        });
                    }
                }
            }
            // Persist each successful file before deferred exports see final group values.
            for (index, state) in states.iter_mut().enumerate() {
                if failed.contains(&index) {
                    continue;
                }
                if let Err(error) = host.flush(state) {
                    failed.insert(index);
                    result.errors.push(FileError {
                        path: initial[index].clone(),
                        error,
                    });
                }
            }
            let mut complete: Vec<_> = states
                .iter()
                .enumerate()
                .filter(|(index, _)| !failed.contains(index))
                .map(|(_, s)| s.clone())
                .collect();
            for effect in deferred {
                if let Err(error) = host.execute_batch(&effect, &mut complete) {
                    result.errors.push(FileError {
                        path: format!("Action group {}", group.name),
                        error,
                    });
                }
            }
        }
    }
    let mut renamed = Vec::new();
    for (index, state) in states.iter_mut().enumerate() {
        let path =
            host.resolve(&state_path(state).unwrap_or_else(|_| PathBuf::from(&initial[index])));
        if path != Path::new(&initial[index]) {
            renamed.push(PathChange {
                from: initial[index].clone(),
                to: path.to_string_lossy().into_owned(),
            });
        }
        if !failed.contains(&index) {
            match crate::settings::read_summary(&path) {
                Ok(entry) => result.entries.push(entry),
                Err(error) => result.errors.push(FileError {
                    path: path.to_string_lossy().into_owned(),
                    error,
                }),
            }
        }
    }
    ActionResult {
        result,
        renamed,
        moved_directories: host.directories,
        outputs: host.outputs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::actions::{ActionSpec, CaseMode};

    #[test]
    fn actions_can_use_the_export_library_display_name() {
        let directory = tempfile::tempdir().unwrap();
        let mut library = crate::exports::Library::default();
        library.configurations[0].name = "My export".into();
        crate::exports::save(&directory.path().join("exports.json"), &library).unwrap();
        let settings = EvaluationSettings {
            app_data_directory: directory.path().to_string_lossy().into_owned(),
            ..Default::default()
        };
        assert_eq!(
            resolve_export_template("My export", &settings).unwrap(),
            library.configurations[0].template
        );
    }

    #[test]
    fn export_actions_honor_saved_global_bom_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let audio = fixture(directory.path(), "source.wav");
        let output = directory.path().join("action.txt");
        let mut library = crate::exports::Library::default();
        library.configurations[0].name = "Saved export".into();
        library.preferences.write_bom = true;
        crate::exports::save(&directory.path().join("exports.json"), &library).unwrap();
        let group = ActionGroup {
            name: "Export".into(),
            enabled: true,
            actions: vec![ActionSpec::Export {
                template: "Saved export".into(),
                filename_template: output.to_string_lossy().into_owned(),
                append: false,
                partition_template: None,
            }],
        };
        let result = execute(
            &[audio.to_string_lossy().into_owned()],
            &[group],
            EvaluationSettings {
                app_data_directory: directory.path().to_string_lossy().into_owned(),
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(fs::read(output).unwrap().starts_with(b"\xef\xbb\xbf"));
    }

    #[test]
    fn standalone_export_accepts_plain_text_templates() {
        let directory = tempfile::tempdir().unwrap();
        let audio = fixture(directory.path(), "source.wav");
        let output = directory.path().join("plain.txt");
        let group = ActionGroup {
            name: "Plain".into(),
            enabled: true,
            actions: vec![ActionSpec::Export {
                template: "Plain text".into(),
                filename_template: output.to_string_lossy().into_owned(),
                append: false,
                partition_template: None,
            }],
        };
        let result = execute(
            &[audio.to_string_lossy().into_owned()],
            &[group],
            EvaluationSettings {
                export_template_inline: true,
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read_to_string(output).unwrap(), "Plain text");
    }

    #[test]
    fn standalone_export_preferences_control_directories_bom_and_append_without_editing_audio() {
        let directory = tempfile::tempdir().unwrap();
        let audio = fixture(directory.path(), "source.wav");
        let before = fs::read(&audio).unwrap();
        let output = directory.path().join("reports/report.txt");
        let group = |append| ActionGroup {
            name: "Standalone export".into(),
            enabled: true,
            actions: vec![ActionSpec::Export {
                template: "$filename(txt,utf-8)$loop(1)%_filename_ext%$char(10)$loopend()".into(),
                filename_template: output.to_string_lossy().into_owned(),
                append,
                partition_template: None,
            }],
        };
        let paths = [audio.to_string_lossy().into_owned()];
        let mut settings = EvaluationSettings {
            export_write_bom: true,
            export_create_directories: false,
            ..Default::default()
        };
        let result = execute(&paths, &[group(false)], settings.clone());
        assert!(!result.errors.is_empty());
        assert!(!output.parent().unwrap().exists());
        settings.export_create_directories = true;
        assert!(execute(&paths, &[group(false)], settings.clone())
            .errors
            .is_empty());
        assert_eq!(
            fs::read(&output).unwrap(),
            [b"\xef\xbb\xbf".as_slice(), b"source.wav\n"].concat()
        );
        assert!(execute(&paths, &[group(true)], settings).errors.is_empty());
        assert_eq!(
            fs::read(&output).unwrap(),
            [b"\xef\xbb\xbf".as_slice(), b"source.wav\nsource.wav\n"].concat()
        );
        assert_eq!(fs::read(audio).unwrap(), before);
    }
    #[test]
    fn output_normalization_accepts_existing_roots_and_missing_children() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("new/../report.txt");
        let expected = dir
            .path()
            .canonicalize()
            .unwrap()
            .join("report.txt")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            normalized_output(&source).unwrap(),
            if cfg!(windows) {
                expected.to_lowercase()
            } else {
                expected
            }
        );
        assert!(!dir.path().join("new").exists());
    }
    fn fixture(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let audio = vec![0u8; 8820];
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36 + audio.len() as u32).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(44100u32.to_le_bytes());
        bytes.extend(88200u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((audio.len() as u32).to_le_bytes());
        bytes.extend(audio);
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn normalized_output_path_preserves_destination_case_without_creating_parents() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("Mixed Album");
        fs::create_dir(&parent).unwrap();
        let target = parent.join("Missing/../Chosen Track.wav");
        assert_eq!(
            normalized_output_path(&target).unwrap(),
            parent.canonicalize().unwrap().join("Chosen Track.wav")
        );
        assert!(!parent.join("Missing").exists());
        assert!(!parent.join("Chosen Track.wav").exists());
    }

    fn covered_fixture(dir: &Path, name: &str, repeated_image: bool) -> (PathBuf, Vec<u8>) {
        let path = fixture(dir, name);
        let png = STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap();
        let image = dir.join("embedded.png");
        fs::write(&image, &png).unwrap();
        tag_core::write_fields_summary(
            &path,
            &Fields::from([
                ("TITLE".into(), vec![name.into()]),
                ("Unmanaged_Field".into(), vec!["one".into(), "two".into()]),
            ]),
        )
        .unwrap();
        tag_core::add_cover_summary(&path, &image).unwrap();
        if repeated_image {
            tag_core::set_cover_properties_summary(&path, Some("CoverBack"), Some("Back"), None)
                .unwrap();
            tag_core::add_cover_summary(&path, &image).unwrap();
            assert_eq!(tag_core::read_entry_summary(&path).unwrap().covers.len(), 2);
        }
        (path, png)
    }

    #[test]
    fn import_cover_sets_type_and_deletes_existing_only_for_a_readable_image() {
        let dir = tempfile::tempdir().unwrap();
        let (path, png) = covered_fixture(dir.path(), "a.wav", false);
        let paths = [path.to_string_lossy().into_owned()];
        let import = |picture_type: Option<&str>, delete_existing: bool, image: &str| ActionGroup {
            name: "Import art".into(),
            enabled: true,
            actions: vec![ActionSpec::ImportCover {
                filename_template: image.into(),
                picture_type: picture_type.map(str::to_owned),
                delete_existing,
            }],
        };
        let covers = || tag_core::read_entry(&path).unwrap().covers;
        let result = execute(
            &paths,
            &[import(Some("CoverBack"), true, "embedded.png")],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let after = covers();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].picture_type, "CoverBack");
        assert_eq!(STANDARD.decode(&after[0].data_base64).unwrap(), png);
        let result = execute(
            &paths,
            &[import(Some("Artist"), false, "embedded.png")],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            covers()
                .iter()
                .map(|c| c.picture_type.as_str())
                .collect::<Vec<_>>(),
            ["CoverBack", "Artist"]
        );
        let before = fs::read(&path).unwrap();
        let result = execute(
            &paths,
            &[import(None, true, "missing.png")],
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    #[test]
    fn exported_group_is_utf16_mta_that_imports_back() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Umlauts.mta");
        let group = ActionGroup {
            name: "Umlauts".into(),
            enabled: true,
            actions: vec![ActionSpec::Replace {
                field: "_FILENAME".into(),
                from: "ä".into(),
                to: "ae".into(),
                whole_word: false,
                case_sensitive: true,
            }],
        };
        export_group(&target, &group).unwrap();
        let bytes = fs::read(&target).unwrap();
        assert_eq!(&bytes[..2], &[0xff, 0xfe]);
        let imported = import_group(&target, None).unwrap();
        assert!(imported.issues.is_empty());
        assert_eq!(imported.group, group);
        let art = ActionGroup {
            name: "Art".into(),
            enabled: true,
            actions: vec![ActionSpec::AdjustCover {
                format: tag_core::actions::CoverImageFormat::Png,
                quality: 90,
                resize: None,
            }],
        };
        export_group(&dir.path().join("Art.mta"), &art).unwrap();
        assert_eq!(
            import_group(&dir.path().join("Art.mta"), None)
                .unwrap()
                .group,
            art
        );
        // A setting .mta cannot store refuses the export without a file.
        assert!(export_group(
            &dir.path().join("lines.mta"),
            &ActionGroup {
                name: "Lines".into(),
                enabled: true,
                actions: vec![ActionSpec::FormatValue {
                    field: "COMMENT".into(),
                    template: "a\nb".into(),
                }],
            }
        )
        .is_err());
        assert!(!dir.path().join("lines.mta").exists());
    }

    #[test]
    fn export_cover_appends_or_replaces_the_image_extension() {
        let png = b"\x89PNG\r\n\x1a\n".as_slice();
        let target = |name: &str, mime: &str, bytes: &[u8]| {
            // Windows rebuilds the path with `\`; compare in one separator style.
            cover_file_target(PathBuf::from(name), mime, bytes)
                .to_string_lossy()
                .replace('\\', "/")
        };
        assert_eq!(target("art/Vol. 2", "image/jpeg", &[]), "art/Vol. 2.jpg");
        assert_eq!(target("cover", "image/png", &[]), "cover.png");
        assert_eq!(target("cover.jpg", "image/png", &[]), "cover.png");
        assert_eq!(target("cover.JPEG", "image/jpeg", &[]), "cover.JPEG");
        assert_eq!(target("cover.jpg", "image/jpeg", &[]), "cover.jpg");
        assert_eq!(target("front", "image/webp", &[]), "front.webp");
        assert_eq!(target("front", "image/bmp", &[]), "front.bmp");
        assert_eq!(
            target("front", "application/octet-stream", png),
            "front.png"
        );
        assert_eq!(target("front", "", b"????"), "front.img");
        assert_eq!(target("Mr. Jones", "image/gif", &[]), "Mr. Jones.gif");
    }

    fn cover_export_group(filename: &str, export_duplicates: bool) -> ActionGroup {
        ActionGroup {
            name: "Export art".into(),
            enabled: true,
            actions: vec![ActionSpec::ExportCover {
                filename_template: filename.into(),
                picture_type: None,
                export_duplicates,
            }],
        }
    }

    #[test]
    fn cover_export_same_bytes_with_distinct_evaluated_names_are_both_written() {
        let dir = tempfile::tempdir().unwrap();
        let (a, png) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let before = [fs::read(&a).unwrap(), fs::read(&b).unwrap()];
        let paths = [
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let result = execute(
            &paths,
            &[cover_export_group("art/%_filename%", false)],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.entries.len(), 2);
        assert_eq!(fs::read(dir.path().join("art/a.png")).unwrap(), png);
        assert_eq!(fs::read(dir.path().join("art/b.png")).unwrap(), png);
        for (path, bytes) in paths.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn cover_export_same_normalized_base_and_bytes_skip_before_index_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let (path, png) = covered_fixture(dir.path(), "multiple.wav", true);
        let before = fs::read(&path).unwrap();
        let mut group = cover_export_group("art/cover", false);
        group
            .actions
            .extend(cover_export_group("art/missing/../cover.png", false).actions);
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[group],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(dir.path().join("art/cover.png")).unwrap(), png);
        assert!(!dir.path().join("art/cover (1).png").exists());
        assert!(!dir.path().join("art/missing").exists());
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn cover_export_duplicates_true_writes_repeated_embedded_images() {
        let dir = tempfile::tempdir().unwrap();
        let (path, png) = covered_fixture(dir.path(), "multiple.wav", true);
        let before = fs::read(&path).unwrap();
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[cover_export_group("duplicates", true)],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(dir.path().join("duplicates.png")).unwrap(), png);
        assert_eq!(
            fs::read(dir.path().join("duplicates (1).png")).unwrap(),
            png
        );
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn cover_export_keeps_existing_files_and_rejects_input_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let (path, _) = covered_fixture(dir.path(), "input.wav", false);
        let before = fs::read(&path).unwrap();
        let output = dir.path().join("cover.png");
        fs::write(&output, b"Existing output").unwrap();
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[cover_export_group("cover", false)],
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read(&output).unwrap(), b"Existing output");
        // The image extension is always added, so a name aimed at the input
        // audio file writes a separate image next to it.
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[cover_export_group("missing/../input.wav", false)],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(dir.path().join("input.wav.png").is_file());
    }

    #[test]
    fn cover_export_duplicates_true_reports_cross_file_destination_collision() {
        let dir = tempfile::tempdir().unwrap();
        let (a, png) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let before = [fs::read(&a).unwrap(), fs::read(&b).unwrap()];
        let paths = [
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let result = execute(
            &paths,
            &[cover_export_group("shared", true)],
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(fs::read(dir.path().join("shared.png")).unwrap(), png);
        assert!(!dir.path().join("shared (1).png").exists());
        for (path, bytes) in paths.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn cover_export_repeated_destination_and_content_are_suppressed_across_files() {
        let dir = tempfile::tempdir().unwrap();
        let (a, png) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let before = [fs::read(&a).unwrap(), fs::read(&b).unwrap()];
        let paths = [
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let result = execute(
            &paths,
            &[cover_export_group("shared", false)],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.entries.len(), 2);
        assert_eq!(fs::read(dir.path().join("shared.png")).unwrap(), png);
        assert!(!dir.path().join("shared (1).png").exists());
        for (path, bytes) in paths.iter().zip(before) {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }

    #[test]
    fn saved_library_roundtrips_and_invalid_save_preserves_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings/actions.json");
        let mut library = ActionLibrary::default();
        library
            .add_group(ActionGroup {
                name: "Test".into(),
                enabled: true,
                actions: vec![ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "Value".into(),
                }],
            })
            .unwrap();
        library.save_selection_preset("Daily".into()).unwrap();
        save_library(&path, &library).unwrap();
        assert_eq!(load_library(&path).unwrap(), library);
        let before = fs::read(&path).unwrap();
        library.groups.push(library.groups[0].clone());
        assert!(save_library(&path, &library).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    #[test]
    fn ordered_import_format_and_filename_actions_write_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path(), "old.wav");
        fs::write(dir.path().join("comment.txt"), "Two lines\nfrom text").unwrap();
        let group = ActionGroup {
            name: "Workflow".into(),
            enabled: true,
            actions: vec![
                ActionSpec::ImportTextFile {
                    field: "COMMENT".into(),
                    filename_template: "comment.txt".into(),
                },
                ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "$left(%comment%,3)".into(),
                },
                ActionSpec::FormatValue {
                    field: "_FILENAME".into(),
                    template: "%title%".into(),
                },
            ],
        };
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[group],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.renamed.len(), 1);
        assert!(!path.exists());
        assert!(dir.path().join("Two.wav").exists());
        assert_eq!(result.entries[0].fields["TITLE"], ["Two"]);
        assert_eq!(
            result.entries[0].fields["COMMENT"],
            ["Two lines\nfrom text"]
        );
    }
    /// MP3 with an ID3v2.3 tag holding text fields, a comment, a private
    /// frame, a GEOB object and one chapter.
    fn mp3_with_unmapped_frames(path: &Path) {
        let frame = |id: &[u8; 4], data: &[u8]| {
            let mut bytes = id.to_vec();
            bytes.extend((data.len() as u32).to_be_bytes());
            bytes.extend([0, 0]);
            bytes.extend(data);
            bytes
        };
        let mut frames = Vec::new();
        frames.extend(frame(b"TIT2", b"\0Title"));
        frames.extend(frame(b"TPE1", b"\0Artist"));
        // A COMM frame with a description would be "COMMENT Note".
        frames.extend(frame(b"COMM", b"\0eng\0Text"));
        frames.extend(frame(b"PRIV", b"WM/MediaClassPrimaryID\0\x01\x02"));
        frames.extend(frame(
            b"GEOB",
            b"\0application/octet-stream\0f.bin\0Object\0\x09",
        ));
        let mut chapter = b"ch0\0".to_vec();
        chapter.extend(0u32.to_be_bytes());
        chapter.extend(500u32.to_be_bytes());
        chapter.extend(u32::MAX.to_be_bytes());
        chapter.extend(u32::MAX.to_be_bytes());
        chapter.extend(frame(b"TIT2", b"\0Intro"));
        frames.extend(frame(b"CHAP", &chapter));
        let size = frames.len() as u32;
        let mut bytes = b"ID3\x03\0\0".to_vec();
        bytes.extend(
            (0..4)
                .rev()
                .map(|shift| ((size >> (7 * shift)) & 0x7f) as u8),
        );
        bytes.extend(frames);
        for frame in 0..20u32 {
            bytes.extend([0xFF, 0xFB, 0x90, 0x64]);
            bytes.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
        }
        fs::write(path, bytes).unwrap();
    }

    /// Remove Fields Except removes every displayed field but, like Mp3tag,
    /// leaves frames without a field (left to Utils → Remove unsupported
    /// fields) and chapters (Utils → Strip chapters).
    #[test]
    fn remove_fields_except_keeps_frames_that_have_no_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mp3");
        mp3_with_unmapped_frames(&path);
        let before = crate::settings::read_summary(&path).unwrap();
        assert_eq!(before.technical["_ID3V2_UNKNOWN_FRAMES"], ["GEOB, PRIV"]);
        assert!(before.fields.contains_key("COMMENT"));
        assert_eq!(before.technical["_SUBSONG_COUNT"], ["1"]);
        let group = ActionGroup {
            name: "Keep title".into(),
            enabled: true,
            actions: vec![ActionSpec::RemoveFieldsExcept {
                fields: vec!["TITLE".into()],
            }],
        };
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[group],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let after = crate::settings::read_summary(&path).unwrap();
        assert_eq!(after.fields.keys().collect::<Vec<_>>(), ["TITLE"]);
        assert_eq!(after.technical["_ID3V2_UNKNOWN_FRAMES"], ["GEOB, PRIV"]);
        assert_eq!(after.technical["_SUBSONG_COUNT"], ["1"]);
    }

    #[test]
    fn import_text_file_reads_bom_utf8_and_ansi_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path(), "song.wav");
        let cases: [(&[u8], &str); 5] = [
            (b"\xef\xbb\xbfCaf\xc3\xa9", "Café"),
            (b"\xff\xfeC\x00a\x00f\x00\xe9\x00", "Café"),
            (b"\xfe\xff\x00C\x00a\x00f\x00\xe9", "Café"),
            (b"Caf\xc3\xa9 \xe2\x82\xac", "Café €"),
            // Windows-1252 text without a BOM is not valid UTF-8.
            (
                b"Caf\xe9 \x80 \x93quoted\x94",
                "Café € \u{201c}quoted\u{201d}",
            ),
        ];
        // Windows reads text without a BOM in its own ANSI codepage.
        let ansi_1252 = EvaluationSettings::default()
            .ansi_codepage()
            .is_none_or(|codepage| codepage == "1252");
        for (index, (bytes, expected)) in cases.into_iter().enumerate() {
            fs::write(dir.path().join("lyrics.txt"), bytes).unwrap();
            let group = ActionGroup {
                name: "Import".into(),
                enabled: true,
                actions: vec![ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: "lyrics.txt".into(),
                }],
            };
            let result = execute(
                &[path.to_string_lossy().into_owned()],
                &[group],
                EvaluationSettings::default(),
            );
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            if index == 4 && !ansi_1252 {
                continue;
            }
            assert_eq!(result.entries[0].fields["LYRICS"], [expected], "{bytes:?}");
        }
    }
    #[test]
    fn directory_moves_rebase_other_selected_files_before_later_actions() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("Album");
        fs::create_dir(&album).unwrap();
        let a = fixture(&album, "a.wav");
        let b = fixture(&album, "b.wav");
        let sibling = fixture(&album, "sibling.wav");
        let group = ActionGroup {
            name: "Move".into(),
            enabled: true,
            actions: vec![
                ActionSpec::FormatValue {
                    field: "_DIRECTORY".into(),
                    template: "NewAlbum".into(),
                },
                ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "%_directory%".into(),
                },
            ],
        };
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &[group],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.moved_directories.len(), 1);
        assert_eq!(result.renamed.len(), 2);
        assert!(!sibling.exists());
        assert!(dir.path().join("NewAlbum/sibling.wav").exists());
        for entry in &result.entries {
            assert_eq!(entry.fields["TITLE"], ["NewAlbum"]);
        }
    }
    #[test]
    fn filename_collision_reports_only_failed_file_and_returns_completed_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture(dir.path(), "a.wav");
        let b = fixture(dir.path(), "b.wav");
        let group = ActionGroup {
            name: "Collision".into(),
            enabled: true,
            actions: vec![ActionSpec::FormatValue {
                field: "_FILENAME".into(),
                template: "same".into(),
            }],
        };
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &[group],
            EvaluationSettings::default(),
        );
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.renamed.len(), 1);
        assert!(b.exists());
        assert!(dir.path().join("same.wav").exists());
    }
    #[test]
    fn export_runs_after_metadata_actions_and_appends_encoded_template_output() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture(dir.path(), "a.wav");
        let b = fixture(dir.path(), "b.wav");
        let export = ActionSpec::Export {
            template: "$filename(txt,utf-16)$loop(%title%)%title%$char(10)$loopend()".into(),
            filename_template: "%_folderpath%report.txt".into(),
            append: true,
            partition_template: None,
        };
        let group = ActionGroup {
            name: "Export".into(),
            enabled: true,
            actions: vec![
                export.clone(),
                ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "Updated".into(),
                },
            ],
        };
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let result = execute(&paths, &[group], EvaluationSettings::default());
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let output = dir.path().join("report.txt");
        let bytes = fs::read(&output).unwrap();
        let expected = "Updated\nUpdated\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(bytes, expected);
        let result = execute(
            &paths,
            &[ActionGroup {
                name: "Again".into(),
                enabled: true,
                actions: vec![export],
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(output).unwrap(), expected.repeat(2));
    }
    #[test]
    fn export_partition_collision_writes_nothing_and_audio_output_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture(dir.path(), "a.wav");
        let b = fixture(dir.path(), "b.wav");
        let paths = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        let mut export = ActionSpec::Export {
            template: "$loop(%_filename%)%_filename%$loopend()".into(),
            filename_template: "same.txt".into(),
            append: false,
            partition_template: Some("%_filename%".into()),
        };
        let run = |action| {
            execute(
                &paths,
                &[ActionGroup {
                    name: "Export".into(),
                    enabled: true,
                    actions: vec![action],
                }],
                EvaluationSettings::default(),
            )
        };
        let result = run(export.clone());
        assert!(!result.errors.is_empty());
        assert!(!dir.path().join("same.txt").exists());
        if let ActionSpec::Export {
            filename_template,
            partition_template,
            ..
        } = &mut export
        {
            *filename_template = a.to_string_lossy().into_owned();
            *partition_template = None;
        }
        let before = fs::read(&a).unwrap();
        let result = run(export);
        assert!(!result.errors.is_empty());
        assert_eq!(fs::read(a).unwrap(), before);
    }
    #[test]
    fn named_utf16_configuration_exports_actual_file_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let audio = fixture(dir.path(), "input.wav");
        let config_dir = dir.path().join("config/export");
        fs::create_dir_all(&config_dir).unwrap();
        let script = "$filename(txt,utf-8)$loop(%_filename%)%_md5%:%_crc%:[%_md5audio%]$loopend()";
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(script.encode_utf16().flat_map(u16::to_le_bytes));
        fs::write(config_dir.join("Checksums.mte"), bytes).unwrap();
        let action = ActionSpec::Export {
            template: "Checksums".into(),
            filename_template: "checksums.txt".into(),
            append: false,
            partition_template: None,
        };
        let result = execute(
            &[audio.to_string_lossy().into_owned()],
            &[ActionGroup {
                name: "Hashes".into(),
                enabled: true,
                actions: vec![action],
            }],
            EvaluationSettings {
                app_data_directory: dir.path().join("config").to_string_lossy().into_owned(),
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let hashes = crate::file_hashes::compute(&audio, &["_MD5".into(), "_CRC".into()]).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("checksums.txt")).unwrap(),
            // WAV has no audio MD5: the export still succeeds, with nothing.
            format!("{}:{}:", hashes["_MD5"][0], hashes["_CRC"][0])
        );
    }
    #[test]
    fn output_alias_through_missing_parent_never_replaces_input_audio() {
        let dir = tempfile::tempdir().unwrap();
        let audio = fixture(dir.path(), "input.wav");
        let original = fs::read(&audio).unwrap();
        let actions = [
            ActionSpec::GeneratePlaylists {
                filename_template: "missing/../input.wav".into(),
                partition_template: "".into(),
            },
            ActionSpec::Export {
                template: "$loop(%_filename%)%_filename%$loopend()".into(),
                filename_template: "missing/../input.wav".into(),
                append: false,
                partition_template: None,
            },
        ];
        for action in actions {
            let result = execute(
                &[audio.to_string_lossy().into_owned()],
                &[ActionGroup {
                    name: "Protect".into(),
                    enabled: true,
                    actions: vec![action],
                }],
                EvaluationSettings::default(),
            );
            assert!(
                !result.errors.is_empty(),
                "Expected output protection error"
            );
            assert_eq!(
                fs::read(&audio).unwrap(),
                original,
                "Export replaced input audio"
            );
        }
    }
    #[test]
    fn playlist_partition_collision_does_not_write_any_output() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture(dir.path(), "a.wav");
        let b = fixture(dir.path(), "b.wav");
        let action = ActionSpec::GeneratePlaylists {
            filename_template: "same.m3u8".into(),
            partition_template: "%_filename%".into(),
        };
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &[ActionGroup {
                name: "Playlist".into(),
                enabled: true,
                actions: vec![action],
            }],
            EvaluationSettings::default(),
        );
        assert!(!result.errors.is_empty());
        assert!(!dir.path().join("same.m3u8").exists());
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
    fn titled(dir: &Path, name: &str, title: &str) -> PathBuf {
        let path = fixture(dir, name);
        tag_core::write_fields(&path, &Fields::from([("TITLE".into(), vec![title.into()])]))
            .unwrap();
        path
    }
    fn run(path: &Path, actions: Vec<ActionSpec>, settings: EvaluationSettings) -> ActionResult {
        let group = ActionGroup {
            name: "Test".into(),
            enabled: true,
            actions,
        };
        execute(&[path.to_string_lossy().into_owned()], &[group], settings)
    }

    #[test]
    fn case_conversion_renames_files_and_folders_by_case_only() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("album");
        fs::create_dir(&album).unwrap();
        let path = fixture(&album, "track one.wav");
        let result = run(
            &path,
            vec![
                ActionSpec::CaseConversion {
                    field: "_FILENAME".into(),
                    mode: CaseMode::Upper,
                    word_separators: String::new(),
                },
                ActionSpec::CaseConversion {
                    field: "_DIRECTORY".into(),
                    mode: CaseMode::Mixed,
                    word_separators: String::new(),
                },
            ],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(dir.path()), ["Album"]);
        assert_eq!(names(&dir.path().join("Album")), ["TRACK ONE.wav"]);
    }

    #[test]
    fn format_value_filenames_follow_tag_to_filename_rules() {
        let dir = tempfile::tempdir().unwrap();
        let path = titled(dir.path(), "old.wav", "AC/DC: Live");
        let format = |template: &str| {
            vec![ActionSpec::FormatValue {
                field: "_FILENAME".into(),
                template: template.into(),
            }]
        };
        // Tag values lose separators and invalid characters; the extension is not doubled.
        let result = run(&path, format("%title%.WAV"), EvaluationSettings::default());
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(dir.path()), ["AC_DC_ Live.WAV"]);
        // A literal backslash creates a folder; the separator option applies to values.
        let settings = EvaluationSettings {
            folder_separator: "-".into(),
            ..Default::default()
        };
        let path = dir.path().join("AC_DC_ Live.WAV");
        let result = run(&path, format("Live\\%title%"), settings.clone());
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(&dir.path().join("Live")), ["AC-DC_ Live.WAV"]);
        // Guess Values treats the guessed name as a value, here with separators removed.
        let path = dir.path().join("Live/AC-DC_ Live.WAV");
        let result = run(
            &path,
            vec![ActionSpec::GuessValues {
                source: "x %title%".into(),
                pattern: "x %_filename%".into(),
            }],
            EvaluationSettings {
                folder_separator: String::new(),
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(&dir.path().join("Live")), ["ACDC_ Live.WAV"]);
    }

    #[test]
    fn format_value_directories_replace_separators_in_values() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("old");
        fs::create_dir(&album).unwrap();
        let path = titled(&album, "a.wav", "Rock\\Classic");
        let result = run(
            &path,
            vec![ActionSpec::FormatValue {
                field: "_DIRECTORY".into(),
                template: "%title%".into(),
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(dir.path()), ["Rock_Classic"]);
    }

    fn flac(dir: &Path) -> Option<PathBuf> {
        let path = dir.join("track.flac");
        let output = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-f", "lavfi", "-i"])
            .arg("sine=frequency=440:sample_rate=8000:duration=0.2")
            .arg(&path)
            .output()
            .ok()?;
        output.status.success().then_some(path)
    }

    #[test]
    fn field_name_case_conversion_rewrites_vorbis_comment_names() {
        let dir = tempfile::tempdir().unwrap();
        let Some(path) = flac(dir.path()) else {
            eprintln!("Skipping _FIELDNAME test: ffmpeg is required");
            return;
        };
        tag_core::write_fields(&path, &Fields::from([("MyField".into(), vec!["x".into()])]))
            .unwrap();
        let result = run(
            &path,
            vec![ActionSpec::CaseConversion {
                field: "_FIELDNAME".into(),
                mode: CaseMode::Upper,
                word_separators: String::new(),
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let fields = &tag_core::read_entry_summary(&path).unwrap().fields;
        assert_eq!(
            fields.get("MYFIELD"),
            Some(&vec!["x".to_string()]),
            "{fields:?}"
        );
        assert!(!fields.contains_key("MyField"));
        // Tags that map field names (ID3v2 here) refuse field-name actions.
        let wav = fixture(dir.path(), "a.wav");
        let result = run(
            &wav,
            vec![ActionSpec::CaseConversion {
                field: "_FIELDNAME".into(),
                mode: CaseMode::Upper,
                word_separators: String::new(),
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors[0].error.contains("APEv2 or VorbisComment"));
    }
}
