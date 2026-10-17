# Import lyrics from .txt files

<div class="recipe-meta" markdown>
**Uses:** Import Text File action · **Works on:** selected writable whole-file tracks with matching text sidecars ·
**Time:** about 5 minutes for a practice pair · **Undo:** recorded tag writes in the current desktop session; keep originals
</div>

## The problem

You have written lyrics in separate text files and want to embed the text
in each corresponding audio file. A sidecar is a separate file kept
beside the audio; importing it copies its current contents into a tag field.
It does not create an ongoing link to the text file.

| Audio file | Text file beside it | Destination |
|---|---|---|
| `01 - Paper Boats.flac` | `01 - Paper Boats.txt` | That audio file's `LYRICS` field |
| `02 - Lanterns on the Water.flac` | `02 - Lanterns on the Water.txt` | That audio file's `LYRICS` field |

## The solution

1. Work on copies of these two audio files. Save pending edits and keep
   copies of any existing lyrics you want to retain.
2. Create the matching `.txt` files as UTF-8 text. Put this invented practice
   text in `01 - Paper Boats.txt`:

    <!-- lyrics-practice:start -->
    ```text
    Paper boats drift past the pier.
    We keep a lantern burning here.
    ```
    <!-- lyrics-practice:end -->

    Give the second sidecar different practice text, so a wrong association
    is visible. Check the actual extension: `01 - Paper Boats.txt.txt` does
    not have the name this recipe expects.

3. Select the two copied audio files. Choose **Actions → Actions (Quick)…**
   and select **Import Text File**.
4. Set **Field** to `LYRICS`. Set **Text filename format** to:

    <!-- lyrics-filename:start -->
    ```text
    %_filename%.txt
    ```
    <!-- lyrics-filename:end -->

5. Choose **Apply action**. This writes immediately. Read the result and
   any per-file errors; there is no separate reviewed text-import preview
   in this action.
6. Select each file separately and open **Extended tags…**. Check that
   `LYRICS` contains that file's complete text, with the intended line breaks
   and accents. Check the player afterward if displaying embedded lyrics
   is your goal; field storage alone does not prove player support.

The action replaces all previous values of `LYRICS` with one value.
It does not append a verse. An empty sidecar supplies empty text and can
replace existing lyrics. Use a small copied selection before a larger run.

## How it works

`%_filename%` is the audio filename without its extension. Appending
`.txt` gives the matching sidecar name. The format is evaluated separately
for every selected file, and a relative result is read from that file's folder.

<!-- lyrics-path-cases:start -->
| Audio basename | Evaluated text filename |
|---|---|
| `01 - Paper Boats.flac` | `01 - Paper Boats.txt` |
| `02 - Lanterns on the Water.mp3` | `02 - Lanterns on the Water.txt` |
| `03 - Café.ogg` | `03 - Café.txt` |
<!-- lyrics-path-cases:end -->

The complete decoded file becomes one field value. Individual lines do
not map to individual tracks, and punctuation is not treated as a separator.
Spaces, blank lines and a final line ending belong to the imported text.
Changing the sidecar afterward does not change the embedded value; rerun
the action when you intentionally want to replace it.

This uses a filename format, so normal tag-derived names have reserved
characters cleaned according to filename rules. Basing the sidecar on the
actual `%_filename%` avoids relying on a title that differs from the file's
name. Use an exact name, including `.txt`; `*.txt` is not a wildcard search.

## Variations

### Use one notes file for the whole folder

Set **Field** to `COMMENT` and **Text filename format** to `notes.txt`.
Every selected track reads the `notes.txt` in its own folder. This is useful
for album recording notes, but it deliberately gives the same text to all
selected tracks in that folder. It also replaces existing comments.

An absolute filename reads that same file for every selected track,
regardless of their folders. **Browse…** can select such a file; inspect
the resulting format before applying it to several tracks.

### Keep sidecars in a lyrics subfolder

Use this when a `Lyrics` subfolder exists beside every selected track:

<!-- lyrics-subfolder:start -->
```text
Lyrics\%_filename%.txt
```
<!-- lyrics-subfolder:end -->

The literal backslash separates folders in filename formats on supported
desktop platforms. Each text file must already exist. Importing does not
create sidecars, search other folders or download missing lyrics.

### Import an older text encoding

Use UTF-8 for new files. A Unicode byte-order mark selects its decoding;
without a BOM, valid UTF-8 is used first. Otherwise the configured ANSI/system
codepage is used, with Windows-1252 when none is available.
Unlike **Text File → Tag**, this action has no per-import legacy-encoding
box. Convert a copy of an older sidecar to known UTF-8 in a suitable text
editor before importing, then inspect accents and symbols in one test track.

## If the result is missing or wrong

| Symptom | What to inspect |
|---|---|
| A file reports that the text could not be read | Check the exact basename, extension, folder, case and read permission |
| All selected tracks receive the same lyrics | Look for an absolute filename or a constant `lyrics.txt` shared in one folder |
| Existing lyrics disappeared | The import replaces values; use recorded **Edit → Undo** or your original copy |
| Text is garbled | Check the source encoding before repeating the write |
| The player shows no lyrics | Verify the embedded field in OpenTag first, then that player's format/lyrics support |

A missing or unreadable sidecar reports an error for that operation; it is
not an instruction to clear the field or to retry later. Action processing
can continue with later work. Do not put a destructive “cleanup” action
after the import on the assumption that every sidecar was read successfully.
Keep this practice action alone until you have checked its results.

Timestamped text imported this way remains text in one field. The action
does not interpret timing markers, synchronize playback, or verify that a
player will treat the field as timed lyrics.

## Related

- [Import Text File action](../actions/import-text-file.md)
- [Actions and action groups](../actions/index.md)
- [Text encodings and codepages](../concepts/text-encodings.md)
- [Apply a tracklist](tracklist-from-text-file.md)
- [Undo and history](../guides/undo-history.md)
