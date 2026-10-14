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

## Native interaction timings on the real library: 2026-10-14

### Method

`scripts/e2e-perf.sh <folder> [runs]` builds a release-optimized profiling bundle
(`--features e2e`, identifier `org.opentag.editor.e2e.perf`; the feature is off
by default and only enables the script hook), makes a fresh APFS clone of the
folder (`cp -cR`, no extra disk), and launches the app with `/fp:<clone>`.
`tests/e2e/performance.js` then drives the real UI and reports with `PERF` lines.
The source library is never opened or written; saves only touch the clone.

- A fresh clone has no cached file pages, so run 1 approximates a cold read
  (the SSD and directory metadata may still be cached; `purge` needs sudo).
  Run 2 reuses the clone and is warm. Only run 1 performs write tests.
- `sync` is the time for the event handlers plus a forced style and layout.
  `frame` adds the time to the next animation frame, which approximates the
  time until the change is painted.
- IPC durations and payload sizes come from opt-in records in `src/main.ts`
  (`window.__OPENTAG_TIMINGS__`, created only by the script).
- App Nap is disabled for the profiling identifier, and the hook keeps the
  window on top. Hidden windows throttle timers and suspend animation frames.
  A frame that does not arrive within 1 s is counted as `suspended_frames` and
  excluded from the conclusions below.

Corpus: `/Users/jneerdael/Music/Beatport`, 434 FLAC files (17 GB, about 1.6 MB
of embedded artwork each), macOS arm64. Other agents were building at the same
time, so expect a few milliseconds of noise. Baseline is commit `9ac3dd4`
(measurement hooks only); "after" is commit `712827d`. A run 1 repeated after
merging `feat/core-parity` (`3601e62`) matched within noise; its clean 52 ms
end-to-end single-file save is used below. The baseline end-to-end figure is
missing because a suspended frame affected it.

### Results (median; run 1 = fresh clone / run 2 = warm)

| Interaction | Baseline | After | Target |
| --- | ---: | ---: | ---: |
| Launch to rows visible | 1460 / 1111 ms | 1042 / 539 ms | < 1 s warm |
| Launch to all column values | 1526 / 1183 ms | 1105 / 607 ms | |
| `scan_files` IPC (434 files) | 481 / 581 ms | 130 / 80 ms | |
| Row click, sync / frame | 13 / 23 ms | 4 / 10 ms | < 16 ms |
| Arrow down, sync / frame | 13 / 25 ms | 3 / 12 ms | < 16 ms |
| Artwork shown after arrow | 28 ms | 16 ms | |
| Sort by Title, sync / frame | 19 / 40 ms | 17 / 23 ms | < 50 ms |
| Sort by Length (evaluated column), settled | 76 ms | 64 ms | < 50 ms |
| Filter keystroke, sync / frame | 10 / 13 ms | 9 / 12 ms | |
| Select all (434) + tag panel, sync / frame | 19 / 27 ms | 9 / 15 ms | |
| Scroll one page, frame | 17 ms | 17 ms | |
| Edit COMMENT for 434 files, sync | 12 ms | 11 ms | |
| Save COMMENT to 434 files | 43.3 s, no progress | 2.8 to 3.6 s, progress bar | with progress |
| Cancel that save | not available | stops in 72 to 93 ms | |
| Save one file: IPC / until the status shows it | 153 ms / not isolated | 15 to 38 ms / 52 ms | < 100 ms |
| Undo the 434-file save | refused | refused | |

The filter result appears 480 ms after the first keystroke in both versions.
That figure includes the script's 60 ms pauses between keystrokes and the
120 ms debounce. The `filter_matches` call itself takes 33 ms and sends 2.66 MB
of field data per query.

Core benchmarks, release build, fresh clone then warm:

| Benchmark | Before | After |
| --- | ---: | ---: |
| `scan_bench` (434 files) | 326 / 147 ms | 118 / 39 ms |
| `save_bench` per file, serial | 61 ms (40 files) | 13.5 ms (434 files) |
| Disk used by the clone after saving 434 files | about 17 GB (40 files: 1.58 GB) | 0.46 GB |

After the in-place save, every file kept its size, two spot-checked files kept
the same decoded-audio MD5, and `flac -t` passed on both.

### What changed

- **Saving rewrote every audio byte.** lofty shifts the whole FLAC stream
  whenever the metadata size changes and does not use padding. The staging copy
  was also a real copy, because `fs::copy` into an existing temporary file
  cannot clone. Staging copies are now created at a fresh path, so APFS and
  reflink filesystems clone them (`staging_copy`). FLAC Vorbis comment and
  picture changes that fit the original metadata region are written in place
  by resizing the padding (`crates/tag-core/src/flac_metadata.rs`). Larger
  changes fall back to lofty.
- **Save was one IPC call per file, with no progress.** `save_fields_batch`
  writes all pending files on up to eight threads as one undo step. It emits
  `opentag://progress` events, and the status bar shows a progress bar with a
  Cancel button; Esc also cancels. Cancelled files keep their pending changes.
- **Folder reads were serial.** `scan_paths_with_progress` reads files in
  parallel, in input order, with the same progress and cancel support (Open
  folder, Add, Refresh).
- **Every selection change rebuilt the whole window.** `refreshSelection()`
  updates only the row styles, tag panel fields, cover, counts and command
  states. The virtual table restyles existing rows when only the selection
  changes, and keeps its rendered window while the viewport stays inside it.
  The full render is kept for provider documents and the compact layout.

### Remaining bottlenecks and gaps

- **Undo after large saves.** The undo journal refuses operations whose files
  total more than 2 GiB (`Limits::default`), counted at full file length.
  Saving 434 FLACs (17 GB) therefore cannot be undone. On APFS the hard-linked
  snapshots now share all but about 1 MB per file with the saved copy, so the
  cap overstates the real cost here. Changing it is a journal policy decision.
  Mp3tag keeps tag data instead of file snapshots.
- **MP3, MP4 and other formats** still use full-file writes through lofty or
  `id3_v23::save`. Only FLAC writes in place.
- **Sorting and filter changes** still re-render the whole window: about 16 to
  26 ms of script and layout. Sorting by script-evaluated columns (Length,
  Bitrate) waits for `evaluate_columns`.
- **Filtering** sends every file's fields to the backend on each query
  (2.66 MB for 434 files). That is fine at this size but grows linearly with
  the list.
- **Startup**: about 0.7 s of the cold launch passes before the page loads
  (process and WebView start), before any OpenTag code runs.
- Not measured: Mp3tag itself (it does not run on this host), Windows/Linux,
  libraries much larger than 434 files, and a true cold cache.
