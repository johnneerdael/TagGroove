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

#[derive(Clone)]
pub struct EvaluationSettings {
    pub columns: Vec<ColumnSpec>,
    pub app_directory: String,
    pub app_data_directory: String,
    pub timestamp_seconds: i64,
    pub export_write_bom: bool,
    pub export_create_directories: bool,
    pub export_template_inline: bool,
    /// Current working folder of the file list, for `%_workingdir%` and
    /// `%_workingpath%`.
    pub working_directory: Option<String>,
    /// Options → Files "Replace folder separator in tag values with".
    pub folder_separator: String,
}

impl Default for EvaluationSettings {
    fn default() -> Self {
        let timestamp_seconds =
            match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
                Err(error) => {
                    -i64::try_from(error.duration().as_secs()).unwrap_or(i64::MAX)
                        - i64::from(error.duration().subsec_nanos() > 0)
                }
            };
        Self {
            columns: Vec::new(),
            app_directory: String::new(),
            app_data_directory: String::new(),
            timestamp_seconds,
            export_write_bom: false,
            export_create_directories: true,
            export_template_inline: false,
            working_directory: None,
            folder_separator: tag_core::DEFAULT_FOLDER_SEPARATOR.into(),
        }
    }
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
            folder_separator: crate::settings::current().files.folder_separator,
            ..Default::default()
        })
    }

    pub(crate) fn context<'a>(
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
        context.timestamp_seconds = Some(self.timestamp_seconds);
        context.working_directory = self.working_directory.as_deref();
        context.folder_separator = Some(&self.folder_separator);
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
        self.evaluate_mode(template, raw, technical, identity, use_snapshot, false)
    }

    /// Evaluate a `_FILENAME`/`_DIRECTORY` target the way Tag → Filename does:
    /// separators in tag values are replaced, literal ones kept.
    pub fn evaluate_filename(
        &self,
        template: &str,
        raw: &Fields,
        technical: &Fields,
        identity: &str,
        use_snapshot: bool,
    ) -> Result<String, String> {
        self.evaluate_mode(template, raw, technical, identity, use_snapshot, true)
    }

    fn evaluate_mode(
        &self,
        template: &str,
        raw: &Fields,
        technical: &Fields,
        identity: &str,
        use_snapshot: bool,
        filename: bool,
    ) -> Result<String, String> {
        let mut fields = raw.clone();
        fields.extend(technical.clone());
        let columns = self.column_values(&fields, raw, identity, use_snapshot)?;
        let context = self.context(&fields, raw, Some(&columns));
        if filename {
            tag_core::evaluate_filename_with_context(template, &context)
        } else {
            tag_core::evaluate_with_context(template, &context)
        }
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
            tag_core::format_length(entry.duration.round() as u64),
        ),
        ("_FILE_SIZE_BYTES", entry.size.to_string()),
        ("_FILE_SIZE_KB", (entry.size / 1024).to_string()),
        (
            "_FILE_SIZE_MB",
            format!("{:.2}", entry.size as f64 / (1024. * 1024.)),
        ),
        ("_FILE_SIZE", human_size(entry.size)),
        ("_COVERS", entry.covers.len().to_string()),
        // A file without chapters is subsong 0 of 1; chapter rows and files
        // with chapters carry their own values.
        ("_SUBSONG", "0".to_string()),
        ("_SUBSONG_COUNT", "1".to_string()),
        ("_COUNTER", index.to_string()),
        ("_APP", app_name()),
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
    // Keep the derived values as a fallback for older serialized entries.
    // Actual file measurements take precedence, while selection, app, clock
    // and export context belong to their native owners rather than the file.
    for (name, values) in &entry.technical {
        let name = name.to_ascii_uppercase();
        if name.starts_with('_')
            && !matches!(
                name.as_str(),
                "_APP"
                    | "_COUNTER"
                    | "_FILETYPE"
                    | "_DATE"
                    | "_DATETIME"
                    | "_WORKINGDIR"
                    | "_WORKINGPATH"
                    | "_FILENAME_REL"
                    | "_FOLDERPATH_REL"
                    | "_CRC"
                    | "_MD5"
                    | "_MD5AUDIO"
                    | "_MAX_COUNTER"
                    | "_TOTAL_FILES"
                    | "_TOTAL_SIZE"
                    | "_TOTAL_SIZE_RAW"
                    | "_TOTAL_TIME"
                    | "_TOTAL_TIME2"
                    | "_TOTAL_TIME_RAW"
                    | "_PLAYLIST_FILENAME"
                    | "_PLAYLIST_FILENAME_EXT"
                    | "_PLAYLIST_FOLDERPATH"
            )
        {
            fields.insert(name, values.clone());
        }
    }
    fields
}

/// `%_app%`: the application name and version.
pub fn app_name() -> String {
    format!("OpenTag v{}", env!("CARGO_PKG_VERSION"))
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
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::{
        fs,
        time::{Duration, UNIX_EPOCH},
    };

    #[test]
    fn operation_contexts_capture_one_clock_for_columns_scripts_and_filters() {
        let raw = Fields::from([("TITLE".into(), vec!["Current".into()])]);
        let settings = EvaluationSettings::default();
        let first = settings.context(&raw, &raw, None);
        let second = settings.context(&raw, &raw, None);
        assert!(first.timestamp_seconds.is_some());
        assert_eq!(first.timestamp_seconds, second.timestamp_seconds);
        let date = tag_core::evaluate_with_context("%_date%", &first).unwrap();
        let datetime = tag_core::evaluate_with_context("%_datetime%", &first).unwrap();
        assert!(!date.is_empty());
        assert!(datetime.starts_with(&date));
        assert_eq!(
            settings
                .evaluate("%_date%|%_datetime%", &raw, &Fields::new(), "first", false)
                .unwrap(),
            format!("{date}|{datetime}")
        );
        let filter = tag_core::compile_filter(&format!("_DATE IS \"{date}\"")).unwrap();
        assert!(settings.matches(&filter, &raw, &raw, "second").unwrap());
        let columns = EvaluationSettings {
            columns: vec![ColumnSpec {
                name: "Clock".into(),
                template: "%_datetime%".into(),
                values: BTreeMap::new(),
            }],
            ..settings
        };
        assert_eq!(
            columns
                .evaluate("$columnValues(,)", &raw, &Fields::new(), "third", false)
                .unwrap(),
            datetime
        );
    }

    fn audio_with_cover(directory: &Path) -> AudioEntry {
        let path = directory.join("measured.wav");
        let mut bytes = b"RIFF".to_vec();
        bytes.extend(16036u32.to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(vec![0u8; 16000]);
        fs::write(&path, bytes).unwrap();
        let updates = Fields::from([
            ("TITLE".into(), vec!["Raw title".into()]),
            ("INVENTORY".into(), vec!["first".into(), "second".into()]),
        ]);
        tag_core::write_fields_summary(&path, &updates).unwrap();
        let image = directory.join("cover.png");
        fs::write(&image, STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap()).unwrap();
        tag_core::add_cover_summary(&path, &image).unwrap();
        // Windows requires write-attributes access when setting file times.
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(
                fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            )
            .unwrap();
        tag_core::read_entry_summary(&path).unwrap()
    }

    #[test]
    fn measured_properties_reach_native_scripting_columns_and_filenames_without_polluting_tags() {
        let directory = tempfile::tempdir().unwrap();
        let entry = audio_with_cover(directory.path());
        let original = entry.fields.clone();
        let technical = technical_fields(&entry, 7);
        let settings = EvaluationSettings {
            columns: vec![ColumnSpec {
                name: "Audio".into(),
                template: "%_codec%/%_bitspersample%".into(),
                values: BTreeMap::new(),
            }],
            ..Default::default()
        };
        assert_eq!(settings.evaluate(
            "%_codec%|%_bitspersample%|%_cover_width%|%_cover_type%|%_file_mod_datetime_raw%|%_counter%|$columnValues(,';')",
            &entry.fields, &technical, &entry.path, false,
        ).unwrap(), "PCM|16|1|Front Cover|1700000000|7|PCM/16;");
        assert_eq!(
            settings
                .filename("%_codec%-%_bitspersample%-%_cover_width%", &entry, 7)
                .unwrap(),
            "PCM-16-1"
        );
        let listed = settings
            .evaluate(
                "$list(,=,';','~')",
                &entry.fields,
                &technical,
                &entry.path,
                false,
            )
            .unwrap();
        assert!(listed.contains("TITLE=Raw title;"));
        assert!(listed.contains("INVENTORY=first~second;"));
        for key in ["_CODEC", "_BITSPERSAMPLE", "_COUNTER", "_COVER_WIDTH"] {
            assert!(!listed.contains(key), "{listed}");
        }
        assert_eq!(entry.fields, original);
        assert!(entry.covers[0].data_base64.is_empty());
    }

    #[test]
    fn measured_context_overrides_legacy_approximations_but_preserves_reserved_application_values()
    {
        let directory = tempfile::tempdir().unwrap();
        let mut entry = audio_with_cover(directory.path());
        let actual_size = fs::metadata(&entry.path).unwrap().len();
        entry.bitrate = 999;
        entry.sample_rate = 3;
        entry.duration = 99.;
        entry.size = 4;
        for (key, value) in [
            ("_counter", "999"),
            ("_App", "Spoofed app"),
            ("_FILETYPE", "Wrong container"),
            ("_DATE", "Stale date"),
            ("TITLE", "Wrong title"),
        ] {
            entry.technical.insert(key.into(), vec![value.into()]);
        }
        let context = enriched_fields(&entry, 3);
        assert_eq!(context["_BITRATE"], ["128"]);
        assert_eq!(context["_SAMPLERATE"], ["8000"]);
        assert_eq!(context["_LENGTH_SECONDS"], ["1"]);
        assert_eq!(context["_FILE_SIZE_BYTES"], [actual_size.to_string()]);
        assert_eq!(context["_COVER_TYPE"], ["Front Cover"]);
        assert_eq!(context["_COUNTER"], ["3"]);
        assert_eq!(
            context["_APP"],
            [format!("OpenTag v{}", env!("CARGO_PKG_VERSION"))]
        );
        assert_eq!(context["_FILETYPE"], ["WAV"]);
        assert_eq!(context["TITLE"], ["Raw title"]);
        assert!(!context.contains_key("_DATE"));
        assert!(!context.contains_key("_counter"));
        assert!(!context.contains_key("_App"));
    }

    #[test]
    fn native_actions_and_exports_receive_refreshed_measured_context_after_cover_removal() {
        use tag_core::actions::{ActionGroup, ActionSpec};
        let directory = tempfile::tempdir().unwrap();
        let entry = audio_with_cover(directory.path());
        let group = ActionGroup {
            name: "Measured context".into(),
            enabled: true,
            actions: vec![
                ActionSpec::FormatValue {
                    field: "COMMENT".into(),
                    template: "%_CODEC%|%_BITSPERSAMPLE%|%_COVER_WIDTH%|%_COVER_TYPE%|%_COUNTER%".into(),
                },
                ActionSpec::RemoveFields { fields: vec!["COVER".into()] },
                ActionSpec::FormatValue {
                    field: "AFTER".into(),
                    template: "%_CODEC%|%_BITSPERSAMPLE%|%_COVERS%|[%_COVER_WIDTH%]".into(),
                },
                ActionSpec::Export {
                    template: "$filename(txt,utf-8)$loop(%_filename%)%COMMENT%/%AFTER%/%_CODEC%/%_BITSPERSAMPLE%/%_COVERS%/$if(%_COVER_WIDTH%,present,absent)/%_COUNTER%$loopend()".into(),
                    filename_template: "technical.txt".into(),
                    append: false,
                    partition_template: None,
                },
            ],
        };
        let result = crate::action_host::execute(
            std::slice::from_ref(&entry.path),
            &[group],
            EvaluationSettings::default(),
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.entries.len(), 1);
        let saved = tag_core::read_entry_summary(Path::new(&entry.path)).unwrap();
        assert_eq!(saved.fields["COMMENT"], ["PCM|16|1|Front Cover|1"]);
        assert_eq!(saved.fields["AFTER"], ["PCM|16|0|"]);
        assert_eq!(saved.fields["INVENTORY"], ["first", "second"]);
        assert!(saved.covers.is_empty());
        assert!(!saved.fields.keys().any(|key| key.starts_with('_')));
        assert_eq!(
            fs::read_to_string(directory.path().join("technical.txt")).unwrap(),
            "PCM|16|1|Front Cover|1/PCM|16|0|/PCM/16/0/absent/1"
        );
        assert!(fs::read(&entry.path).unwrap()[44..16044]
            .iter()
            .all(|byte| *byte == 0));
    }

    #[test]
    fn exports_receive_the_working_folder_in_body_and_file_name() {
        use tag_core::actions::{ActionGroup, ActionSpec};
        let directory = tempfile::tempdir().unwrap();
        let working = directory.path().join("Working");
        fs::create_dir(&working).unwrap();
        let entry = audio_with_cover(&working);
        let group = ActionGroup {
            name: "Working folder".into(),
            enabled: true,
            actions: vec![ActionSpec::Export {
                template: "$filename(txt,utf-8)$loop(1)%_workingdir%|%_filename_ext%$loopend()"
                    .into(),
                filename_template: "%_workingpath%%_workingdir%-list.txt".into(),
                append: false,
                partition_template: None,
            }],
        };
        let settings = EvaluationSettings {
            working_directory: Some(working.to_string_lossy().into_owned()),
            ..Default::default()
        };
        let result =
            crate::action_host::execute(std::slice::from_ref(&entry.path), &[group], settings);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            fs::read_to_string(working.join("Working-list.txt")).unwrap(),
            "Working|measured.wav"
        );
    }

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
            ..Default::default()
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
