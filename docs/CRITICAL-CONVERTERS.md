# Critical converter audit

References read 2026-10-14: [Filename → Filename](https://docs.mp3tag.de/converters/rearrange-filenames/), [Text File → Tag](https://docs.mp3tag.de/converters/import-tags-from-text-files/), and [format strings](https://docs.mp3tag.de/format/). Target artifact: Mp3tag 3.36.1. This is independently authored Rust with no proprietary implementation or bundled template copies.

These converters cover the documented numbered-part rearrangement and line/mask import mechanisms. Tests establish the local implementation's contracts; execution against the original 3.36.1 runtime remains outstanding. Typed native preview/apply services are registered and tested below. The Convert menu exposes Filename → Filename and Text File → Tag through those services. Bounded macOS desktop native workflows now pass with the evidence linked below; other platforms and unexercised branches remain separate verification obligations.

## Pure APIs

`crates/tag-core/src/converters.rs` exposes reusable compiled converters:

```rust
FilenameConverter::compile(old_pattern: &str, new_pattern: &str)
    -> Result<FilenameConverter, String>
FilenameConverter::render(&self, source: &str, context: &EvalContext)
    -> Result<FilenameConversion, String>

TextTagConverter::compile(mask: &str) -> Result<TextTagConverter, String>
TextTagConverter::plan(&self, text: &str, selected_paths: &[String], options: &TextImportOptions)
    -> Result<Vec<TextTagImport>, String>
TextTagConverter::plan_bytes(&self, bytes: &[u8], selected_paths: &[String], options: &TextImportOptions)
    -> Result<Vec<TextTagImport>, String>
```

`FilenameConversion` contains the unchanged `source`, generated `target` including the preserved original extension, and captured `Fields`. `TextTagImport` contains `selection_index`, one-based `source_line`, the selected `path`, and a field patch. It does not replace all existing tags. `TextImportOptions` contains an optional explicit legacy codepage and `case_sensitive_paths` (default true).

The shared matching API is now `CompiledFilenamePattern::compile(mask)` followed by `.parse(path)`. Existing `parse_filename` delegates to it, preserving its extension, folder-suffix, placeholder, and error contracts. Compilation happens once per reusable filename converter, rather than once per selected file. A text converter also compiles its literal matching expression once.

No converter reads or writes files. The native host supplies selected paths in actual selection order, provides real scripting context, resolves a generated relative filename against the working directory, validates every destination and collision, and only then executes the batch. Tag application must preserve fields outside the returned patch and retain format-specific mapping/write policy. Selection/source dialogs, deriving the text source filename from the first selected row, file reads, preview, rollback, and directory creation belong to the host.

## Filename → Filename

The documented old pattern labels up to nine parts using `%1` through `%9`; literal text separates captures, and backslashes include folders. The new pattern reuses parts and permits scripting. A selected file whose name does not match the old pattern is left unchanged ("No match, not changed") and the other files are renamed in one undo step. The core translates numbered references into the shared matcher/evaluator's internal placeholders while keeping quoted new-pattern text literal.

| Contract | Evidence in `crates/tag-core/tests/converters.rs` |
| --- | --- |
| Rearrange parts and retain the original extension, including its case | `filename_rearrangement_retains_extension_and_uses_numbered_parts` |
| Folder matching, absolute output, scripting, literal brackets | `filename_directory_captures_and_quoted_literals_follow_documented_example` |
| All nine numbered parts; literal quoted `%9`; doubled apostrophe | `all_nine_filename_parts_and_quoted_numbered_literals_are_distinct` |
| Nested optional literal directories; functions see raw captures before filename sanitizing | `filename_context_and_optional_paths_evaluate_captures_before_sanitizing` |
| Reuse compilation across source files without stale captures | `compiled_filename_converter_reuses_values_without_stale_captures` |
| Existing matching behavior survives extraction | `compiled_filename_matcher_preserves_the_existing_public_contract`, plus all 35 existing scripting tests |
| Original metadata/context stays unchanged; `$list` sees raw stored tags | `filename_plans_preserve_original_context_and_do_not_mutate_raw_metadata` |
| Invalid parts, ambiguous adjacency, undefined references, unsupported functions fail | `invalid_filename_masks_and_unknown_numbered_references_fail` |

Literal separators in the new pattern retain the shared AST provenance behavior, including optional and selected conditional branches. Metadata and nonconditional function outputs are sanitized after evaluation. All regular scripting functions and context values remain available; numbered captures override same-named numeric context fields for rendering without modifying supplied metadata.

The generated pattern describes the filename without its automatically retained source extension. This implementation always appends that original extension; explicitly putting an extension in the new pattern can therefore duplicate it. Files with no extension remain extensionless. Empty final components, `.`/`..`, and control-character output fail. Old numbered labels may be used once each; repeated labels fail as ambiguous. Exact original treatment of these unusual forms remains unqualified.

## Text File → Tag

The documentation defines one record per physical line, assigning records by selected-file order. `%dummy%` discards a capture. `_filename_ext` and `_path` enable filename or absolute-path matching instead. The input format is expressed by literal delimiters and `%field%` captures.

Matching consumes the complete line. It never strips an apparent audio extension, removes path fragments, trims whitespace, or treats matching-mask brackets/apostrophes as generating syntax. Arbitrary writable field names are uppercased in the patch; repeated fields accumulate values in capture order. `_filename_ext` and `_path` are matching keys and are never written as tags. Other underscore fields are rejected as read-only.

Quotes are literal delimiters when present in the mask, or captured data otherwise. This is not an RFC CSV reader: there is no automatic doubled-quote unescaping, quote-aware comma detection, or multiline record decoding. A quoted field can contain delimiters when the surrounding literal mask disambiguates them. Line breaks inside a tag require a separate explicit transformation outside this converter.

| Contract | Evidence |
| --- | --- |
| Positional order, Unicode, dots and slash-containing values | `positional_text_import_keeps_selected_order_and_never_drops_dots_or_paths` |
| Literal quotes/separators and discarded dummy fields | `text_separators_and_quotes_are_mask_literals_without_implicit_csv_decoding` |
| UTF-8 BOM, UTF-16 BOM, explicit legacy codepage, strict corruption errors | `text_import_boms_codepages_and_all_line_endings_are_decoded_strictly` |
| Filename keys match reordered source records | `filename_keyed_text_import_matches_reordered_rows_without_storing_the_key` |
| Absolute-path keys disambiguate identical leaf names | `absolute_path_keys_disambiguate_duplicate_filenames_and_normalize_separators` |
| Case policy is explicit; unmatched keys never fall back to position | `text_import_case_matching_is_explicit_and_missing_targets_never_fall_back_to_position` |
| Empty values/repeated fields and literal matching special characters | `empty_values_repeated_fields_and_literal_special_characters_are_preserved` |
| Blank records keep their position; final terminators add no extra record | `text_blank_records_and_trailing_terminators_do_not_shift_selected_rows` |
| Any malformed, duplicate, ambiguous, unknown, or missing target prevents a returned plan | `text_matching_errors_are_atomic_and_report_line_numbers` |

CRLF, LF, and CR are accepted as line terminators. A trailing terminator does not add another record; an interior blank line remains a real record, including an empty tag value if the mask matches it. No blank/header/comment lines are skipped implicitly. Source-line numbers refer to the physical input after a leading BOM is removed.

Positional planning requires exactly one source record per selected file. Keyed planning (`_filename_ext` or `_path`) skips records for files that are not selected and leaves selected files that no record names unchanged; the preview lists them as "No match, not changed", and a list that names no selected file is an error (`keyed_text_import_skips_records_of_other_files_and_leaves_unlisted_files_unchanged`, 2026-10-16). Both fail on duplicate selection paths, repeated matched targets, ambiguous leaf names, and malformed lines. This deliberately explicit batch contract returns no partially usable edits. Native/UI code must not bypass it by writing successful records before validating the entire import.

Path matching normalizes slash direction but does not access the filesystem, resolve symlinks, collapse `..`, or assume case-insensitive filesystems. Case-insensitive mode uses Unicode lowercase strings; exact filesystem/Windows case equivalence requires additional native evidence. `_path` input must be absolute. Native selections should contain resolved paths and the host's actual case policy.

Byte decoding reuses the strict portable decoder: BOM-marked UTF-8/UTF-16 little or big endian first, otherwise valid UTF-8, otherwise the explicitly supplied legacy codepage. Malformed encodings fail without replacement characters. An explicit legacy codepage is a fallback, not an override of valid UTF-8 detection. NUL-containing source data fails. Source data is bounded at 256 MiB and masks/individual lines at 1 MiB.

## Validation and remaining qualification

Validation checkpoint: 18 converter tests and 35 existing scripting tests pass. Strict clippy and formatting checks apply to the converter target and touched files. `large_keyed_import_indexes_the_selection_and_matches_every_reversed_record` validates every field/path/index for 20,000 reversed records; one development-host debug sample took 439 ms. This is an individual local sample, not a cross-platform performance guarantee. The implementation indexes selected targets once instead of searching the file list for each line.

The tests use independently authored tiny fixtures and patterns illustrating the documentation. Still required before claiming original-runtime parity:

- Numbered-label repetition, undefined labels, literal percent escaping, adjacent capture ambiguity, source extensions included explicitly in old/new masks, and conditional/function-produced directory separators.
- Original matching greediness when delimiters occur inside multiple captured fields, empty versus absent values, repeated writable fields, whitespace, and quoting corner cases.
- Mismatched positional source/selection counts, duplicates, blank/header lines, and the original application's continue/skip dialogs. The strict core contract does not reproduce an interactive partial-import workflow.
- Legacy codepage auto-detection, BOMless UTF-16, exact Windows case comparison, lexical path aliases and native path resolution across operating systems.
- The actual native source chooser/format-string resolution, previews, tag patch application, filename batch execution, collision handling, undo, and mobile adapters. Pure planning tests are not native execution evidence.

## Native reviewed preview/apply services

`src-tauri/src/converters.rs` provides four registered Tauri commands. The new Filename → Filename and Text File → Tag controls use these reviewed services; the pre-existing Tag → Filename and Filename → Tag controls retain their older endpoints.

| Command | Input | Output |
| --- | --- | --- |
| `preview_filename_conversion` | `request: FilenameRequest` | `{plan_token, items:[{path,target,captures}]}` |
| `apply_filename_conversion` | The exact same request plus mandatory `planToken` | Refreshed `entries`, `errors`, and actual `renamed:[{from,to}]` |
| `preview_text_import` | `request: TextRequest` | `{plan_token, items:[{selection_index,source_line,path,fields}]}` |
| `apply_text_import` | The exact same request plus mandatory `planToken` | Refreshed `ScanResult` after the complete prepared batch |

Request fields use snake_case inside the request object. Tauri command arguments use the normal camelCase `planToken` argument. For example:

```json
{
  "request": {
    "paths": ["/music/001 Artist - Title.mp3"],
    "old_pattern": "%1 %2 - %3",
    "new_pattern": "%2 - %3-$uuid()",
    "columns": null
  }
}
```

`TextRequest` contains `paths`, `mask`, `options:{ansi_codepage,case_sensitive_paths}`, and a typed `source`: either `{"kind":"text","text":"..."}` or `{"kind":"file","path":"..."}`. File input uses the core's strict BOM/legacy-codepage decoder; inline text is already Unicode. The text source may not alias a selected input audio path. `_path` matching uses the supplied selected-path spellings and explicit case policy, not a new implicit filesystem-alias matcher.

Preview compiles the converter once and plans every row. The filename service supplies enriched technical fields, actual raw fields, ordered columns, application paths and platform codepage through the existing evaluation settings. It resolves generated destinations through the same source-parent/extension policy used by native renaming and the shared `normalized_output_path` helper. Windows verbatim paths retain their namespace/backslashes. Missing destination subdirectories are allowed at preview and are created only when applying a valid plan.

Each preview creates an opaque OS-random UUID receipt stored only in this process. It binds the converter kind, exact serialized request (including input order, masks, columns, text/options), complete source-content SHA256 values, metadata and canonical identities, optional text-file source, and immutable rendered destinations/patches. Receipts expire after five minutes, the store retains at most 32, and any apply attempt consumes the receipt. Request edits, replay, cross-converter tokens, legacy rename tokens, unknown tokens and unrecognized request properties cannot authorize alternative paths or patches. Neither apply endpoint accepts client-supplied destinations or updates.

Every selected source is revalidated before mutation. Full content hashes detect same-size changes even when modification timestamps are restored. Metadata guards also cover permissions, creation/modification information and Unix file identity where available. Direct input-file symlinks are rejected; resolved local file paths are required. Hashing streams in 64 KiB blocks. It deliberately does not cache source hashes across preview/apply, so reviewing large selections costs reads proportional to their total file size.

Filename previews reject duplicate/occupied destinations, selected-source collisions, planned files used as another planned file's parent, and uninspectable paths. Existing ancestor directories are recorded and revalidated; deleting/changing a reviewed directory or retargeting an existing path alias makes the receipt stale before a move. Windows and macOS use conservative case-folded duplicate keys. This can reject case-distinct batches on case-sensitive macOS volumes; Linux uses exact keys. Filesystem-specific Unicode/case behavior still needs platform qualification.

Filename apply retains the exact reviewed random/date/counter results rather than evaluating templates again. Moves use the existing exclusive `rename_no_clobber` implementation. A handled later move failure attempts reverse no-clobber restoration; it does not overwrite an externally changed destination or newly occupied original path. An error identifies any moved file that could not be restored. Swaps/cycles through occupied source paths remain rejected, matching the current native planner's policy.

Text apply stages **all** patches on private sibling copies using the existing `write_fields_summary` implementation. It does not duplicate ID3/Vorbis/MP4 logic. Every format write and validation must succeed on those copies, then all selected sources/text input are revalidated again before the first source replacement. An invalid later row or failed preparation therefore commits no selected audio file. Existing atomic per-file persistence and permission preservation remain in use.

For a handled failure after a text commit, prior commits are restored from original backups where possible. Rollback refuses restoration when the checked postcommit regular-file identity, permissions, timestamps or content differs. The final check-to-replace interval remains subject to outside-writer races. When a checked change is detected, or if restoration fails, the original recovery copy is retained and its path appears in the error. Successful rollback restores original bytes, permissions and modification time; inode/creation identity can change through replacement, as with the existing atomic writer. The implementation is **not** a crash-safe multi-file transaction: process termination, power loss, filesystem failures, and outside writers in the final check/commit interval remain outside its guarantee. Newly created empty rename directories can remain after a handled rollback.

The app's existing file-operation mutex serializes these commands with other native file operations. It cannot lock out unrelated processes. GUI callers must reload after an error, display any recovery location, and require a new preview instead of retrying a consumed token.

### Native validation checkpoint

On macOS with Rust 1.95.0, the complete native library suite passes **47/47**, including **15** new converter service tests on actual WAV files. Strict native all-target clippy and touched-file formatting checks also pass. Initial preview/apply success tests were observed failing before implementation; missing-directory and recovery-copy regressions were also observed failing before their fixes. Tests cover:

- Exact reviewed UUID destinations and byte preservation, single use, request/kind binding and rejection of legacy/guessed tokens or injected destination properties.
- Same-size/restored-mtime source changes, externally changed metadata, missing sources, late occupied targets, missing reviewed directories, duplicate sources/targets, and file-as-parent conflicts, all rejected before source mutation.
- UTF-16 BOM text sources, reordered filename-keyed rows, explicit option/text/source edits, full-mask failures, preserved unmentioned tags and replay rejection.
- A second preparation failure after a real first staged tag write, with neither selected source changed.
- A handled second commit failure restoring earlier real-file bytes, permissions and modification time.
- Detected external content, modification-time-only and Unix permission-only changes during failed rollback remaining untouched, with the original backup retained for recovery.

Checks use a separate absolute target directory and disable Cargo cache auto-clean:

```sh
CARGO_TARGET_DIR=/tmp/opentag-parity-converters-195 CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked -p opentag --lib
CARGO_TARGET_DIR=/tmp/opentag-parity-converters-195 CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never RUSTC_WRAPPER= rustup run 1.95.0 cargo clippy --locked -p opentag --all-targets -- -D warnings
```

These native tests do not claim Windows/Linux, mobile document-URI/save-back, multi-format converter execution or crash recovery. Their evidence is separate from the bounded macOS frontend replay below. Core format tests and existing real-writer tests remain separate evidence.

### Legacy endpoint gap and follow-on work

The older `rename_files` command still accepts an omitted receipt and can recompute its template; its legacy `rename_plan` uses size/mtime rather than full content hashes. The older `filename_to_tags` endpoint also writes directly per file. The existing Tag → Filename and Filename → Tag controls still use those endpoints; this frontend addition does not migrate or qualify them. The new Filename → Filename and Text File → Tag controls use only their required-receipt endpoints and never send client-rendered destinations or patches. Removing or separately qualifying the legacy bypasses and integrating document-access adapters remain follow-on work. No original-runtime equivalence is claimed here.

Filename → Tag leaves a file whose name does not match the pattern unchanged, like Mp3tag: the preview marks it "No match, not changed", Apply writes only the matching files as one undo step, and the status counts the skipped files (`Converted filenames to tags: 8 files, 2 didn't match`). An invalid pattern still fails the whole conversion.


## Frontend integration checkpoint

The existing Convert menu exposes two additional dialogs using the current compact brand/modal/preview system:

- **Filename → Filename** accepts old and new numbered patterns, sends the selected files in their current table order and their actual column specifications, and previews each source, captured field value and full reviewed destination. The Apply label explicitly says Rename.
- **Text File → Tag** offers a native text-file picker and editable source path, or literal pasted records. Its mask, optional explicit legacy-encoding fallback and case-sensitive filename/path matching option map directly to `TextRequest`. Case sensitivity defaults to true. File decoding remains in the native service; pasted records remain Unicode. The preview shows each physical source line, selected-file index/path and individual field values, including empty and multiline values. Apply explicitly saves the reviewed tags immediately.

`src/converter-review.ts` owns a small in-memory review state. A preview captures an independent JSON request snapshot and workspace context; only the current request generation can complete it. Any input/source/encoding/case-option edit, source-picker opening, changed selected paths/order, pending selected edits, workspace data change or dialog close invalidates it. Restoring the previous text does not resurrect a discarded receipt. Apply remains unavailable until the current preview is accepted, consumes it before the native call, and passes the exact captured request plus `planToken`. The frontend expires its review after five minutes; native receipt validation remains authoritative for source-file edits and all filesystem state.

Late preview successes or failures cannot enable Apply or replace a newer result. Filename preview rows must match the reviewed selection order, with files whose name does not match the old pattern listed as unchanged. Keyed text records may be returned in source-line order; each selection index/path matches at most once, and the remaining selected files must be the ones listed as unchanged. Dirty selected files block preview/apply. Missing required inputs keep Preview disabled. Input controls and closing are blocked during Apply, with visible loading state; canceling or editing during Preview remains possible. Focus returns to the Convert menu when closing and to Preview after an Apply error. A control-value tracker ignores duplicate input/change commits, while genuine edits and changed-then-restored values still invalidate the review. Preview/Apply commit editor focus before capturing state; the fixed native first-activation replay now passes within the desktop scope below.

Successful results reuse the existing rename/path/selection reconciliation and metadata acceptance. On an apply exception, the frontend reloads the original reviewed source paths, retains and displays the actual converter/recovery error and any reload errors, and requires a new preview. It does not infer a successful move from a destination named in an error or overwrite unrelated pending drafts. A source that could not be restored can still require the user's explicit recovery from the path reported by the native host.

Validation: `npm run test:ui` passed all **52** Node tests, including **12** review-state/row-contract/event-order tests; `npx tsc --noEmit` passed. Focused tests cover captured request isolation, exact/single-use apply, input and workspace changes, out-of-order success/failure, source/options/order edits, expiry, changed-then-restored inputs, keyed text row order, malformed responses and failed preview state. That initial frontend test checkpoint did not build or operate the native app. The subsequent rebuilt macOS desktop replay is recorded separately below. Portrait/landscape, mobile document-provider compatibility and original-application parity remain unqualified.

### Bounded desktop native workflow checkpoint: PASS

[Converter workflow evidence](verification/converter-workflows.md) and [measured receipts](verification/converter-workflows.json) record actual macOS arm64 native GUI execution on two selected disposable MP3s in six-file libraries. The isolated desktop identifier was `org.opentag.editor.usertest.desktop`; executable SHA256 `ea605ca210c821eb49e7720001877f775c6454a5159522b9e41c5b57ae149307` and archived dirty-source hashes identify the tested build.

The original first Preview activation displayed `Inputs changed. Run Preview again.`; a second Preview worked and preserved exact destinations. After the event-order fix and rebuild, a fresh first activation succeeded. Editing NEW invalidated Rename until a fresh preview; Apply/Reload used both exact reviewed UUID destinations and preserved all six complete-file/audio identities. Undefined numbered references and a true old-pattern nonmatch displayed explicit errors and blocked Rename; Cancel added no mutation.

The standalone Text File → Tag dialog used a real native picker for BOM UTF-16 input. Malformed line 2 was rejected before any write; the valid positional import applied/reloaded only expected title/track values. Case-option changes invalidated the review even when restored. Reversed pasted filename-keyed records visibly mapped line 1→selected file 2 and line 2→file 1, then saved exact Unicode title/comment values while retaining tracks and every other tag value. Preview preserved the relevant saved checkpoint, four unselected complete files stayed unchanged and all six audio payloads were retained.

This qualifies only the recorded desktop success, preview-error, input-invalidation and cancel paths. Five-minute wall-clock expiry, native Apply error/recovery branches, other selected formats, Windows/Linux, mobile layouts and Android/iOS document-provider/save-back remain NOT RUN here. The existing **47/47** native and **52** Node checks, full-workspace/clippy/fmt/frontend/native builds are supporting evidence; none establishes full original parity.
