//! Helper lists behind the ► buttons (https://docs.mp3tag.de/manage-list/):
//! the field-name list and the remembered format strings, filter expressions
//! and Find criteria of each input, with their names, descriptions and
//! favorites. They are part of the configuration (`lists.json`), so a
//! configuration backup carries them; earlier versions kept them in the
//! window's local storage, which the frontend migrates once.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const FIELD_LIST: &str = "opentag.field-list";
pub const HISTORY_PREFIX: &str = "opentag.history.";
const MAX_LISTS: usize = 256;
const MAX_NAMES: usize = 2000;
const MAX_ENTRIES: usize = 200;
const MAX_TEXT: usize = 16 * 1024;

/// One remembered value of a history list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Ticked in Manage List: offered in the ► menu.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub favorite: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lists {
    pub version: u32,
    pub lists: BTreeMap<String, Value>,
}

#[derive(Serialize)]
pub struct Loaded {
    /// False before the first save: the frontend then migrates its local storage.
    pub exists: bool,
    pub lists: BTreeMap<String, Value>,
}

fn history_key(key: &str) -> bool {
    key.strip_prefix(HISTORY_PREFIX).is_some_and(|id| {
        !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    })
}

/// Check every list and return it in its stored form.
pub fn validate(lists: &BTreeMap<String, Value>) -> Result<BTreeMap<String, Value>, String> {
    if lists.len() > MAX_LISTS {
        return Err("Too many helper lists".into());
    }
    let mut checked = BTreeMap::new();
    for (key, value) in lists {
        let stored = if key == FIELD_LIST {
            let names: Vec<String> =
                serde_json::from_value(value.clone()).map_err(|e| format!("{key}: {e}"))?;
            if names.len() > MAX_NAMES || names.iter().any(|name| name.len() > 256) {
                return Err(format!("{key}: the field list is too long"));
            }
            serde_json::to_value(names)
        } else if history_key(key) {
            let entries: Vec<Entry> =
                serde_json::from_value(value.clone()).map_err(|e| format!("{key}: {e}"))?;
            if entries.len() > MAX_ENTRIES {
                return Err(format!("{key}: too many entries"));
            }
            for entry in &entries {
                let texts = [
                    Some(&entry.value),
                    entry.name.as_ref(),
                    entry.description.as_ref(),
                ];
                if texts.iter().flatten().any(|text| text.len() > MAX_TEXT) {
                    return Err(format!("{key}: an entry is too long"));
                }
            }
            serde_json::to_value(entries)
        } else {
            return Err(format!("{key} is not a helper list"));
        };
        checked.insert(key.clone(), stored.map_err(|e| e.to_string())?);
    }
    Ok(checked)
}

pub fn load(path: &Path) -> Result<Loaded, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded {
                exists: false,
                lists: BTreeMap::new(),
            })
        }
        Err(e) => return Err(e.to_string()),
    };
    let saved: Lists =
        serde_json::from_slice(&bytes).map_err(|e| format!("Cannot read helper lists: {e}"))?;
    if saved.version != 1 {
        return Err("Unsupported helper list version".into());
    }
    Ok(Loaded {
        exists: true,
        lists: validate(&saved.lists)?,
    })
}

pub fn save(path: &Path, lists: &BTreeMap<String, Value>) -> Result<(), String> {
    let lists = validate(lists)?;
    let bytes =
        serde_json::to_vec_pretty(&Lists { version: 1, lists }).map_err(|e| e.to_string())?;
    crate::action_host::atomic_output(path, &bytes, false)
}

fn lists_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?.with_file_name("lists.json"))
}

// Saving a list must not wait for a running file operation, so these do not
// take the file operation lock.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_helper_lists(app: tauri::AppHandle) -> Result<Loaded, String> {
    let path = lists_path(&app)?;
    blocking(move || load(&path)).await
}

#[tauri::command]
pub async fn save_helper_lists(
    app: tauri::AppHandle,
    lists: BTreeMap<String, Value>,
) -> Result<(), String> {
    let path = lists_path(&app)?;
    blocking(move || {
        let _guard = crate::profile_migration::mutation_guard(
            path.parent()
                .ok_or("Configuration directory is unavailable")?,
        )?;
        save(&path, &lists)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lists_round_trip_with_names_descriptions_and_favorites() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lists.json");
        assert!(!load(&path).unwrap().exists);
        let lists: BTreeMap<String, Value> = [
            (FIELD_LIST.to_string(), json!(["ALBUM", "MOOD"])),
            (
                "opentag.history.filter".to_string(),
                json!([{"value":"%genre% IS rock","name":"Rock","description":"Only rock","favorite":true},{"value":"x"}]),
            ),
            ("opentag.history.find".to_string(), json!([{"value":"Björk"}])),
        ]
        .into();
        save(&path, &lists).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.exists);
        assert_eq!(loaded.lists, lists);
    }

    #[test]
    fn unknown_or_malformed_lists_are_refused() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lists.json");
        for (key, value) in [
            ("opentag.panel-width", json!(300)),
            ("opentag.history.", json!([])),
            ("opentag.history.bad/key", json!([])),
            (FIELD_LIST, json!([1, 2])),
            ("opentag.history.filter", json!(["plain text"])),
            ("opentag.history.filter", json!([{"name":"no value"}])),
        ] {
            let lists: BTreeMap<String, Value> = [(key.to_string(), value)].into();
            assert!(save(&path, &lists).is_err(), "{key}");
        }
        assert!(!path.exists());
        let long: BTreeMap<String, Value> = [(
            "opentag.history.filter".to_string(),
            json!((0..=MAX_ENTRIES)
                .map(|i| json!({"value": i.to_string()}))
                .collect::<Vec<_>>()),
        )]
        .into();
        assert!(save(&path, &long).is_err());
        fs::write(&path, r#"{"version":2,"lists":{}}"#).unwrap();
        assert!(load(&path).is_err());
    }
}
