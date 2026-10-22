# Rename & convert

Converters move information between tags, filenames and structured text:
rename files from their tags, fill tags from names, import a tracklist, copy or
calculate one field from another, and number tracks and discs. Choose the
route from the information you already trust to the result you want. A
preview shows the proposed result before you commit it. They live in the
**Convert** menu.

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

## Tutorials and recipes

Start with a tutorial, then use the recipes for specific jobs.

| If you want to… | Open |
|---|---|
| Make filenames match the tags | [Rename files and folders](rename-files-and-folders.md) (tutorial) |
| Recover information from filenames | [Get tags from filenames](tags-from-filenames.md) (tutorial) |
| Use zero-padded numbers, lowercase names or portable characters | [Zero-padded track numbers](recipes/rename-zero-padded-track-numbers.md), [lowercase names](recipes/rename-lowercase-filenames.md), [portable filenames](recipes/rename-safe-characters.md) |
| Sort files into folders | [Artist and album folders](recipes/rename-into-artist-album-folders.md), [multi-disc albums](recipes/rename-multi-disc-albums.md), [compilations](recipes/rename-compilations.md) |
| Fill tags from names or a text file | [Folder and file names](recipes/tags-from-folder-and-filename.md), [a tracklist](recipes/tracklist-from-text-file.md), [lyrics sidecars](recipes/lyrics-from-text-files.md) |
| Number tracks and discs | [Number tracks per folder](recipes/number-tracks-per-folder.md), [disc numbers from folders](recipes/disc-numbers-from-folders.md), [read or split track totals](recipes/split-track-total.md) |
| Derive a credit or sort name | [Fill missing album artists](recipes/album-artist-from-artist.md), [sort a leading “The”](recipes/the-artist-sort-names.md) |

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

- [Scripting: format strings, placeholders and functions](../scripting/index.md)
- [Format strings](../scripting/format-strings.md)
- [Placeholders](../scripting/placeholders.md)
- [Convert menu reference](../reference/menus/convert.md)
- [Choose the right tool](../getting-started/choose-a-tool.md)
