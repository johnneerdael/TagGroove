# Converters

Converters move information between tags, filenames and structured text.
Choose the route from the information you already trust to the result you
want. A preview shows the proposed result before you commit it.

## Choose a converter

| Trusted source | Goal | Converter | Shortcut |
|---|---|---|---|
| Existing tags | Rename or move the files | [Tag → Filename](tag-to-filename.md) | ++alt+1++ |
| File or folder names | Fill tag fields | [Filename → Tag](filename-to-tag.md) | ++alt+2++ |
| Parts of the existing names | Rearrange filenames without first tagging | [Filename → Filename](filename-to-filename.md) | ++alt+3++ |
| One text record per audio file | Fill several fields | [Text File → Tag](text-file-to-tag.md) | ++alt+4++ |
| Existing tag values | Calculate or copy one field | [Tag → Tag](tag-to-tag.md) | ++alt+5++ |
| Current file-list order | Assign track and disc numbers | [Auto-numbering](auto-numbering.md) | ++cmd+k++ / ++ctrl+k++ |

Alt is ⌥ Option on a Mac. Save or discard pending edits and select the
files before opening a converter.

## Read a template versus write a pattern

These two uses of `%title%` look similar but point in opposite directions:

| Context | Meaning of `%title%` | Example |
|---|---|---|
| A format string building output | Read the existing title | `%artist% - %title%` → `Juniper Vale - Paper Boats` |
| A pattern importing information | Capture text into the title field | `%track% - %title%` reads `03 - Paper Boats` |

Output formats can evaluate functions such as `$num(%track%,2)`.
Import patterns describe exact separators and destination names; they do
not execute scripting functions. **Filename → Filename** uses numbered
parts `%1` through `%9` instead of tag-field names in its matching pattern.

## Preview, apply and save

1. Work on copied practice files first.
2. Enter the format or pattern and choose **Preview** when provided.
   **Tag → Tag** refreshes its preview automatically as you type.
3. Check multiple rows, including unusual names and missing fields.
4. Apply the reviewed conversion. If the input or selection changes,
   refresh the preview before applying.

The five converters with arrows save or rename when applied. There is no
second **Save tags** step. **Auto-numbering** is different: **Stage numbers**
creates pending edits, and **Save tags** writes them afterward.

A preview is not a backup or proof that every later disk write will
succeed. Inspect operation errors and the actual files afterward. Use
**Edit → Undo** for recorded changes in the current desktop session, and
keep original copies for recovery beyond that session.

## Related

- [Format strings](../reference/format-strings.md)
- [Placeholders](../reference/placeholders.md)
- [Tags from folder and file names](../cookbook/tags-from-folder-and-filename.md)
