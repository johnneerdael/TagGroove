# Import Cover from File

Use **Import Cover from File** to embed an existing image in each selected
audio file. A filename format lets one action find the right picture in
each album folder. This action works with local files; action groups are
not available for provider documents yet.

<!-- figure:action-import-cover -->
<figure markdown>
![The Edit action dialog with the action type set to Import Cover from File and its settings](../assets/screens/action-import-cover-light.png#only-light)
![The Edit action dialog with the action type set to Import Cover from File and its settings](../assets/screens/action-import-cover-dark.png#only-dark)
<figcaption>The Import Cover from File action's settings in an action group.</figcaption>
</figure>
<!-- /figure -->

## Embed each album's cover

1. Put the intended image in each album folder as `cover.jpg`.
2. Keep copies of the audio files, save pending edits and select a few
   practice files.
3. Choose **Actions → Actions (Quick)…**, then **Import Cover from File**.
4. Enter `cover.jpg` in **Image filename format**.
5. Choose **Front Cover** in **Import cover as**.
6. Leave **Delete existing covers** off for this trial.
7. Choose **Apply action** and inspect the embedded pictures.

For a reusable sequence, open **Actions → Actions…** (++alt+6++; ⌥ Option
on macOS), create a group and add the same action there. Applying either
route writes the audio files immediately.

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

Use `*` for any run of characters or `?` for one character in the final
filename component. For example, `%_folderpath%front*.png` finds the first
importable matching image, while `%_folderpath%cover?.jpg` can find `cover1.jpg`.
Candidates are considered in natural name order, ignoring filename case:
`front2.png` comes before `front10.png`. Folders, non-images and formats the
importer refuses are skipped. JPEG, PNG, GIF, BMP, TIFF and WebP are accepted.
Folder components are used as literal existing paths; matching does not
expand a folder pattern or search several folders. On macOS and Linux,
`*` and `?` in an actual source-folder name remain literal, including when
that folder comes from `%_folderpath%`. Prefer a specific filename pattern;
a broad `*` can choose unintended artwork.

Use `\` to separate literal folders in the format on every platform.
`%_folderpath%`, `%_path%` and `%_workingpath%` keep their native path components,
including a backslash inside a macOS or Linux folder name. Ordinary tag
values have separators and reserved filename characters cleaned: an album
value `AC/DC` does not create an `AC` folder.

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
- A missing or invalid image reports an error; it does not trigger an
  online artwork search.
- The action writes immediately. Inspect the pictures and **Edit → Undo**
  for recorded file changes before applying it to the whole album.
- A missing or invalid match leaves the existing covers in place, even with
  **Delete existing covers** enabled. A warning does not stop that file's
  remaining actions or later groups; inspect earlier successful writes
  before running the group again.

<!-- verified: src-tauri/src/action_host.rs:NativeHost::execute, import_cover_wildcard_takes_webp_and_skips_what_the_import_refuses; crates/tag-core/src/cover_ops.rs:resolve_cover_source -->

## Related

- [Export Cover to File](export-cover.md)
- [Adjust Cover](adjust-cover.md)
- [Set Cover Properties](set-cover-properties.md)
- [Actions and action groups](index.md)
