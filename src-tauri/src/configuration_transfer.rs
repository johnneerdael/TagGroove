//! Portable saved configuration adapters; recovery lives in profile_migration.
use crate::profile_migration::{self as engine, WindowState, Workspace};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::Manager;

const LIMIT: u64 = crate::utilities::BUNDLE_LIMIT;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub path_style: String,
    pub platform: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub encoding: String,
    pub text_encoding: String,
    pub bytes: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    pub origin: Origin,
    pub files: BTreeMap<String, Value>,
    pub export_assets: BTreeMap<String, Asset>,
    pub window: Option<WindowState>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub files: Option<Vec<String>>,
    #[serde(default = "yes")]
    pub preferences: bool,
    #[serde(default = "yes")]
    pub workspace: bool,
    #[serde(default = "yes")]
    pub export_assets: bool,
}
fn yes() -> bool {
    true
}
impl Default for Options {
    fn default() -> Self {
        Self {
            files: None,
            preferences: true,
            workspace: true,
            export_assets: true,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Serialize)]
pub struct SourceWorkspace {
    pub rows: usize,
    pub roots: Vec<String>,
    pub path_style: String,
}
#[derive(Debug, Serialize)]
pub struct Preview {
    pub token: String,
    pub version: u32,
    pub source_hash: String,
    pub files: Vec<String>,
    pub export_assets: Vec<String>,
    pub preference_keys: Vec<String>,
    pub preferences_available: bool,
    pub source_workspace: Option<SourceWorkspace>,
    pub workspace: Option<Workspace>,
    pub workspace_available: bool,
    pub unavailable: Vec<String>,
    pub warnings: Vec<String>,
    pub replaces_snapshot: bool,
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub version: u32,
    pub files: usize,
    pub export_assets: usize,
    pub preferences: usize,
    pub workspace_rows: usize,
    pub omitted: Vec<String>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn quick_action_shape(name: &str, saved: &Value) -> bool {
    // Complete legacy partial memory only for structural deserialization. These
    // neutral values are never persisted, evaluated or used as UI defaults.
    let mut shape = match name {
        "adjust_cover" => serde_json::json!({"format":"original","quality":90,"resize":null}),
        "case_conversion" => {
            serde_json::json!({"field":"TITLE","mode":"mixed","word_separators":""})
        }
        "convert_codepage" => serde_json::json!({"field":"TITLE","codepage":"1251"}),
        "export" => {
            serde_json::json!({"template":"","filename_template":"","append":false,"partition_template":null})
        }
        "export_cover" => {
            serde_json::json!({"filename_template":"","picture_type":null,"export_duplicates":false})
        }
        "format_value" => serde_json::json!({"field":"TITLE","template":""}),
        "generate_playlists" => serde_json::json!({"filename_template":"","partition_template":""}),
        "guess_values" => serde_json::json!({"source":"","pattern":""}),
        "import_cover" => {
            serde_json::json!({"filename_template":"","picture_type":null,"delete_existing":false})
        }
        "import_text_file" => serde_json::json!({"field":"TITLE","filename_template":""}),
        "merge_duplicate_fields" | "split_field" => {
            serde_json::json!({"field":"TITLE","separator":""})
        }
        "remove_duplicate_fields" => serde_json::json!({"field":"TITLE","only_same_content":false}),
        "remove_fields" | "remove_fields_except" => serde_json::json!({"fields":[]}),
        "replace" => {
            serde_json::json!({"field":"TITLE","from":"","to":"","whole_word":false,"case_sensitive":false})
        }
        "regex_replace" => {
            serde_json::json!({"field":"TITLE","pattern":"","replacement":"","case_sensitive":false})
        }
        "set_cover_properties" => {
            serde_json::json!({"picture_type":null,"description":null,"only_picture_type":null})
        }
        _ => return false,
    };
    let Some(saved) = saved.as_object() else {
        return false;
    };
    if saved
        .get("type")
        .is_some_and(|value| value.as_str() != Some(name))
    {
        return false;
    }
    shape["type"] = name.into();
    shape.as_object_mut().unwrap().extend(
        saved
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    serde_json::from_value::<tag_core::actions::ActionSpec>(shape).is_ok()
}

/// Validate stored values without running a template, action, tool or provider.
fn validate_preferences(values: &BTreeMap<String, String>) -> Result<(), String> {
    for (key, text) in values {
        if !engine::PREFERENCE_KEYS.contains(&key.as_str())
            || text.len() > engine::WINDOW_LIMIT
            || text.contains('\0')
        {
            return Err(format!(
                "Unsupported or oversized configuration preference: {key}"
            ));
        }
        let json = || serde_json::from_str::<Value>(text).map_err(|e| format!("{key}: {e}"));
        let valid = match key.as_str() {
            "opentag.last-version" => true,
            "opentag.language" => {
                !text.is_empty()
                    && text.len() <= 128
                    && text
                        .chars()
                        .all(|c| c.is_alphanumeric() || "-_.".contains(c))
            }
            "opentag.appearance" => ["system", "light", "dark"].contains(&text.as_str()),
            "opentag.filter-auto-apply" | "opentag.filter-shown" => {
                ["true", "false"].contains(&text.as_str())
            }
            "opentag.panel-width" => text.parse::<f64>().is_ok_and(|v| v.is_finite() && v > 0.0),
            "opentag.cover.aspect" => ["correct", "stretch"].contains(&text.as_str()),
            "opentag.cover.extractFormat" => true,
            "opentag.tagSources.last"
            | "opentag.tagSources.lastSource"
            | "opentag.tagSources.lastBuiltIn" => {
                !text.is_empty() && text.len() <= 4096 && !text.contains(['\r', '\n'])
            }
            "opentag.tagSources.suggestSorting" => ["", "title", "length"].contains(&text.as_str()),
            "opentag.tagSources.saveCover" => ["1", "0"].contains(&text.as_str()),
            key if key.starts_with("opentag.tagSources.disabled.") => json()?
                .as_array()
                .is_some_and(|values| values.iter().all(Value::is_string)),
            "opentag.tag-tag" => {
                let value = json()?;
                value.as_object().is_some_and(|v| {
                    v.iter()
                        .all(|(k, v)| ["field", "template"].contains(&k.as_str()) && v.is_string())
                })
            }
            "opentag.numbering" => {
                let value = json()?;
                value.as_object().is_some_and(|values| {
                    values.iter().all(|(key, v)| match key.as_str() {
                        "track_start" | "disc_start" | "disc_total_value" => v
                            .as_u64()
                            .is_some_and(|n| (1..=u32::MAX as u64).contains(&n)),
                        "track_digits" | "disc_digits" => {
                            v.as_u64().is_some_and(|n| (1..=10).contains(&n))
                        }
                        "next_disc_on_field" | "reset_disc_on_field" => v.is_string(),
                        "track_total"
                        | "disc_enabled"
                        | "disc_total"
                        | "next_disc_on_directory"
                        | "reset_track_on_next_disc" => v.is_boolean(),
                        _ => false,
                    })
                })
            }
            "opentag.main-layout" => {
                let value = json()?;
                value.as_object().is_some_and(|values| {
                    values.iter().all(|(key, v)| match key.as_str() {
                        "dock" | "last_dock" => v.as_str().is_some_and(|s| {
                            ["left", "right", "top", "bottom", "float"].contains(&s)
                        }),
                        "filter" => v.as_str().is_some_and(|s| {
                            ["list", "toolbar-left", "toolbar-right"].contains(&s)
                        }),
                        "width" | "height" => v.as_f64().is_some_and(f64::is_finite),
                        "float" => v.as_object().is_some_and(|r| {
                            r.iter().all(|(k, v)| {
                                ["x", "y", "width", "height"].contains(&k.as_str())
                                    && v.as_f64().is_some_and(f64::is_finite)
                            })
                        }),
                        _ => false,
                    })
                })
            }
            "opentag.actions.quick" => {
                let value = json()?;
                let object = value
                    .as_object()
                    .ok_or_else(|| format!("{key}: expected quick-action settings"))?;
                let actions = object
                    .get("actions")
                    .and_then(Value::as_object)
                    .ok_or_else(|| format!("{key}: expected saved action settings"))?;
                // Older quick memory may contain partial actions; its consumer fills
                // missing values from the current defaults. Preserve that contract.
                let action_names = [
                    "adjust_cover",
                    "case_conversion",
                    "convert_codepage",
                    "export",
                    "export_cover",
                    "format_value",
                    "generate_playlists",
                    "guess_values",
                    "import_cover",
                    "import_text_file",
                    "merge_duplicate_fields",
                    "remove_duplicate_fields",
                    "remove_fields",
                    "remove_fields_except",
                    "replace",
                    "regex_replace",
                    "set_cover_properties",
                    "split_field",
                ];
                object
                    .keys()
                    .all(|k| ["actions", "last", "groupCodepage"].contains(&k.as_str()))
                    && object
                        .get("last")
                        .is_none_or(|v| v.as_str().is_some_and(|s| action_names.contains(&s)))
                    && object.get("groupCodepage").is_none_or(|v| {
                        v.as_str().is_some_and(|s| {
                            [
                                "1250", "1251", "1252", "1253", "1254", "1255", "1256", "1257",
                                "1258", "874", "437", "720", "737", "775", "850", "852", "855",
                                "857", "858", "860", "861", "862", "863", "864", "865", "866",
                                "869", "932", "936", "949", "950", "54936", "51932", "20866",
                                "21866", "10000", "10007", "28591", "28592", "28593", "28594",
                                "28595", "28596", "28597", "28598", "28603", "28605", "65001",
                            ]
                            .contains(&s)
                        })
                    })
                    && actions
                        .iter()
                        .all(|(name, value)| quick_action_shape(name, value))
            }
            _ => false,
        };
        if !valid {
            return Err(format!("Invalid saved configuration preference: {key}"));
        }
    }
    Ok(())
}

fn asset_name(name: &str) -> Result<(), String> {
    engine::relative(name)?;
    if name.contains('/') || name.starts_with('.') || !name.to_ascii_lowercase().ends_with(".mte") {
        return Err(format!("Unsupported portable export asset: {name}"));
    }
    Ok(())
}
fn text_encoding(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) {
        "utf-16le".into()
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        "utf-16be".into()
    } else if std::str::from_utf8(bytes).is_ok() {
        "utf-8".into()
    } else {
        crate::evaluation::EvaluationSettings::default()
            .ansi_codepage()
            .unwrap_or_else(|| "windows-1252".into())
    }
}
fn decode_asset(name: &str, asset: &Asset) -> Result<Vec<u8>, String> {
    asset_name(name)?;
    if asset.encoding != "base64" || asset.bytes.len() as u64 > LIMIT * 2 {
        return Err(format!(
            "Unsupported or oversized export asset encoding: {name}"
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&asset.bytes)
        .map_err(|e| format!("{name}: {e}"))?;
    if bytes.len() as u64 > LIMIT || digest(&bytes) != asset.sha256 {
        return Err(format!("Export asset checksum or size mismatch: {name}"));
    }
    crate::exports::decode_recorded(&bytes, &asset.text_encoding)
        .map_err(|e| format!("{name}: {e}"))?;
    Ok(bytes)
}

fn validate_native(files: &engine::Files, physical: bool) -> Result<(), String> {
    let stage = tempfile::tempdir().map_err(|e| e.to_string())?;
    engine::write_stage(stage.path(), files)?;
    crate::exports::encoding_overrides(&stage.path().join("export"))?;
    for name in crate::utilities::CONFIGURATION {
        if files.contains_key(name) {
            if name == "exports.json" && !physical {
                crate::exports::validate_metadata(&files[name])?;
            } else {
                crate::utilities::validate_file(name, &stage.path().join(name))?;
            }
        }
    }
    Ok(())
}
fn serial_bytes(bundle: &Bundle) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec_pretty(bundle).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("The configuration file is too large (32 MB). Nothing was truncated.".into());
    }
    Ok(bytes)
}
fn capture(directory: &Path, mut window: WindowState) -> Result<Bundle, String> {
    engine::validate_window(&window)?;
    if window.version != 2 {
        return Err("Portable backups require window-state version 2.".into());
    }
    validate_preferences(&window.preferences)?;
    let state = engine::inspect_status(directory)?;
    if state.pending.is_some() || ["applying", "restoring"].contains(&state.phase.as_str()) {
        return Err("Complete the pending configuration recovery before saving a backup.".into());
    }
    let native = engine::native(directory)?;
    validate_native(&native, true)?;
    let mut files = crate::utilities::collect(directory)?.files;
    // Reflect the currently loaded export metadata, including externally deleted
    // discovered entries, without modifying or compiling any physical template.
    if files.contains_key("exports.json") {
        let library = crate::exports::load(&directory.join("exports.json"))?;
        let mut metadata = serde_json::to_value(library).map_err(|e| e.to_string())?;
        metadata.as_object_mut().unwrap().remove("observed_files");
        metadata.as_object_mut().unwrap().remove("pending_removals");
        files.insert("exports.json".into(), metadata);
    }
    let mut export_assets = BTreeMap::new();
    let encodings = crate::exports::encoding_overrides(&directory.join("export"))?;
    let mut cases = BTreeSet::new();
    for (path, bytes) in &native {
        let Some(name) = path.strip_prefix("export/") else {
            continue;
        };
        if name.contains('/')
            || name.starts_with('.')
            || !name.to_ascii_lowercase().ends_with(".mte")
        {
            continue;
        }
        asset_name(name)?;
        if !cases.insert(name.to_lowercase()) {
            return Err(format!(
                "Duplicate portable export asset ignoring case: {name}"
            ));
        }
        let asset = Asset {
            encoding: "base64".into(),
            text_encoding: crate::exports::matching_encoding(&encodings, name, bytes)
                .map(str::to_owned)
                .unwrap_or_else(|| text_encoding(bytes)),
            bytes: base64::engine::general_purpose::STANDARD.encode(bytes),
            sha256: digest(bytes),
        };
        decode_asset(name, &asset)?;
        export_assets.insert(name.into(), asset);
    }
    window.preferences.remove("opentag.last-version");
    let bundle = Bundle {
        format: crate::utilities::BUNDLE_FORMAT.into(),
        version: 2,
        origin: Origin {
            path_style: if cfg!(unix) {
                "posix"
            } else {
                "unsupported-local"
            }
            .into(),
            platform: std::env::consts::OS.into(),
        },
        files,
        export_assets,
        window: Some(window),
    };
    serial_bytes(&bundle)?;
    if engine::native(directory)? != native {
        return Err("Saved configuration changed during backup capture; try again.".into());
    }
    Ok(bundle)
}

fn parse(bytes: &[u8]) -> Result<(Bundle, Vec<String>), String> {
    if bytes.len() as u64 > LIMIT {
        return Err("The configuration file is too large (32 MB).".into());
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Not a TagGroove configuration: {e}"))?;
    if value.get("format").and_then(Value::as_str) != Some(crate::utilities::BUNDLE_FORMAT) {
        return Err("Not a TagGroove configuration file".into());
    }
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or("Configuration version is missing.")?;
    let mut bundle = match version {
        1 => {
            let legacy: crate::utilities::Bundle =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            if legacy.files.is_empty() {
                return Err("The configuration file contains no settings".into());
            }
            Bundle {
                format: legacy.format,
                version: 1,
                origin: Origin {
                    path_style: "legacy".into(),
                    platform: "unknown".into(),
                },
                files: legacy.files,
                export_assets: BTreeMap::new(),
                window: None,
            }
        }
        2 => serde_json::from_value(value)
            .map_err(|e| format!("Invalid portable configuration: {e}"))?,
        _ => return Err(format!("Configuration version {version} is not supported")),
    };
    if bundle.files.len() + bundle.export_assets.len() > 4096 {
        return Err("The portable configuration contains more than 4096 files.".into());
    }
    let mut warnings = Vec::new();
    // A future optional asset kind must not prevent restoring supported settings.
    // Unsafe paths still fail before any component can be selected or published.
    let unsupported: Vec<_> = bundle
        .export_assets
        .keys()
        .filter_map(|name| {
            if name.contains('/')
                || name.starts_with('.')
                || !name.to_ascii_lowercase().ends_with(".mte")
            {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();
    for name in unsupported {
        engine::relative(&name)?;
        bundle.export_assets.remove(&name);
        warnings.push(format!(
            "Unsupported portable asset was not selected: {name}"
        ));
    }
    for name in bundle.files.keys() {
        if !crate::utilities::CONFIGURATION.contains(&name.as_str()) {
            return Err(format!("{name} is not a TagGroove configuration file"));
        }
    }
    let mut cases = BTreeSet::new();
    let mut decoded = engine::Files::new();
    for (name, asset) in &bundle.export_assets {
        if !cases.insert(name.to_lowercase()) {
            return Err(format!(
                "Duplicate portable export asset ignoring case: {name}"
            ));
        }
        decoded.insert(format!("export/{name}"), decode_asset(name, asset)?);
    }
    for (name, value) in &bundle.files {
        decoded.insert(
            name.clone(),
            serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
        );
    }
    validate_native(&decoded, false)?;
    if let Some(window) = &mut bundle.window {
        engine::validate_window_shape(window)?;
        if window.version != 2
            || window
                .workspace
                .as_ref()
                .is_some_and(|w| w.version != Some(2))
        {
            return Err(
                "Portable configuration requires window-state and workspace version 2.".into(),
            );
        }
        let unknown: Vec<_> = window
            .preferences
            .keys()
            .filter(|key| !engine::PREFERENCE_KEYS.contains(&key.as_str()))
            .cloned()
            .collect();
        for key in unknown {
            window.preferences.remove(&key);
            warnings.push(format!("Unsupported preference was not selected: {key}"));
        }
        window.preferences.remove("opentag.last-version");
        validate_preferences(&window.preferences)?;
        // A foreign/nonlocal workspace does not prevent transporting saved settings.
        let preferences = WindowState {
            version: 2,
            preferences: window.preferences.clone(),
            workspace: None,
        };
        engine::validate_window(&preferences)?;
    }
    Ok((bundle, warnings))
}

fn local(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains('\0')
        && !path.contains("://")
        && !Path::new(path).components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}
fn remap(workspace: &Workspace, mappings: &[Mapping]) -> Result<Workspace, String> {
    for (i, mapping) in mappings.iter().enumerate() {
        if !local(&mapping.from) || !local(&mapping.to) {
            return Err(
                "Workspace mappings require absolute local POSIX directories without traversal."
                    .into(),
            );
        }
        for other in &mappings[..i] {
            if Path::new(&mapping.from).starts_with(&other.from)
                || Path::new(&other.from).starts_with(&mapping.from)
            {
                return Err("Workspace source mappings overlap; choose unambiguous roots.".into());
            }
        }
    }
    // Scanning returns canonical local row identities. Resolve the selected
    // destination root before deriving them, including /var and symlink aliases.
    let targets: Vec<_> = mappings
        .iter()
        .map(|mapping| engine::canonical_target(Path::new(&mapping.to)))
        .collect::<Result<_, _>>()?;
    let map = |path: &str| -> String {
        mappings
            .iter()
            .zip(&targets)
            .find_map(|(mapping, target)| {
                Path::new(path)
                    .strip_prefix(&mapping.from)
                    .ok()
                    .map(|rest| {
                        if rest.as_os_str().is_empty() {
                            target.clone()
                        } else {
                            target.join(rest)
                        }
                        .to_string_lossy()
                        .into_owned()
                    })
            })
            .unwrap_or_else(|| path.into())
    };
    let mut mapped = workspace.clone();
    mapped.paths = mapped.paths.iter().map(|p| map(p)).collect();
    mapped.selected = mapped.selected.iter().map(|p| map(p)).collect();
    mapped.roots = mapped.roots.iter().map(|p| map(p)).collect();
    mapped.anchor = mapped.anchor.as_deref().map(map);
    mapped.playlist = mapped.playlist.as_deref().map(map);
    mapped.working_directory = mapped.working_directory.as_deref().map(map);
    mapped.source_order = mapped
        .source_order
        .as_ref()
        .map(|order| order.iter().map(|p| map(p)).collect());
    if mapped.location_is_data {
        mapped.location = map(&mapped.location);
    }
    Ok(mapped)
}
#[derive(Clone, Debug, PartialEq)]
struct Stamp {
    canonical: PathBuf,
    bytes: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    let canonical = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let metadata = std::fs::metadata(&canonical).map_err(|e| e.to_string())?;
    Ok(Stamp {
        canonical,
        bytes: metadata.len(),
        modified: metadata.modified().ok(),
        #[cfg(unix)]
        device: {
            use std::os::unix::fs::MetadataExt;
            metadata.dev()
        },
        #[cfg(unix)]
        inode: {
            use std::os::unix::fs::MetadataExt;
            metadata.ino()
        },
    })
}
fn workspace_state(
    workspace: &Workspace,
    tags: &tag_core::TagOptions,
) -> (Vec<String>, BTreeMap<String, Stamp>) {
    let mut unavailable = Vec::new();
    let mut stamps = BTreeMap::new();
    let mut physical = BTreeSet::new();
    for row in &workspace.paths {
        let file = tag_core::subsong_parts(row).map_or(row.as_str(), |(file, _)| file);
        if !local(file) || crate::documents::is_managed(Path::new(file)) {
            unavailable.push(format!("Workspace row is not a portable local file: {row}"));
            continue;
        }
        physical.insert(file.to_owned());
    }
    for path in physical
        .iter()
        .chain(&workspace.roots)
        .chain(workspace.playlist.iter())
        .chain(workspace.working_directory.iter())
    {
        match stamp(Path::new(path)) {
            Ok(value) => {
                stamps.insert(path.clone(), value);
            }
            Err(error) => unavailable.push(error),
        }
    }
    if unavailable.is_empty() && !physical.is_empty() {
        let options = crate::scan_read_options(tags, true);
        let result = tag_core::scan_paths_with_options(&workspace.paths, false, &options, None);
        let found: BTreeSet<_> = result.entries.iter().map(|e| e.path.as_str()).collect();
        unavailable.extend(
            result
                .errors
                .into_iter()
                .map(|e| format!("{}: {}", e.path, e.error)),
        );
        for row in &workspace.paths {
            if !found.contains(row.as_str()) {
                unavailable.push(format!("Workspace row could not be read: {row}"));
            }
        }
    }
    for (path, before) in &stamps {
        if stamp(Path::new(path)).as_ref() != Ok(before) {
            unavailable.push(format!("Workspace changed during preview: {path}"));
        }
    }
    (unavailable, stamps)
}
#[derive(Clone)]
struct Plan {
    path: PathBuf,
    identity: engine::SourceIdentity,
    source_hash: String,
    target_hash: String,
    before_window: WindowState,
    desired_window: WindowState,
    after: engine::Files,
    unavailable: Vec<String>,
    stamps: BTreeMap<String, Stamp>,
    summary: Value,
}
static PLANS: OnceLock<Mutex<BTreeMap<String, Plan>>> = OnceLock::new();
fn preview(
    directory: &Path,
    path: &Path,
    window: WindowState,
    options: Options,
    mappings: Vec<Mapping>,
) -> Result<Preview, String> {
    engine::validate_window(&window)?;
    let identity = engine::source_identity(directory, path)?;
    let bytes = engine::read(path, LIMIT)?;
    let (bundle, mut warnings) = parse(&bytes)?;
    let metadata = engine::inspect_status(directory)?;
    if metadata.pending.is_some() || ["applying", "restoring"].contains(&metadata.phase.as_str()) {
        return Err(
            "Complete the pending configuration recovery before loading another backup.".into(),
        );
    }
    let before = engine::native(directory)?;
    let selected: Vec<String> = options
        .files
        .clone()
        .unwrap_or_else(|| bundle.files.keys().cloned().collect());
    let unique: BTreeSet<_> = selected.iter().collect();
    if unique.len() != selected.len()
        || selected.iter().any(|name| !bundle.files.contains_key(name))
    {
        return Err("Configuration selection contains duplicate or unseen files.".into());
    }
    let mut after = before.clone();
    for name in &selected {
        after.insert(
            name.clone(),
            serde_json::to_vec_pretty(&bundle.files[name]).map_err(|e| e.to_string())?,
        );
    }
    let mut assets = Vec::new();
    if options.export_assets {
        let mut encodings = crate::exports::encoding_overrides(&directory.join("export"))?;
        for (name, asset) in &bundle.export_assets {
            let existing: Vec<_> = after
                .keys()
                .filter(|path| {
                    path.strip_prefix("export/")
                        .is_some_and(|old| old.eq_ignore_ascii_case(name))
                })
                .cloned()
                .collect();
            for old in existing {
                if old != format!("export/{name}") {
                    return Err(format!(
                        "Export asset collides with existing filename case: {name}"
                    ));
                }
            }
            if before.contains_key(&format!("export/{name}")) {
                warnings.push(format!("Existing export template will be replaced: {name}"));
            }
            after.insert(format!("export/{name}"), decode_asset(name, asset)?);
            if let Some(conflict) = encodings
                .keys()
                .find(|key| key.to_lowercase() == name.to_lowercase() && *key != name)
            {
                return Err(format!(
                    "Export encoding filename case conflicts with incoming asset: {conflict}"
                ));
            }
            encodings.insert(
                name.clone(),
                crate::exports::EncodingOverride {
                    sha256: asset.sha256.clone(),
                    text_encoding: asset.text_encoding.clone(),
                },
            );
            assets.push(name.clone());
        }
        if !assets.is_empty() {
            after.insert(
                format!("export/{}", crate::exports::ENCODING_SIDECAR),
                crate::exports::encoding_bytes(&encodings)?,
            );
        }
    }
    validate_native(&after, true)?;
    let mut desired = window.clone();
    let mut workspace = None;
    let mut unavailable = Vec::new();
    let mut stamps = BTreeMap::new();
    let mut keys = Vec::new();
    if let Some(incoming) = &bundle.window {
        if let Some(locale) = incoming.preferences.get("opentag.language") {
            if !["system", "en", "nl", "de"].contains(&locale.as_str())
                && !directory
                    .join("languages")
                    .join(format!("{locale}.json"))
                    .is_file()
            {
                warnings.push(format!("The selected language catalog is unavailable; the built-in fallback will be used: {locale}"));
            }
        }
        if let Some(id) = incoming
            .preferences
            .get("opentag.tagSources.lastSource")
            .and_then(|source| source.strip_prefix("web:"))
        {
            if engine::relative(id).is_err() || !directory.join("data/sources").join(id).is_file() {
                warnings.push(format!("Remembered web source is not installed: {id}"));
            }
        }
        if options.preferences {
            let bookkeeping = desired.preferences.get("opentag.last-version").cloned();
            desired.preferences = incoming.preferences.clone();
            if let Some(value) = bookkeeping {
                desired
                    .preferences
                    .insert("opentag.last-version".into(), value);
            }
            keys = incoming.preferences.keys().cloned().collect();
        }
        if let Some(incoming_workspace) = &incoming.workspace {
            if bundle.origin.path_style != "posix" {
                unavailable.push(format!(
                    "Workspace path style is unsupported: {}",
                    bundle.origin.path_style
                ));
            } else {
                let mapped = remap(incoming_workspace, &mappings)?;
                let checked = WindowState {
                    version: 2,
                    preferences: desired.preferences.clone(),
                    workspace: Some(mapped.clone()),
                };
                if let Err(error) = engine::validate_window(&checked) {
                    unavailable.push(error);
                } else {
                    let stage = tempfile::tempdir().map_err(|e| e.to_string())?;
                    engine::write_stage(stage.path(), &after)?;
                    let tags = crate::settings::load(&stage.path().join("settings.json"))
                        .0
                        .tags;
                    let state = workspace_state(&mapped, &tags);
                    unavailable = state.0;
                    stamps = state.1;
                    for mapping in &mappings {
                        match stamp(Path::new(&mapping.to)) {
                            Ok(value) => {
                                stamps.insert(mapping.to.clone(), value);
                            }
                            Err(error) => unavailable.push(error),
                        }
                    }
                    workspace = Some(mapped.clone());
                    if options.workspace {
                        desired.version = 2;
                        desired.workspace = Some(mapped);
                    }
                }
            }
        }
    }
    if !options.workspace {
        stamps.clear();
    }
    let workspace_available = workspace.is_some() && unavailable.is_empty();
    let pending_unavailable = if options.workspace {
        unavailable.clone()
    } else {
        Vec::new()
    };
    if options.workspace
        && bundle
            .window
            .as_ref()
            .and_then(|w| w.workspace.as_ref())
            .is_some()
        && !workspace_available
        && pending_unavailable.is_empty()
    {
        return Err("The selected workspace cannot be restored.".into());
    }
    let rows = if options.workspace && workspace_available {
        workspace.as_ref().map_or(0, |w| w.paths.len())
    } else {
        0
    };
    let mut omitted = vec![
        "Unsaved tag, artwork and action-editor values are not configuration backup data."
            .to_string(),
    ];
    omitted.extend(
        warnings
            .iter()
            .filter(|warning| warning.starts_with("Unsupported"))
            .cloned(),
    );
    if !options.workspace
        || bundle
            .window
            .as_ref()
            .and_then(|w| w.workspace.as_ref())
            .is_none()
    {
        omitted.push(
            "Incoming workspace was not selected; the current captured workspace is preserved."
                .into(),
        );
    }
    let summary = serde_json::to_value(Summary {
        version: bundle.version,
        files: selected.len(),
        export_assets: assets.len(),
        preferences: keys.len(),
        workspace_rows: rows,
        omitted,
    })
    .map_err(|e| e.to_string())?;
    let token = uuid::Uuid::new_v4().to_string();
    let source_hash = digest(&bytes);
    let plan = Plan {
        path: path.into(),
        identity,
        source_hash: source_hash.clone(),
        target_hash: engine::hash(&before, Some(&window))?,
        before_window: window,
        desired_window: desired,
        after,
        unavailable: pending_unavailable,
        stamps,
        summary,
    };
    let mut plans = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?;
    plans.clear();
    plans.insert(token.clone(), plan);
    Ok(Preview {
        token,
        version: bundle.version,
        source_hash,
        files: selected,
        export_assets: assets,
        preference_keys: keys,
        preferences_available: bundle.window.is_some(),
        source_workspace: bundle
            .window
            .as_ref()
            .and_then(|window| window.workspace.as_ref())
            .map(|workspace| SourceWorkspace {
                rows: workspace.paths.len(),
                roots: workspace.roots.clone(),
                path_style: bundle.origin.path_style.clone(),
            }),
        workspace,
        workspace_available,
        unavailable,
        warnings,
        replaces_snapshot: metadata.report.is_some(),
    })
}
fn apply(directory: &Path, token: &str, window: WindowState) -> Result<engine::Status, String> {
    let plan = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?
        .remove(token)
        .ok_or("Configuration preview expired; inspect the backup again.")?;
    if engine::source_identity(directory, &plan.path)? != plan.identity
        || digest(&engine::read(&plan.path, LIMIT)?) != plan.source_hash
    {
        return Err("The backup changed after preview; inspect it again.".into());
    }
    engine::validate_window(&window)?;
    let before = engine::native(directory)?;
    if window != plan.before_window || engine::hash(&before, Some(&window))? != plan.target_hash {
        return Err(
            "The current configuration or workspace changed after preview; inspect it again."
                .into(),
        );
    }
    if !plan.unavailable.is_empty() {
        return Err(format!(
            "The selected workspace is unavailable: {}",
            plan.unavailable.join("; ")
        ));
    }
    for (path, expected) in &plan.stamps {
        if stamp(Path::new(path))? != *expected {
            return Err(format!("Workspace files changed after preview: {path}"));
        }
    }
    let report = engine::Report {
        legacy_encoding: None,
        items: Vec::new(),
        imported: plan
            .after
            .iter()
            .filter(|(name, bytes)| before.get(*name) != Some(*bytes))
            .map(|(name, _)| name.clone())
            .collect(),
        kept: Vec::new(),
        source_hash: plan.source_hash,
        operation: Some("configuration".into()),
        configuration: Some(plan.summary),
    };
    engine::publish_configuration(
        directory,
        before,
        plan.after,
        window,
        plan.desired_window,
        report,
    )
}
fn summary(bundle: &Bundle) -> Summary {
    Summary {
        version: bundle.version,
        files: bundle.files.len(),
        export_assets: bundle.export_assets.len(),
        preferences: bundle.window.as_ref().map_or(0, |w| w.preferences.len()),
        workspace_rows: bundle
            .window
            .as_ref()
            .and_then(|w| w.workspace.as_ref())
            .map_or(0, |w| w.paths.len()),
        omitted: vec!["Unsaved tag, artwork and action-editor values are excluded.".into()],
    }
}

fn check_backup_output(
    directory: &Path,
    output_path: &Path,
    window: &WindowState,
    protected_roots: &[PathBuf],
) -> Result<(), String> {
    let output = engine::canonical_target(output_path)?;
    let config = engine::canonical_target(directory)?;
    if output.starts_with(config)
        || protected_roots
            .iter()
            .map(|root| engine::canonical_target(root))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|root| output.starts_with(root))
    {
        return Err("Save the backup outside the active configuration and recovery folder.".into());
    }
    #[cfg(unix)]
    let output_identity = std::fs::metadata(output_path).ok().map(|metadata| {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino())
    });
    if let Some(workspace) = &window.workspace {
        for row in &workspace.paths {
            let container = tag_core::subsong_parts(row).map_or(row.as_str(), |(file, _)| file);
            // A CUE row's container and its actual referenced audio are separate
            // identities. An unreadable/changed sheet cannot be guessed safely.
            let audio = tag_core::subsong_file(Path::new(row))?;
            for physical in [Path::new(container), audio.as_path()] {
                let alias = engine::canonical_target(physical)? == output;
                #[cfg(unix)]
                let alias = alias
                    || output_identity.is_some_and(|identity| {
                        std::fs::metadata(physical).ok().is_some_and(|metadata| {
                            use std::os::unix::fs::MetadataExt;
                            (metadata.dev(), metadata.ino()) == identity
                        })
                    });
                if alias {
                    return Err("A configuration backup cannot replace an audio file.".into());
                }
            }
        }
    }
    Ok(())
}

fn export_to(
    directory: &Path,
    output_path: &Path,
    window: WindowState,
    protected_roots: &[PathBuf],
) -> Result<Summary, String> {
    check_backup_output(directory, output_path, &window, protected_roots)?;
    let bundle = capture(directory, window.clone())?;
    let bytes = serial_bytes(&bundle)?;
    check_backup_output(directory, output_path, &window, protected_roots)?;
    crate::action_host::atomic_output(output_path, &bytes, false)?;
    Ok(summary(&bundle))
}

fn require_desktop() -> Result<(), String> {
    if cfg!(mobile) {
        Err("Saved settings and local workspace transfer is available in the desktop app; mobile document and provider transfer is not supported.".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn configuration_transfer_export(
    app: tauri::AppHandle,
    output_path: String,
    window: WindowState,
) -> Result<Summary, String> {
    require_desktop()?;
    let directory = crate::utilities::configuration_directory(&app)?;
    let protected_roots = vec![
        app.path().app_data_dir().map_err(|e| e.to_string())?,
        app.path().app_cache_dir().map_err(|e| e.to_string())?,
        app.path().app_local_data_dir().map_err(|e| e.to_string())?,
    ];
    crate::io(move || {
        let _guard = engine::CONFIGURATION_LOCK
            .lock()
            .map_err(|e| e.to_string())?;
        export_to(
            &directory,
            Path::new(&output_path),
            window,
            &protected_roots,
        )
    })
    .await
}

#[tauri::command]
pub async fn configuration_transfer_preview(
    app: tauri::AppHandle,
    path: String,
    window: WindowState,
    options: Option<Options>,
    mappings: Option<Vec<Mapping>>,
) -> Result<Preview, String> {
    require_desktop()?;
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || {
        let _guard = engine::CONFIGURATION_LOCK
            .lock()
            .map_err(|e| e.to_string())?;
        let path = crate::auxiliary_input(&path)?;
        preview(
            &directory,
            &path,
            window,
            options.unwrap_or(Options {
                files: None,
                preferences: true,
                workspace: true,
                export_assets: true,
            }),
            mappings.unwrap_or_default(),
        )
    })
    .await
}
#[tauri::command]
pub async fn configuration_transfer_apply(
    app: tauri::AppHandle,
    token: String,
    window: WindowState,
) -> Result<engine::Status, String> {
    require_desktop()?;
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || {
        let _guard = engine::CONFIGURATION_LOCK
            .lock()
            .map_err(|e| e.to_string())?;
        let state = apply(&directory, &token, window)?;
        crate::settings::reload(&app)?;
        Ok(state)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    static TESTS: Mutex<()> = Mutex::new(());

    fn window() -> WindowState {
        serde_json::from_value(serde_json::json!({"version":2,"preferences":{
            "opentag.tag-tag":"{\"field\":\"TITLE\",\"template\":\"%title%\"}",
            "opentag.actions.quick":"{\"actions\":{},\"groupCodepage\":\"1251\"}",
            "opentag.last-version":"destination-bookkeeping"
        },"workspace":{"version":2,"paths":[],"selected":[],"roots":[],"working_directory":null,"list_order":"manual","source_order":null,"location":"","location_is_data":false,"filter":"","filter_draft":null,"anchor":null,"playlist":null}})).unwrap()
    }

    #[test]
    fn portable_capture_keeps_unfinished_raw_mte_bytes_and_excludes_private_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("export")).unwrap();
        let raw: Vec<u8> = [0xff, 0xfe]
            .into_iter()
            .chain("$loop(".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        std::fs::write(dir.path().join("export/Unfinished.mte"), &raw).unwrap();
        std::fs::write(dir.path().join("export/notes.txt"), "not a template").unwrap();
        std::fs::write(
            dir.path().join("tag-sources.json"),
            "private credentials stay excluded",
        )
        .unwrap();
        let bundle = capture(dir.path(), window()).unwrap();
        assert_eq!(bundle.version, 2);
        assert!(!bundle.files.contains_key("tag-sources.json"));
        assert_eq!(bundle.export_assets.len(), 1);
        let asset = &bundle.export_assets["Unfinished.mte"];
        assert_eq!(asset.text_encoding, "utf-16le");
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(&asset.bytes)
                .unwrap(),
            raw
        );
        assert_eq!(asset.sha256, format!("{:x}", Sha256::digest(&raw)));
        assert!(!bundle
            .window
            .unwrap()
            .preferences
            .contains_key("opentag.last-version"));
    }

    #[test]
    fn portable_asset_only_restore_retains_encoding_sidecar_and_raw_bytes_across_load_save() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        std::fs::create_dir(source.join("export")).unwrap();
        std::fs::create_dir(target.join("export")).unwrap();
        let raw = b"\xcf\xf0\xe8\xe2\xe5\xf2";
        let other = b"caf\xe9";
        std::fs::write(source.join("export/Cyrillic.mte"), raw).unwrap();
        std::fs::write(target.join("export/Other.mte"), other).unwrap();
        let source_map = BTreeMap::from([(
            "Cyrillic.mte".into(),
            crate::exports::EncodingOverride {
                sha256: digest(raw),
                text_encoding: "windows-1251".into(),
            },
        )]);
        let target_map = BTreeMap::from([(
            "Other.mte".into(),
            crate::exports::EncodingOverride {
                sha256: digest(other),
                text_encoding: "windows-1252".into(),
            },
        )]);
        std::fs::write(
            source.join("export").join(crate::exports::ENCODING_SIDECAR),
            crate::exports::encoding_bytes(&source_map).unwrap(),
        )
        .unwrap();
        std::fs::write(
            target.join("export").join(crate::exports::ENCODING_SIDECAR),
            crate::exports::encoding_bytes(&target_map).unwrap(),
        )
        .unwrap();
        let bundle = capture(&source, window()).unwrap();
        assert_eq!(
            bundle.export_assets["Cyrillic.mte"].text_encoding,
            "windows-1251"
        );
        let backup = archive(root.path(), &bundle);
        let reviewed = preview(
            &target,
            &backup,
            window(),
            Options {
                files: Some(vec![]),
                preferences: false,
                workspace: false,
                export_assets: true,
            },
            vec![],
        )
        .unwrap();
        let state =
            serde_json::to_value(apply(&target, &reviewed.token, window()).unwrap()).unwrap();
        let id = state["pending"]["id"].as_str().unwrap();
        engine::status(&target, Some(id), None, None).unwrap();
        engine::status(&target, None, Some(id), None).unwrap();
        let library = crate::exports::load(&target.join("exports.json")).unwrap();
        assert_eq!(
            library
                .configurations
                .iter()
                .find(|c| c.file.as_deref() == Some("Cyrillic.mte"))
                .unwrap()
                .template,
            "Привет"
        );
        assert_eq!(
            library
                .configurations
                .iter()
                .find(|c| c.file.as_deref() == Some("Other.mte"))
                .unwrap()
                .template,
            "café"
        );
        crate::exports::save(&target.join("exports.json"), &library).unwrap();
        assert_eq!(
            std::fs::read(target.join("export/Cyrillic.mte")).unwrap(),
            raw
        );
        assert_eq!(
            std::fs::read(target.join("export/Other.mte")).unwrap(),
            other
        );
        let captured = capture(&target, window()).unwrap();
        assert_eq!(captured.export_assets["Cyrillic.mte"].sha256, digest(raw));
        assert_eq!(
            captured.export_assets["Cyrillic.mte"].text_encoding,
            "windows-1251"
        );
        assert!(crate::exports::encoding_overrides(&target.join("export"))
            .unwrap()
            .contains_key("Other.mte"));
        std::fs::write(
            target.join("export").join(crate::exports::ENCODING_SIDECAR),
            b"invalid prior metadata",
        )
        .unwrap();
        let before = engine::native(&target).unwrap();
        assert!(preview(&target, &backup, window(), options(), vec![]).is_err());
        assert_eq!(engine::native(&target).unwrap(), before);
    }

    #[test]
    fn portable_unknown_path_style_keeps_supported_settings_selectable_without_importing_rows() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let mut bundle = capture(&source, window()).unwrap();
        bundle.origin.path_style = "foreign-opaque".into();
        let incoming = bundle.window.as_mut().unwrap().workspace.as_mut().unwrap();
        incoming.paths = vec!["opaque row one".into(), "opaque row two".into()];
        incoming.roots = vec!["original opaque root".into()];
        incoming.selected = vec!["opaque row two".into()];
        incoming.source_order = Some(vec!["opaque row two".into(), "opaque row one".into()]);
        let audio = root.path().join("destination.wav");
        wave(&audio);
        let mut current = window();
        current.workspace.as_mut().unwrap().paths = vec![audio.to_string_lossy().into_owned()];
        let before = engine::native(&target).unwrap();
        let backup = archive(root.path(), &bundle);
        let unavailable = preview(&target, &backup, current.clone(), options(), vec![]).unwrap();
        assert!(!unavailable.workspace_available);
        assert!(unavailable.workspace.is_none());
        assert_eq!(
            serde_json::to_value(&unavailable).unwrap()["source_workspace"],
            serde_json::json!({"rows":2,"roots":["original opaque root"],"path_style":"foreign-opaque"})
        );
        assert!(apply(&target, &unavailable.token, current.clone()).is_err());
        assert_eq!(engine::native(&target).unwrap(), before);
        let reviewed = preview(
            &target,
            &backup,
            current.clone(),
            Options {
                workspace: false,
                ..options()
            },
            vec![],
        )
        .unwrap();
        let state = serde_json::to_value(apply(&target, &reviewed.token, current.clone()).unwrap())
            .unwrap();
        assert_eq!(
            state["pending"]["window"]["workspace"],
            serde_json::to_value(current.workspace).unwrap()
        );
        assert!(
            !crate::settings::load(&target.join("settings.json"))
                .0
                .general
                .natural_sort
        );
    }

    #[test]
    fn portable_unsupported_workspace_structural_errors_refuse_without_mutation() {
        let _guard = TESTS.lock().unwrap_or_else(|p| p.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let mut base = capture(&source, window()).unwrap();
        base.origin.path_style = "foreign-opaque".into();
        let before = engine::native(&target).unwrap();
        for case in [
            "duplicate",
            "selected",
            "rank",
            "rows",
            "roots",
            "size",
            "version",
            "nul",
        ] {
            let mut malformed = base.clone();
            let workspace = malformed
                .window
                .as_mut()
                .unwrap()
                .workspace
                .as_mut()
                .unwrap();
            match case {
                "duplicate" => workspace.paths = vec!["same".into(), "same".into()],
                "selected" => workspace.selected = vec!["not listed".into()],
                "rank" => {
                    workspace.paths = vec!["one".into()];
                    workspace.source_order = Some(vec![]);
                }
                "rows" => workspace.paths = (0..10001).map(|n| format!("row{n}")).collect(),
                "roots" => workspace.roots = (0..10001).map(|n| format!("root{n}")).collect(),
                "size" => workspace.location = "x".repeat(engine::WINDOW_LIMIT),
                "version" => workspace.version = Some(3),
                "nul" => workspace.roots = vec!["opaque\0root".into()],
                _ => unreachable!(),
            }
            let path = archive(root.path(), &malformed);
            let bytes = std::fs::read(&path).unwrap();
            assert!(
                preview(
                    &target,
                    &path,
                    window(),
                    Options {
                        workspace: false,
                        ..options()
                    },
                    vec![]
                )
                .is_err(),
                "{case}"
            );
            assert_eq!(engine::native(&target).unwrap(), before, "{case}");
            assert_eq!(std::fs::read(path).unwrap(), bytes, "{case}");
        }
    }

    #[test]
    fn portable_unknown_optional_assets_are_reported_but_unsafe_paths_still_refuse() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let mut bundle = capture(&source, window()).unwrap();
        let asset = Asset {
            encoding: "base64".into(),
            text_encoding: "utf-8".into(),
            bytes: base64::engine::general_purpose::STANDARD.encode(b"optional note"),
            sha256: digest(b"optional note"),
        };
        bundle
            .export_assets
            .insert("notes.txt".into(), asset.clone());
        let backup = archive(root.path(), &bundle);
        let reviewed = preview(&target, &backup, window(), options(), vec![]).unwrap();
        assert!(reviewed.export_assets.is_empty());
        assert!(reviewed
            .warnings
            .iter()
            .any(|warning| warning.contains("notes.txt")));
        apply(&target, &reviewed.token, window()).unwrap();
        assert!(!target.join("export/notes.txt").exists());
        bundle.export_assets.insert("../outside.mte".into(), asset);
        assert!(parse(&serial_bytes(&bundle).unwrap()).is_err());
    }

    #[test]
    fn portable_valid_backup_can_repair_malformed_current_preference_values() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let bundle = capture(&source, window()).unwrap();
        let backup = archive(root.path(), &bundle);
        let mut current = window();
        current
            .preferences
            .insert("opentag.tag-tag".into(), "damaged browser JSON".into());
        let before = current.clone();
        let reviewed = preview(&target, &backup, current.clone(), options(), vec![]).unwrap();
        apply(&target, &reviewed.token, current).unwrap();
        let restored = serde_json::to_value(engine::restore(&target, window()).unwrap()).unwrap();
        assert_eq!(
            restored["pending"]["window"],
            serde_json::to_value(before).unwrap()
        );
    }

    fn options() -> Options {
        Options::default()
    }
    fn archive(root: &Path, bundle: &Bundle) -> PathBuf {
        let path = root.join(format!("backup-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, serial_bytes(bundle).unwrap()).unwrap();
        path
    }
    fn configured(root: &Path, natural_sort: bool) {
        let mut settings = crate::settings::Settings::default();
        settings.general.natural_sort = natural_sort;
        crate::settings::save(&root.join("settings.json"), &settings).unwrap();
    }

    fn quick_window(literal_bytes: usize) -> WindowState {
        // action-ui.ts rememberQuick stores this complete ActionSpec without a
        // template-length restriction; the literal is never evaluated here.
        WindowState {
            version: 2,
            preferences: BTreeMap::from([(
                "opentag.actions.quick".into(),
                serde_json::json!({"last":"format_value","actions":{"format_value":{
                    "type":"format_value","field":"TITLE","template":"x".repeat(literal_bytes)
                }}})
                .to_string(),
            )]),
            workspace: None,
        }
    }

    #[test]
    fn portable_saved_quick_literal_above_256_kib_transfers_without_truncation() {
        let _guard = TESTS.lock().unwrap_or_else(|p| p.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let saved = quick_window(300 * 1024);
        assert!(serde_json::to_vec(&saved).unwrap().len() < 2 * 1024 * 1024);
        let output = root.path().join("saved.json");
        export_to(&source, &output, saved.clone(), &[]).unwrap();
        let review = preview(&target, &output, quick_window(0), options(), vec![]).unwrap();
        let pending =
            serde_json::to_value(apply(&target, &review.token, quick_window(0)).unwrap()).unwrap();
        assert_eq!(
            pending["pending"]["window"]["preferences"]["opentag.actions.quick"],
            saved.preferences["opentag.actions.quick"]
        );
    }

    #[test]
    fn portable_window_exact_2_mib_transfers_and_one_over_preserves_output_and_target() {
        let _guard = TESTS.lock().unwrap_or_else(|p| p.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let bound = 2 * 1024 * 1024;
        let overhead = serde_json::to_vec(&quick_window(0)).unwrap().len();
        let saved = quick_window(bound - overhead);
        assert_eq!(serde_json::to_vec(&saved).unwrap().len(), bound);
        let output = root.path().join("saved.json");
        export_to(&source, &output, saved.clone(), &[]).unwrap();
        let exact_output = std::fs::read(&output).unwrap();
        let before = engine::native(&target).unwrap();
        let over = quick_window(bound - overhead + 1);
        assert_eq!(serde_json::to_vec(&over).unwrap().len(), bound + 1);
        assert!(export_to(&source, &output, over.clone(), &[])
            .unwrap_err()
            .contains("2 MB"));
        assert_eq!(std::fs::read(&output).unwrap(), exact_output);
        let mut malformed = capture(&source, quick_window(0)).unwrap();
        malformed.window = Some(over);
        let incoming = root.path().join("oversized-window.json");
        std::fs::write(&incoming, serial_bytes(&malformed).unwrap()).unwrap();
        assert!(
            preview(&target, &incoming, quick_window(0), options(), vec![])
                .unwrap_err()
                .contains("2 MB")
        );
        assert_eq!(engine::native(&target).unwrap(), before);
        let review = preview(&target, &output, quick_window(0), options(), vec![]).unwrap();
        let pending =
            serde_json::to_value(apply(&target, &review.token, quick_window(0)).unwrap()).unwrap();
        assert_eq!(
            pending["pending"]["window"]["preferences"]["opentag.actions.quick"],
            saved.preferences["opentag.actions.quick"]
        );
    }

    #[test]
    fn portable_serialized_archive_exact_32_mib_and_one_over_are_not_truncated() {
        let _guard = TESTS.lock().unwrap_or_else(|p| p.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let mut bundle = capture(&source, quick_window(0)).unwrap();
        let mut raw = vec![b'x'; (LIMIT as usize / 4) * 3 - 4096];
        let asset = |raw: &[u8]| Asset {
            encoding: "base64".into(),
            text_encoding: "utf-8".into(),
            bytes: base64::engine::general_purpose::STANDARD.encode(raw),
            sha256: digest(raw),
        };
        bundle
            .export_assets
            .insert("Boundary.mte".into(), asset(&raw));
        let remaining = LIMIT as usize - serial_bytes(&bundle).unwrap().len();
        raw.resize(raw.len() + (remaining / 4) * 3, b'x');
        bundle
            .export_assets
            .insert("Boundary.mte".into(), asset(&raw));
        let remaining = LIMIT as usize - serial_bytes(&bundle).unwrap().len();
        assert!(remaining < 4);
        bundle.origin.platform.push_str(&"x".repeat(remaining));
        let exact = serial_bytes(&bundle).unwrap();
        assert_eq!(exact.len() as u64, LIMIT);
        assert_eq!(
            parse(&exact).unwrap().0.export_assets["Boundary.mte"].sha256,
            digest(&raw)
        );
        bundle.origin.platform.push('x');
        assert!(serial_bytes(&bundle).unwrap_err().contains("32 MB"));
        let over = serde_json::to_vec_pretty(&bundle).unwrap();
        assert_eq!(over.len() as u64, LIMIT + 1);
        assert!(parse(&over).unwrap_err().contains("32 MB"));
        let incoming = root.path().join("oversized.json");
        std::fs::write(&incoming, over).unwrap();
        let before = engine::native(&target).unwrap();
        assert!(preview(&target, &incoming, quick_window(0), options(), vec![]).is_err());
        assert_eq!(engine::native(&target).unwrap(), before);
        std::fs::create_dir(source.join("export")).unwrap();
        std::fs::write(
            source.join("export/Boundary.mte"),
            vec![b'x'; 24 * 1024 * 1024],
        )
        .unwrap();
        let output = root.path().join("existing.json");
        std::fs::write(&output, b"previous backup").unwrap();
        assert!(export_to(&source, &output, quick_window(0), &[])
            .unwrap_err()
            .contains("32 MB"));
        assert_eq!(std::fs::read(output).unwrap(), b"previous backup");
    }

    #[test]
    fn portable_combined_4096_entries_accept_and_one_over_preserves_output_and_target() {
        let _guard = TESTS.lock().unwrap_or_else(|p| p.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let exports = source.join("export");
        std::fs::create_dir(&exports).unwrap();
        for i in 0..4095 {
            std::fs::write(exports.join(format!("Boundary{i}.mte")), b"").unwrap();
        }
        let bundle = capture(&source, quick_window(0)).unwrap();
        assert_eq!(bundle.files.len() + bundle.export_assets.len(), 4096);
        assert_eq!(
            parse(&serial_bytes(&bundle).unwrap())
                .unwrap()
                .0
                .export_assets
                .len(),
            4095
        );
        let output = root.path().join("saved.json");
        export_to(&source, &output, quick_window(0), &[]).unwrap();
        let exact_output = std::fs::read(&output).unwrap();
        std::fs::write(exports.join("OneOver.mte"), b"").unwrap();
        assert!(export_to(&source, &output, quick_window(0), &[]).is_err());
        assert_eq!(std::fs::read(&output).unwrap(), exact_output);
        let mut over = bundle;
        over.export_assets.insert(
            "OneOver.mte".into(),
            Asset {
                encoding: "base64".into(),
                text_encoding: "utf-8".into(),
                bytes: String::new(),
                sha256: digest(b""),
            },
        );
        assert_eq!(over.files.len() + over.export_assets.len(), 4097);
        let incoming = archive(root.path(), &over);
        let before = engine::native(&target).unwrap();
        assert!(
            preview(&target, &incoming, quick_window(0), options(), vec![])
                .unwrap_err()
                .contains("4096")
        );
        assert_eq!(engine::native(&target).unwrap(), before);
    }

    fn wave(path: &Path) {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend(68u32.to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(32u32.to_le_bytes());
        bytes.extend([0; 32]);
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn portable_exact_cue_and_embedded_chapter_rows_ignore_destination_list_chapters_switch() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap();
        let source = directory.join("source");
        let target = directory.join("target");
        configured(&source, false);
        configured(&target, true);
        let audio = directory.join("Practice.wav");
        wave(&audio);
        let sheet="FILE \"Practice.wav\" WAVE\n  TRACK 01 AUDIO\n    TITLE \"Opening\"\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    TITLE \"Closing\"\n    INDEX 01 00:00:03\n";
        tag_core::write_fields(
            &audio,
            &tag_core::Fields::from([("CUESHEET".into(), vec![sheet.into()])]),
        )
        .unwrap();
        let cue = directory.join("Practice.cue");
        std::fs::write(&cue, sheet).unwrap();
        let saved_audio = std::fs::read(&audio).unwrap();
        let saved_cue = std::fs::read(&cue).unwrap();
        let rows = vec![
            tag_core::subsong_path(audio.to_str().unwrap(), 2),
            tag_core::subsong_path(cue.to_str().unwrap(), 1),
        ];
        let mut incoming = window();
        let workspace = incoming.workspace.as_mut().unwrap();
        workspace.paths = rows.clone();
        workspace.selected = rows.clone();
        workspace.roots = vec![
            cue.to_string_lossy().into_owned(),
            audio.to_string_lossy().into_owned(),
        ];
        workspace.source_order = Some(rows.iter().rev().cloned().collect());
        let backup = archive(&directory, &capture(&source, incoming).unwrap());
        assert!(
            !crate::settings::load(&target.join("settings.json"))
                .0
                .tags
                .advanced
                .list_chapters
        );
        let reviewed = preview(
            &target,
            &backup,
            window(),
            Options {
                files: Some(vec![]),
                export_assets: false,
                ..options()
            },
            vec![],
        )
        .unwrap();
        assert!(reviewed.workspace_available, "{:?}", reviewed.unavailable);
        let result =
            serde_json::to_value(apply(&target, &reviewed.token, window()).unwrap()).unwrap();
        assert_eq!(
            result["pending"]["window"]["workspace"]["paths"],
            serde_json::json!(rows)
        );
        assert_eq!(std::fs::read(audio).unwrap(), saved_audio);
        assert_eq!(std::fs::read(cue).unwrap(), saved_cue);
    }

    #[test]
    fn portable_physical_chapter_parent_survives_destination_list_chapters_enabled() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().canonicalize().unwrap();
        let source = directory.join("source");
        let target = directory.join("target");
        configured(&source, false);
        configured(&target, true);
        let audio = directory.join("Practice.wav");
        wave(&audio);
        let sheet="FILE \"Practice.wav\" WAVE\n TRACK 01 AUDIO\n TITLE \"First\"\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n TITLE \"Second\"\n INDEX 01 00:00:03\n";
        tag_core::write_fields(
            &audio,
            &tag_core::Fields::from([("CUESHEET".into(), vec![sheet.into()])]),
        )
        .unwrap();
        let before = std::fs::read(&audio).unwrap();
        let mut target_settings = crate::settings::load(&target.join("settings.json")).0;
        target_settings.tags.advanced.list_chapters = true;
        crate::settings::save(&target.join("settings.json"), &target_settings).unwrap();
        let mut incoming = window();
        incoming.workspace.as_mut().unwrap().paths = vec![audio.to_string_lossy().into_owned()];
        let backup = archive(&directory, &capture(&source, incoming).unwrap());
        let reviewed = preview(
            &target,
            &backup,
            window(),
            Options {
                files: Some(vec![]),
                export_assets: false,
                ..options()
            },
            vec![],
        )
        .unwrap();
        assert!(reviewed.workspace_available, "{:?}", reviewed.unavailable);
        let state =
            serde_json::to_value(apply(&target, &reviewed.token, window()).unwrap()).unwrap();
        assert_eq!(
            state["pending"]["window"]["workspace"]["paths"],
            serde_json::json!([audio])
        );
        assert_eq!(std::fs::read(audio).unwrap(), before);
    }

    #[test]
    fn portable_local_mapping_preserves_rows_rank_working_directory_and_audio_bytes() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let old = root.path().join("old-music");
        let new = root.path().join("new-music");
        std::fs::create_dir(&old).unwrap();
        wave(&old.join("Practice.wav"));
        let mut incoming = window();
        let row = old.join("Practice.wav").to_string_lossy().into_owned();
        let workspace = incoming.workspace.as_mut().unwrap();
        workspace.paths = vec![row.clone()];
        workspace.selected = vec![row.clone()];
        workspace.anchor = Some(row.clone());
        workspace.roots = vec![old.to_string_lossy().into_owned()];
        workspace.working_directory = Some(old.to_string_lossy().into_owned());
        workspace.source_order = Some(vec![row]);
        workspace.list_order = Some("playlist".into());
        let backup = archive(root.path(), &capture(&source, incoming).unwrap());
        std::fs::rename(&old, &new).unwrap();
        let audio = std::fs::read(new.join("Practice.wav")).unwrap();
        let missing = preview(&target, &backup, window(), options(), vec![]).unwrap();
        assert!(!missing.workspace_available);
        assert!(!missing.unavailable.is_empty());
        assert!(apply(&target, &missing.token, window()).is_err());
        let mapped = preview(
            &target,
            &backup,
            window(),
            options(),
            vec![Mapping {
                from: old.to_string_lossy().into_owned(),
                to: new.to_string_lossy().into_owned(),
            }],
        )
        .unwrap();
        assert!(mapped.workspace_available, "{:?}", mapped.unavailable);
        let status =
            serde_json::to_value(apply(&target, &mapped.token, window()).unwrap()).unwrap();
        let restored = &status["pending"]["window"]["workspace"];
        assert_eq!(
            restored["paths"],
            serde_json::json!([new.canonicalize().unwrap().join("Practice.wav")])
        );
        assert_eq!(restored["source_order"], restored["paths"]);
        assert_eq!(
            restored["working_directory"],
            serde_json::json!(new.canonicalize().unwrap())
        );
        assert_eq!(restored["list_order"], "playlist");
        assert_eq!(std::fs::read(new.join("Practice.wav")).unwrap(), audio);
    }

    #[cfg(unix)]
    #[test]
    fn portable_export_refuses_private_roots_and_audio_symlink_aliases_without_writes() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("config");
        configured(&source, false);
        let private = source.join("tag-sources.json");
        std::fs::write(&private, b"excluded private container").unwrap();
        let alias = root.path().join("backup.json");
        std::os::unix::fs::symlink(&private, &alias).unwrap();
        assert!(export_to(&source, &alias, window(), &[]).is_err());
        assert_eq!(
            std::fs::read(&private).unwrap(),
            b"excluded private container"
        );
        let undo = root.path().join("private-cache/undo");
        std::fs::create_dir_all(&undo).unwrap();
        let undo_file = undo.join("snapshot");
        std::fs::write(&undo_file, b"existing recovery data").unwrap();
        assert!(export_to(
            &source,
            &undo_file,
            window(),
            &[root.path().join("private-cache")]
        )
        .is_err());
        assert_eq!(
            std::fs::read(&undo_file).unwrap(),
            b"existing recovery data"
        );
        let audio = root.path().join("Practice.wav");
        wave(&audio);
        let original = std::fs::read(&audio).unwrap();
        let audio_alias = root.path().join("audio-alias.json");
        std::os::unix::fs::symlink(&audio, &audio_alias).unwrap();
        let mut current = window();
        current.workspace.as_mut().unwrap().paths = vec![audio.to_string_lossy().into_owned()];
        assert!(export_to(&source, &audio_alias, current, &[]).is_err());
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        let hard_alias = root.path().join("audio-hard-alias.json");
        std::fs::hard_link(&audio, &hard_alias).unwrap();
        let mut current = window();
        current.workspace.as_mut().unwrap().paths = vec![audio.to_string_lossy().into_owned()];
        assert!(export_to(&source, &hard_alias, current, &[]).is_err());
        assert_eq!(std::fs::read(&hard_alias).unwrap(), original);
        let external = root.path().join("external.json");
        assert_eq!(
            export_to(&source, &external, window(), &[])
                .unwrap()
                .version,
            2
        );
        assert_eq!(
            parse(&std::fs::read(external).unwrap()).unwrap().0.version,
            2
        );
    }

    #[cfg(unix)]
    #[test]
    fn portable_cue_only_workspace_protects_container_referenced_audio_and_all_aliases() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for case in [
            "audio",
            "audio-symlink",
            "audio-hardlink",
            "cue",
            "cue-symlink",
            "cue-hardlink",
        ] {
            let root = tempfile::tempdir().unwrap();
            let config = root.path().join("config");
            configured(&config, false);
            let audio = root.path().join("Practice.wav");
            wave(&audio);
            let cue = root.path().join("Practice.cue");
            std::fs::write(
                &cue,
                "FILE \"Practice.wav\" WAVE\n TRACK 01 AUDIO\n TITLE \"One\"\n INDEX 01 00:00:00\n",
            )
            .unwrap();
            let audio_before = std::fs::read(&audio).unwrap();
            let cue_before = std::fs::read(&cue).unwrap();
            let mut current = window();
            current.workspace.as_mut().unwrap().paths =
                vec![tag_core::subsong_path(cue.to_str().unwrap(), 1)];
            let protected = if case.starts_with("audio") {
                &audio
            } else {
                &cue
            };
            let output = if case.ends_with("symlink") {
                let alias = root.path().join("alias.json");
                std::os::unix::fs::symlink(protected, &alias).unwrap();
                alias
            } else if case.ends_with("hardlink") {
                let alias = root.path().join("alias.json");
                std::fs::hard_link(protected, &alias).unwrap();
                alias
            } else {
                protected.clone()
            };
            assert!(export_to(&config, &output, current, &[]).is_err(), "{case}");
            assert_eq!(std::fs::read(&audio).unwrap(), audio_before, "{case}");
            assert_eq!(std::fs::read(&cue).unwrap(), cue_before, "{case}");
            assert_eq!(
                std::fs::read(&output).unwrap(),
                if case.starts_with("audio") {
                    audio_before
                } else {
                    cue_before
                },
                "{case}"
            );
        }
    }

    #[test]
    fn portable_unreadable_cue_refuses_before_replacing_an_existing_backup() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        configured(&config, false);
        let cue = root.path().join("Broken.cue");
        std::fs::write(&cue, "not a readable track").unwrap();
        let output = root.path().join("backup.json");
        std::fs::write(&output, b"previous backup").unwrap();
        let mut current = window();
        current.workspace.as_mut().unwrap().paths =
            vec![tag_core::subsong_path(cue.to_str().unwrap(), 1)];
        assert!(export_to(&config, &output, current, &[]).is_err());
        assert_eq!(std::fs::read(output).unwrap(), b"previous backup");
        assert_eq!(std::fs::read(cue).unwrap(), b"not a readable track");
    }

    #[test]
    fn portable_journal_and_acknowledgement_survive_a_fresh_backend_process() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let before = engine::native(&target).unwrap();
        let backup = archive(root.path(), &capture(&source, window()).unwrap());
        let reviewed = preview(&target, &backup, window(), options(), vec![]).unwrap();
        apply(&target, &reviewed.token, window()).unwrap();
        for phase in ["ack", "restore"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "configuration_transfer::tests::portable_process_helper",
                    "--ignored",
                    "--nocapture",
                ])
                .env("TAGGROOVE_PORTABLE_TEST_DIRECTORY", &target)
                .env("TAGGROOVE_PORTABLE_TEST_PHASE", phase)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            if phase == "ack" {
                assert!(engine::inspect_status(&target).unwrap().pending.is_none());
            }
        }
        assert_eq!(engine::native(&target).unwrap(), before);
    }

    #[test]
    #[ignore = "fresh-process control invoked by portable journal test"]
    fn portable_process_helper() {
        let directory = PathBuf::from(std::env::var("TAGGROOVE_PORTABLE_TEST_DIRECTORY").unwrap());
        match std::env::var("TAGGROOVE_PORTABLE_TEST_PHASE")
            .unwrap()
            .as_str()
        {
            "ack" => {
                let state =
                    serde_json::to_value(engine::status(&directory, None, None, None).unwrap())
                        .unwrap();
                assert_eq!(state["report"]["operation"], "configuration");
                let id = state["pending"]["id"].as_str().unwrap();
                engine::status(&directory, Some(id), None, None).unwrap();
                engine::status(&directory, None, Some(id), None).unwrap();
            }
            "restore" => {
                engine::restore(&directory, window()).unwrap();
            }
            _ => panic!("unknown portable helper phase"),
        }
    }

    #[test]
    fn portable_all_preference_groups_accept_actual_component_stored_values() {
        let mut values = BTreeMap::from([
            ("opentag.language".into(), "system".into()),
            ("opentag.appearance".into(), "dark".into()),
            ("opentag.main-layout".into(), r#"{"dock":"left","width":264,"height":260,"float":{"x":80,"y":120,"width":300,"height":520},"filter":"list","last_dock":"left"}"#.into()),
            ("opentag.panel-width".into(), "264".into()),
            ("opentag.filter-auto-apply".into(), "true".into()),
            ("opentag.filter-shown".into(), "false".into()),
            ("opentag.numbering".into(), r#"{"track_start":1,"track_digits":2,"track_total":false,"disc_enabled":false,"disc_start":1,"disc_digits":1,"disc_total":false,"disc_total_value":1,"next_disc_on_directory":false,"next_disc_on_field":"","reset_track_on_next_disc":false,"reset_disc_on_field":""}"#.into()),
            ("opentag.actions.quick".into(), r#"{"last":"format_value","actions":{"format_value":{"type":"format_value","field":"TITLE","template":"%title%"}},"groupCodepage":"1251"}"#.into()),
            ("opentag.tag-tag".into(), r#"{"field":"TITLE","template":"%artist%"}"#.into()),
            ("opentag.cover.aspect".into(), "correct".into()),
            ("opentag.cover.extractFormat".into(), "%_folderpath%%album%".into()),
            ("opentag.tagSources.lastSource".into(), "web:Practice.src".into()),
            ("opentag.tagSources.lastBuiltIn".into(), "source-discogs".into()),
            ("opentag.tagSources.last".into(), "discogs".into()),
            ("opentag.tagSources.saveCover".into(), "1".into()),
            ("opentag.tagSources.suggestSorting".into(), "".into()),
            ("opentag.last-version".into(), "0.2.39".into()),
        ]);
        for key in engine::PREFERENCE_KEYS
            .iter()
            .filter(|key| key.starts_with("opentag.tagSources.disabled."))
        {
            values.insert((*key).into(), r#"["COMMENT","ALBUMARTIST"]"#.into());
        }
        assert_eq!(
            values.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            engine::PREFERENCE_KEYS.iter().copied().collect()
        );
        validate_preferences(&values).unwrap();
        for (key, value) in [
            ("opentag.cover.aspect", "stretch"),
            ("opentag.tagSources.saveCover", "0"),
            ("opentag.tagSources.suggestSorting", "title"),
            ("opentag.tagSources.suggestSorting", "length"),
        ] {
            let mut changed = values.clone();
            changed.insert(key.into(), value.into());
            validate_preferences(&changed).unwrap();
        }
        for (key, value) in [
            ("opentag.tagSources.saveCover", "{}"),
            ("opentag.filter-shown", "[]"),
            ("opentag.numbering", r#"{"track_digits":0}"#),
            (
                "opentag.actions.quick",
                r#"{"actions":[],"groupCodepage":"unknown"}"#,
            ),
            ("opentag.tag-tag", r#"{"field":3}"#),
        ] {
            let mut invalid = values.clone();
            invalid.insert(key.into(), value.into());
            assert!(validate_preferences(&invalid).is_err(), "{key}");
        }
        values.insert("opentag.unknown".into(), "not captured".into());
        assert!(validate_preferences(&values).is_err());
    }

    #[test]
    fn portable_settings_only_preserves_destination_workspace_and_shared_rollback() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        let mut source_settings = crate::settings::load(&source.join("settings.json")).0;
        source_settings.tags.mpeg.read_ape = true;
        crate::settings::save(&source.join("settings.json"), &source_settings).unwrap();
        assert!(
            !crate::settings::load(&target.join("settings.json"))
                .0
                .tags
                .mpeg
                .read_ape
        );
        std::fs::write(
            target.join("tag-sources.json"),
            b"excluded credential container",
        )
        .unwrap();
        let source_window = window();
        let mut current = window();
        current.workspace.as_mut().unwrap().filter = "current filter draft".into();
        let before = engine::native(&target).unwrap();
        let backup = archive(root.path(), &capture(&source, source_window).unwrap());
        let preview = preview(
            &target,
            &backup,
            current.clone(),
            Options {
                workspace: false,
                ..options()
            },
            vec![],
        )
        .unwrap();
        let status = apply(&target, &preview.token, current.clone()).unwrap();
        let status_json = serde_json::to_value(&status).unwrap();
        assert_eq!(
            status_json["pending"]["window"]["workspace"],
            serde_json::to_value(&current.workspace).unwrap()
        );
        assert_eq!(
            status_json["pending"]["window"]["preferences"]["opentag.last-version"],
            "destination-bookkeeping"
        );
        assert!(
            !crate::settings::load(&target.join("settings.json"))
                .0
                .general
                .natural_sort
        );
        assert_eq!(
            std::fs::read(target.join("tag-sources.json")).unwrap(),
            b"excluded credential container"
        );
        assert!(
            crate::settings::load(&target.join("settings.json"))
                .0
                .tags
                .mpeg
                .read_ape
        );
        let restored = engine::restore(&target, current.clone()).unwrap();
        assert_eq!(engine::native(&target).unwrap(), before);
        assert!(
            !crate::settings::load(&target.join("settings.json"))
                .0
                .tags
                .mpeg
                .read_ape
        );
        assert_eq!(
            serde_json::to_value(restored).unwrap()["pending"]["window"],
            serde_json::to_value(current).unwrap()
        );
    }

    #[test]
    fn portable_empty_complete_preferences_reset_but_absent_window_preserves() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for absent in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("source");
            let target = root.path().join("target");
            configured(&source, false);
            configured(&target, true);
            let mut incoming = window();
            incoming.preferences.clear();
            let mut bundle = capture(&source, incoming).unwrap();
            if absent {
                bundle.window = None;
            }
            let backup = archive(root.path(), &bundle);
            let current = window();
            let preview = preview(
                &target,
                &backup,
                current.clone(),
                Options {
                    workspace: false,
                    ..options()
                },
                vec![],
            )
            .unwrap();
            assert_eq!(preview.preferences_available, !absent);
            let status =
                serde_json::to_value(apply(&target, &preview.token, current.clone()).unwrap())
                    .unwrap();
            let preferences = &status["pending"]["window"]["preferences"];
            if absent {
                assert_eq!(
                    *preferences,
                    serde_json::to_value(current.preferences).unwrap()
                );
            } else {
                assert_eq!(
                    *preferences,
                    serde_json::json!({"opentag.last-version":"destination-bookkeeping"})
                );
            }
        }
    }

    #[test]
    fn portable_v1_adapter_uses_same_journal_and_preserves_physical_templates() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        std::fs::create_dir(target.join("export")).unwrap();
        std::fs::write(target.join("export/Target.mte"), b"unfinished $loop(").unwrap();
        let before = engine::native(&target).unwrap();
        let backup = root.path().join("legacy.json");
        std::fs::write(
            &backup,
            serde_json::to_vec(&crate::utilities::collect(&source).unwrap()).unwrap(),
        )
        .unwrap();
        let current = window();
        let preview = preview(&target, &backup, current.clone(), options(), vec![]).unwrap();
        assert_eq!(preview.version, 1);
        assert!(!preview.preferences_available);
        let status = apply(&target, &preview.token, current.clone()).unwrap();
        assert!(status.pending.is_some());
        assert_eq!(
            std::fs::read(target.join("export/Target.mte")).unwrap(),
            b"unfinished $loop("
        );
        engine::restore(&target, current).unwrap();
        assert_eq!(engine::native(&target).unwrap(), before);
    }

    #[test]
    fn portable_preview_and_apply_reject_tampered_assets_and_source_or_target_drift() {
        let _guard = TESTS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        configured(&source, false);
        configured(&target, true);
        std::fs::create_dir(source.join("export")).unwrap();
        std::fs::write(source.join("export/Practice.mte"), b"raw $loop(").unwrap();
        let bundle = capture(&source, window()).unwrap();
        let mut bad = bundle.clone();
        bad.export_assets.get_mut("Practice.mte").unwrap().sha256 = "not its hash".into();
        let backup = archive(root.path(), &bad);
        let before = engine::native(&target).unwrap();
        assert!(preview(&target, &backup, window(), options(), vec![]).is_err());
        assert_eq!(engine::native(&target).unwrap(), before);
        assert!(!target.join(".profile-migration").exists());
        let backup = archive(root.path(), &bundle);
        let reviewed = preview(&target, &backup, window(), options(), vec![]).unwrap();
        std::fs::write(&backup, b"changed archive").unwrap();
        assert!(apply(&target, &reviewed.token, window())
            .unwrap_err()
            .contains("backup changed"));
        assert_eq!(engine::native(&target).unwrap(), before);
        let backup = archive(root.path(), &bundle);
        let reviewed = preview(&target, &backup, window(), options(), vec![]).unwrap();
        std::fs::write(target.join("lists.json"), b"external change").unwrap();
        let changed = engine::native(&target).unwrap();
        assert!(apply(&target, &reviewed.token, window())
            .unwrap_err()
            .contains("changed after preview"));
        assert_eq!(engine::native(&target).unwrap(), changed);
    }
}
