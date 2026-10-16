# Export

Use the **Export** action to produce a text report from selected files as
part of a saved action group. An export configuration defines the report's
contents; an output filename format decides where to write it.

## Produce a small track report

1. In a plain-text editor, save this configuration as a UTF-8 file named
   `practice-report.mte`:

    ```text
    $filename(tags.txt,utf-8)$loop(1)%artist% - %title%
    $loopend()
    ```

2. Save pending tag edits and select copied practice files in the order
   you want them listed.
3. Choose **Actions → Actions (Quick)…**, then **Export**.
4. Next to **Export configuration**, choose **Browse…** and select
   `practice-report.mte`.
5. In **Output filename format**, enter `tags.txt`.
6. Leave **Append data** and **Create one output file per partition** off.
7. Choose **Apply action**, then open the report beside the first selected
   audio file.

With two selected titles, the content can look like:

```text
Juniper Vale - Paper Boats
Juniper Vale - North Pier
```

`$filename` declares an export destination and encoding, `$loop(1)` repeats
its body in input order, and `$loopend()` ends the repetition. A constant
loop key keeps the input order; a key such as `%_path%` sorts by that value.
Field-key loops follow **Options → General → natural sorting**: enabled,
numbered names compare as 1, 2, 10; disabled, they use text order. Constant
loop keys retain input order. These are export-template commands, separate
from ordinary tag-format functions.

## Configure the output

| Setting | Meaning |
|---|---|
| **Export configuration** | A saved configuration name, an `.mte` file path, or complete export-template text |
| **Browse…** | Choose an export configuration file |
| **Output filename format** | A format string overriding the configuration's output filename; leave empty to use its declaration |
| **Append data** | Append rendered data rather than replacing the output |
| **Create one output file per partition** | Enable grouping by **Partition format** |
| **Partition format** | A per-file format string whose equal results belong to the same output group |

**Export configuration** is a single-line input. Use a saved configuration
name or an `.mte` file path for templates containing line breaks; do not
paste the multiline file contents into that box. A complete one-line
template can be entered directly. Export settings control byte-order marks
and whether missing output folders may be created.

## Partition a report

To write one report per album folder, enable partitioning and use:

| Setting | Value |
|---|---|
| **Partition format** | `%_folderpath%` |
| **Output filename format** | `%_folderpath%tags.txt` |

All selected files with the same folder path enter the same report.
The filename and the grouping key serve different purposes. Ensure each
partition has a distinct output path; do not group by disc number while
sending every disc to the same `tags.txt`.

The action uses the selected files' final values in its group. Export
runs after the per-file actions even when its row is listed first. Files with
an earlier action warning still participate. Use a separate earlier group if
the report must capture values before cleanup.

## Gotchas

- Export writes a report; it does not create copies of the audio files.
- Appending can repeat rows when the group is run again. Check the output
  before enabling it in a frequently used workflow.
- A relative output path is based on the first input file's folder. Use
  an explicit destination for a selection spanning several folders.
- Protect existing reports and check operation errors. Generated output
  may require separate cleanup, and earlier group effects can remain
  after a later error.

<!-- verified: src-tauri/src/action_host.rs:resolve_export_template, NativeHost::execute_batch; crates/tag-core/src/export.rs -->

## Related

- [Generate Playlists](generate-playlists.md)
- [Format strings](../reference/format-strings.md)
- [Placeholders](../reference/placeholders.md)
- [Actions and action groups](index.md)
