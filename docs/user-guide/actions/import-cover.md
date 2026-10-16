# Import cover from file

Use **Import Cover from File** to embed an existing image in each selected
audio file. A filename format can find a different image for each album.
This action works with local files; action groups are not available for
provider documents yet.

## Add the action

1. Select a few practice files and open **Actions → Actions…** (++alt+6++;
   Alt is ⌥ Option on a Mac).
2. Create a group, choose **Edit actions…**, and add **Import Cover from File**.
3. Enter **Image filename format**, for example:

    ```text
    %_folderpath%folder.jpg
    ```

4. Set **Import cover as** to **Front Cover**, or choose another cover type.
5. Leave **Delete existing covers** off to append the image. Enable it to
   replace all existing covers, including other types.
6. Save the group, check only that group, and run it. Inspect the cover in
   the file list's selected file.

Running the group writes the files immediately. Use **Edit → Undo**
(++cmd+z++ / ++ctrl+z++) in the same session to reverse the recorded changes.
Keep original files as a backup.

## Filename formats and wildcards

[Placeholders](../reference/placeholders.md) and
[format strings](../reference/format-strings.md) are evaluated for each file.
A relative name is resolved beside that audio file. `%_folderpath%` keeps the
file's folder; `%_path%` and `%_workingpath%` also keep their path components,
including a backslash inside a macOS or Linux folder name. Backslashes written
in the format itself separate folders.
Use `\` to separate folders inside a format string on every platform.
Separators and reserved characters coming from ordinary tag values are
cleaned rather than creating folders: an album value of `AC/DC` does not
create an `AC` folder.

Use `*` for any run of characters or `?` for one character in the final
filename component:

| Format | Chooses |
|---|---|
| `%_folderpath%folder.jpg` | That exact file |
| `%_folderpath%front*.png` | The first importable image with a matching filename |
| `%_folderpath%cover?.jpg` | A matching name such as `cover1.jpg` |

Matching ignores filename case. Candidates are considered in natural name
order, so `front2.png` comes before `front10.png`. Folders, non-images and
formats the cover importer does not accept are skipped. JPEG, PNG, GIF,
BMP, TIFF and WebP are accepted. Wildcards in folder names are refused.

<!-- verified: crates/tag-core/src/cover_ops.rs:resolve_cover_source; src-tauri/src/action_host.rs:import_cover_wildcard_takes_webp_and_skips_what_the_import_refuses -->

## Gotchas

A broad pattern such as `*` may select an unintended image. Prefer a name
that identifies the front cover. No matching importable image gives a
warning for that file and leaves its existing covers in place, even with
**Delete existing covers** enabled.

A warning does not stop that file's remaining actions or later groups.
Check every warning before running the group again: earlier successful
changes have already been written.

## Related

- [Export cover to file](export-cover.md)
- [Placeholders](../reference/placeholders.md)
- [Format strings](../reference/format-strings.md)
