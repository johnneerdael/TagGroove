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

Status 2026-10-15: round 2 below addressed the MP3/MP4 full-file writes, the full-window re-render on sort and filter, and the per-query filter payload. The undo cap now counts clone-backed snapshots by the bytes they keep ([edit history](verification/edit-history.md)), so the 434-file FLAC save below can be undone (6.4 s in round 2). The list is kept as measured.

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

## Round 2: in-place ID3v2/MP4 writes, filter cache, list refresh (2026-10-15)

Baseline is `5efc322` (`main` after round 1); "after" is `feat/perf-2`. macOS
arm64, APFS, release builds, other agents building at the same time (expect a
few milliseconds and a few percent of noise). Every write test ran on APFS
clones in a scratch directory; `/Users/jneerdael/Music/Beatport` was only read
or cloned.

### Corpora

- **FLAC**: the Beatport folder, 434 files, 17 GB (as in round 1).
- **MP3**: 434 clones of one 4-minute 320 kbit/s LAME file made by ffmpeg
  (`-id3v2_version 3 -write_id3v1 1`, 7 text frames and an 84 KB front cover).
  ffmpeg leaves almost no ID3v2 padding, so the first edit of each file must
  rewrite it.
- **MP3, padded**: the same template after one OpenTag save, i.e. with the 4 KiB
  of padding a rewrite now reserves (as a file previously tagged by Mp3tag or
  OpenTag would have).
- **M4A** and **M4A, padded**: the same for a 256 kbit/s AAC file with
  `-movflags +faststart` (`moov` before `mdat`), 7 text atoms and a cover; ffmpeg
  writes no `free` atom beside `ilst`.

### Core: saving one COMMENT value to 434 files (`save_bench`)

`save_bench <clone> 434 <suffix>`, serial, two consecutive edits that each
change the tag size (edit 1 grows the value, edit 2 shrinks it). Median per
file and total; peak RSS of edit 2 from `/usr/bin/time -l`.

| Corpus | Edit | Before median / total | After median / total | RSS before → after |
| --- | --- | ---: | ---: | ---: |
| MP3 | 1 (first rewrite) | 52.5 ms / 23.6 s | 46.7 ms / 21.3 s | |
| MP3 | 2 | 24.9 ms / 11.0 s | 15.4 ms / 7.6 s | 29 → 31 MB |
| MP3, padded | 1 | 26.0 ms / 13.8 s | 12.3 ms / 6.4 s | |
| MP3, padded | 2 | 43.2 ms / 29.4 s | 12.0 ms / 5.9 s | 30 → 32 MB |
| M4A | 1 (first rewrite) | 23.8 ms / 15.0 s | 17.0 ms / 8.5 s | |
| M4A | 2 | 18.3 ms / 8.6 s | 12.7 ms / 6.1 s | 138 → 25 MB |
| M4A, padded | 1 | 16.1 ms / 7.6 s | 8.7 ms / 4.4 s | |
| M4A, padded | 2 | 18.5 ms / 11.0 s | 8.5 ms / 3.8 s | 179 → 24 MB |
| FLAC (Beatport) | 1 | 16.1 ms / 7.1 s | 14.3 ms / 6.3 s | |
| FLAC (Beatport) | 2 | 15.8 ms / 7.0 s | 16.3 ms / 7.2 s | 224 → 251 MB |

The remaining 9 to 12 ms per file are the clone, the verification read-back,
`fsync` and the rename, not tag I/O. FLAC is unchanged (it was already in
place). The real gain is in the bytes written: an in-place edit rewrites the
tag region (about 4 to 110 KB here) instead of the whole 6 to 10 MB file, and
lofty no longer reads whole MP4 files into memory (peak RSS 138 to 179 MB →
24 MB).

### Undo of a 434-file save (`journal_bench`)

`journal_bench <clone>` saves COMMENT to every file as one undo step through
the real journal (`Limits::default`, 2 GiB):

| Corpus (4.2 GB / 2.7 GB) | Before | After |
| --- | --- | --- |
| MP3, padded | refused: too large to undo (17.9 s) | undoable, 46 MB retained (6.1 s) |
| M4A, padded | refused: too large to undo (9.4 s) | undoable, 46 MB retained (4.9 s) |

The first save of an unpadded file is still a full rewrite and still counts
the whole file.

### Filter query payload (`filter_cache_bench`, windowless)

`OPENTAG_FILTER_BENCH=<folder> cargo test --release -p opentag
filter_cache_bench -- --ignored --nocapture` on the Beatport folder (read only)
builds the request the window sends, with the 11 default column templates:

| Request | Bytes | Backend time |
| --- | ---: | ---: |
| Before: every query (fields, raw fields, column values of 434 files) | 1,788,771 | |
| After: first query of a session (all files once) | 1,391,822 | 13.5 ms |
| After: any later query with unchanged files | 757 | 1.1 ms (median of 20) |

The in-app figure for the old request was 2.66 MB (the window's filter fields
include more technical values than this approximation). After a save, undo or
pending edit, the next query carries only the changed files.

### Native UI timings

Measured with `scripts/e2e-perf.sh` before local GUI runs were stopped
(release profiling build, run 1 / run 2 medians where both exist). Later
native runs, including the second-filter-query step now in
`tests/e2e/performance.js` (`filter_requery`), were **not measured**; they run
only in CI from now on.

| Interaction (Beatport FLAC) | Before | After |
| --- | ---: | ---: |
| Sort by Title, sync / frame | 17–20 / 35–39 ms | 10–11 / 14–27 ms |
| Sort by Length (evaluated column), sync / frame | 28–32 / 49–59 ms, plus an `evaluate_columns` call of 1.4 MB | 11–12 / 17–19 ms, no call |
| Filter keystroke, sync / frame | 10 / 13 ms | 2 / 3–4 ms |
| First filter query, request bytes / call | 2.67 MB / 40–55 ms | 1.73 MB / 28–29 ms |
| Save COMMENT to 434 files | 4.3 s | 3.9 s |
| Undo that save | 11.9 s | 6.4 s |

On the generated MP3 and M4A corpora sort sync time fell from 11–13 ms to
8–10 ms and filter keystrokes from 6–7 ms to 2 ms. Saving 434 padded MP3s in
the app took 2.6 s and could be undone (4.4 s); the unpadded MP3 save took
9.7 s in both versions and was refused for undo in both. The padded baselines
in the app and the padded M4A run were **not measured**.

### What changed

- **ID3v2 in place** (`crates/tag-core/src/id3_padding.rs`). When the new
  ID3v2.3 or v2.4 tag fits the existing tag plus its padding, it is rendered
  with exactly the padding that keeps its size and written over the old tag in
  the staging clone: at the start of MPEG/AAC streams (including a v2.4
  footer) and in the single `ID3 ` chunk of WAV/AIFF. An existing ID3v1 tag is
  overwritten in its 128 bytes. Rewrites that do not fit reserve 4 KiB of
  padding. Mp3tag documents no ID3v2 padding option
  (`customization/options/tags/advanced.html`); 4 KiB matches the padding its
  Utils → Optimize FLAC reserves.
- **MP4 in place** (`crates/tag-core/src/mp4_metadata.rs`). lofty encodes the
  new `ilst` against an in-memory copy of `ftyp` and `moov` only. If it fits
  the old `ilst` plus the `free` atoms beside it in `moov.udta.meta`, it is
  written there and the rest becomes a `free` atom, so `moov` keeps its size
  and no chunk offset changes. Otherwise the new `ilst` and a 4 KiB `free`
  atom are spliced into `moov`, `stco`/`co64` offsets past `moov` are shifted,
  and everything after `moov` is written once. Files without an `ilst`,
  fragmented files and 64-bit atom sizes keep lofty's full write.
- Both report their region to the undo journal like the FLAC writer, and all
  writes still go to the staging clone that is renamed over the original
  (tests in `crates/tag-core/tests/in_place_tags.rs`, including a hard-linked
  original and journal accounting).
- **Filter cache** (`src-tauri/src/filter_cache.rs`, `src/filter-sync.ts`). The
  backend keeps each window session's filter fields, list order and column
  snapshots. A query sends only files whose entry, pending edits, source state
  or evaluated column values changed (compared by identity), and the order
  only when it changed. A backend without a listed file answers with a marker
  and the window resends everything; expression errors keep the cache.
  Requests are serialized so cache updates apply in order.
- **List refresh** (`refreshList()` in `src/main.ts`). Sort and filter changes
  update the header sort indicators, virtual rows, empty state, counts and
  command states instead of rebuilding the window; the filter input keeps its
  focus without being replaced. Sort values of all shown columns are evaluated
  with the cells, so sorting by Length or Bitrate needs no new evaluation.

### Remaining bottlenecks and gaps

- The first save of a file without padding (fresh rips, ffmpeg output) is
  still a full rewrite and counts in full against the undo cap.
- In-place regions keep their size: removing a large cover leaves its space as
  padding (as for FLAC). APE tags, RIFF INFO, Ogg, and the other container
  formats still rewrite whole files.
- Per-file save time is now dominated by clone, verification read-back,
  `fsync` and rename (about 9 to 12 ms).
- Saving one FLAC right after undoing the 434-file save took 1.3 to 2.0 s in
  both versions (54 ms on the other corpora); not investigated.
- Native UI numbers after the last changes, and Windows/Linux, are not
  measured.

## Library (2026-10-15)

Options → Library stores list entries in SQLite and loads unchanged files without reading them. The only measurement so far is a debug build on 3000 small generated WAV files (warm disk cache): plain scan 394 ms, first scan through the library 620 ms, second scan 281 ms with every file from the library ([library](verification/library.md)). These files are nearly free to parse, so this is the smallest possible saving. Release builds, real collections, Windows/Linux and comparison with Mp3tag's own library were not measured.
