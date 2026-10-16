# Converters and export parity — 2026-10-16

Branch `feat/convert-export` from `origin/main` at `5a57053`. It closes the WS-2 and WS-3 items of the 2026-10-16 gap audit listed below. Nothing here was checked against the original program. Each decision names the documentation it follows, or says that it is undocumented. Only unit and component tests were run; the app was not launched.

## Filename → Filename: a non-matching file is left unchanged

Before, one file whose name did not match the old pattern stopped the whole preview (`converter.render(...)?`). Filename → Tag already leaves such files alone (PR #23).

- `FilenameConverter::try_render` returns `None` for a name that does not match, using the shared `CompiledFilenamePattern::try_parse`. `render` keeps its error for callers that need a match.
- `preview_filename` lists those files in `FilenamePreview.unmatched`. The reviewed plan holds only the matching files, and each move records its position in the selection. Apply renames them together, in one undo step (`history::record("Filename → Filename", …)` covers the whole selection).
- If no file matches, the preview has no renames and Apply stays disabled.
- The dialog shows unmatched files with the existing "No match, not changed" text. The Apply button counts only the files that will change.

Tests: `non_matching_file_is_left_unchanged_and_matching_files_rename_together` (src-tauri), `filename_rearrangement_retains_extension_and_uses_numbered_parts` (tag-core) and the converter-review UI tests. These include `converterValidator`, the adapter the dialog itself passes to the review. The first push called the validators without the unmatched list, which rejected every partial plan (Codex review finding).

## Text File → Tag: keyed lists may name more files than the selection

The docs say records are matched by position, or by file when the mask uses `%_filename_ext%` or `%_path%`. With a key:

- the list may be longer than the selection. A record for a file that is not selected is skipped.
- a selected file that no record names is left unchanged and listed as unmatched in the preview ("No match, not changed").
- a list that names none of the selected files is still an error, so nothing is written by mistake.
- a malformed record, a duplicate record for one file and an ambiguous selection (two selected files with the same name under `%_filename_ext%`) are still errors.

Positional records still need exactly one line per selected file. The UI validators check that the planned and unmatched files together account for the whole selection.

Tests: `keyed_text_import_skips_records_of_other_files_and_leaves_unlisted_files_unchanged` (tag-core), `keyed_text_list_longer_than_the_selection_imports_only_the_named_files` (src-tauri).

## `%_workingdir%` / `%_workingpath%` and the shared evaluation context

The claim in `script-export.md` that the working folder reached exports was false: `ExportOptions` had no working folder. That note now carries a correction.

- `ExportOptions.working_directory` feeds `%_workingdir%` and `%_workingpath%` in the export body, the `$filename` name and a typed export file name. The Export action and File → Export (`run_export` → `action_host`) pass the folder the file list was last changed to. Before a folder is opened, both stay empty, as everywhere else.
- Filename → Filename now uses `EvaluationSettings::context` like Tag → Filename and actions. That brings the working folder, Options → Files "Replace folder separator" (it used to be always `_`) and the shared clock.
- Tag-source cover names (Save image to disc) use `EvaluationSettings::filename`. `$columnValues` and the other column functions work there now; before, they failed because the context was bare.

Tests: `working_folder_reaches_the_export_body_and_file_name` (tag-core), `exports_receive_the_working_folder_in_body_and_file_name` and `new_filename_pattern_uses_the_working_folder_and_folder_separator` (src-tauri), and the extended `saved_cover_files_*` cover-name test.

## Export file name and `$filename`

Export docs: "The default file name and/or file extension can be set from the export configuration (see `$filename()`). Otherwise the previously used file name is prefilled. The file name to be used can always be adjusted upon export."

Before, choosing a configuration left the name field alone. A typed name replaced `$filename` completely, so an HTML configuration chosen after `export.txt` was written to `export.txt`. Now choosing a configuration (and New, Import and Remove, which select one) prefills the field the way Mp3tag does (`configurationFilename` in `export-ui.ts`):

- `$filename(html,…)`: the current name gets that extension (`export.txt` → `export.html`; `C:\Lists\%album%.txt` → `C:\Lists\%album%.html`).
- `$filename(tracks.cue)`: a bare name replaces the file name and keeps the current folder. A relative name with subfolders also keeps the current output folder: `$filename(sub/report.csv)` changes `/lists/export.txt` to `/lists/sub/report.csv`. A rooted or drive-prefixed name replaces the whole field.
- A first argument with placeholders, functions or quotes (`$filename(%EXPORT_NAME%,utf-8)`) clears the field. The engine then evaluates `$filename` for each export. Prefilling the raw text would have sent it as a typed name: `out.%EXPORT_NAME%`, written as `out.report.csv` instead of `report.csv` (Codex review finding).
- A configuration without `$filename`, or with an empty first argument, keeps the field ("the previously used file name").
- An empty field stays empty. It already means "the configuration's default" (`export.txt` with `$filename` applied).

A name the user then types is final, as in Mp3tag. The `$filename` encoding still applies to it. The rendering core is unchanged: `filename_override` is the final name.

Tests: the configuration-prefill and relative-destination cases in `tests/export-ui.test.mjs`, plus `a_dynamic_filename_argument_is_evaluated_when_no_name_is_typed` and `a_relative_filename_prefill_agrees_with_the_template_destination` (tag-core export).

### Final review follow-up, 2026-10-16

Preserved the existing main refresh at `684f2ff` and the uncommitted review
fixes handed over by the previous owner. Reproduced the current Codex finding
against the old production helper: the dynamic argument prefills
`out.%EXPORT_NAME%`. The corrected helper leaves it empty, and the actual
renderer produces `report.csv` when that field contains `report.csv`.

The next review identified relative template subfolders losing the existing output folder. Reproduced `/lists/export.txt` plus `$filename(sub/report.csv)` prefilling only `sub/report.csv`, which the renderer then resolved under the audio folder. The corrected prefill retains `/lists/sub/report.csv`. Tests cover POSIX, drive, UNC, verbatim, rooted and Unicode paths, and compare the actual renderer with and without the dialog override. Integrated current main `be68d833` once after this fix.

Previous validation on macOS arm64 with Rust 1.99 and an isolated temporary directory:
316 UI tests, 700 Rust tests and all 29 focused export tests passed; four
opt-in Rust cases remain ignored. Production build, workspace Clippy with
warnings denied, formatting, eight publication tests, strict guide build,
writing-plan validation and diff checks passed. No OpenTag application launch
or original-Mp3tag runtime comparison was performed. Affected guide briefs
record the new behavior while retaining their draft status; this does not
claim those larger chapters are complete or publicly published.

Combined validation after integrating `be68d833`: all 317 UI tests and 721
locked Rust workspace tests passed, with four opt-in cases ignored. All 30
focused export tests passed. Production build, workspace Clippy with warnings
denied, formatting, eight publication tests, strict guide build, writing-plan
and diff checks passed. The native application was not launched.

## Default encodings

- **Export:** a configuration that does not choose an encoding with `$filename(name,enc)` is now written as ANSI. It used to be UTF-8. The docs say Write BOM applies when "writing of UTF-8 or UTF-16 is enabled via the `$filename` function", which makes ANSI the default. ANSI is the system codepage on Windows and Windows-1252 elsewhere. Characters the codepage cannot store become `?`, as before. Every bundled configuration and OpenTag's New template name `utf-8` explicitly, so they are unaffected.
- **`.m3u` on macOS and Linux:** these systems have no ANSI codepage. `.m3u` is now written in Windows-1252, which playlist reading already assumes, but only when the playlist reads back unchanged. Reading tries UTF-8 first, so the playlist is written in UTF-8 instead in two cases. One is an entry Windows-1252 cannot store (a `?` would break it). The other is Windows-1252 bytes that are also valid UTF-8: `Â©.mp3` is `C2 A9` in Windows-1252, which reads back as `©.mp3` (Codex review finding). On Windows the system codepage is kept, and an unstorable path is still an error that suggests `.m3u8`. `.m3u8` and `.pls` stay UTF-8.

Tests: `exports_without_an_encoding_argument_are_ansi_as_in_mp3tag` (tag-core export), and `pls_and_ansi_m3u_output` (playlist), which round-trips ASCII, Windows-1252, UTF-8-lookalike and CJK names with and without a BOM.

## Ctrl+Shift+K: totals per folder

Confirmed: the per-folder wizard restarted the track counter in each folder but saved the selection count as every folder's total, which gave `3/30`. `NumberingOptions.count_track_total` makes the total the last track number before the counter restarts. That is the folder's track count when numbering starts at 1. Ctrl+Shift+K sends it when "Save total track count" is ticked, and disables the "Total tracks" value. Provider documents (Android) have no folders, so the shortcut does not number them per folder and keeps the entered total, like Ctrl+K (`numberingTotal`; Codex review finding). Ctrl+K still uses the entered total. Per-folder totals are not documented; this follows from "counter reset for each subfolder" (keyboard reference).

Tests: `per_folder_numbering_counts_each_folders_own_total` (tag-core numbering), and `only per-folder numbering counts totals; provider documents keep the entered total` (`tests/numbering-review.test.mjs`).

## Not changed

- Relative Filename → Filename targets still resolve against the file's folder, as Tag → Filename does. The converter page says "below the current working directory"; this is shared rename code and is left for a separate change.
- `/` as a folder marker in Filename → Filename's old pattern (WS-5, `script.rs`).
- Two partitions writing one output file are still refused, and the strict export argument checks are kept (audit rows marked unverified).
- Converter dialogs still open on their built-in default patterns, with history offered.

## Checks run

`AR=/usr/bin/ar npm test` (UI unit tests and `cargo test --workspace --locked`), `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check`, `tsc --noEmit`, `git diff --check`.
