# Export Cover to File

Use **Export Cover to File** to copy embedded pictures to separate image
files. Exporting leaves the pictures embedded in the audio. It does not
resize or convert the image data.

## Write a front-cover file beside the album

1. Select copied practice files and save pending edits.
2. Choose **Actions → Actions (Quick)…**, then **Export Cover to File**.
3. Set **Output filename format (extension is added)** to `folder`.
4. Set **Export covers of type** to **Front Cover**.
5. Leave **Export duplicate covers** off.
6. Choose **Apply action**, then inspect the album folder.

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
numbered names such as `cover (1).jpg`. Prefer a type filter when the
desired output is one front cover rather than every embedded picture.

## Understand duplicates and existing files

With duplicate export off, OpenTag skips a picture when the same image
bytes have already been exported to the same requested destination in
this run. This is useful when every track in an album contains identical
artwork.

Different pictures aimed at the same name are not duplicates. Nor does
enabling duplicate export create a unique name for every track. Use a
per-track expression such as `%_filename%-cover` when every track needs
its own output file.

This action does not overwrite an existing output file. A collision
reports an error. Choose a new destination or inspect and move the
existing file before repeating the export.

## Gotchas

- Running the same export again can encounter files created by the first
  run. Duplicate detection for a run is not a disk cleanup policy.
- Pictures of different roles may contain identical image bytes. A type
  filter makes the intended export clearer.
- An audio file without a qualifying picture produces no image for it.
- Generated image files may require separate cleanup; do not assume
  **Edit → Undo** removes every exported file.

<!-- verified: src-tauri/src/action_host.rs:cover_file_target, NativeHost::execute -->

## Related

- [Import Cover from File](import-cover.md)
- [Adjust Cover](adjust-cover.md)
- [Set Cover Properties](set-cover-properties.md)
- [Actions and action groups](index.md)
