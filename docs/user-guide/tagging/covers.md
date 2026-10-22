# Working with covers

Embedded covers travel inside an audio file's metadata. A folder image such
as `folder.jpg` is a separate file that a player or TagGroove can display.
Use the cover window to inspect embedded images, and **Manage covers…** to
review a selection's cover set before writing a batch change.

For MP3 files, check the [MPEG Read and Write choices](../reference/options/tags-mpeg.md)
when a saved cover appears unchanged. A retained APE tag can supply the displayed
image while a cover operation successfully updates ID3. Choose the read view
that matches the tag type being inspected.

Start with copied practice files. Cover operations have their own write
steps; they are not all ordinary pending text-tag edits.

## Identify what the window shows

The tag panel and Extended tags dialog share the cover window. It reports
the current picture role, available dimensions, image size/type, description
and position within a common cover set. Previous/next controls appear when
there are several shared images.

| Display | Meaning |
| --- | --- |
| A shared embedded image | All selected files have the same ordered cover identities; the current image can be inspected individually. |
| **Covers vary** | The selected files have different cover sets, including files with no artwork. The app does not present the first file's cover as common to all. |
| **Folder image** / **Not embedded in the file** | A local image is being shown as a fallback; it is not inside the selected audio files. |
| **No artwork** | No embedded cover is available for the selection and no applicable folder fallback is shown. |
| Loading/unavailable/error message | Artwork has not been obtained successfully; do not interpret this as a confirmed empty cover set. |

Image bytes, picture role, description and cover order all participate in a
shared cover set. Two visually identical files with different descriptions
can produce **Covers vary**. Select one file at a time to investigate.

## Add a front cover to two practice files

1. Save pending changes and select only the two copied audio files.
2. Choose **Edit → Manage covers…** (++cmd+alt+m++ on macOS;
   ++ctrl+alt+m++ on Windows/Linux).
3. Choose **Add…** and select an image file. The list gains a new cover,
   initially **Front Cover**, to be embedded in every selected file.
4. Inspect its dimensions, image type, picture type and description.
   Review the whole list so an unintended old cover is not left first.
5. Choose **OK** to write the planned cover set. Inspect any file errors.
6. Select each practice file separately and verify its saved artwork.

The shortcut respects text-entry/AltGr combinations on Windows; use the
menu if the key combination types a character in the current input.
The picker lists supported image files such as JPEG, PNG, GIF, BMP and
WebP. Its starting folder follows the configured cover-art directory and
fallback order explained in the [cover context reference](../reference/menus/context-menus.md).

Adding an image does not convert the audio format. Some tag formats cannot
store embedded artwork or preserve every picture-role distinction; see
[file format capabilities](../reference/file-formats.md).

## Understand the Manage covers list

The dialog combines the selection's distinct covers into one editable list.
A full circle **●** means a cover is present in every selected file. A
partly filled circle **◐** identifies a cover present in only some files,
with its file count shown. These indicators describe membership, not image
quality or whether the cover is the front image.

<!-- figure:manage-covers -->
<figure markdown>
![The Manage covers dialog for twelve selected files: one front cover, 1200 × 1200 JPEG, with buttons to add, paste, replace, adjust, remove, reorder and extract](../assets/screens/manage-covers-light.png#only-light)
![The Manage covers dialog for twelve selected files: one front cover, 1200 × 1200 JPEG, with buttons to add, paste, replace, adjust, remove, reorder and extract](../assets/screens/manage-covers-dark.png#only-dark)
<figcaption>Manage covers lists every picture shared by the selected files.</figcaption>
</figure>
<!-- /figure -->

For example, suppose a front cover is embedded in one of two selected files:

<!-- cover-presence-examples:start -->
| State | Presence label |
| --- | --- |
| `original` | `In 1 of 2 files` |
| `apply-to-all` | `In all 2 files` |
<!-- cover-presence-examples:end -->

Select that partial cover and choose **Apply to all** from its right-click
menu, or enable **Embed in all selected files**. The plan now adds it to the
other selected file too. This is an intentional redistribution of artwork;
merely selecting the partial cover does not add it anywhere.

An existing partial cover normally stays only in the files that had it.
Replacing or adjusting that cover preserves its membership unless applying
it to all is also chosen. A newly added cover targets all files selected
when the dialog opened; choose the initial audio selection accordingly.

## Select and reorder covers

Click a cover to select it. Command/Ctrl-click toggles individual covers,
and Shift-click selects a range. With the cover list focused, use arrow
keys, Home/End, Shift for a range, Space to toggle the focused cover, and
Command/Ctrl+A to select every cover in that dialog.

Use **Move up** / **Move down**, Alt+Up / Alt+Down, or drag selected covers
to reorder them. A selected block retains its relative order. Moving an
item at a boundary cannot move it beyond the list. For three cover labels
`Front, Leaflet, Back`, selecting Leaflet and moving it up gives
`Leaflet, Front, Back`.

The resulting first cover is often what a player initially displays, but
the receiving player can use its own role/order rules. Reordering does not
change a cover's picture type. For an existing partial cover, order changes
affect the files in which it will actually be embedded, not files that lack
it and are not part of an Apply-to-all plan.

## Change images, types and descriptions

| Control | What is planned |
| --- | --- |
| **Replace…** | Replace one selected cover's image, retaining its role, description and position. |
| **Adjust…** | Resize/convert the selected covers through the cover-adjustment settings, staging the results for the dialog's later **OK**. |
| **Remove** | Remove the selected cover entries from the planned set. Delete/Backspace in the focused cover list performs this list action. |
| **Cover type** | Assign a picture role to all selected covers. A mixed **keep** indication preserves current types until a real choice is made. |
| **Description** | Set one description on selected covers; empty text clears it. |
| **Extract…** | Save one unchanged original embedded image to an external file. New/replaced staged images are not available through this original-image extraction path. |

For precise scaling policies and image encoding, see
[Adjust Cover](../actions/adjust-cover.md). A status saying the selected
covers already match those settings means no image replacement was needed.
The available role names and their format limits are listed in the
[cover menu reference](../reference/menus/context-menus.md).

A new cover starts as Front Cover. If a cover already uses that type with
an empty description, a new cover can receive the selected image filename
as its description to keep the entries distinguishable. Inspect those
descriptions before saving when a consuming player relies on them.

## Confirm or leave the dialog

Changing the Manage covers list creates a plan. **OK** writes changed plans
to the selected audio files. Closing without OK drops those unconfirmed
cover-list edits. A failed write shows the actual file errors; check which
files were changed rather than assuming a multi-file operation is atomic.

Extraction is different: an image already written to an external destination
remains there when the dialog is closed. Tag-edit undo is not a general
cleanup operation for external files. For an applied recorded cover change,
inspect **Edit → Undo** and its operation label.

## Use direct cover commands deliberately

The cover window's **⋯** or right-click menu has Add, Remove, Replace,
Extract, Adjust, Manage, type/description and clipboard commands. Direct
cover mutations write through their own operation immediately. They do not
wait for the Extended tags dialog's **Keep changes** button.

For an equal cover set, **Remove cover** acts on the current displayed
occurrence. For **Covers vary**, it asks before removing all covers from
the selected files. Replace, Extract, Copy and Cut require a common
displayed image; use Manage covers for a mixed set.

Copy/Cut use the image clipboard, and Paste embeds a supplied image. This is
separate from copying tag fields or all tags. Save tags first if the displayed
artwork is pasted but not yet written: Copy, Cut and Extract require its
saved form. Platform image-clipboard support can make commands unavailable.

## Drag, drop and inspect larger

- Drop supported image files onto the cover area to add embedded artwork.
  When Extended tags is open, its cover area is the target. A file dropped
  elsewhere can load files instead, so aim at the image area.
- Where native desktop image drag-out is available, drag the displayed
  embedded image into a file manager to copy it externally. Use **Extract
  cover…** when drag-out is unavailable or an explicit destination is needed.
- Double-click the displayed embedded image, press Enter while its window
  is focused, or choose **View larger…**. Previous/next changes the displayed
  cover, not the stored cover order. Close the viewer to return.
- **Correct aspect ratio** changes how the image fits the preview/viewer.
  Stretching a display is not resizing or re-encoding the embedded image.

## Folder-image fallback

When none of the selected files has embedded or pending artwork, TagGroove
can show a local image if every selected local file is in the active file's
folder. **Options → Tags → Don’t display first image from file directory as cover art**
disables this fallback. Provider documents without a browsable local folder
use a different access path and do not gain folder browsing through this
setting.

The fallback is labeled **Not embedded in the file**. It does not mean a
failed embed was repaired, and removing embedded art does not delete the
external image. Add/embed the image explicitly when
it should travel with the audio files.

## Gotchas

- Choose the audio selection before opening a batch cover dialog. Its
  presence counts and write targets refer to that selection.
- A partial cover and an all-files cover are different plans; check the
  circle indicator and **Embed in all selected files** before applying.
- Changing a file suffix from `.jpg` to `.png` does not convert image bytes.
  Use a real cover-adjustment operation when a different encoding is needed.
- Confirm saved results and format limits in the receiving player. A common
  cover preview does not prove every tag format will retain its description
  or role after saving.

## Related

- [Cover context and helper reference](../reference/menus/context-menus.md)
- [Adjust Cover action](../actions/adjust-cover.md)
- [Set cover properties](../actions/set-cover-properties.md)
- [File format capabilities](../reference/file-formats.md)
- [Extended tags](extended-tags.md)

<!-- Source baseline main4e4d5fc: covers.ts markup/perform/manage/rowMenu/dragDrop/dragOut; cover-model.ts selectionCovers/managedFromSet/newManaged/presence/moveManaged/dropManaged/planItems; main.ts CoverUI host/folder fallback; list-behaviour.ts folderImageFolder; src-tauri/src/covers.rs start_directory/cover_set/apply_cover_plan and tag-core cover management. Canonical manage-covers/cover-window topics independently taught; no installed native drag, clipboard or save-back validation claimed. -->
