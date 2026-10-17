# TagGroove reconstruction specification

The user requests a fully open source cross-platform reconstruction of Mp3tag 3.36.1 with exact functionality, using Tauri 2 and a Rust backend. This is a continuing architectural implementation, not a request for a prototype. Full parity is the final acceptance criterion.

## Design

Create independently authored code under `taggroove/`, licensed MIT. Keep the proprietary executable, assets, translations, export templates and source scripts outside the distributable application. The supplied release remains the reference artifact. Preserve its operator workflow: menu and toolbar, editable tag panel, sortable selectable file table, filter, extended tags, covers, converters, actions, numbering, playlists, exports, sources, and options. Use native system fonts and compact light desktop controls; motion only for focus and dialog transitions.

Tauri 2 hosts a TypeScript/Vite interface. All audio reads/writes and format-string operations run in Rust. A platform-independent `tag-core` crate contains metadata I/O and scripting; a small Tauri adapter provides commands and native file dialogs. Target Windows, macOS and Linux. CI must eventually build and test all three; a local macOS build alone does not prove cross-platform delivery.

The user also requires mobile interfaces to be considered during the port. Include Android and iOS in architecture and verification: phone/tablet adaptive panes, touch targets, explicit document-picker capabilities, native URI/scoped-resource file access, lifecycle and export/save-back behavior. Do not expose desktop folder selection on runtimes where the native picker does not support it. Portable Rust compilation is necessary but does not prove mobile storage or device usability.

List loading and bulk mutation replies carry lightweight metadata/cover descriptors; active selection fetches artwork separately. Keep table work proportional to visible rows, compile filter expressions once and reuse cached sorting keys. Establish measured loading/interaction/memory budgets against the original with the same corpus; a static screenshot is not performance proof.

## Shared interfaces for the initial vertical slices

Rust `Fields = BTreeMap<String, Vec<String>>`. `AudioEntry { path: String, filename: String, format: String, duration: f64, bitrate: u32, sample_rate: u32, channels: u8, size: u64, fields: Fields, covers: Vec<CoverInfo> }`. `CoverInfo { index: usize, mime: String, description: String, picture_type: String, data_base64: String }`.

Tag-core I/O exports `read_entry(path: &Path) -> Result<AudioEntry, String>`, `scan_paths(paths: &[String], recursive: bool) -> ScanResult`, `write_fields(path: &Path, updates: &Fields) -> Result<AudioEntry, String>`, `remove_tags(path: &Path) -> Result<AudioEntry, String>`, `add_cover(path: &Path, image: &Path) -> Result<AudioEntry, String>`, `remove_cover(path: &Path, index: usize) -> Result<AudioEntry, String>`. Empty vectors remove a field; absent fields preserve it. `ScanResult { entries: Vec<AudioEntry>, errors: Vec<FileError> }`, `FileError { path: String, error: String }`. Preserve unknown metadata wherever supported. Do not silently advertise unsupported formats.

Scripting exports `evaluate(template: &str, fields: &Fields) -> Result<String, String>`, `parse_filename(filename: &str, pattern: &str) -> Result<Fields, String>`, `apply_action(fields: &Fields, action: &Action) -> Result<Fields, String>` with tagged serde enum `Action` (`type` snake_case) variants `Format { field, template }`, `Replace { field, from, to, case_sensitive }`, `RegexReplace { field, pattern, replacement }`, `Case { field, mode }`, `Remove { fields: Vec<String> }`. Reject unknown functions and malformed input; implement documented semantics and track missing functions.

Tauri commands use camelCase JS arguments: `scan_files({paths,recursive})`, `save_fields({paths,updates})`, `remove_tags({paths})`, `add_cover({paths,imagePath})`, `remove_cover({paths,index})`, `preview_format({paths,template})`, `filename_to_tags({paths,pattern})`, `apply_action({paths,action})`, `rename_files({paths,template})`, `export_csv({paths,outputPath})`, `export_playlist({paths,outputPath})`. Batch mutation returns `ScanResult`, so successful files and per-file failures stay visible.

## Required final parity

Inventory every menu command, option, action type, scripting function, metadata mapping, format, language, CLI switch, export and source framework feature. For every item retain its source and implementation/verification status. Required workflows include all five converters, action-group import and editing, complete scripting and filters, extended multi-value fields, multiple covers with type/description/transforms, format-specific tag options, undo/redo, multi-selection and inline edits, configurable columns/toolbars/panel/docking, custom field values, numbering/disc counters, playlists/cues/chapter editing, grouping exports, source query and matching, MusicBrainz/Discogs/freedb/source scripts, external tools, settings import/export, keyboard shortcuts, drag/drop, accessibility and translations.

## Verification

Prove metadata write/read round trips with generated disposable audio fixtures, preservation of untouched fields and audio data, errors for malformed files, refusal to overwrite rename destinations, literal and nested scripting behavior, converter captures and action outputs. Build the frontend and native app, operate the actual desktop flow, retain screenshots, and later differential-test against the original on a Windows/Wine runtime. Static resource presence proves feature surface, not execution. The goal remains active until exact parity and all platform artifacts are demonstrated.
