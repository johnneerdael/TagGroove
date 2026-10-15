//! Saves one COMMENT value into every audio file of a folder as one undo step
//! and reports what the undo journal retains. It modifies files: point it at a
//! disposable copy (on APFS, `cp -cR <folder> <copy>` costs no extra disk). The
//! journal lives in `<folder>/../.journal-bench`, on the same volume.
use std::{path::Path, time::Instant};
use tag_core::history::{Journal, Limits};

fn main() {
    let mut args = std::env::args().skip(1);
    let folder = args.next().expect("Provide a disposable music directory");
    let suffix = args.next().unwrap_or_default();
    let scan = tag_core::scan_paths(std::slice::from_ref(&folder), true);
    let root = Path::new(&folder)
        .canonicalize()
        .expect("folder")
        .with_file_name(".journal-bench");
    let _ = std::fs::remove_dir_all(&root);
    let mut journal = Journal::open(&root, Limits::default()).expect("journal");
    let paths: Vec<String> = scan.entries.iter().map(|e| e.path.clone()).collect();
    let started = Instant::now();
    let capture = journal.begin("Save tags", None, &paths);
    for (index, path) in paths.iter().enumerate() {
        let updates = [(
            "COMMENT".to_string(),
            vec![format!("journal_bench {index}{suffix}")],
        )]
        .into_iter()
        .collect();
        tag_core::write_fields_summary(Path::new(path), &updates).expect("write");
    }
    journal.commit(capture, &[], &[]);
    let state = journal.state();
    println!(
        "files={} total_ms={:.1} undo={} retained_bytes={} notice={:?}",
        paths.len(),
        started.elapsed().as_secs_f64() * 1000.0,
        state.undo.is_some(),
        journal.retained_bytes(),
        state.notice
    );
    drop(journal);
    let _ = std::fs::remove_dir_all(&root);
}
