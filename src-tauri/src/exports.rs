use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub template: String,
    /// The `.mte` file in the export folder holding the template, for
    /// configurations found there (Mp3tag names them after the
    /// configuration). Configurations made in OpenTag use `<id>.mte`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

impl Configuration {
    fn file_name(&self) -> String {
        self.file
            .clone()
            .unwrap_or_else(|| format!("{}.mte", self.id))
    }
}

/// A plain `.mte` file name inside the export folder.
fn valid_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['/', '\\', ':', '\0'])
        && name != ".."
        && Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mte"))
}

fn legacy_codepage() -> Option<String> {
    crate::evaluation::EvaluationSettings::default().ansi_codepage()
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
/// Built-in cue sheet export: one FILE and TRACK per file in File List order,
/// which File → Load playlist/cue sheet reads back.
pub const CUE_SHEET_TEMPLATE: &str = "$filename(cue,utf-8)TITLE \"%album%\"\r\nPERFORMER \"%albumartist%\"\r\n$loop(1)FILE \"%_filename_rel%\" $if($eql($lower(%_extension%),mp3),MP3,WAVE)\r\n  TRACK $num(%_counter%,2) AUDIO\r\n    TITLE \"%title%\"\r\n    PERFORMER \"%artist%\"\r\n    INDEX 01 00:00:00\r\n$loopend()";

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            configurations: vec![
                Configuration {
                    id: "text".into(),
                    name: "Text".into(),
                    template:
                        "$filename(txt,utf-8)$loop(1)%artist% - %title%$char(13)$char(10)$loopend()"
                            .into(),
                    file: None,
                },
                Configuration {
                    id: "cue-sheet".into(),
                    name: "Cue sheet".into(),
                    template: CUE_SHEET_TEMPLATE.into(),
                    file: None,
                },
            ],
            preferences: Preferences::default(),
        }
    }
}

/// Options → Export (`pages.export`) values for writing BOMs and creating
/// directories. Values never saved there fall back to the export library's
/// preferences, where earlier versions stored them.
pub fn output_options(page: Option<&serde_json::Value>, saved: &Preferences) -> (bool, bool) {
    let flag = |key: &str| page.and_then(|p| p.get(key)).and_then(|v| v.as_bool());
    (
        flag("write_bom").unwrap_or(saved.write_bom),
        flag("dont_create_directories").map_or(saved.create_directories, |v| !v),
    )
}
pub fn current_output_options(saved: &Preferences) -> (bool, bool) {
    output_options(crate::settings::current().pages.get("export"), saved)
}

/// File → Export with Shift (Ctrl+Shift+E): one export file per directory.
/// Returns the file name and partition used instead of the saved ones: names
/// that do not already depend on %_folderpath% are placed in each folder.
pub fn per_directory(filename: &str, template: &str) -> (String, String) {
    let partition = "%_folderpath%".to_string();
    let filename = filename.trim();
    if filename.to_ascii_lowercase().contains("%_folderpath%") {
        return (filename.into(), partition);
    }
    let base = |name: &str| name.rsplit(['/', '\\']).next().unwrap_or(name).to_string();
    let name = if filename.is_empty() {
        // Default name of the configuration: $filename(ext) or $filename(name).
        let argument = template
            .trim_start()
            .get(..10)
            .filter(|head| head.eq_ignore_ascii_case("$filename("))
            .and_then(|_| template.trim_start()[10..].split([',', ')']).next())
            .map(str::trim)
            .unwrap_or_default();
        if argument.is_empty() {
            "export.txt".into()
        } else if argument.contains(['.', '/', '\\', ':']) {
            base(argument)
        } else {
            format!("export.{argument}")
        }
    } else {
        base(filename)
    };
    (format!("%_folderpath%{name}"), partition)
}

fn validate(library: &Library, templates: bool) -> Result<(), String> {
    if library.version != 1 {
        return Err("Unsupported export library version".into());
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut files = HashSet::new();
    for configuration in &library.configurations {
        if configuration
            .file
            .as_deref()
            .is_some_and(|file| !valid_file_name(file))
            || !files.insert(configuration.file_name().to_lowercase())
        {
            return Err("Export configurations require distinct .mte file names".into());
        }
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
    let directory = path.parent().unwrap_or(Path::new(".")).join("export");
    let codepage = legacy_codepage();
    if path.exists() {
        // Validate identities before using them as stable MTE filenames.
        validate(&library, false)?;
        let mut kept = Vec::with_capacity(library.configurations.len());
        for mut configuration in std::mem::take(&mut library.configurations) {
            let source = directory.join(configuration.file_name());
            let bytes = match fs::read(&source) {
                Ok(bytes) => bytes,
                // A file found in the folder and deleted there since leaves
                // the list with it.
                Err(e)
                    if e.kind() == std::io::ErrorKind::NotFound && configuration.file.is_some() =>
                {
                    continue
                }
                Err(e) => return Err(format!("Cannot read {}: {e}", source.display())),
            };
            configuration.template = tag_core::decode_export_template(&bytes, codepage.as_deref())?;
            kept.push(configuration);
        }
        library.configurations = kept;
        if library
            .preferences
            .configuration
            .as_deref()
            .is_some_and(|id| !library.configurations.iter().any(|c| c.id == id))
        {
            library.preferences.configuration =
                library.configurations.first().map(|c| c.id.clone());
        }
    }
    discover(&directory, &mut library, codepage.as_deref());
    validate(&library, false)?;
    Ok(library)
}

/// Add the `.mte` files in the export folder that no configuration uses yet,
/// such as configurations copied from Mp3tag, named after their file.
/// Unreadable files are skipped.
fn discover(directory: &Path, library: &mut Library, codepage: Option<&str>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let used: HashSet<String> = library
        .configurations
        .iter()
        .map(|c| c.file_name().to_lowercase())
        .collect();
    let mut found: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| valid_file_name(name) && !used.contains(&name.to_lowercase()))
        .collect();
    found.sort_by_key(|name| name.to_lowercase());
    for file in found {
        let Ok(bytes) = fs::read(directory.join(&file)) else {
            continue;
        };
        let Ok(template) = tag_core::decode_export_template(&bytes, codepage) else {
            continue;
        };
        let stem = Path::new(&file)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .trim()
            .to_string();
        let taken = |name: &str| {
            library
                .configurations
                .iter()
                .any(|c| c.name.to_lowercase() == name.to_lowercase())
        };
        let base = if stem.is_empty() {
            "Export".to_string()
        } else {
            stem
        };
        let mut name = base.clone();
        for counter in 2.. {
            if !taken(&name) {
                break;
            }
            name = format!("{base} ({counter})");
        }
        // Stable across loads, so the selected configuration survives until
        // the library is saved with the file recorded.
        let slug: String = base
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let mut id = format!(
            "mte-{}-{:08x}",
            slug.trim_matches('-'),
            crc32fast::hash(file.to_lowercase().as_bytes())
        );
        while library
            .configurations
            .iter()
            .any(|c| c.id.eq_ignore_ascii_case(&id))
        {
            id.push('x');
        }
        library.configurations.push(Configuration {
            id,
            name,
            template,
            file: Some(file),
        });
    }
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
    let codepage = legacy_codepage();
    for configuration in &library.configurations {
        let target = path
            .parent()
            .unwrap_or(Path::new("."))
            .join("export")
            .join(configuration.file_name());
        // Unchanged files keep their bytes and encoding.
        if fs::read(&target).ok().is_some_and(|bytes| {
            tag_core::decode_export_template(&bytes, codepage.as_deref()).as_deref()
                == Ok(configuration.template.as_str())
        }) {
            continue;
        }
        // Files shared with Mp3tag get a BOM, so they are not read as ANSI.
        let bytes = if configuration.file.is_some() {
            [
                b"\xef\xbb\xbf".as_slice(),
                configuration.template.as_bytes(),
            ]
            .concat()
        } else {
            configuration.template.as_bytes().to_vec()
        };
        crate::action_host::atomic_output(&target, &bytes, false)?;
    }
    #[derive(Serialize)]
    struct SavedConfiguration<'a> {
        id: &'a str,
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        file: Option<&'a str>,
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
                file: c.file.as_deref(),
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
        !library.configurations.iter().any(|current| {
            current.id == old.id || current.file_name().eq_ignore_ascii_case(&old.file_name())
        })
    }) {
        let directory = path.parent().unwrap_or(Path::new(".")).join("export");
        let source = directory.join(removed.file_name());
        let archive = directory.join("removed");
        let stem = Path::new(&removed.file_name())
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let retire = || -> std::io::Result<()> {
            fs::create_dir_all(&archive)?;
            fs::rename(
                &source,
                archive.join(format!("{stem}-{}.mte", uuid::Uuid::new_v4())),
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
    let codepage = legacy_encoding
        .map(str::to_owned)
        .filter(|encoding| !encoding.trim().is_empty())
        .or_else(legacy_codepage);
    let template = tag_core::decode_export_template(
        &fs::read(path).map_err(|e| e.to_string())?,
        codepage.as_deref(),
    )?;
    Ok(Configuration {
        id: uuid::Uuid::new_v4().to_string(),
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        template,
        file: None,
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
        assert_eq!(retired.len(), 2);
        assert!(retired
            .iter()
            .any(|path| fs::read_to_string(path).unwrap() == original));
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
    fn cue_sheet_export_lists_files_in_order_and_loads_back() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::create_dir_all(root.join("disc")).unwrap();
        let row = |name: &str, title: &str| {
            let path = root.join(name);
            std::fs::write(&path, b"x").unwrap();
            let fields: tag_core::Fields = [
                ("TITLE".to_string(), vec![title.to_string()]),
                ("ARTIST".to_string(), vec!["Band".to_string()]),
                ("ALBUM".to_string(), vec!["Live".to_string()]),
            ]
            .into();
            tag_core::ExportRow::new(fields, path.to_string_lossy(), 1.0, 1)
        };
        let rows = [row("disc/b.mp3", "One"), row("disc/a.flac", "Two")];
        let options = tag_core::ExportOptions {
            filename: root.join("export.txt").to_string_lossy().into_owned(),
            base_directory: root.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let documents = tag_core::render_export(CUE_SHEET_TEMPLATE, &rows, &options).unwrap();
        let document = &documents[0];
        assert!(document.filename.ends_with("export.cue"));
        let separator = std::path::MAIN_SEPARATOR;
        assert_eq!(
            document.text,
            format!("TITLE \"Live\"\r\nPERFORMER \"\"\r\nFILE \"disc{separator}b.mp3\" MP3\r\n  TRACK 01 AUDIO\r\n    TITLE \"One\"\r\n    PERFORMER \"Band\"\r\n    INDEX 01 00:00:00\r\nFILE \"disc{separator}a.flac\" WAVE\r\n  TRACK 02 AUDIO\r\n    TITLE \"Two\"\r\n    PERFORMER \"Band\"\r\n    INDEX 01 00:00:00\r\n")
        );
        std::fs::write(&document.filename, &document.text).unwrap();
        let loaded =
            tag_core::playlist::load_playlist(Path::new(&document.filename), None).unwrap();
        assert_eq!(
            loaded.files,
            [root.join("disc/b.mp3"), root.join("disc/a.flac")]
        );
    }

    #[test]
    fn per_directory_exports_place_one_file_in_each_folder() {
        let folder = |name: &str| (format!("%_folderpath%{name}"), "%_folderpath%".to_string());
        assert_eq!(
            per_directory("", "$filename(csv,utf-8)$loop(1)$loopend()"),
            folder("export.csv")
        );
        assert_eq!(
            per_directory("", "$filename(%album%.html)x"),
            folder("%album%.html")
        );
        assert_eq!(
            per_directory("", "$filename(out/list.txt)x"),
            folder("list.txt")
        );
        assert_eq!(per_directory("", "plain"), folder("export.txt"));
        assert_eq!(per_directory("/tmp/x.txt", ""), folder("x.txt"));
        assert_eq!(per_directory("C:\\out\\y.txt", ""), folder("y.txt"));
        assert_eq!(
            per_directory("%_folderpath%sub\\z.txt", ""),
            ("%_folderpath%sub\\z.txt".into(), "%_folderpath%".into())
        );
    }

    #[test]
    fn options_page_overrides_library_output_preferences() {
        let saved = Preferences {
            write_bom: true,
            create_directories: false,
            ..Default::default()
        };
        assert_eq!(output_options(None, &saved), (true, false));
        let page = serde_json::json!({"write_bom":false,"dont_create_directories":false});
        assert_eq!(output_options(Some(&page), &saved), (false, true));
        let partial = serde_json::json!({"dont_create_directories":true});
        assert_eq!(
            output_options(Some(&partial), &Preferences::default()),
            (false, false)
        );
    }

    #[test]
    fn mte_files_dropped_into_the_export_folder_are_listed_and_kept() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        let folder = directory.path().join("export");
        // Before anything was saved, and next to UUID-named configurations.
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("HTML List.mte"), "$filename(html,utf-8)<ul>").unwrap();
        // An ANSI file from Mp3tag on Windows.
        fs::write(folder.join("Cue Sheet.MTE"), b"Bj\xf6rk").unwrap();
        fs::write(folder.join("notes.txt"), "not a configuration").unwrap();
        let library = load(&path).unwrap();
        let names: Vec<_> = library
            .configurations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["Text", "Cue sheet", "Cue Sheet (2)", "HTML List"]);
        let html = &library.configurations[3];
        assert_eq!(html.file.as_deref(), Some("HTML List.mte"));
        assert_eq!(html.template, "$filename(html,utf-8)<ul>");
        assert_eq!(library.configurations[2].template, "Björk");
        // Identities are stable until the library records the files.
        assert_eq!(load(&path).unwrap().configurations[3].id, html.id);

        let mut saved = library.clone();
        saved.preferences.configuration = Some(html.id.clone());
        saved.configurations[3].template = "$filename(html,utf-8)<ol>".into();
        save(&path, &saved).unwrap();
        assert_eq!(
            fs::read_to_string(folder.join("HTML List.mte")).unwrap(),
            "\u{feff}$filename(html,utf-8)<ol>"
        );
        // The untouched ANSI file is not rewritten.
        assert_eq!(fs::read(folder.join("Cue Sheet.MTE")).unwrap(), b"Bj\xf6rk");
        assert!(!folder.join(format!("{}.mte", html.id)).exists());
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded.configurations.len(), 4);
        assert_eq!(reloaded.preferences.configuration, Some(html.id.clone()));

        // Another file dropped later joins the saved library.
        fs::write(folder.join("Late.mte"), "late").unwrap();
        assert_eq!(load(&path).unwrap().configurations[4].name, "Late");
        // Deleting a found file in the folder removes it from the list.
        fs::remove_file(folder.join("HTML List.mte")).unwrap();
        let without = load(&path).unwrap();
        assert!(without.configurations.iter().all(|c| c.name != "HTML List"));
        assert_eq!(without.preferences.configuration.as_deref(), Some("text"));
        // Removing a found configuration retires its file.
        let mut removed = load(&path).unwrap();
        removed.configurations.retain(|c| c.name != "Late");
        save(&path, &removed).unwrap();
        assert!(!folder.join("Late.mte").exists());
        assert!(load(&path)
            .unwrap()
            .configurations
            .iter()
            .all(|c| c.name != "Late"));
    }

    #[test]
    fn recorded_file_names_cannot_leave_the_export_folder() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        for file in ["../escape.mte", "sub/x.mte", "x.txt", ""] {
            let mut library = Library::default();
            library.configurations[0].file = Some(file.into());
            assert!(save(&path, &library).is_err(), "{file}");
        }
        let mut library = Library::default();
        library.configurations[1].file = Some("TEXT.mte".into());
        library.configurations[0].file = Some("text.mte".into());
        assert!(save(&path, &library).is_err());
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
