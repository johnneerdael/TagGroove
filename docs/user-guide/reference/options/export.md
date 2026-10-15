# Export options

Open **File → Options… → Export** for output-file behavior shared by export configurations. A configuration's format, filename, and template are edited in the **Export…** dialog itself.

!!! tip "New to this?"
    Leave the selection dialog enabled while building a template. Check both the active configuration and destination before running a batch export.

## Show selection dialog

Defaults to on. **File → Export…** opens the configuration selector, where an output configuration can be chosen and edited. With the setting off, the command immediately runs the active configuration. This is useful for a repeated report but makes the remembered selection important.

Holding Shift with the export command, ++cmd+shift+e++ / ++ctrl+shift+e++, requests an export per folder. That partitioning is separate from creating directories.

## Don't create directories

Defaults to off, allowing parent folders needed by the export destination to be created. Enable it when a misspelled or unexpected path should fail rather than produce a new folder tree. Existing output directories can still be used. The option does not prohibit writing files inside them.

## Write BOM

Defaults to off. **Write BOM (UTF-8 and UTF-16 exports)** adds a byte-order mark for those encodings. Some spreadsheet/text applications use a BOM to detect Unicode correctly; other consumers prefer files without it. Match the requirements of the program that reads the output. A BOM changes the start of the output bytes, not the exported field values.

## Open the result

[Messages](messages.md) controls whether OpenTag offers to open the written export through the operating system's associated application. Disabling that message leaves the output on disk and does not disable export itself.

These settings do not define CSV quoting, HTML structure, sorting, or which fields a template includes. Those belong to the export configuration/template. A playlist has its own [Playlist options](playlist.md), including its own BOM setting.

## Related

- [Playlist options](playlist.md)
- [Messages](messages.md)
- [Files](files.md)

<!-- verified: src/playlist-ui.ts EXPORT_DEFAULTS, exportOptionsPage; src/export-ui.ts ExportUI; src-tauri/src export settings; crates/tag-core/src/export.rs output encodings. Canonical options/export delegates to export feature; this page covers the actual Options controls. -->
