//! File List column layout persistence and batched column evaluation.
use crate::evaluation::EvaluationSettings;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};
use tag_core::{CompiledTemplate, Fields};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

fn visible_default() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub name: String,
    /// Display format string.
    pub value: String,
    /// Edited placeholder; empty means the column is read-only.
    #[serde(default)]
    pub field: String,
    /// Optional sort format string; the value is used when empty.
    #[serde(default)]
    pub sort_by: String,
    pub width: u32,
    #[serde(default)]
    pub align: Align,
    #[serde(default)]
    pub numeric: bool,
    #[serde(default = "visible_default")]
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SortKey {
    pub id: String,
    pub ascending: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub version: u32,
    pub columns: Vec<Column>,
    #[serde(default)]
    pub sort: Vec<SortKey>,
}

pub const MIN_WIDTH: u32 = 24;
pub const MAX_WIDTH: u32 = 4000;
const MAX_COLUMNS: usize = 200;

/// Resolve an editable column field to its tag key. Only tag field
/// placeholders and the two filename placeholders can be edited.
pub fn editable_field(field: &str) -> Result<Option<String>, String> {
    let field = field.trim();
    if field.is_empty() {
        return Ok(None);
    }
    let name = field
        .strip_prefix('%')
        .and_then(|rest| rest.strip_suffix('%'))
        .filter(|name| {
            !name.trim().is_empty()
                && !name.contains(['%', '$', '(', ')', ',', '[', ']', '\'', '\n', '\r'])
        })
        .ok_or_else(|| format!("Field {field} must be a single placeholder such as %ISRC%"))?
        .trim()
        .to_ascii_uppercase();
    if name.starts_with('_') && name != "_FILENAME" && name != "_FILENAME_EXT" {
        return Err(format!(
            "Field {field} is read-only. Use a tag field, %_filename% or %_filename_ext%"
        ));
    }
    Ok(Some(name))
}

pub fn validate(layout: &Layout) -> Result<(), String> {
    if layout.version != 1 {
        return Err("Unsupported column configuration version".into());
    }
    if layout.columns.is_empty() || layout.columns.len() > MAX_COLUMNS {
        return Err(format!(
            "Configure between 1 and {MAX_COLUMNS} File List columns"
        ));
    }
    if !layout.columns.iter().any(|column| column.visible) {
        return Err("Keep at least one File List column visible".into());
    }
    let mut ids = HashSet::new();
    for column in &layout.columns {
        if column.id.is_empty()
            || column.id.len() > 64
            || !column
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            || !ids.insert(column.id.to_ascii_lowercase())
        {
            return Err("File List columns require unique identifiers".into());
        }
        let label = if column.name.trim().is_empty() {
            return Err("Enter a name for every File List column".into());
        } else {
            column.name.trim()
        };
        if column.value.trim().is_empty() {
            return Err(format!("{label}: enter a value format string"));
        }
        CompiledTemplate::compile(&column.value).map_err(|e| format!("{label}: value: {e}"))?;
        if !column.sort_by.trim().is_empty() {
            CompiledTemplate::compile(&column.sort_by)
                .map_err(|e| format!("{label}: sort by: {e}"))?;
        }
        editable_field(&column.field).map_err(|e| format!("{label}: {e}"))?;
        if !(MIN_WIDTH..=MAX_WIDTH).contains(&column.width) {
            return Err(format!(
                "{label}: width must be between {MIN_WIDTH} and {MAX_WIDTH} pixels"
            ));
        }
    }
    let mut sorted = HashSet::new();
    for key in &layout.sort {
        if !ids.contains(&key.id.to_ascii_lowercase()) || !sorted.insert(key.id.as_str()) {
            return Err("Sort criteria must name distinct configured columns".into());
        }
    }
    Ok(())
}

fn parse(bytes: &[u8]) -> Result<Layout, String> {
    let layout: Layout = serde_json::from_slice(bytes)
        .map_err(|e| format!("Cannot read column configuration: {e}"))?;
    validate(&layout)?;
    Ok(layout)
}

/// `None` means no saved layout; the UI then uses its default column set.
pub fn load(path: &Path) -> Result<Option<Layout>, String> {
    match fs::read(path) {
        Ok(bytes) => parse(&bytes).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn save(path: &Path, layout: &Layout) -> Result<(), String> {
    validate(layout)?;
    crate::action_host::atomic_output(
        path,
        &serde_json::to_vec_pretty(layout).map_err(|e| e.to_string())?,
        false,
    )
}

pub fn import(path: &Path) -> Result<Layout, String> {
    parse(&fs::read(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?)
}

/// Items carry the UI's merged fields (pending drafts included) plus file properties.
#[derive(Debug, Deserialize)]
pub struct ColumnItem {
    pub fields: Fields,
    #[serde(default)]
    pub raw_fields: Option<Fields>,
}

#[derive(Debug, Default, Serialize)]
pub struct Evaluation {
    /// One row per item, one value per template.
    pub values: Vec<Vec<String>>,
    /// Distinct evaluation failures; failing cells are empty.
    pub errors: Vec<String>,
}

/// Evaluate every template for every item, compiling each template once.
pub fn evaluate(
    settings: &EvaluationSettings,
    items: &[ColumnItem],
    templates: &[String],
) -> Evaluation {
    let mut errors = Vec::new();
    let mut note = |message: String| {
        if !errors.contains(&message) && errors.len() < 20 {
            errors.push(message);
        }
    };
    let compiled: Vec<Option<CompiledTemplate>> = templates
        .iter()
        .map(|template| match CompiledTemplate::compile(template) {
            Ok(compiled) => Some(compiled),
            Err(error) => {
                note(format!("{template}: {error}"));
                None
            }
        })
        .collect();
    let values = items
        .iter()
        .map(|item| {
            let raw = item.raw_fields.as_ref().unwrap_or(&item.fields);
            let context = settings.context(&item.fields, raw, None);
            compiled
                .iter()
                .zip(templates)
                .map(|(compiled, template)| match compiled {
                    Some(compiled) => {
                        compiled
                            .evaluate_with_context(&context)
                            .unwrap_or_else(|error| {
                                note(format!("{template}: {error}"));
                                String::new()
                            })
                    }
                    None => String::new(),
                })
                .collect()
        })
        .collect();
    Evaluation { values, errors }
}

fn layout_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(crate::action_host::library_path(app)?.with_file_name("columns.json"))
}

#[tauri::command]
pub async fn load_columns(app: tauri::AppHandle) -> Result<Option<Layout>, String> {
    let path = layout_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || load(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_columns(app: tauri::AppHandle, layout: Layout) -> Result<(), String> {
    let path = layout_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || save(&path, &layout))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_columns(path: String) -> Result<Layout, String> {
    tauri::async_runtime::spawn_blocking(move || import(&super::auxiliary_input(&path)?))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn export_columns(path: String, layout: Layout) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || save(Path::new(&path), &layout))
        .await
        .map_err(|e| e.to_string())?
}

/// Column values are display data only, so evaluation does not wait for file operations.
#[tauri::command]
pub async fn evaluate_columns(
    items: Vec<ColumnItem>,
    templates: Vec<String>,
) -> Result<Evaluation, String> {
    let settings = super::evaluation_settings(None)?;
    tauri::async_runtime::spawn_blocking(move || evaluate(&settings, &items, &templates))
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(id: &str, value: &str, field: &str) -> Column {
        Column {
            id: id.into(),
            name: id.to_uppercase(),
            value: value.into(),
            field: field.into(),
            sort_by: String::new(),
            width: 100,
            align: Align::Left,
            numeric: false,
            visible: true,
        }
    }
    fn layout(columns: Vec<Column>) -> Layout {
        Layout {
            version: 1,
            columns,
            sort: vec![],
        }
    }
    fn fields(pairs: &[(&str, &[&str])]) -> Fields {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    #[test]
    fn editable_fields_accept_tag_and_filename_placeholders_only() {
        assert_eq!(editable_field("").unwrap(), None);
        assert_eq!(editable_field("%isrc%").unwrap().as_deref(), Some("ISRC"));
        assert_eq!(
            editable_field(" %Album Artist% ").unwrap().as_deref(),
            Some("ALBUM ARTIST")
        );
        assert_eq!(
            editable_field("%_filename_ext%").unwrap().as_deref(),
            Some("_FILENAME_EXT")
        );
        for invalid in [
            "%_length%",
            "artist",
            "%artist% %title%",
            "$upper(%a%)",
            "%%",
        ] {
            assert!(editable_field(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn validation_rejects_unusable_layouts() {
        let valid = layout(vec![
            column("title", "%title%", "%title%"),
            column("length", "%_length%", ""),
        ]);
        validate(&valid).unwrap();
        let mut duplicate = valid.clone();
        duplicate.columns[1].id = "TITLE".into();
        assert!(validate(&duplicate).is_err());
        let mut hidden = valid.clone();
        hidden.columns.iter_mut().for_each(|c| c.visible = false);
        assert!(validate(&hidden).unwrap_err().contains("visible"));
        let mut broken = valid.clone();
        broken.columns[0].value = "$if(%title%".into();
        assert!(validate(&broken).unwrap_err().starts_with("TITLE: value"));
        let mut sort = valid.clone();
        sort.columns[1].sort_by = "$num(".into();
        assert!(validate(&sort).unwrap_err().contains("sort by"));
        let mut narrow = valid.clone();
        narrow.columns[0].width = 2;
        assert!(validate(&narrow).is_err());
        let mut unknown = valid.clone();
        unknown.sort = vec![SortKey {
            id: "missing".into(),
            ascending: true,
        }];
        assert!(validate(&unknown).is_err());
        let mut readonly = valid;
        readonly.columns[1].field = "%_length%".into();
        assert!(validate(&readonly).unwrap_err().contains("read-only"));
    }

    #[test]
    fn layouts_round_trip_and_missing_files_use_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join("columns.json");
        assert_eq!(load(&path).unwrap(), None);
        let mut saved = layout(vec![column("track", "$num(%track%,2)", "%track%")]);
        saved.columns[0].numeric = true;
        saved.columns[0].align = Align::Right;
        saved.sort = vec![SortKey {
            id: "track".into(),
            ascending: false,
        }];
        save(&path, &saved).unwrap();
        assert_eq!(load(&path).unwrap(), Some(saved.clone()));
        assert_eq!(import(&path).unwrap(), saved);
        fs::write(
            &path,
            br#"{"version":1,"columns":[{"id":"a","name":"A","value":"%a%","width":80}]}"#,
        )
        .unwrap();
        let minimal = load(&path).unwrap().unwrap();
        assert!(minimal.columns[0].visible && minimal.columns[0].field.is_empty());
        fs::write(&path, b"{").unwrap();
        assert!(load(&path).unwrap_err().contains("column configuration"));
    }

    #[test]
    fn batch_evaluation_uses_scripting_and_isolates_failures() {
        let settings = EvaluationSettings::default();
        let items = vec![
            ColumnItem {
                fields: fields(&[
                    ("TRACK", &["3"]),
                    ("ARTIST", &["A", "B"]),
                    ("_LENGTH", &["3:05"]),
                ]),
                raw_fields: None,
            },
            ColumnItem {
                fields: fields(&[("TRACK", &["12"])]),
                raw_fields: None,
            },
        ];
        let templates = vec![
            "$num(%track%,2)".to_string(),
            "$meta_sep(artist,' \\\\ ')".to_string(),
            "%_length%".to_string(),
            "$columnvalues(<,>)".to_string(),
            "$if(".to_string(),
        ];
        let result = evaluate(&settings, &items, &templates);
        assert_eq!(result.values[0][..3], ["03", "A \\\\ B", "3:05"]);
        assert_eq!(result.values[1][..3], ["12", "", ""]);
        assert!(result
            .values
            .iter()
            .all(|row| row[3].is_empty() && row[4].is_empty()));
        assert_eq!(result.errors.len(), 2, "{:?}", result.errors);
    }

    #[test]
    fn batch_evaluation_of_a_large_list_stays_fast() {
        let settings = EvaluationSettings::default();
        let items: Vec<_> = (0..1000)
            .map(|i| ColumnItem {
                fields: fields(&[
                    ("TITLE", &[&format!("Title {i}")]),
                    ("ARTIST", &["Artist"]),
                    ("ALBUM", &["Album"]),
                    ("TRACK", &[&(i % 20).to_string()]),
                    ("_FILENAME_EXT", &[&format!("{i}.flac")]),
                    ("_FOLDERPATH", &["/music/"]),
                    ("_LENGTH", &["3:00"]),
                    ("_BITRATE", &["900"]),
                ]),
                raw_fields: None,
            })
            .collect();
        let templates: Vec<String> = [
            "%_filename_ext%",
            "%_folderpath%",
            "$meta_sep(title,' \\\\ ')",
            "$meta_sep(artist,' \\\\ ')",
            "$meta_sep(album,' \\\\ ')",
            "$num(%track%,2)",
            "%year%",
            "$meta_sep(genre,' \\\\ ')",
            "%_length%",
            "%_bitrate% kbps",
            "%_codec%",
            "%_length_seconds%",
        ]
        .map(String::from)
        .to_vec();
        let started = std::time::Instant::now();
        let result = evaluate(&settings, &items, &templates);
        let elapsed = started.elapsed();
        assert_eq!(result.values.len(), 1000);
        assert_eq!(result.values[7][5], "07");
        assert!(result.errors.is_empty());
        // Generous bound for unoptimized CI builds; release builds are far faster.
        assert!(elapsed.as_secs_f64() < 2.0, "{elapsed:?}");
        eprintln!(
            "evaluated 1000 x {} column cells in {elapsed:?}",
            templates.len()
        );
    }
}
