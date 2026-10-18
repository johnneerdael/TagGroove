# Library options

Open **File → Options… → Library** to keep previously read metadata between sessions. The Library is a cache of file information, not a second collection of audio files.

!!! tip "New to this?"
    Enable it when repeatedly opening a large collection. A first load still has to read the files; later loads provide the opportunity for a cache hit.

## Enable Library

**Enable Library** defaults to off. With it off, files are read from disk when loaded. With it on, metadata is stored in a database in the configuration folder and reused on later loads when the path, modification time, size, and reading-settings fingerprint still match.

TagGroove updates the cache when it saves, renames, moves, or deletes files. **File → Reload tags** forces selected files to be read again and refreshes their entries. A change to tag-reading settings or reader version prevents reuse of an entry interpreted with the older settings.

The status reports whether the database exists, the cached file count and size, and how many files in the last load came from the Library versus disk. A high disk-read count after enabling the Library is expected for the first load or changed files.

Loading cached entries formats their technical dates again from the saved raw
timestamps. A formatting change therefore does not require clearing the whole
Library or rewriting audio. Refresh or load the list again after a system locale
change; if the system has not exposed its new locale to the running app, restart
TagGroove and reload. This changes displayed date text, not file timestamps or
stored tag values.

!!! info "Coming from Mp3tag"
    TagGroove caches metadata and image descriptors. It reads image bytes from the audio file when needed; it does not move embedded images or other binary payloads into the Library database. Do not expect the binary-storage or memory behavior described for Mp3tag's Library.

## Only include these directories

This restriction defaults to off. Turn it on, choose **Add…**, and select the folders whose files should be cached. **Remove** removes the selected inclusion entry. Files outside the included folders can still be loaded and edited; they are read from disk each time and are not added to the persistent cache.

With the restriction enabled and no folders listed, nothing is cached. Add an album's parent folder if all its descendant albums should be included. This list controls caching, not the file-list contents and not an automatic background scan.

## Cleanup Library

**Cleanup Library** checks cached paths and removes entries for files no longer accessible, including files moved or deleted outside TagGroove. With a directory restriction, it also removes entries outside the included folders. It uses the restriction currently displayed in the dialog.

Progress and checked/removed counts appear on the page. **Cancel** stops further cleanup; entries already removed stay removed. Cleanup does not delete music files. Reconnect removable or network drives before cleanup if their entries should remain cached: an inaccessible path can be treated as an orphan.

## Clear Library

**Clear Library…** asks whether to remove every cache entry. Choose **Clear** to do so or **Keep** to cancel. The audio files remain where they are; their next load reads metadata from disk. Clearing is useful when deliberately rebuilding the cache, and is performed immediately rather than waiting for the outer Options **OK**.

## Performance and stale values

Place the active configuration on reliable, fast storage when practical. The Library still performs file-stat checks and database writes, so a slow portable drive or network filesystem can limit improvement. Compare the status counts before attributing loading time to tag parsing.

Mp3tag's original troubleshooting mentions excluding its process from Windows Defender. TagGroove provides no equivalent antivirus setting; those instructions name a different executable and should not be applied as a TagGroove prerequisite. Investigate actual I/O or scanning delays under the computer's existing policy.

If another program changes metadata while preserving both file size and modification time, an old cache entry can remain valid by those checks. Use **Reload tags** after that kind of external edit.

## Related

- [Files and timestamp preservation](files.md)
- [Directories](directories.md)
- [Tag-reading policies](tags-mpeg.md)

<!-- verified: src/library.ts LIBRARY_DEFAULTS, libraryOptionsPage, bindMaintenance; src-tauri/src/library.rs; crates/library/src/lib.rs Stamp, fingerprint, Library.lookup, Session. Source-backed performance mechanism, no measured speedup claim. -->
