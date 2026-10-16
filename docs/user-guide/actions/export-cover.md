# Export cover to file

Use **Export Cover to File** to save embedded artwork as separate image
files. For example, export each album's front cover beside its audio files
for a player that reads `folder.jpg`. Exporting does not remove the embedded
cover. This action works with local files; action groups are not available
for provider documents yet.

## Add the action

1. Select a few practice files and open **Actions → Actions…** (++alt+6++;
   Alt is ⌥ Option on a Mac).
2. Create a group, choose **Edit actions…**, and add **Export Cover to File**.
3. Set **Output filename format (extension is added)** to:

    ```text
    %_folderpath%folder
    ```

4. Set **Export covers of type** to **Front Cover**. Choose **All cover types**
   when other artwork should be exported too.
5. Leave **Export duplicate covers** off to skip repeats of the same image
   for the same requested output name during this run.
6. Save the group, check only that group, and run it. Open the audio file's
   folder to inspect the exported image.

The image's format determines the extension: a JPEG becomes `folder.jpg`,
a PNG becomes `folder.png`, and a WebP becomes `folder.webp`. An extension
entered in the format is corrected to the image's type. Additional covers
exported from one file receive a suffix such as `folder (1).png`.

## Names and duplicate covers

[Placeholders](../reference/placeholders.md) and
[format strings](../reference/format-strings.md) are evaluated for each file.
A relative name is resolved beside the audio file. Use `\` to create folder
components on every platform. `%_folderpath%`, `%_path%` and `%_workingpath%`
keep their path components; ordinary tag values have separators and reserved
filename characters cleaned.

**Export duplicate covers** controls repeats within the current run, not
whether existing image files can be replaced. With it off, an identical
image requested for the same name is skipped. With it on, repeated covers
are processed, but identical bytes already at a destination need no rewrite.

## Existing files and warnings

An existing destination with identical image bytes stays unchanged. A
different image is replaced. Use **Edit → Undo** (++cmd+z++ / ++ctrl+z++) in
the same session to restore a replaced image. Newly created image files
remain after undo; remove them separately if needed. Keep backups of images
to retain beyond the current session.

When two selected files try to export different images to the same name in
one run, the later file gets a warning and the earlier image stays. A file
may replace its own earlier export after another action changes its cover.
Another file that finds those same image bytes already there does not take
over ownership of the output name.

Output that would replace an input audio file, including an alias through a
symlink, is refused. A warning does not stop the file's remaining actions
or later groups. Check the warning list and the images before repeating a
group; earlier successful writes remain in place.

<!-- verified: src-tauri/src/action_host.rs:cover_export_replaces_existing_files_and_rejects_input_aliases; cover_export_of_an_image_already_there_keeps_its_writer; cover_export_cross_file_destination_keeps_the_first_different_image -->

## Related

- [Import cover from file](import-cover.md)
- [Placeholders](../reference/placeholders.md)
- [Format strings](../reference/format-strings.md)
