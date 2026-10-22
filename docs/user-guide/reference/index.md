# Reference

Use the reference to look up exact field names, storage mappings, supported file formats, menus, options and shortcuts. To learn a feature, start with its own section instead: [Tagging](../tagging/index.md), [Tag sources](../tag-sources/index.md), [Rename & convert](../converters/index.md), [Actions](../actions/index.md), [Scripting](../scripting/index.md) or [Export & playlists](../export/index.md). For a complete filtering task, start with the [filtering guide](../tagging/filter.md).

| Reference | Look up |
|---|---|
| [Field names](field-names.md) | Editable metadata vocabulary, custom fields, and special action selectors. |
| [Tag mapping table](tag-mapping.md) | The native ID3 frames, MP4 atoms, ASF attributes, Matroska target levels, and RIFF identifiers used for fields. |
| [Supported file formats](file-formats.md) | Recognized extensions, tag read/write/remove capabilities, artwork, chapters, and limitations. |
| [Filter syntax](filter-syntax.md) | Plain search, every filter operator, quoting, precedence, and worked examples. |
| [Format strings](../scripting/format-strings.md) | Placeholders, functions, and rules for constructing a value. (Part of [Scripting](../scripting/index.md).) |
| [Placeholders](../scripting/placeholders.md) | File information and other values available inside format strings. |
| [Scripting functions](../scripting/functions/index.md) | All 64 functions, grouped into six families. |
| [Regular expressions](../scripting/regular-expressions.md) | Pattern syntax, replacements, escaping, and Boost-specific behavior. |
| [Options](options/index.md) | Every setting in the Options dialog, page by page. |
| [Command line](command-line.md) | Open or append files/folders with platform-specific shell examples. |
| [Keyboard shortcuts](keyboard-shortcuts.md) | Focus-sensitive commands, platform modifiers and dialog keys. |
| [Menu reference](menus/index.md) | Visible commands, their input scope and write behavior. |

## Choose the right kind of name

`ARTIST` is an editable field name. `%artist%` reads its value in a format string. `TPE1` is the ID3v2 frame used to store that field. `ARTIST IS "Juniper Vale"` is a filter expression. These names participate in different layers; a native frame identifier is not automatically an editable field or placeholder.

Technical information such as duration describes a file; editing a similarly named custom field does not change the audio's measured duration. Check [Field names](field-names.md) before creating a field intended to control a player or an action.

## Scope of the reference

These pages describe the inspected TagGroove source and executable documentation examples. Recognition of an extension does not establish playback, DRM support, or compatibility with every application that reads that format. Native/runtime checks and release availability are separate from source inspection.

Additional context-menu, options and automation references are published as their checked pages become available. Unfinished pages remain outside the public navigation and search.

## Related

- [Filtering the file list](../tagging/filter.md)
- [Field vocabulary](field-names.md)
- [Formats and capabilities](file-formats.md)

<!-- Source baseline for vocabulary:007a806; navigation references checked against main5f4112e. Links stay limited to completed pages. -->
