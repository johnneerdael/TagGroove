# Actions robustness (WS-1) — 2026-10-16

Branch `feat/actions-2`, started from `main` at `5a57053`. It closes the Actions rows of the 2026-10-16 gap re-audit (workstream WS-1). Nothing was checked against the original program, and nothing was launched locally. The evidence is unit tests in `crates/tag-core` and `src-tauri`, which run real WAV fixtures through the native action host.

## Status of each item

| Item | Before | Now | Evidence |
|---|---|---|---|
| A problem with one file inside a group | The file was marked failed; its remaining actions and every later group were skipped, and it was left out of exports | The problem is reported for that file and action, and the file's remaining actions and groups still run. It stays in deferred exports and playlists and in the refreshed file list | `a_failed_action_is_a_warning_and_later_actions_and_groups_still_run`, `filename_collision_reports_only_the_failed_file_and_returns_both` |
| Export Cover onto an existing file | Failed (no-clobber) | The same image is left alone; another image is replaced, after undo saved the old file | `cover_export_replaces_existing_files_and_rejects_input_aliases`, `cover_export_by_the_same_file_replaces_its_earlier_output_in_order`, `cover_export_of_an_image_already_there_keeps_its_writer`, `cover_export_cross_file_destination_keeps_the_first_different_image` |
| `\\` in a Format Value or Tag → Tag result | Stored as one value | Several values, as in the Tag Panel | `format_value_double_backslash_creates_several_values` |
| File names in Import Cover, Export Cover, Import Text File and Generate Playlists | Plain text; `\` literal on macOS and Linux, `/` in a value made a folder, `:` and `?` kept | Filename mode, as in Tag → Filename | `cover_and_text_file_names_use_filename_mode`, `file_name_formats_use_backslash_folders_and_clean_tag_values` |
| Import Cover with `*.jpg` | Literal path | First matching image the import accepts; WebP files can be imported | `wildcard_cover_source_is_the_first_image_in_natural_order`, `wildcards_match_whole_names_case_insensitively`, `import_cover_wildcard_uses_the_first_matching_image`, `import_cover_wildcard_takes_webp_and_skips_what_the_import_refuses` |
| `$loop` sort order in Export actions | Always byte order (1, 10, 2) | Follows Options → General natural sorting | `export_loops_follow_the_natural_sort_option` |
| Bulk `.mta` import | One file per call | Backend command `import_action_groups` takes several files and folders | `bulk_import_reads_files_and_folders_and_reports_each_failure`, `bulk_group_import_reports_an_unreadable_input_and_imports_the_rest` |
| Example action groups | A missing `actions.json` gave an empty library | Five unchecked OpenTag groups under Examples | `a_missing_configuration_offers_unchecked_example_groups`, `example_groups_do_what_their_names_say` |
| `.mta` IDs for Adjust Cover, Set Cover Properties, Generate Playlists | Invented IDs 1001–1003 | Unchanged | Needs one `.mta` file saved by Mp3tag for each type |

## Decisions

### Warnings instead of failed files

The Messages options page says "at warnings from Actions" shows "any errors and warnings that resulted in executing the last action or action groups". The audit records that Mp3tag keeps running the remaining actions after such a warning. The native host now does the same: an action that fails for one file adds a line `Group / action N: reason` for that file and the next action starts on the file as it is. Field changes made before the problem are kept and written at the end of the group, as they would be without the problem.

Two cases still stop a file. When the file's changes cannot be written at the end of a group, the file is left out of later groups, exports and the returned entries, as before: its pending changes are not on disk, so continuing would work on values the file does not have. When an imported group contains an unresolved `.mta` action, nothing runs at all, as before.

The UI already shows these lines in the action warnings message when that message is enabled, and in the file error list otherwise. The result keeps the `errors` field, so no UI change was needed. The status line still counts them as "failed"; renaming that word is left to the translations work.

### Export Cover onto an existing file

The Export Cover page documents one option about repeats: with "Export duplicate covers" enabled, "identical covers (same name and content) are exported". It does not say that an existing file stops the export. OpenTag now reads it like this:

- When the file already holds the same bytes, nothing is written. Running an Export Cover action again is quiet whichever way the option is set.
- When it holds something else, it is replaced atomically. The action dialogs and Quick Action run through `history::record_with_side_files`, so undo puts the old image back. A file that existed before the run is given to undo the first time the run uses its name; undo drops that copy when the file did not change. A new image stays after undo, as before.
- When another selected file wrote a different image to the same name earlier in the same run, the later file gets a warning and the first image stays. The same image from a later file is no collision, also when duplicates are exported (it would write the same bytes again).
- A file that finds the image already there writes nothing and does not take the name over, so the file that wrote it can still replace it later in the run.
- When the same file exports again to a name it wrote earlier in the run (its cover changed in between, in the same or a later group), the later export replaces the earlier one, so the actions apply in order.
- The existing protections stay: an output that is an input audio file, a chapter or cue sheet row's file, or an alias of one through `..` or a symlink is refused.

The duplicate check that keys on destination plus image content is unchanged. So is the per-file numbering of several covers (`cover.png`, `cover (1).png`).

### `\\` creates several values

Mp3tag separates multiple values of one field with `\\`, and OpenTag's Tag Panel already splits on it (`splitValues` in `src/tag-panel.ts`). Format Value, and so Tag → Tag, now split their result the same way. Each piece is trimmed, as in the Tag Panel, so `Jazz \\ Blues` (the spacing OpenTag uses to show several values) gives `Jazz` and `Blues`. Empty pieces are dropped, so `%genre%\\%style%` with an empty STYLE gives one value. A result without `\\` is stored as it is, spaces included. A single `\` is ordinary text. `_FILENAME` and `_DIRECTORY` targets are not affected: there `\` separates folders.

### File names are evaluated in filename mode

The four action pages call these settings a "relative or absolute file name". OpenTag now evaluates them the way Tag → Filename does, with the shared `TemplateMode::Filename`:

- A literal `\` in the format string separates folders on every platform, and so does a literal `/`. On macOS and Linux the host turns `\` into `/`; on Windows `/` into `\`.
- Tag values have `\` and `/` replaced with Options → Files "Replace folder separator in tag values with" (default `_`), and `: * ? " < > |` with `_`. `AC/DC` no longer creates a folder.
- Literal text is kept as typed, so `C:\Covers\%album%` and wildcards such as `*.jpg` still work.
- `%_path%`, `%_folderpath%` and `%_workingpath%` are paths already and keep their separators in filename mode. This keeps the default file names of these actions (`%_folderpath%cover`, `%_folderpath%playlist.m3u8`) working. It also applies to Tag → Filename and to Format Value on `_FILENAME`, where these placeholders used to turn into one long name such as `_Users_me_Music_Album_title.mp3`.

Export keeps its own file-name handling (`export.rs`), which already treats `\` as a folder separator. The playlist partition format string is still evaluated as text, because it only groups files.

### Import Cover wildcards

Wildcards are not on the 3.36.1 Import Cover page; the audit cites forum usage where the first matching image is used. A `*` (any run of characters) or `?` (one character) in the last part of the file name selects files in that folder, matching names without regard to case. Matches are taken in natural, case-insensitive name order (`front2.png` before `front10.png`), and the first one the cover import accepts is imported: the file's first bytes must look like an image, and the whole file must then pass the same check the import makes (`picture_from_bytes`: JPEG, PNG, GIF, BMP, TIFF or WebP). Folders, files that are not images and image formats the import does not take (QOI, for example) are skipped, so a broad `*` moves on to the next match. Import Cover and Add cover now take WebP files too, as Manage Covers already did; before, lofty's reader refused them. No match is a warning for that file ("No image matches …"), and with "delete existing covers" set nothing is deleted. Wildcards in folder names are refused. A name without wildcards is used as before.

### Natural `$loop` order

Export actions now pass Options → General "natural sorting" (on by default) to the export renderer, so `$loop(%title%)` lists 1, 2, 10. With the option off the order is byte order, as before. The standalone Export dialog runs through the same action host, so it follows the option too.

### Bulk `.mta` import

`import_action_groups(paths, legacyEncoding)` imports each path on its own and returns one result per file: `{ path, imported }` or `{ path, error }`. A folder stands for the `.mta` files in it (other files are ignored), in natural name order, so Mp3tag's `data/actions` folder can be imported in one step. An empty folder, an unreadable file and a provider document that cannot be copied each give an error entry and do not stop the others. Group names come from the file names, as with the single import; the dialog already adds `(2)` to a name that is taken.

The Actions dialog still imports one file. Its picker and import flow are in `src/main.ts` and `src/action-ui.ts`, which the pending translations PR rewrites, so switching the picker to `multiple: true` and calling the new command is left for after that PR. That step needs no new strings beyond pluralising the existing "Import action group" title.

### Example groups

Mp3tag ships its own groups (Standard, Case conversion, CD-R). OpenTag does not copy them. When `actions.json` does not exist, the library now holds five groups written for OpenTag, all under an `Examples` submenu and all unchecked, so Apply does nothing until one is checked:

- Tidy spaces in tag fields: two Replace with Regular Expression actions on `_TAG` (runs of spaces to one, then trim).
- Split artists at semicolons: Split Field on ARTIST.
- Capitalize titles and names: Case Conversion (Mixed Case) on TITLE, ARTIST, ALBUMARTIST and ALBUM.
- Name files 01 - Title: Format Value `_FILENAME` = `[$num(%track%,2) - ]%title%`.
- Save front cover as folder image: Export Cover `folder`, front covers only.

A saved library, even an empty one, is loaded as saved, so removed examples do not come back. Every example passes validation and exports to `.mta`.

## Checks

Review follow-up (2026-10-16): both new regressions were run against the
previous production implementation before restoring the fixes. The cover
ownership case failed because the original writer received a cross-file
collision after the other file's equal-image export. The wildcard case
failed with `failed to parse picture` for the first WebP match. With the
fixes restored, all 41 native action-host tests passed. The ownership case
enables duplicate export so it reaches the reviewed no-op branch; it checks
that the original writer succeeds, the other file retains its expected
warnings, and the final exported image contains the changed cover.

- `AR=/usr/bin/ar npm test`: 312 UI tests and the whole Rust workspace pass.
- `AR=/usr/bin/ar cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.

## Not covered

- The `.mta` type IDs for Adjust Cover, Set Cover Properties and Generate Playlists, and the guessed schemas for types 4, 6 and 11–15, need `.mta` files saved by Mp3tag.
- The Actions dialog's multi-file `.mta` picker (see above).
- None of these behaviours was compared with Mp3tag at runtime.
