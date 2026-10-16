# Start using OpenTag

OpenTag changes the information stored inside audio files: titles, artists,
albums, track numbers and embedded artwork. It can also rename files from
those values, import information from names, and repeat a sequence of changes.
A renamed file and a changed song title are different results; choose the
operation that matches the problem you want to solve.

This starting path is for a first desktop session. Use a copied album with a
few ordinary audio files. Keep the originals elsewhere until you have checked
the saved result in OpenTag and in the player you use.

## 1. Install a package for your computer

Follow [Download and install](install.md). Pick the operating system and
processor architecture on the release page. The source-code archive is for
building OpenTag; it is not an installer. Mobile previews have different
access and save-back limits, so do not use a desktop walkthrough as a promise
of the same result on a phone.

After opening the application, check **Help → About OpenTag** when reporting
a problem. Keep its version and your operating system with your notes.

## 2. Open only your practice folder

Choose **File → Change directory…** and select the copied album. This replaces
the loaded list; **Add directory…** and **Add files…** add to it instead.
Read [Opening files](opening-files.md) before mixing several folders or
turning on startup restoration.

Loading reads metadata and file properties. It does not, by itself, rewrite
the files. The list is a working view, not a separate music-library copy.
Check the loaded count and remove an active filter if fewer rows appear
than expected. [The main window](main-window.md) explains where these counts
and controls live.

## 3. Make one recognizable pending edit

1. Select **one** practice file. Check the selected count beside the filter.
2. In the tag panel, change **Album** to `Harbour Lights — practice`.
3. Look for its pending marker and the **unsaved files** count. The changed
   text is visible, but an ordinary tag-panel edit still needs saving.
4. Choose **File → Save tags** (++cmd+s++ / ++ctrl+s++).
5. Read the status and any file errors. Choose **File → Reload tags** to read
   the saved result again. If the album returns to the old value, investigate
   the error rather than repeating a larger batch.

For this exercise, check **Options → Tags → Save tags when using arrow keys
or a single mouse click**. It is off by default. When enabled, a selection
change can write pending edits, so moving to another row is no longer a
reliable way to leave the file untouched.

A displayed `< keep >` means preserve each selected file's current value.
Clearing an editable field requests removal. It does not mean “leave it
alone.” Read [Editing, saving and undo](editing-and-saving.md) before editing
several files together.

## 4. Choose a task and its write timing

| Your goal | Use | When the result is written |
| --- | --- | --- |
| Give several tracks the same album | [Tag panel](../guides/tag-panel.md) | Review pending edits, then **Save tags** |
| Inspect fields outside the panel | [Extended tags](../guides/extended-tags.md) | **Keep changes** stages edits; save afterward |
| Fill fields from folder and file names | [Filename → Tag](../converters/filename-to-tag.md) | Applying the converter writes its result |
| Give filenames consistent track/title text | [Tag → Filename](../converters/tag-to-filename.md) | Applying the rename changes paths |
| Calculate a field from other fields | [Tag → Tag](../converters/tag-to-tag.md) | Applying the converter writes its result |
| Repeat cleanup in a defined order | [Action groups](../learn/first-action-group.md) | Running the group writes its changes |
| Create a report for a spreadsheet | [CSV export](../export/csv.md) | Write an external report; it is not a tag backup |

The same panel can edit files from different containers, but native storage
and supported writes differ. Check [tag formats](../concepts/tag-formats.md)
when a player disagrees with the editor. Extended tags also allows custom
field names; another player need not display or understand those fields. For
online lookup commands, start with the [Tag Sources menu](../reference/menus/tag-sources.md)
and review the retrieved release and track matches before applying anything.

An ordinary tag-panel text box does not calculate scripting functions. Typing
`$upper(%artist%)` there stores that text. The format-string lesson shows
where expressions are evaluated and why quoted text and optional sections
matter.

## 5. Recover deliberately

Before saving, use **Edit → Discard pending changes** on the intended
selection to abandon its unsaved edits. After a recorded write, inspect the
operation named by **Edit → Undo …**. History belongs to the current session;
it is not a permanent backup and does not reverse every external output.
A batch can partly succeed, so check individual failures before closing.

Replacing the list or closing can discard pending work. Prompts are
configurable under **Options → Messages**; turning one off does not save the
edits. Save or discard deliberately before changing folders or loading a
configuration. If another program changed a file, reload it before deciding
which values to keep.

## Next steps

- [Your first tagging session](../learn/first-tagging-session.md) walks through
  a complete practice album.
- [What tags are](../concepts/what-are-tags.md) separates stored fields,
  filenames and player display.
- [Format strings from zero](../learn/format-strings-from-zero.md) builds
  expressions from worked inputs and outputs.
- [Regular expressions from zero](../learn/regex-from-zero.md) introduces
  matching and captures before larger cleanup recipes.
