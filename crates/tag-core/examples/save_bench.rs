//! Writes one COMMENT value into the first `count` audio files of a folder and
//! reports the time per file. It modifies files: point it at a disposable copy
//! (on APFS, `cp -cR <folder> <copy>` costs no extra disk).
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("Provide a disposable music directory");
    let count: usize = args
        .next()
        .map_or(usize::MAX, |n| n.parse().expect("count"));
    let scan = tag_core::scan_paths(std::slice::from_ref(&path), true);
    let mut times = Vec::new();
    let started = Instant::now();
    for entry in scan.entries.iter().take(count) {
        let updates = [(
            "COMMENT".to_string(),
            vec![format!("save_bench {}", times.len())],
        )]
        .into_iter()
        .collect();
        let file = Instant::now();
        tag_core::write_fields_summary(std::path::Path::new(&entry.path), &updates).expect("write");
        times.push(file.elapsed().as_secs_f64() * 1000.0);
    }
    let total = started.elapsed().as_secs_f64() * 1000.0;
    times.sort_by(f64::total_cmp);
    println!(
        "files={} total_ms={total:.1} median_ms={:.2} p95_ms={:.2} max_ms={:.2}",
        times.len(),
        times[times.len() / 2],
        times[times.len() * 95 / 100],
        times[times.len() - 1]
    );
}
