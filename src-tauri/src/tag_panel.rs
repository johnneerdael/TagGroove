//! Tag panel layout and custom field value lists (Options → Tag Panel / Field Values).
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Default value that leaves differing per-file values untouched (`< blank >` clears the field).
pub const KEEP: &str = "< keep >";
pub const DIRECTORY: &str = "_DIRECTORY";
pub const SEPARATOR: &str = "_SEPARATOR";
pub const MAX_FIELDS: usize = 100;
const MAX_VALUES: usize = 5000;
const MAX_TEXT: usize = 1000;
const LAYOUT_FORMAT: &str = "opentag-tag-panel";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Control {
    Text,
    Check,
    Multiline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Full,
    Larger,
    Large,
    Medium,
    Normal,
    Small,
    Smaller,
    Tiny,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelField {
    pub name: String,
    pub field: String,
    #[serde(rename = "type")]
    pub control: Control,
    pub size: Size,
    pub rows: u8,
    pub default_value: String,
    pub visible: bool,
}
impl Default for PanelField {
    fn default() -> Self {
        Self {
            name: String::new(),
            field: String::new(),
            control: Control::Text,
            size: Size::Full,
            rows: 3,
            default_value: KEEP.into(),
            visible: true,
        }
    }
}
fn field(name: &str, id: &str, size: Size) -> PanelField {
    PanelField {
        name: name.into(),
        field: id.into(),
        size,
        ..PanelField::default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub fields: Vec<PanelField>,
    pub show_panel: bool,
    pub show_cover: bool,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            fields: vec![
                field("Tit&le", "TITLE", Size::Full),
                field("Art&ist", "ARTIST", Size::Full),
                field("&Album", "ALBUM", Size::Full),
                field("&Year", "YEAR", Size::Normal),
                field("T&rack", "TRACK", Size::Normal),
                field("&Genre", "GENRE", Size::Full),
                PanelField {
                    control: Control::Multiline,
                    rows: 3,
                    ..field("Co&mment", "COMMENT", Size::Full)
                },
                field("Album Artist", "ALBUMARTIST", Size::Full),
                field("Composer", "COMPOSER", Size::Full),
                field("Discnumber", "DISCNUMBER", Size::Normal),
                field("&Directory", DIRECTORY, Size::Full),
            ],
            show_panel: true,
            show_cover: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FieldValues {
    pub lists: BTreeMap<String, Vec<String>>,
    pub only_user_genres: bool,
    pub prefer_tag_values: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    #[serde(default)]
    pub panel: Layout,
    #[serde(default)]
    pub field_values: FieldValues,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            panel: Layout::default(),
            field_values: FieldValues::default(),
        }
    }
}

pub fn text(value: &str, label: &str, multiline: bool) -> Result<(), String> {
    if value.chars().count() > MAX_TEXT {
        return Err(format!("{label} is too long"));
    }
    if value
        .chars()
        .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
    {
        return Err(format!("{label} contains control characters"));
    }
    Ok(())
}
pub fn field_id(id: &str, panel: bool) -> Result<(), String> {
    if id.is_empty() || id.trim() != id || id.to_uppercase() != id {
        return Err(format!(
            "Field \"{id}\" must be a trimmed uppercase field name"
        ));
    }
    text(id, "Field name", false)?;
    if id.contains(['=', '%']) {
        return Err(format!("Field \"{id}\" contains an invalid character"));
    }
    if id.starts_with('_') && !(panel && (id == DIRECTORY || id == SEPARATOR)) {
        return Err(format!(
            "{id}: technical fields cannot be edited in the tag panel"
        ));
    }
    Ok(())
}
pub fn validate_layout(layout: &Layout) -> Result<(), String> {
    if layout.fields.len() > MAX_FIELDS {
        return Err(format!(
            "The tag panel supports at most {MAX_FIELDS} fields"
        ));
    }
    let mut used = HashSet::new();
    for entry in &layout.fields {
        field_id(&entry.field, true)?;
        let separator = entry.field == SEPARATOR;
        if !separator && entry.name.trim().is_empty() {
            return Err(format!("{}: enter a name for the field", entry.field));
        }
        text(&entry.name, "Field label", false)?;
        text(
            &entry.default_value,
            "Default value",
            entry.control == Control::Multiline,
        )?;
        if !(1..=20).contains(&entry.rows) {
            return Err(format!("{}: rows must be between 1 and 20", entry.name));
        }
        if !separator && !used.insert(entry.field.clone()) {
            return Err(format!(
                "{} is already used for another field and cannot be added twice",
                entry.field
            ));
        }
    }
    Ok(())
}
pub fn validate(settings: &Settings) -> Result<(), String> {
    if settings.version != 1 {
        return Err("Unsupported tag panel configuration version".into());
    }
    validate_layout(&settings.panel)?;
    for (key, values) in &settings.field_values.lists {
        field_id(key, false)?;
        if values.len() > MAX_VALUES {
            return Err(format!(
                "{key}: at most {MAX_VALUES} custom values are supported"
            ));
        }
        let mut seen = HashSet::new();
        for value in values {
            if value.trim().is_empty() || value.trim() != value {
                return Err(format!("{key}: custom values must be nonempty and trimmed"));
            }
            text(value, "Custom value", false)?;
            if !seen.insert(value.as_str()) {
                return Err(format!("{key}: duplicate custom value \"{value}\""));
            }
        }
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<Settings, String> {
    let settings: Settings = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read tag panel configuration: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
        Err(e) => return Err(e.to_string()),
    };
    validate(&settings)?;
    Ok(settings)
}
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    validate(settings)?;
    crate::action_host::atomic_output(
        path,
        &serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?,
        false,
    )
}

#[derive(Serialize, Deserialize)]
struct LayoutFile {
    format: String,
    version: u32,
    panel: Layout,
}
pub fn export_layout(path: &Path, panel: &Layout) -> Result<(), String> {
    validate_layout(panel)?;
    let file = LayoutFile {
        format: LAYOUT_FORMAT.into(),
        version: 1,
        panel: panel.clone(),
    };
    crate::action_host::atomic_output(
        path,
        &serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?,
        false,
    )
}
pub fn import_layout(path: &Path) -> Result<Layout, String> {
    let file: LayoutFile = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Not a tag panel configuration: {e}"))?;
    if file.format != LAYOUT_FORMAT || file.version != 1 {
        return Err("Not a supported tag panel configuration".into());
    }
    validate_layout(&file.panel)?;
    Ok(file.panel)
}

/// One value per line; BOM-marked UTF-8/UTF-16 and UTF-8 first, Windows-1252 otherwise.
pub fn read_value_list(path: &Path) -> Result<Vec<String>, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let text = tag_core::decode_export_template(&bytes, Some("windows-1252"))
        .map_err(|e| format!("Cannot read value list: {e}"))?;
    let mut seen = HashSet::new();
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && seen.insert(line.to_string()))
        .map(str::to_owned)
        .collect())
}
pub fn write_value_list(path: &Path, values: &[String]) -> Result<(), String> {
    let mut text = String::from("\u{feff}");
    for value in values {
        text.push_str(value);
        text.push_str("\r\n");
    }
    crate::action_host::atomic_output(path, text.as_bytes(), false)
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?.with_file_name("tag-panel.json"))
}
#[tauri::command]
pub async fn load_tag_panel(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    super::io(move || load(&path)).await
}
#[tauri::command]
pub async fn save_tag_panel(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app)?;
    super::io(move || {
        let _guard = crate::profile_migration::mutation_guard(
            path.parent()
                .ok_or("Configuration directory is unavailable")?,
        )?;
        save(&path, &settings)
    })
    .await
}
#[tauri::command]
pub async fn import_tag_panel_layout(path: String) -> Result<Layout, String> {
    super::io(move || import_layout(&super::auxiliary_input(&path)?)).await
}
#[tauri::command]
pub async fn export_tag_panel_layout(path: String, panel: Layout) -> Result<(), String> {
    super::io(move || {
        let _guard = crate::profile_migration::auxiliary_configuration_guard(Path::new(&path))?;
        export_layout(Path::new(&path), &panel)
    })
    .await
}
#[tauri::command]
pub async fn import_value_list(path: String) -> Result<Vec<String>, String> {
    super::io(move || read_value_list(&super::auxiliary_input(&path)?)).await
}
#[tauri::command]
pub async fn export_value_list(path: String, values: Vec<String>) -> Result<(), String> {
    super::io(move || {
        let _guard = crate::profile_migration::auxiliary_configuration_guard(Path::new(&path))?;
        write_value_list(Path::new(&path), &values)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_mp3tag_style_defaults_and_round_trips() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tag-panel.json");
        let mut settings = load(&path).unwrap();
        let ids: Vec<_> = settings
            .panel
            .fields
            .iter()
            .map(|f| f.field.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "TITLE",
                "ARTIST",
                "ALBUM",
                "YEAR",
                "TRACK",
                "GENRE",
                "COMMENT",
                "ALBUMARTIST",
                "COMPOSER",
                "DISCNUMBER",
                "_DIRECTORY"
            ]
        );
        assert!(settings.panel.show_panel && settings.panel.show_cover);
        settings.panel.show_cover = false;
        settings
            .field_values
            .lists
            .insert("GENRE".into(), vec!["Shoegaze".into(), "Synthwave".into()]);
        settings.field_values.only_user_genres = true;
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap(), settings);
    }

    #[test]
    fn older_files_fill_missing_options_with_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tag-panel.json");
        fs::write(
            &path,
            r#"{"version":1,"panel":{"fields":[{"name":"Mood","field":"MOOD"}]}}"#,
        )
        .unwrap();
        let settings = load(&path).unwrap();
        let mood = &settings.panel.fields[0];
        assert_eq!(
            (mood.control, mood.size, mood.default_value.as_str()),
            (Control::Text, Size::Full, KEEP)
        );
        assert!(settings.panel.show_panel && settings.field_values.lists.is_empty());
    }

    #[test]
    fn validation_reports_the_exact_invalid_field_identifier() {
        for (id, expected) in [
            (
                "ARTIST ",
                "Field \"ARTIST \" must be a trimmed uppercase field name",
            ),
            (
                " TITLE",
                "Field \" TITLE\" must be a trimmed uppercase field name",
            ),
            ("ARTIST=", "Field \"ARTIST=\" contains an invalid character"),
            ("TITLE%", "Field \"TITLE%\" contains an invalid character"),
            (
                "_LENGTH",
                "_LENGTH: technical fields cannot be edited in the tag panel",
            ),
        ] {
            let mut settings = Settings::default();
            settings.panel.fields[0].field = id.into();
            assert_eq!(validate(&settings).unwrap_err(), expected);
            let mut settings = Settings::default();
            settings
                .field_values
                .lists
                .insert(id.into(), vec!["Value".into()]);
            assert_eq!(validate(&settings).unwrap_err(), expected);
        }
    }

    #[test]
    fn validation_rejects_duplicates_technical_fields_and_bad_lists() {
        let mut settings = Settings::default();
        settings
            .panel
            .fields
            .push(field("Title again", "TITLE", Size::Full));
        assert!(validate(&settings)
            .unwrap_err()
            .contains("cannot be added twice"));
        let mut settings = Settings::default();
        settings
            .panel
            .fields
            .push(field("Path", "_PATH", Size::Full));
        assert!(validate(&settings).unwrap_err().contains("technical"));
        let mut settings = Settings::default();
        settings
            .panel
            .fields
            .push(field("lower", "mood", Size::Full));
        assert!(validate(&settings).is_err());
        let mut settings = Settings::default();
        let separator = PanelField {
            name: String::new(),
            ..field("", "_SEPARATOR", Size::Full)
        };
        settings.panel.fields.extend([separator.clone(), separator]);
        validate(&settings).unwrap();
        settings
            .field_values
            .lists
            .insert("GENRE".into(), vec!["Rock".into(), "Rock".into()]);
        assert!(validate(&settings).unwrap_err().contains("duplicate"));
        settings
            .field_values
            .lists
            .insert("GENRE".into(), vec![" padded".into()]);
        assert!(validate(&settings).is_err());
        let mut settings = Settings::default();
        settings
            .field_values
            .lists
            .insert("_DIRECTORY".into(), vec!["x".into()]);
        assert!(validate(&settings).is_err());
        let mut settings = Settings::default();
        settings.panel.fields[0].rows = 0;
        assert!(validate(&settings).is_err());
        settings.version = 2;
        assert!(validate(&settings).unwrap_err().contains("version"));
    }

    #[test]
    fn layout_export_import_and_value_lists_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let layout_path = directory.path().join("panel.json");
        let mut layout = Layout::default();
        layout.fields.truncate(3);
        layout.show_cover = false;
        export_layout(&layout_path, &layout).unwrap();
        assert_eq!(import_layout(&layout_path).unwrap(), layout);
        fs::write(&layout_path, r#"{"format":"other","version":1,"panel":{}}"#).unwrap();
        assert!(import_layout(&layout_path).is_err());

        let list_path = directory.path().join("genres.txt");
        write_value_list(&list_path, &["Café".into(), "Rock".into()]).unwrap();
        assert_eq!(read_value_list(&list_path).unwrap(), ["Café", "Rock"]);
        fs::write(&list_path, b"Caf\xe9\r\n\r\n  Rock \nRock\n").unwrap();
        assert_eq!(read_value_list(&list_path).unwrap(), ["Café", "Rock"]);
        let utf16: Vec<u8> = [0xff, 0xfe]
            .into_iter()
            .chain("Jazz\nSoul".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        fs::write(&list_path, utf16).unwrap();
        assert_eq!(read_value_list(&list_path).unwrap(), ["Jazz", "Soul"]);
    }
}
