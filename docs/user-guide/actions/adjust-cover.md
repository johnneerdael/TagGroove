# Adjust Cover

Use **Adjust Cover** to resize or change the format of pictures embedded
in the selected audio files. It changes all embedded pictures in each file;
it does not change a separate `cover.jpg` alongside the music.

## Shrink oversized covers

1. Keep original copies and select a few practice files.
2. Choose **Actions → Actions (Quick)…**, then **Adjust Cover**.
3. Keep **Image format** as **Original**.
4. Check **Resize cover**, set **Size in pixels** to `600`, choose
   **Maximum size** and **Scale proportionally**.
5. Choose **Apply action**, then inspect the cover dimensions.

A 1200 × 800 image becomes 600 × 400. A 300 × 300 image keeps its size.
The picture's cover type and description are retained.

## Format and quality settings

| Setting | Choices and effect |
|---|---|
| **Image format** | **Original**, **JPEG**, or **PNG** |
| **Quality** | **Best** = 100, **High** = 90, **Medium** = 75, **Low** = 50, or **Custom** |
| **Quality (0–100)** | A whole number used when encoding JPEG |
| **Resize cover** | Off initially; enables the target size. Square shaping can also apply with size disabled |

**Original** retains a writable original format: JPEG, PNG, GIF, BMP or
WebP. If TagGroove cannot write that original format, choose JPEG or PNG.
PNG preserves transparency; JPEG does not. Quality does not control PNG
compression or resize dimensions.

If the format and dimensions remain the same, the picture stays byte for
byte unchanged. Changing JPEG quality alone therefore does not recompress
an otherwise unchanged JPEG. Choose the format and resizing operation to
match the actual goal, and check the resulting file size rather than
assuming a particular quality number guarantees it.

## Size policy and scaling

New adjustments and native imported actions use a target of 10–10000 pixels
when **Resize cover** is checked. An enabled target outside that range skips
the whole adjustment, including format conversion. Imported zero or out-of-range
targets stay unchanged when the action is opened and saved.

With size disabled, proportional scaling keeps the dimensions. **Scale to
square** stretches a rectangle to the longer side; **Crop to square** takes a
centered square using the shorter side. Size and shape are independent.

| Size policy | Proportional scaling |
|---|---|
| **Maximum size** | Reduce the larger side to the target when it exceeds it |
| **Minimum size** | Match the smaller side to the target, enlarging or reducing as necessary |

Square shaping also applies when no size change is needed. With **Maximum
size**, stretching uses the smaller of the target and longer source side;
cropping uses the smaller of the target and shorter source side. It does not
enlarge a crop whose shorter side is already below the maximum.

With a 600 × 300 cover:

| Settings | Result |
|---|---|
| Maximum 500 + proportional | 500 × 250 |
| Maximum 500 + square | 500 × 500, stretched |
| Maximum 500 + crop | 300 × 300, centered crop |
| Size disabled + square | 600 × 600, stretched |
| Size disabled + crop | 300 × 300, centered crop |
| Minimum 200 + square or crop | 200 × 200 |

For minimum size, when the shorter side already equals the target, only shape
is applied: minimum 300 + square stretches this 600 × 300 example to 600 × 600.
Minimum 300 + crop produces 300 × 300.

Previously saved TagGroove actions keep their earlier sizing behavior and
1–32768 target range. In those actions, maximum size leaves an image already
within the target unchanged in every shape mode; square/crop resizes that do
run use the requested square. Editing their existing settings preserves those
rules. Create a new action to use the native sizing rules. Exchanging an older
rule or a JPEG quality below 20 uses an app-only `.mta` record when needed;
review the export notice.

Large dimensions can require much more working memory than the compressed
image file. TagGroove checks a conservative 512 MiB resize working-buffer
budget before cropping or resampling, including intermediate images. A request
that exceeds it reports an error and leaves the current file unchanged. Choose
smaller dimensions. This applies to both new native sizing rules and older saved
actions; it does not change their pixel targets or the image input limits.

## Gotchas

- Enlarging cannot restore image detail. Keep a high-resolution original
  outside the audio file if it may be needed later.
- Cropping and JPEG conversion can lose information. Copy the files first;
  inspect **Edit → Undo** for recorded changes in the current session.
- An invalid image or unsupported output format reports an error. Check
  the operation result before processing the rest of the library.

<!-- verified: crates/tag-core/src/cover_ops.rs:resize_image, adjust_image; src/action-ui.ts:qualityPresets -->

## Related

- [Export Cover to File](export-cover.md)
- [Import Cover from File](import-cover.md)
- [Set Cover Properties](set-cover-properties.md)
- [Actions and action groups](index.md)
