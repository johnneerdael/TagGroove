# TagGroove Implementation Plan

**Goal:** Fully open source Tauri 2/Rust reconstruction with exact Mp3tag 3.36.1 behavior on Windows, macOS and Linux.

**Architecture:** Portable tag-core crate, Tauri native adapter, TypeScript desktop interface. Independently authored implementation; original artifacts remain outside the app.

**Spec:** `docs/superpowers/specs/2026-10-14-opentag.md`

## Constraints and review focus

Preserve unedited metadata and audio, report per-file failures, never silently overwrite rename targets, preserve ordered multi-value fields, and distinguish tested implementation from unverified parity. Validate malformed inputs and scripting literals, including Unicode and quoted separators.

## Execution slices

- [ ] Inventory shipped resources, menus, languages, actions, scripts and defaults with artifact hashes and REA evidence references.
- [ ] Implement real metadata loading, recursive scanning, patch writes, tag removal and multiple covers in `crates/tag-core/src/tags.rs`. Test using generated WAV/MP3/FLAC fixtures, unknown fields, empty removals, partial scan errors and audio preservation.
- [ ] Implement format evaluator, filename parsing and actions in `crates/tag-core/src/script.rs`. Test nested functions, literals, absent placeholders, Unicode, regex failures and repeated capture fields before implementation.
- [ ] Implement TypeScript tag-panel/table workflow in `src/`, including multi-selection, pending edits, save results, sorting, filter, extended tags, converters, actions, numbering, covers, playlists and CSV export. Match the command contract in the spec. Inspect a rendered screenshot and operate real files.
- [ ] Integrate Tauri commands in `src-tauri/src/lib.rs`; validate no-clobber rename plans and batch errors. Build native app and frontend; run portable crate tests and lint.
- [ ] Extend source frameworks, full scripts/filters/actions, undo/redo, configuration and remaining formats according to inventory; obtain Windows reference execution and differential results.
- [ ] Package reproducible open source builds and prove all target OS workflows; audit every parity item before goal completion.

Use separate workers for independent tag I/O, scripting and interface scopes, as directed by the reverse-engineer-anything staged-investigation workflow. Keep integration and evidence ownership with the parent. Track incomplete final requirements in `docs/PARITY.md` rather than narrowing the goal.
