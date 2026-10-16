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
    pub errors: Vec<CatalogReadError>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CatalogReadError {
    pub file: String,
    pub error: String,
    pub kind: CatalogReadErrorKind,
}
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogReadErrorKind {
    TooLarge,
    NotObject,
    External,
}

fn languages_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::utilities::configuration_directory(app)?.join("languages"))
}

/// Read a complete listing, except files starting with `_` (the template).
/// An absent folder is empty; enumeration failures must not replace a good snapshot.
pub fn read_catalogs(directory: &Path) -> Result<UserCatalogs, String> {
    let mut result = UserCatalogs::default();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .path();
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('_'))
        {
            continue;
        }
        let metadata =
            fs::metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if metadata.is_file() {
            paths.push((path, metadata.len()));
        }
    }
    paths.sort();
    for (path, size) in paths {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let read = || -> Result<serde_json::Value, (CatalogReadErrorKind, String)> {
            if size > CATALOG_LIMIT {
                return Err((
                    CatalogReadErrorKind::TooLarge,
                    "the file is too large".into(),
                ));
            }
            let bytes =
                fs::read(&path).map_err(|e| (CatalogReadErrorKind::External, e.to_string()))?;
            let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
            let value: serde_json::Value = serde_json::from_slice(bytes)
                .map_err(|e| (CatalogReadErrorKind::External, e.to_string()))?;
            if value.is_object() {
                Ok(value)
            } else {
                Err((CatalogReadErrorKind::NotObject, "not a JSON object".into()))
            }
        };
        match read() {
            Ok(catalog) => result.catalogs.push(UserCatalog { file, catalog }),
            Err((kind, error)) => result.errors.push(CatalogReadError { file, error, kind }),
        }
    }
    Ok(result)
}

/// Catalog files of the languages folder; a missing folder has none.
#[tauri::command]
pub async fn language_catalogs(app: tauri::AppHandle) -> Result<UserCatalogs, String> {
    let directory = languages_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || read_catalogs(&directory))
        .await
        .map_err(|e| e.to_string())?
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
    fn read_error_ipc_keeps_warning_shaped_filenames_separate() {
        let directory = tempfile::tempdir().unwrap();
        let names = [
            "fr; 3 missing, 0 invalid.json",
            #[cfg(unix)]
            "fr: 3 missing, 0 invalid.json",
            #[cfg(unix)]
            "fr\nwarning.json",
        ];
        for name in &names {
            fs::write(directory.path().join(name), "[]").unwrap();
        }
        fs::write(
            directory.path().join("large.json"),
            vec![b' '; CATALOG_LIMIT as usize + 1],
        )
        .unwrap();
        fs::write(directory.path().join("broken.json"), "{").unwrap();
        let result = read_catalogs(directory.path()).unwrap();
        assert!(result.catalogs.is_empty());
        assert_eq!(result.errors.len(), names.len() + 2);
        let json = serde_json::to_value(&result).unwrap();
        for name in names {
            let error = json["errors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|error| error["file"] == name)
                .unwrap();
            assert_eq!(error["error"], "not a JSON object");
            assert_eq!(error["kind"], "not_object");
        }
        let large = result
            .errors
            .iter()
            .find(|error| error.file == "large.json")
            .unwrap();
        assert_eq!(large.kind, CatalogReadErrorKind::TooLarge);
        assert_eq!(large.error, "the file is too large");
        let broken = result
            .errors
            .iter()
            .find(|error| error.file == "broken.json")
            .unwrap();
        assert_eq!(broken.kind, CatalogReadErrorKind::External);
        assert!(!broken.error.is_empty());
    }

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
        fs::create_dir(dir.path().join("subfolder.json")).unwrap();
        prepare(dir.path(), r#"{"Save tags":"Save tags"}"#).unwrap();
        let result = read_catalogs(dir.path()).unwrap();
        let files: Vec<&str> = result.catalogs.iter().map(|c| c.file.as_str()).collect();
        assert_eq!(files, ["de.json", "fr.json"]);
        assert_eq!(result.catalogs[1].catalog["Save tags"], "Enregistrer");
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors[0].file, "broken.json");
        assert!(!result.errors[0].error.is_empty());
        assert_eq!(
            result.errors[1],
            CatalogReadError {
                file: "list.json".into(),
                error: "not a JSON object".into(),
                kind: CatalogReadErrorKind::NotObject
            }
        );
        assert!(dir.path().join(TEMPLATE).is_file());
    }

    #[test]
    fn an_unenumerable_languages_path_is_reported_instead_of_an_empty_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("languages");
        fs::write(&path, "not a directory").unwrap();
        let error = read_catalogs(&path).unwrap_err();
        assert!(error.starts_with(&format!("{}: ", path.display())));
    }

    #[test]
    #[cfg(unix)]
    fn uncertain_catalog_metadata_does_not_publish_a_partial_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("fr.json"), r#"{"Save tags":"Enregistrer"}"#).unwrap();
        let broken = dir.path().join("unreadable.json");
        std::os::unix::fs::symlink(dir.path().join("missing"), &broken).unwrap();
        let error = read_catalogs(dir.path()).unwrap_err();
        assert!(error.starts_with(&format!("{}: ", broken.display())));
    }

    #[test]
    fn a_missing_folder_has_no_catalogs_and_an_invalid_template_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            read_catalogs(&dir.path().join("missing")).unwrap(),
            UserCatalogs::default()
        );
        assert!(prepare(&dir.path().join("languages"), "{").is_err());
    }
}
