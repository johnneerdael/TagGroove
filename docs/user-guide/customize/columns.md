# Customize file-list columns

Choose **View → Customize columns…** to decide what the file list displays,
which values can be edited there, and how each column sorts. A column can
show a computed value while editing a different underlying field, or can
be read-only. Changing a view definition does not convert the audio format.

## Add an editable recording-code column

1. Select a copied practice file whose ISRC field contains the invented
   practice value `QZABC2400003`.
2. Open **View → Customize columns…** and choose **New**.
3. Set **Name** to `Recording code` and **Value** to `%isrc%`.
4. Set **Field (empty = read-only)** to `%isrc%`. Leave **Sort by (empty =
   value)** empty and **Numeric (sort numerically)** off.
5. Set a suitable width, such as `140`, and keep its visibility checkbox on.
6. Inspect the active-file preview, then choose **OK** to save/apply the layout.

The header says Recording code, its value comes from ISRC, and the editable
field is still ISRC. Merely naming a column ISRC would not establish that
connection. Canceling the customization dialog leaves its unconfirmed layout
edits unapplied.

## Understand Name, Value, Field and Sort by

| Control | Purpose |
| --- | --- |
| **Name** | Literal header text. Required; it is not evaluated as a format string. |
| **Value** | Required display format string, using placeholders/functions. |
| **Field (empty = read-only)** | One writable field placeholder, or empty for a read-only column. It is not a write-back formula. |
| **Sort by (empty = value)** | Optional sort format. Empty uses Value as the sort key. |
| **Width** | Column width, constrained to 24–4,000 pixels. |
| **Alignment** | Left, Center or Right text alignment. |
| **Numeric (sort numerically)** | Compare leading numeric values before the text fallback. |

For these examples, the practice file has:

<!-- column-example-fields:start -->
| Field | Value 1 | Value 2 |
| --- | --- | --- |
| `ISRC` | `QZABC2400003` | *(absent)* |
| `ARTIST` | `Juniper Vale` | `Mira Solberg` |
| `TITLE` | `Paper Boats` | *(absent)* |
<!-- column-example-fields:end -->

<!-- column-expression-examples:start -->
| Name | Value | Field | Sort by | Displayed result | Sort key |
| --- | --- | --- | --- | --- | --- |
| `Recording code` | `%isrc%` | `%isrc%` | *(empty)* | `QZABC2400003` | `QZABC2400003` |
| `Credits` | `$meta_sep(artist,' + ')` | `%artist%` | `$lower(%artist%)` | `Juniper Vale + Mira Solberg` | `juniper vale` |
| `Upper title` | `$upper(%title%)` | *(empty)* | `%title%` | `PAPER BOATS` | `Paper Boats` |
<!-- column-expression-examples:end -->

The Credits column explicitly joins separate ARTIST values for display.
Its editable target remains ARTIST, and sorting uses its first artist
lowercased. The Upper title column is read-only because Field is empty;
its uppercase display does not silently uppercase the stored TITLE.

A Field entry accepts a tag placeholder such as `%isrc%`, plus the special
filename targets `%_filename%` and `%_filename_ext%`. Technical properties
such as `%_bitrate%` cannot be written there. Scripting functions, several
placeholders or literal surrounding text are not valid Field targets.
To calculate and save a new metadata value, use
[Tag → Tag](../converters/tag-to-tag.md) or
[Format Value](../actions/format-value.md) instead.

## Sort and alignment rules

Click a header to sort its configured sort key, and click it again to
reverse the primary direction. Shift-click adds/reverses a secondary key.
The [file-list guide](../guides/file-list.md) explains ordering and how it
feeds numbering and positional imports.

Numeric sorting reads a leading number; empty/nonnumeric values sort before
valid numbers in ascending order, with text comparison resolving ties.
Enabling Numeric changes Left alignment to Right; disabling it changes
Right to Left. Alignment can then be chosen independently, and Center is
not forcibly replaced by that toggle.

A display like `%_length%` can use a numeric technical value as Sort by
when its formatted clock text is not the desired sorting representation.
Use actual information placeholders from the
[placeholder reference](../reference/placeholders.md); a custom tag named
LENGTH is not a measured duration.

## Add, duplicate, move, hide or remove

- **New** adds a column after the current one, initially using ISRC.
- **Duplicate** copies the current definition into a separately identifiable
  column. Give it a useful name and review its editable Field.
- **Move up** / **Move down** change column order. Headers can also be dragged
  directly in the file list; dividers resize them.
- A checkbox next to a column name shows or hides it. Keep at least one
  column visible.
- **Remove** deletes the column definition, not the file's metadata. A removed
  column's sort criterion is removed too.
- **Reset to defaults** replaces the draft layout with the built-in columns;
  review it and choose OK only when that reset is intended.

Right-clicking a header or using the View menu's column list offers quick
visibility controls. Those quick changes apply and persist directly; check
reported errors. The customization dialog instead collects layout edits
until its OK step. Auto-size and divider double-clicks fit widths to measured
listed content within limits; long lists are sampled, so some values can
still need manual widening.

## Save, load and import a layout

**Save configuration…** writes the current dialog's column layout to a JSON
file. It can export a valid draft before OK; canceling later does not remove
that external file. **Load configuration…** fills the customization dialog
from a supported layout. Review it, then choose OK to apply it.

The picker can also import a user-supplied Mp3tag `columns.ini`. Review the
reported imported/skipped items and the resulting expressions; unsupported
settings are not a promise of identical appearance or behavior.

A column-layout file contains the layout and sort configuration. It is
separate from backing up audio tags or saving the whole application
configuration through the [File menu](../reference/menus/file.md).

## Gotchas

- A correct-looking preview does not make an invalid Field writable. Check
  display Value and edit target separately.
- Hiding a column does not erase its tags, and removing it does not remove
  the metadata from a file.
- A custom column can require context-dependent functions. In particular,
  column-name/value functions need a supplied column context and cannot be
  used to recursively evaluate the column itself.
- Large expressions can make previews/sorting more expensive than a plain
  placeholder. Start with one field and add formatting in small steps.

## Related

- [The file list](../guides/file-list.md)
- [Format strings](../reference/format-strings.md)
- [Scripting functions](../reference/scripting/index.md)
- [Placeholder reference](../reference/placeholders.md)
- [View menu](../reference/menus/view.md)

<!-- Source baseline maina9cd900: columns.ts ColumnUI/customize/layoutProblem/editKey/sortTemplate/sortClick/compareCells; main.ts ColumnUI host/columnValues/applyColumns/commitCell; src-tauri column evaluation and configuration commands. All canonical customization/file-list controls/tasks independently explained. No installed native layout or tag-write session. -->
