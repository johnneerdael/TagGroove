# Rename files and folders

Build a two-disc folder layout from tags, review the complete destinations,
then rename a small practice selection. This lesson takes about 15 minutes.
The audio extension is retained and the tag values remain unchanged.

- [ ] Two copied audio files in one writable practice folder.
- [ ] Pending edits saved or discarded.
- [ ] Both files: `ARTIST` = `Juniper Vale`, `ALBUM` = `Harbour Lights`.
- [ ] First file: `TITLE` = `Paper Boats`, `TRACK` = `1`, `DISCNUMBER` = `1`.
- [ ] Second file: `TITLE` = `North Pier`, `TRACK` = `1`, `DISCNUMBER` = `2`.
- [ ] Original copies kept outside the folder being reorganized.

Select both files after checking their tags in **Extended tags…**
(++alt+t++; Alt is ⌥ Option on a Mac). Their current filenames can be
different from their titles; this converter will read the tags.

## 1. Preview a filename before adding folders

Choose **Convert → Tag → Filename…** (++alt+1++), and enter:

```text
$num(%track%,2) - %title%
```

Choose **Preview**. For MP3 copies, expect `01 - Paper Boats.mp3` and
`01 - North Pier.mp3`. For another audio extension, that extension remains
instead. Do not apply yet; first build the intended folder layout.

`$num` reads the track number and pads it to at least two digits. `%title%`
supplies each title. The fixed ` - ` keeps its two spaces.

## 2. Add an album folder

Change the expression to:

```text
%album%\$num(%track%,2) - %title%
```

Run **Preview** again. The album part before the backslash becomes a folder:

```text
Harbour Lights/01 - Paper Boats.mp3
Harbour Lights/01 - North Pier.mp3
```

Backslashes in the format string separate folders on macOS, Windows and
Linux. The preview can display platform-native path separators. This
relative destination is under each source file's current containing folder.

## 3. Keep disc numbers in the path

Both tracks start at 1 because they belong to different discs. Add a disc
folder so the layout retains that information:

```text
%album%\CD$num(%discnumber%,1)\$num(%track%,2) - %title%
```

Run **Preview**. For the two MP3 practice files, the final relative paths are:

```text
Harbour Lights/CD1/01 - Paper Boats.mp3
Harbour Lights/CD2/01 - North Pier.mp3
```

Read the expression in order:

| Piece | Role |
|---|---|
| `%album%` | Name the album folder |
| `\` | Start the next path component |
| `CD$num(%discnumber%,1)` | Build `CD1` or `CD2` from the disc tag |
| `\` | Begin the filename inside that disc folder |
| `$num(%track%,2) - %title%` | Build the filename stem |

Do not use this fixed `CD…` piece on files with missing disc tags: the
numeric function would produce `CD0`. Choose a selection with known disc
tags, or deliberately design an optional folder rule afterward.

## 4. Check the safety rules

Before applying, verify each destination:

- **The root is correct.** A relative path stays under the source folder.
  If the selection spans several source folders, they do not automatically
  converge on one archive root. An absolute expression is needed for that.
- **Every destination is distinct.** Track `01` titled `Intro` can occur on
  more than one disc. Keep enough album/disc/title information to avoid
  two inputs producing the same destination.
- **Tag values are usable.** Missing album or title fields can produce
  incomplete names. A valid expression is not proof that every source tag
  was filled correctly.
- **Existing files are accounted for.** A preview does not reserve a
  destination forever; a new collision or changed source can block applying.

Literal separators in the expression create folders. A slash or backslash
coming from a tag value is treated as part of a name and uses the configured
replacement, initially `_`. For example, an artist value `Kofi/Mira` becomes
the component `Kofi_Mira` under the default setting; it does not create two
artist folders. Other forbidden characters from values become underscores.

## 5. Apply the reviewed plan

When the final preview is correct, choose **Apply to … files**. The files
are renamed or moved immediately. There is no second **Save tags** step.
Inspect the two disc folders in the file manager and check the operation
result for errors.

If a file moved outside the folders currently shown in the file list,
**Keep moved files** determines whether it remains listed. Inspect the
destination before assuming a missing row means the file was deleted.
Creating the new layout does not promise to remove old empty folders or
move unrelated sidecar images and text files with the audio.

## 6. Practice recovery

Use **Edit → Undo** (++cmd+z++ / ++ctrl+z++) in the same desktop session
to reverse the recorded rename. Inspect the original paths afterward.
Undo can also encounter occupied destinations; keep the untouched copies
for recovery beyond that session.

Changing an expression or selection invalidates the preview. Source
changes or an expired plan can also require a new preview. Repeating a
relative folder format after moving the files may nest the layout again,
because the source files now have different containing folders.

## What you learned

- Tag → Filename reads tags and changes paths when applied.
- Literal separators create folders; separators inside values are replaced.
- Disc folders retain information that track numbers alone cannot express.
- Relative paths depend on the current source folder.
- A reviewed plan still needs successful file operations and inspection.
- Session undo helps reverse a recorded rename; copies provide longer-term recovery.

## Next steps

- [File and folder rules](../reference/format-strings.md#file-and-folder-names)
- [Zero-padded track numbers](../cookbook/rename-zero-padded-track-numbers.md)
- [Tags from folder and file names](../cookbook/tags-from-folder-and-filename.md)

<!-- verified: src/main.ts:converter, applyRenamed; src-tauri/src/lib.rs:filename_target; crates/tag-core/src/script.rs:evaluate_filename_with_context -->
