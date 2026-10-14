//! Persistent user options (File → Options…), stored as versioned JSON in the
//! app config directory. Pages registered by other frontend features keep their
//! values in `pages` without needing backend fields.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};
use tag_core::{AudioEntry, Fields, TagOptions};
use tauri::Manager;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// Sort 1, 2, 11 instead of 1, 11, 2.
    pub natural_sort: bool,
    pub show_directory_in_title: bool,
}
impl Default for General {
    fn default() -> Self {
        Self {
            natural_sort: true,
            show_directory_in_title: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Files {
    /// Extensions read from directories, for example `*.mp3;*.flac`. Empty
    /// reads every supported format.
    pub restrict_extensions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Directories {
    pub subfolders: bool,
}
impl Default for Directories {
    fn default() -> Self {
        Self { subfolders: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub general: General,
    pub files: Files,
    pub directories: Directories,
    pub tags: TagOptions,
    /// Values of option pages registered by other features, keyed by page id.
    pub pages: BTreeMap<String, serde_json::Value>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            general: General::default(),
            files: Files::default(),
            directories: Directories::default(),
            tags: TagOptions::default(),
            pages: BTreeMap::new(),
        }
    }
}

/// Parse `*.mp3; flac, .ogg` into lowercase extensions. `None` means all.
pub fn parse_extensions(value: &str) -> Result<Option<Vec<String>>, String> {
    let mut output = Vec::new();
    for item in value
        .split([';', ',', ' ', '\t', '\n'])
        .filter(|s| !s.is_empty())
    {
        let extension = item
            .strip_prefix("*.")
            .or_else(|| item.strip_prefix('.'))
            .unwrap_or(item)
            .to_ascii_lowercase();
        if extension.is_empty()
            || !extension
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+')
        {
            return Err(format!("Invalid file extension: {item}"));
        }
        if !output.contains(&extension) {
            output.push(extension);
        }
    }
    Ok((!output.is_empty()).then_some(output))
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err(format!("Unsupported settings version {}", self.version));
        }
        self.tags.validate()?;
        parse_extensions(&self.files.restrict_extensions)?;
        for (id, value) in &self.pages {
            if id.is_empty()
                || !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            {
                return Err(format!("Invalid options page identifier: {id:?}"));
            }
            if !value.is_object() {
                return Err(format!("Options page {id} must store an object"));
            }
        }
        Ok(())
    }
}

/// Read settings. Missing files yield defaults; unreadable or newer files keep
/// the defaults in use and report why, so saving replaces them deliberately.
pub fn load(path: &Path) -> (Settings, Option<String>) {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => {
            return (
                Settings::default(),
                Some(format!("Options could not be read: {e}")),
            )
        }
    };
    let value: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(e) => {
            return (
                Settings::default(),
                Some(format!("Options could not be read: {e}")),
            )
        }
    };
    let version = value.get("version").and_then(serde_json::Value::as_u64);
    if version.is_some_and(|v| v > u64::from(VERSION)) {
        return (
            Settings::default(),
            Some("Options were saved by a newer OpenTag version; defaults are in use".into()),
        );
    }
    // Version 1 is the first schema; older files without a version migrate as-is.
    let mut settings = match serde_json::from_value::<Settings>(value) {
        Ok(settings) => settings,
        Err(e) => {
            return (
                Settings::default(),
                Some(format!("Options could not be read: {e}")),
            )
        }
    };
    settings.version = VERSION;
    match settings.validate() {
        Ok(()) => (settings, None),
        Err(e) => (
            Settings::default(),
            Some(format!(
                "Saved options are invalid ({e}); defaults are in use"
            )),
        ),
    }
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    settings.validate()?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    crate::action_host::atomic_output(path, &bytes, false)
}

pub struct SettingsStore {
    path: Option<PathBuf>,
    state: RwLock<(Settings, Option<String>)>,
}
impl SettingsStore {
    pub fn open(app: &tauri::AppHandle) -> Self {
        let path = app
            .path()
            .app_config_dir()
            .ok()
            .map(|dir| dir.join("settings.json"));
        let state = path.as_deref().map(load).unwrap_or_default();
        Self {
            path,
            state: RwLock::new(state),
        }
    }
}

/// The options in effect. Without a running app (unit tests) these are the defaults.
pub fn current() -> Settings {
    crate::APP_HANDLE
        .get()
        .and_then(|app| app.try_state::<SettingsStore>())
        .and_then(|store| store.state.read().ok().map(|state| state.0.clone()))
        .unwrap_or_default()
}
pub fn tag_options() -> TagOptions {
    current().tags
}
pub fn extensions() -> Option<Vec<String>> {
    parse_extensions(&current().files.restrict_extensions)
        .ok()
        .flatten()
}

pub fn read_summary(path: &Path) -> Result<AudioEntry, String> {
    tag_core::read_entry_summary_with_options(path, &[], &tag_options())
}
pub fn read_full(path: &Path) -> Result<AudioEntry, String> {
    tag_core::read_entry_with_options(path, &[], &tag_options())
}
pub fn write_summary(path: &Path, updates: &Fields) -> Result<AudioEntry, String> {
    tag_core::write_fields_with_options(path, updates, false, &[], &tag_options())
}

#[derive(Serialize)]
pub struct SettingsResponse {
    settings: Settings,
    defaults: Settings,
    warning: Option<String>,
}

#[tauri::command]
pub async fn load_settings(
    store: tauri::State<'_, SettingsStore>,
) -> Result<SettingsResponse, String> {
    let (settings, warning) = store
        .state
        .read()
        .map_err(|_| "Options are unavailable")?
        .clone();
    Ok(SettingsResponse {
        settings,
        defaults: Settings::default(),
        warning,
    })
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    mut settings: Settings,
) -> Result<SettingsResponse, String> {
    settings.version = VERSION;
    settings.validate()?;
    // File operations hold this lock, so a batch never sees options change.
    crate::io(move || {
        let store = app.state::<SettingsStore>();
        let path = store
            .path
            .as_deref()
            .ok_or("The options location is unavailable")?;
        save(path, &settings)?;
        *store.state.write().map_err(|_| "Options are unavailable")? = (settings.clone(), None);
        Ok(SettingsResponse {
            settings,
            defaults: Settings::default(),
            warning: None,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_and_partial_files_are_completed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/settings.json");
        assert_eq!(load(&path), (Settings::default(), None));
        let mut settings = Settings::default();
        settings.tags.mpeg.write_ape = true;
        settings.files.restrict_extensions = "*.mp3; FLAC".into();
        settings
            .pages
            .insert("tag-panel".into(), serde_json::json!({"fields": ["TITLE"]}));
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), (settings, None));
        fs::write(&path, r#"{"general":{"natural_sort":false}}"#).unwrap();
        let (partial, warning) = load(&path);
        assert!(warning.is_none());
        assert!(!partial.general.natural_sort);
        assert_eq!(partial.version, VERSION);
        assert_eq!(partial.tags, TagOptions::default());
    }

    #[test]
    fn invalid_newer_and_corrupt_files_report_and_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        for contents in [
            "{not json",
            r#"{"version":99}"#,
            r#"{"tags":{"advanced":{"id3v2_comment_language":"english"}}}"#,
            r#"{"files":{"restrict_extensions":"*.mp3;../x"}}"#,
        ] {
            fs::write(&path, contents).unwrap();
            let (settings, warning) = load(&path);
            assert_eq!(settings, Settings::default(), "{contents}");
            assert!(warning.is_some(), "{contents}");
        }
        let mut invalid = Settings::default();
        invalid.pages.insert("bad id".into(), serde_json::json!({}));
        assert!(save(&path, &invalid).is_err());
        invalid.pages.clear();
        invalid.pages.insert("ok".into(), serde_json::json!(3));
        assert!(save(&path, &invalid).is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            r#"{"files":{"restrict_extensions":"*.mp3;../x"}}"#
        );
    }

    #[test]
    fn extension_lists_accept_mp3tag_patterns() {
        assert_eq!(parse_extensions("").unwrap(), None);
        assert_eq!(
            parse_extensions("*.MP3;*.flac, .ogg mp+ *.mp3").unwrap(),
            Some(vec![
                "mp3".into(),
                "flac".into(),
                "ogg".into(),
                "mp+".into()
            ])
        );
        assert!(parse_extensions("*.mp3;*").is_err());
    }
}
