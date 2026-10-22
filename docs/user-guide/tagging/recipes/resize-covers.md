# Shrink oversized covers without enlarging small ones

Reduce the longer side of large embedded images to at most 600 pixels while
keeping their proportions. This recipe changes the pictures inside selected
audio files; it leaves a separate `folder.jpg` untouched.

## Choose the scope before applying

**Adjust Cover** as an action affects all embedded pictures in each selected
file, including back covers and booklet images. If only selected front
pictures should change, use **Manage covers…**, select those rows and choose
**Adjust…** there instead; its changes wait for that dialog's **OK**.

Keep original copies before shrinking or converting images. Resampling can
remove detail, and a small source cannot regain detail by being enlarged.
A player that requires a square or JPEG image needs a different explicit
choice, not an assumption that every resize produces one.

## Apply a maximum-size proportional resize

1. Save or discard pending edits and select a few copied practice files.
2. Choose **Actions → Actions (Quick)…**, then **Adjust Cover**.
3. Use these settings and inspect their scope once more.
4. Choose **Apply action** and inspect dimensions and file errors afterward.

<!-- resize-settings:start -->
| Setting | Value |
|---|---|
| Image format | Original |
| Quality | High (`90` for JPEG) |
| Resize cover | On |
| Size in pixels | `600` |
| Size policy | Maximum size |
| Scaling | Scale proportionally |
<!-- resize-settings:end -->

Quality applies when encoding JPEG. It does not select dimensions or control
PNG compression. With Original format, an already small image keeps its
format and bytes when no resize is needed.

<!-- resize-main-cases:start -->
| Original width | Original height | Result width | Result height |
|---|---|---|---|
| `1600` | `1000` | `600` | `375` |
| `800` | `1200` | `400` | `600` |
| `1200` | `1200` | `600` | `600` |
| `400` | `400` | `400` | `400` |
<!-- resize-main-cases:end -->

For a 1600 × 1000 picture, 600/1600 is the scale, so the other side becomes
1000 × 600/1600 = 375. The longer side controls the maximum. The 400 × 400
picture is already within it and is not enlarged.

Picture type and description remain associated with adjusted artwork. An
action can resize several pictures in one file; inspect all of them rather
than only the image currently displayed first.

## Distinguish maximum, minimum and square policies

These examples all use Size in pixels `600`, but change the other controls:

<!-- resize-policy-cases:start -->
| Original width | Original height | Size policy | Scaling | Result width | Result height |
|---|---|---|---|---|---|
| `1200` | `800` | Maximum size | Scale proportionally | `600` | `400` |
| `1200` | `800` | Minimum size | Scale proportionally | `900` | `600` |
| `1200` | `800` | Maximum size | Scale to square | `600` | `600` |
| `1200` | `800` | Maximum size | Crop to square | `600` | `600` |
| `400` | `200` | Maximum size | Crop to square | `200` | `200` |
| `400` | `200` | Minimum size | Crop to square | `600` | `600` |
<!-- resize-policy-cases:end -->

Minimum size matches the shorter side and can enlarge an image. Scale to
square stretches it; Crop to square keeps proportions while discarding
excess around the center. Square shaping still applies when a picture is
already within Maximum size. The 400 × 200 crop becomes 200 × 200 without
enlargement. With size disabled, stretching uses the longer source side and
cropping uses the shorter side. Older saved TagGroove actions preserve their
previous rule that left an already-small image unchanged; recreate the action
to adopt these native sizing rules.

A very thin image can need a large intermediate buffer when enlarged or made
square. Adjustments exceeding the conservative 512 MiB working-buffer budget
are refused before cropping/resampling, with the current file unchanged. Use a
smaller target or maximum-size proportional scaling, then inspect the result.

## Choose encoding deliberately

Original keeps a supported writable original format. Select JPEG or PNG
when a new encoding is intended or the original format cannot be written.
JPEG does not preserve transparency; inspect its appearance before replacing
transparent originals. PNG can preserve it. Changing a filename extension
alone does not convert image bytes.

A JPEG whose format and dimensions remain the same is left byte-for-byte
unchanged, even if Quality was changed. Quality alone is therefore not a
“recompress all JPEGs” command. When conversion or resizing occurs, file size
still depends on content, dimensions and encoding; a quality number is not
a guaranteed byte budget.

## Save timing and recovery

The action writes when run. **Save groups** stores a reusable setup without
running it; applying its checked group does the work. In **Manage covers…**,
Adjust stages replacement images and **OK** commits the plan. A direct cover
window **Adjust cover…** has its own immediate operation.

Inspect errors and saved artwork on the intended player. A successful core
resize is not proof that a particular device accepts that size, format or
picture role. Use **Edit → Undo …** for recorded writes in the current desktop
session and keep original images/audio for recovery beyond it.

## Related

- [Adjust Cover settings](../../actions/adjust-cover.md)
- [Working with covers](../covers.md)
- [Embed a folder image](embed-folder-jpg.md)
- [Keep front covers](keep-only-front-cover.md)

<!-- verified: crates/tag-core/src/cover_ops.rs:resize_image,adjust_image,adjust_covers_with_options; src/covers.ts:manage,perform. Actual written dimensions/policies checked against core; role/description retention traced in adjust_covers_with_options. No native/device claim. -->
