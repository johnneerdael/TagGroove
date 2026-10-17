# Keep each file's front covers and remove the others

Remove back/leaflet pictures while retaining the front artwork that already
belongs to each file. Use **Manage covers…** to review the whole selection's
cover list. A quick Remove-cover command on a varying selection can remove
all artwork, which is not this task.

## Use role-aware practice files

Start with copied local MP3 files whose writable ID3v2 artwork roles have
been checked. In this example, labels identify the picture contents for the
exercise; they are not filenames the app must display:

<!-- front-catalog:start -->
| Picture label | Cover type | Description |
|---|---|---|
| `front-A` | Front Cover | `Harbour front` |
| `front-B` | Front Cover | `Low Tide front` |
| `back` | Back Cover | `Rear` |
| `booklet` | Leaflet | `Booklet` |
<!-- front-catalog:end -->

Use the role labels shown in the dialog, and inspect the actual pictures
as well: a stored role can be wrong. The three practice files contain these lists:

<!-- front-membership:start -->
| File | Pictures before | Pictures after this cleanup |
|---|---|---|
| `Paper Boats.mp3` | `["front-A","back"]` | `["front-A"]` |
| `Evening Ferry.mp3` | `["front-B","booklet"]` | `["front-B"]` |
| `Archive.mp3` | `["back"]` | `[]` |
<!-- front-membership:end -->

The archive file has no Front Cover to keep, so removing its back picture
leaves it without embedded art. Exclude such a file if that is unwanted.
This recipe does not recognize a front image visually or invent a missing one.

## Remove only the unwanted cover rows

1. Save or discard pending edits and select the three copied audio files.
2. Choose **Edit → Manage covers…** (++cmd+alt+m++ on macOS;
   ++ctrl+alt+m++ on Windows/Linux).
3. Inspect every listed cover's role, description, picture and membership.
   Cancel and resolve read errors before interpreting unavailable art as empty.
4. Select the **Back Cover** and **Leaflet** rows for this exercise. Use
   Command/Ctrl-click to select separate rows, or Shift-click for a range;
   check the actual highlighted rows before removing them.
5. Choose **Remove** inside Manage covers. Leave the two Front Cover rows.
6. Inspect the remaining rows and choose **OK** only when the plan is right.
7. Select each practice file separately and verify its saved artwork.

Remove edits the dialog's planned set. **OK** writes it to the audio files;
Cancel before OK abandons this unconfirmed removal plan. It is not an ordinary
text edit waiting for Save tags after OK. Inspect any per-file write errors.

## Preserve partial membership

A full circle **●** means the same cover is present in every selected file.
A partial circle **◐** means only some contain it. This describes membership,
not quality or whether it is a front cover.

<!-- front-presence:start -->
| Remaining cover | Membership without Apply to all |
|---|---|
| `front-A` | `In 1 of 3 files` |
| `front-B` | `In 1 of 3 files` |
<!-- front-presence:end -->

Leave **Embed in all selected files** unchanged/off for these partial rows.
Merely keeping a partial cover retains it only where it was already present.
Do not choose **Apply to all** just to turn the circle solid: applying front-A
to all would add Harbour's front to Evening Ferry as well as to Archive.
Evening Ferry would then have both front-A and its own front-B in this plan.
That is a different, intentional redistribution task.

If several Front Cover rows belong to one file, this recipe keeps those
front pictures, not necessarily exactly one picture. To keep only one,
inspect that file alone and select the other front occurrence(s) for removal
too. A picture role does not establish which scan or edition is correct.

## Respect formats without Front/Back roles

MP4 artwork does not retain Front/Back type and description distinctions;
TagGroove normalizes it to **Other**. Do not remove every Other picture while
expecting this role-based recipe to discover a front automatically. For such
a file, inspect images individually and keep the chosen image by its content.
Other tag formats can also restrict roles, descriptions or repeated pictures.
See [format capabilities](../reference/file-formats.md).

A folder image can appear after embedded artwork is removed. Its **Not
embedded in the file** label is not proof that the old cover survived or was
restored. Use the embedded cover list and individual files to verify results.

## Recover a mistaken cleanup

Before OK, Cancel abandons the plan. After a recorded write, inspect
**Edit → Undo …** and its operation label in the current desktop session.
Keep the original audio and images for recovery after quitting. A failed
multi-file write can leave some files changed; check the operation errors
before retrying or applying another cleanup.

## Related

- [Working with covers](../guides/covers.md)
- [Embed a folder image](embed-folder-jpg.md)
- [Shrink oversized covers](resize-covers.md)
- [Cover and context commands](../reference/menus/context-menus.md)

<!-- verified: src/covers.ts:manage remove/OK; src/cover-model.ts:managedFromSet,presence,planItems; crates/tag-core/src/cover_manage.rs:planned_pictures,normalize_for,apply_cover_changes_with_options. Written catalog/membership/presence execute against model and core MP3 fixtures; no native UI/all-format claim. -->
