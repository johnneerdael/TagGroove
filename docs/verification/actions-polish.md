# Actions, converters and cover polish — 2026-10-15

Branch `feat/actions-polish`, started from `feat/core-parity` at `d67e5c8`. It works through the Actions, Converters and cover rows of `docs/research/remaining-gaps.md` that were still open after batch 5. Nothing was checked against the original program. The decisions below say which documentation page each one follows, and how the page was read where it leaves room.

Nothing was launched locally. The evidence is unit tests, code reading and the native suite `tests/e2e/actions-polish.js`, which runs headless in the CI `native-e2e` job.

## Status of each item

| Item | Before | Now | Evidence |
|---|---|---|---|
| Actions (Quick) reopens on the last type and settings | Always Format Value with defaults | Done | `recallQuick`/`rememberQuick` in `src/action-ui.ts`; `tests/action-ui.test.mjs`; e2e step 1 |
| Convert Codepage: OEM 437/850/852 | 32 pages, no DOS pages except 866 | Done: 437, 720, 737, 775, 850, 852, 855, 857, 858, 860–865, 869 | `crates/tag-core/src/oem_codepages.rs`; `codepage_conversion_covers_dos_oem_pages`; e2e step 2 |
| Import Text File legacy encodings | UTF-8 and BOM UTF-16 only | Done: BOM, then UTF-8, then ANSI | `import_text_file_reads_bom_utf8_and_ansi_text`; e2e step 3 |
| Remove Fields Except and unmapped frames | PRIV, GEOB, chapters survive | Kept on purpose (see decisions) and pinned by a test | `remove_fields_except_keeps_frames_that_have_no_field` |
| Tag → Filename with one clashing destination | Whole batch stopped | Done: that file is skipped and reported | `rename_batch_skips_and_reports_only_clashing_destinations`; e2e step 7 |
| Filename → Filename with an extension in the new pattern | Extension doubled | Already fixed before this branch (`ends_with_extension` in `FilenameConverter::render`, `filename_target`) | Existing `case_only_rearrangement_renames_without_doubling_the_extension` |
| Adjust Cover min. size and "Original" format | Only enlarged; GIF/BMP/WebP failed; re-encoded unchanged covers | Already fixed by batch 5 (`script-export`); BMP and WebP now also have tests | `maximum_size_never_enlarges_and_minimum_size_matches_the_smaller_side`, `covers_that_keep_size_and_format_are_not_re_encoded`, `original_format_resizes_gif_covers_as_gif`, new `single_images_adjust_like_the_action_and_keep_bmp_and_webp` |
| Options → Directories → Cover art directory | Missing | Done | `add_cover_starts_in_the_cover_art_directory_then_an_image_folder_then_the_working_folder`; `tests/options-ui.test.mjs`; e2e step 5 |
| Adjust cover inside Manage Covers | Missing | Done: Adjust… button and row menu item | `manage_covers_adjust_stages_a_resized_copy_and_keeps_fitting_images`; `tests/covers.test.mjs`; e2e step 6 |
| Auto-numbering wizard remembers its options | Missing | Done | `tests/numbering-review.test.mjs`; e2e step 4 |
| Ctrl+Shift+K restarts the counter per folder | — | Already done by batch 5 (`keyboard-extended`); still presets its options on top of the remembered ones | e2e step 4, `tests/e2e/keyboard-extended.js` |

## Decisions

### Actions (Quick) memory

The actions page only says that Quick Actions apply one chosen action type to the selection. The register's row ("reopens on the last type and settings") is the behaviour being matched. The dialog now opens on the type last applied, with the settings it was applied with. Every type keeps its own last settings, so switching the type inside the dialog shows that type's last settings rather than defaults. Settings are recorded when Apply is pressed with valid input, before the action runs. A failed run still keeps them, so the dialog can be corrected and run again.

Cover window → Adjust cover… opens Actions (Quick) on Adjust Cover with its last settings, as before, and is remembered like any quick action. Manage Covers → Adjust… shares Adjust Cover's settings, but it does not make Adjust Cover the last type, because it does not run a quick action.

The memory lives in WebView local storage (`opentag.actions.quick`), like the format-string histories. It is not part of the configuration backup. Stored values are applied only to keys the type has, so a damaged entry falls back to that type's defaults.

### OEM code pages

`encoding_rs` implements only the WHATWG encodings, which include 866 but no other DOS page. The new tables cover the DOS pages Windows lists as installed OEM code pages: 437, 720, 737, 775, 850, 852, 855, 857, 858, 860, 861, 862, 863, 864, 865 and 869. They were generated from the Unicode mapping data shipped with Python's `cpNNN` codecs. Bytes 0x00–0x7F are always ASCII. That matters for 864, where the Unicode table maps `%` to U+066A; Windows keeps the ASCII percent sign. A byte the page leaves undefined (for example 0xD5 in 857) is refused with its value, like an invalid sequence in the other pages, so nothing is replaced silently. The Convert Codepage list offers these pages, and the existing check that every listed page is understood by the native converter now also reads the OEM table.

### Import Text File encodings

The Import Text File page only says that the file's contents are imported. The action now decodes the file the same way export configurations and Text File → Tag sources are decoded (`decode_export_template`). A BOM (UTF-8, UTF-16 LE/BE) decides first. Otherwise the text is UTF-8 if it is valid UTF-8, and ANSI if it is not. ANSI is the system code page on Windows (`GetACP`) and Windows-1252 elsewhere, which matches what Mp3tag reads from such files on a Western Windows system. No codepage option was added: the documentation shows none, and the `.mta` layout of type 14 has no slot for one. A file that is valid UTF-8 is always read as UTF-8, even when it was written as ANSI and happens to be valid UTF-8, which is rare for real text.

### Remove Fields Except keeps frames without a field

The Remove Fields Except page says the action keeps the listed fields "and removes all other tag fields from the tag". The Utils page describes Remove Unsupported Fields as removing "metadata entries present in the files that have no visual representation in Mp3tag", and Mp3tag 3.30's release notes added that menu item for MP3. Strip Chapters is a separate util for MP4 as well. If field actions already removed those entries, the utils would have nothing to do. The conservative reading is that "tag fields" means the fields Mp3tag displays, so PRIV, GEOB, other unknown ID3v2 frames and chapters stay. OpenTag already behaved this way. The behaviour is unchanged, and a test now pins it: on an MP3 with TIT2, TPE1, COMM, PRIV, GEOB and a CHAP frame, keeping TITLE removes ARTIST and COMMENT, while `%_id3v2_unknown_frames%` still lists `GEOB, PRIV` and the chapter count is still 1. Use Utils → Remove unsupported fields or Strip chapters to remove those entries. This is the item most worth checking against the original.

### Tag → Filename clashes

Rename Files does not describe clashes. The register's row records that Mp3tag skips and reports the clashing file. OpenTag used to check the whole plan and stop the batch if any destination was taken. The batch now renames every file whose destination is free and reports each of the others with "Destination exists or another file maps to …". When two files map to the same new name, the first in list order gets it. A destination that already exists on disk is never overwritten, and that includes case-insensitive matches on macOS and Windows. Evaluation errors are reported the same way and no longer stop the batch. The status line reads like other partial batches ("Renamed: 1 file, 1 failed"), and the whole run stays one undo step for the files that were renamed. The reviewed Filename → Filename converter still refuses the whole plan at preview time. That is its two-step review design, and changing it was out of scope.

### Cover art directory

The Directories page says the option "specifies a default directory that is displayed when adding cover art" and that it can be a full path, a relative path or contain placeholders. If it is empty or cannot be opened, Mp3tag "opens the directory of the first selected file if it contains cover art or the current working directory in all other cases". OpenTag reads this as follows:

- The value is evaluated as a format string for the first selected file, the same file the cover window shows. A relative result is taken relative to that file's folder, the way relative names work in Import Cover and Import Text File.
- If the result is not an existing folder, the file's folder is used when it holds an image file (jpg, jpeg, png, gif, bmp, webp). "It contains cover art" is read as the folder containing cover images, because the dialog picks an image file.
- Otherwise the working folder is used: the folder the list was last changed to. Without one, the file's folder is used.

The option is used by Add cover… and Replace cover… in the cover window and by Add… and Replace… in Manage Covers. It is stored as `pages.directories.cover_art` in the settings file, so it is part of the configuration backup.

### Adjust inside Manage Covers

Manage Covers has an Adjust… button and an "Adjust cover…" item on the row menu. They act on the selected covers. The settings dialog is the Adjust Cover action editor with its type fixed. Each selected cover is resized and converted in the backend (`stage_adjusted_cover`) by the same code as the Adjust Cover action (`tag_core::adjust_cover_image`), and the result replaces the cover in the list. Its type, description and the files it appears in are kept. Nothing is written until OK, which writes the whole list as one undoable step. A cover that already matches the settings is left as it is, so OK does not rewrite it. Covers added or replaced in the dialog can be adjusted too, since they are staged images.

### Auto-numbering memory

The auto-numbering page says "Save total count of tracks" is "prefilled with the selected files count". So the total track count is not remembered, while every other option is: track start and digits, the total-track checkbox, disc numbers with start, digits, total checkbox and total, both "next disc" options, and both reset options. A field chosen in either field menu stays selected even when the current selection lacks that field. Options are recorded when numbers are staged, not on Preview or Cancel. A disc input that is disabled and invalid keeps its last valid value.

Ctrl+Shift+K opens the wizard with the remembered options, then sets "Next disc on change of directory" and "Reset track counter on next disc" and turns disc numbers off. When those numbers are staged, the other options are remembered. The three preset options keep their earlier Ctrl+K values, so a per-folder run does not change what Ctrl+K opens with. The memory lives in WebView local storage (`opentag.numbering`).

### Test seam for the image file dialog

Headless CI cannot drive the system file dialog. When a native suite creates `window.__OPENTAG_E2E_DIALOGS__ = {requests, answers}`, the cover image picker records its title and start folder in `requests` and returns the next entry of `answers` instead of opening the dialog. Normal runs never create the object. `__OPENTAG_TIMINGS__`, used by the performance suite, works the same way.

## Verification

All on macOS 15 (Darwin 24.6):

- `cargo test --workspace --locked` (with `AR=/usr/bin/ar`): 558 passed, 0 failed, 4 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- `npm run build` (`tsc --noEmit` and Vite): clean.
- `node --test tests/*.test.mjs`: 252 passed.

New or changed unit tests:

- `crates/tag-core/src/actions.rs`: `codepage_conversion_covers_dos_oem_pages`: 437/850 umlauts, 852 Polish letters, 437 box drawing, 858's euro against 850's dotless i, ASCII kept in 864, and an undefined byte refused in 857.
- `crates/tag-core/src/cover_ops.rs`: `single_images_adjust_like_the_action_and_keep_bmp_and_webp`.
- `src-tauri/src/action_host.rs`: `import_text_file_reads_bom_utf8_and_ansi_text` (UTF-8 BOM, UTF-16 LE/BE BOM, plain UTF-8, Windows-1252 with curly quotes and euro) and `remove_fields_except_keeps_frames_that_have_no_field`.
- `src-tauri/src/lib.rs`: `rename_batch_skips_and_reports_only_clashing_destinations` replaces `rename_batch_collision_aborts_every_source`. Two files map to the same name and a third to an existing file. The first is renamed, the other two are reported in order and left alone.
- `src-tauri/src/covers.rs`: `manage_covers_adjust_stages_a_resized_copy_and_keeps_fitting_images` and `add_cover_starts_in_the_cover_art_directory_then_an_image_folder_then_the_working_folder`.
- `tests/action-ui.test.mjs`: quick-action memory per type, the shared Adjust Cover settings, damaged entries, and the OEM pages in the codepage list.
- `tests/numbering-review.test.mjs`: remembered options without the track total, per-value fallback, and Ctrl+Shift+K presets that are not remembered.
- `tests/covers.test.mjs`: `adjustSource` and `applyAdjusted`. `tests/options-ui.test.mjs`: the Directories control.

Native UI: `tests/e2e/actions-polish.js` with `tests/e2e/actions-polish-fixtures.sh` was added to the `native-e2e` job in `.github/workflows/build.yml`. It runs under xvfb on Ubuntu and checks:

1. Actions (Quick) applies Replace (whole word) to TITLE, then reopens on Replace with the same field, strings and checkbox.
2. Convert Codepage lists 437, 850 and 852, and converting `Gr\x94\xE1e` from 850 writes `Größe`.
3. Import Text File reads a Windows-1252 file without a BOM into COMMENT (`Café € “quoted”`).
4. Auto-numbering staged and saved with start 7, three digits and the total checkbox writes `008/2`. It reopens with 7, 3, the checkbox set, the total at the selection size and the folder option off. Ctrl+Shift+K opens with 7 and the per-folder options set, and Ctrl+K afterwards still has the folder option off.
5. With the cover art directory set to `Artwork` in Options → Directories, Add cover starts in `album/Artwork` and embeds the chosen 400×200 image. With the option cleared, it starts in the album folder.
6. Manage covers → Adjust… with JPEG and max. 100 px shows `100×50` in the list. OK writes a 100×50 JPEG front cover.
7. Tag → Filename with `same` on both files renames one to `same.mp3`, keeps `sine-b.mp3`, shows "Renamed: 1 file, 1 failed" and reports "Destination exists".

CI: the suite passed 7/7 in the native-e2e job of run 38690494193 (commit `2a64798`), alongside the 13 existing suites. It was not run outside CI.

## Open

- No differential evidence against Mp3tag for any decision above. The main risks are Remove Fields Except leaving unsupported frames, the cover art directory's relative base, and "contains cover art".
- Convert Codepage has no Mac code pages other than Mac Roman and Mac Cyrillic, and no EBCDIC or ISO-2022 pages. Windows lists more pages than the dropdown offers.
- Import Text File has no explicit codepage choice. A UTF-8-valid ANSI file is read as UTF-8.
- The quick-action and numbering memories are per WebView profile and are not included in the configuration backup.
- Filename → Filename still refuses the whole reviewed plan when one destination clashes. This is intentional for that two-step converter.
