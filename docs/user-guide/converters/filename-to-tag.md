# Filename → Tag

Use **Filename → Tag** when the names contain reliable metadata but the
tag fields are missing or wrong. A pattern captures pieces of each name
into fields. It can read parent folders as well as the filename.

<!-- figure:filename-to-tag -->
<figure markdown>
![The Filename → Tag dialog with the pattern %track% - R.E.M. %title% and a preview of the tags it reads from the two selected files](../assets/screens/filename-to-tag-light.png#only-light)
![The Filename → Tag dialog with the pattern %track% - R.E.M. %title% and a preview of the tags it reads from the two selected files](../assets/screens/filename-to-tag-dark.png#only-dark)
<figcaption>The preview shows what each file would give its tags.</figcaption>
</figure>
<!-- /figure -->

## Extract a track number and title

1. Save pending edits and select copied files named like
   `03 - Paper Boats.mp3`.
2. Choose **Convert → Filename → Tag…** (++alt+2++; Alt is ⌥ Option on a Mac).
3. In **Filename pattern**, enter:

    ```text
    %track% - %title%
    ```

4. Choose **Preview**. Check for `TRACK` = `03` and `TITLE` = `Paper Boats`.
5. Choose **Apply to … files** to save the matched files immediately.

The converter replaces fields named in the pattern and retains other
fields. It does not rename the audio file. Files that do not match stay
unchanged; the preview marks them **No match, not changed**.

## Match separators exactly

`%track%` names a destination field. ` - ` is literal text separating it
from `%title%`; its spaces and punctuation must occur in the filename.

| Filename | Pattern | Fields captured |
|---|---|---|
| `03 - Paper Boats.mp3` | `%track% - %title%` | `TRACK` = `03`; `TITLE` = `Paper Boats` |
| `Juniper Vale - Paper Boats.flac` | `%artist% - %title%` | `ARTIST` = `Juniper Vale`; `TITLE` = `Paper Boats` |
| `03_Paper Boats_demo.mp3` | `%track%_%title%_%dummy%` | `TRACK` = `03`; `TITLE` = `Paper Boats` |
| `03. Paper Boats.mp3` | `%track% - %title%` | No match |

The entire candidate name must match. `%dummy%` consumes unwanted text
without writing a field. Captures are not trimmed, so put fixed surrounding
spaces in the pattern rather than capturing them into a field.

The normal pattern omits the audio extension. TagGroove removes the extension
from the name before matching unless the pattern ends in that same literal
extension. Thus `%track% - %title%.mp3` also works for the MP3 example, but
is less useful for a selection mixing formats.

## Read parent folders

For a layout such as:

```text
Music/Juniper Vale/Harbour Lights (2024)/03 - Paper Boats.mp3
```

use backslashes in the pattern, including on macOS and Linux:

```text
%artist%\%album% (%year%)\%track% - %title%
```

The result includes `ARTIST` = `Juniper Vale`, `ALBUM` = `Harbour Lights`,
`YEAR` = `2024`, `TRACK` = `03` and `TITLE` = `Paper Boats`.
Without an absolute root, the pattern reads the required trailing levels
of the path; the earlier `Music` folder is not needed. A leading backslash
or drive specification makes that root part of the match too.

## Gotchas

- `%artist%%title%` is rejected because adjacent captures have no separator.
  Use a real delimiter from the names.
- A delimiter repeated inside an artist or title can make the split
  ambiguous for your intended meaning. Check those rows in the preview.
- This is a capture pattern, not a scripting expression or regex. For
  later cleanup, apply a separate field-formatting step.
- Read-only underscore placeholders are not destinations here. Use ordinary
  field names and `%dummy%`.
- Applying saves immediately. Keep copies and inspect operation errors and
  **Edit → Undo** for recorded changes.

<!-- verified: crates/tag-core/src/script.rs:CompiledFilenamePattern; src-tauri/src/lib.rs:filename_to_tags -->

## Related

- [Tags from folder and file names](recipes/tags-from-folder-and-filename.md)
- [Tag → Filename](tag-to-filename.md)
- [Text File → Tag](text-file-to-tag.md)
- [Converters](index.md)
