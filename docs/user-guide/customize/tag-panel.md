# Customize the tag panel

Choose **View → Customize tag panel…**, the tag panel's **⋯ → Customize tag
panel…**, or its Options page to choose the fields you edit most often.
A control's **Name** is its label; **Field** is the actual metadata name.
Showing, hiding or rearranging a control does not by itself remove metadata
from an audio file.

<!-- figure:customize-tag-panel -->
<figure markdown>
![The Customize tag panel dialog: the list of panel fields and the editor for the selected field's name, field, control type and default](../assets/screens/customize-tag-panel-light.png#only-light)
![The Customize tag panel dialog: the list of panel fields and the editor for the selected field's name, field, control type and default](../assets/screens/customize-tag-panel-dark.png#only-dark)
<figcaption>Each tag-panel control is one field with a type, a size and a default.</figcaption>
</figure>
<!-- /figure -->

## Add a Mood control

1. Open **Customize tag panel…** and choose **New**.
2. Set **Name** to `M&ood` and **Field** to `MOOD`.
3. Keep **Default value** at `< keep >` for this practice change.
4. Choose **Text** as Type and **Normal** as Size of field.
5. Leave its visibility checkbox checked and choose **OK**.
6. Select one copied practice file. The panel now offers Mood; enter `Calm`
   to stage that field, then save tags when the value is intended.

The ampersand marks `o` as the label's Alt mnemonic. It is not part of the
stored field name or value. `&&` writes a literal ampersand. On macOS,
Option combinations can remain text entry in a focused editor; use the
visible control when the keyboard context does not route that mnemonic.

## Every field setting

| Setting | Purpose |
| --- | --- |
| **Name** | Display label; an ampersand can mark its mnemonic. |
| **Field** | Uppercase metadata identifier such as MOOD, ISRC or a custom name; type it without `%` wrappers. |
| **Default value** | Literal value used by the panel's default-filling rules, or the special keep/blank choices below. |
| **Type** | Text, Check or Multiline control. |
| **Size of field** | Relative width: Full, Larger, Large, Medium, Normal, Small, Smaller or Tiny. |
| **Rows** | Visible height of a Multiline control, from 1 to 20 rows. Disabled for other types. |
| Visibility checkbox | Whether this control appears on the panel. |

A normal metadata field can appear only once in the layout. The field input
normalizes names to uppercase and rejects technical information fields,
with the two special layout controls described below. Do not paste
`%mood%` into Field; that placeholder syntax belongs in a format string.

## Understand defaults before selecting a batch

Keep `< keep >` unless a default should deliberately fill selected files.
When several files are selected and the tag panel is visible, its visible
fields with non-keep defaults can be staged automatically. Existing pending
field edits are protected from that automatic fill. The command **Default
values in input fields** applies configured defaults to visible controls explicitly as well.

<!-- panel-default-examples:start -->
| Default value | Intended staged result |
| --- | --- |
| `< keep >` | No update |
| `< blank >` | Remove field |
| `Reviewed` | `Reviewed` |
| `%artist%` | `%artist%` |
<!-- panel-default-examples:end -->

The final row is literal text, not a lookup of the artist. Use
[Tag → Tag](../converters/tag-to-tag.md) or a
[Format Value action](../actions/format-value.md) for calculated metadata.
These defaults produce pending edits; save tags to write them, and inspect
save-on-navigation preferences before assuming they will remain pending.

A non-keep default is not restricted to an empty existing field. It can
replace selected files' loaded values when no pending edit blocks that
fill. Test a default on copied files and restore `< keep >` when it should
not continue filling later selections.

## Choose Text, Check or Multiline

**Text** offers the field's selection values, configured suggestions and
applicable special choices in a drop-down. **Multiline** offers a textarea
for notes/lyrics. Rows changes its visible height, not the allowed number
of text lines or the audio file's tag format. For individually editing
separate values, use [Extended tags](../tagging/extended-tags.md).

**Check** is intended for a flag such as COMPILATION. A checked change stages
`1`; an unchecked change stages clearing the field. The displayed checkbox
can be mixed when selected files disagree. Loaded text `0`, `false`, `no`
and empty text count as off, ignoring case and surrounding spaces. Other
nonempty values count as on; using Check for a free-text field can therefore
normalize its contents when the box is changed.

The following examples refer to one loaded value from each of two selected
files, before any checkbox change:

<!-- panel-check-examples:start -->
| File A value | File B value | Checkbox state |
| --- | --- | --- |
| `1` | `1` | On |
| `0` | *(absent)* | Off |
| `1` | `0` | Mixed |
<!-- panel-check-examples:end -->

An untouched mixed state keeps each file's values. Choosing a real on/off
state intentionally standardizes that field for the selection.

## Arrange, hide or remove controls

Use **Move up** / **Move down** to reorder fields; Alt+Up/Down in the layout
list also moves the current field. Consecutive controls share available
space according to their relative sizes. Full occupies the widest slot;
smaller controls can fit beside each other. Dock position and available
window space affect the final arrangement.

Uncheck a field to hide it temporarily. The tag panel's own menu provides
quick show/hide choices too. **Remove** deletes the layout entry; it does
not delete that field's existing metadata. **Restore defaults** replaces
the draft layout with the built-in one for review before OK.

The panel supports at most 100 layout fields. Invalid names, duplicate
real fields, unsupported sizes/types or out-of-range row counts prevent
saving. Correct the displayed error instead of assuming the saved layout
has changed.

## Directory and separator controls

`_DIRECTORY` is a folder control, not an editable audio tag. Its drop-down
can select files already loaded from a known folder; its browse/new-path
routes can load a folder under the normal workspace/pending-change rules.
It is different from renaming a folder through a converter or action.

`_SEPARATOR` starts a new layout row, or a new column when the panel is
docked above/below the file list. Separators can be repeated. They do not
write a metadata field, and ordinary default filling ignores both special
controls. Other underscore-prefixed technical properties cannot be added
as editable tag-panel fields.

The **Display cover art** checkbox shows/hides the cover section, without
removing embedded images. [Working with covers](../tagging/covers.md)
explains image editing and its separate confirmation paths.

## Supply consistent suggested values

Use **Field values…** or [Options → Field Values](../reference/options/field-values.md)
to maintain a list of composer names, moods or another field's preferred
spellings. A custom list offers suggestions; selecting a field's layout
entry does not rewrite existing tags to match the list.

Shift+Enter in a populated tag-panel field adds its current value to that
field's custom suggestions when valid. Genre has additional standard-list
preferences. The field-values reference explains list import/export and
how selected-file values can take priority over custom suggestions.

## Save, load and import a panel layout

In **Customize tag panel…**, **Save configuration…** writes the current valid
layout to a JSON file. It can save a draft before OK; canceling later does
not remove that external file. **Load configuration…** fills the dialog for
review, and **OK** persists/applies it.

The tag panel's direct **⋯ → Load configuration…** route instead loads and
persists the chosen panel layout immediately. Use View → Show tag panel or
physical Control+Q if the loaded layout hides the panel. Keep a known layout export
before replacing it. This panel file is separate from audio metadata,
custom field-value lists and the whole-application configuration backup.

Where **Import Mp3tag configuration…** is offered, select a user-supplied
`usrfields.ini` or supported saved panel file. Inspect the imported fields
and **Not imported** report, then choose OK to keep the reviewed layout.
Legacy custom suggestions and their per-field preferences appear in the
field-value settings summary and are saved with OK. Matching imported entries
replace their saved counterparts; unrelated lists stay intact. Cancel leaves
the settings unchanged. The separate `usrfieldvalues.ini` is imported through
[Import Mp3tag setup](../getting-started/coming-from-mp3tag.md).
Unsupported controls/settings are not an identical-layout guarantee.

## Related

- [Tag-panel options](../reference/options/tag-panel.md)
- [Field Values options](../reference/options/field-values.md)
- [Extended tags](../tagging/extended-tags.md)
- [Column customization](columns.md)
- [Context and helper menus](../reference/menus/context-menus.md)

<!-- Source baseline maina9cd900: tag-panel.ts configure/layoutError/defaultText/automaticDefaults/checkState/fieldsHTML/bind/command; main.ts TagPanel host/applyAutoDefaults/bindFields; native panel configuration/import validation. All canonical customization/tag-panel settings and tasks independently explained. No native application or metadata-writing session. -->
