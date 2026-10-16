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
use tag_core::batch_errors::{push_error, BatchError, Detailed, ErrorCause};
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(example_library()),
        Err(e) => Err(e.to_string()),
    }
}
/// Groups offered before any action configuration was saved. They are
/// unchecked, listed under Examples, and written for OpenTag.
pub fn example_library() -> ActionLibrary {
    use tag_core::actions::CaseMode;
    let group = |name: &str, actions| ActionGroup {
        name: format!("Examples#{name}"),
        enabled: false,
        actions,
    };
    ActionLibrary {
        groups: vec![
            group(
                "Tidy spaces in tag fields",
                vec![
                    ActionSpec::RegexReplace {
                        field: "_TAG".into(),
                        pattern: "\\s{2,}".into(),
                        replacement: " ".into(),
                        case_sensitive: false,
                    },
                    ActionSpec::RegexReplace {
                        field: "_TAG".into(),
                        pattern: "^\\s+|\\s+$".into(),
                        replacement: String::new(),
                        case_sensitive: false,
                    },
                ],
            ),
            group(
                "Split artists at semicolons",
                vec![ActionSpec::SplitField {
                    field: "ARTIST".into(),
                    separator: ";".into(),
                }],
            ),
            group(
                "Capitalize titles and names",
                ["TITLE", "ARTIST", "ALBUMARTIST", "ALBUM"]
                    .into_iter()
                    .map(|field| ActionSpec::CaseConversion {
                        field: field.into(),
                        mode: CaseMode::Mixed,
                        word_separators: " ([-/".into(),
                    })
                    .collect(),
            ),
            group(
                "Name files 01 - Title",
                vec![ActionSpec::FormatValue {
                    field: "_FILENAME".into(),
                    template: "[$num(%track%,2) - ]%title%".into(),
                }],
            ),
            group(
                "Save front cover as folder image",
                vec![ActionSpec::ExportCover {
                    filename_template: "folder".into(),
                    picture_type: Some("CoverFront".into()),
                    export_duplicates: false,
                }],
            ),
        ],
        presets: Vec::new(),
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
#[cfg(test)]
pub fn import_group(path: &Path, legacy_encoding: Option<&str>) -> Result<MtaImport, String> {
    import_group_diagnostic(path, legacy_encoding)
        .map(|report| report.result)
        .map_err(|cause| cause.to_string())
}
pub fn import_group_diagnostic(
    path: &Path,
    legacy_encoding: Option<&str>,
) -> Result<tag_core::actions::MtaImportReport, ErrorCause> {
    let bytes = fs::read(path).map_err(ErrorCause::external)?;
    tag_core::actions::import_mta_diagnostic(
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        &bytes,
        legacy_encoding,
    )
    .map_err(ErrorCause::owned_text)
}
/// One file of a bulk `.mta` import.
#[derive(Serialize)]
pub struct GroupFileImport {
    /// The caller's provider URI, or an imported native child-file path.
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported: Option<MtaImport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
#[derive(Serialize)]
pub struct GroupFileImportReport {
    #[serde(flatten)]
    pub result: GroupFileImport,
    pub issue_diagnostics: Vec<tag_core::batch_errors::IndexedCause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_cause: Option<ErrorCause>,
}
#[cfg(test)]
pub fn import_groups(paths: &[PathBuf], legacy_encoding: Option<&str>) -> Vec<GroupFileImport> {
    import_groups_diagnostic(paths, legacy_encoding)
        .into_iter()
        .map(|report| report.result)
        .collect()
}
pub fn import_groups_diagnostic(
    paths: &[PathBuf],
    legacy_encoding: Option<&str>,
) -> Vec<GroupFileImportReport> {
    let outcome = |path: &Path, result: Result<tag_core::actions::MtaImportReport, ErrorCause>| {
        let (imported, error, issue_diagnostics, error_cause) = match result {
            Ok(report) => (Some(report.result), None, report.issue_diagnostics, None),
            Err(cause) => (None, Some(cause.to_string()), Vec::new(), Some(cause)),
        };
        GroupFileImportReport {
            result: GroupFileImport {
                path: path.to_string_lossy().into_owned(),
                imported,
                error,
            },
            issue_diagnostics,
            error_cause,
        }
    };
    let mut results = Vec::new();
    for path in paths {
        if !path.is_dir() {
            results.push(outcome(
                path,
                import_group_diagnostic(path, legacy_encoding),
            ));
            continue;
        }
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) => {
                results.push(outcome(path, Err(ErrorCause::external(error))));
                continue;
            }
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|file| {
                file.is_file()
                    && file
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("mta"))
            })
            .collect();
        files.sort_by_cached_key(|file| file.to_string_lossy().to_lowercase());
        files.sort_by(|a, b| {
            tag_core::natural_compare(
                &a.to_string_lossy().to_lowercase(),
                &b.to_string_lossy().to_lowercase(),
            )
        });
        if files.is_empty() {
            results.push(outcome(
                path,
                Err(ErrorCause::owned("The folder has no .mta files")),
            ));
        }
        for file in files {
            results.push(outcome(
                &file,
                import_group_diagnostic(&file, legacy_encoding),
            ));
        }
    }
    results
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
    atomic_output_with_directories_diagnostic(path, bytes, append, create_directories)
        .map_err(|error| error.to_string())
}

pub(crate) fn atomic_output_diagnostic(
    path: &Path,
    bytes: &[u8],
    append: bool,
) -> Result<(), ErrorCause> {
    atomic_output_with_directories_diagnostic(path, bytes, append, true)
}

fn atomic_output_with_directories_diagnostic(
    path: &Path,
    bytes: &[u8],
    append: bool,
    create_directories: bool,
) -> Result<(), ErrorCause> {
    let parent = path.parent().unwrap_or(Path::new("."));
    if create_directories {
        fs::create_dir_all(parent).map_err(ErrorCause::external)?;
    }
    let mut staging = tempfile::NamedTempFile::new_in(parent).map_err(ErrorCause::external)?;
    if append && path.exists() {
        let mut original = fs::File::open(path).map_err(ErrorCause::external)?;
        std::io::copy(&mut original, &mut staging).map_err(ErrorCause::external)?;
    }
    staging
        .write_all(bytes)
        .and_then(|_| staging.as_file().sync_all())
        .map_err(ErrorCause::external)?;
    staging
        .persist(path)
        .map_err(|e| ErrorCause::external(e.error))?;
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

/// Sanitize guessed filename values. Templates already have native separators
/// from AST rendering; edited names and path-placeholder characters stay intact.
fn action_name(name: &str, origin: NameOrigin, separator: &str) -> String {
    match origin {
        NameOrigin::Value => tag_core::filename_value(name, separator),
        NameOrigin::Template | NameOrigin::Edited => name.to_owned(),
    }
}

/// The files an export must not replace for one input: the file itself, or
/// for a chapter or cue sheet row its file and the audio it plays from (a
/// row path is no file name on Windows).
fn protected_files(path: &Path) -> Vec<PathBuf> {
    match path.to_str().and_then(tag_core::subsong_parts) {
        Some((file, _)) => {
            let mut files = vec![PathBuf::from(file)];
            files.extend(tag_core::subsong_file(path).ok().filter(|f| f != &files[0]));
            files
        }
        None => vec![path.to_path_buf()],
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

/// Resolve an already evaluated native cover, text or playlist path.
fn template_path(name: &str, source: &Path) -> PathBuf {
    relative_to_file(name, source)
}

struct NativeHost<'a> {
    settings: EvaluationSettings,
    directories: Vec<PathChange>,
    /// Requested name and digest, with the actual (possibly numbered) files
    /// still holding that image. Replaced output is no longer a duplicate.
    exported_covers: BTreeMap<(String, [u8; 32]), HashSet<String>>,
    /// Cover images written by this run, by normalized destination: the
    /// image's digest and the input file that wrote it.
    written_covers: BTreeMap<String, ([u8; 32], String)>,
    /// Outputs whose original entry/absence was captured before an actual write.
    captured_covers: HashSet<String>,
    /// Called before the first actual write to an image output, so undo can
    /// restore its previous entry or remove a new image.
    before_replace: &'a mut dyn FnMut(&Path, usize) -> Result<(), ErrorCause>,
    identities: Vec<String>,
    initial_fields: Vec<Fields>,
    inputs: Vec<PathBuf>,
    /// Playlists and export files written, for opening after export.
    outputs: Vec<String>,
}
impl NativeHost<'_> {
    fn resolve(&self, path: &Path) -> PathBuf {
        let mut current = path.to_path_buf();
        for change in &self.directories {
            if let Ok(suffix) = current.strip_prefix(&change.from) {
                current = Path::new(&change.to).join(suffix);
            }
        }
        current
    }
    fn move_cover_outputs(&mut self, from: &str, to: &str) {
        let moved = |path: String| match Path::new(&path).strip_prefix(from) {
            Ok(suffix) => Path::new(to).join(suffix).to_string_lossy().into_owned(),
            Err(_) => path,
        };
        // Destination entries move; original input identities remain stable.
        self.written_covers = std::mem::take(&mut self.written_covers)
            .into_iter()
            .map(|(path, owner)| (moved(path), owner))
            .collect();
        let mut exported: BTreeMap<_, HashSet<_>> = BTreeMap::new();
        for ((path, digest), outputs) in std::mem::take(&mut self.exported_covers) {
            exported
                .entry((moved(path), digest))
                .or_default()
                .extend(outputs.into_iter().map(&moved));
        }
        self.exported_covers = exported;
        self.captured_covers = std::mem::take(&mut self.captured_covers)
            .into_iter()
            .map(moved)
            .collect();
    }
    fn capture_cover_output(&mut self, destination: &str, target: &Path) -> Result<(), ErrorCause> {
        if !self.captured_covers.contains(destination) {
            (self.before_replace)(target, self.directories.len())?;
            self.captured_covers.insert(destination.to_owned());
        }
        Ok(())
    }
    fn synchronize(&self, state: &mut ActionState) -> Result<(), String> {
        let path = self.resolve(&state_path(state)?);
        state
            .context
            .insert("_PATH".into(), vec![path.to_string_lossy().into_owned()]);
        Ok(())
    }
    fn flush(&self, state: &mut ActionState) -> Result<AudioEntry, String> {
        self.flush_diagnostic(state)
            .map_err(|error| error.to_string())
    }
    fn flush_diagnostic(&self, state: &mut ActionState) -> Result<AudioEntry, ErrorCause> {
        self.synchronize(state).map_err(ErrorCause::owned_text)?;
        let path = state_path(state).map_err(ErrorCause::owned_text)?;
        let original = crate::settings::read_summary_diagnostic(&path)?;
        let updates = delta(&original.fields, &state.fields);
        let entry = if updates.is_empty() {
            original
        } else {
            crate::settings::write_summary_diagnostic(&path, &updates)?
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
        self.protect_output_diagnostic(target, states)
            .map_err(|error| error.to_string())
    }
    fn protect_output_diagnostic(
        &self,
        target: &Path,
        states: &[ActionState],
    ) -> Result<(), ErrorCause> {
        let target = normalized_output_diagnostic(target)?;
        let protected = self
            .inputs
            .iter()
            .cloned()
            .chain(
                states
                    .iter()
                    .map(|state| state_path(state).map_err(ErrorCause::owned_text))
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .flat_map(|path| protected_files(&path));
        for source in protected {
            let source = self.resolve(&source);
            let source = normalized_output_diagnostic(&source)?;
            if source == target {
                return Err(ErrorCause::owned(
                    "Export output cannot replace an input audio file",
                ));
            }
        }
        Ok(())
    }
}
impl NativeHost<'_> {
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
impl NativeHost<'_> {
    fn evaluate_diagnostic(
        &self,
        template: &str,
        state: &ActionState,
    ) -> Result<String, ErrorCause> {
        let (identity, unchanged) = self.identity(state);
        let context = self.path_context(state).map_err(ErrorCause::owned_text)?;
        self.settings
            .evaluate_diagnostic(template, &state.fields, &context, &identity, unchanged)
    }
    fn evaluate_filename_diagnostic(
        &self,
        template: &str,
        state: &ActionState,
    ) -> Result<String, ErrorCause> {
        let (identity, unchanged) = self.identity(state);
        let context = self.path_context(state).map_err(ErrorCause::owned_text)?;
        self.settings.evaluate_native_filename_diagnostic(
            template,
            &state.fields,
            &context,
            &identity,
            unchanged,
        )
    }
}

impl NativeHost<'_> {
    fn execute_diagnostic(
        &mut self,
        effect: &ActionEffect,
        state: &mut ActionState,
    ) -> Result<(), ErrorCause> {
        self.flush_diagnostic(state)?;
        let path = state_path(state).map_err(ErrorCause::owned_text)?;
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
                tag_core::operations::rename_no_clobber_cause(&path, &target)?;
                let entry = crate::settings::read_summary_diagnostic(&target)?;
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
                let source = path
                    .parent()
                    .ok_or_else(|| ErrorCause::owned("File has no parent directory"))?;
                let target = if Path::new(&directory).is_absolute() {
                    PathBuf::from(&directory)
                } else {
                    source.parent().unwrap_or(Path::new(".")).join(&directory)
                };
                if source != target {
                    if target.starts_with(source) {
                        return Err(ErrorCause::owned("Cannot move a directory inside itself"));
                    }
                    let from = normalized_output_diagnostic(source)?;
                    tag_core::operations::rename_directory_no_clobber_diagnostic(source, &target)?;
                    self.directories.push(PathChange {
                        from: source.to_string_lossy().into_owned(),
                        to: target.to_string_lossy().into_owned(),
                    });
                    self.move_cover_outputs(&from, &normalized_output_diagnostic(&target)?);
                }
                self.synchronize(state).map_err(ErrorCause::owned_text)?;
            }
            ActionEffect::RemoveCovers => {
                tag_core::remove_all_covers_with_options_diagnostic(
                    &path,
                    &crate::settings::tag_options(),
                )?;
            }
            ActionEffect::ImportCover {
                filename,
                picture_type,
                delete_existing,
            } => {
                let image =
                    tag_core::resolve_cover_source_diagnostic(&template_path(filename, &path))?;
                tag_core::import_cover_with_options_diagnostic(
                    &path,
                    &image,
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
                let entry = crate::settings::read_full_diagnostic(&path)?;
                let filter = picture_type
                    .as_deref()
                    .map(tag_core::picture_type_from_name_diagnostic)
                    .transpose()?;
                struct Output {
                    bytes: Vec<u8>,
                    digest: [u8; 32],
                    identity: (String, [u8; 32]),
                    target: PathBuf,
                    destination: String,
                    skip: bool,
                }
                let mut outputs = Vec::new();
                let mut seen = HashSet::new();
                // Fix every ordinal before deciding which writes can be skipped.
                for cover in &entry.covers {
                    if filter.is_some_and(|f| format!("{f:?}") != cover.picture_type) {
                        continue;
                    }
                    let bytes = STANDARD
                        .decode(&cover.data_base64)
                        .map_err(ErrorCause::external)?;
                    let digest: [u8; 32] = Sha256::digest(&bytes).into();
                    let mut target =
                        cover_file_target(template_path(filename, &path), &cover.mime, &bytes);
                    // Identity uses the requested name before ordinal suffixes.
                    let identity = (normalized_cover_output_diagnostic(&target)?, digest);
                    let count = outputs.len();
                    if count > 0 {
                        let stem = target.file_stem().unwrap_or_default().to_string_lossy();
                        let extension = target.extension().unwrap_or_default().to_string_lossy();
                        target.set_file_name(format!("{stem} ({count}).{extension}"));
                    }
                    outputs.push(Output {
                        destination: normalized_cover_output_diagnostic(&target)?,
                        // The first occurrence will either write its image or
                        // retain a proven surviving output from this run.
                        skip: !export_duplicates && !seen.insert(identity.clone()),
                        identity,
                        bytes,
                        digest,
                        target,
                    });
                }
                if !export_duplicates {
                    let mut pending: BTreeMap<_, _> = outputs
                        .iter()
                        .filter(|output| !output.skip)
                        .map(|output| (output.destination.clone(), output.digest))
                        .collect();
                    loop {
                        let mut changed = false;
                        for output in &mut outputs {
                            if !output.skip
                                && self.exported_covers.get(&output.identity).is_some_and(
                                    |existing| {
                                        existing.iter().any(|destination| {
                                            pending
                                                .get(destination)
                                                .is_none_or(|digest| *digest == output.digest)
                                        })
                                    },
                                )
                            {
                                output.skip = true;
                                pending.remove(&output.destination);
                                changed = true;
                            }
                        }
                        // Removing a write can make another image safe to skip.
                        // Each pass removes at least one, so this is bounded by
                        // the number of matching covers.
                        if !changed {
                            break;
                        }
                    }
                }
                for output in outputs.into_iter().filter(|output| !output.skip) {
                    let Output {
                        bytes,
                        digest,
                        identity,
                        target,
                        destination,
                        ..
                    } = output;
                    self.protect_output_diagnostic(&target, std::slice::from_ref(state))?;
                    let (writer, _) = self.identity(state);
                    let previous = self
                        .written_covers
                        .get(&destination)
                        .map(|(digest, _)| *digest);
                    match self.written_covers.get(&destination) {
                        // That image is already there; whoever wrote it keeps the name.
                        Some((written, _)) if *written == digest => {}
                        // Another file of this run already wrote a different image there.
                        Some((_, earlier)) if *earlier != writer => {
                            return Err(ErrorCause::message(
                                "{value} was already written for another file",
                                [("value", target.to_string_lossy().into_owned())],
                            ));
                        }
                        // A prior no-op has an owner but no snapshot yet.
                        // Capture its original entry at the first real replacement.
                        Some(_) => {
                            self.capture_cover_output(&destination, &target)?;
                            atomic_output_diagnostic(&target, &bytes, false)?;
                            self.written_covers
                                .insert(destination.clone(), (digest, writer));
                        }
                        None => {
                            // Identical bytes need neither a write nor undo
                            // capability. Ownership still belongs to this input.
                            let identical = fs::metadata(&target)
                                .is_ok_and(|meta| meta.len() == bytes.len() as u64)
                                && fs::read(&target).is_ok_and(|existing| existing == bytes);
                            if !identical {
                                self.capture_cover_output(&destination, &target)?;
                                atomic_output_diagnostic(&target, &bytes, false)?;
                            }
                            self.written_covers
                                .insert(destination.clone(), (digest, writer));
                        }
                    }
                    if previous.is_some_and(|previous| previous != digest) {
                        self.exported_covers.retain(|_, outputs| {
                            outputs.remove(&destination);
                            !outputs.is_empty()
                        });
                    }
                    self.exported_covers
                        .entry(identity)
                        .or_default()
                        .insert(destination);
                }
            }
            ActionEffect::AdjustCover {
                format,
                quality,
                resize,
            } => {
                tag_core::adjust_covers_with_options_diagnostic(
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
                tag_core::set_cover_properties_with_options_diagnostic(
                    &path,
                    picture_type.as_deref(),
                    description.as_deref(),
                    only_picture_type.as_deref(),
                    &crate::settings::tag_options(),
                )?;
            }
            ActionEffect::ImportTextFile { field, filename } => {
                let text = read_action_text(
                    fs::read(template_path(filename, &path)),
                    self.settings.ansi_codepage().as_deref(),
                )?;
                state.fields.insert(field.clone(), vec![text]);
                self.flush_diagnostic(state)?;
            }
        }
        // Refresh properties without losing imported data needed by later actions.
        let entry = crate::settings::read_summary_diagnostic(
            &self.resolve(&state_path(state).map_err(ErrorCause::owned_text)?),
        )?;
        let index = state
            .context
            .get("_COUNTER")
            .and_then(|v| v.first())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        state.context = technical_fields(&entry, index);
        Ok(())
    }
}

impl ActionHost for NativeHost<'_> {
    fn evaluate(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        let (identity, unchanged) = self.identity(state);
        let context = self.path_context(state)?;
        self.settings
            .evaluate(template, &state.fields, &context, &identity, unchanged)
    }
    fn evaluate_filename(&mut self, template: &str, state: &ActionState) -> Result<String, String> {
        let (identity, unchanged) = self.identity(state);
        let context = self.path_context(state)?;
        self.settings.evaluate_native_filename(
            template,
            &state.fields,
            &context,
            &identity,
            unchanged,
        )
    }
    fn execute(&mut self, effect: &ActionEffect, state: &mut ActionState) -> Result<(), String> {
        self.execute_diagnostic(effect, state)
            .map_err(|error| error.to_string())
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
                    let name = self.evaluate_filename(filename_template, first)?;
                    let target =
                        normalized_output_path(&template_path(&name, &state_path(first)?))?;
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
                    working_directory: self.settings.working_directory.clone(),
                    ansi_codepage: self.settings.ansi_codepage(),
                    timestamp_seconds: Some(self.settings.timestamp_seconds),
                    natural_sort: self.settings.natural_sort,
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

fn read_action_text(
    bytes: std::io::Result<Vec<u8>>,
    ansi_codepage: Option<&str>,
) -> Result<String, ErrorCause> {
    let bytes = bytes.map_err(ErrorCause::external)?;
    tag_core::export::decode_export_template_diagnostic(&bytes, ansi_codepage)
        .map_err(|cause| ErrorCause::wrap("Cannot decode the text file: {e}", [], "e", cause))
}

// Atomic cover writes replace the final directory entry, rather than the
// object a final symlink points at. Resolve parent aliases but keep that
// entry's identity stable before and after replacement.
fn normalized_cover_output_diagnostic(path: &Path) -> Result<String, ErrorCause> {
    let parent = normalized_output_path_diagnostic(path.parent().unwrap_or(Path::new(".")))?;
    let name = path
        .file_name()
        .ok_or_else(|| ErrorCause::owned("Cover output has no filename"))?;
    let entry = parent.join(name);
    let resolved = match fs::symlink_metadata(&entry) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                // canonicalize follows the final link. Inspect its directory
                // entry instead, including its stored spelling on volumes
                // where a differently cased lookup names the same entry.
                let mut alias = None;
                for candidate in fs::read_dir(&parent).map_err(ErrorCause::external)? {
                    let candidate = candidate.map_err(ErrorCause::external)?;
                    let candidate_name = candidate.file_name();
                    if candidate_name == name {
                        alias = Some(candidate.path());
                        break;
                    }
                    if candidate_name.to_string_lossy().to_lowercase()
                        == name.to_string_lossy().to_lowercase()
                    {
                        let found =
                            fs::symlink_metadata(candidate.path()).map_err(ErrorCause::external)?;
                        if found.dev() == metadata.dev() && found.ino() == metadata.ino() {
                            alias = Some(candidate.path());
                        }
                    }
                }
                alias.ok_or_else(|| {
                    ErrorCause::owned("Cannot identify the cover output directory entry")
                })?
            }
            #[cfg(not(unix))]
            {
                entry
            }
        }
        Ok(_) => normalized_output_path_diagnostic(&entry)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => entry,
        Err(error) => return Err(ErrorCause::external(error)),
    };
    let value = resolved.to_string_lossy().into_owned();
    Ok(if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    })
}

pub(crate) fn normalized_output(path: &Path) -> Result<String, String> {
    normalized_output_diagnostic(path).map_err(|error| error.to_string())
}
pub(crate) fn normalized_output_diagnostic(path: &Path) -> Result<String, ErrorCause> {
    let value = normalized_output_path_diagnostic(path)?
        .to_string_lossy()
        .into_owned();
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
    normalized_output_path_diagnostic(path).map_err(|error| error.to_string())
}
pub(crate) fn normalized_output_path_diagnostic(path: &Path) -> Result<PathBuf, ErrorCause> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(ErrorCause::external)?
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
                        resolved = dunce::canonicalize(&resolved).map_err(ErrorCause::external)?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(ErrorCause::wrap(
                            "Cannot inspect output path {value}: {error}",
                            [("value", resolved.to_string_lossy().into_owned())],
                            "error",
                            ErrorCause::external(error),
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
    execute_with_side_files(paths, groups, settings, &mut |_| Ok(()))
}

/// [`execute`], calling `before_replace` before the first actual cover write
/// (including creations), so undo can restore its original state.
pub fn execute_with_side_files(
    paths: &[String],
    groups: &[ActionGroup],
    settings: EvaluationSettings,
    before_replace: &mut dyn FnMut(&Path) -> Result<(), String>,
) -> ActionResult {
    execute_with_side_file_epochs(paths, groups, settings, &mut |path, _| before_replace(path))
}

/// Execute with a capture hook that receives the current directory-move epoch.
pub fn execute_with_side_file_epochs(
    paths: &[String],
    groups: &[ActionGroup],
    settings: EvaluationSettings,
    before_replace: &mut dyn FnMut(&Path, usize) -> Result<(), String>,
) -> ActionResult {
    execute_with_side_file_epochs_diagnostic(paths, groups, settings, &mut |path, epoch| {
        before_replace(path, epoch).map_err(ErrorCause::external)
    })
    .result
}

pub fn execute_diagnostic(
    paths: &[String],
    groups: &[ActionGroup],
    settings: EvaluationSettings,
) -> Detailed<ActionResult> {
    execute_with_side_file_epochs_diagnostic(paths, groups, settings, &mut |_, _| Ok(()))
}

pub fn execute_with_side_file_epochs_diagnostic(
    paths: &[String],
    groups: &[ActionGroup],
    settings: EvaluationSettings,
    before_replace: &mut dyn FnMut(&Path, usize) -> Result<(), ErrorCause>,
) -> Detailed<ActionResult> {
    let mut result = ScanResult::default();
    let mut error_diagnostics = Vec::new();
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
        exported_covers: BTreeMap::new(),
        written_covers: BTreeMap::new(),
        captured_covers: HashSet::new(),
        before_replace,
        identities: initial.clone(),
        inputs: initial.iter().map(PathBuf::from).collect(),
        initial_fields: states.iter().map(|s| s.fields.clone()).collect(),
        outputs: Vec::new(),
    };
    let mut failed = HashSet::new();
    let mut preflight = None;
    for group in groups.iter().filter(|g| g.enabled) {
        for action in &group.actions {
            if let Err(error) = tag_core::actions::validate_action_diagnostic(action) {
                preflight = Some(BatchError::ActionValidation {
                    group: group.name.clone(),
                    cause: error,
                });
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
            push_error(
                &mut result.errors,
                &mut error_diagnostics,
                path.clone(),
                error.clone(),
            );
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
                    let applied = tag_core::actions::apply_action_spec_with_modes_diagnostic(
                        state,
                        action,
                        &mut |template, state, mode| match mode {
                            TemplateMode::Text => host.evaluate_diagnostic(template, state),
                            TemplateMode::Filename => {
                                host.evaluate_filename_diagnostic(template, state)
                            }
                        },
                    );
                    let outcome = applied.and_then(|effects| {
                        for effect in effects {
                            host.execute_diagnostic(&effect, state)?;
                        }
                        Ok(())
                    });
                    // As in Mp3tag, a problem is a warning: the file's
                    // remaining actions and groups still run.
                    if let Err(error) = outcome {
                        let path = host.resolve(
                            &state_path(state).unwrap_or_else(|_| PathBuf::from(&initial[index])),
                        );
                        push_error(
                            &mut result.errors,
                            &mut error_diagnostics,
                            path.to_string_lossy().into_owned(),
                            BatchError::Action {
                                group: group.name.clone(),
                                index: action_index + 1,
                                cause: error,
                            },
                        );
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
    Detailed {
        result: ActionResult {
            result,
            renamed,
            moved_directories: host.directories,
            outputs: host.outputs,
        },
        error_diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::actions::{ActionSpec, CaseMode};

    #[test]
    fn text_import_distinguishes_owned_decode_guard_from_identical_io_text() {
        let owned = read_action_text(Ok(vec![0xff, 0xfe, 0]), None).unwrap_err();
        let text = "Cannot decode the text file: Truncated UTF-16 export configuration";
        let external = read_action_text(Err(std::io::Error::other(text)), None).unwrap_err();
        assert_eq!(owned.to_string(), text);
        assert_eq!(external.to_string(), text);
        assert!(matches!(&owned, ErrorCause::Owned { children, .. }
            if matches!(children.get("e"), Some(ErrorCause::Owned { key, .. }) if key == "Truncated UTF-16 export configuration")));
        assert!(matches!(external, ErrorCause::External { .. }));
        let foreign_utf8 = read_action_text(Ok(vec![0xef, 0xbb, 0xbf, 0xff]), None).unwrap_err();
        assert!(matches!(foreign_utf8, ErrorCause::Owned { children, .. }
            if matches!(children.get("e"), Some(ErrorCause::External { .. }))));
        assert_eq!(
            tag_core::export::decode_export_template(&[0xff, 0xfe, 0], None).unwrap_err(),
            "Truncated UTF-16 export configuration"
        );
    }

    #[test]
    fn detailed_action_validation_keeps_authored_group_and_has_no_fake_action_index() {
        let directory = tempfile::tempdir().unwrap();
        let audio = fixture(directory.path(), "source.wav")
            .to_string_lossy()
            .into_owned();
        let group = |action| ActionGroup {
            name: "Help: Title / action 1: Help".into(),
            enabled: true,
            actions: vec![action],
        };
        let owned_group = group(ActionSpec::SplitField {
            field: "ARTIST".into(),
            separator: String::new(),
        });
        let detailed = execute_diagnostic(
            std::slice::from_ref(&audio),
            std::slice::from_ref(&owned_group),
            EvaluationSettings::default(),
        );
        assert_eq!(detailed.result.errors.len(), 1);
        let json = serde_json::to_value(&detailed).unwrap();
        let frame = &json["error_diagnostics"][0]["diagnostic"];
        assert_eq!(frame["kind"], "action_validation");
        assert_eq!(frame["group"], owned_group.name);
        assert!(frame.get("index").is_none());
        assert_ne!(frame["cause"]["kind"], "external");
        let legacy = execute(
            std::slice::from_ref(&audio),
            &[owned_group],
            EvaluationSettings::default(),
        );
        assert_eq!(legacy.errors[0].error, detailed.result.errors[0].error);
        let regex_group = group(ActionSpec::RegexReplace {
            field: "TITLE".into(),
            pattern: "[".into(),
            replacement: "".into(),
            case_sensitive: true,
        });
        let regex = execute_diagnostic(&[audio], &[regex_group], EvaluationSettings::default());
        let json = serde_json::to_value(&regex).unwrap();
        assert_eq!(
            json["error_diagnostics"][0]["diagnostic"]["cause"]["kind"],
            "external"
        );
    }

    #[test]
    fn detailed_action_warnings_retain_raw_group_and_real_index_while_later_actions_run() {
        let directory = tempfile::tempdir().unwrap();
        let audio = fixture(directory.path(), "source.wav")
            .to_string_lossy()
            .into_owned();
        let group = ActionGroup {
            name: "Help / action 1: Title".into(),
            enabled: true,
            actions: vec![
                ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "Before".into(),
                },
                ActionSpec::FormatValue {
                    field: "_FILENAME".into(),
                    template: String::new(),
                },
                ActionSpec::FormatValue {
                    field: "COMMENT".into(),
                    template: "After".into(),
                },
            ],
        };
        let result = execute_diagnostic(
            std::slice::from_ref(&audio),
            std::slice::from_ref(&group),
            EvaluationSettings::default(),
        );
        assert_eq!(result.result.errors.len(), 1);
        assert_eq!(
            result.result.errors[0].error,
            "Help / action 1: Title / action 2: Filename must not be empty"
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["error_diagnostics"][0]["index"], 0);
        assert_eq!(
            json["error_diagnostics"][0]["diagnostic"]["group"],
            "Help / action 1: Title"
        );
        assert_eq!(json["error_diagnostics"][0]["diagnostic"]["index"], 2);
        assert_eq!(
            json["error_diagnostics"][0]["diagnostic"]["cause"]["kind"],
            "owned"
        );
        assert_eq!(
            json["error_diagnostics"][0]["diagnostic"]["cause"]["key"],
            "Filename must not be empty"
        );
        let saved = tag_core::read_entry_summary(Path::new(&audio)).unwrap();
        assert_eq!(saved.fields["TITLE"], ["Before"]);
        assert_eq!(saved.fields["COMMENT"], ["After"]);
        let legacy = execute(
            std::slice::from_ref(&audio),
            &[group],
            EvaluationSettings::default(),
        );
        assert_eq!(
            legacy.errors[0].error,
            "Help / action 1: Title / action 2: Filename must not be empty"
        );
        assert!(serde_json::to_value(legacy)
            .unwrap()
            .get("error_diagnostics")
            .is_none());
    }

    #[test]
    fn missing_export_configuration_reports_the_authored_name_or_path() {
        let directory = tempfile::tempdir().unwrap();
        let settings = EvaluationSettings {
            app_data_directory: directory.path().to_string_lossy().into_owned(),
            ..Default::default()
        };
        for source in ["Help", "Title", "My  Files"] {
            assert_eq!(
                resolve_export_template(source, &settings).unwrap_err(),
                format!("Export configuration not found: {source}")
            );
        }
        let path = directory.path().join("My  Files").join("Help.mte");
        let source = path.to_string_lossy();
        assert_eq!(
            resolve_export_template(&source, &settings).unwrap_err(),
            format!("Export configuration not found: {source}")
        );
    }

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
        let expected = dunce::canonicalize(dir.path())
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
            dunce::canonicalize(&parent)
                .unwrap()
                .join("Chosen Track.wav")
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

    #[cfg(unix)]
    #[test]
    fn cover_export_symlink_replacement_keeps_cross_file_ownership() {
        let dir = tempfile::tempdir().unwrap();
        let (a, original) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let other = bytes.into_inner();
        let image = dir.path().join("other.png");
        fs::write(&image, &other).unwrap();
        tag_core::import_cover_with_options(
            &b,
            &image,
            None,
            true,
            &crate::settings::tag_options(),
        )
        .unwrap();
        let target = dir.path().join("shared.png");
        std::os::unix::fs::symlink(&image, &target).unwrap();
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &[cover_export_group("shared", false)],
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].path.ends_with("b.wav"));
        assert!(result.errors[0]
            .error
            .contains("already written for another file"));
        assert_eq!(fs::read(&target).unwrap(), original);
        assert!(!fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read(image).unwrap(), other);
    }

    #[cfg(unix)]
    #[test]
    fn cover_export_existing_symlink_case_alias_keeps_ownership() {
        let dir = tempfile::tempdir().unwrap();
        let (a, image) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let other = dir.path().join("other.png");
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        fs::write(&other, bytes.into_inner()).unwrap();
        tag_core::import_cover_with_options(
            &b,
            &other,
            None,
            true,
            &crate::settings::tag_options(),
        )
        .unwrap();
        let stored = dir.path().join("shared.png");
        std::os::unix::fs::symlink(&other, &stored).unwrap();
        // This actual-volume probe needs no assumption about macOS/Linux
        // filesystem case sensitivity. The ordinary symlink case runs on both.
        if !dir.path().join("SHARED.PNG").exists() {
            return;
        }
        let group = cover_export_group("$if($eql(%_filename%,a),SHARED.PNG,shared.png)", false);
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &[group],
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].path.ends_with("b.wav"));
        assert!(result.errors[0]
            .error
            .contains("already written for another file"));
        assert_eq!(fs::read(stored).unwrap(), image);
    }

    #[cfg(unix)]
    #[test]
    fn action_path_placeholders_keep_backslashes_inside_native_components() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("AC\\DC");
        fs::create_dir(&album).unwrap();
        let (path, image) = covered_fixture(&album, "a.wav", false);
        fs::write(album.join("note.txt"), "from actual folder").unwrap();
        let settings = EvaluationSettings {
            working_directory: Some(album.to_string_lossy().into_owned()),
            ..Default::default()
        };
        let result = run(
            &path,
            vec![
                ActionSpec::ExportCover {
                    filename_template: "%_folderpath%art\\folder".into(),
                    picture_type: None,
                    export_duplicates: false,
                },
                ActionSpec::ImportTextFile {
                    field: "COMMENT".into(),
                    filename_template: "%_workingpath%note.txt".into(),
                },
                ActionSpec::GeneratePlaylists {
                    filename_template: "%_path%.m3u8".into(),
                    partition_template: String::new(),
                },
            ],
            settings,
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(album.join("art/folder.png")).unwrap(), image);
        assert_eq!(
            tag_core::read_entry_summary(&path).unwrap().fields["COMMENT"],
            ["from actual folder"]
        );
        assert!(album.join("a.wav.m3u8").is_file());
        assert!(!dir.path().join("AC").exists());
    }

    #[test]
    fn cover_export_ownership_follows_a_later_directory_move() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("Album");
        fs::create_dir(&album).unwrap();
        let (a, original) = covered_fixture(&album, "a.wav", false);
        let (b, _) = covered_fixture(&album, "b.wav", false);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let other = dir.path().join("other.png");
        fs::write(&other, bytes.into_inner()).unwrap();
        tag_core::import_cover_with_options(
            &b,
            &other,
            Some("CoverBack"),
            true,
            &crate::settings::tag_options(),
        )
        .unwrap();
        let group = ActionGroup {
            name: "Moved output".into(),
            enabled: true,
            actions: vec![
                ActionSpec::ExportCover {
                    filename_template: "shared".into(),
                    picture_type: None,
                    export_duplicates: false,
                },
                ActionSpec::FormatValue {
                    field: "_DIRECTORY".into(),
                    template: "Moved".into(),
                },
                ActionSpec::ExportCover {
                    filename_template: "shared".into(),
                    picture_type: Some("CoverBack".into()),
                    export_duplicates: false,
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
        assert_eq!(
            fs::read(dir.path().join("Moved/shared.png")).unwrap(),
            original
        );
        assert_eq!(result.errors.len(), 2, "{:?}", result.errors);
        assert!(result
            .errors
            .iter()
            .all(|error| error.path.ends_with("b.wav")
                && error.error.contains("already written for another file")));
        assert!(!album.exists());
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
    fn cover_export_replaces_existing_files_and_rejects_input_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let (path, png) = covered_fixture(dir.path(), "input.wav", false);
        let before = fs::read(&path).unwrap();
        let output = dir.path().join("cover.png");
        fs::write(&output, b"Existing output").unwrap();
        let mut replaced = Vec::new();
        let run = |replaced: &mut Vec<PathBuf>| {
            execute_with_side_files(
                &[path.to_string_lossy().into_owned()],
                &[cover_export_group("cover", false)],
                EvaluationSettings::default(),
                &mut |file| {
                    replaced.push(file.to_path_buf());
                    Ok(())
                },
            )
        };
        // Another image there is replaced, after undo saw the old file.
        let result = run(&mut replaced);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(&output).unwrap(), png);
        assert_eq!(replaced.len(), 1);
        assert!(replaced[0].ends_with("cover.png"));
        // Running again finds the same image and writes nothing.
        let modified = fs::metadata(&output).unwrap().modified().unwrap();
        let result = run(&mut replaced);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(&output).unwrap(), png);
        assert_eq!(fs::metadata(&output).unwrap().modified().unwrap(), modified);
        assert_eq!(fs::read(&path).unwrap(), before);
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
    fn cover_export_identical_output_does_not_require_undo_capture() {
        let dir = tempfile::tempdir().unwrap();
        let (path, image) = covered_fixture(dir.path(), "a.wav", false);
        let audio = fs::read(&path).unwrap();
        let output = dir.path().join("cover.png");
        fs::write(&output, &image).unwrap();
        let modified = fs::metadata(&output).unwrap().modified().unwrap();
        let mut calls = 0;
        let result = execute_with_side_files(
            &[path.to_string_lossy().into_owned()],
            &[cover_export_group("cover", false)],
            EvaluationSettings::default(),
            &mut |_| {
                calls += 1;
                Err("Undo capture deliberately unavailable".into())
            },
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(calls, 0);
        assert_eq!(fs::read(&output).unwrap(), image);
        assert_eq!(fs::metadata(output).unwrap().modified().unwrap(), modified);
        assert_eq!(fs::read(path).unwrap(), audio);
    }

    #[test]
    fn cover_export_by_the_same_file_replaces_its_earlier_output_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let (path, png) = covered_fixture(dir.path(), "a.wav", false);
        let before_run = fs::read(&path).unwrap();
        let image = |width| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            bytes.into_inner()
        };
        fs::write(dir.path().join("second.png"), image(2)).unwrap();
        fs::write(dir.path().join("third.png"), image(3)).unwrap();
        let export = || ActionSpec::ExportCover {
            filename_template: "art".into(),
            picture_type: None,
            export_duplicates: false,
        };
        let import = |name: &str| ActionSpec::ImportCover {
            filename_template: name.into(),
            picture_type: None,
            delete_existing: true,
        };
        let groups = [
            ActionGroup {
                name: "First".into(),
                enabled: true,
                actions: vec![export(), import("second.png"), export()],
            },
            ActionGroup {
                name: "Later".into(),
                enabled: true,
                actions: vec![import("third.png"), export()],
            },
        ];
        let output = dunce::canonicalize(dir.path()).unwrap().join("art.png");
        let mut replaced = Vec::new();
        let run = |groups: &[ActionGroup], replaced: &mut Vec<PathBuf>| {
            execute_with_side_files(
                &[path.to_string_lossy().into_owned()],
                groups,
                EvaluationSettings::default(),
                &mut |file| {
                    replaced.push(file.to_path_buf());
                    Ok(())
                },
            )
        };
        let result = run(&groups[..1], &mut replaced);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(&output).unwrap(), image(2));
        // Capture the original absence once; later same-run replacements
        // reuse it instead of snapshotting an intermediate image.
        assert_eq!(replaced.as_slice(), std::slice::from_ref(&output));
        let result = run(&groups[1..], &mut replaced);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(&output).unwrap(), image(3));
        assert_eq!(replaced, [output.clone(), output.clone()]);
        // Both groups in one run, starting from a folder image equal to the
        // embedded cover: each export replaces the file's own earlier one.
        fs::write(&path, &before_run).unwrap();
        fs::write(&output, &png).unwrap();
        replaced.clear();
        let result = run(&groups, &mut replaced);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(&output).unwrap(), image(3));
        // Undo was given the image from before the run, once.
        assert_eq!(replaced.len(), 1);
    }

    #[test]
    fn cover_export_appends_new_images_after_suppressed_existing_covers() {
        let dir = tempfile::tempdir().unwrap();
        let (path, original) = covered_fixture(dir.path(), "a.wav", false);
        let mut other = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut other, image::ImageFormat::Png)
            .unwrap();
        let other = other.into_inner();
        fs::write(dir.path().join("other.png"), &other).unwrap();
        let export = || ActionSpec::ExportCover {
            filename_template: "art".into(),
            picture_type: None,
            export_duplicates: false,
        };
        let result = run(
            &path,
            vec![
                export(),
                ActionSpec::ImportCover {
                    filename_template: "other.png".into(),
                    picture_type: None,
                    delete_existing: false,
                },
                export(),
            ],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(dir.path().join("art.png")).unwrap(), original);
        assert_eq!(fs::read(dir.path().join("art (1).png")).unwrap(), other);
        assert!(!dir.path().join("art (2).png").exists());
    }

    #[test]
    fn cover_export_does_not_reexport_unchanged_secondary_images() {
        let dir = tempfile::tempdir().unwrap();
        let (path, original) = covered_fixture(dir.path(), "a.wav", false);
        let mut other = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut other, image::ImageFormat::Png)
            .unwrap();
        let other = other.into_inner();
        let image = dir.path().join("other.png");
        fs::write(&image, &other).unwrap();
        tag_core::add_cover(&path, &image).unwrap();
        let mut group = cover_export_group("art", false);
        group
            .actions
            .extend(cover_export_group("art", false).actions);
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &[group],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(dir.path().join("art.png")).unwrap(), original);
        assert_eq!(fs::read(dir.path().join("art (1).png")).unwrap(), other);
        assert!(!dir.path().join("art (2).png").exists());
    }

    #[test]
    fn cover_export_preserves_images_when_later_positions_replace_their_outputs() {
        for next in [[1, 2], [1, 0], [1, 1]] {
            let dir = tempfile::tempdir().unwrap();
            let (path, original) = covered_fixture(dir.path(), "a.wav", false);
            let mut images = vec![original];
            for width in [2, 3] {
                let mut bytes = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                    .write_to(&mut bytes, image::ImageFormat::Png)
                    .unwrap();
                images.push(bytes.into_inner());
            }
            for (index, bytes) in images.iter().enumerate() {
                fs::write(dir.path().join(format!("input{index}.png")), bytes).unwrap();
            }
            let import = |index, delete_existing| ActionSpec::ImportCover {
                filename_template: format!("input{index}.png"),
                picture_type: None,
                delete_existing,
            };
            let export = || cover_export_group("art", false).actions.remove(0);
            let result = run(
                &path,
                vec![
                    import(1, false),
                    export(),
                    import(next[0], true),
                    import(next[1], false),
                    export(),
                ],
                EvaluationSettings::default(),
            );
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let outputs = [
                fs::read(dir.path().join("art.png")).unwrap(),
                fs::read(dir.path().join("art (1).png")).unwrap(),
            ];
            for index in next {
                assert!(
                    outputs.contains(&images[index]),
                    "missing image {index} in {next:?}"
                );
            }
            if next[0] != next[1] {
                assert_eq!(outputs[0], images[next[0]]);
                assert_eq!(outputs[1], images[next[1]]);
            }
            assert!(!dir.path().join("art (2).png").exists());
        }
    }

    #[test]
    fn cover_export_suppression_keeps_outputs_when_later_writes_are_also_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let (path, original) = covered_fixture(dir.path(), "a.wav", false);
        let mut images = vec![original];
        for width in [2, 3] {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            images.push(bytes.into_inner());
            fs::write(
                dir.path().join(format!("input{width}.png")),
                images.last().unwrap(),
            )
            .unwrap();
        }
        let import = |width, delete_existing| ActionSpec::ImportCover {
            filename_template: format!("input{width}.png"),
            picture_type: None,
            delete_existing,
        };
        let export = || cover_export_group("art", false).actions.remove(0);
        let result = run(
            &path,
            vec![
                import(2, false),
                import(3, false),
                export(),
                import(2, true),
                import(3, false),
                export(),
            ],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        // C remains at (2), so suppressing C also makes B safe at (1).
        for (index, bytes) in images.iter().enumerate() {
            let name = if index == 0 {
                "art.png".into()
            } else {
                format!("art ({index}).png")
            };
            assert_eq!(fs::read(dir.path().join(name)).unwrap(), *bytes);
        }
        assert!(!dir.path().join("art (3).png").exists());
    }

    #[test]
    fn cover_export_can_return_to_an_earlier_image_in_the_same_run() {
        let dir = tempfile::tempdir().unwrap();
        let (path, original) = covered_fixture(dir.path(), "a.wav", false);
        fs::write(dir.path().join("original.png"), &original).unwrap();
        let mut other = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut other, image::ImageFormat::Png)
            .unwrap();
        let other = other.into_inner();
        assert_ne!(original, other);
        fs::write(dir.path().join("other.png"), &other).unwrap();
        let export = || ActionSpec::ExportCover {
            filename_template: "art".into(),
            picture_type: None,
            export_duplicates: false,
        };
        let import = |name: &str| ActionSpec::ImportCover {
            filename_template: name.into(),
            picture_type: None,
            delete_existing: true,
        };
        let result = run(
            &path,
            vec![
                export(),
                import("other.png"),
                export(),
                import("original.png"),
                export(),
            ],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(fs::read(dir.path().join("art.png")).unwrap(), original);
    }

    #[test]
    fn cover_export_of_an_image_already_there_keeps_its_writer() {
        let dir = tempfile::tempdir().unwrap();
        let (a, _) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let other = bytes.into_inner();
        fs::write(dir.path().join("other.png"), &other).unwrap();
        let export = || ActionSpec::ExportCover {
            filename_template: "shared".into(),
            picture_type: None,
            export_duplicates: true,
        };
        let groups = [
            // b exports the image a just wrote: nothing is written.
            ActionGroup {
                name: "Both".into(),
                enabled: true,
                actions: vec![export()],
            },
            // a's cover changes and a exports again: a still owns the name.
            ActionGroup {
                name: "Only a".into(),
                enabled: true,
                actions: vec![
                    ActionSpec::ImportCover {
                        filename_template: "$if($eql(%_filename%,a),other.png,missing.png)".into(),
                        picture_type: None,
                        delete_existing: true,
                    },
                    export(),
                ],
            },
        ];
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &groups,
            EvaluationSettings::default(),
        );
        // a keeps ownership after b's no-op export and replaces its own output.
        // b still has the old cover: its missing import and later attempt to
        // replace a's changed image both warn without changing that output.
        assert_eq!(result.errors.len(), 2, "{:?}", result.errors);
        assert!(result
            .errors
            .iter()
            .all(|error| error.path.ends_with("b.wav")));
        assert!(result.errors[0].error.contains("Only a / action 1"));
        assert!(result.errors[1].error.contains("Only a / action 2"));
        assert!(result.errors[1]
            .error
            .contains("already written for another file"));
        assert_eq!(fs::read(dir.path().join("shared.png")).unwrap(), other);
    }

    #[test]
    fn cover_export_cross_file_destination_keeps_the_first_different_image() {
        let dir = tempfile::tempdir().unwrap();
        let (a, png) = covered_fixture(dir.path(), "a.wav", false);
        let (b, _) = covered_fixture(dir.path(), "b.wav", false);
        let (c, _) = covered_fixture(dir.path(), "c.wav", false);
        let other = dir.path().join("other.png");
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        fs::write(&other, bytes.into_inner()).unwrap();
        tag_core::import_cover_with_options(&c, &other, None, true, &Default::default()).unwrap();
        let before = [
            fs::read(&a).unwrap(),
            fs::read(&b).unwrap(),
            fs::read(&c).unwrap(),
        ];
        let paths = [
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
            c.to_string_lossy().into_owned(),
        ];
        // The same image from a second file is no collision, even with
        // duplicates exported; a different one from a third file is.
        let result = execute(
            &paths,
            &[cover_export_group("shared", true)],
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].path.ends_with("c.wav"));
        assert!(result.errors[0].error.contains("already written"));
        assert_eq!(result.entries.len(), 3);
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
    fn filename_collision_reports_only_the_failed_file_and_returns_both() {
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
        // The file that kept its name is still listed with its warning.
        assert_eq!(result.entries.len(), 2);
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
    fn chapter_and_cue_rows_protect_their_files() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("disc.flac");
        fs::write(&audio, b"x").unwrap();
        let cue = dir.path().join("disc.cue");
        fs::write(
            &cue,
            "FILE \"disc.flac\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        let row = tag_core::subsong_path(&cue.to_string_lossy(), 1);
        let protected = protected_files(Path::new(&row));
        assert_eq!(protected[0], cue);
        assert_eq!(
            dunce::canonicalize(&protected[1]).unwrap(),
            dunce::canonicalize(&audio).unwrap()
        );
        let book = dir.path().join("book.m4b");
        let chapter = tag_core::subsong_path(&book.to_string_lossy(), 2);
        assert_eq!(protected_files(Path::new(&chapter)), [book]);
        assert_eq!(protected_files(&audio), [audio]);
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

    #[test]
    fn a_failed_action_is_a_warning_and_later_actions_and_groups_still_run() {
        let dir = tempfile::tempdir().unwrap();
        let a = fixture(dir.path(), "a.wav");
        let b = fixture(dir.path(), "b.wav");
        fs::write(dir.path().join("a.txt"), "Text for a").unwrap();
        let groups = [
            ActionGroup {
                name: "Notes".into(),
                enabled: true,
                actions: vec![
                    ActionSpec::ImportTextFile {
                        field: "COMMENT".into(),
                        filename_template: "%_filename%.txt".into(),
                    },
                    ActionSpec::FormatValue {
                        field: "TITLE".into(),
                        template: "Done".into(),
                    },
                    ActionSpec::Export {
                        template: "$loop(%_filename%)%_filename%:%title%;$loopend()".into(),
                        filename_template: "report.txt".into(),
                        append: false,
                        partition_template: None,
                    },
                ],
            },
            ActionGroup {
                name: "Second".into(),
                enabled: true,
                actions: vec![ActionSpec::FormatValue {
                    field: "ALBUM".into(),
                    template: "Later".into(),
                }],
            },
        ];
        let result = execute(
            &[
                a.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
            ],
            &groups,
            EvaluationSettings::default(),
        );
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].path.ends_with("b.wav"));
        assert!(result.errors[0].error.starts_with("Notes / action 1:"));
        assert_eq!(result.entries.len(), 2);
        for entry in &result.entries {
            assert_eq!(entry.fields["TITLE"], ["Done"]);
            assert_eq!(entry.fields["ALBUM"], ["Later"]);
        }
        assert_eq!(result.entries[0].fields["COMMENT"], ["Text for a"]);
        assert!(!result.entries[1].fields.contains_key("COMMENT"));
        // The file with the warning is still part of the export.
        assert_eq!(
            fs::read_to_string(dir.path().join("report.txt")).unwrap(),
            "a:Done;b:Done;"
        );
    }

    #[test]
    fn import_cover_wildcard_uses_the_first_matching_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path(), "a.wav");
        let scans = dir.path().join("Scans");
        fs::create_dir(&scans).unwrap();
        let image = |width| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, 1))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            bytes.into_inner()
        };
        fs::write(scans.join("front10.png"), image(10)).unwrap();
        fs::write(scans.join("front2.png"), image(2)).unwrap();
        fs::write(scans.join("front1.png"), b"not an image").unwrap();
        let result = run(
            &path,
            vec![ActionSpec::ImportCover {
                filename_template: "Scans\\front*.png".into(),
                picture_type: None,
                delete_existing: false,
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let covers = tag_core::read_entry(&path).unwrap().covers;
        assert_eq!(covers.len(), 1);
        assert_eq!(STANDARD.decode(&covers[0].data_base64).unwrap(), image(2));
        let result = run(
            &path,
            vec![ActionSpec::ImportCover {
                filename_template: "Scans\\*.jpg".into(),
                picture_type: None,
                delete_existing: true,
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors[0].error.contains("No image matches"));
        assert_eq!(tag_core::read_entry(&path).unwrap().covers.len(), 1);
    }

    #[test]
    fn import_cover_wildcard_takes_webp_and_skips_what_the_import_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let path = fixture(dir.path(), "a.wav");
        let encode = |format| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 4))
                .write_to(&mut bytes, format)
                .unwrap();
            bytes.into_inner()
        };
        let webp = encode(image::ImageFormat::WebP);
        let jpeg = encode(image::ImageFormat::Jpeg);
        fs::write(dir.path().join("art1.webp"), &webp).unwrap();
        fs::write(dir.path().join("art2.jpg"), &jpeg).unwrap();
        let import = |template: &str| {
            run(
                &path,
                vec![ActionSpec::ImportCover {
                    filename_template: template.into(),
                    picture_type: None,
                    delete_existing: true,
                }],
                EvaluationSettings::default(),
            )
        };
        let cover = || {
            let covers = tag_core::read_entry(&path).unwrap().covers;
            assert_eq!(covers.len(), 1);
            STANDARD.decode(&covers[0].data_base64).unwrap()
        };
        let result = import("art*");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(cover(), webp);
        // A file whose header looks like an image the import does not take
        // (QOI) is passed over for the next match.
        fs::write(
            dir.path().join("art0.png"),
            b"qoif\0\0\0\x04\0\0\0\x04\x03\0",
        )
        .unwrap();
        fs::remove_file(dir.path().join("art1.webp")).unwrap();
        let result = import("art*");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(cover(), jpeg);
    }

    #[test]
    #[cfg(unix)]
    fn import_cover_wildcard_preserves_native_folder_placeholder_characters() {
        let dir = tempfile::tempdir().unwrap();
        let album = dir.path().join("What?*");
        fs::create_dir(&album).unwrap();
        let path = fixture(&album, "a.wav");
        let encode = |width| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            bytes.into_inner()
        };
        let first = encode(2);
        fs::write(album.join("cover2.png"), &first).unwrap();
        fs::write(album.join("cover10.png"), encode(3)).unwrap();
        let result = run(
            &path,
            vec![ActionSpec::ImportCover {
                filename_template: "%_folderpath%cover*.png".into(),
                picture_type: None,
                delete_existing: true,
            }],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let covers = tag_core::read_entry(&path).unwrap().covers;
        assert_eq!(covers.len(), 1);
        assert_eq!(STANDARD.decode(&covers[0].data_base64).unwrap(), first);
    }

    #[test]
    fn file_name_formats_use_backslash_folders_and_clean_tag_values() {
        let dir = tempfile::tempdir().unwrap();
        let (path, png) = covered_fixture(dir.path(), "a.wav", false);
        tag_core::write_fields(
            &path,
            &Fields::from([("ALBUM".into(), vec!["AC/DC: Live?".into()])]),
        )
        .unwrap();
        fs::create_dir(dir.path().join("Texts")).unwrap();
        fs::write(dir.path().join("Texts/AC_DC_ Live_.txt"), "Lyrics").unwrap();
        let result = run(
            &path,
            vec![
                ActionSpec::ExportCover {
                    filename_template: "Covers\\%album%".into(),
                    picture_type: None,
                    export_duplicates: false,
                },
                ActionSpec::ImportTextFile {
                    field: "LYRICS".into(),
                    filename_template: "%_folderpath%Texts\\%album%.txt".into(),
                },
                ActionSpec::GeneratePlaylists {
                    filename_template: "Lists\\%album%.m3u8".into(),
                    partition_template: String::new(),
                },
            ],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(&dir.path().join("Covers")), ["AC_DC_ Live_.png"]);
        assert_eq!(
            fs::read(dir.path().join("Covers/AC_DC_ Live_.png")).unwrap(),
            png
        );
        assert_eq!(result.entries[0].fields["LYRICS"], ["Lyrics"]);
        assert_eq!(names(&dir.path().join("Lists")), ["AC_DC_ Live_.m3u8"]);
    }

    #[test]
    fn export_loops_follow_the_natural_sort_option() {
        let dir = tempfile::tempdir().unwrap();
        let paths: Vec<String> = ["10", "2", "1"]
            .iter()
            .map(|title| {
                titled(dir.path(), &format!("{title}.wav"), title)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let export = |natural_sort| {
            let result = execute(
                &paths,
                &[ActionGroup {
                    name: "Export".into(),
                    enabled: true,
                    actions: vec![ActionSpec::Export {
                        template: "$loop(%title%)%title% $loopend()".into(),
                        filename_template: "sorted.txt".into(),
                        append: false,
                        partition_template: None,
                    }],
                }],
                EvaluationSettings {
                    natural_sort,
                    export_template_inline: true,
                    ..Default::default()
                },
            );
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            fs::read_to_string(dir.path().join("sorted.txt")).unwrap()
        };
        assert_eq!(export(true), "1 2 10 ");
        assert_eq!(export(false), "1 10 2 ");
    }

    #[test]
    fn a_missing_configuration_offers_unchecked_example_groups() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("actions.json");
        let library = load_library(&path).unwrap();
        assert_eq!(library, example_library());
        assert!(library.groups.len() >= 3);
        for group in &library.groups {
            assert!(!group.enabled, "{}", group.name);
            assert!(group.name.starts_with("Examples#"));
            for action in &group.actions {
                tag_core::actions::validate_action(action).unwrap();
            }
            // Every example can be shared as an .mta file.
            export_group(&dir.path().join("example.mta"), group).unwrap();
        }
        save_library(&path, &library).unwrap();
        // A saved configuration without groups stays empty.
        save_library(&path, &ActionLibrary::default()).unwrap();
        assert!(load_library(&path).unwrap().groups.is_empty());
    }

    #[test]
    fn example_groups_do_what_their_names_say() {
        let dir = tempfile::tempdir().unwrap();
        let (path, _) = covered_fixture(dir.path(), "song.wav", false);
        tag_core::write_fields(
            &path,
            &Fields::from([
                ("TITLE".into(), vec!["  the  long road ".into()]),
                ("ARTIST".into(), vec!["ann;bob".into()]),
                ("TRACK".into(), vec!["3".into()]),
            ]),
        )
        .unwrap();
        let groups: Vec<_> = example_library()
            .groups
            .into_iter()
            .map(|group| ActionGroup {
                enabled: true,
                ..group
            })
            .collect();
        let result = execute(
            &[path.to_string_lossy().into_owned()],
            &groups,
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let entry = &result.entries[0];
        assert_eq!(entry.fields["TITLE"], ["The Long Road"]);
        assert_eq!(entry.fields["ARTIST"], ["Ann", "Bob"]);
        assert!(dir.path().join("03 - The Long Road.wav").is_file());
        assert!(dir.path().join("folder.png").is_file());
    }

    #[test]
    fn bulk_import_reads_files_and_folders_and_reports_each_failure() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("actions");
        fs::create_dir(&folder).unwrap();
        let group = |name: &str| ActionGroup {
            name: name.into(),
            enabled: true,
            actions: vec![ActionSpec::FormatValue {
                field: "TITLE".into(),
                template: name.into(),
            }],
        };
        for name in ["Group 10", "Group 2", "Group 1"] {
            export_group(&folder.join(format!("{name}.mta")), &group(name)).unwrap();
        }
        fs::write(folder.join("notes.txt"), "ignored").unwrap();
        let single = dir.path().join("Single.mta");
        export_group(&single, &group("Single")).unwrap();
        let broken = dir.path().join("Broken.mta");
        fs::write(&broken, b"\xff\xfe[#0\x00").unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir(&empty).unwrap();
        let results = import_groups(
            &[
                single.clone(),
                folder.clone(),
                broken.clone(),
                dir.path().join("missing.mta"),
                empty,
            ],
            None,
        );
        let names: Vec<_> = results
            .iter()
            .map(|r| r.imported.as_ref().map(|i| i.group.name.clone()))
            .collect();
        assert_eq!(
            names,
            [
                Some("Single".to_string()),
                Some("Group 1".into()),
                Some("Group 2".into()),
                Some("Group 10".into()),
                None,
                None,
                None,
            ]
        );
        assert_eq!(
            results[1].imported.as_ref().unwrap().group,
            group("Group 1")
        );
        assert!(results[4..].iter().all(|r| r.error.is_some()));
        assert!(results[6]
            .error
            .as_deref()
            .unwrap()
            .contains("no .mta files"));
    }
}
