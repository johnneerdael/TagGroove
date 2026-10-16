# Embed each folder's folder.jpg

Copy an album-folder image into its audio files so the picture can travel
with them. A picture already displayed as **Folder image / Not embedded in
the file** is still an external file; seeing it in the tag panel does not
mean it is inside the audio.

## Prepare one album copy

Put the intended image in the copied album folder as `folder.jpg`. Check
that it opens as a real supported image, and select a few copied audio
files with a writable artwork-capable tag, such as MP3 with ID3v2 enabled.
Keep the separate high-resolution image and original audio files.

Start with the exact filename rather than a broad wildcard. When several
album folders are selected, each needs its own image in the expected place.
This action does not find online artwork or choose an image by album title.

## Run the import once

1. Save or discard pending edits and select the practice audio files.
2. Choose **Actions → Actions (Quick)…**, then **Import Cover from File**.
3. Enter the settings below. The filename expression includes the extension.
4. Choose **Apply action** and inspect the operation result.
5. Select each practice file separately and inspect its embedded cover set.

<!-- embed-settings:start -->
| Setting | Value |
|---|---|
| Image filename format | `%_folderpath%folder.jpg` |
| Import cover as | Front Cover |
| Delete existing covers | Off |
<!-- embed-settings:end -->

`%_folderpath%` already ends with the file's native folder separator. Appending
`folder.jpg` reads that folder's image; it does not introduce a second
separator. A relative `folder.jpg` also resolves beside each audio file.
An absolute filename chosen with Browse instead reads the same image for
every selected file.

The action writes immediately. There is no later **Save tags** step for the
completed import. Saving an equivalent group with **Save groups** only stores
the workflow; applying the checked group runs it.

## Understand adding versus replacing

With deletion off, existing artwork stays and a new front-cover picture is
added. A repeated import can add another picture. It is not an automatic
“replace the old front cover” operation.

<!-- embed-count-cases:start -->
| Existing embedded pictures | Delete existing covers | Source image | Pictures after successful import, or retained after refusal |
|---|---|---|---|
| `0` | Off | Valid | `1` |
| `1` | Off | Valid | `2` |
| `2` | Off | Valid | `3` |
| `2` | On | Valid | `1` |
| `2` | On | Invalid image | `2` |
<!-- embed-count-cases:end -->

These are practice MP3 cases with supported picture roles/descriptions.
The deletion-on row removes all existing pictures, including booklet and
back images, before adding the new front. Use [Manage covers](../guides/covers.md)
to replace one selected image while keeping the others instead.

A missing or invalid image is refused before that removal step, but a later
write failure still needs inspection. Read all file errors and keep backups;
a multi-file run is not one atomic write. Ordinary action warnings do not
stop later actions/groups, so a following adjustment may still act on a
file's old artwork if its import warned.

## Use a wildcard only when needed

<!-- embed-wildcard-format:start -->
For an explicit local naming convention, `%_folderpath%front*.png` can select
the first importable matching image in natural, case-insensitive name order.
<!-- embed-wildcard-format:end -->
For example, this folder's candidates are considered as follows:

<!-- embed-wildcard-cases:start -->
| Candidate | What it contains | Result for front*.png |
|---|---|---|
| `front1.png` | Plain text, not an image | Skipped |
| `Front2.PNG` | Valid PNG | Chosen |
| `front3.png` | A folder | Skipped |
| `front10.png` | Valid PNG | Later valid candidate |
<!-- embed-wildcard-cases:end -->

The action does not compare image quality or recognize which picture is the
correct front. A broad `*` can select unwanted artwork. Filename matching
uses the final component; folder components are literal existing paths,
not a recursive search pattern. See [Import Cover](../actions/import-cover.md)
for accepted formats and path-placeholder behavior.

## Verify and recover

The audio format must support embedded pictures; ID3v1 alone cannot store
them. Some formats cannot preserve every role or description. Check
[format capabilities](../reference/file-formats.md) and the saved result
instead of assuming a folder image proves a successful embed.

Use **Edit → Undo …** for a recorded import in the current desktop session.
Undo is not a replacement for retaining the original files. These action
paths require local files; provider-document action groups are unavailable
in the current baseline.

## Related

- [Import Cover from File](../actions/import-cover.md)
- [Working with covers](../guides/covers.md)
- [Shrink oversized covers](resize-covers.md)
- [Keep front covers](keep-only-front-cover.md)

<!-- verified: src-tauri/src/action_host.rs:NativeHost::execute ImportCover; crates/tag-core/src/cover_ops.rs:resolve_cover_source; crates/tag-core/src/tags.rs:import_cover_with_options. Exact written settings, count cases and wildcard folder execute against core fixtures; not native UI/all-format proof. -->
