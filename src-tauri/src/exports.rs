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
    /// configuration). Configurations made in TagGroove use `<id>.mte`.
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
    /// The newest set of bundled configurations this library has been offered.
    /// Libraries saved before a set was added receive its configurations once;
    /// bundled configurations removed afterwards stay removed.
    #[serde(default = "first_bundled_set")]
    pub bundled: u32,
    /// Files seen by the caller when the library was loaded. Carry this through
    /// dialog snapshots, but never persist it in exports.json. Along with saved
    /// metadata, this identifies files eligible for explicit removal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_files: Option<Vec<String>>,
    /// Internal durable removal intent. A failed retirement can be retried even
    /// after metadata no longer lists the configuration. Never send it to callers.
    #[serde(default, skip_serializing)]
    pending_removals: Vec<String>,
}
/// Libraries saved before sets were recorded were offered the first set.
fn first_bundled_set() -> u32 {
    1
}
/// Built-in cue sheet export: one FILE and TRACK per file in File List order,
/// which File → Load playlist/cue sheet reads back.
pub const CUE_SHEET_TEMPLATE: &str = "$filename(cue,utf-8)TITLE \"%album%\"\r\nPERFORMER \"%albumartist%\"\r\n$loop(1)FILE \"%_filename_rel%\" $if($eql($lower(%_extension%),mp3),MP3,WAVE)\r\n  TRACK $num(%_counter%,2) AUDIO\r\n    TITLE \"%title%\"\r\n    PERFORMER \"%artist%\"\r\n    INDEX 01 00:00:00\r\n$loopend()";

/// Comma-separated values with a header row; every value is quoted and
/// embedded quotes are doubled, so spreadsheet programs read any text back.
pub const CSV_TEMPLATE: &str = concat!(
    "$filename(csv,utf-8)",
    "\"Path\",\"Title\",\"Artist\",\"Album\",\"Album artist\",\"Track\",\"Disc\",\"Year\",\"Genre\",\"Length\"$char(13)$char(10)",
    "$loop(1)",
    "\"$replace(%_path%,\",\"\")\",\"$replace(%title%,\",\"\")\",\"$replace(%artist%,\",\"\")\",",
    "\"$replace(%album%,\",\"\")\",\"$replace(%albumartist%,\",\"\")\",\"$replace(%track%,\",\"\")\",",
    "\"$replace(%discnumber%,\",\"\")\",\"$replace(%year%,\",\"\")\",\"$replace(%genre%,\",\"\")\",\"%_length%\"$char(13)$char(10)",
    "$loopend()",
);

/// A standalone HTML page with one table row per file and a summary line.
/// Text is escaped for HTML; the page has no scripts or external resources.
pub const HTML_TABLE_TEMPLATE: &str = concat!(
    "$filename(html,utf-8)",
    "<!DOCTYPE html>$char(10)<html>$char(10)<head>$char(10)<meta charset=\"utf-8\">$char(10)",
    "<title>Track list</title>$char(10)",
    "<style>body{font-family:sans-serif;margin:2em}table{border-collapse:collapse}",
    "th,td{border:1px solid #bbb;padding:.3em .6em;text-align:left;vertical-align:top}",
    "th{background:#eee}td.n{text-align:right}</style>$char(10)</head>$char(10)<body>$char(10)",
    "<table>$char(10)<thead><tr><th>#</th><th>Title</th><th>Artist</th><th>Album</th><th>Year</th><th>Length</th><th>File</th></tr></thead>$char(10)<tbody>$char(10)",
    "$loop(1)<tr><td class=\"n\">%_counter%</td>",
    "<td>$replace(%title%,&,&amp;,<,&lt;,>,&gt;)</td>",
    "<td>$replace(%artist%,&,&amp;,<,&lt;,>,&gt;)</td>",
    "<td>$replace(%album%,&,&amp;,<,&lt;,>,&gt;)</td>",
    "<td>$replace(%year%,&,&amp;,<,&lt;,>,&gt;)</td>",
    "<td class=\"n\">%_length%</td>",
    "<td>$replace(%_filename_ext%,&,&amp;,<,&lt;,>,&gt;)</td></tr>$char(10)$loopend()",
    "</tbody>$char(10)</table>$char(10)<p>%_total_files% files, %_total_time%, %_total_size%</p>$char(10)</body>$char(10)</html>$char(10)",
);

/// An extended M3U playlist (UTF-8) with durations and display titles,
/// entries relative to the export file.
pub const M3U_EXTENDED_TEMPLATE: &str = concat!(
    "$filename(m3u8,utf-8)",
    "#EXTM3U$char(13)$char(10)",
    "$loop(1)#EXTINF:%_length_seconds%,$if2(%artist%,Unknown artist) - $if2(%title%,%_filename%)$char(13)$char(10)",
    "%_filename_rel%$char(13)$char(10)$loopend()",
);

/// A JSON array with one object per file. Backslashes, quotes and line
/// breaks are escaped; brackets are quoted because they are format-string syntax.
pub const JSON_TEMPLATE: &str = concat!(
    "$filename(json,utf-8)",
    "'['$char(10)",
    "$loop(1)$if($grtr(%_counter%,1),$char(44)$char(10),)",
    "  {\"path\": \"$replace(%_path%,\\,\\\\,\",\\\",$char(13),\\r,$char(10),\\n,$char(9),\\t)\", ",
    "\"title\": \"$replace(%title%,\\,\\\\,\",\\\",$char(13),\\r,$char(10),\\n,$char(9),\\t)\", ",
    "\"artist\": \"$replace(%artist%,\\,\\\\,\",\\\",$char(13),\\r,$char(10),\\n,$char(9),\\t)\", ",
    "\"album\": \"$replace(%album%,\\,\\\\,\",\\\",$char(13),\\r,$char(10),\\n,$char(9),\\t)\", ",
    "\"track\": \"$replace(%track%,\\,\\\\,\",\\\",$char(13),\\r,$char(10),\\n,$char(9),\\t)\", ",
    "\"year\": \"$replace(%year%,\\,\\\\,\",\\\",$char(13),\\r,$char(10),\\n,$char(9),\\t)\", ",
    "\"seconds\": %_length_seconds%}",
    "$loopend()$char(10)']'$char(10)",
);

/// One line per album: each album artist and album once, with its year.
pub const ALBUM_LIST_TEMPLATE: &str = concat!(
    "$filename(txt,utf-8)",
    "$loop(%albumartist%%album%,1)$if2(%albumartist%,%artist%) - %album%[ (%year%)]$char(13)$char(10)$loopend()",
);

/// The bundled configurations with the set (`Library::bundled`) that
/// introduced them. All templates are written for TagGroove.
fn bundled() -> [(u32, &'static str, &'static str, &'static str); 7] {
    [
        (
            1,
            "text",
            "Text",
            "$filename(txt,utf-8)$loop(1)%artist% - %title%$char(13)$char(10)$loopend()",
        ),
        (1, "cue-sheet", "Cue sheet", CUE_SHEET_TEMPLATE),
        (2, "csv", "CSV (spreadsheet)", CSV_TEMPLATE),
        (2, "html-table", "HTML table", HTML_TABLE_TEMPLATE),
        (
            2,
            "m3u-extended",
            "Extended M3U playlist",
            M3U_EXTENDED_TEMPLATE,
        ),
        (2, "json", "JSON", JSON_TEMPLATE),
        (2, "album-list", "Album list", ALBUM_LIST_TEMPLATE),
    ]
}
/// The newest bundled set.
pub const BUNDLED: u32 = 2;

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            configurations: bundled()
                .into_iter()
                .map(|(_, id, name, template)| Configuration {
                    id: id.into(),
                    name: name.into(),
                    template: template.into(),
                    file: None,
                })
                .collect(),
            preferences: Preferences::default(),
            bundled: BUNDLED,
            observed_files: None,
            pending_removals: Vec::new(),
        }
    }
}

/// Offer the bundled configurations added since the library was saved. Names
/// or identities the library already uses are left alone.
fn add_bundled(library: &mut Library) {
    if library.bundled >= BUNDLED {
        return;
    }
    for (set, id, name, template) in bundled() {
        let taken = library.configurations.iter().any(|c| {
            c.id.eq_ignore_ascii_case(id)
                || c.name.to_lowercase() == name.to_lowercase()
                || c.file_name().eq_ignore_ascii_case(&format!("{id}.mte"))
        });
        if set > library.bundled && !taken {
            library.configurations.push(Configuration {
                id: id.into(),
                name: name.into(),
                template: template.into(),
                file: None,
            });
        }
    }
    library.bundled = BUNDLED;
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
/// Returns the file name and partition used instead of the saved ones. Names
/// that already depend on %_folderpath% are kept. A plain name, or a name in
/// a relative folder, is placed in each folder. A name in an absolute folder
/// (or below %_workingpath%) keeps that output folder and mirrors each directory
/// below it, relative to the folder all exported files share.
pub fn per_directory(filename: &str, template: &str, paths: &[String]) -> (String, String) {
    let partition = "%_folderpath%".to_string();
    let filename = filename.trim();
    if filename.to_ascii_lowercase().contains("%_folderpath%") {
        return (filename.into(), partition);
    }
    let split = |name: &str| match name.rfind(['/', '\\']) {
        Some(index) => (name[..=index].to_string(), name[index + 1..].to_string()),
        None => (String::new(), name.to_string()),
    };
    let (folder, name) = split(filename);
    let name = if !name.is_empty() {
        name
    } else {
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
            split(argument).1
        } else {
            format!("export.{argument}")
        }
    };
    let absolute = folder.starts_with(['/', '\\'])
        || folder.as_bytes().get(1) == Some(&b':')
        || folder.to_ascii_lowercase().starts_with("%_workingpath%");
    if folder.is_empty() || !absolute {
        return (format!("%_folderpath%{folder}{name}"), partition);
    }
    let common = common_folder(paths);
    let below = if common.is_empty() {
        // Nothing shared (different drives): one folder per directory name.
        "%_directory%/".to_string()
    } else {
        format!("$cutLeft(%_folderpath%,{})", common.chars().count())
    };
    (format!("{folder}{below}{name}"), partition)
}

/// The longest folder (with its trailing separator, as `%_folderpath%` writes
/// it) that contains every path.
fn common_folder(paths: &[String]) -> String {
    let folder = |path: &str| {
        path.rfind(['/', '\\'])
            .map(|index| path[..=index].to_string())
            .unwrap_or_default()
    };
    let mut iter = paths.iter().map(|path| folder(path));
    let Some(mut common) = iter.next() else {
        return String::new();
    };
    for next in iter {
        let shared = common
            .char_indices()
            .zip(next.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((index, c), _)| index + c.len_utf8());
        common.truncate(shared);
        // Keep whole folder names only.
        let end = common.rfind(['/', '\\']).map_or(0, |index| index + 1);
        common.truncate(end);
    }
    common
}

fn validate(library: &Library, templates: bool) -> Result<(), String> {
    if library.version != 1 {
        return Err("Unsupported export library version".into());
    }
    for observed in [
        library.observed_files.as_deref().unwrap_or_default(),
        library.pending_removals.as_slice(),
    ] {
        let mut files = HashSet::new();
        if observed
            .iter()
            .any(|file| !valid_file_name(file) || !files.insert(file.to_lowercase()))
        {
            return Err("Export configurations require distinct .mte file names".into());
        }
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
        add_bundled(&mut library);
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
    library.observed_files = Some(
        library
            .configurations
            .iter()
            .map(Configuration::file_name)
            .collect(),
    );
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
    // Persisted identities include templates created by an earlier save in the
    // same dialog. Never discover here: unseen files are not implied removals.
    let mut previous: Vec<String> = match fs::read(path) {
        Ok(bytes) => {
            let saved: Library = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Cannot read export configurations: {e}"))?;
            validate(&saved, false)?;
            let mut files: Vec<_> = saved
                .configurations
                .iter()
                .map(Configuration::file_name)
                .collect();
            files.extend(saved.pending_removals);
            files
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.to_string()),
    };
    // Also permit explicit removal of discovered files not yet saved in JSON.
    if let Some(observed) = &library.observed_files {
        for file in observed {
            if !previous
                .iter()
                .any(|saved| saved.eq_ignore_ascii_case(file))
            {
                previous.push(file.clone());
            }
        }
    }
    let directory = path.parent().unwrap_or(Path::new(".")).join("export");
    let mut distinct = HashSet::new();
    let retiring: Vec<_> = previous
        .iter()
        .filter(|old| {
            distinct.insert(old.to_lowercase())
                && !library
                    .configurations
                    .iter()
                    .any(|current| current.file_name().eq_ignore_ascii_case(old))
                && directory.join(old).exists()
        })
        .collect();
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
        bundled: u32,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pending_removals: Vec<&'a str>,
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
        bundled: library.bundled.max(BUNDLED),
        // Keep intent in the same metadata commit that removes configurations.
        // A later save prunes files already retired; no second commit is needed.
        pending_removals: retiring.iter().map(|file| file.as_str()).collect(),
    };
    crate::action_host::atomic_output(
        path,
        &serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?,
        false,
    )?;
    for removed in retiring {
        let source = directory.join(removed);
        // Bundled configurations offered since the last save have no file yet.
        if !source.exists() {
            continue;
        }
        let archive = directory.join("removed");
        let stem = Path::new(removed)
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
                "Export library saved, but could not retire {path}: {value}",
                path = source.display(),
                value = e
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
    fn stale_library_save_keeps_new_mte_files_in_export_folder() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        // The dialog keeps this JSON-roundtripped snapshot while a new file
        // arrives externally. Saving preferences must not imply its removal.
        let mut cached: Library =
            serde_json::from_value(serde_json::to_value(load(&path).unwrap()).unwrap()).unwrap();
        cached.preferences.append = true;
        let folder = directory.path().join("export");
        let dropped = folder.join("Late.MTE");
        let bytes = b"Bj\xf6rk";
        fs::write(&dropped, bytes).unwrap();
        for _ in 0..2 {
            save(&path, &cached).unwrap();
            assert_eq!(fs::read(&dropped).unwrap(), bytes);
            let reopened = load(&path).unwrap();
            assert!(reopened.preferences.append);
            assert_eq!(
                reopened
                    .configurations
                    .iter()
                    .find(|c| c.file.as_deref() == Some("Late.MTE"))
                    .unwrap()
                    .template,
                "Björk"
            );
            assert!(!folder.join("removed").exists());
        }
    }

    #[test]
    fn repeated_dialog_saves_retire_newly_saved_templates_but_keep_unseen_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let mut cached = load(&path).unwrap();
        let folder = directory.path().join("export");
        // Neither the backend command nor Save configurations reloads the
        // dialog's snapshot after success, and Export can fail after saving.
        cached.configurations.push(Configuration {
            id: "created".into(),
            name: "Created in dialog".into(),
            template: "created template".into(),
            file: None,
        });
        fs::write(folder.join("Unseen.mte"), b"Bj\xf6rk").unwrap();
        save(&path, &cached).unwrap();
        assert_eq!(
            fs::read_to_string(folder.join("created.mte")).unwrap(),
            "created template"
        );
        cached.configurations.retain(|c| c.id != "created");
        for _ in 0..2 {
            save(&path, &cached).unwrap();
            assert!(!folder.join("created.mte").exists());
            assert_eq!(fs::read(folder.join("Unseen.mte")).unwrap(), b"Bj\xf6rk");
            let metadata: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert!(metadata.get("observed_files").is_none());
            assert!(metadata["configurations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["file"] != "Unseen.mte"));
        }
        let retired: Vec<_> = fs::read_dir(folder.join("removed"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(retired.len(), 1);
        assert_eq!(fs::read_to_string(&retired[0]).unwrap(), "created template");
        let reopened = load(&path).unwrap();
        assert!(reopened
            .configurations
            .iter()
            .any(|c| c.file.as_deref() == Some("Unseen.mte")));
        assert!(reopened.configurations.iter().all(|c| c.id != "created"));
    }

    #[test]
    fn failed_retirement_can_retry_same_dialog_payload() {
        check_failed_retirement_retry(false);
    }

    #[test]
    fn failed_retirement_can_retry_legacy_payload() {
        check_failed_retirement_retry(true);
    }

    fn check_failed_retirement_retry(legacy: bool) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let mut cached = load(&path).unwrap();
        if legacy {
            cached.observed_files = None;
        }
        cached.configurations.push(Configuration {
            id: "created".into(),
            name: "Created".into(),
            template: "created template".into(),
            file: None,
        });
        save(&path, &cached).unwrap();
        let folder = directory.path().join("export");
        fs::write(folder.join("Unseen.mte"), b"Bj\xf6rk").unwrap();
        cached.configurations.retain(|c| c.id != "created");
        // A real filesystem obstruction makes retirement fail after the
        // metadata commits, without mocking persistence or its error.
        fs::write(folder.join("removed"), "owned obstruction").unwrap();
        let error = save(&path, &cached).unwrap_err();
        assert!(
            error.starts_with("Export library saved, but could not retire"),
            "{error}"
        );
        assert_eq!(
            fs::read_to_string(folder.join("created.mte")).unwrap(),
            "created template"
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(metadata["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != "created"));
        assert!(metadata.get("observed_files").is_none());
        fs::remove_file(folder.join("removed")).unwrap();
        for _ in 0..2 {
            save(&path, &cached).unwrap();
            assert!(!folder.join("created.mte").exists(), "legacy={legacy}");
            assert_eq!(fs::read(folder.join("Unseen.mte")).unwrap(), b"Bj\xf6rk");
        }
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(metadata.get("pending_removals").is_none());
        let retired: Vec<_> = fs::read_dir(folder.join("removed"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(retired.len(), 1);
        assert_eq!(fs::read_to_string(&retired[0]).unwrap(), "created template");
        let reopened = load(&path).unwrap();
        assert!(reopened.configurations.iter().all(|c| c.id != "created"));
        assert!(reopened
            .configurations
            .iter()
            .any(|c| c.file.as_deref() == Some("Unseen.mte")));
        // Recovery copies become ordinary listed files again.
        fs::copy(&retired[0], folder.join("created.mte")).unwrap();
        let recovered = load(&path).unwrap();
        save(&path, &recovered).unwrap();
        assert_eq!(
            fs::read_to_string(folder.join("created.mte")).unwrap(),
            "created template"
        );
    }

    #[test]
    fn pending_removal_metadata_rejects_unsafe_or_duplicate_names_before_writing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let library = load(&path).unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for files in [
            vec!["../outside.mte"],
            vec!["text.mte", "TEXT.MTE"],
            vec!["notes.txt"],
        ] {
            let mut metadata = original.clone();
            metadata["pending_removals"] = serde_json::json!(files);
            let bytes = serde_json::to_vec(&metadata).unwrap();
            fs::write(&path, &bytes).unwrap();
            assert!(load(&path).is_err());
            assert!(save(&path, &library).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(
                fs::read_to_string(directory.path().join("export/text.mte")).unwrap(),
                Library::default().configurations[0].template
            );
        }
    }

    #[test]
    fn recovered_copy_keeps_its_file_despite_pending_retirement_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let mut library = load(&path).unwrap();
        library.configurations.retain(|c| c.id != "text");
        library.preferences.configuration = Some("cue-sheet".into());
        save(&path, &library).unwrap();
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            metadata["pending_removals"],
            serde_json::json!(["text.mte"])
        );
        // The internal retry intent must not become a caller-controlled baseline.
        assert!(serde_json::to_value(load(&path).unwrap())
            .unwrap()
            .get("pending_removals")
            .is_none());
        let folder = directory.path().join("export");
        let retired = fs::read_dir(folder.join("removed"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        fs::copy(&retired, folder.join("text.mte")).unwrap();
        let recovered = load(&path).unwrap();
        assert!(recovered
            .configurations
            .iter()
            .any(|c| c.file.as_deref() == Some("text.mte")));
        save(&path, &recovered).unwrap();
        assert_eq!(
            fs::read(folder.join("text.mte")).unwrap(),
            fs::read(retired).unwrap()
        );
        let metadata: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert!(metadata.get("pending_removals").is_none());
    }

    #[test]
    fn removing_observed_unsaved_mte_retires_its_original_bytes() {
        for saved_before in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("exports.json");
            if saved_before {
                save(&path, &Library::default()).unwrap();
            }
            let folder = directory.path().join("export");
            fs::create_dir_all(&folder).unwrap();
            let bytes = b"Bj\xf6rk";
            fs::write(folder.join("Observed.MTE"), bytes).unwrap();
            let mut library = load(&path).unwrap();
            let observed = library
                .configurations
                .iter()
                .find(|c| c.file.as_deref() == Some("Observed.MTE"))
                .unwrap()
                .id
                .clone();
            library.configurations.retain(|c| c.id != observed);
            save(&path, &library).unwrap();
            assert!(!folder.join("Observed.MTE").exists());
            let retired: Vec<_> = fs::read_dir(folder.join("removed"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            assert_eq!(retired.len(), 1);
            assert_eq!(fs::read(&retired[0]).unwrap(), bytes);
            assert!(load(&path)
                .unwrap()
                .configurations
                .iter()
                .all(|c| c.id != observed));
            // Returning a recovery copy to the active folder discovers it again.
            fs::copy(&retired[0], folder.join("Observed.MTE")).unwrap();
            assert!(load(&path)
                .unwrap()
                .configurations
                .iter()
                .any(|c| c.id == observed));
        }
    }

    #[test]
    fn library_without_observation_baseline_only_retires_saved_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        let mut library = Library::default();
        save(&path, &library).unwrap();
        let folder = directory.path().join("export");
        fs::write(folder.join("Unseen.mte"), "unseen").unwrap();
        library.configurations.retain(|c| c.id != "text");
        library.preferences.configuration = Some("cue-sheet".into());
        save(&path, &library).unwrap();
        assert!(!folder.join("text.mte").exists());
        assert_eq!(
            fs::read_to_string(folder.join("Unseen.mte")).unwrap(),
            "unseen"
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(metadata.get("observed_files").is_none());
        assert!(metadata["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c.get("template").is_none()));
    }

    #[test]
    fn observation_baseline_rejects_unsafe_or_duplicate_names_before_writing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let before = fs::read(&path).unwrap();
        for files in [
            vec!["../outside.mte"],
            vec!["text.mte", "TEXT.MTE"],
            vec!["notes.txt"],
        ] {
            let mut library = load(&path).unwrap();
            library.configurations[0].template = "changed".into();
            library.observed_files = Some(files.into_iter().map(str::to_owned).collect());
            assert!(save(&path, &library).is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
            assert_eq!(
                fs::read_to_string(directory.path().join("export/text.mte")).unwrap(),
                Library::default().configurations[0].template
            );
        }
    }

    #[test]
    fn restored_metadata_keeps_external_mte_discovery_and_explicit_removal() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        save(&path, &Library::default()).unwrap();
        let bundle = serde_json::json!({
            "format": crate::utilities::BUNDLE_FORMAT,
            "version": 1,
            "files": { "exports.json": {
                "version": 1, "configurations": [], "preferences": { "configuration": null }, "bundled": BUNDLED
            }}
        });
        assert_eq!(
            crate::utilities::restore(directory.path(), &serde_json::to_vec(&bundle).unwrap())
                .unwrap(),
            ["exports.json"]
        );
        // Configuration restore replaces metadata, retaining physical MTE files.
        let mut library = load(&path).unwrap();
        assert_eq!(
            library.configurations.len(),
            Library::default().configurations.len()
        );
        library
            .configurations
            .retain(|c| c.file.as_deref() != Some("text.mte"));
        save(&path, &library).unwrap();
        assert!(!directory.path().join("export/text.mte").exists());
        assert!(load(&path)
            .unwrap()
            .configurations
            .iter()
            .all(|c| c.file.as_deref() != Some("text.mte")));
        let metadata: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert!(metadata.get("observed_files").is_none());
    }

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
        assert_eq!(retired.len(), Library::default().configurations.len());
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
        // As in Mp3tag, a cue sheet lists its tracks, each playing its FILE.
        let cue = document.filename.clone();
        assert_eq!(
            loaded.files,
            [
                std::path::PathBuf::from(tag_core::subsong_path(&cue, 1)),
                std::path::PathBuf::from(tag_core::subsong_path(&cue, 2))
            ]
        );
        assert_eq!(
            dunce::canonicalize(tag_core::subsong_file(&loaded.files[1]).unwrap()).unwrap(),
            dunce::canonicalize(root.join("disc").join("a.flac")).unwrap()
        );
    }

    #[test]
    fn per_directory_exports_place_one_file_in_each_folder() {
        let folder = |name: &str| (format!("%_folderpath%{name}"), "%_folderpath%".to_string());
        let none: &[String] = &[];
        assert_eq!(
            per_directory("", "$filename(csv,utf-8)$loop(1)$loopend()", none),
            folder("export.csv")
        );
        assert_eq!(
            per_directory("", "$filename(%album%.html)x", none),
            folder("%album%.html")
        );
        assert_eq!(
            per_directory("", "$filename(out/list.txt)x", none),
            folder("list.txt")
        );
        assert_eq!(per_directory("", "plain", none), folder("export.txt"));
        assert_eq!(
            per_directory("%_folderpath%sub\\z.txt", "", none),
            ("%_folderpath%sub\\z.txt".into(), "%_folderpath%".into())
        );
        // A relative folder is created inside each folder.
        assert_eq!(
            per_directory("lists/x.txt", "", none),
            folder("lists/x.txt")
        );
        assert_eq!(
            per_directory("%album%/x.txt", "", none),
            folder("%album%/x.txt")
        );
    }

    #[test]
    fn per_directory_exports_keep_an_absolute_output_folder() {
        let paths = [
            "/music/Artist/Album/CD1/01.mp3".to_string(),
            "/music/Artist/Album/CD1/02.mp3".to_string(),
            "/music/Artist/Album/CD2/01.mp3".to_string(),
            "/music/Other/Album/CD1/01.mp3".to_string(),
        ];
        assert_eq!(common_folder(&paths), "/music/");
        assert_eq!(common_folder(&paths[..2]), "/music/Artist/Album/CD1/");
        assert_eq!(
            common_folder(&["/music/abc/1.mp3".into(), "/music/abd/2.mp3".into()]),
            "/music/"
        );
        assert_eq!(
            common_folder(&["C:\\a\\1.mp3".into(), "D:\\a\\2.mp3".into()]),
            ""
        );
        let (filename, partition) = per_directory("/out/list.txt", "", &paths);
        assert_eq!(filename, "/out/$cutLeft(%_folderpath%,7)list.txt");
        assert_eq!(partition, "%_folderpath%");
        assert_eq!(
            per_directory(
                "C:\\out\\y.txt",
                "",
                &["C:\\m\\a\\1.mp3".into(), "C:\\m\\b\\2.mp3".into()]
            )
            .0,
            "C:\\out\\$cutLeft(%_folderpath%,5)y.txt"
        );
        assert_eq!(
            per_directory("%_workingpath%exports/", "$filename(html)x", &paths[..1]).0,
            "%_workingpath%exports/$cutLeft(%_folderpath%,24)export.html"
        );
        assert_eq!(
            per_directory(
                "C:\\out\\y.txt",
                "",
                &["C:\\a\\1.mp3".into(), "D:\\a\\2.mp3".into()]
            )
            .0,
            "C:\\out\\%_directory%/y.txt"
        );

        // Rendered, every folder gets its own file below the output folder.
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_string_lossy().replace('\\', "/");
        let paths: Vec<String> = ["a/CD1/1.mp3", "a/CD1/2.mp3", "b/CD1/1.mp3"]
            .iter()
            .map(|name| format!("{root}/music/{name}"))
            .collect();
        let (filename, partition) = per_directory(&format!("{root}/out/list.txt"), "", &paths);
        let rows: Vec<_> = paths
            .iter()
            .map(|path| {
                let fields: tag_core::Fields =
                    [("TITLE".to_string(), vec!["t".to_string()])].into();
                tag_core::ExportRow::new(fields, path.as_str(), 1.0, 1)
            })
            .collect();
        let options = tag_core::ExportOptions {
            filename_override: Some(filename),
            partition: Some(partition),
            base_directory: format!("{root}/music/a/CD1"),
            ..Default::default()
        };
        let documents =
            tag_core::render_export("$loop(1)%title%$loopend()", &rows, &options).unwrap();
        let mut targets: Vec<_> = documents
            .iter()
            .map(|d| d.filename.replace('\\', "/"))
            .collect();
        targets.sort();
        assert_eq!(
            targets,
            [
                format!("{root}/out/a/CD1/list.txt"),
                format!("{root}/out/b/CD1/list.txt")
            ]
        );
    }

    fn sample_rows(root: &Path) -> Vec<tag_core::ExportRow> {
        let row = |name: &str, fields: &[(&str, &str)], seconds: f64| {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"x").unwrap();
            let fields: tag_core::Fields = fields
                .iter()
                .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
                .collect();
            let mut row = tag_core::ExportRow::new(fields, path.to_string_lossy(), seconds, 1000);
            // Technical fields that the app supplies with every row.
            let length = format!("{}:{:02}", seconds as u64 / 60, seconds as u64 % 60);
            row.context_fields.insert("_LENGTH".into(), vec![length]);
            row
        };
        vec![
            row(
                "a/01.mp3",
                &[
                    ("TITLE", "Say \"Hi\", <now> & \\ later"),
                    ("ARTIST", "Band"),
                    ("ALBUM", "First"),
                    ("ALBUMARTIST", "Band"),
                    ("YEAR", "2001"),
                    ("TRACK", "1"),
                    ("COMMENT", "x"),
                ],
                61.0,
            ),
            row(
                "a/02.flac",
                &[
                    ("TITLE", "Line\nbreak"),
                    ("ARTIST", "Band"),
                    ("ALBUM", "First"),
                    ("ALBUMARTIST", "Band"),
                    ("YEAR", "2001"),
                    ("TRACK", "2"),
                ],
                30.0,
            ),
            row(
                "b/01.mp3",
                &[("TITLE", "Other"), ("ARTIST", "Solo"), ("ALBUM", "Second")],
                5.0,
            ),
        ]
    }

    fn render(template: &str, root: &Path) -> tag_core::ExportDocument {
        let options = tag_core::ExportOptions {
            filename: root.join("export.txt").to_string_lossy().into_owned(),
            base_directory: root.to_string_lossy().into_owned(),
            ..Default::default()
        };
        let mut documents =
            tag_core::render_export(template, &sample_rows(root), &options).unwrap();
        assert_eq!(documents.len(), 1);
        documents.remove(0)
    }

    #[test]
    fn bundled_templates_validate_and_have_unique_identities() {
        let library = Library::default();
        assert_eq!(library.bundled, BUNDLED);
        validate(&library, true).unwrap();
        assert_eq!(
            library
                .configurations
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Text",
                "Cue sheet",
                "CSV (spreadsheet)",
                "HTML table",
                "Extended M3U playlist",
                "JSON",
                "Album list"
            ]
        );
    }

    #[test]
    fn csv_template_quotes_every_value() {
        let directory = tempfile::tempdir().unwrap();
        let document = render(CSV_TEMPLATE, directory.path());
        assert!(document.filename.ends_with("export.csv"));
        let lines: Vec<_> = document.text.split("\r\n").collect();
        assert_eq!(
            lines[0],
            "\"Path\",\"Title\",\"Artist\",\"Album\",\"Album artist\",\"Track\",\"Disc\",\"Year\",\"Genre\",\"Length\""
        );
        let path = directory
            .path()
            .join("a/01.mp3")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            lines[1],
            format!("\"{path}\",\"Say \"\"Hi\"\", <now> & \\ later\",\"Band\",\"First\",\"Band\",\"1\",\"\",\"2001\",\"\",\"1:01\"")
        );
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[4], "");
    }

    #[test]
    fn html_template_escapes_text_and_sums_the_files() {
        let directory = tempfile::tempdir().unwrap();
        let document = render(HTML_TABLE_TEMPLATE, directory.path());
        assert!(document.filename.ends_with("export.html"));
        let text = &document.text;
        assert!(text.starts_with("<!DOCTYPE html>\n<html>"), "{text}");
        assert!(
            text.contains("<td>Say \"Hi\", &lt;now&gt; &amp; \\ later</td>"),
            "{text}"
        );
        assert!(!text.contains("<now>"));
        assert_eq!(text.matches("<tr><td class=\"n\">").count(), 3);
        assert!(text.contains("<td>01.flac</td>") || text.contains("<td>02.flac</td>"));
        assert!(text.contains("<p>3 files, "), "{text}");
        assert!(text.trim_end().ends_with("</html>"));
    }

    #[test]
    fn m3u_template_writes_extended_entries_that_load_back() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let document = render(M3U_EXTENDED_TEMPLATE, root);
        assert!(document.filename.ends_with("export.m3u8"));
        assert!(document
            .text
            .starts_with("#EXTM3U\r\n#EXTINF:61,Band - Say \"Hi\""));
        assert!(document.text.contains("#EXTINF:5,Solo - Other\r\n"));
        std::fs::write(&document.filename, &document.text).unwrap();
        let loaded =
            tag_core::playlist::load_playlist(Path::new(&document.filename), None).unwrap();
        assert_eq!(
            loaded.files,
            [
                root.join("a/01.mp3"),
                root.join("a/02.flac"),
                root.join("b/01.mp3")
            ]
        );
    }

    #[test]
    fn json_template_writes_valid_json() {
        let directory = tempfile::tempdir().unwrap();
        let document = render(JSON_TEMPLATE, directory.path());
        assert!(document.filename.ends_with("export.json"));
        let value: serde_json::Value = serde_json::from_str(&document.text)
            .unwrap_or_else(|e| panic!("{e}: {}", document.text));
        let items = value.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["title"], "Say \"Hi\", <now> & \\ later");
        assert_eq!(items[1]["title"], "Line\nbreak");
        assert_eq!(items[0]["seconds"], 61);
        assert_eq!(
            items[2]["path"],
            directory.path().join("b/01.mp3").to_string_lossy().as_ref()
        );
    }

    #[test]
    fn album_list_template_names_each_album_once() {
        let directory = tempfile::tempdir().unwrap();
        let document = render(ALBUM_LIST_TEMPLATE, directory.path());
        assert_eq!(document.text, "Band - First (2001)\r\nSolo - Second\r\n");
    }

    #[test]
    fn saved_libraries_receive_new_bundled_configurations_once() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("exports.json");
        fs::create_dir_all(directory.path().join("export")).unwrap();
        // Saved before sets were recorded, with the cue sheet removed and a
        // configuration of its own called "JSON".
        fs::write(
            &path,
            r#"{"version":1,"configurations":[{"id":"text","name":"Text"},{"id":"mine","name":"json"}],"preferences":{"configuration":"mine"}}"#,
        )
        .unwrap();
        fs::write(directory.path().join("export/text.mte"), "%title%").unwrap();
        fs::write(directory.path().join("export/mine.mte"), "%artist%").unwrap();
        let library = load(&path).unwrap();
        let names: Vec<_> = library
            .configurations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Text",
                "json",
                "CSV (spreadsheet)",
                "HTML table",
                "Extended M3U playlist",
                "Album list"
            ]
        );
        assert_eq!(library.configurations[0].template, "%title%");
        assert_eq!(library.preferences.configuration.as_deref(), Some("mine"));
        // Removing an offered configuration before it was ever saved works,
        // and it is not offered again.
        let mut removed = library.clone();
        removed.configurations.retain(|c| c.id != "csv");
        save(&path, &removed).unwrap();
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded.bundled, BUNDLED);
        assert!(reloaded.configurations.iter().all(|c| c.id != "csv"));
        assert!(reloaded.configurations.iter().any(|c| c.id == "html-table"));
        assert_eq!(
            reloaded
                .configurations
                .iter()
                .find(|c| c.id == "json")
                .map(|c| c.name.as_str()),
            None
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
        let bundled = Library::default().configurations.len();
        let names: Vec<_> = library.configurations[bundled - 7..]
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names[..2], ["Text", "Cue sheet"]);
        assert_eq!(
            library.configurations[bundled..]
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["Cue Sheet (2)", "HTML List"]
        );
        let html = &library.configurations[bundled + 1];
        assert_eq!(html.file.as_deref(), Some("HTML List.mte"));
        assert_eq!(html.template, "$filename(html,utf-8)<ul>");
        assert_eq!(library.configurations[bundled].template, "Björk");
        // Identities are stable until the library records the files.
        assert_eq!(load(&path).unwrap().configurations[bundled + 1].id, html.id);

        let mut saved = library.clone();
        saved.preferences.configuration = Some(html.id.clone());
        saved.configurations[bundled + 1].template = "$filename(html,utf-8)<ol>".into();
        save(&path, &saved).unwrap();
        assert_eq!(
            fs::read_to_string(folder.join("HTML List.mte")).unwrap(),
            "\u{feff}$filename(html,utf-8)<ol>"
        );
        // The untouched ANSI file is not rewritten.
        assert_eq!(fs::read(folder.join("Cue Sheet.MTE")).unwrap(), b"Bj\xf6rk");
        assert!(!folder.join(format!("{}.mte", html.id)).exists());
        let reloaded = load(&path).unwrap();
        assert_eq!(reloaded.configurations.len(), bundled + 2);
        assert_eq!(reloaded.preferences.configuration, Some(html.id.clone()));

        // Another file dropped later joins the saved library.
        fs::write(folder.join("Late.mte"), "late").unwrap();
        assert_eq!(
            load(&path).unwrap().configurations[bundled + 2].name,
            "Late"
        );
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
