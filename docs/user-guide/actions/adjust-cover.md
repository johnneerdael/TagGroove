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
| **Resize cover** | Off initially; enables the size and scaling settings |

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

**Size in pixels** accepts 1–32768. The policy selects which dimension is
compared with that number; scaling selects the resulting shape.

| Size policy | Meaning |
|---|---|
| **Maximum size** | Reduce images whose larger side exceeds the size; leave images already within it unchanged |
| **Minimum size** | Match the smaller side to the size, enlarging or reducing as necessary |

| Scaling | What happens when resizing is needed |
|---|---|
| **Scale proportionally** | Keep the aspect ratio |
| **Scale to square** | Stretch to exactly the requested width and height |
| **Crop to square** | Keep proportions, fill the requested square, and crop the excess around the center |

With a 1200 × 800 cover and size `600`:

| Policy and scaling | Result |
|---|---|
| Maximum + proportional | 600 × 400 |
| Minimum + proportional | 900 × 600 |
| Maximum + square | 600 × 600, stretched |
| Maximum + crop | 600 × 600, cropped |

Maximum size does not force a small image into a square: a 300 × 200 image
already within a maximum of 600 keeps its shape in every scaling mode.
Use the policy deliberately when a player needs a square image.

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
