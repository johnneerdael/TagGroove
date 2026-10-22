# Start using TagGroove

TagGroove changes the information stored inside audio files: titles, artists,
albums, track numbers and embedded artwork. It can also look that information
up online, rename files from it, fill it in from names, and repeat a sequence
of changes. A renamed file and a changed song title are different results;
choose the operation that matches the problem you want to solve.

This page gets you from installation to a first saved edit, then points to the
feature that fits your next job. Use a copied album with a few ordinary audio
files, and keep the originals elsewhere until you have checked the saved
result in TagGroove and in the player you use.

## What you can do

<div class="grid cards" markdown>

-   :material-tag-edit-outline:{ .lg .middle } **Tagging**

    ---

    Edit titles, artists, albums, numbers and cover art in the file list and
    tag panel, one file or a whole selection.

    [:octicons-arrow-right-24: Tagging](../tagging/index.md)

-   :material-cloud-search-outline:{ .lg .middle } **Tag sources**

    ---

    Look up an album online, compare several providers field by field, then
    save the result you chose.

    [:octicons-arrow-right-24: Tag sources](../tag-sources/index.md)

-   :material-file-swap-outline:{ .lg .middle } **Rename & convert**

    ---

    Rename files from tags, fill tags from names, number tracks and discs.

    [:octicons-arrow-right-24: Rename & convert](../converters/index.md)

-   :material-playlist-check:{ .lg .middle } **Actions**

    ---

    Save a cleanup as an action group and run it on any selection.

    [:octicons-arrow-right-24: Actions](../actions/index.md)

-   :material-code-braces:{ .lg .middle } **Scripting**

    ---

    Format strings, functions and regular expressions for rules that describe
    many files at once.

    [:octicons-arrow-right-24: Scripting](../scripting/index.md)

-   :material-export-variant:{ .lg .middle } **Export & playlists**

    ---

    Write CSV and HTML reports, playlists and cue sheets.

    [:octicons-arrow-right-24: Export & playlists](../export/index.md)

</div>

Not sure which one you need? [Choose the right tool](choose-a-tool.md) matches
a starting point and a goal to a feature, and says when each one writes.
Make TagGroove fit your routine in [Customize](../customize/index.md).

## Your first hour

1. **Install** a package for your computer: [Download and install](install.md).
   Pick the operating system and processor architecture on the release page.
   The source-code archive is for building TagGroove; it is not an installer.
   Mobile previews have different access and save-back limits, so do not use
   a desktop walkthrough as a promise of the same result on a phone. After
   opening the application, check **Help → About TagGroove** when reporting a
   problem, and keep its version and your operating system with your notes.
2. **Open only your practice folder** with **File → Change directory…**. This
   replaces the loaded list; **Add directory…** and **Add files…** add to it
   instead. Loading reads metadata and file properties; it does not rewrite
   the files. [Opening files](opening-files.md) covers mixing folders and
   startup restoration, and [the main window](main-window.md) shows where the
   loaded count and the filter live. Check the count, and remove an active
   filter if fewer rows appear than expected.
3. **Make one recognizable edit.** Select **one** practice file, change
   **Album** in the tag panel to `Harbour Lights — practice`, then choose
   **File → Save tags** (++cmd+s++ / ++ctrl+s++). An ordinary tag-panel edit is
   *pending* until you save; the changed text and the **unsaved files** count
   show that. Choose **File → Reload tags** to read the saved value back. If the
   album returns to its old value, investigate the error rather than repeating
   a larger batch. The full walkthrough is [Your first tagging session](first-tagging-session.md).
4. **Pick your next job** from the cards above.

## Three things that surprise new users

- **`< keep >` means “leave each file's own value alone.”** Clearing an editable
  field requests removal; it does not mean “leave it alone.” Read
  [Editing, saving and undo](editing-and-saving.md) before editing several
  files together.
- **Moving between rows can save.** **Options → Tags → Save tags when using
  arrow keys or a single mouse click** is off by default. When it is on, a
  selection change can write pending edits, so moving to another row is no
  longer a reliable way to leave the file untouched.
- **A tag-panel text box does not calculate.** Typing `$upper(%artist%)` there
  stores that text. Expressions are evaluated only in converters, actions,
  columns and exports; [Scripting](../scripting/index.md) shows where.

## Recover deliberately

Before saving, use **Edit → Discard pending changes** on the intended selection
to abandon its unsaved edits. After a recorded write, inspect the operation
named by **Edit → Undo …**. History belongs to the current session; it is not
a permanent backup and does not reverse every external output. A batch can
partly succeed, so check individual failures before closing.

Replacing the list or closing can discard pending work. Prompts are
configurable under **Options → Messages**; turning one off does not save the
edits. Save or discard deliberately before changing folders or loading a
configuration. If another program changed a file, reload it before deciding
which values to keep.

## Next steps

- [Your first tagging session](first-tagging-session.md) walks through a
  complete practice album.
- [What tags are](../tagging/concepts/what-are-tags.md) separates stored fields,
  filenames and player display.
- [All tutorials](../learn/index.md) and [all recipes](../cookbook/index.md)
  list every lesson and task recipe by feature.
- [Coming from Mp3tag](coming-from-mp3tag.md) explains what carries across.
- [Troubleshooting](../help/troubleshooting.md) and the
  [FAQ](../help/faq.md) cover the usual first-day problems.
