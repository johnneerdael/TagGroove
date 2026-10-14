# Critical scripting and format audit

Reference: [Scripting functions](https://docs.mp3tag.de/scripting/) and [format strings and placeholders](https://docs.mp3tag.de/format/), read 2026-10-14. Target artifact remains Mp3tag 3.36.1.

All 64 documented scripting function names are implemented by independently authored code. The table records contract tests, not execution against the proprietary reference. Exact 3.36.1 behavior remains unqualified until a differential corpus runs on that release. Documentation can describe a broader/current surface than the particular shipped artifact.

The engine parses reusable `CompiledTemplate` values. It accepts `EvalContext` for actual locale/number settings, clock/timezone, codepage, ordered rendered column values, environment variables, app paths, and raw stored fields. Missing columns/app paths/Windows codepage produce errors instead of fabricated values. `$list` uses `raw_fields` to exclude synthetic aliases. Metadata lookup and `$meta` use the enriched internal field context.

User regular expressions, legacy regex actions, and filter MATCHES use the same open-source Boost Perl engine in `regex-compat`. Internal filename capture patterns remain generated Rust regexes. See the separate regex audit for engine-level coverage.

## Function inventory

Each row is implemented and has the listed contract evidence in `crates/tag-core/tests/script.rs`. Every row still needs original-runtime differential qualification.

| Function | Contract evidence |
| --- | --- |
| `$lower` | `remaining_literal_string_and_metadata_forms` |
| `$upper` | `remaining_literal_string_and_metadata_forms; formatting_nested_quotes_and_presence` |
| `$len` | `unicode_positions_arithmetic_and_replacement` |
| `$left` | `formatting_nested_quotes_and_presence` |
| `$right` | `unicode_positions_arithmetic_and_replacement` |
| `$mid` | `unicode_positions_arithmetic_and_replacement` |
| `$cutleft` | `documented_function_families` |
| `$cutright` | `documented_function_families` |
| `$trim` | `documented_function_families` |
| `$trimleft` | `documented_function_families` |
| `$trimright` | `documented_function_families` |
| `$caps` | `documented_function_families` |
| `$caps2` | `documented_function_families` |
| `$caps3` | `sentence_case_uses_only_explicit_trigger_characters` |
| `$ansi` | `ansi_conversion_uses_real_encoding_and_replaces_unrepresentable_characters; ansi_conversion_roundtrips_encoded_glyphs_instead_of_assuming_identity` |
| `$fmtdate` | `localized_dates_cover_every_documented_pattern` |
| `$fmtduration` | `durations_use_units_hours_and_milliseconds; durations_avoid_float_rounding_and_handle_invalid_numbers` |
| `$fmtnum` | `number_grouping_follows_explicit_locale; explicit_system_number_settings_override_locale_defaults` |
| `$folderdepth` | `folder_depth_understands_cross_platform_paths` |
| `$num` | `unicode_positions_arithmetic_and_replacement` |
| `$replace` | `unicode_positions_arithmetic_and_replacement` |
| `$regexp` | `boost_regex_is_used_end_to_end_by_scripting_and_legacy_actions; regex_boost_references_and_ascii_digits` |
| `$strstr` | `documented_function_families` |
| `$strrstr` | `documented_function_families` |
| `$strchr` | `documented_function_families` |
| `$strrchr` | `documented_function_families` |
| `$strcmp` | `documented_function_families` |
| `$stricmp` | `documented_function_families` |
| `$reverse` | `documented_function_families` |
| `$repeat` | `documented_function_families` |
| `$char` | `documented_function_families` |
| `$ord` | `documented_function_families` |
| `$sort` | `documented_function_families` |
| `$dedup` | `documented_function_families` |
| `$validate` | `documented_function_families` |
| `$distance` | `documented_function_families` |
| `$if` | `booleans_are_typed_instead_of_numeric_text` |
| `$if2` | `booleans_are_typed_instead_of_numeric_text` |
| `$ifgreater` | `unicode_positions_arithmetic_and_replacement` |
| `$iflonger` | `documented_function_families` |
| `$eql` | `unicode_positions_arithmetic_and_replacement` |
| `$neql` | `documented_function_families` |
| `$grtr` | `documented_function_families` |
| `$geql` | `documented_function_families` |
| `$less` | `documented_function_families` |
| `$leql` | `documented_function_families` |
| `$and` | `booleans_are_typed_instead_of_numeric_text` |
| `$or` | `booleans_are_typed_instead_of_numeric_text` |
| `$not` | `documented_function_families` |
| `$odd` | `documented_function_families` |
| `$isdigit` | `documented_function_families` |
| `$add` | `unicode_positions_arithmetic_and_replacement` |
| `$sub` | `unicode_positions_arithmetic_and_replacement` |
| `$mul` | `documented_function_families` |
| `$div` | `unicode_positions_arithmetic_and_replacement` |
| `$mod` | `unicode_positions_arithmetic_and_replacement` |
| `$rand` | `random_and_uuid_functions_return_valid_values_and_enforce_arity` |
| `$uuid` | `random_and_uuid_functions_return_valid_values_and_enforce_arity` |
| `$getenv` | `environment_and_app_paths_are_explicit_and_absent_variables_are_empty` |
| `$columnnames` | `column_functions_require_context_and_keep_column_order` |
| `$columnvalues` | `column_functions_require_context_and_keep_column_order` |
| `$meta` | `formatting_nested_quotes_and_presence` |
| `$meta_sep` | `formatting_nested_quotes_and_presence` |
| `$list` | `list_handles_arbitrary_fields_multivalues_and_explicit_delimiters; raw_list_excludes_phantom_aliases_and_preserves_actual_names` |

## Generating format rules

| Rule | Implementation and evidence |
| --- | --- |
| Arbitrary `%field%` names | Resolve supplied fields without a fixed field-name list; `formatting_nested_quotes_and_presence`, `all_documented_placeholders_resolve_from_supplied_context`. |
| Raw quoted literals | Ignore reserved syntax inside single quotes; doubled apostrophes render one apostrophe; `optional_sections_track_resolved_fields_through_nested_functions`. |
| Reserved `[]$%` characters | Quote them to emit literals; the same test verifies an unparsed combined literal. |
| Optional `[...]` | Retain only sections whose evaluated contents resolve a nonempty field; nested functions and nested optional sections preserve presence; `optional_sections_track_resolved_fields_through_nested_functions`. |
| Nested function parameters | Compile nested calls and evaluate parameter templates against original field text; literal/function examples and malformed/arity tests. |
| Variable parameter limits | Fewer than 64 string/logical parameters; up to 64 arithmetic parameters; `recursion_and_arithmetic_limits`. |
| Explicit directory separators | Filename rendering tracks literal AST nodes inside optional and conditional branches; `filename_mode_preserves_documented_optional_directories`, `filename_mode_keeps_selected_literal_conditional_path_branches`. |
| Sanitize after evaluating | Filename components originating from metadata or nonconditional functions are sanitized after evaluation; replacements/comparisons see unchanged source text; `filename_mode_sanitizes_derived_values_after_evaluation`. |

`evaluate_filename_with_context` returns the generated path string without automatically adding a file extension or performing filesystem operations. The native adapter must retain extensions, validate destinations, and perform file operations. In this engine, conditional functions preserve the selected branch's literal separators. Other function-produced separators become underscores: `$upper('folder/file')` yields one component. This provenance policy is explicit and tested, but equivalence for every function-produced path is **not established by the documentation or original runtime**.

## Matching format rules

| Rule | Implementation and evidence |
| --- | --- |
| Matching differs from generating | Matching patterns use literal text plus placeholders; quotes/brackets are literal matching delimiters rather than generating syntax; `matching_format_strings_treat_quotes_and_brackets_as_literals`. |
| Whole filename structure | Reject a nonmatching literal suffix without returning partial captures; `filename_captures_are_whole_and_literal`. |
| Filename extension omission | Ignore the actual final extension unless the pattern explicitly includes it; preserve dots inside titles; `folder_patterns_use_relative_suffix_and_only_remove_real_extension`. |
| Folder structures/backslashes | Match Windows-style folder patterns against normalized cross-platform paths; relative patterns address the final corresponding folder components; same test. |
| Explicit drive prefixes | Match rooted drive patterns completely instead of extracting accidental partial fields; same test. |
| `%dummy%` | Match/discard a segment without creating a field; `filename_captures_are_whole_and_literal`. |
| Repeated placeholders | Accumulate captures as multiple values. Native-runtime equivalence remains pending; same test records the current behavior. |
| Adjacent captures | Report ambiguity instead of guessing a split. Original malformed/ambiguous behavior remains pending. |
| Unicode captures and literal delimiters | Preserve captured text; escaping literal matching delimiters prevents interpretation as regex; filename tests. |

## Date/time pattern inventory

All documented patterns are exercised together by `localized_dates_cover_every_documented_pattern`: `yyyy`, `yy`, `MMMM`, `MMM`, `MM`, `M`, `dddd`, `ddd`, `dd`, `d`, `HH`, `H`, `hh`, `h`, `mm`, `m`, `ss`, `s`, and the compatibility aliases `EEEE`, `EE`. Explicit Unix timestamps, deterministic current-time context, timezone offsets and localized names are tested. Native OS date-name differences and DST behavior still require cross-platform/reference tests.

## Placeholder inventory

These 72 technical/dynamic/export/playlist names all resolve correctly **when their values are supplied in Fields**. `all_documented_placeholders_resolve_from_supplied_context` verifies lookup for every name. It does not prove that the audio/native/export adapters compute the values. Those computation and format-specific availability obligations belong to the mapping/native/export inventories.

| Placeholder | Resolution | Actual value production |
| --- | --- | --- |
| `%_bitrate%` | Contract tested | Inspect mapping/native/export inventory |
| `%_bitspersample%` | Contract tested | Inspect mapping/native/export inventory |
| `%_codec%` | Contract tested | Inspect mapping/native/export inventory |
| `%_cover_description%` | Contract tested | Inspect mapping/native/export inventory |
| `%_cover_mimetype%` | Contract tested | Inspect mapping/native/export inventory |
| `%_cover_size%` | Contract tested | Inspect mapping/native/export inventory |
| `%_cover_type%` | Contract tested | Inspect mapping/native/export inventory |
| `%_cover_height%` | Contract tested | Inspect mapping/native/export inventory |
| `%_cover_width%` | Contract tested | Inspect mapping/native/export inventory |
| `%_covers%` | Contract tested | Inspect mapping/native/export inventory |
| `%_id3v2_character_encoding%` | Contract tested | Inspect mapping/native/export inventory |
| `%_id3v2_unknown_frames%` | Contract tested | Inspect mapping/native/export inventory |
| `%_length%` | Contract tested | Inspect mapping/native/export inventory |
| `%_length_seconds%` | Contract tested | Inspect mapping/native/export inventory |
| `%_mode%` | Contract tested | Inspect mapping/native/export inventory |
| `%_mp4_creation_time%` | Contract tested | Inspect mapping/native/export inventory |
| `%_mp4_creation_time_raw%` | Contract tested | Inspect mapping/native/export inventory |
| `%_mp4_modification_time%` | Contract tested | Inspect mapping/native/export inventory |
| `%_mp4_modification_time_raw%` | Contract tested | Inspect mapping/native/export inventory |
| `%_samplerate%` | Contract tested | Inspect mapping/native/export inventory |
| `%_subsong%` | Contract tested | Inspect mapping/native/export inventory |
| `%_subsong_count%` | Contract tested | Inspect mapping/native/export inventory |
| `%_tag%` | Contract tested | Inspect mapping/native/export inventory |
| `%_tag_read%` | Contract tested | Inspect mapping/native/export inventory |
| `%_tag_size%` | Contract tested | Inspect mapping/native/export inventory |
| `%_tag_size_appended%` | Contract tested | Inspect mapping/native/export inventory |
| `%_tag_size_prepended%` | Contract tested | Inspect mapping/native/export inventory |
| `%_tool%` | Contract tested | Inspect mapping/native/export inventory |
| `%_vbr%` | Contract tested | Inspect mapping/native/export inventory |
| `%_video_bitrate%` | Contract tested | Inspect mapping/native/export inventory |
| `%_video_codec%` | Contract tested | Inspect mapping/native/export inventory |
| `%_video_height%` | Contract tested | Inspect mapping/native/export inventory |
| `%_video_width%` | Contract tested | Inspect mapping/native/export inventory |
| `%_directory%` | Contract tested | Inspect mapping/native/export inventory |
| `%_extension%` | Contract tested | Inspect mapping/native/export inventory |
| `%_filename%` | Contract tested | Inspect mapping/native/export inventory |
| `%_filename_ext%` | Contract tested | Inspect mapping/native/export inventory |
| `%_folderpath%` | Contract tested | Inspect mapping/native/export inventory |
| `%_path%` | Contract tested | Inspect mapping/native/export inventory |
| `%_parent_directory%` | Contract tested | Inspect mapping/native/export inventory |
| `%_volume%` | Contract tested | Inspect mapping/native/export inventory |
| `%_workingdir%` | Contract tested | Inspect mapping/native/export inventory |
| `%_workingpath%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_create_date%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_create_datetime%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_create_datetime_raw%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_mod_date%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_mod_datetime%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_mod_datetime_raw%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_size%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_size_bytes%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_size_kb%` | Contract tested | Inspect mapping/native/export inventory |
| `%_file_size_mb%` | Contract tested | Inspect mapping/native/export inventory |
| `%_app%` | Contract tested | Inspect mapping/native/export inventory |
| `%_counter%` | Contract tested | Inspect mapping/native/export inventory |
| `%_date%` | Contract tested | Inspect mapping/native/export inventory |
| `%_datetime%` | Contract tested | Inspect mapping/native/export inventory |
| `%_filename_rel%` | Contract tested | Inspect mapping/native/export inventory |
| `%_folderpath_rel%` | Contract tested | Inspect mapping/native/export inventory |
| `%_crc%` | Contract tested | Inspect mapping/native/export inventory |
| `%_md5%` | Contract tested | Inspect mapping/native/export inventory |
| `%_md5audio%` | Contract tested | Inspect mapping/native/export inventory |
| `%_max_counter%` | Contract tested | Inspect mapping/native/export inventory |
| `%_total_files%` | Contract tested | Inspect mapping/native/export inventory |
| `%_total_size%` | Contract tested | Inspect mapping/native/export inventory |
| `%_total_size_raw%` | Contract tested | Inspect mapping/native/export inventory |
| `%_total_time%` | Contract tested | Inspect mapping/native/export inventory |
| `%_total_time2%` | Contract tested | Inspect mapping/native/export inventory |
| `%_total_time_raw%` | Contract tested | Inspect mapping/native/export inventory |
| `%_playlist_filename%` | Contract tested | Inspect mapping/native/export inventory |
| `%_playlist_filename_ext%` | Contract tested | Inspect mapping/native/export inventory |
| `%_playlist_folderpath%` | Contract tested | Inspect mapping/native/export inventory |

## Remaining fidelity and delivery requirements

- Run differential behavior against Mp3tag 3.36.1, including empty/multiple fields, optional presence, numeric coercion/overflow, literal-Boolean flags, supplementary Unicode positions, and every function family.
- Verify default capitalization delimiters, locale-sensitive sorting, `$folderdepth` drive/root/UNC conventions, `$fmtDuration` padding/negative/precision conventions and the current `$rand` range/UUID representation. Current implementations have tests; these details are not fully specified by the docs.
- Supply the actual Windows ANSI codepage and system-specific numeric formatting overrides; CLDR defaults are not proof of customized OS settings. Validate less-common codepages and localized date names on each platform.
- Feed real ordered File List column values and real application/configuration paths into every application entry point using context-dependent functions.
- Apply reviewed filename outputs or an equivalent receipt for nondeterministic templates. Reevaluating `$rand`, `$uuid`, or current time can change a previewed destination.
- Verify supplement-plane Unicode behavior against the Windows binary. String positions currently count Unicode scalar values while the Boost bridge operates on UTF-16; original position semantics are not proven.
- Input/output/nesting/work limits are explicit errors. These limits are implementation constraints, not parity evidence. Date/time functions currently validate malformed numeric inputs instead of claiming undocumented coercion compatibility.

## Reproduction

```sh
cargo test -p tag-core --test script --test filter
cargo test -p regex-compat
cargo clippy -p tag-core --test script --test filter -- -D warnings
```

No original-runtime qualification or complete cross-platform release claim follows from these commands.
