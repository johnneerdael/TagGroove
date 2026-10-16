# Export Cover to File

Use **Export Cover to File** to copy embedded pictures to separate image
files. Exporting leaves the pictures embedded in the audio. It does not
resize or convert the image data. This action works with local files; action
groups are not available for provider documents yet.

## Write a front-cover file beside the album

1. Select copied practice files and save pending edits.
2. Choose **Actions → Actions (Quick)…**, then **Export Cover to File**.
3. Set **Output filename format (extension is added)** to `folder`.
4. Set **Export covers of type** to **Front Cover**.
5. Leave **Export duplicate covers** off.
6. Choose **Apply action**, then inspect the album folder.

For a reusable workflow, create the action in **Actions → Actions…**
(++alt+6++; ⌥ Option on macOS). A new installation includes the unchecked
**Examples → Save front cover as folder image** group for this task.

A JPEG front cover becomes `folder.jpg`. A PNG becomes `folder.png`.
A relative output path is resolved beside each audio file. The extension
comes from the picture's actual format, not the audio file's extension.

## Settings and naming examples

| Setting | Meaning |
|---|---|
| **Output filename format (extension is added)** | Format string evaluated per audio file |
| **Export covers of type** | All cover types, or one role such as front or back |
| **Export duplicate covers** | Whether to attempt exports of repeated image content at the same requested destination |

| Output format | Embedded picture | Output name |
|---|---|---|
| `folder` | JPEG | `folder.jpg` |
| `cover.jpg` | PNG | `cover.png` |
| `%album%` with album `Harbour Lights` | JPEG | `Harbour Lights.jpg` |
| `Vol. 2` | JPEG | `Vol. 2.jpg` |

An existing matching image extension is kept; a different recognized
image extension is replaced. A suffix such as `. 2` in the last example
is part of the name, so the image extension is appended.

When several pictures in one audio file qualify, later pictures receive
numbered names such as `cover (1).jpg`. The suffix follows the position
among matching cover types, including earlier covers skipped as duplicates.
Appending another embedded cover and exporting again keeps the first image
and writes the new one to its numbered name. Prefer a type filter when
one front cover is wanted rather than every embedded picture.

Use `\` to create literal folder components on every platform. Ordinary
tag values have separators and reserved filename characters cleaned.
`%_folderpath%`, `%_path%` and `%_workingpath%` retain native path components,
including a backslash inside a macOS or Linux folder name. See
[placeholders](../reference/placeholders.md) and
[format strings](../reference/format-strings.md).

## Understand duplicates and existing files

With duplicate export off, OpenTag skips a picture when the same image
bytes remain in an output for the same requested name after this action
finishes. This is useful when every track in an album contains identical
artwork. Reordering or replacing embedded pictures exports a repeated image
again when a later numbered write would overwrite its only output.

Different pictures aimed at the same name are not duplicates. Nor does
enabling duplicate export create a unique name for every track. Use a
per-track expression such as `%_filename%-cover` when every track needs
its own output file.

An existing destination with identical image bytes stays unchanged. A
different image is replaced. **Export duplicate covers** controls repeated
images in this run; it does not prevent replacement of existing files. An
export that only finds identical images does not add an undo step.

Use **Edit → Undo** (++cmd+z++ / ++ctrl+z++) in the same session to restore a
replaced image or remove a new image created by the action. Redo recreates
those images. Empty output folders can remain. Keep backups of images to
retain beyond this session and inspect the available undo operation.

When two selected files export different images to one name in a run, the
later file gets a warning and the first image stays. A file can replace its
own earlier output after another action changes its cover, including
returning to an image exported earlier. A different file's equal-image
no-op does not take ownership of the output name.

On macOS and Linux, exporting to an image symlink replaces the link entry;
its target file stays unchanged. With the undo step available, Undo restores
the original link and Redo restores the exported image. If link undo cannot
be prepared, the action warns before replacing it. Dangling output links
are refused. A later folder move carries output ownership and undo capture
with the image; repeated exports retain the snapshot from before the first
replacement.

Output aliases that would replace an input audio file are refused. A
warning does not stop the file's remaining actions or later groups. Inspect
all warnings and earlier successful writes before repeating the group.

## Gotchas

- Running the same export again can encounter files created by the first
  run. Duplicate detection for a run is not a disk cleanup policy.
- Pictures of different roles may contain identical image bytes. A type
  filter makes the intended export clearer.
- An audio file without a qualifying picture produces no image for it.
- Undo covers this action's created and replaced images. Other external
  exports use their own write/undo rules; it is not a general cleanup tool.

<!-- verified: src-tauri/src/action_host.rs:cover_file_target, NativeHost::execute; src-tauri/src/history.rs:action_cover_creation_only_has_undo_and_redo, action_cover_undo_follows_later_directory_moves_and_keeps_the_first_snapshot -->

## Related

- [Import Cover from File](import-cover.md)
- [Adjust Cover](adjust-cover.md)
- [Set Cover Properties](set-cover-properties.md)
- [Actions and action groups](index.md)
