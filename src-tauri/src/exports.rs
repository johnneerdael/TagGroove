use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub template: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub configuration: Option<String>,
    pub filename: String,
    pub append: bool,
    pub partition: Option<String>,
    pub write_bom: bool,
    pub create_directories: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            configuration: Some("text".into()),
            filename: String::new(),
            append: false,
            partition: None,
            write_bom: false,
            create_directories: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub configurations: Vec<Configuration>,
    pub preferences: Preferences,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            configurations: vec![Configuration {
                id: "text".into(),
                name: "Text".into(),
                template:
                    "$filename(txt,utf-8)$loop(1)%artist% - %title%$char(13)$char(10)$loopend()"
                        .into(),
            }],
            preferences: Preferences::default(),
        }
    }
}

fn validate(library: &Library, templates: bool) -> Result<(), String> {
    if library.version != 1 {
        return Err("Unsupported export library version".into());
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for configuration in &library.configurations {
        if configuration.id.trim().is_empty()
            || !configuration
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            || !ids.insert(configuration.id.to_ascii_lowercase())
            || configuration.name.trim().is_empty()
            || !names.insert(configuration.name.to_lowercase())
        {
            return Err(
                "Export configurations require unique IDs and nonempty unique names".into(),
            );
        }
        if templates {
            tag_core::validate_export_template(&configuration.template)
                .map_err(|e| format!("{}: {e}", configuration.name))?;
        }
    }
    if library
        .preferences
        .configuration
        .as_deref()
        .is_some_and(|id| !library.configurations.iter().any(|c| c.id == id))
    {
        return Err("Selected export configuration does not exist".into());
    }
    if let Some(partition) = &library.preferences.partition {
        if partition.trim().is_empty() {
            return Err("Enter a partition format or disable partitioning".into());
        }
        tag_core::CompiledTemplate::compile(partition)?;
    }
    if !library.preferences.filename.trim().is_empty() {
        tag_core::CompiledTemplate::compile(&library.preferences.filename)?;
    }
    Ok(())
}
pub fn load(path: &Path) -> Result<Library, String> {
    let mut library: Library = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read export configurations: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Library::default(),
        Err(e) => return Err(e.to_string()),
    };
    if path.exists() {
        // Validate identities before using them as stable MTE filenames.
        validate(&library, false)?;
        for configuration in &mut library.configurations {
            let source = path
                .parent()
                .unwrap_or(Path::new("."))
                .join("export")
                .join(format!("{}.mte", configuration.id));
            configuration.template = tag_core::decode_export_template(
                &fs::read(&source).map_err(|e| format!("Cannot read {}: {e}", source.display()))?,
                None,
            )?;
        }
    }
    validate(&library, false)?;
    Ok(library)
}

/// Read global export options without loading or compiling the template library.
pub fn saved_preferences(path: &Path) -> Result<Option<Preferences>, String> {
    #[derive(Deserialize)]
    struct SavedOptions {
        version: u32,
        preferences: Preferences,
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let saved: SavedOptions =
        serde_json::from_slice(&bytes).map_err(|e| format!("Cannot read export options: {e}"))?;
    if saved.version != 1 {
        return Err("Unsupported export library version".into());
    }
    Ok(Some(saved.preferences))
}
pub fn save(path: &Path, library: &Library) -> Result<(), String> {
    validate(library, true)?;
    let previous = if path.exists() {
        load(path)?.configurations
    } else {
        Vec::new()
    };
    for configuration in &library.configurations {
        let target = path
            .parent()
            .unwrap_or(Path::new("."))
            .join("export")
            .join(format!("{}.mte", configuration.id));
        crate::action_host::atomic_output(&target, configuration.template.as_bytes(), false)?;
    }
    #[derive(Serialize)]
    struct SavedConfiguration<'a> {
        id: &'a str,
        name: &'a str,
    }
    #[derive(Serialize)]
    struct SavedLibrary<'a> {
        version: u32,
        configurations: Vec<SavedConfiguration<'a>>,
        preferences: &'a Preferences,
    }
    let saved = SavedLibrary {
        version: library.version,
        configurations: library
            .configurations
            .iter()
            .map(|c| SavedConfiguration {
                id: &c.id,
                name: &c.name,
            })
            .collect(),
        preferences: &library.preferences,
    };
    crate::action_host::atomic_output(
        path,
        &serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?,
        false,
    )?;
    for removed in previous.iter().filter(|old| {
        !library
            .configurations
            .iter()
            .any(|current| current.id == old.id)
    }) {
        let directory = path.parent().unwrap_or(Path::new(".")).join("export");
        let source = directory.join(format!("{}.mte", removed.id));
        let archive = directory.join("removed");
        let retire = || -> std::io::Result<()> {
            fs::create_dir_all(&archive)?;
            fs::rename(
                &source,
                archive.join(format!("{}-{}.mte", removed.id, uuid::Uuid::new_v4())),
            )
        };
        retire().map_err(|e| {
            format!(
                "Export library saved, but could not retire {}: {e}",
                source.display()
            )
        })?;
    }
    Ok(())
}
pub fn import(path: &Path, legacy_encoding: Option<&str>) -> Result<Configuration, String> {
    let template = tag_core::decode_export_template(
        &fs::read(path).map_err(|e| e.to_string())?,
        legacy_encoding,
    )?;
    Ok(Configuration {
        id: uuid::Uuid::new_v4().to_string(),
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        template,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_saved_configuration_retires_its_mte_from_active_exports() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        let mut library = Library::default();
        save(&path, &library).unwrap();
        let original = library.configurations[0].template.clone();
        library.configurations.clear();
        library.preferences.configuration = None;
        save(&path, &library).unwrap();
        assert!(load(&path).unwrap().configurations.is_empty());
        assert!(!directory.path().join("export/text.mte").exists());
        let retired: Vec<_> = fs::read_dir(directory.path().join("export/removed"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(retired.len(), 1);
        assert_eq!(fs::read_to_string(&retired[0]).unwrap(), original);
    }

    #[test]
    fn saved_templates_are_mte_files_and_external_edits_are_loaded() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let template = directory.path().join("export/text.mte");
        assert!(template.exists());
        fs::write(template, "A plain-text export header").unwrap();
        assert_eq!(
            load(&path).unwrap().configurations[0].template,
            "A plain-text export header"
        );
        fs::write(
            directory.path().join("export/text.mte"),
            "$loop(1)unfinished",
        )
        .unwrap();
        let unfinished = load(&path).unwrap();
        assert_eq!(unfinished.configurations[0].template, "$loop(1)unfinished");
        assert!(save(&path, &unfinished).is_err());
    }

    #[test]
    fn library_roundtrips_templates_and_every_export_preference() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings/exports.json");
        let mut library = load(&path).unwrap();
        library.configurations[0].name = "Mémoire 猫".into();
        library.preferences = Preferences {
            configuration: Some("text".into()),
            filename: "exports/%album%.txt".into(),
            append: true,
            partition: Some("%album%".into()),
            write_bom: true,
            create_directories: false,
        };
        save(&path, &library).unwrap();
        assert_eq!(
            serde_json::to_value(load(&path).unwrap()).unwrap(),
            serde_json::to_value(&library).unwrap()
        );
    }

    #[test]
    fn invalid_libraries_never_replace_the_saved_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        let original = Library::default();
        save(&path, &original).unwrap();
        for kind in 0..6 {
            let mut invalid = original.clone();
            match kind {
                0 => invalid.version = 2,
                1 => invalid
                    .configurations
                    .push(invalid.configurations[0].clone()),
                2 => invalid.configurations[0].name = " ".into(),
                3 => invalid.configurations[0].template = "$loop(1)unclosed".into(),
                4 => invalid.preferences.configuration = Some("missing".into()),
                _ => invalid.preferences.partition = Some(" ".into()),
            }
            assert!(save(&path, &invalid).is_err(), "kind {kind}");
            assert_eq!(
                load(&path).unwrap().configurations[0].template,
                original.configurations[0].template
            );
        }
    }

    #[test]
    fn imports_utf16_mte_as_editable_text_without_changing_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Unicode.mte");
        let text = "$filename(txt,utf-8)$loop(1)%title% 猫$loopend()";
        let bytes = [
            vec![0xff, 0xfe],
            text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        ]
        .concat();
        fs::write(&path, &bytes).unwrap();
        let configuration = import(&path, None).unwrap();
        assert_eq!(configuration.name, "Unicode");
        assert_eq!(configuration.template, text);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
