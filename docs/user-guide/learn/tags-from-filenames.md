# Get tags from filenames

Turn reliable names into saved track, title, artist and album fields. In
about 15 minutes you will read two filenames, add their parent-folder
information, and learn to recognize a pattern that matches but captures
the wrong meaning. The names themselves will stay unchanged.

- [ ] Two copied, writable whole-track audio files; keep the originals elsewhere.
- [ ] A practice layout like the one below, with no unsaved OpenTag edits.
- [ ] Familiarity with selecting files and opening **Extended tags…**.

<!-- learn-filename-layout:start -->
```text
Music/Juniper Vale/Harbour Lights (2024)/01 - Paper Boats.mp3
Music/Juniper Vale/Harbour Lights (2024)/02 - Lanterns on the Water.mp3
```
<!-- learn-filename-layout:end -->

Use copies already named that way, or name your practice copies in your
file manager before opening them. Do not change an audio extension to
pretend a different format. This lesson writes tags from your names;
it does not verify their musical accuracy.

## 1. Inspect names and tags separately

Open the copied album through **File → Change directory…**. Select the
first file and open **Extended tags…**. Compare its `TITLE` and `TRACK`
with `01 - Paper Boats.mp3` in the file list. A good filename and an empty
title can coexist because they are stored in different places.

Close Extended tags without changing anything. Select both practice files.
Save or discard any pending edits before using a converter. Its Apply step
writes immediately, unlike ordinary tag-panel edits that wait for Save.

## 2. Draw a pattern of the filename

Choose **Convert → Filename → Tag…** and put this in **Filename pattern**:

<!-- learn-filename-basic:start -->
```text
%track% - %title%
```
<!-- learn-filename-basic:end -->

Read it as “capture a track, then find a space, hyphen and space, then
capture the title.” Here `%track%` names a **destination field**. It does
not read an existing track value. The extension is omitted automatically.

Choose **Preview**. You should see these captures:

<!-- learn-filename-basic-cases:start -->
| Filename | TRACK | TITLE |
|---|---|---|
| `01 - Paper Boats.mp3` | `01` | `Paper Boats` |
| `02 - Lanterns on the Water.mp3` | `02` | `Lanterns on the Water` |
<!-- learn-filename-basic-cases:end -->

Inspect **both** rows. A preview is the place to discover a wrong separator
or an unexpected title, before applying it to audio files.

## 3. Apply and verify the first conversion

Choose **Apply to 2 files**. The captured fields are saved now. Open each
file's Extended tags and check `TRACK` and `TITLE`; other fields, such as an
existing comment, are retained. The audio filenames remain unchanged.

There is no extra **Save tags** step for these completed converter writes.
Read operation errors before assuming both files succeeded. Session
**Edit → Undo** can reverse recorded changes; keep the original files for
recovery beyond that session.

## 4. Read the artist and album folders too

Select the same two files, reopen **Filename → Tag…**, and use:

<!-- learn-filename-folders:start -->
```text
%artist%\%album% (%year%)\%track% - %title%
```
<!-- learn-filename-folders:end -->

Use literal backslashes in the pattern on macOS and Linux too. They divide
the folder levels. This relative pattern takes the last two folders plus
the filename, so the earlier `Music` folder is irrelevant.

Preview again. Both files should gain `ARTIST` = `Juniper Vale`, `ALBUM` =
`Harbour Lights` and `YEAR` = `2024`, while keeping their respective titles
and tracks. The literal ` (` and `)` separate the year from the album.
Apply only when those captures match your actual layout, then inspect the
saved fields. Those captured fields replace any previous values there.

## 5. Learn what a successful match cannot tell you

Captures take the shortest text that allows the rest of the pattern to
match. That is a matching rule, not an understanding of album names.
With the folder pattern above:

<!-- learn-filename-ambiguity:start -->
| Album folder | Captured ALBUM | Captured YEAR |
|---|---|---|
| `Harbour Lights (2024)` | `Harbour Lights` | `2024` |
| `Harbour Lights (Deluxe) (2024)` | `Harbour Lights` | `Deluxe) (2024` |
<!-- learn-filename-ambiguity:end -->

The second row matches syntactically, but its year is wrong. Cancel that
preview and handle that naming convention separately. One explicit pattern
for it is:

<!-- learn-filename-deluxe:start -->
```text
%artist%\%album% (Deluxe) (%year%)\%track% - %title%
```
<!-- learn-filename-deluxe:end -->

This deliberately omits the literal `(Deluxe)` from `ALBUM`. If you want
it in the stored album name, choose a different explicit extraction and
cleanup policy instead. A separator repeated inside an artist or title
deserves the same careful preview.

For a filename `03 - North Pier - archive.mp3`, unwanted suffix text can
be consumed with `%dummy%`:

<!-- learn-filename-dummy:start -->
```text
%track% - %title% - %dummy%
```
<!-- learn-filename-dummy:end -->

That produces track `03` and title `North Pier`, without a `DUMMY` field.
It does not know whether `archive` is disposable; that is your decision.
Adjacent captures such as `%artist%%title%` are refused because there is
no separator defining where one value ends.

## 6. Distinguish a mismatch from a bad capture

A file named `01. Paper Boats.mp3` does not fit `%track% - %title%`.
Preview marks it **No match, not changed**. Check these rows rather than
assuming every selected file was retagged. Change the pattern to fit a
separate selection; do not remove meaningful punctuation from every file
to force one pattern to work.

A matched row can still be semantically wrong, as the Deluxe example
shows. Check captured text and actual destination fields, not only the
number on the Apply button. Patterns are neither regular expressions
nor a place to put `$upper(...)`; clean captured values as a separate step.

## 7. Undo deliberately

Open **Edit** and read the **Undo** operation label. Undo the latest folder
extraction and inspect the artist/album fields; another undo can reverse
the earlier track/title conversion. Work backward through actual recorded
operations rather than assuming one undo reverses every step of a lesson.
Your copied filenames have stayed the same throughout.

## What you learned

- Filenames and tags are independent; a converter can connect them deliberately.
- A capture pattern names destinations and matches literal separators.
- Relative folder patterns read trailing path levels; `%dummy%` discards a part.
- A nonmatch stays unchanged, while a successful match still needs a meaning check.
- Applying a converter saves immediately, and undo follows recorded operations.

## Next steps

- [Tags from folder and file names](../cookbook/tags-from-folder-and-filename.md): disc folders, repeated artist captures and other layouts.
- [Filename → Tag reference](../converters/filename-to-tag.md): exact extension and matching rules.
- [Guess Values](../actions/guess-values.md): use `%title%` as a **source format string** and `%artist% / %title%` as the **guessing pattern** when `Juniper Vale / Paper Boats` is already in a tag, rather than in a filename. The source is read before those destination fields are replaced; applying that action also writes immediately.
- [Clean up a messy library](clean-a-messy-library.md): tidy known metadata after checking the extraction.
