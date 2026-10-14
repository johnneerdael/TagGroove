# Standalone export parity

Continue the user's core parity goal on `feat/core-parity`. Reuse the portable MTE renderer and native action export path. Add a persisted configuration library and File → Export (Ctrl/Cmd E), without Android layout work.

Deliver configuration creation, duplication, renaming/editing, removal, and MTE import; remembered selection, filename, append and partition choices; BOM and directory-creation options. Validate configuration syntax before saving. Preserve raw tag values, selected table order, existing output checks, and action interfaces. Export-only requests must leave source audio byte-identical.

The dialog edits a local copy. Save persists it; Cancel discards it; Export persists the current choices and executes the selected configuration. Show failures without closing the dialog or discarding configuration edits. Relative filenames use the first selected file's folder; an empty filename uses the configuration default. Keep existing CSV and playlist shortcuts.

Verification: failing tests for persisted library validation/import/roundtrip, frontend CRUD identity/selection, and native BOM/append/directory export behavior; full suite, build, Clippy/fmt; actual desktop CRUD/import/export/reopen on disposable generated files. Record remaining differences, including skip-selection dialog and open-output options, without claiming exact parity.

Implemented and verified: 101 frontend + 279 Rust tests, strict Clippy, formatting, production frontend build and macOS bundle. Native management/export/append/restart/invalid-edit/removal and named-action checks passed on generated fixtures. See `docs/verification/export-manager.md` for the evidence boundary and remaining parity requirements.
