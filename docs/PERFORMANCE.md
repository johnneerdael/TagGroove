# Performance evidence

Performance parity with Mp3tag 3.36.1 is not demonstrated. Retain measurements of the reconstruction separately from original-app comparisons.

## Baseline: 2026-10-14

Read-only scan of the user-selected Beatport directory: 434 FLAC files, 0 errors. `cargo build -p tag-core --example scan_bench`, then `/usr/bin/time -l target/debug/examples/scan_bench /Users/jneerdael/Music/Beatport`. This is an unoptimized Rust debug build on this macOS host, after the folder had already been read by the desktop app; it is not a controlled cold-cache measurement.

| Run | Scan ms | JSON serialization ms | JSON bytes | Cover base64 bytes |
| --- | ---: | ---: | ---: | ---: |
| 1 | 18375.90 | 37679.91 | 740914575 | 740379636 |
| 2 | 20255.74 | 36200.14 | 740914575 | 740379636 |
| 3 | 23275.61 | 34230.37 | 740914575 | 740379636 |

Process maximum resident set size: 1,240,891,392 bytes. Peak memory footprint reported by macOS time: 1,605,420,856 bytes. Median scan plus serialization: about 56.5 seconds. These measure the core and serialization, excluding actual Tauri transport and UI rendering, so user-visible load can take longer.

The payload is dominated by eagerly encoded artwork: 99.93 percent of JSON bytes. A native stack sample was recorded at `/tmp/opentag-scan.sample.txt`; the available symbolization was insufficient to attribute a detailed CPU breakdown. Source inspection also finds a second concrete FLAC metadata read used to recover custom fields.

## Changes being measured

- Summary file entries without artwork payloads; retain descriptors and retrieve active-file covers separately.
- Read FLAC metadata once for summary loading while preserving native custom fields.
- Virtualize table rows; cache order/filter keys; avoid replacing the full workspace for cover hydration.
- Compile filters and scripting expressions once per query.

Repeat the same benchmark after these changes, then measure release builds and actual UI latency. Original Windows reference load, sorting/filtering/editing latency and memory still require a comparable runtime and corpus. A faster reconstruction benchmark alone will not prove performance parity.

## Summary loading: 2026-10-14

Same command, host, debug build and 434-file directory after summary/lazy-artwork changes. 0 errors in all runs. All entries retain their fields and artwork descriptors, with payloads fetched only for the active selection.

| Run | Scan ms | JSON serialization ms | JSON bytes | Cover base64 bytes |
| --- | ---: | ---: | ---: | ---: |
| 1 | 6912.46 | 22.43 | 534939 | 0 |
| 2 | 7160.18 | 71.57 | 534939 | 0 |
| 3 | 7062.51 | 19.64 | 534939 | 0 |

Maximum resident set size: 151,633,920 bytes. IPC input fell from 740.9 MB to 0.535 MB (about 1,385 times smaller). Median scan plus serialization fell from roughly 56.5 seconds to 7.08 seconds (about 8 times faster). This is a measured core improvement, not a claim of original-app or actual device/UI performance parity.

## Optimized core with measured technical properties: 2026-10-14

Built `tag-core`'s existing `scan_bench` example with `--release --locked` using
Rust 1.95.0 and isolated target `target/parity-performance`, then ran
`/usr/bin/time -l target/parity-performance/release/examples/scan_bench /Users/jneerdael/Music/Beatport`.
The benchmark only reads files. This is the current uncommitted parity-core
source with actual technical-property maps, not the older debug-build source.
Other development processes were active; filesystem caches were not flushed.

| Run | Files | Errors | Scan ms | Serialization ms | JSON bytes | Cover base64 bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 434 | 0 | 469.41 | 1.01 | 1,073,982 | 0 |
| 2 | 434 | 0 | 159.83 | 0.94 | 1,073,982 | 0 |
| 3 | 434 | 0 | 157.08 | 0.95 | 1,073,982 | 0 |

Maximum resident set size was 137,265,152 bytes (130.9 MiB). The median scan plus
serialization was 160.77 ms in this warm-cache sequence. Retain these figures as
an optimized core baseline: differing source and optimization settings prevent
attributing the difference from earlier debug results to a single code change.
Tauri transport, rendering, interaction latency, original-Mp3tag comparison,
cold-cache behavior and other platforms/devices remain unmeasured here.

Raw output, resource counters, exact core-source hashes, compiler version and
benchmark executable SHA-256 are recorded in
`/tmp/opentag-parity-release-scan.json`, with the two corresponding `.log` files.
