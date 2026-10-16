# Tag Panel options

Open **File → Options… → Tag Panel**, then **Customize tag panel…**, to choose the fields shown beside the file list. **View → Customize tag panel…** opens the same editor.

!!! tip "New to this?"
    Add a field for metadata edited frequently, such as `MOOD`. Hiding a control removes it from the panel, not from the audio files.

## Field list

**New** inserts a field after the selected entry. Select a row to edit it; **Move up** and **Move down** change order. Uncheck its box to hide it without removing its configuration. **Remove** deletes the control definition. **Restore defaults** restores the standard layout in this editor; the editor's **OK** saves the result.

Up/Down selects entries. Alt+Up/Down moves the selected entry, and Space toggles its visibility. The inspected implementation allows at most 100 entries. Ordinary fields cannot appear twice; separator entries can.

The initial panel contains Title, Artist, Album, Year, Track, Genre, Comment, Album Artist, Composer, Discnumber, and Directory, with cover art displayed. Most defaults keep each file's existing value.

## Field controls

| Control | Meaning |
|---|---|
| **Name** | Visible label. An `&` before a letter makes it an Alt mnemonic; `&&` displays a literal ampersand. |
| **Field** | Metadata field identifier, such as `TITLE` or `MOOD`. OpenTag normalizes it to uppercase. It is independent of the label. |
| **Default value** | The value staged by default-value workflows; see below. |
| **Type** | **Text**, **Check**, or **Multiline**. Choose Multiline for comments or lyrics. |
| **Size of field** | Layout width: **Full**, **Larger**, **Large**, **Medium**, **Normal**, **Small**, **Smaller**, or **Tiny**. Consecutive fields share a row where their widths fit. |
| **Rows** | Height of a Multiline control, from 1 to 20. Unavailable for other types. |

A label such as `&Mood` gives Alt+M when that mnemonic is available. Alt is Option on macOS. The label is not the saved field name: a label `Mood` with Field `MOOD` still writes `MOOD`.

`_DIRECTORY` adds a folder switcher rather than an editable metadata field. `_SEPARATOR` starts a new layout row, or a new column when the panel is docked above/below the file list. Other technical fields beginning with `_` are not editable panel fields.

## Default values

`< keep >` leaves each file's existing value alone. `< blank >` requests an empty field. Any other default is literal text. Defaults can be staged when multiple files are selected and through **View → Default values in input fields**. A field that already has a pending edit is not overwritten by automatic defaults.

For example, a default `MOOD=Reflective` is useful during a deliberate classification session, but can stage that value when later selecting another batch. Restore `< keep >` when finished. Default text is not a format-string evaluation engine.

A Check control treats empty text and `0`, `false`, or `no` as off, and other nonempty text as on. A mixed selection can have an indeterminate state. Inspect the target field's conventions before using a checkbox for arbitrary text.

## Covers and saved configurations

**Display cover art** controls whether the panel shows its artwork area. [Tags](tags.md) separately controls whether a folder image can stand in for missing embedded artwork.

**Save configuration…** exports the layout currently being edited. **Load configuration…** loads an OpenTag layout for review. **Import Mp3tag configuration…** reads a selected supported Mp3tag panel configuration, reports skipped entries, and places imported controls in the editor. Review that report before the editor's **OK** makes the layout active. Importing a layout does not import the audio collection or tag values.

The editor saves independently of its parent Options dialog. Canceling it discards unsaved layout edits, but does not remove an already exported configuration file.

## Related

- [Field values](field-values.md)
- [Tags and save behavior](tags.md)
- [Appearance](appearance.md)

<!-- verified: src/tag-panel.ts defaultLayout, configure, layoutError, mnemonic, sizeSpan, automaticDefaults, checkState; src-tauri/src/mp3tag_tag_panel.rs import; baseline007a806 includes this editor. Canonical options/tag-panel delegates to customization; detail is original OpenTag reference. -->
