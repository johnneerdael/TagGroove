//! Utils → Remove Tag Types, Remove Unsupported Fields, Optimize and Strip
//! chapters MP4, Options → Tools (user-defined external programs)
//! and File → Save/Load configuration (https://docs.mp3tag.de/utils/,
//! https://docs.mp3tag.de/customization/options/tools/, menus/file).
use crate::{history, io, mutate_document, settings, whole_file_batch};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tag_core::{RemovableTag, ScanResult};

#[tauri::command]
pub async fn remove_tag_types(
    paths: Vec<String>,
    kinds: Vec<RemovableTag>,
) -> Result<ScanResult, String> {
    if kinds.is_empty() {
        return Err("Choose at least one tag type to remove".into());
    }
    io(move || {
        crate::reject_provider_operation(&paths, "Removing tag types")?;
        history::record("Remove tag types", None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::remove_tag_kinds_with_options(
                        p,
                        &kinds,
                        false,
                        &settings::tag_options(),
                    )
                })
            }))
        })
    })
    .await
}

/// Utils that rewrite files without changing the displayed tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Util {
    RemoveUnsupportedFields,
    Optimize,
}
impl Util {
    fn label(self) -> &'static str {
        match self {
            Self::RemoveUnsupportedFields => "Remove unsupported fields",
            Self::Optimize => "Optimize",
        }
    }
}

/// Run a util on each file as one undo step. Files the util has nothing to
/// do for are returned unchanged; unsupported formats are reported per file.
#[tauri::command]
pub async fn apply_util(paths: Vec<String>, util: Util) -> Result<ScanResult, String> {
    io(move || {
        crate::reject_provider_operation(&paths, util.label())?;
        history::record(util.label(), None, &paths, || {
            let options = settings::tag_options();
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| match util {
                    Util::RemoveUnsupportedFields => {
                        tag_core::remove_unsupported_fields_with_options(p, false, &options)
                    }
                    Util::Optimize => tag_core::optimize_with_options(p, false, &options),
                })
            }))
        })
    })
    .await
}

/// Utils → Strip chapters MP4. Files without MP4 chapters are left untouched.
#[tauri::command]
pub async fn strip_mp4_chapters(paths: Vec<String>) -> Result<ScanResult, String> {
    io(move || {
        crate::reject_provider_operation(&paths, "Stripping MP4 chapters")?;
        history::record("Strip chapters MP4", None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::strip_mp4_chapters_with_options(p, false, &settings::tag_options())
                })
            }))
        })
    })
    .await
}

/// One entry of Options → Tools.
#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub parameters: String,
    /// Run once per selected file instead of once for the first one.
    #[serde(default)]
    pub for_each_file: bool,
}

/// Limit for one invocation, so a mistaken selection cannot fork thousands of processes.
pub const TOOL_LIMIT: usize = 100;

/// Split a parameter line into arguments. Whitespace separates arguments and
/// double quotes group them; backslashes are literal so Windows paths survive.
pub fn split_parameters(line: &str) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut started) = (false, false);
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    output.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err("The tool parameters contain an unmatched quote".into());
    }
    if started {
        output.push(current);
    }
    Ok(output)
}

fn with_separator(path: &Path) -> String {
    let mut text = path.to_string_lossy().into_owned();
    if !text.is_empty() && !text.ends_with(std::path::MAIN_SEPARATOR) {
        text.push(std::path::MAIN_SEPARATOR);
    }
    text
}

/// Replace the documented placeholders (case-insensitive) in one argument.
/// Relative values are relative to `root`, or absolute outside it.
pub fn expand(argument: &str, file: &Path, root: Option<&Path>) -> String {
    let folder = file.parent().unwrap_or(Path::new(""));
    let relative = |path: &Path| {
        root.and_then(|root| path.strip_prefix(root).ok())
            .unwrap_or(path)
            .to_path_buf()
    };
    let name = |path: &Path| path.to_string_lossy().into_owned();
    let os =
        |value: Option<&std::ffi::OsStr>| value.unwrap_or_default().to_string_lossy().into_owned();
    let values = [
        ("%_path%", name(file)),
        ("%_filename_ext%", os(file.file_name())),
        ("%_filename_rel%", name(&relative(file))),
        ("%_filename%", os(file.file_stem())),
        ("%_directory%", os(folder.file_name())),
        ("%_folderpath_rel%", {
            let folder = relative(folder);
            if folder.as_os_str().is_empty() {
                String::new()
            } else {
                with_separator(&folder)
            }
        }),
        ("%_folderpath%", with_separator(folder)),
    ];
    let mut output = String::new();
    let mut rest = argument;
    'scan: while let Some(start) = rest.find('%') {
        output.push_str(&rest[..start]);
        let candidate = &rest[start..];
        for (placeholder, value) in &values {
            if candidate.len() >= placeholder.len()
                && candidate.is_char_boundary(placeholder.len())
                && candidate[..placeholder.len()].eq_ignore_ascii_case(placeholder)
            {
                output.push_str(value);
                rest = &candidate[placeholder.len()..];
                continue 'scan;
            }
        }
        output.push('%');
        rest = &candidate[1..];
    }
    output.push_str(rest);
    output
}

/// Argument lists for each process a tool starts. Empty parameters pass the
/// file path, so a plain program path opens the file.
pub fn invocations(
    tool: &Tool,
    files: &[PathBuf],
    root: Option<&Path>,
) -> Result<Vec<Vec<String>>, String> {
    let template = if tool.parameters.trim().is_empty() {
        vec!["%_path%".to_string()]
    } else {
        split_parameters(&tool.parameters)?
    };
    let targets = if tool.for_each_file {
        files
    } else {
        &files[..files.len().min(1)]
    };
    if targets.len() > TOOL_LIMIT {
        return Err(format!(
            "Select at most {TOOL_LIMIT} files to run a tool for each file"
        ));
    }
    Ok(targets
        .iter()
        .map(|file| {
            template
                .iter()
                .map(|argument| expand(argument, file, root))
                .collect()
        })
        .collect())
}

fn program(tool: &Tool) -> Result<PathBuf, String> {
    if tool.name.trim().is_empty() {
        return Err("The tool has no name".into());
    }
    let path = PathBuf::from(tool.path.trim());
    if !path.is_absolute() {
        return Err(format!("{}: enter the full path of the program", tool.name));
    }
    let bundle = cfg!(target_os = "macos")
        && path.is_dir()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("app"));
    if !path.is_file() && !bundle {
        return Err(format!("{}: {} was not found", tool.name, path.display()));
    }
    Ok(path)
}

/// Start the tool without a shell: the program receives the expanded argument
/// list directly. macOS application bundles are started with `open -a`.
pub fn launch(
    tool: &Tool,
    files: &[PathBuf],
    root: Option<&Path>,
) -> Result<Vec<std::process::Child>, String> {
    let program = program(tool)?;
    let runs = invocations(tool, files, root)?;
    let mut children = Vec::new();
    for (arguments, file) in runs.into_iter().zip(files) {
        let mut command = if program.is_dir() {
            let mut command = std::process::Command::new("/usr/bin/open");
            command.arg("-a").arg(&program).arg("--args");
            command
        } else {
            std::process::Command::new(&program)
        };
        command
            .args(&arguments)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Some(folder) = file.parent().filter(|p| p.is_dir()) {
            command.current_dir(folder);
        }
        children.push(command.spawn().map_err(|e| format!("{}: {e}", tool.name))?);
    }
    Ok(children)
}

#[derive(Serialize)]
pub struct ToolRun {
    pub started: usize,
}

#[tauri::command]
pub async fn run_tool(
    tool: Tool,
    paths: Vec<String>,
    root: Option<String>,
) -> Result<ToolRun, String> {
    io(move || {
        crate::reject_provider_operation(&paths, "Tools")?;
        if paths.is_empty() {
            return Err("Select files first".into());
        }
        let files: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        if let Some(missing) = files.iter().find(|p| !p.is_file()) {
            return Err(format!("{} was not found", missing.display()));
        }
        let children = launch(&tool, &files, root.as_deref().map(Path::new))?;
        let started = children.len();
        // Reap the processes so finished tools do not linger as zombies.
        std::thread::spawn(move || {
            for mut child in children {
                let _ = child.wait();
            }
        });
        Ok(ToolRun { started })
    })
    .await
}

/// Configuration files that make up a backup, with the validation that each
/// module applies when it loads them. Discogs credentials are deliberately excluded.
const CONFIGURATION: [&str; 7] = [
    "settings.json",
    "actions.json",
    "exports.json",
    "columns.json",
    "tag-panel.json",
    "directories.json",
    "lists.json",
];
pub const BUNDLE_FORMAT: &str = "opentag-configuration";
const BUNDLE_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub version: u32,
    pub files: BTreeMap<String, serde_json::Value>,
}

fn validate_file(name: &str, path: &Path) -> Result<(), String> {
    match name {
        "settings.json" => match settings::load(path) {
            (_, Some(warning)) => Err(warning),
            _ => Ok(()),
        },
        "actions.json" => crate::action_host::load_library(path).map(drop),
        "exports.json" => crate::exports::load(path).map(drop),
        "columns.json" => crate::columns::load(path).map(drop),
        "tag-panel.json" => crate::tag_panel::load(path).map(drop),
        "directories.json" => crate::file_ops::load(path).map(drop),
        "lists.json" => crate::lists::load(path).map(drop),
        _ => Err(format!("{name} is not an OpenTag configuration file")),
    }
}

/// Collect the existing configuration files of `directory` into one bundle.
pub fn collect(directory: &Path) -> Result<Bundle, String> {
    let mut files = BTreeMap::new();
    for name in CONFIGURATION {
        let bytes = match fs::read(directory.join(name)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{name}: {e}")),
        };
        let value = serde_json::from_slice(&bytes).map_err(|e| format!("{name}: {e}"))?;
        files.insert(name.to_string(), value);
    }
    Ok(Bundle {
        format: BUNDLE_FORMAT.into(),
        version: 1,
        files,
    })
}

/// Validate every file of a bundle before writing any, then replace the
/// matching files of `directory`. Files missing from the bundle are kept.
pub fn restore(directory: &Path, bytes: &[u8]) -> Result<Vec<String>, String> {
    let bundle: Bundle =
        serde_json::from_slice(bytes).map_err(|e| format!("Not an OpenTag configuration: {e}"))?;
    if bundle.format != BUNDLE_FORMAT {
        return Err("Not an OpenTag configuration file".into());
    }
    if bundle.version != 1 {
        return Err(format!(
            "Configuration version {} is not supported",
            bundle.version
        ));
    }
    if bundle.files.is_empty() {
        return Err("The configuration file contains no settings".into());
    }
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut validated = Vec::new();
    for (name, value) in &bundle.files {
        if !CONFIGURATION.contains(&name.as_str()) {
            return Err(format!("{name} is not an OpenTag configuration file"));
        }
        let content = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
        let path = staging.path().join(name);
        fs::write(&path, &content).map_err(|e| e.to_string())?;
        validate_file(name, &path).map_err(|e| format!("{name}: {e}"))?;
        validated.push((name.clone(), content));
    }
    for (name, content) in &validated {
        crate::action_host::atomic_output(&directory.join(name), content, false)?;
    }
    Ok(validated.into_iter().map(|(name, _)| name).collect())
}

fn configuration_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?
        .parent()
        .ok_or("The configuration folder is unavailable")?
        .to_path_buf())
}

#[tauri::command]
pub async fn export_configuration(
    app: tauri::AppHandle,
    output_path: String,
) -> Result<usize, String> {
    let directory = configuration_directory(&app)?;
    io(move || {
        let bundle = collect(&directory)?;
        let bytes = serde_json::to_vec_pretty(&bundle).map_err(|e| e.to_string())?;
        crate::action_host::atomic_output(Path::new(&output_path), &bytes, false)?;
        Ok(bundle.files.len())
    })
    .await
}

#[tauri::command]
pub async fn import_configuration(
    app: tauri::AppHandle,
    path: String,
) -> Result<Vec<String>, String> {
    let directory = configuration_directory(&app)?;
    io(move || {
        let path = crate::auxiliary_input(&path)?;
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > BUNDLE_LIMIT {
            return Err("The configuration file is too large".into());
        }
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        let restored = restore(&directory, &bytes)?;
        if restored.iter().any(|name| name == "settings.json") {
            if let Some(warning) = settings::reload(&app)? {
                return Err(warning);
            }
        }
        Ok(restored)
    })
    .await
}

#[tauri::command]
pub async fn open_configuration_folder(app: tauri::AppHandle) -> Result<(), String> {
    let directory = configuration_directory(&app)?;
    io(move || {
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        #[cfg(desktop)]
        return tauri_plugin_opener::open_path(&directory, None::<&str>).map_err(|e| e.to_string());
        #[cfg(mobile)]
        return Err("Opening folders is not available here".into());
    })
    .await
}

/// Edit → Copy as Text writes through the native clipboard; WebKit rejects
/// `navigator.clipboard` once the click's user activation has been awaited away.
#[tauri::command]
pub async fn write_clipboard_text(text: String) -> Result<(), String> {
    #[cfg(desktop)]
    {
        // The clipboard object is kept so X11 can keep serving the text.
        static CLIPBOARD: std::sync::Mutex<Option<arboard::Clipboard>> =
            std::sync::Mutex::new(None);
        tauri::async_runtime::spawn_blocking(move || {
            let mut guard = CLIPBOARD.lock().map_err(|_| "Clipboard lock failed")?;
            if guard.is_none() {
                *guard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
            }
            guard
                .as_mut()
                .expect("clipboard was opened")
                .set_text(text)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(mobile)]
    {
        let _ = text;
        Err("The clipboard is not available here".into())
    }
}

/// Help menu destinations. Only these fixed pages can be opened.
const HELP_PAGES: [(&str, &str); 4] = [
    ("repository", "https://github.com/johnneerdael/opentag"),
    (
        "documentation",
        "https://johnneerdael.github.io/audio-tag-guide/",
    ),
    ("issues", "https://github.com/johnneerdael/opentag/issues"),
    (
        "releases",
        "https://github.com/johnneerdael/opentag/releases",
    ),
];

#[tauri::command]
pub async fn open_help_page(page: String) -> Result<(), String> {
    let url = HELP_PAGES
        .iter()
        .find(|(id, _)| *id == page)
        .map(|(_, url)| *url)
        .ok_or("Unknown help page")?;
    #[cfg(desktop)]
    return tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string());
    #[cfg(mobile)]
    return Err(format!("Open {url} in a browser"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_split_on_whitespace_and_keep_quoted_groups() {
        assert_eq!(
            split_parameters(r#"/add  "%_path%" -x "" C:\dir\ "a b"c"#).unwrap(),
            vec!["/add", "%_path%", "-x", "", r"C:\dir\", "a bc"]
        );
        assert!(split_parameters(r#""open"#).is_err());
        assert!(split_parameters("   ").unwrap().is_empty());
    }

    #[test]
    fn placeholders_expand_per_file_and_relative_to_the_root() {
        let root = Path::new("/music");
        let file = Path::new("/music/Artist/Album/01 Song.mp3");
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            expand("%_path%", file, Some(root)),
            "/music/Artist/Album/01 Song.mp3"
        );
        assert_eq!(expand("%_FILENAME_EXT%", file, Some(root)), "01 Song.mp3");
        assert_eq!(expand("%_filename%.txt", file, Some(root)), "01 Song.txt");
        assert_eq!(
            expand("%_filename_rel%", file, Some(root)),
            "Artist/Album/01 Song.mp3"
        );
        assert_eq!(expand("%_directory%", file, Some(root)), "Album");
        assert_eq!(
            expand("%_folderpath%", file, Some(root)),
            format!("/music/Artist/Album{sep}")
        );
        assert_eq!(
            expand("%_folderpath_rel%", file, Some(root)),
            format!("Artist/Album{sep}")
        );
        assert_eq!(
            expand("%_filename_rel%", file, Some(Path::new("/other"))),
            "/music/Artist/Album/01 Song.mp3"
        );
        assert_eq!(
            expand("100% %title% %_path", file, None),
            "100% %title% %_path"
        );
        // Values are inserted verbatim: no second expansion and no shell meaning.
        let odd = Path::new("/m/%_path% $(x) ;rm.mp3");
        assert_eq!(expand("%_filename_ext%", odd, None), "%_path% $(x) ;rm.mp3");
    }

    #[test]
    fn invocations_follow_the_for_each_file_switch_and_limit() {
        let files: Vec<PathBuf> = ["/a/1.mp3", "/a/2.mp3"].iter().map(PathBuf::from).collect();
        let mut tool = Tool {
            name: "Echo".into(),
            path: "/bin/echo".into(),
            parameters: String::new(),
            for_each_file: false,
        };
        assert_eq!(
            invocations(&tool, &files, None).unwrap(),
            vec![vec!["/a/1.mp3"]]
        );
        tool.for_each_file = true;
        tool.parameters = r#"-n "%_filename%""#.into();
        assert_eq!(
            invocations(&tool, &files, None).unwrap(),
            vec![vec!["-n", "1"], vec!["-n", "2"]]
        );
        let many: Vec<PathBuf> = (0..=TOOL_LIMIT)
            .map(|i| PathBuf::from(format!("/a/{i}.mp3")))
            .collect();
        assert!(invocations(&tool, &many, None).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn tools_receive_arguments_without_a_shell() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("record.sh");
        fs::write(&script, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$3.out\"\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let file = dir.path().join("a song; $(touch pwned).mp3");
        fs::write(&file, b"x").unwrap();
        let tool = Tool {
            name: "Record".into(),
            path: script.to_string_lossy().into_owned(),
            parameters: r#"--file "%_path%" %_directory%"#.into(),
            for_each_file: true,
        };
        for mut child in launch(&tool, std::slice::from_ref(&file), None).unwrap() {
            assert!(child.wait().unwrap().success());
        }
        let folder = dir.path().file_name().unwrap().to_string_lossy();
        let output = fs::read_to_string(dir.path().join(format!("{folder}.out"))).unwrap();
        assert_eq!(output, format!("--file\n{}\n{folder}\n", file.display()));
        assert!(!dir.path().join("pwned").exists());

        let missing = Tool {
            path: dir.path().join("absent").to_string_lossy().into_owned(),
            ..tool.clone()
        };
        assert!(launch(&missing, std::slice::from_ref(&file), None).is_err());
        let relative = Tool {
            path: "record.sh".into(),
            ..tool
        };
        assert!(launch(&relative, &[file], None).is_err());
    }

    #[test]
    fn configuration_bundles_round_trip_and_reject_invalid_files() {
        let source = tempfile::tempdir().unwrap();
        let mut options = settings::Settings::default();
        options.general.natural_sort = false;
        settings::save(&source.path().join("settings.json"), &options).unwrap();
        fs::write(
            source.path().join("directories.json"),
            r#"{"version":1,"recent":["/music"],"favorites":[]}"#,
        )
        .unwrap();
        fs::write(
            source.path().join("tag-sources.json"),
            r#"{"discogs_token":"secret"}"#,
        )
        .unwrap();
        let bundle = collect(source.path()).unwrap();
        assert_eq!(
            bundle.files.keys().collect::<Vec<_>>(),
            ["directories.json", "settings.json"]
        );
        let bytes = serde_json::to_vec(&bundle).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret"));

        let target = tempfile::tempdir().unwrap();
        fs::write(target.path().join("columns.json"), "kept").unwrap();
        assert_eq!(
            restore(target.path(), &bytes).unwrap(),
            ["directories.json", "settings.json"]
        );
        assert_eq!(
            settings::load(&target.path().join("settings.json")),
            (options, None)
        );
        assert_eq!(
            fs::read_to_string(target.path().join("columns.json")).unwrap(),
            "kept"
        );

        // One invalid file rejects the whole bundle before anything is written.
        let fresh = tempfile::tempdir().unwrap();
        for invalid in [
            r#"{"format":"opentag-configuration","version":1,"files":{"settings.json":{"general":{}},"columns.json":{"columns":"no"}}}"#,
            r#"{"format":"opentag-configuration","version":1,"files":{"settings.json":{"version":99}}}"#,
            r#"{"format":"opentag-configuration","version":1,"files":{"../evil.json":{}}}"#,
            r#"{"format":"opentag-configuration","version":2,"files":{"settings.json":{}}}"#,
            r#"{"format":"other","version":1,"files":{"settings.json":{}}}"#,
            r#"{"format":"opentag-configuration","version":1,"files":{}}"#,
            "not json",
        ] {
            assert!(
                restore(fresh.path(), invalid.as_bytes()).is_err(),
                "{invalid}"
            );
            assert_eq!(fs::read_dir(fresh.path()).unwrap().count(), 0, "{invalid}");
        }
    }

    #[test]
    fn configuration_bundles_carry_helper_lists() {
        let source = tempfile::tempdir().unwrap();
        let lists: BTreeMap<String, serde_json::Value> = [(
            "opentag.history.filter".to_string(),
            serde_json::json!([{"value":"%genre% IS jazz","name":"Jazz","favorite":true}]),
        )]
        .into();
        crate::lists::save(&source.path().join("lists.json"), &lists).unwrap();
        let bundle = collect(source.path()).unwrap();
        assert_eq!(bundle.files.keys().collect::<Vec<_>>(), ["lists.json"]);
        let target = tempfile::tempdir().unwrap();
        let bytes = serde_json::to_vec(&bundle).unwrap();
        assert_eq!(restore(target.path(), &bytes).unwrap(), ["lists.json"]);
        assert_eq!(
            crate::lists::load(&target.path().join("lists.json"))
                .unwrap()
                .lists,
            lists
        );
        let invalid = br#"{"format":"opentag-configuration","version":1,"files":{"lists.json":{"version":1,"lists":{"opentag.other":[]}}}}"#;
        assert!(restore(target.path(), invalid).is_err());
    }

    #[test]
    fn apply_util_reports_unsupported_formats_per_file() {
        let dir = tempfile::tempdir().unwrap();
        let mp3 = dir.path().join("song.mp3");
        let mut audio = Vec::new();
        for frame in 0..20u32 {
            audio.extend([0xFF, 0xFB, 0x90, 0x64]);
            audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
        }
        fs::write(&mp3, &audio).unwrap();
        let key = mp3.to_string_lossy().into_owned();
        let result =
            tauri::async_runtime::block_on(apply_util(vec![key.clone()], Util::Optimize)).unwrap();
        assert!(result.entries.is_empty());
        assert!(result.errors[0].error.contains("FLAC and MP4"));
        let result =
            tauri::async_runtime::block_on(apply_util(vec![key], Util::RemoveUnsupportedFields))
                .unwrap();
        assert_eq!(result.entries.len(), 1, "nothing to remove is not an error");
        assert_eq!(fs::read(&mp3).unwrap(), audio);
    }

    #[test]
    fn remove_tag_types_command_strips_only_the_chosen_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mp3");
        let mut audio = Vec::new();
        for frame in 0..20u32 {
            audio.extend([0xFF, 0xFB, 0x90, 0x64]);
            audio.extend((0..413u32).map(|i| ((i + frame * 7) % 200) as u8));
        }
        fs::write(&path, &audio).unwrap();
        let mut options = settings::Settings::default().tags;
        options.mpeg.write_ape = true;
        tag_core::write_fields_with_options(
            &path,
            &tag_core::Fields::from([("TITLE".into(), vec!["Song".into()])]),
            false,
            &[],
            &options,
        )
        .unwrap();
        let key = path.to_string_lossy().into_owned();
        let result = tauri::async_runtime::block_on(remove_tag_types(
            vec![key.clone()],
            vec![RemovableTag::Ape, RemovableTag::Id3v1],
        ))
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.entries[0].technical["_TAG"], ["ID3v2.3"]);
        assert_eq!(result.entries[0].fields["TITLE"], ["Song"]);
        assert!(tauri::async_runtime::block_on(remove_tag_types(vec![key], vec![])).is_err());
    }
}
