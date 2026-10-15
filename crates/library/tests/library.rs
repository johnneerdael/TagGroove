use opentag_library::{fingerprint, CleanupReport, Library, Scope, Session, Stamp};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use tag_core::{
    parallel::{Progress, Silent},
    scan_paths_cached, scan_paths_with_options, AudioEntry, Fields, ScanResult, TagOptions,
};

/// A short PCM WAV file with a RIFF INFO title and artist.
fn wav(path: &Path, title: &str, artist: &str) {
    let mut info = b"INFO".to_vec();
    for (id, value) in [(b"INAM", title), (b"IART", artist)] {
        let mut data = value.as_bytes().to_vec();
        data.push(0);
        info.extend(id);
        info.extend((data.len() as u32).to_le_bytes());
        if data.len() % 2 == 1 {
            data.push(0);
        }
        info.extend(data);
    }
    let samples = [0u8; 64];
    let mut body = b"WAVEfmt ".to_vec();
    body.extend(16u32.to_le_bytes());
    body.extend(1u16.to_le_bytes());
    body.extend(1u16.to_le_bytes());
    body.extend(8000u32.to_le_bytes());
    body.extend(16000u32.to_le_bytes());
    body.extend(2u16.to_le_bytes());
    body.extend(16u16.to_le_bytes());
    body.extend(b"data");
    body.extend((samples.len() as u32).to_le_bytes());
    body.extend(samples);
    body.extend(b"LIST");
    body.extend((info.len() as u32).to_le_bytes());
    body.extend(info);
    let mut file = b"RIFF".to_vec();
    file.extend((body.len() as u32).to_le_bytes());
    file.extend(body);
    fs::write(path, file).unwrap();
}

/// `count` files spread over album folders of `root`.
fn collection(root: &Path, count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|n| {
            let album = root.join(format!("Artist {}/Album {}", n / 200, n / 20));
            fs::create_dir_all(&album).unwrap();
            let path = album.join(format!("{:05} track.wav", n));
            wav(&path, &format!("Title {n}"), &format!("Artist {}", n / 200));
            path
        })
        .collect()
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn scan(
    library: &Library,
    scope: &Scope,
    options: &TagOptions,
    roots: &[String],
    refresh: bool,
) -> (ScanResult, usize, usize) {
    let session = Session::new(library, scope, fingerprint(options, "test"), refresh);
    let result = scan_paths_cached(roots, true, options, None, &Silent, Some(&session));
    (result, session.hits(), session.stored())
}

fn title(entry: &AudioEntry) -> &str {
    &entry.fields["TITLE"][0]
}

fn json(entries: &[AudioEntry]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect()
}

#[test]
fn unchanged_files_come_from_the_library_and_match_a_fresh_read() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    collection(&music, 40);
    let library = Library::open(&dir.path().join("config/library/library.db")).unwrap();
    let (options, scope, roots) = (TagOptions::default(), Scope::everything(), [text(&music)]);
    let (first, hits, stored) = scan(&library, &scope, &options, &roots, false);
    assert_eq!((first.entries.len(), hits, stored), (40, 0, 40));
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    let (second, hits, stored) = scan(&library, &scope, &options, &roots, false);
    assert_eq!((hits, stored), (40, 0));
    assert_eq!(json(&second.entries), json(&first.entries));
    let plain = scan_paths_with_options(&roots, true, &options, None);
    assert_eq!(json(&second.entries), json(&plain.entries));
    assert_eq!(title(&second.entries[7]), "Title 7");
    assert_eq!(library.stats().unwrap().files, 40);
    assert!(library.stats().unwrap().bytes > 0);
}

#[test]
fn changed_files_are_read_again_and_replace_their_entry() {
    let dir = tempfile::tempdir().unwrap();
    let files = collection(&dir.path().join("music"), 5);
    let library = Library::in_memory().unwrap();
    let (options, scope) = (TagOptions::default(), Scope::everything());
    let roots = [text(&dir.path().join("music"))];
    scan(&library, &scope, &options, &roots, false);
    // Writing a longer title changes the size and the modification time.
    let mut updates = Fields::new();
    updates.insert(
        "TITLE".into(),
        vec!["A much longer replacement title".into()],
    );
    tag_core::write_fields_summary(&files[2], &updates).unwrap();
    let (result, hits, stored) = scan(&library, &scope, &options, &roots, false);
    assert_eq!((hits, stored), (4, 1));
    assert_eq!(title(&result.entries[2]), "A much longer replacement title");
    // Only the modification time changes.
    let modified = fs::metadata(&files[3]).unwrap().modified().unwrap();
    fs::File::options()
        .write(true)
        .open(&files[3])
        .unwrap()
        .set_modified(modified + Duration::from_secs(5))
        .unwrap();
    let (_, hits, stored) = scan(&library, &scope, &options, &roots, false);
    assert_eq!((hits, stored), (4, 1));
}

#[test]
fn other_read_options_and_refresh_read_files_again() {
    let dir = tempfile::tempdir().unwrap();
    collection(&dir.path().join("music"), 6);
    let library = Library::in_memory().unwrap();
    let scope = Scope::everything();
    let roots = [text(&dir.path().join("music"))];
    scan(&library, &scope, &TagOptions::default(), &roots, false);
    let mut other = TagOptions::default();
    other.advanced.popularimeter_ratings = true;
    assert_ne!(
        fingerprint(&other, "test"),
        fingerprint(&TagOptions::default(), "test")
    );
    assert_ne!(
        fingerprint(&TagOptions::default(), "1.0"),
        fingerprint(&TagOptions::default(), "1.1")
    );
    let (_, hits, stored) = scan(&library, &scope, &other, &roots, false);
    assert_eq!((hits, stored), (0, 6));
    // File → Reload tags reads every file and keeps the result.
    let (result, hits, stored) = scan(&library, &scope, &other, &roots, true);
    assert_eq!((result.entries.len(), hits, stored), (6, 0, 6));
    let (_, hits, _) = scan(&library, &scope, &other, &roots, false);
    assert_eq!(hits, 6);
}

#[test]
fn only_included_directories_are_stored() {
    let dir = tempfile::tempdir().unwrap();
    let (kept, other) = (dir.path().join("kept"), dir.path().join("kept-not"));
    collection(&kept, 3);
    collection(&other, 4);
    let library = Library::in_memory().unwrap();
    let options = TagOptions::default();
    let scope = Scope::only(&[text(&kept), "  ".into()]);
    assert!(scope.contains(&dunce::canonicalize(&kept).unwrap().join("x/y.wav")));
    assert!(!scope.contains(&dunce::canonicalize(&other).unwrap().join("y.wav")));
    let roots = [text(&kept), text(&other)];
    let (result, hits, stored) = scan(&library, &scope, &options, &roots, false);
    assert_eq!((result.entries.len(), hits, stored), (7, 0, 3));
    let (_, hits, stored) = scan(&library, &scope, &options, &roots, false);
    assert_eq!((hits, stored), (3, 0));
    let nothing = Scope::only::<&str>(&[]);
    let (_, hits, stored) = scan(&library, &nothing, &options, &roots, false);
    assert_eq!((hits, stored), (0, 0));
}

struct StopAfter(AtomicUsize, usize, AtomicBool);
impl Progress for StopAfter {
    fn proceed(&self) -> bool {
        !self.2.load(Ordering::SeqCst)
    }
    fn advance(&self) {
        if self.0.fetch_add(1, Ordering::SeqCst) + 1 >= self.1 {
            self.2.store(true, Ordering::SeqCst);
        }
    }
}

#[test]
fn cleanup_removes_missing_and_excluded_files_and_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    let files_a = collection(&a, 5);
    collection(&b, 5);
    let library = Library::in_memory().unwrap();
    let options = TagOptions::default();
    scan(
        &library,
        &Scope::everything(),
        &options,
        &[text(&a), text(&b)],
        false,
    );
    fs::remove_file(&files_a[0]).unwrap();
    fs::remove_file(&files_a[1]).unwrap();
    let report = library.cleanup(&Scope::everything(), &Silent).unwrap();
    assert_eq!(
        report,
        CleanupReport {
            checked: 10,
            removed: 2,
            cancelled: false
        }
    );
    assert_eq!(library.stats().unwrap().files, 8);
    // Cleanup non-library files.
    let report = library.cleanup(&Scope::only(&[text(&a)]), &Silent).unwrap();
    assert_eq!((report.checked, report.removed), (8, 5));
    assert_eq!(library.stats().unwrap().files, 3);
    scan(&library, &Scope::everything(), &options, &[text(&b)], false);
    let stop = StopAfter(AtomicUsize::new(0), 2, AtomicBool::new(false));
    let report = library.cleanup(&Scope::only(&[text(&a)]), &stop).unwrap();
    assert!(report.cancelled, "{report:?}");
    assert!(report.checked < 8 && report.removed <= report.checked);
    library.clear().unwrap();
    assert_eq!(library.stats().unwrap().files, 0);
}

#[test]
fn moved_and_deleted_files_are_forgotten() {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let library = Library::in_memory().unwrap();
    let sample = root.join("sample.wav");
    wav(&sample, "t", "a");
    let read = tag_core::read_entry_summary(&sample).unwrap();
    let entry = |path: &Path| {
        let mut entry = read.clone();
        entry.path = text(path);
        entry
    };
    let stamp = Stamp {
        modified: 1,
        size: 2,
    };
    let paths = [
        root.join("album/1.wav"),
        root.join("album/sub/2.wav"),
        root.join("album-2/3.wav"),
        root.join("album.x/4.wav"),
        root.join("album0/5.wav"),
        root.join("other/6.wav"),
    ];
    let entries: Vec<AudioEntry> = paths.iter().map(|p| entry(p)).collect();
    let rows: Vec<(&AudioEntry, Stamp)> = entries.iter().map(|e| (e, stamp)).collect();
    library.store(&rows, "f").unwrap();
    assert_eq!(library.forget_below(&text(&root.join("album"))).unwrap(), 2);
    assert_eq!(
        library.forget(&[text(&paths[5]), text(&paths[5])]).unwrap(),
        1
    );
    let keys: Vec<String> = paths.iter().map(|p| text(p)).collect();
    let found = library
        .lookup(
            &keys.iter().map(|k| (k.as_str(), stamp)).collect::<Vec<_>>(),
            "f",
        )
        .unwrap();
    assert_eq!(
        found.iter().map(Option::is_some).collect::<Vec<_>>(),
        [false, false, true, true, true, false]
    );
    // Windows separators.
    let windows = |p: &str| {
        let mut e = entries[0].clone();
        e.path = p.into();
        e
    };
    let (w1, w2) = (windows(r"C:\Music\A\1.wav"), windows(r"C:\Music\AB\1.wav"));
    library.store(&[(&w1, stamp), (&w2, stamp)], "f").unwrap();
    assert_eq!(library.forget_below(r"C:\Music\A\").unwrap(), 1);
    // Another fingerprint or stamp is not a match.
    let key = [(keys[2].as_str(), stamp)];
    assert!(library.lookup(&key, "g").unwrap()[0].is_none());
    let moved = [(
        keys[2].as_str(),
        Stamp {
            modified: 1,
            size: 3,
        },
    )];
    assert!(library.lookup(&moved, "f").unwrap()[0].is_none());
}

#[test]
fn unusable_or_outdated_database_files_are_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.db");
    fs::write(&path, b"this is not an SQLite database, just some text....").unwrap();
    let library = Library::open(&path).unwrap();
    assert_eq!(library.stats().unwrap().files, 0);
    collection(&dir.path().join("m"), 2);
    let options = TagOptions::default();
    scan(
        &library,
        &Scope::everything(),
        &options,
        &[text(&dir.path().join("m"))],
        false,
    );
    drop(library);
    let reopened = Library::open(&path).unwrap();
    assert_eq!(reopened.stats().unwrap().files, 2);
    drop(reopened);
    // A future schema is not understood; the library starts again.
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 99)
        .unwrap();
    assert_eq!(Library::open(&path).unwrap().stats().unwrap().files, 0);
}

/// Several thousand small files: the second load takes every file from the
/// library. Timings are printed for docs/verification/library.md.
#[test]
fn a_large_collection_loads_from_the_library() {
    let count: usize = std::env::var("OPENTAG_LIBRARY_FILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    let started = Instant::now();
    collection(&music, count);
    let generated = started.elapsed();
    let library = Library::open(&dir.path().join("library.db")).unwrap();
    let (options, scope, roots) = (TagOptions::default(), Scope::everything(), [text(&music)]);
    let started = Instant::now();
    let plain = scan_paths_with_options(&roots, true, &options, None);
    let uncached = started.elapsed();
    let started = Instant::now();
    let (cold, hits, stored) = scan(&library, &scope, &options, &roots, false);
    let cold_time = started.elapsed();
    assert_eq!((cold.entries.len(), hits, stored), (count, 0, count));
    let started = Instant::now();
    let (warm, hits, stored) = scan(&library, &scope, &options, &roots, false);
    let warm_time = started.elapsed();
    assert_eq!((hits, stored), (count, 0));
    assert_eq!(json(&warm.entries), json(&plain.entries));
    let started = Instant::now();
    let report = library.cleanup(&scope, &Silent).unwrap();
    let cleanup = started.elapsed();
    assert_eq!(report.removed, 0);
    let stats = library.stats().unwrap();
    eprintln!(
        "library: {count} files generated in {generated:?}; plain scan {uncached:?}; \
         first library scan {cold_time:?}; second {warm_time:?}; cleanup {cleanup:?}; \
         database {} KiB",
        stats.bytes / 1024
    );
}
