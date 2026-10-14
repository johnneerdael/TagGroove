use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("Provide a music directory");
    for iteration in 1..=3 {
        let start = Instant::now();
        let result = tag_core::scan_paths(std::slice::from_ref(&path), true);
        let scan_ms = start.elapsed().as_secs_f64() * 1000.0;
        let image_bytes: usize = result
            .entries
            .iter()
            .flat_map(|e| &e.covers)
            .map(|c| c.data_base64.len())
            .sum();
        let json = Instant::now();
        let bytes = serde_json::to_vec(&result).unwrap();
        println!("iteration={iteration} files={} errors={} scan_ms={scan_ms:.2} serialize_ms={:.2} json_bytes={} cover_base64_bytes={image_bytes}", result.entries.len(), result.errors.len(), json.elapsed().as_secs_f64() * 1000.0, bytes.len());
    }
}
