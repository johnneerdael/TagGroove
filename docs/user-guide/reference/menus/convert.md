# Convert menu

Converters derive new tag values or filenames from existing information.
Select the intended files and save pending edits first. The menu has five
conversion directions plus track/disc numbering; choose the direction by
what you already have and what you want to create.

| Item | Shortcut | Use it when |
| --- | --- | --- |
| [**Tag → Filename…**](../../converters/tag-to-filename.md) | Alt+1 | Tags contain the artist, album or title needed for a new name/folder layout |
| [**Filename → Tag…**](../../converters/filename-to-tag.md) | Alt+2 | File/folder text contains the metadata you want to capture |
| [**Filename → Filename…**](../../converters/filename-to-filename.md) | Alt+3 | Parts of the old name need rearranging, optionally with scripting functions |
| [**Text File → Tag…**](../../converters/text-file-to-tag.md) | Alt+4 | An external text table supplies tag values |
| [**Tag → Tag…**](../../converters/tag-to-tag.md) | Alt+5 | One field should be copied, combined or transformed from others |
| [**Auto-numbering…**](../../converters/auto-numbering.md) | Primary+K | Selected files need track/disc counters and optional totals |

Primary is Ctrl on Windows/Linux and Command on macOS. Option+digit works
outside text entry on a Mac; while typing use Control+Option+digit so Option
can continue entering characters normally.

## Read the direction before applying

For `TITLE = Paper Boats` and `TRACK = 3/8`, a Tag → Filename pattern
`$num(%track%,2) - %title%` produces the stem `03 - Paper Boats`, while keeping
the audio extension. It does not change `TITLE`. Filename → Tag performs the
opposite kind of task: `%track% - %title%` can capture `03 - Paper Boats.flac`
into fields. Those are different languages at the input boundary: a capture
pattern names pieces to read, while a format string calculates output.

Tag → Tag can calculate `%artist% - %title%` into `COMMENT`, for example,
without changing its source title. Text import can match rows by position or
a filename key; inspect that matching mode before applying a table to several
files. The linked converter pages give complete worked examples.

## Preview and write behavior

Use the dialog's preview before its Apply/rename step. In the current shortcut
router, shifted Alt+digit chords do not invoke these five converters; use the
explicit dialog controls to inspect the proposed result.

Filename changes and a converter's Apply operation can write immediately.
Auto-numbering instead stages its proposed TRACK/DISCNUMBER edits as pending
changes, then **Save tags** writes them. Primary+Shift+K opens numbering with
per-folder reset requested; it does not immediately number or save the files.
Do not infer the write step from the fact that two commands share this menu.

If a preview skips a nonmatching filename or reports an invalid expression,
correct the input or pattern before applying. A successful calculation is
not proof that every output path is writable or distinct. Check collisions,
missing fields and the affected-file count, then inspect reported errors.
