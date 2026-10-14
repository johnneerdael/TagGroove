use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use tag_core::{AudioEntry, EvalContext, Fields, ScriptColumn};
use tauri::Manager;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColumnSpec {
    pub name: String,
    pub template: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
pub struct EvaluationSettings {
    pub columns: Vec<ColumnSpec>,
    pub app_directory: String,
    pub app_data_directory: String,
}

impl EvaluationSettings {
    pub fn ansi_codepage(&self) -> Option<String> {
        #[cfg(windows)]
        {
            Some(windows_ansi_codepage().to_string())
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
    pub fn for_app(
        app: &tauri::AppHandle,
        columns: Option<Vec<ColumnSpec>>,
    ) -> Result<Self, String> {
        Ok(Self {
            columns: columns.unwrap_or_default(),
            app_directory: std::env::current_exe()
                .map_err(|e| e.to_string())?
                .parent()
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .into_owned(),
            app_data_directory: app
                .path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned(),
        })
    }

    fn context<'a>(
        &'a self,
        fields: &'a Fields,
        raw: &'a Fields,
        columns: Option<&'a [ScriptColumn]>,
    ) -> EvalContext<'a> {
        let mut context = EvalContext::new(fields);
        context.raw_fields = Some(raw);
        context.columns = columns;
        context.app_directory = Some(&self.app_directory);
        context.app_data_directory = Some(&self.app_data_directory);
        #[cfg(windows)]
        {
            context.ansi_codepage = Some(windows_ansi_codepage().to_string());
        }
        context
    }

    pub fn column_values(
        &self,
        fields: &Fields,
        raw: &Fields,
        identity: &str,
        use_snapshot: bool,
    ) -> Result<Vec<ScriptColumn>, String> {
        if self.columns.is_empty() {
            return Ok(default_columns(fields, raw));
        }
        self.columns
            .iter()
            .map(|column| {
                let value = if use_snapshot {
                    column.values.get(identity).cloned()
                } else {
                    None
                };
                let value = match value {
                    Some(value) => value,
                    None => tag_core::evaluate_with_context(
                        &column.template,
                        &self.context(fields, raw, None),
                    )?,
                };
                Ok(ScriptColumn {
                    name: column.name.clone(),
                    value,
                })
            })
            .collect()
    }

    pub fn evaluate(
        &self,
        template: &str,
        raw: &Fields,
        technical: &Fields,
        identity: &str,
        use_snapshot: bool,
    ) -> Result<String, String> {
        let mut fields = raw.clone();
        fields.extend(technical.clone());
        let columns = self.column_values(&fields, raw, identity, use_snapshot)?;
        tag_core::evaluate_with_context(template, &self.context(&fields, raw, Some(&columns)))
    }

    pub fn matches(
        &self,
        filter: &tag_core::filter::Filter,
        fields: &Fields,
        raw: &Fields,
        identity: &str,
    ) -> Result<bool, String> {
        let columns = self.column_values(fields, raw, identity, true)?;
        filter.matches_with_context(&self.context(fields, raw, Some(&columns)))
    }

    pub fn filename(
        &self,
        template: &str,
        entry: &AudioEntry,
        index: usize,
    ) -> Result<String, String> {
        let fields = enriched_fields(entry, index);
        let columns = self.column_values(&fields, &entry.fields, &entry.path, true)?;
        tag_core::evaluate_filename_with_context(
            template,
            &self.context(&fields, &entry.fields, Some(&columns)),
        )
    }
}

#[cfg(windows)]
fn windows_ansi_codepage() -> u32 {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetACP() -> u32;
    }
    // Pure system query; no environment-based approximation of Windows ACP.
    unsafe { GetACP() }
}

fn value(fields: &Fields, key: &str) -> String {
    fields
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.join(" \\\\ "))
        .unwrap_or_default()
}

fn default_columns(fields: &Fields, raw: &Fields) -> Vec<ScriptColumn> {
    let get = |key: &str| value(fields, key);
    let mut columns = vec![ScriptColumn {
        name: "Filename".into(),
        value: get("_FILENAME_EXT"),
    }];
    for (name, key) in [
        ("Title", "TITLE"),
        ("Artist", "ARTIST"),
        ("Album", "ALBUM"),
        ("Track", "TRACK"),
        ("Year", "YEAR"),
        ("Genre", "GENRE"),
    ] {
        let mut result = value(raw, key);
        if key == "TRACK" && result.is_empty() {
            result = value(raw, "TRACKNUMBER");
        }
        if key == "YEAR" && result.is_empty() {
            result = value(raw, "DATE");
        }
        columns.push(ScriptColumn {
            name: name.into(),
            value: result,
        });
    }
    columns.push(ScriptColumn {
        name: "Length".into(),
        value: get("_LENGTH"),
    });
    columns.push(ScriptColumn {
        name: "Format".into(),
        value: get("_FILETYPE"),
    });
    columns.push(ScriptColumn {
        name: "Bitrate".into(),
        value: format!("{} kbps", get("_BITRATE")),
    });
    columns
}

pub fn technical_fields(entry: &AudioEntry, index: usize) -> Fields {
    let path = Path::new(&entry.path);
    let mut fields = Fields::new();
    let parent = path.parent().unwrap_or(Path::new(""));
    for (key, value) in [
        (
            "_FILENAME",
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        ("_FILENAME_EXT", entry.filename.clone()),
        (
            "_EXTENSION",
            path.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        ("_PATH", entry.path.clone()),
        (
            "_FOLDERPATH",
            format!("{}{}", parent.display(), std::path::MAIN_SEPARATOR),
        ),
        (
            "_DIRECTORY",
            parent
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "_PARENT_DIRECTORY",
            parent
                .parent()
                .and_then(Path::file_name)
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        ("_BITRATE", entry.bitrate.to_string()),
        ("_SAMPLERATE", entry.sample_rate.to_string()),
        (
            "_LENGTH_SECONDS",
            (entry.duration.round() as u64).to_string(),
        ),
        (
            "_LENGTH",
            format!(
                "{}:{:02}",
                entry.duration as u64 / 60,
                entry.duration as u64 % 60
            ),
        ),
        ("_FILE_SIZE_BYTES", entry.size.to_string()),
        ("_FILE_SIZE_KB", (entry.size / 1024).to_string()),
        (
            "_FILE_SIZE_MB",
            format!("{:.2}", entry.size as f64 / (1024. * 1024.)),
        ),
        ("_FILE_SIZE", human_size(entry.size)),
        ("_COVERS", entry.covers.len().to_string()),
        ("_COUNTER", index.to_string()),
        ("_APP", "OpenTag v0.1.0".into()),
        // App table property; this is not advertised as an original placeholder.
        ("_FILETYPE", entry.format.clone()),
    ] {
        fields.insert(key.into(), vec![value]);
    }
    if let Some(cover) = entry.covers.first() {
        fields.insert("_COVER_DESCRIPTION".into(), vec![cover.description.clone()]);
        fields.insert("_COVER_MIMETYPE".into(), vec![cover.mime.clone()]);
        fields.insert("_COVER_TYPE".into(), vec![cover.picture_type.clone()]);
    }
    fields
}

pub fn enriched_fields(entry: &AudioEntry, index: usize) -> Fields {
    let mut fields = entry.fields.clone();
    fields.extend(technical_fields(entry, index));
    fields
}
fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024. * 1024.))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_table_values_and_raw_tag_list_are_distinct() {
        let raw = Fields::from([
            ("DATE".into(), vec!["2026-10-14".into()]),
            ("TRACKNUMBER".into(), vec!["01".into()]),
        ]);
        let technical = Fields::from([("_COUNTER".into(), vec!["2".into()])]);
        let settings = EvaluationSettings {
            columns: vec![ColumnSpec {
                name: "Track".into(),
                template: "%tracknumber%".into(),
                values: BTreeMap::from([("identity".into(), "01/12".into())]),
            }],
            app_directory: "/app".into(),
            app_data_directory: "/config".into(),
        };
        assert_eq!(
            settings
                .evaluate(
                    "$columnNames(,':')$columnValues(,';')",
                    &raw,
                    &technical,
                    "identity",
                    true
                )
                .unwrap(),
            "Track:01/12;"
        );
        let list = settings
            .evaluate("$list(,=,';')", &raw, &technical, "identity", false)
            .unwrap();
        assert!(list.contains("TRACKNUMBER=01;"));
        assert!(!list.contains("_COUNTER"));
        assert!(!list.contains("YEAR="));
        assert_eq!(
            settings
                .evaluate(
                    "$getenv(MP3TAGAPPDATA)",
                    &raw,
                    &technical,
                    "identity",
                    false
                )
                .unwrap(),
            "/config"
        );
    }
}
