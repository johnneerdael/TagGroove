//! Options → Language: catalog files people add to the `languages` folder of
//! the configuration folder (`<locale>.json`, keyed by the English UI text).
//! The frontend validates and applies them; this module only lists and reads
//! the files and keeps `_template.json` current for translators.
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Catalogs larger than this are not language files.
const CATALOG_LIMIT: u64 = 4 * 1024 * 1024;
pub const TEMPLATE: &str = "_template.json";

#[derive(Debug, Serialize, PartialEq)]
pub struct UserCatalog {
    pub file: String,
    pub catalog: serde_json::Value,
}

#[derive(Debug, Serialize, Default, PartialEq)]
pub struct UserCatalogs {
    pub catalogs: Vec<UserCatalog>,
    /// Files that could not be read, with the reason.
    pub errors: Vec<String>,
}

fn languages_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::utilities::configuration_directory(app)?.join("languages"))
}

/// Read every `*.json` of `directory` except files starting with `_` (the template).
pub fn read_catalogs(directory: &Path) -> UserCatalogs {
    let mut result = UserCatalogs::default();
    let Ok(entries) = fs::read_dir(directory) else {
        return result;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('_'))
        })
        .collect();
    paths.sort();
    for path in paths {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let read = || -> Result<serde_json::Value, String> {
            let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
            if size > CATALOG_LIMIT {
                return Err("the file is too large".into());
            }
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
            let value: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            if value.is_object() {
                Ok(value)
            } else {
                Err("not a JSON object".into())
            }
        };
        match read() {
            Ok(catalog) => result.catalogs.push(UserCatalog { file, catalog }),
            Err(error) => result.errors.push(format!("{file}: {error}")),
        }
    }
    result
}

/// Catalog files of the languages folder; a missing folder has none.
#[tauri::command]
pub async fn language_catalogs(app: tauri::AppHandle) -> Result<UserCatalogs, String> {
    let directory = languages_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || read_catalogs(&directory))
        .await
        .map_err(|e| e.to_string())
}

/// Create the languages folder with a current `_template.json` (the English catalog).
pub fn prepare(directory: &Path, template: &str) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    serde_json::from_str::<serde_json::Value>(template).map_err(|e| e.to_string())?;
    fs::write(directory.join(TEMPLATE), template).map_err(|e| e.to_string())
}

/// The languages folder, created with a current template; returns its path.
#[tauri::command]
pub async fn prepare_languages_folder(
    app: tauri::AppHandle,
    template: String,
) -> Result<String, String> {
    let directory = languages_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        prepare(&directory, &template)?;
        Ok(directory.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Options → Language → Open languages folder.
#[tauri::command]
pub async fn open_languages_folder(app: tauri::AppHandle, template: String) -> Result<(), String> {
    let directory = languages_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        prepare(&directory, &template)?;
        #[cfg(desktop)]
        return tauri_plugin_opener::open_path(&directory, None::<&str>).map_err(|e| e.to_string());
        #[cfg(mobile)]
        return Err("Opening folders is not available here".into());
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_are_read_sorted_and_the_template_and_broken_files_are_left_out() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("fr.json"), r#"{"Save tags":"Enregistrer"}"#).unwrap();
        fs::write(
            dir.path().join("de.json"),
            b"\xEF\xBB\xBF{\"@meta\":{\"name\":\"Deutsch\"}}",
        )
        .unwrap();
        fs::write(dir.path().join("broken.json"), "{").unwrap();
        fs::write(dir.path().join("list.json"), "[]").unwrap();
        fs::write(dir.path().join("notes.txt"), "{}").unwrap();
        prepare(dir.path(), r#"{"Save tags":"Save tags"}"#).unwrap();
        let result = read_catalogs(dir.path());
        let files: Vec<&str> = result.catalogs.iter().map(|c| c.file.as_str()).collect();
        assert_eq!(files, ["de.json", "fr.json"]);
        assert_eq!(result.catalogs[1].catalog["Save tags"], "Enregistrer");
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors[0].starts_with("broken.json: "));
        assert_eq!(result.errors[1], "list.json: not a JSON object");
        assert!(dir.path().join(TEMPLATE).is_file());
    }

    #[test]
    fn a_missing_folder_has_no_catalogs_and_an_invalid_template_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            read_catalogs(&dir.path().join("missing")),
            UserCatalogs::default()
        );
        assert!(prepare(&dir.path().join("languages"), "{").is_err());
    }
}
