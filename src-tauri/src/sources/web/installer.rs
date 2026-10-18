//! Bounded bundle intake and replacement. Private source text never leaves the machine.
use super::{extension, load, plain_name, relative_name, schema, MAX_DEPTH, MAX_FILE_BYTES};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(super) static LOCK: Mutex<()> = Mutex::new(());
const MAX_ENTRIES: usize = 4096;
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXISTING_DEPTH: usize = 64;
const BACKUP: &str = ".opentag-sources-previous";
const MARKER: &str = ".opentag-source-install";

type Files = BTreeMap<String, Vec<u8>>;

#[derive(Default)]
struct ExistingTree {
    files: BTreeMap<PathBuf, Vec<u8>>,
    directories: BTreeSet<PathBuf>,
    bytes: u64,
    entries: usize,
}

/// Source identifiers use Unicode and forward slashes; unrelated native paths need not.
fn source_name(path: &Path) -> Option<String> {
    path.iter()
        .map(|part| part.to_str())
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
}

fn supported(name: &str) -> bool {
    matches!(extension(name).as_str(), "src" | "inc" | "settings")
}

fn add(files: &mut Files, name: &str, bytes: Vec<u8>, total: &mut u64) -> Result<(), String> {
    relative_name(name)?;
    if files
        .keys()
        .any(|old| old.to_lowercase() == name.to_lowercase())
    {
        return Err(format!(
            "The bundle repeats {name} (file names ignore case)."
        ));
    }
    *total += bytes.len() as u64;
    if bytes.len() as u64 > MAX_FILE_BYTES || *total > MAX_BYTES || files.len() >= MAX_ENTRIES {
        return Err(
            "Source bundle exceeds the limit: 4 MB per source file, 128 MB total, 4096 files."
                .into(),
        );
    }
    files.insert(name.into(), bytes);
    Ok(())
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.len() > limit {
        return Err(format!(
            "{} must be a regular file of at most {limit} bytes (no symbolic links).",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("File grew beyond the installation limit.".into());
    }
    Ok(bytes)
}

fn directory(
    path: &Path,
    prefix: &str,
    depth: usize,
    files: &mut Files,
    total: &mut u64,
    entries: &mut usize,
) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        *entries += 1;
        if *entries > MAX_ENTRIES {
            return Err("Source folder exceeds 4096 entries.".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Source folder has a non-Unicode file name.")?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "{name}: symbolic links are not accepted in source folders."
            ));
        }
        // Download metadata and documentation are never installed.
        if name.starts_with('.') {
            continue;
        }
        let relative = format!("{prefix}{name}");
        if kind.is_dir() {
            let depth_limit = MAX_DEPTH;
            if depth >= depth_limit {
                return Err(format!(
                    "{relative}: source folders can be at most {depth_limit} levels deep."
                ));
            }
            plain_name(&name)?;
            directory(
                &entry.path(),
                &format!("{relative}/"),
                depth + 1,
                files,
                total,
                entries,
            )?;
        } else if supported(&name) {
            plain_name(&name)?;
            let bytes = read(&entry.path(), MAX_FILE_BYTES)?;
            *total += bytes.len() as u64;
            if *total > MAX_BYTES {
                return Err("Sources folder exceeds the 128 MB staging limit.".into());
            }
            *total -= bytes.len() as u64;
            add(files, &relative, bytes, total)?;
        }
    }
    Ok(())
}

/// Snapshot local files and directories without converting their native names.
fn existing_directory(
    path: &Path,
    prefix: &Path,
    depth: usize,
    tree: &mut ExistingTree,
) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        tree.entries += 1;
        if tree.entries > MAX_ENTRIES {
            return Err("Source folder exceeds 4096 entries.".into());
        }
        let name = entry.file_name();
        let relative = prefix.join(&name);
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            let name = name.to_string_lossy();
            return Err(format!(
                "{name}: symbolic links are not accepted in source folders."
            ));
        }
        if kind.is_dir() {
            if depth >= MAX_EXISTING_DEPTH {
                let relative = relative.display();
                let depth_limit = MAX_EXISTING_DEPTH;
                return Err(format!(
                    "{relative}: source folders can be at most {depth_limit} levels deep."
                ));
            }
            tree.directories.insert(relative.clone());
            existing_directory(&entry.path(), &relative, depth + 1, tree)?;
        } else {
            let bytes = read(&entry.path(), MAX_BYTES)?;
            tree.bytes += bytes.len() as u64;
            if tree.bytes > MAX_BYTES {
                return Err("Sources folder exceeds the 128 MB staging limit.".into());
            }
            tree.files.insert(relative, bytes);
        }
    }
    Ok(())
}

fn archive(
    path: &Path,
    files: &mut Files,
    total: &mut u64,
    entries: &mut usize,
) -> Result<(), String> {
    let bytes = read(path, MAX_BYTES)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    *entries += zip.len();
    if *entries > MAX_ENTRIES {
        return Err("Source ZIP exceeds 4096 entries.".into());
    }
    let mut seen = BTreeSet::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("ZIP entry {i}: {e}"))?;
        let name = entry.name().replace('\\', "/");
        let name = name.trim_end_matches('/');
        // Validate every archive entry, including ignored documentation. Never
        // extract any paths before this check.
        if name.starts_with('/')
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
        {
            return Err(format!("ZIP contains an unsafe path: {name}"));
        }
        if name.contains(':') {
            return Err(format!("ZIP contains an unsafe path: {name}"));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(format!("The ZIP repeats {name} (file names ignore case)."));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!(
                "{name}: symbolic links are not accepted in ZIP files."
            ));
        }
        if entry.is_dir()
            || name
                .split('/')
                .any(|part| part.starts_with('.') || part == "__MACOSX")
            || !supported(name)
        {
            continue;
        }
        relative_name(name)?;
        if entry.size() > MAX_FILE_BYTES {
            return Err(format!("{name} is larger than 4 MB."));
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("{name}: {e}"))?;
        add(files, name, bytes, total)?;
    }
    Ok(())
}

/// An interrupted directory swap retains the old complete folder beside it.
/// Only restore/delete a backup carrying our marker, never an unrelated folder.
pub(super) fn recover(folder: &Path) -> Result<(), String> {
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    let backup = parent.join(BACKUP);
    if !backup.exists() {
        return Ok(());
    }
    if std::fs::symlink_metadata(&backup)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
        || std::fs::read(backup.join(MARKER)).ok().as_deref()
            != Some(b"source installer backup v1\n")
    {
        return Err(format!(
            "Installation recovery folder {} has no marker; move it aside before installing.",
            backup.display()
        ));
    }
    if !folder.exists() {
        std::fs::rename(&backup, folder)
            .map_err(|e| format!("Restore {} to {}: {e}", backup.display(), folder.display()))?;
        let _ = std::fs::remove_file(folder.join(MARKER));
    }
    Ok(())
}

/// Canonicalize source identifiers while retaining every native metadata path.
fn prepare_incoming(existing: &ExistingTree, incoming: Files) -> Result<Files, String> {
    let mut spellings = BTreeMap::new();
    for name in existing
        .files
        .keys()
        .chain(existing.directories.iter())
        .filter_map(|path| source_name(path))
    {
        let mut prefix = String::new();
        for part in name.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(previous) = spellings.insert(prefix.to_lowercase(), prefix.clone()) {
                if previous != prefix {
                    return Err(format!("Installed paths differ only in case: {previous} and {prefix}. Rename one before installing."));
                }
            }
        }
    }
    let incoming = incoming
        .into_iter()
        .map(|(name, bytes)| {
            let mut canonical = String::new();
            for part in name.split('/') {
                if !canonical.is_empty() {
                    canonical.push('/');
                }
                canonical.push_str(part);
                let known = spellings
                    .entry(canonical.to_lowercase())
                    .or_insert_with(|| canonical.clone());
                canonical = known.clone();
            }
            (canonical, bytes)
        })
        .collect::<Files>();
    let incoming_paths = incoming.keys().map(PathBuf::from).collect::<BTreeSet<_>>();
    let mut combined_entries = BTreeSet::new();
    for path in existing
        .files
        .keys()
        .chain(existing.directories.iter())
        .chain(incoming_paths.iter())
    {
        for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
            combined_entries.insert(ancestor.to_path_buf());
        }
    }
    if combined_entries.len() > MAX_ENTRIES {
        return Err("Source folder exceeds 4096 entries.".into());
    }
    let combined_bytes = existing
        .files
        .iter()
        .filter(|(path, _)| !incoming_paths.contains(*path))
        .map(|(_, bytes)| bytes.len() as u64)
        .chain(incoming.values().map(|bytes| bytes.len() as u64))
        .sum::<u64>();
    if combined_bytes > MAX_BYTES {
        return Err("Combined sources exceed the 128 MB staging limit.".into());
    }
    Ok(incoming)
}

fn write_all<P: AsRef<Path>>(folder: &Path, files: &BTreeMap<P, Vec<u8>>) -> Result<(), String> {
    for (name, bytes) in files {
        let relative = name.as_ref();
        let path = folder.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| {
            let name = relative.display();
            format!("{name}: {e}")
        })?;
    }
    Ok(())
}

pub(super) fn prepare_folder_open(folder: &Path) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    recover(folder)?;
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())
}

pub(super) fn install(folder: &Path, paths: &[String]) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    recover(folder)?;
    if paths.len() > MAX_ENTRIES {
        return Err("Choose at most 4096 source paths.".into());
    }
    if paths.is_empty() {
        return Err("Choose a source folder, ZIP, or .src, .inc and .settings files.".into());
    }
    let mut incoming = Files::new();
    let mut complete = false;
    let mut total = 0;
    let mut entries = 0;
    for path in paths {
        let path = Path::new(path);
        let meta =
            std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.is_symlink() {
            return Err("Symbolic links cannot be installed as source bundles.".into());
        }
        if meta.is_dir() {
            complete = true;
            directory(path, "", 0, &mut incoming, &mut total, &mut entries)?;
        } else if extension(&path.to_string_lossy()) == "zip" {
            complete = true;
            archive(path, &mut incoming, &mut total, &mut entries)?;
        } else {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Source file has no usable name.")?;
            if !supported(name) {
                return Err(format!(
                    "{name} is not a .src, .inc or .settings file or ZIP."
                ));
            }
            add(&mut incoming, name, read(path, MAX_FILE_BYTES)?, &mut total)?;
        }
    }
    if incoming.is_empty() {
        return Err("The bundle contains no .src, .inc or .settings files.".into());
    }
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".opentag-sources-stage-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let mut existing = ExistingTree::default();
    if folder.exists() {
        let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.is_symlink() {
            return Err("The sources folder must be a real directory.".into());
        }
        existing_directory(folder, Path::new(""), 0, &mut existing)?;
    }
    let incoming = prepare_incoming(&existing, incoming)?;
    for name in &existing.directories {
        std::fs::create_dir_all(stage.path().join(name)).map_err(|e| e.to_string())?;
    }
    write_all(stage.path(), &existing.files)?;
    write_all(stage.path(), &incoming)?;
    // Folder/ZIP bundles must be self-contained. Explicit-file selections also
    // support incremental updates using validated installed companions.
    let check = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    write_all(check.path(), &incoming)?;
    let checked = if complete { check.path() } else { stage.path() };
    for name in incoming.keys() {
        match extension(name).as_str() {
            "src" => {
                let file = load(checked, name)
                    .map_err(|e| format!("{name}: {e}. Choose the complete bundle."))?;
                super::validate_scripts(&file)?;
                if let Some(settings) = file.settings {
                    schema(checked, &settings)
                        .map_err(|e| format!("{name}: [Settings]={settings}: {e}"))?;
                }
            }
            "settings" => {
                schema(checked, name)?;
            }
            "inc" => {
                super::source::validate_includes(name, &super::reader(checked))?;
            }
            _ => {}
        }
    }
    // A shared include/settings update must not break a previously working
    // source outside the incoming collection. Keep pre-existing problems visible
    // without making an unrelated old broken source block every new install.
    let validate_source = |root: &Path, name: &str| -> Result<(), String> {
        let source = load(root, name)?;
        super::validate_scripts(&source)?;
        if let Some(settings) = source.settings {
            schema(root, &settings)?;
        }
        Ok(())
    };
    for name in existing
        .files
        .keys()
        .filter_map(|path| source_name(path))
        .filter(|name| extension(name) == "src")
    {
        if validate_source(folder, &name).is_ok() {
            validate_source(stage.path(), &name)
                .map_err(|e| format!("{name}: {e}. Choose the complete bundle."))?;
        }
    }
    let backup = parent.join(BACKUP);
    if backup.exists() {
        // recover already checked the marker. Keep the last successful snapshot
        // until another fully validated install is ready to replace it.
        std::fs::remove_dir_all(&backup).map_err(|e| e.to_string())?;
    }
    if folder.exists() {
        std::fs::write(folder.join(MARKER), b"source installer backup v1\n")
            .map_err(|e| e.to_string())?;
        std::fs::rename(folder, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(error) = std::fs::rename(stage.path(), folder) {
        if backup.exists() {
            std::fs::rename(&backup, folder).map_err(|restore| {
                format!(
                    "Install failed ({error}); restore {} to {}: {restore}",
                    backup.display(),
                    folder.display()
                )
            })?;
            let _ = std::fs::remove_file(folder.join(MARKER));
        }
        return Err(format!("Sources were not replaced: {error}"));
    }
    // TempDir now points at the old staging path, so dropping it cannot remove
    // the installed folder. The complete previous folder remains recoverable.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    const SOURCE: &str = "[Settings]=Options.settings\n[Include]=Common.inc\n";
    const INCLUDE: &str =
        "[AlbumUrl]=https://example.invalid/%s\n[ParserScriptAlbum]=...\nSayRest\n";
    const SETTINGS: &str =
        r#"{"key":"example","settings":[{"type":"string","key":"language","default":"en"}]}"#;
    fn collection(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("Shop#&Album.src"), SOURCE).unwrap();
        std::fs::write(path.join("Common.inc"), INCLUDE).unwrap();
        std::fs::write(path.join("Options.settings"), SETTINGS).unwrap();
    }
    fn selected(path: &Path) -> Vec<String> {
        vec![path.to_string_lossy().into_owned()]
    }
    fn zip(path: &Path, entries: &[(&str, &str)]) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, text) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    fn snapshot(folder: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn collect(root: &Path, folder: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    collect(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root).unwrap().into(),
                        std::fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut files = BTreeMap::new();
        collect(folder, folder, &mut files);
        files
    }

    #[test]
    fn capability_preflight_rejects_parser_errors_before_replacing_sources_or_backup() {
        for (section, command, reason) in [
            (
                "ParserScriptIndex",
                "FutureCommand \"x\"",
                "not a known command",
            ),
            (
                "ParserScriptAlbum",
                "FutureCommand \"x\"",
                "not a known command",
            ),
            ("ParserScriptAlbum", "If \"x\"", "no Endif"),
            (
                "ParserScriptAlbum",
                "SayRest unexpected",
                "takes 0 parameters",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let bundle = dir.path().join("download");
            let target = dir.path().join("sources");
            collection(&bundle);
            install(&target, &selected(&bundle)).unwrap();
            std::fs::write(
                target.join("settings.json"),
                r#"{"example":{"language":"de"}}"#,
            )
            .unwrap();
            install(&target, &selected(&bundle)).unwrap();
            let backup = target.parent().unwrap().join(BACKUP);
            let before = snapshot(&target);
            let previous = snapshot(&backup);
            let script = if section == "ParserScriptIndex" {
                format!("[IndexUrl]=https://example.invalid/%s\n[{section}]=...\n{command}\n[ParserScriptAlbum]=...\nSayRest\n")
            } else {
                format!("[MinAppVersionWin]=1.0\n[{section}]=...\n{command}\n")
            };
            std::fs::write(bundle.join("Common.inc"), script).unwrap();
            let error = install(&target, &selected(&bundle)).unwrap_err();
            assert!(
                error.contains(&format!("Common.inc [{section}] line 3 ({command})")),
                "{error}"
            );
            assert!(error.contains(reason), "{error}");
            assert_eq!(snapshot(&target), before);
            assert_eq!(snapshot(&backup), previous);
        }
    }

    #[test]
    fn capability_preflight_rejects_an_include_update_that_breaks_a_working_source() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let before = snapshot(&target);
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("SayRest", "FutureCommand"),
        )
        .unwrap();
        let error = install(&target, &selected(&bundle.join("Common.inc"))).unwrap_err();
        assert!(error.contains("Shop#&Album.src"), "{error}");
        assert!(
            error.contains("Common.inc [ParserScriptAlbum] line 3 (FutureCommand)"),
            "{error}"
        );
        assert_eq!(snapshot(&target), before);
    }

    #[test]
    fn capability_preflight_accepts_supported_scripts_with_unrelated_minimum_versions() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::fs::write(
            bundle.join("Common.inc"),
            format!("[MinAppVersionWin]=999.0\n[MinAppVersionMac]=999.0\n{INCLUDE}"),
        )
        .unwrap();
        install(&target, &selected(&bundle)).unwrap();
        let source = load(&target, "Shop#&Album.src").unwrap();
        assert_eq!(source.min_app_version_win, "999.0");
        assert_eq!(source.min_app_version_mac, "999.0");
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn capability_preflight_keeps_existing_incompatible_sources_without_blocking_an_install() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::fs::create_dir_all(&target).unwrap();
        let old = "[AlbumUrl]=https://example.invalid/%s\n[ParserScriptAlbum]=...\nFutureCommand\n";
        std::fs::write(target.join("Old.src"), old).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Old.src")).unwrap(),
            old
        );
        assert!(target.join("Shop#&Album.src").is_file());
    }
    #[test]
    fn folder_updates_preserve_values_other_files_and_previous_complete_install() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("data/sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let values = br#"{"example":{"language":"de"}}"#;
        std::fs::write(target.join("settings.json"), values).unwrap();
        std::fs::write(target.join("notes.txt"), "local file").unwrap();
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("example.invalid", "changed.invalid"),
        )
        .unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(std::fs::read(target.join("settings.json")).unwrap(), values);
        assert_eq!(
            std::fs::read_to_string(target.join("notes.txt")).unwrap(),
            "local file"
        );
        assert!(
            std::fs::read_to_string(target.parent().unwrap().join(BACKUP).join("Common.inc"))
                .unwrap()
                .contains("example.invalid")
        );
        let listing = super::super::list(&target);
        assert_eq!(listing.sources.len(), 1);
        assert_eq!(listing.sources[0].menu, "Shop#&Album");
        assert!(listing.problems.is_empty());
    }
    #[test]
    fn existing_hidden_directories_survive_an_update() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("data/sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let metadata = target.join(".git/logs/refs/remotes/origin");
        std::fs::create_dir_all(&metadata).unwrap();
        std::fs::write(metadata.join("main"), b"retained local history").unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(
            std::fs::read(metadata.join("main")).unwrap(),
            b"retained local history"
        );
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn existing_empty_directories_survive_consecutive_updates_and_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let directories = [
            "Empty",
            ".git/objects",
            ".git/refs/heads",
            "Cache/Nested/Empty",
        ];
        for name in directories {
            std::fs::create_dir_all(target.join(name)).unwrap();
        }
        for _ in 0..2 {
            install(&target, &selected(&bundle)).unwrap();
            for name in directories {
                assert!(
                    target.join(name).is_dir(),
                    "active directory {name} was lost"
                );
                assert!(
                    dir.path().join(BACKUP).join(name).is_dir(),
                    "snapshot directory {name} was lost"
                );
            }
        }
        std::fs::remove_dir_all(&target).unwrap();
        prepare_folder_open(&target).unwrap();
        for name in directories {
            assert!(
                target.join(name).is_dir(),
                "recovered directory {name} was lost"
            );
        }
    }

    #[test]
    fn existing_empty_directories_retain_the_64_level_bound() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let mut deepest = target.clone();
        for _ in 0..MAX_EXISTING_DEPTH {
            deepest.push("e");
        }
        std::fs::create_dir_all(&deepest).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert!(deepest.is_dir());
        let too_deep = deepest.join("e");
        std::fs::create_dir(&too_deep).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("64 levels deep"));
        assert!(
            too_deep.is_dir(),
            "rejected update must leave existing directories intact"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
    }

    #[test]
    fn existing_empty_directories_cannot_be_replaced_by_incoming_files() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        std::fs::create_dir(target.join("Reserved.src")).unwrap();
        std::fs::write(bundle.join("Reserved.src"), SOURCE).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        assert!(target.join("Reserved.src").is_dir());
        assert!(!dir.path().join(BACKUP).exists());
    }

    #[test]
    fn existing_empty_directories_keep_their_case_for_new_children() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle.join("keptcase"));
        collection(&target);
        std::fs::create_dir(target.join("KeptCase")).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        let names = std::fs::read_dir(&target)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>();
        assert!(names.iter().any(|name| name == "KeptCase"));
        assert!(!names.iter().any(|name| name == "keptcase"));
        assert!(target.join("KeptCase/Shop#&Album.src").is_file());
    }

    #[test]
    fn existing_empty_directories_count_toward_the_combined_entry_limit() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::fs::create_dir(&target).unwrap();
        for index in 0..MAX_ENTRIES {
            std::fs::create_dir(target.join(format!("empty-{index:04}"))).unwrap();
        }
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("4096"));
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), MAX_ENTRIES);
        assert!(!dir.path().join(BACKUP).exists());
    }

    #[cfg(unix)]
    #[test]
    fn native_metadata_paths_do_not_alias_unicode_source_identifiers() {
        use std::os::unix::ffi::OsStringExt;
        let opaque = PathBuf::from(std::ffi::OsString::from_vec(b"Upper-\xff".to_vec()));
        assert!(
            opaque.as_os_str().to_os_string().into_string().is_err(),
            "legacy conversion rejects this native name"
        );
        let mut existing = ExistingTree::default();
        existing.directories.insert(opaque.clone());
        existing
            .files
            .insert(opaque.join("notes.bin"), b"native metadata".to_vec());
        existing.directories.insert(PathBuf::from("Known"));
        existing.files.insert(
            Path::new("Known").join(std::ffi::OsString::from_vec(b"entry-\xfe".to_vec())),
            Vec::new(),
        );
        let incoming = Files::from([
            ("upper-\u{fffd}/Child.inc".into(), b"new source".to_vec()),
            ("known/Child.inc".into(), b"new child".to_vec()),
        ]);
        let prepared = prepare_incoming(&existing, incoming).unwrap();
        assert!(prepared.contains_key("upper-\u{fffd}/Child.inc"));
        assert!(!prepared.contains_key("Upper-\u{fffd}/Child.inc"));
        assert!(prepared.contains_key("Known/Child.inc"));
        assert!(existing.files.contains_key(&opaque.join("notes.bin")));
        assert!(source_name(&opaque).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn native_metadata_paths_remain_in_the_combined_entry_budget() {
        use std::os::unix::ffi::OsStringExt;
        let mut existing = ExistingTree::default();
        for index in 0..MAX_ENTRIES {
            let mut name = format!("metadata-{index}-").into_bytes();
            name.push(0xff);
            existing.files.insert(
                PathBuf::from(std::ffi::OsString::from_vec(name)),
                Vec::new(),
            );
        }
        let incoming = Files::from([("New.inc".into(), b"new source".to_vec())]);
        assert!(prepare_incoming(&existing, incoming)
            .unwrap_err()
            .contains("4096"));
    }

    // APFS rejects these names. Protected Linux CI exercises the real filesystem case.
    #[cfg(unix)]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "requires a Linux filesystem accepting non-UTF-8 names"
    )]
    #[test]
    fn existing_non_unicode_entries_survive_updates_snapshots_and_recovery() {
        use std::os::unix::{ffi::OsStringExt, fs::symlink};
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let first = PathBuf::from(std::ffi::OsString::from_vec(b"metadata-\xff".to_vec()));
        let second = PathBuf::from(std::ffi::OsString::from_vec(b"metadata-\xfe".to_vec()));
        let native_dir = PathBuf::from(std::ffi::OsString::from_vec(b".cache-\xfd".to_vec()));
        let empty = native_dir.join(std::ffi::OsString::from_vec(b"empty-\xfc".to_vec()));
        let nested = native_dir.join(std::ffi::OsString::from_vec(b"value-\xfb".to_vec()));
        std::fs::create_dir_all(target.join(&empty)).unwrap();
        for (name, bytes) in [
            (&first, b"first".as_slice()),
            (&second, b"second"),
            (&nested, b"nested"),
        ] {
            std::fs::write(target.join(name), bytes).unwrap();
        }
        std::fs::write(target.join("metadata-\u{fffd}"), b"Unicode peer").unwrap();
        for _ in 0..2 {
            install(&target, &selected(&bundle)).unwrap();
            for root in [&target, &dir.path().join(BACKUP)] {
                assert_eq!(std::fs::read(root.join(&first)).unwrap(), b"first");
                assert_eq!(std::fs::read(root.join(&second)).unwrap(), b"second");
                assert_eq!(std::fs::read(root.join(&nested)).unwrap(), b"nested");
                assert_eq!(
                    std::fs::read(root.join("metadata-\u{fffd}")).unwrap(),
                    b"Unicode peer"
                );
                assert!(root.join(&empty).is_dir());
            }
        }
        std::fs::remove_dir_all(&target).unwrap();
        prepare_folder_open(&target).unwrap();
        assert_eq!(std::fs::read(target.join(&first)).unwrap(), b"first");
        assert!(target.join(&empty).is_dir());
        let outside = dir.path().join("outside.txt");
        std::fs::write(&outside, b"outside stays private").unwrap();
        let link = std::ffi::OsString::from_vec(b"link-\xfa".to_vec());
        symlink(&outside, target.join(link)).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("symbolic links"));
        assert_eq!(std::fs::read(&outside).unwrap(), b"outside stays private");
    }

    #[cfg(unix)]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "requires a Linux filesystem accepting non-UTF-8 names"
    )]
    #[test]
    fn incoming_non_unicode_source_names_still_reject_without_replacement() {
        use std::os::unix::ffi::OsStringExt;
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        collection(&target);
        let name = std::ffi::OsString::from_vec(b"incoming-\xff.src".to_vec());
        std::fs::write(bundle.join(name), SOURCE).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("non-Unicode"));
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert!(!dir.path().join(BACKUP).exists());
    }

    #[test]
    fn folder_open_restores_an_interrupted_installation_before_creating_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        let backup = dir.path().join(BACKUP);
        collection(&backup);
        std::fs::write(backup.join(MARKER), b"source installer backup v1\n").unwrap();
        prepare_folder_open(&target).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert!(!backup.exists());
    }

    #[test]
    fn folder_open_waits_for_an_installation_swap() {
        use std::sync::mpsc;
        use std::time::Duration;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        let backup = dir.path().join(BACKUP);
        let stage = dir.path().join("stage");
        collection(&target);
        collection(&stage);
        std::fs::write(stage.join("notes.txt"), "new complete installation").unwrap();
        let guard = LOCK.lock().unwrap();
        std::fs::write(target.join(MARKER), b"source installer backup v1\n").unwrap();
        std::fs::rename(&target, &backup).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let folder = target.clone();
        let opener = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let result = prepare_folder_open(&folder);
            done_tx.send(result).unwrap();
        });
        started_rx.recv().unwrap();
        assert!(matches!(
            done_rx.recv_timeout(Duration::from_millis(250)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert!(
            !target.exists(),
            "folder opening must not recreate the swap destination"
        );
        std::fs::rename(&stage, &target).unwrap();
        drop(guard);
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        opener.join().unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("notes.txt")).unwrap(),
            "new complete installation"
        );
        assert!(backup.join("Common.inc").is_file());
    }

    #[test]
    fn second_file_and_dependency_errors_leave_every_installed_file_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::write(bundle.join("Common.inc"), "updated").unwrap();
        std::fs::write(bundle.join("Options.settings"), "invalid JSON").unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        std::fs::write(bundle.join("Options.settings"), SETTINGS).unwrap();
        std::fs::remove_file(bundle.join("Common.inc")).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("complete bundle"));
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
    }
    #[test]
    fn explicit_file_update_reuses_installed_companions_without_weakening_bundle_preflight() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::remove_file(bundle.join("Common.inc")).unwrap();
        std::fs::remove_file(bundle.join("Options.settings")).unwrap();
        install(&target, &selected(&bundle.join("Shop#&Album.src"))).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        std::fs::write(
            bundle.join("Common.inc"),
            format!("{INCLUDE}\n[Settings]=Absent.settings\n"),
        )
        .unwrap();
        assert!(install(&target, &selected(&bundle.join("Common.inc"))).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn nested_zip_preflights_all_companions_and_rejects_duplicate_and_escape_paths() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bundle.zip");
        let target = dir.path().join("sources");
        zip(
            &archive,
            &[
                ("Shop/Album.src", SOURCE),
                ("Shop/Common.inc", INCLUDE),
                ("Shop/Options.settings", SETTINGS),
                (".DS_Store", "ignored download metadata"),
                ("__MACOSX/._Shop", "ignored download metadata"),
            ],
        );
        install(&target, &selected(&archive)).unwrap();
        assert!(target.join("Shop/Album.src").exists());
        for entries in [
            vec![("../outside.src", INCLUDE)],
            vec![("A.src", INCLUDE), ("a.SRC", INCLUDE)],
            vec![("C:/evil.src", INCLUDE)],
            vec![("foo/../../evil.src", INCLUDE)],
        ] {
            zip(&archive, &entries);
            assert!(install(&target, &selected(&archive)).is_err());
            assert!(target.join("Shop/Album.src").exists());
        }
        assert!(!dir.path().join("outside.src").exists());
    }
    #[test]
    fn updates_reuse_installed_directory_spelling_and_preserve_ids() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle.join("Shop"));
        install(&target, &selected(&bundle)).unwrap();
        std::fs::rename(bundle.join("Shop"), bundle.join("shop")).unwrap();
        std::fs::rename(
            bundle.join("shop/Shop#&Album.src"),
            bundle.join("shop/shop#&album.SRC"),
        )
        .unwrap();
        std::fs::write(bundle.join("shop/Extra.inc"), INCLUDE).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert!(target.join("Shop/Extra.inc").is_file());
        let directories = std::fs::read_dir(&target)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(directories, ["Shop"]);
        assert_eq!(
            super::super::list(&target).sources[0].id,
            "Shop/Shop#&Album.src"
        );
    }

    #[test]
    fn interrupted_swap_restores_previous_complete_sources_on_next_read() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        std::fs::remove_dir_all(&target).unwrap();
        assert_eq!(super::super::list(&target).sources.len(), 1);
        assert!(!dir.path().join(BACKUP).exists());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_folder_and_zip_symlinks_and_bounded_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::os::unix::fs::symlink(bundle.join("Common.inc"), bundle.join("Link.inc")).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("symbolic links"));
        std::fs::remove_file(bundle.join("Link.inc")).unwrap();
        std::fs::write(bundle.join("Big.inc"), vec![0; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        let archive = dir.path().join("links.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        writer
            .add_symlink(
                "Link.src",
                "elsewhere",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        assert!(install(&target, &selected(&archive))
            .unwrap_err()
            .contains("symbolic links"));
        assert!(!target.exists());
    }
    #[test]
    #[ignore = "read-only qualification of user-owned local source files; requires OPENTAG_SOURCE_BUNDLE"]
    fn qualify_private_bundle() {
        let bundle = std::env::var("OPENTAG_SOURCE_BUNDLE").expect("OPENTAG_SOURCE_BUNDLE");
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        install(&target, &[bundle]).expect("bundle preflight/install failed");
        let listing = super::super::list(&target);
        assert!(listing.problems.is_empty());
        let mut scripts = 0;
        for source in &listing.sources {
            let file = load(&target, &source.id).unwrap();
            for block in [file.index_script, file.album_script].into_iter().flatten() {
                super::super::engine::Script::compile(&block.lines)
                    .expect("parser compilation failed");
                scripts += 1;
            }
        }
        println!("qualification: {} sources, {} settings schemas, {scripts} parser blocks compile; no provider requests, metadata changes or source text recorded", listing.sources.len(), listing.settings.len());
    }
}
