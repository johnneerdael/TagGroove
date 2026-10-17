# Cover art

Why can an album show artwork in one folder, then lose it when a track is
copied elsewhere? An image beside the audio file and an image embedded in
its tags are different objects. OpenTag labels its folder-image fallback so
you can tell which one you are seeing.

## Where the picture lives

| Picture location | What travels when only the audio file is copied |
|---|---|
| `folder.jpg` beside the tracks | The image stays behind unless it is copied separately. |
| Embedded picture in each track's tags | Each copied track carries its own saved picture, where its format supports artwork. |
| Pending or staged picture in an editor | A visible preview alone does not establish that the audio file has been written. |

A shared folder image avoids storing a copy in every track. Embedding makes
each file self-contained. Those are storage choices. They do not guarantee
what a particular player displays. Some players prefer a folder image;
others choose an embedded role, the first image, or cached artwork.

OpenTag can show **Folder image** / **Not embedded in the file** when the
eligible local selection has no embedded or pending artwork. This fallback
is a display convenience. Adding an embedded image does not delete the
external source. Removing embedded images does not remove `folder.jpg`.
The exact selection and option rules are in [Working with covers](../guides/covers.md#folder-image-fallback).

## An embedded cover has more than pixels

A cover consists of image bytes, a picture type, a description and a position
in the file's ordered cover list. **Front Cover**, **Back Cover** and
**Leaflet** are roles. Assigning a role does not crop, rotate or rename the
image. The description is separate text; it is not a filesystem path.

These selection examples use the same front image A and a different back
image B. A label includes image, role and description; list order matters.

<!-- cover-concept-definitions:start -->
| Label | Image identity | Picture type | Description |
|---|---|---|---|
| `front` | `image-A` | Front Cover | `Album front` |
| `front-other-description` | `image-A` | Front Cover | `Alternate note` |
| `same-image-back-role` | `image-A` | Back Cover | `Album front` |
| `back` | `image-B` | Back Cover | `Track list` |
<!-- cover-concept-definitions:end -->

<!-- cover-concept-selections:start -->
| First file's ordered covers | Second file's ordered covers | Shared cover-window result |
|---|---|---|
| `front` | `front` | Same |
| `front` | `front-other-description` | Covers vary |
| `front` | `same-image-back-role` | Covers vary |
| `front,back` | `back,front` | Covers vary |
| `front` | None | Covers vary |
| None | None | No embedded artwork |
<!-- cover-concept-selections:end -->

**Covers vary** therefore does not necessarily mean that every image looks
different. Inspect individual files or open **Manage covers…**. Its full
circle marks an image present in every selected file; the partial circle
marks one present in a subset. Keeping a partial entry normally preserves
its membership. **Apply to all** deliberately adds it to other selected files.
A newly added image is planned for the whole initial selection.

Not every format preserves these properties equally. MP4 artwork does not
retain the same role distinctions as ID3v2; OpenTag reads its pictures as
**Other**. ID3v1 cannot hold pictures. Read the [file format capabilities](../reference/file-formats.md)
before relying on a role or description surviving a conversion.

## Dimensions, bytes and encoding are different

A 1200 × 1200 image describes pixel dimensions, not its file size. Two images
with those dimensions can have very different byte sizes and compression
artifacts. Embedding a large image into eight tracks stores a separate copy
in each; keeping an external high-resolution original can make later
revisions easier.

JPEG compression trades image detail for size and does not retain a PNG's
alpha transparency. PNG can preserve transparency but may be larger for
photographic artwork. The best choice depends on the source and receiving
software; no single pixel limit is guaranteed for every player.

Changing `.jpg` to `.png` in a filename does not convert the bytes. Use
**Adjust…** or an [Adjust Cover action](../actions/adjust-cover.md) for actual
resize/conversion. A preview's **Correct aspect ratio** changes how it is
shown, not the embedded pixels. Upscaling cannot restore detail missing from
a small original. Settings that already match can leave image bytes unchanged;
JPEG quality alone does not force OpenTag's adjustment path to recompress an
otherwise unchanged image.

## What this means in OpenTag

Start with a copy and decide whether the task is display, embedding,
extraction, reordering or image conversion. **Manage covers…** stages a list
plan and writes it on **OK**; direct cover-window mutation commands write
through their own operation. External extraction creates a separate file,
which tag undo does not generally remove. Inspect operation errors and each
saved track rather than assuming an entire selection changed together.

For a recorded cover edit, inspect **Edit → Undo** and its label. Keep the
original images and audio files: a session history is not a permanent image
archive. The detailed guide explains clipboard/platform limits and the
separate handling of pending pasted artwork.

## Related

- [Working with covers](../guides/covers.md)
- [Embed a folder image](../cookbook/embed-folder-jpg.md)
- [Resize covers](../cookbook/resize-covers.md)
- [Keep only front covers](../cookbook/keep-only-front-cover.md)
- [File format capabilities](../reference/file-formats.md)

<!-- verified: guide-media-concepts.test.mjs executes the actual selectionCovers model with all written role/description/order cases and LF/CRLF. Existing cover_recipe_guide.rs tests real MP3 imports and image adjustments; no native/player fallback or every-format proof. -->
