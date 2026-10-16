# Apply a tracklist from a text file

<div class="recipe-meta" markdown>
**Uses:** Text File → Tag · **Works on:** selected writable audio files ·
**Time:** about 5 minutes for one album · **Undo:** recorded tag writes in the current desktop session; keep originals
</div>

## The problem

Your audio files have anonymous names, but a booklet or text listing has the
track numbers, artist, album and titles. Import those records instead of
typing the same album information into every file.

| Selected file, in this order | Imported track | Imported title |
|---|---|---|
| `01.flac` | `1` | `Paper Boats` |
| `02.flac` | `2` | `Lanterns on the Water` |

This recipe starts with two records and a controlled file order. The
filename-key variation below is preferable when the records and files are
in different orders. Neither method identifies the audio by listening to it.

## The solution

1. Work on copies of two ordinary whole-track audio files. Save pending
   edits, select only `01.flac` and `02.flac`, and check that they appear
   in that order in the file list.
2. Choose **Convert → Text File → Tag…**. Set **Text source** to
   **Pasted text** and put this into **Records to import**:

    <!-- tracklist-records:start -->
    ```text
    81. Juniper Vale - [Harbour Lights #1] Paper Boats
    82. Juniper Vale - [Harbour Lights #2] Lanterns on the Water
    ```
    <!-- tracklist-records:end -->

3. Put this into **Import mask**:

    <!-- tracklist-mask:start -->
    ```text
    %dummy%. %artist% - [%album% #%track%] %title%
    ```
    <!-- tracklist-mask:end -->

4. Choose **Preview**. Inspect the **destination file**, source line and
   captured fields for both rows. The first file should receive title
   `Paper Boats`, artist `Juniper Vale`, album `Harbour Lights` and track `1`.
   The second should receive `Lanterns on the Water` and track `2`.
5. Choose **Save tags to 2 files** only after those associations are correct.
   This writes the tags immediately; it does not wait for a later **Save tags**.
6. Check both files in **Extended tags…**, including a field you intended
   to retain. Fields named in the mask are replaced; fields outside it remain.
   Read any per-file errors before treating the whole batch as complete.

If a result is wrong, stop before importing more albums. **Edit → Undo** can
restore recorded tag changes in this desktop session. Keep the original
copies for recovery beyond that session and for comparing player behavior.

## How it works

The mask reads the **whole record**, from left to right. It is an extraction
pattern, not a scripting expression that calculates a new string.

| Mask part | What it reads from the first record |
|---|---|
| `%dummy%` | `81`, which is discarded rather than stored as a field |
| `. ` | A literal period and space |
| `%artist% - ` | Artist up to the literal ` - ` separator |
| `[%album% #%track%] ` | Album and track inside literal brackets, separated by ` #` |
| `%title%` | The remaining title |

The brackets here are literal record punctuation. They are not the optional
sections used in an output format string. Literal spaces and separators
must match the actual records. Captured whitespace is preserved, so inspect
padding rather than assuming it will be trimmed.

Without a key, record 1 goes to selected file 1, record 2 to selected file 2,
and so on. The number `81` does not choose the file; `%dummy%` discards it.
A positional import requires exactly as many records as selected files.

## Variations

### Read one album's saved tracklist

Set **Text source** to **Text file**, choose the listing, or use this in
**Source file**:

<!-- tracklist-source:start -->
```text
%_folderpath%tracks.txt
```
<!-- tracklist-source:end -->

The expression uses the **first selected audio file**. It reads one listing
for the entire selection, even when that selection spans several folders.
Process one album folder at a time if each has its own `tracks.txt`.
Use UTF-8 for a new listing; check accents in the preview when decoding
older text with **Legacy encoding fallback (optional)**.

### Match by filename instead of position

For selected files `01.flac` and `02.flac`, these records deliberately put
track 2 first:

<!-- tracklist-keyed-records:start -->
```text
02.flac;Lanterns on the Water;2
01.flac;Paper Boats;1
03.flac;North Pier;3
```
<!-- tracklist-keyed-records:end -->

Use this mask:

<!-- tracklist-keyed-mask:start -->
```text
%_filename_ext%;%title%;%track%
```
<!-- tracklist-keyed-mask:end -->

Now `02.flac` receives the first record, and `01.flac` the second. The record
for unselected `03.flac` is skipped. This mask does not set artist or album.
If you also selected `04.flac`, it would have **No match, not changed** and
keep its tags. Review unmatched files as carefully as matched ones.

Filename keys include the extension. Two selected folders containing the
same filename are ambiguous; use an absolute `%_path%` key instead.
**Match filename and path keys case-sensitively** controls key comparison,
not the case of the imported title. Leave it on unless the intended keys
really differ only in capitalization.

### Start from a simple delimited listing

For plain records such as `1;Juniper Vale;Paper Boats`, use
`%track%;%artist%;%title%`. Choose a separator absent from those values.
A spreadsheet CSV with escaped quotes, embedded separators or multiline
cells requires deliberate conversion first: this converter is **not a CSV
decoder**. In particular, doubled CSV quotes are not automatically unescaped.
Keep filenames or paths when preparing a file-keyed import.

## When preview refuses the listing

| Problem | Check before trying again |
|---|---|
| Wrong number of records | Remove a header before positional import; compare the record count with the selection |
| Extra blank record | Remove unintended empty lines; a final line ending is allowed, but an additional blank record is still a record |
| Record does not match the mask | Compare punctuation and spacing; use `%dummy%` for an unwanted prefix |
| One file matched twice | Correct duplicate keys rather than expecting later records to override earlier ones |
| No selected files match | Check extensions, absolute paths, key case and the current selection |
| Preview became stale | Run **Preview** again after changing input, selection or source |

Do not use this record importer for multiline lyrics. Import the complete
sidecar into one field with the [lyrics recipe](lyrics-from-text-files.md).

## Related

- [Text File → Tag reference](../converters/text-file-to-tag.md)
- [Tag from folder and file names](tags-from-folder-and-filename.md)
- [Spreadsheet CSV workflow](csv-for-spreadsheets.md)
- [Undo and history](../guides/undo-history.md)
