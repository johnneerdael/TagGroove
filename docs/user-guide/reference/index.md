# Reference

Use the reference to look up exact field names, syntax, storage mappings, and supported file formats. For a complete filtering task, start with the [filtering guide](../guides/filter.md).

| Reference | Look up |
|---|---|
| [Field names](field-names.md) | Editable metadata vocabulary, custom fields, and special action selectors. |
| [Tag mapping table](tag-mapping.md) | The native ID3 frames, MP4 atoms, ASF attributes, Matroska target levels, and RIFF identifiers used for fields. |
| [Supported file formats](file-formats.md) | Recognized extensions, tag read/write/remove capabilities, artwork, chapters, and limitations. |
| [Filter syntax](filter-syntax.md) | Plain search, every filter operator, quoting, precedence, and worked examples. |
| [Format strings](format-strings.md) | Placeholders, functions, and rules for constructing a value. |
| [Placeholders](placeholders.md) | File information and other values available inside format strings. |
| [Regular expressions](regular-expressions.md) | Pattern syntax, replacements, escaping, and Boost-specific behavior. |

## Choose the right kind of name

`ARTIST` is an editable field name. `%artist%` reads its value in a format string. `TPE1` is the ID3v2 frame used to store that field. `ARTIST IS "Juniper Vale"` is a filter expression. These names participate in different layers; a native frame identifier is not automatically an editable field or placeholder.

Technical information such as duration describes a file; editing a similarly named custom field does not change the audio's measured duration. Check [Field names](field-names.md) before creating a field intended to control a player or an action.

## Scope of the reference

These pages describe the inspected OpenTag source and executable documentation examples. Recognition of an extension does not establish playback, DRM support, or compatibility with every application that reads that format. Native/runtime checks and release availability are separate from source inspection.

Additional menu, options, keyboard, and automation references are published as their checked pages become available. Unfinished pages remain outside the public navigation and search.

## Related

- [Filtering the file list](../guides/filter.md)
- [Field vocabulary](field-names.md)
- [Formats and capabilities](file-formats.md)

<!-- verified: sourcebaseline007a806; links intentionally limited to completed baseline pages plus this source-checked reference batch. -->
