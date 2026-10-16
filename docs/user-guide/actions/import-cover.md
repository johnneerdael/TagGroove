# Import Cover from File

Use **Import Cover from File** to embed an existing image in each selected
audio file. A filename format lets one action find the right picture in
each album folder.

## Embed each album's cover

1. Put the intended image in each album folder as `cover.jpg`.
2. Keep copies of the audio files, save pending edits and select a few
   practice files.
3. Choose **Actions → Actions (Quick)…**, then **Import Cover from File**.
4. Enter `cover.jpg` in **Image filename format**.
5. Choose **Front Cover** in **Import cover as**.
6. Leave **Delete existing covers** off for this trial.
7. Choose **Apply action** and inspect the embedded pictures.

A relative image path is resolved beside each audio file. Thus files in
different folders can use different `cover.jpg` images with the same
action. **Browse…** can choose one specific image; an absolute path uses
that image for every selected file.

## Choose the filename and cover type

| Setting | Meaning |
|---|---|
| **Image filename format** | A format string evaluated for each file, including the image's extension |
| **Import cover as** | The picture role to store, initially **Front Cover** |
| **Delete existing covers** | Remove all previously embedded pictures before adding the new one; off initially |

| Image filename format | Where it reads |
|---|---|
| `cover.jpg` | `cover.jpg` next to each audio file |
| `%_folderpath%folder.jpg` | `folder.jpg` in that audio file's folder |
| `%album%.png` | For `ALBUM` = `Harbour Lights`, `Harbour Lights.png` beside the audio file |

The expression must produce one existing image path. `*.jpg` is not a
wildcard search: the action does not choose among several matching files.
Use an exact naming convention, or choose the file with **Browse…**.

Cover roles include front, back, leaflet, media, artist, band and other
standard picture types. Choose the role that describes the image; changing
the role does not change the image itself.

## Decide whether to keep existing pictures

With **Delete existing covers** off, importing adds a picture and retains
the existing pictures. Repeated runs can add repeated artwork.

With it on, every existing picture is removed, including back covers and
booklet images, before the import. The source image is checked before
removal, but a later write failure is still a reason to inspect the file
and retain a backup. To replace only one particular picture, use the
cover-management controls instead of clearing all covers.

## Gotchas

- The audio format must support embedded artwork. A sidecar image existing
  on disk does not mean every selected tag format can store it.
- Fields used in paths must contain valid path text. Image paths are not
  filename-mode templates and do not automatically sanitize tag values.
- A missing or invalid image reports an error; it does not trigger an
  online artwork search.
- The action writes immediately. Inspect the pictures and **Edit → Undo**
  for recorded file changes before applying it to the whole album.

<!-- verified: src-tauri/src/action_host.rs:relative_to_file, NativeHost::execute; crates/tag-core/src/tags.rs:import_cover_with_options -->

## Related

- [Export Cover to File](export-cover.md)
- [Adjust Cover](adjust-cover.md)
- [Set Cover Properties](set-cover-properties.md)
- [Actions and action groups](index.md)
