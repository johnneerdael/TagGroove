# Options → Library

Scope: Options → Library (Enable Library, Only include these directories, Cleanup Library) and the metadata database behind it. Reference: `customization/options/library`, the v2.86 release note that introduced the feature, the v3.17 note on force-reloading and starting with a fresh library, and the Library strings of Mp3tag 3.36.1 (`_C_OPT_LIB*`, `_M_STR_SB_CLEANUPDATABASE*`).

## What the documentation describes

- With the library enabled, metadata is read from files once and stored in a database in the configuration folder. Later reads of the same file take the stored data, and the entry is updated when the file's modification time changed.
- Only include these directories: files outside the listed directories are not stored permanently.
- Mp3tag keeps the database current when files are deleted or moved inside Mp3tag. Cleanup Library checks whether the files of stored entries still exist and removes the entries it cannot access. The status text of 3.36.1 also has a "Cleanup non-library files" stage.
- File → Read Tag (Ctrl+T) force-reloads tags; "start with a fresh library" is the other remedy when the stored data is out of date.
- The documentation describes no separate browser, query or search over the library. The library is a cache behind loading folders, directories and playlists, so OpenTag adds none either.

## Implemented

**Store** (`crates/library`, package `opentag-library`)
- SQLite through `rusqlite` with the `bundled` feature, so every platform (Windows, macOS, Linux, Android) gets the same engine without a system library. Mp3tag 3.36.1 links SQLite as well. SQLite gives atomic batched writes, a crash-safe write-ahead log, indexed lookups by path and range deletes for moved folders. A key-value store (redb, sled) would have needed hand-written range scans and gives no ad-hoc inspection.
- Location: `<app config dir>/library/library.db` (plus `-wal`/`-shm`), next to Mp3tag's own `library\` folder layout. It is not part of File → Save configuration: the backup bundle is an allow-list of JSON files (`utilities.rs` `CONFIGURATION`), and a test checks the database stays out.
- One row per file: canonical path (primary key), modification time in nanoseconds, size, a fingerprint, and the list entry as JSON. The fingerprint hashes the Options → Tags values (read flags, mappings, Matroska mapping, ratings …), the app version and an entry-format constant, so changing a read option or upgrading reads files again instead of showing stale fields.
- A stored entry is used only when the modification time, size and fingerprint all match; anything else reads the file and replaces the row. Mp3tag checks the modification time; OpenTag also compares the size.
- Cover art is not stored. OpenTag's list entries already carry only image descriptors (count, size, type); image data is read from the file when the cover pane or a cover dialog needs it. This meets the documented aim (no binary data in memory) without duplicating artwork into the database.
- Lookups are batched: one stat per file (in parallel), one prepared query per path under a single lock, JSON parsed in parallel outside the lock. New entries are written in 500-row transactions. A file that changed between its stat and the end of its read is not stored.
- A database that is not SQLite or is corrupt is deleted and recreated; another schema version is dropped and rebuilt (it only holds data that can be read again). If the library cannot be opened, loading reads the files as before.

**Loading** (`tag_core::scan_paths_cached`, `EntryCache`; `src-tauri/src/library.rs`)
- `scan_files` (open/add directory, add files, drag and drop, startup folder, command line, playlists, View → Refresh, re-reads after changing tag options) goes through the library when it is enabled. Unchanged files count towards the existing progress bar immediately; only the rest are read, in parallel as before, and cancellation keeps working.
- Files outside the included directories (with the restriction on) are neither looked up nor stored. An empty list with the restriction on stores nothing; the page says so.
- File → Reload tags (⌘/Ctrl T) passes `refresh`, which reads every selected file and replaces its row.
- Provider documents (Android) and chapter/cue sheet row paths are never stored; chapter rows are expanded from the stored file entry as from a read one.

**Keeping it current**
- Every command that writes audio files runs through `history::record` (save, batch save, paste, actions, conversions, rename, covers, remove tags, utils, tag source import). After the work, successful or not, the written files, both sides of each rename and both sides of each moved folder are removed from the library, so the next load reads them. This also covers Options → Files → Preserve timestamps, where a write keeps the modification time (and in-place writes keep the size).
- Undo and redo, Delete, Delete permanently and Move files do the same.
- This runs whether or not the library is enabled, as long as the database exists, so re-enabling it later never serves entries for files OpenTag changed in the meantime.

**Options → Library page** (`src/library.ts`, between Appearance and Tags as in Mp3tag)
- Enable Library (off by default).
- Only include these directories, with a directory list, Add… (folder picker, several at once, duplicates skipped) and Remove. The list is disabled while the checkbox is off.
- Cleanup Library: runs in the background with "Processing files … n of m" and Cancel. It removes entries whose file is missing or not a regular file and, when the restriction is on, entries outside the included directories (Mp3tag's "Cleanup non-library files"). It uses the values shown in the dialog. Entries checked before a cancel are still cleaned. It does not take the file-operation lock, so loading and saving are not held up.
- Clear Library… with an inline confirmation: removes every entry and compacts the file ("start with a fresh library").
- A status line: number of stored files, database size, and how many files the last load took from the library versus read and stored.
- `pages.library` is validated in `settings.rs` (unknown keys and empty directory names are rejected).

## Tests

- `crates/library/tests/library.rs` (8): a second scan takes every file from the library and equals a plain read; a changed title (size and time) and a time-only change are read again; other read options and the reader version miss, refresh reads and stores; only included directories are stored (`kept` vs `kept-not`, empty list stores nothing); Cleanup removes missing files, then non-library files, and stops on cancel; Clear; forgetting files and folders (including `album-2`, `album.x`, `album0` siblings and Windows separators); a text file in place of the database and a newer schema are replaced.
- Large collection: `a_large_collection_loads_from_the_library` generates 3000 small WAV files with RIFF INFO tags in 150 album folders (`OPENTAG_LIBRARY_FILES` changes the count). Debug build on an Apple Silicon MacBook (APFS, warm disk cache): plain scan 394 ms, first scan through the library 620 ms (reads and stores everything), second scan 281 ms with 3000 of 3000 files from the library, Cleanup 4 ms, database 8.5 MiB (about 2.9 KiB per file). A breakdown of the second scan: 4 ms for 3000 stats and 28 ms for the lookups including JSON parsing; the rest is the directory walk and path canonicalisation that every load does. These files take almost no time to parse, so the saving here is the floor: real files with large ID3v2/Vorbis tags, artwork and slow or network disks spend far more time per read, which the library skips.
- `src-tauri/src/library.rs` (3): option parsing and validation; loading through the library with hit/store counters and refresh; history-recorded saves (also failed ones and chapter rows), renames and folder moves forget their files; configuration backups leave `library/library.db` out.
- `tests/library.test.mjs` (4): option fallbacks, adding directories without duplicates, status and cleanup texts, page placement, controls and markup.
- Native e2e `tests/e2e/library.js` (fixtures `library-fixtures.sh`), in the CI `native-e2e` list for Linux, macOS and Windows: page order and controls; enable through the dialog; first refresh stores every file and the second takes all from the library; a folder outside the album is stored while unrestricted; a tag panel save with Preserve timestamps on is read again on the next refresh; undo likewise; Reload tags reads and stores the selected file; Only include + Add… + Cleanup Library in the dialog removes the other folder's entry and later scans of it store nothing; Delete permanently removes the file's entry; Clear Library empties the database.

## Decisions

- No library browser, query view or collection search: the documentation describes none, and parity comes first.
- No background crawler of the included directories: Mp3tag fills the library as files are loaded, and so does OpenTag. Loading already runs off the UI thread with progress and cancel.
- Stale entries are forgotten rather than rewritten after a write; the next load reads the file once and stores it. This keeps one code path for every write command and never stores an entry that differs from a fresh read.
- The fingerprint uses the app version, so development builds with reader changes but the same version can keep old entries; Reload tags or Clear Library fixes that, as Mp3tag's own release notes advise.

## Gaps

- Files changed outside OpenTag with their modification time and size both preserved keep their old entry until Reload tags or Clear Library (Mp3tag only checks the time, so it shares this).
- Locale-formatted technical fields (`%_file_mod_date%` and similar) are stored as formatted when read; changing the system locale does not refresh them.
- Release-build and Windows/Linux timings, and comparison with Mp3tag on the same collection, were not measured.
