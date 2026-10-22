# Your first tagging session

<p class="lead">In about fifteen minutes you'll open an album, fix tags on one file and then on all of them at once, look inside a file's complete tag, and rename every file from its tags. You'll also undo a change, so you can recover a change during the session. Keep your original files as a backup.</p>

**You need:**

- [ ] TagGroove installed ([download a desktop package](https://github.com/johnneerdael/TagGroove/releases/latest))
- [ ] A folder with a few music files, ideally one album

!!! tip "Practise on a copy"
    TagGroove changes your files when you save. Copy an album folder to your
    desktop and use the copy for this tutorial. The screenshots use an
    invented album, *Harbour Lights* by *Juniper Vale*. Your own music
    will look different, and the steps are the same.

## 1. Open the album

Choose **File → Change directory…** (++cmd+d++ / ++ctrl+d++) and pick the
album folder. You can also drag the folder from Finder, Explorer or your file
manager onto the TagGroove window, or click **Open folder** in the toolbar.

<figure markdown>
![TagGroove with the album Harbour Lights open: the tag panel on the left shows the first track's tags, the file list on the right lists eight files](../assets/screens/main-window-light.png#only-light)
![TagGroove with the album Harbour Lights open: the tag panel on the left shows the first track's tags, the file list on the right lists eight files](../assets/screens/main-window-dark.png#only-dark)
<figcaption>The main window with the album open and the first file selected.</figcaption>
</figure>

The window has two main parts:

- The **file list** on the right has one row per file. Its columns show the
  file name and the most important tags. Click a column header to sort by it.
- The **tag panel** on the left shows the tags of the file you selected.
  This is where you edit.

The bar at the bottom of the file list describes the selection: how many
files, their playing time and size, and the audio format of the selected
file.

??? question "Don't see subfolders?"
    By default TagGroove also lists the files in subfolders, so a box set with
    `CD1` and `CD2` folders appears as one list. You can change this under
    **File → Options… → Directories → Read subfolders**.

## 2. Look at one file's tags

Click the first file in the list. The tag panel fills with its **fields**:
Title, Artist, Album, Year, Track, Genre and more. If the file has cover art,
it appears at the bottom of the panel with its type, size and format.

Each field has a ▾ button on the right. It opens a list of values you can
pick from. For **Genre**, that includes the standard genre names.

## 3. Edit a field and save

Let's add a comment to this file.

1. Click in the **Comment** field and type `First tagging session`.
2. Look at the file list: the changed value is shown in the accent colour, and
   the status bar at the bottom of the window says **1 unsaved file**. Your
   edit is *pending*: TagGroove remembers it, but the file on disk is unchanged.
3. Press ++cmd+s++ / ++ctrl+s++, or click **Save tags** under the tag panel.

The status bar now reports **Saved tags: 1 file**. The comment is written into
the file, and any music player will see it.

!!! note "Why edits wait for you to save"
    Pending edits let you change many fields, check them in the file list,
    and then write everything in one go. Until you save, you can throw edits
    away with **Edit → Discard pending changes** (++cmd+b++ / ++ctrl+b++).
    Save writes the pending edits of the **selected** files. If you edited
    several files, select them all before you save. When you try to close
    TagGroove or open another folder with edits still pending, it asks what
    to do.

## 4. Edit all files at once

Apply a shared value to every selected file. Suppose the whole album should
have the genre *Folk* instead of *Indie Folk*.

1. Click any file in the list, then press ++cmd+a++ / ++ctrl+a++ to select
   all files.
2. Look at the tag panel. Fields that are the same in every selected file
   show that value: Artist, Album, Year, Genre. Fields that differ, like
   Title and Track, show **`< keep >`**.

<figure markdown>
![The tag panel with all eight files selected: Title and Track show < keep >, Artist, Album, Year and Genre show the shared values](../assets/screens/tag-panel-keep-light.png#only-light)
![The tag panel with all eight files selected: Title and Track show < keep >, Artist, Album, Year and Genre show the shared values](../assets/screens/tag-panel-keep-dark.png#only-dark)
<figcaption>With several files selected, fields that differ show <code>&lt; keep &gt;</code>. Leaving them alone keeps each file's own value.</figcaption>
</figure>

3. Select the text in the **Genre** field, type `Folk` and press
   ++cmd+s++ / ++ctrl+s++.

All eight files now have the genre *Folk*, and every title and track number is
untouched. `< keep >` shows the rule of the tag panel: a field you don't touch is
never changed.

??? question "How do I empty a field in every selected file?"
    Typing nothing leaves the field at `< keep >`. To clear it, open the
    field's ▾ list and choose **`< blank >`**, then save.

## 5. See everything in a file's tag

The tag panel shows the common fields only. A file can hold many more:
composer, lyrics, sort names, identifiers from online databases and fields
that other programs invented.

Select one file and press ++alt+t++ (Alt is ⌥ Option on a Mac), or click
**Extended…** under the tag panel.

<figure markdown>
![The Extended tags dialog for one file: a list of field names with their values, and the cover art on the right](../assets/screens/extended-tags-light.png#only-light)
![The Extended tags dialog for one file: a list of field names with their values, and the cover art on the right](../assets/screens/extended-tags-dark.png#only-dark)
<figcaption>Extended tags lists every field of the file. The title bar shows the tag type, here ID3v2.3.</figcaption>
</figure>

Here you can edit any field, give a field several values with **＋ Add
value**, and add fields of your own with **＋ Add field**. Close the dialog
with **Cancel** for now.

## 6. Rename the files from their tags

Look at the file names in the list: `01_low_tide.mp3`, `track04.mp3`,
`05 - juniper vale - salt & cedar.mp3`. Every file is named differently. The tags
are tidy, though, so let TagGroove build the names from them.

1. Select all files (++cmd+a++ / ++ctrl+a++).
2. Open the **Convert** menu and choose **Tag → Filename…** (++alt+1++).

    <figure markdown>
    ![The Convert menu open, listing Tag → Filename, Filename → Tag, Filename → Filename, Text File → Tag, Tag → Tag and Auto-numbering](../assets/screens/menu-convert-light.png#only-light)
    ![The Convert menu open, listing Tag → Filename, Filename → Tag, Filename → Filename, Text File → Tag, Tag → Tag and Auto-numbering](../assets/screens/menu-convert-dark.png#only-dark)
    </figure>

3. In **Format string**, type:

    ```text
    $num(%track%,2) - %artist% - %title%
    ```

    `%track%`, `%artist%` and `%title%` are **placeholders**: TagGroove replaces
    each with that field's value. `$num(…,2)` writes the track number with
    two digits, so `4/8` becomes `04`.

4. Click **Preview**. Each file shows its current name and, below it, its
   new one.

    <figure markdown>
    ![The Tag → Filename dialog with the format string and a preview: 01_low_tide.mp3 becomes 01 - Juniper Vale - Low Tide.mp3](../assets/screens/tag-to-filename-light.png#only-light)
    ![The Tag → Filename dialog with the format string and a preview: 01_low_tide.mp3 becomes 01 - Juniper Vale - Low Tide.mp3](../assets/screens/tag-to-filename-dark.png#only-dark)
    <figcaption>Preview shows every new name before anything changes. The extension is kept.</figcaption>
    </figure>

5. Check the list, then click **Apply to 8 files**.

The files are renamed immediately. Renaming changes no tags, so there's
nothing to save.

## 7. Undo, and change your mind

Suppose you'd rather have names without the artist. Choose **Edit → Undo**
(++cmd+z++ / ++ctrl+z++). The menu item names the step it undoes, and the
files get their old names back. Press ++cmd+y++ / ++ctrl+y++ to redo it.

Undo can restore saved tags and renames recorded in this session. Check
the menu label before undoing, and keep the practice copy until you have
verified your results. You can also undo the genre change from step 4.

## What you learned

- The **file list** shows your files; the **tag panel** edits the selected ones.
- Edits are **pending** until you save them with ++cmd+s++ / ++ctrl+s++.
- With several files selected, **`< keep >`** protects the values that differ,
  and **`< blank >`** clears a field.
- **Extended tags** (++alt+t++) shows every field in a file.
- **Tag → Filename** builds file names from tags with a **format string**.
  Always preview first.
- **Undo** (++cmd+z++ / ++ctrl+z++) reverses saves and renames.

## Next steps

- [Zero-padded track numbers](../converters/recipes/rename-zero-padded-track-numbers.md):
  adapt the naming pattern you just used.
- [Tags from folder and file names](../converters/recipes/tags-from-folder-and-filename.md):
  fill tags from names when working in the opposite direction.
- [Format strings](../scripting/format-strings.md): look up optional parts,
  quoting and conditions for more complex names.
