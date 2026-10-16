# Tags → Mapping

Open **File → Options… → Tags → Mapping** to give a format's fields the names used throughout OpenTag. A mapping changes the field name exposed to the editor and format strings; it does not translate its text or calculate a new value.

!!! tip "New to this?"
    Use a mapping when two formats store the same concept under different field names. Do not use it to extract a year from a full date: that requires a value transformation.

## Add and edit a rule

Choose **New**, then set:

| Control | Meaning |
|---|---|
| **Tag type** | The format family the rule applies to: **APEv2**, **ID3v2**, **MP4**, **Vorbis Comments**, or **WMA**. |
| **Source** | The field name OpenTag otherwise shows for this format. |
| **Target** | The preferred field name to show and use in OpenTag. |

Select an existing row to edit it. **Move up** and **Move down** change list order; **Remove** removes the selected rule. The default list is empty. Press **OK** to save the rules. Clean local files are reloaded; files with pending edits require deliberate saving/discarding and reloading before inspecting their new mapped view.

Source and target must be nonempty names without line breaks. Matching is case-insensitive for validation, and one source in a tag type cannot map to several different targets. Multiple sources may map to the same target.

## Reading mapped fields

Suppose FLAC files contain `DATE=2024-03-15`, while the collection's column and format strings use `YEAR`. Add **Vorbis Comments**, Source `DATE`, Target `YEAR`.

| Stage | Field and value |
|---|---|
| Stored in FLAC | `DATE=2024-03-15` |
| Shown after mapping/reload | `YEAR=2024-03-15` |
| Format string `%year%` | `2024-03-15` |

The value is still a date, even though the displayed name is YEAR. A field-name rule does not validate or shorten it. An ID3v2 rule does not affect the FLAC, because the tag-type choice is part of the rule.

## Saving mapped fields

Writing reverses the mapping. Editing `YEAR` in the example writes the value to the native `DATE` field. If both `DATE` and `RELEASE` map to `YEAR`, a write of that target fans out to both source names.

This makes many-to-one mapping useful for harmonizing a collection, but it also means saving can write more than one underlying field. Avoid merging distinct concepts into one target unless they are intended to receive the same value. Test rules on a copy and inspect the result after reload.

## Field names versus native identifiers

ID3v2 and MP4 already have built-in mappings from native frame/atom identifiers to readable field names. Use the readable OpenTag field name as **Source**, such as `COMPOSER`, rather than entering an ID3 frame identifier or MP4 atom code. This page adds aliases above that mapping layer; it does not replace the built-in identifier-to-field table.

With [reuse of unmodified frames/atoms](tags-advanced.md) enabled, an unchanged field may retain its prior physical representation. Changing the mapping view is therefore distinct from deliberately rewriting a value into a new native location.

## Related

- [Advanced reuse controls](tags-advanced.md)
- [Mpeg policy](tags-mpeg.md)
- [Field values](field-values.md)

<!-- verified: src/field-mapping.ts MAPPING_TAG_TYPES, mappingsError, mappingOptionsPage; crates/tag-core/src/tags.rs map_read_fields/map_write_fields and validate_user_mappings; src/main.ts optionsApplied. Examples derive directly from alias read/write semantics, not runtime UI parity. -->
