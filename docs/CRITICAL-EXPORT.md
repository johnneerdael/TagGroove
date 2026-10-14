# Critical export audit

Reference: [Export](https://docs.mp3tag.de/export/), [format strings](https://docs.mp3tag.de/format/), and [scripting](https://docs.mp3tag.de/scripting/), read 2026-10-14. Target artifact: Mp3tag 3.36.1. The exporter is independently authored Rust; original bundled templates were inspected only as external reference material and were not copied into the app.

This document records implemented contracts and executable tests. It does **not** establish byte-for-byte equivalence with the proprietary executable. Original-runtime differential qualification remains outstanding, especially the behaviors listed below.

## Portable API and native boundary

`crates/tag-core/src/export.rs` exports these functions:

```rust
render_export(template: &str, rows: &[ExportRow], options: &ExportOptions)
    -> Result<Vec<ExportDocument>, String>
encode_export_document(document: &ExportDocument, ansi_codepage: Option<&str>)
    -> Result<Vec<u8>, String>
decode_export_template(bytes: &[u8], ansi_codepage: Option<&str>)
    -> Result<String, String>
requested_hash_fields(template: &str, options: &ExportOptions)
    -> Result<Vec<String>, String>
```

`ExportRow` carries actual stored `fields`, enriched `context_fields`, the source path, duration in seconds, byte size, and optional ordered rendered columns. Stored fields remain separate for `$list`; generated technical values and aliases belong to `context_fields`. File hashes are accepted **only** from `context_fields`, never from same-named stored tags, and are validated as hexadecimal values of the expected length. The host must provide independently computed values and must not copy raw underscore tags into trusted hash slots.

`ExportOptions` carries the fallback filename, explicit filename override, base directory, encoding/BOM, optional partition expression, sorting settings, clock/locale/codepage/number/environment/application context, and output/work limits. `ExportDocument` contains the resolved filename, normalized encoding, BOM flag, and rendered Unicode text.

The renderer performs no file reads, writes, application launches, or network operations. The native host resolves template names or paths, supplies audio/file information, reads real hashes, validates all destinations before writing, protects input audio files, implements append and directory-creation settings, and shows confirmation/open-file controls. On append to a nonempty file, the host must suppress a second BOM. Template collection add/duplicate/remove/rename/edit operations also belong to the host/UI.

## Export directives

The documented directive inventory is complete. Each directive is parsed structurally, with normal scripting available inside its parameters.

| Directive | Implemented behavior | Evidence in `tests/export.rs` |
| --- | --- | --- |
| `$filename(name,enc)` | Leading optional directive; name or extension and optional `ansi`, `utf-8`, or `utf-16` encoding. Misplaced directives fail. | `csv_export_sorts_and_keeps_exact_template_line_endings`; `explicit_dialog_filename_override_and_encoding_bom_are_separate` |
| `$loop(sort,num)` | Sort input, iterate, optionally limit entries per equal-key group. Nested loops create grouped output. | `unsorted_loop_and_per_key_limits_do_not_become_global_limits`; `nested_loops_form_groups_and_restore_outer_counters` |
| `$loopend()` | Close the corresponding loop; unmatched or malformed terminators fail. | `malformed_templates_invalid_rows_and_colliding_partitions_fail_before_writes` |
| `$get(name)` | Fetch the current document's variable, empty if unset. | `variables_work_inside_normal_functions_and_retain_typed_flags` |
| `$put(name,value)` | Assign and emit the value. | Same test |
| `$puts(name,value)` | Assign without emitting. | Same test |

The implementation compiles the export program once per render call. Ordinary expressions use the shared `CompiledTemplate` evaluator, including all 64 documented functions and the same Boost regular-expression engine. Thin opt-in extension hooks provide variables, loops, and live dynamic fields. Ordinary scripting outside export still rejects export-only functions. Variables preserve text, placeholder-presence, and boolean flags, rather than reconstructing truth from printed numbers.

Loop sort keys are evaluated once per row. A constant key retains input order; bare field names resolve metadata. Comparison settings are explicit: default case-insensitive Unicode lowercase keys with lexical order, optional natural ASCII-digit ordering, or case-sensitive ordering. Distinct spellings such as `01` and `1` remain distinct duplicate groups. Sort expressions see the local input selection index, enabling reversal with `$loop($sub(999,%_counter%))`. Nested bodies receive all rows in the current equal-key group; leaf loops emit each retained row. A `num` limit applies separately to each equal-key group. These are tested implementation choices pending reference qualification.

## Filename, partition, and encoding contracts

Partition expressions group rows by their evaluated value in first-occurrence group order. Each group receives fresh variable state and an independently evaluated filename. Destination collisions fail before any document is returned. There is no additional export filter directive in the cited documentation; callers supply the selected/filtered rows.

Filename precedence is explicit: evaluate `options.filename` as the fallback, apply the leading `$filename` name or bare extension, then apply `filename_override` if supplied. An empty directive name retains the fallback. Relative directive names resolve beside the fallback output; remaining relative paths resolve against `base_directory`. Lexical normalization handles slash styles, repeated separators, and dot components. This normalization performs no filesystem access or symlink resolution.

The encoder writes UTF-8, UTF-16 little endian, or an explicitly supplied ANSI codepage. Unicode encodings can include a BOM; ANSI output never includes one. ANSI-unrepresentable characters become `?`, not HTML numeric entities. The template decoder accepts UTF-8/UTF-16 BOMs, otherwise valid UTF-8, otherwise an explicit legacy codepage. Invalid encoded input is rejected. Endianness, exact line endings, and literal text are retained; no automatic HTML/CSV escaping occurs.

Relevant tests: `partitioned_exports_evaluate_filename_per_group_and_reset_variables`, `fallback_filenames_are_generating_expressions_and_paths_are_normalized`, `mte_decoding_supports_boms_and_explicit_legacy_codepages`, `script_columns_and_csv_or_html_escaping_use_normal_scripting`, and the filename/encoding tests above.

## Export placeholder production

The [format reference](https://docs.mp3tag.de/format/) identifies relative paths, checksums, loop counters, and aggregate placeholders for export. The following table describes this implementation's production rules.

| Names | Implementation | Contract evidence |
| --- | --- | --- |
| `_counter` | Live local loop position; outer state restored after nested loops. Read through either a placeholder or `$meta`. | `nested_loops_form_groups_and_restore_outer_counters`; `loops_can_occur_inside_conditionals_without_stale_counter_snapshots` |
| `_max_counter` | Emitted iteration count of the last closed loop. | Nested-loop and unique-loop tests |
| `_total_files`, `_total_size_raw`, `_total_time_raw` | Checked aggregate source count, bytes, and rounded seconds; unique-output limits do not discard source totals. | `global_totals_cover_all_source_rows_even_when_unique_output_is_limited` |
| `_total_size` | Binary size units with two decimal places and locale decimal separator. | Same aggregate test |
| `_total_time`, `_total_time2` | Duration with optional days, or total hours without days. | Same aggregate test |
| `_filename_rel`, `_folderpath_rel` | Relative to the actual resolved export destination; different Windows drives retain absolute paths. | `relative_paths_follow_actual_output_location_and_keep_other_drives_absolute` |
| `_crc`, `_md5`, `_md5audio` | Require trusted host-supplied hashes; never read files or silently fabricate values. | `hash_placeholders_require_real_supplied_values_including_meta_access`; `raw_tags_cannot_impersonate_file_hashes_even_through_dynamic_meta` |

Current aggregate state initially covers the document's rows, covers the current equal-key group during iteration, and covers the completed loop's full source scope after closure. Invalid negative/nonfinite durations and byte-size overflow fail. Other technical placeholders resolve the supplied context; missing basic path/size/duration values are supplemented from `ExportRow`.

`requested_hash_fields` discovers direct hash placeholders and static `$meta`/`$meta_sep` names in the body and filename/override/partition expressions, ignoring quoted literal text. Dynamic names such as `$meta($get(name))` cannot be predicted by static scanning. They still pass through the runtime trust check and fail explicitly unless the host supplied the requested hash. The native host may choose to precompute additional hashes; the pure renderer has no lazy hash callback.

## Validation and remaining qualification

Validation checkpoint: 21 export tests, 3 scripting-extension tests, 35 scripting tests, and 14 filter tests. The extension tests verify typed optional/boolean results, dynamic field evaluation order, and scoped larger output limits while retaining ordinary scripting defaults. Export output defaults to a 256 MiB per-document limit and ten million counted steps. Nesting and malformed syntax are bounded and rejected. Strict clippy is run for the export and extension test targets.

Independent tiny CSV, HTML-like text, grouped summaries, Unicode paths, quotes, dynamic counters, encodings, partition collisions, and hash provenance are covered. Passing these tests proves the local contracts, not original application equivalence. Required remaining comparisons include:

- Nested-loop representative-row selection, unusual constant/bare-field sort expressions, `num` corner cases, equal-key case behavior, natural ordering, and native locale collation.
- Counter state before/after loops, aggregate scope in more complex sibling/conditional loops, fractional-duration rounding, size-unit formatting, and very large totals.
- Variable name case, unset-variable presence/truth, assignments inside skipped branches, and unusual variable/loop interactions.
- Whitespace/newlines adjacent to `$filename`, all legacy MTE encodings/codepages, BOM defaults, filename empty/extension edge cases, and platform path behavior.
- Runtime-discovered hash names and actual native `_md5audio` behavior for every supported audio format. Successful field lookup is not proof of audio hashing.
- Cross-platform filesystem execution, append semantics, directory options, template collection UI, and opening exported files. These require native/UI evidence outside this pure module.

Default sorting currently uses deterministic Rust string comparison, not Windows locale collation. Default formatted aggregate sizes use English when no locale is supplied. Callers should supply actual locale settings; exact host-locale and proprietary formatting equivalence remains unqualified. The exporter preserves a newline following `$filename` because it is template content; any original special trimming behavior requires differential evidence before changing that rule.

## Equal-key loop storage performance

Loop frames share immutable `Rc<[usize]>` member indices. A group allocates its member storage once; advancing a row or saving the parent frame increments a reference count rather than copying every index. This removes quadratic index copying from large constant/equal-key loops without changing public APIs or row order.

`large_equal_key_loop_retains_every_row_and_counter_in_input_order` checks all 40,000 titles and counters, final totals, and the complete output string. One unoptimized full-render sample on the development host measured 1.391 s before the storage change and 1.349 s after. These single samples do not establish a significant application-level speedup; script evaluation still dominates this case.

An isolated optimized Rust measurement cloned the same `0..n` member array once per row, passed each clone through `std::hint::black_box`, and read its current index. The comparison used `Vec::clone` versus `Rc::clone` with the same input and iteration count. Results on the development host:

| Rows in one group | Vector storage copying | Shared storage cloning |
| --- | --- | --- |
| 20,000 | 56.849 ms | 30.875 µs |
| 40,000 | 585.773 ms | 192.708 µs |
| 80,000 | 2.327 s | 130.583 µs |

These are individual microbenchmark samples, not portable throughput guarantees. Reference-count timings at this scale are noisy. The structural improvement is independent of the timing: a 40,000-row group on a 64-bit target previously copied approximately 12.8 GB of indices across its row iterations; shared storage copies none of those arrays.
