# Give every album a cover

Embed each album folder's own image without giving the whole selection
one album's picture. In about 15 minutes you will distinguish folder
previews from embedded artwork, filter a copied selection, import its
images and verify the saved result. You will also learn when resizing
and online artwork lookup belong in separate steps.

- [ ] Two copied album folders, each with a different, known `folder.jpg`.
- [ ] A few copied local audio files whose writable tag formats support
      embedded artwork, such as MP3 with ID3v2 enabled.
- [ ] The original audio and full-resolution images kept elsewhere.
- [ ] Pending edits saved or discarded before using cover actions.

The images can be your own practice drawings. Check that each opens as a
real JPEG; renaming an unrelated file to `.jpg` does not convert it.
This lesson uses those local images and needs no live provider search.

## 1. Prepare a folder for each album

Use this arrangement, with a distinct picture in each album folder:

<!-- learn-covers-layout:start -->
```text
Practice/Harbour Lights/folder.jpg
Practice/Harbour Lights/01.mp3
Practice/Harbour Lights/02.mp3
Practice/Low Tide/folder.jpg
Practice/Low Tide/01.mp3
```
<!-- learn-covers-layout:end -->

Open `Practice` with **File → Change directory…** and include its subfolders
when loading. Wait for the file reads to complete. Resolve read errors
instead of treating an unreadable file as proof that it has no cover.
The [opening-files guide](../getting-started/opening-files.md) explains
the subfolder option and how to load another folder deliberately.

## 2. Identify what is actually embedded

Select one track and inspect the tag panel's cover window. A label
**Folder image** / **Not embedded in the file** means TagGroove is showing
an external image. The audio does not gain a picture merely because that
preview is visible. Selecting files from different folders can also make
the display differ from selecting one file.

A shared embedded image, **Covers vary**, **No artwork** and an artwork
loading/error message mean different things. When uncertain, select one
file at a time or inspect **Edit → Manage covers…**. Do not treat a failed
image display as a confirmed embedded-cover count of zero.

## 3. Find only tracks without embedded pictures

Show the Filter with **View → Show filter** and enter:

<!-- learn-covers-filter:start -->
```text
"%_covers%" EQUAL 0
```
<!-- learn-covers-filter:end -->

Wait for automatic application, or press Enter in manual mode. The query
counts embedded entries, including pictures that are not Front Cover.
It does not count `folder.jpg` beside a track.

<!-- learn-covers-counts:start -->
| Track snapshot | Embedded pictures | Folder image exists | Matches filter |
|---|---|---|---|
| A | `0` | Yes | Yes |
| B | `1` | Yes | No |
| C | `0` | No | Yes |
<!-- learn-covers-counts:end -->

The filter finds missing embedded art; it does not prove that a suitable
source image exists. Check the copied folders before continuing. A track
with only a back cover already has one embedded entry and will not match.

Choose **Edit → Unselect all**, then select only the intended visible
practice tracks. A filter hides rows; it does not guarantee that an old
hidden selection has been cleared. Read the selected-file count.

## 4. Import the image from each file's own folder

Choose **Actions → Actions (Quick)…**, then **Import Cover from File**.
Set the controls as follows:

<!-- learn-covers-import:start -->
| Setting | Value |
|---|---|
| Image filename format | `%_folderpath%folder.jpg` |
| Import cover as | Front Cover |
| Delete existing covers | Off |
<!-- learn-covers-import:end -->

`%_folderpath%` is evaluated separately for every audio file and already
ends with its native folder separator. The Harbour Lights tracks read
Harbour Lights' image; Low Tide reads Low Tide's image. An absolute path
chosen with **Browse…** would instead point every selected track to that
one image, which is not this lesson's goal.

Choose **Apply action**, then read the result and all file errors.
Importing writes immediately; a later **Save tags** is not required for
the completed cover action. A missing or invalid image reports a warning;
it does not download a replacement. Other selected files or later actions
can still be processed, so inspect partial results before retrying.

## 5. Verify the written files, not only the old preview

Clear the filter to see the whole copied set. Select each practice file
separately and inspect its saved embedded pictures. Check the correct
album image, its type/description and the number of entries. If successful,
the newly covered files no longer match the zero-cover filter.

With deletion off, import **adds** artwork while retaining the existing
pictures. Rerunning on an already covered track can add another copy.
Starting with the missing-cover selection avoids intentionally adding the
same image to every already illustrated track. It is not a global
duplicate-picture detector.

Leave **Delete existing covers** off in this lesson. Turning it on removes
all previous embedded pictures, including back covers and booklets. To
replace one chosen picture while retaining others, use Manage covers and
its explicit plan, then **OK**. A batch of file writes is not one atomic
transaction; inspect any failures and keep the original copies.

## 6. Shrink a test image only when needed

Keep the original full-resolution image. Select one successfully covered
practice file, clear any filter, and inspect every embedded picture it has.
Choose **Actions → Actions (Quick)… → Adjust Cover** and use:

<!-- learn-covers-adjust:start -->
| Setting | Value |
|---|---|
| Image format | Original |
| Quality | High (`90` for JPEG) |
| Resize cover | On |
| Size in pixels | `600` |
| Size policy | Maximum size |
| Scaling | Scale proportionally |
<!-- learn-covers-adjust:end -->

Choose **Apply action** only if changing **all embedded pictures** in that
file is intended. This action does not restrict itself to Front Cover.
For a single selected image instead, use **Manage covers… → Adjust…** and
confirm the planned list with **OK**.

<!-- learn-covers-dimensions:start -->
| Original dimensions | Expected dimensions with these settings |
|---|---|
| `1200 × 800` | `600 × 400` |
| `400 × 300` | `400 × 300` |
<!-- learn-covers-dimensions:end -->

Maximum size keeps the longer side at or below 600 while retaining the
proportions. Already small images are not enlarged. The separate folder
image is untouched. Original format does not force JPEG or a square image;
follow a player's actual requirements rather than assuming one size or
encoding suits every device. Quality alone also does not force recompression
of an already unchanged JPEG.

Inspect the saved dimensions and errors, then test the player if that is
the intended destination. Successful storage does not prove its display
supports this picture format, role or size.

## 7. Obtain missing source images as a separate decision

For a real album lacking `folder.jpg`, decide which release image is correct
before importing it. Existing desktop commands **Tag Sources → Covers:
MusicBrainz…** and **Covers: Discogs…** provide a covers-only lookup. Search
one album, review its edition and displayed artwork, and inspect the
**Write cover art**, **Replace existing artwork** and **Save image to file**
choices. **Save cover to … files** writes the selected artwork immediately
without changing the tag fields in this covers-only route.

**Save cover as…** saves the displayed online image to an external destination
and is distinct from embedding it. An external image saved this way remains
when the lookup dialog is closed. Network availability, provider credentials,
release coverage and image choices can limit a lookup; a successful local
import does not verify those remote services. See the
[Tag Sources menu](../reference/menus/tag-sources.md) and
[Tag Sources options](../reference/options/tag-sources.md) before a live lookup.

## 8. Recover before moving on

Use **Edit → Undo** and read the operation label for recorded cover writes
in the current desktop session. The import and adjustment are separate
operations. Undo for tags is not a general deletion or restoration tool for
external images. Keep the original audio and source pictures until you
have checked the final result and any target player.

## What you learned

- A folder preview and an embedded image are different storage choices.
- The cover-count filter asks about embedded entries, not sidecar existence.
- Per-file folder placeholders choose each album's image; a fixed absolute path does not.
- Import, adjustment, Manage covers plans and external-image saving have different write boundaries.
- Repeating an import can add pictures; resize scope and online release selection deserve separate review.

## Next steps

- [Embed each folder's image](recipes/embed-folder-jpg.md): exact names, wildcard order and repeated-import counts.
- [Working with covers](covers.md): partial membership, order, roles and planned batch edits.
- [Shrink oversized covers](recipes/resize-covers.md): maximum/minimum, proportional and square policies.
- [Find files without embedded covers](recipes/find-missing-covers.md): precise counts and mixed selections.
