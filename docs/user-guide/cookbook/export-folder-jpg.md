# Save embedded covers as folder.jpg

<div class="recipe-meta" markdown>
**Uses:** Export Cover to File action · **Works on:** copied local audio files with embedded artwork ·
**Time:** about 5 minutes per practice album · **Undo:** this action can record external-image changes in the current desktop session; inspect the available operation and keep existing images
</div>

## The problem

You have a front cover inside the audio files and want a separate album-folder
image. Export copies the embedded bytes out; it leaves the embedded pictures
and audio in place. A JPEG exports as `.jpg`; a PNG exports as `.png`.
Naming the output `folder.jpg` does not convert a PNG to JPEG.

For this recipe's exact `folder.jpg` result, start with a JPEG Front Cover.
If the picture has another encoding, keep its actual extension or deliberately
make a converted copy using appropriate image tools. Changing embedded images
with **Adjust Cover** is a separate audio-file mutation; keep originals and
inspect its scope before choosing that route.

## The solution

1. Copy one album folder, including any existing folder images, and keep
   the original elsewhere. Save pending tag edits in OpenTag.
2. Select one copied audio file with the intended embedded **Front Cover**.
   Inspect it in **Manage covers…** or the cover window. A **Folder image /
   Not embedded in the file** fallback is not an embedded source for this action.
3. Inspect the copied folder's current image files. If an existing `folder.jpg`
   matters, keep a separate copy or choose another output basename for the trial.
4. Choose **Actions → Actions (Quick)…**, then **Export Cover to File**.
   Enter these settings:

    <!-- folder-export-settings:start -->
    | Setting | Value |
    |---|---|
    | Output filename format (extension is added) | `folder` |
    | Export covers of type | Front Cover |
    | Export duplicate covers | Off |
    <!-- folder-export-settings:end -->

5. Choose **Apply action** and read the result and any warnings. This writes
   the external image immediately; it does not wait for **Save tags**.
6. Open the copied folder and inspect `folder.jpg`. For a JPEG source, its
   bytes are copied without resizing or recompression. Check that the embedded
   Front Cover is still present in the audio file too.

A relative `folder` is resolved beside that selected audio file. The action
adds an extension from the embedded picture's actual encoding. It is not a
wildcard search for an existing external image.

## How filenames and image formats interact

<!-- folder-export-name-cases:start -->
| Output format | Embedded image | Output name |
|---|---|---|
| `folder` | JPEG | `folder.jpg` |
| `folder` | PNG | `folder.png` |
| `folder.jpg` | PNG | `folder.png` |
| `folder.jpeg` | JPEG | `folder.jpeg` |
| `Vol. 2` | JPEG | `Vol. 2.jpg` |
<!-- folder-export-name-cases:end -->

The extension comes from the image, not `.mp3` or `.flac`. An existing matching
image extension is retained; another recognized image extension is replaced.
A dot elsewhere in the basename does not necessarily mean an image extension,
which is why `Vol. 2` receives `.jpg` afterward.

When several embedded Front Covers qualify, the first uses `folder.jpg` and
later qualifying positions receive names such as `folder (1).jpg`. A repeated
image can be skipped with duplicate export off, while later numbering still
reflects its qualifying position. Inspect the source cover set if you want
one particular image rather than several qualifying fronts.

## Existing files and repeated images

An existing output with identical image bytes stays unchanged. A different
image can be replaced. **Export duplicate covers** controls repeated content
during the run; turning it off is not an “avoid overwriting existing images”
setting. A run that finds only identical outputs may add no undo step.

For an album in which every track has the same cover, one representative
track is enough for this trial. If you select more tracks, repeated content
at the same requested destination can be skipped. This does not compare
musical album identities or choose the best image.

Different embedded pictures are not duplicates. When different selected
files attempt different images at the same output name in one run, a later
file can warn that it was already written for another file. Do not treat
processing order as an artwork-selection policy; inspect the first output
and all warnings, then choose the intended representative track explicitly.

## Variations

### Export from several album folders

Select one representative copied track per album and use the same relative
`folder` format. Each file writes beside its own audio. An absolute output
instead points the selected files toward the same location.

There is no **Create one output file per** partition control in this action.
The per-album behavior comes from your selection and each input's folder.
**File → Export…** creates text-based reports with separate partition controls;
it is not this image-export action.

### Give each track a separate output

Use this in **Output filename format (extension is added)**:

<!-- folder-export-per-track:start -->
```text
%_filename%-cover
```
<!-- folder-export-per-track:end -->

For `03 - Paper Boats.flac` containing a PNG front cover, it requests
`03 - Paper Boats-cover.png` in that audio file's folder. The basename
placeholder omits the audio extension. This distinguishes ordinary per-track
names; it does not make every possible destination or filesystem alias unique.

### Store images in an explicit subfolder

Use `art\folder` when you intentionally want an `art` subfolder below each
input's folder. Literal backslashes separate folders in filename formats
on supported desktop platforms. Tag-derived values and native path placeholders
have their own handling; see the [action reference](../actions/export-cover.md)
before building more complex destinations.

## Recovery and limits

This image-export action can record created and replaced external images in
session undo. After a successful recorded change, inspect **Edit → Undo** and
its operation label: Undo can remove a created image or restore a replaced
one, and Redo can recreate the export. Empty output folders can remain.
This is specific to this action; it is not a promise that ordinary report
exports or arbitrary external files have the same undo behavior.

Snapshot retention and available space can prevent preparing an undo capture.
The action can warn before a write when that capture cannot be prepared; a
later retention failure can also produce an undo notice. Read the actual
result and available undo entry instead of assuming the previous operation
label now represents this export. If history itself is unavailable, an export
can still run without an undo entry. Keep any image you need beyond the session.

A file without a qualifying embedded cover produces no image for it. A missing
image, unsupported bytes or file error is a reason to inspect that source;
export does not download, invent or repair artwork. Save a copied result and
test it with the receiving player or folder viewer when compatibility matters.

## Related

- [Export Cover to File reference](../actions/export-cover.md)
- [Working with embedded covers](../guides/covers.md)
- [Embed each folder image](embed-folder-jpg.md)
- [Resize embedded covers](resize-covers.md)
