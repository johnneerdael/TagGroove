# Text File → Tag

Use **Text File → Tag** to import one structured text record per selected
audio file. A mask assigns parts of each record to tag fields. Match by
list order for a controlled selection, or include a filename/path key to
connect records to files explicitly.

## Import two pasted records

1. Keep copies, save pending edits and select two practice files in the
   order `Paper Boats`, then `North Pier`.
2. Choose **Convert → Text File → Tag…** (++alt+4++; Alt is ⌥ Option on a Mac).
3. Set **Text source** to **Pasted text** and enter in **Records to import**:

    ```text
    Paper Boats;Juniper Vale;3
    North Pier;Juniper Vale;4
    ```

4. Set **Import mask** to:

    ```text
    %title%;%artist%;%track%
    ```

5. Choose **Preview** and inspect every destination file, source line and
   captured field. The first selected file should receive track `3`, and
   the second track `4`.
6. Choose **Save tags to … files** to write the reviewed values immediately.

Each named field is replaced with its captured value; fields absent from
the mask are retained. The mask's semicolons are literal separators. It is
not a spreadsheet CSV parser: quoting a cell does not automatically protect
delimiters inside that cell.

## Read a text file

Set **Text source** to **Text file**, then choose **Choose file…** or enter
a path in **Source file**. The path can itself be a format string:

```text
%_folderpath%tracks.txt
```

It is evaluated from the **first selected audio file**. One source file is
read for the whole selection; this expression does not independently read
`tracks.txt` in every selected album folder. Process one album at a time
when each folder has its own text listing.

UTF-8 and BOM-marked Unicode are detected first. **Legacy encoding fallback
(optional)** selects how to decode older non-UTF-8 files, for example
`windows-1251` or `windows-1252`. It is used only when the source is not
valid UTF-8. Check non-ASCII characters in the preview before applying.

## Match records by filename

When the records might be in a different order from the file list, add
`%_filename_ext%` to the mask. For selected files `03.mp3` and `04.mp3`, this
text intentionally lists track 4 first:

```text
04.mp3;North Pier;4
03.mp3;Paper Boats;3
```

Use:

```text
%_filename_ext%;%title%;%track%
```

`04.mp3` is matched to that selected filename regardless of its position.
The key includes the extension but no folders. Two selected files with
the same filename are ambiguous; use absolute paths instead.

For example, a record `/Music/Harbour Lights/03.mp3;Paper Boats` matches with:

```text
%_path%;%title%
```

`%_path%` requires an absolute path. Backslashes and forward slashes are
normalized for path-key matching. **Match filename and path keys
case-sensitively** is checked initially; uncheck it only when the intended
matching rule ignores case. It controls keys, not the literal punctuation
of the mask or the capitalization of imported values.

## Shape the records

- Use `%dummy%` to skip an unwanted part: `%dummy%;%title%;%artist%`.
- Repeat a field name to create multiple values:
  `%title%;%artist%;%artist%` captures two artist values.
- Include a real separator between captures. `%artist%%title%` is rejected.
- Match each complete line. Extra prefix/suffix text must be represented
  by literals or a `%dummy%` capture.
- Captured whitespace is preserved. Put fixed padding in the mask or clean
  it in a later field-formatting step.

A positional import requires exactly one record per selected file, in the
intended order. A keyed import can contain rows for files outside the
selection; those rows are skipped. Selected files with no matching row keep
their tags and appear as **No match, not changed**. This lets you select a
few tracks from a whole-album text listing without deleting its other rows.
If no row names a selected file, the plan reports that no changes are planned.

Both modes reject malformed records, duplicate selection paths and repeated
matches for the same selected file before importing tags. A final line ending
is allowed; an extra blank record is still a record, not an ignored comment.
Inspect both matched and unmatched rows before applying a partial plan.

## Gotchas

- This converter imports one record into several fields. To put an entire
  multiline file into one lyrics or comment field, use the Import Text
  File action instead.
- Mask placeholders for technical properties cannot be written. The only
  supported underscore placeholders are the two matching keys, with at
  most one key in a mask.
- Editing inputs, changing the selection or changing a source after preview
  requires a new preview. Check errors instead of repeatedly applying a
  stale plan.
- Applying saves immediately. Keep originals and inspect **Edit → Undo**
  for recorded changes in the current desktop session.

<!-- verified: crates/tag-core/src/converters.rs:TextTagConverter, text_lines, match_key; src/main.ts:reviewedConverter -->

## Related

- [Filename → Tag](filename-to-tag.md)
- [Tag → Tag](tag-to-tag.md)
- [Placeholders](../reference/placeholders.md)
- [Converters](index.md)
