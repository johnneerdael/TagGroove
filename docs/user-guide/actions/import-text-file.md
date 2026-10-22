# Import Text File

Use **Import Text File** to replace one field with the complete contents
of a text file. It suits lyrics, a long comment or album notes. It does
not parse columns or assign different lines to different audio files.

<!-- figure:action-import-text-file -->
<figure markdown>
![The Edit action dialog with the action type set to Import Text File and its settings](../assets/screens/action-import-text-file-light.png#only-light)
![The Edit action dialog with the action type set to Import Text File and its settings](../assets/screens/action-import-text-file-dark.png#only-dark)
<figcaption>The Import Text File action's settings in an action group.</figcaption>
</figure>
<!-- /figure -->

## Load a text file into a field

1. Put a UTF-8 file named `notes.txt` beside your practice audio file.
   For this example, give it the text `Recorded at the north pier.`
2. Save pending edits and select that copied audio file.
3. Choose **Actions → Actions (Quick)…**, then **Import Text File**.
4. Set **Field** to `COMMENT` and **Text filename format** to `notes.txt`.
5. Choose **Apply action**, then inspect `COMMENT` in **Extended tags…**.

The comment becomes `Recorded at the north pier.`. Existing comment
values are replaced with one value containing the imported text.

## Filename formats

| Setting | Meaning |
|---|---|
| **Field** | The destination tag field, such as `LYRICS` or `COMMENT` |
| **Text filename format** | A format string evaluated separately for each selected audio file |
| **Browse…** | Choose a specific text file |

A relative path is resolved in each audio file's folder. An absolute path
reads the same file for every selected track.

Literal `\` separates folders on every platform. Ordinary tag values have
separators and reserved filename characters cleaned, using the configured
folder-separator replacement. `%_folderpath%`, `%_path%` and `%_workingpath%`
retain native path components, including a backslash within a macOS or Linux
folder name.

| Filename format | Example use |
|---|---|
| `notes.txt` | The album folder's notes for every selected track there |
| `%_filename%.txt` | For `02 - Paper Boats.flac`, read `02 - Paper Boats.txt` beside it |
| `%_folderpath%lyrics.txt` | Read one lyrics file from that audio file's folder |

Use an exact path, including the extension. The filename expression does
not expand wildcard searches. A missing file warns for that file; later
actions and groups still run. It does not schedule an import for later.

## Text and encoding rules

The entire file becomes one value, including its line breaks and spaces.
Use UTF-8 for new text files. A byte-order mark selects the corresponding
Unicode decoding. Without one, valid UTF-8 is accepted; otherwise the
configured ANSI/system codepage is used, with Windows-1252 as the fallback
when none is configured.

Check accented characters after importing older files. **Convert
Codepage** repairs some already-misdecoded field text, but choosing a
known encoding for the source text before import is preferable.

## Gotchas

- The action replaces all existing values of the destination field. It
  does not append the text.
- An empty source file replaces the existing content with empty text.
  Use **Remove Fields** when the goal is to delete the destination field.
- `_FILENAME`, `_DIRECTORY` and other underscore targets are not supported
  destinations for this action.
- This action writes immediately. Keep copies and inspect operation errors
  and **Edit → Undo** for recorded changes.

<!-- verified: src-tauri/src/action_host.rs:ActionEffect::ImportTextFile; crates/tag-core/src/export.rs:decode_export_template -->

## Related

- [Convert Codepage](convert-codepage.md)
- [Remove Fields](remove-fields.md)
- [Format Value](format-value.md)
- [Actions and action groups](index.md)
