# Case-only renames, filename-mode targets and the folder separator option

Scope: gap register items 1, 13, 14 and 23 and row X1 (`docs/research/remaining-gaps.md`). References: `actions/case-conversion`, `actions/format`, `actions/guess`, `converters/rename-files`, `converters/rearrange-filenames`, `converters/copy-compose-tags`, `customization/options/files`.

## Case-only renames

On case-insensitive volumes (APFS and HFS+ by default, NTFS, exFAT, ext4 with casefolding) `Song.mp3` and `song.mp3` name the same entry. The no-clobber rename made a hard link at the destination and got "exists", and several callers refused the destination before trying.

`crates/tag-core/src/operations.rs` now classifies every destination (`destination_state`):

| State | Meaning | Rename |
|---|---|---|
| `Free` | nothing there | as before (hard link, or exclusive copy across volumes) |
| `Alias` | the source itself in another case or Unicode normalization | through a unique temporary name in the same folder, then to the new spelling |
| `Unchanged` | only the spelling of a folder above differs | nothing to do |
| `Taken` | a different file or folder | refused, nothing touched |

An entry counts as an alias only when it has the source's identity (device and inode on Unix, the resolved path on Windows), sits in the same folder, and the folder does not list the destination name byte for byte. A second hard link to the same file is listed under its own name, so it stays `Taken`. If the folder cannot be read the destination is treated as taken.

Both steps of the alias rename refuse to replace anything: files move by hard link and unlink where the volume supports it, otherwise by a checked rename. If the second step fails, the first is reversed.

Callers:
- Tag → Filename and the File List's inline filename edit (`rename_files`, `src-tauri/src/lib.rs`): the plan check accepts an alias; collisions inside one batch are compared case-insensitively on macOS and Windows.
- Filename → Filename (`src-tauri/src/converters.rs`): accepts an alias. The reviewed target keeps the requested spelling of the last name (`normalized_rename_target`); before, resolving the existing entry folded it back to the stored name.
- Actions on `_FILENAME` (Case Conversion, Replace, Format Value, Guess Values) go through `rename_no_clobber`.
- Actions on `_DIRECTORY` and the undo journal's folder moves use `rename_directory_no_clobber`, which handles the same cases for folders.
- Undo and redo of renames use the same functions, so a case-only rename is reversed by a case-only rename.
- Move files to… reports "already in the destination folder" when the destination is the same folder spelled differently.
- The file list keeps its listed folders when a folder is renamed only in case (`rebaseCaseOnly`, `src/action-ui.ts`), so "Keep moved files" off does not drop its files.

## `_FILENAME` and `_DIRECTORY` targets in filename mode

Format Value and Convert → Tag - Tag (which runs Format Value) evaluate a `_FILENAME` or `_DIRECTORY` format string in filename mode, like Tag → Filename (`TemplateMode::Filename`, `ActionHost::evaluate_filename`):
- `\` typed in the format string creates folders (on macOS and Linux too); `\` and `/` inside tag values and function results are replaced by the folder separator option; `: * ? " < > |` and control characters from values become `_`.
- A format string that already ends in the file's extension (any case) does not get it a second time: `%title%.mp3` on an `.mp3` file names it once. Tag → Filename and Filename → Filename compare the extension case-insensitively as well.
- The Tag → Tag preview evaluates the same way and shows the same name.

Guess Values treats a guessed `_FILENAME` or `_DIRECTORY` as a value: separators and invalid characters are replaced, and the extension is not doubled.

Case Conversion, Replace and Regular-expression replace edit the current name and keep its characters; on macOS and Linux a `\` already in a filename does not become a folder.

The effect records where the name came from (`ActionEffect::RenameFile/RenameDirectory.origin`: `edited`, `template`, `value`; older serialized effects default to `edited`).

## Replace folder separator in tag values

Options → Files → Rename files from tags → "Replace folder separator in tag values with" (`files.folder_separator`, default `_`, may be empty to remove separators, up to 16 characters without separators or invalid characters). It applies wherever tag values become file or folder names: Tag → Filename, Filename → Filename, Format Value, Tag → Tag and Guess Values. Validation exists in the backend (`tag_core::validate_folder_separator`) and the Options page (`folderSeparatorProblem`).

## Case Conversion on `_FIELDNAME`

The check compared `_TAG` with `APEv2`/`VorbisComments`, while the reader reported `APE`/`VorbisComment`, and `_TAG` lists every tag in the file. It now reads `_TAG_READ` (the displayed tag) and `tags::keeps_field_names`. The reader reports APE tags as `APEv2`, as Mp3tag does (`APEv2`, `APEv1` and `VorbisComment` are the names in Mp3tag.exe); `VorbisComment` was already right. The Utils tag-type counter accepts `APEv2`.

Case-only key changes are written: the action host sends the new key, and both the Vorbis and APE writers replace an item whose key matches case-insensitively.

## Tests

- `operations.rs`: destination classification with simulated case-insensitive and case-sensitive folders (aliases, distinct files, hard links, parent spelling); real case-only file renames that never replace another file; hard links stay distinct; folder renames in case and to new paths.
- `tests/history.rs`: case-only file and folder renames are undone and redone.
- `tests/script.rs`: the folder separator option, empty replacement, unsafe replacement characters, validation.
- `tests/converters.rs` (tag-core) and `converters.rs` (app): Filename → Filename keeps one extension; a case-only rearrangement renames on disk.
- `actions.rs`: Format Value and Guess Values produce filename-mode names with their origin; `_FIELDNAME` accepts APEv2 and VorbisComment and refuses ID3v2 and MP4.
- `action_host.rs`: Case Conversion renames a file and its folder by case only; Format Value names follow Tag → Filename rules (sanitized values, literal folders, no doubled extension, separator option); Guess Values with an empty separator; `_DIRECTORY` from a value; `_FIELDNAME` upper-cases a FLAC Vorbis comment name on disk (needs ffmpeg).
- `lib.rs`: Tag → Filename and the inline-edit literal change only case; a different file differing only in case is never replaced.
- Node: Tag → Tag preview keeps one extension, list folders follow case-only folder renames, folder separator validation, `APEv2` tag-type counting.
- Native e2e (CI only): `tests/e2e/rename-fixes.js`, added to the `native-e2e` suite list. Tag → Filename through the dialog with a case-only result and undo; Tag → Tag on `_FILENAME` with a value separator and a typed extension. `tests/e2e/rename-fixes-fixtures.sh` mounts a casefolded ext4 image when the Linux runner allows `sudo` and `mkfs.ext4 -O casefold`; then the suite also checks a case-only file rename, the refusal to replace `Other.mp3`, and a case-only `_DIRECTORY` Case Conversion with undo. Otherwise those steps are reported as skipped.

## Gaps

- Mp3tag's handling of `: * ? " < > |` in tag values is not documented; OpenTag replaces them with `_` (unchanged).
- An alias whose folder above differs only in case keeps the folder's stored spelling; only the last name changes.
- Replace and Regular-expression replace on `_FILENAME` can still produce characters that are invalid on Windows; the operating system then refuses the rename.
- The casefolded e2e steps depend on the runner's kernel and `sudo`; the macOS and Windows behaviour is covered by the cargo tests on those platforms' CI test jobs.
