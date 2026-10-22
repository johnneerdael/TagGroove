# Choose the right tool

TagGroove has several ways to change many files at once. Choose a tool based
on where the information comes from and what should change: tags, names,
artwork or a separate report. Use automation when the same rule describes
several files.

Start with a copied album. Save intended pending edits, inspect the selected
files and check the result of one operation before applying it more widely.
The tools below do not all wait for **Save tags**.

## Choose by the result you need

| Starting point → desired result | Tool | Example |
| --- | --- | --- |
| Existing tags → new file/folder names | [Tag → Filename](../converters/tag-to-filename.md) | Turn track `3` and title `Paper Boats` into a name beginning `03 - Paper Boats`. |
| Existing file/folder names → tags | [Filename → Tag](../converters/filename-to-tag.md) | Read artist, album and title from a consistent folder/name structure. |
| Existing filename → rearranged filename | [Filename → Filename](../converters/filename-to-filename.md) | Reorder known filename parts without using saved tags as the source. |
| A tracklist or data file → tags | [Text File → Tag](../converters/text-file-to-tag.md) | Match imported rows to selected audio files. |
| Existing tag values → calculated tag values | [Tag → Tag](../converters/tag-to-tag.md) | Copy or compose a value using placeholders and functions. |
| Current file order/grouping → track/disc numbers | [Auto-numbering](../converters/auto-numbering.md) | Restart track numbers for each album folder. |
| One or more cleanup operations → repeatable changes | [Actions](../actions/index.md) | Trim spaces, then apply a chosen case rule. |
| Online/local catalog → reviewed metadata or artwork | [Tag sources](../tag-sources/index.md) | Choose the right edition and match its tracks to your files. |
| Several online catalogs → the best value for each field | [Compare metadata](../tag-sources/compare.md) | Pick a provider, keep the file's value or remove the field, field by field. |
| A few common fields on selected files → shared values | [The tag panel](../tagging/tag-panel.md) | Give an album's tracks the same album and year. |
| Selected files → separate text report | [Export](../export/index.md) | Create a CSV list or HTML catalogue. |
| Selected files → playback list or cue sheet | [Playlists](../export/playlists.md) | Create a playlist for an album folder. |

A converter is a focused operation. An action group is useful when several
operations belong together or when the same settings will be reused.
Keep a one-off task as one operation until you know it needs a reusable group.

## When each tool writes

| Your goal | Use | When the result is written |
| --- | --- | --- |
| Give several tracks the same album | [Tag panel](../tagging/tag-panel.md) | Review pending edits, then **Save tags** |
| Inspect fields outside the panel | [Extended tags](../tagging/extended-tags.md) | **Keep changes** stages edits; save afterward |
| Fill fields from folder and file names | [Filename → Tag](../converters/filename-to-tag.md) | Applying the converter writes its result |
| Give filenames consistent track/title text | [Tag → Filename](../converters/tag-to-filename.md) | Applying the rename changes paths |
| Calculate a field from other fields | [Tag → Tag](../converters/tag-to-tag.md) | Applying the converter writes its result |
| Repeat cleanup in a defined order | [Action groups](../actions/first-action-group.md) | Running the group writes its changes |
| Look up or compare online metadata | [Tag sources](../tag-sources/index.md) | Saving in the review dialog writes immediately |
| Create a report for a spreadsheet | [CSV export](../export/csv.md) | Write an external report; it is not a tag backup |

The same panel can edit files from different containers, but native storage
and supported writes differ. Check [tag formats](../tagging/concepts/tag-formats.md)
when a player disagrees with the editor. Extended tags also allows custom
field names; another player need not display or understand those fields. For
online lookup commands, start with the [Tag Sources menu](../reference/menus/tag-sources.md)
and review the retrieved release and track matches before applying anything.

## Read a rule before running it

You will meet three different kinds of instruction:

| Kind | What it describes | Small example |
| --- | --- | --- |
| Format string | How to build text from fields and functions | `$num(%track%,2) - %title%` builds a padded track number and title. |
| Regular expression | Which text pattern to match | `^[0-9]+` matches digits at the start of text. |
| Action settings | Which field/operation to apply, and with which inputs | A Replace action names a field, search text and replacement. |

A placeholder such as `%title%` reads a value. A scripting function such as
`$num` calculates text. Neither chooses what to overwrite on its own: the
converter, action or export context supplies that purpose.

The same punctuation can have different rules in a format string and a regex.
Learn the [format-string basics](../scripting/format-strings-from-zero.md), then
the [regex basics](../scripting/regex-from-zero.md). Use the
[regex reference](../scripting/regular-expressions.md) for context-specific
quoting, case behavior and replacement syntax.

## Keep a batch predictable

1. Pick one goal: for example, remove accidental trailing spaces from titles.
2. Choose a small copied selection that includes both a matching and a
   nonmatching file.
3. Read the command's write timing. A preview is an inspection step, not a
   backup; applying an operation can write immediately.
4. Compare before and after values and inspect any warnings. Check the
   available Undo operation when the command records one.
5. Add the next operation only after the first result is right. In an action
   group, later actions see the results of earlier ones.

An action group is not an all-or-nothing transaction. A warning or failure
does not establish that earlier writes were rolled back. Read the result
before repeating the group, especially when it writes images, reports or
playlists as well as audio metadata.

## Know what remains separate

Changing a title does not rename the audio file. Exporting a CSV does not
write spreadsheet changes back to tags. Writing an HTML image reference
does not create the referenced image. Downloading artwork and saving a
separate folder image are different choices from embedding it in audio.

These workflows primarily describe local desktop files. Provider documents,
chapter rows and mobile builds have their own limits. Check the relevant
[platform notes](install.md#understand-the-mobile-downloads)
and the tool's guide before using a path-based batch workflow.

## Related

- [Build your first action group](../actions/first-action-group.md)
- [Converters overview](../converters/index.md)
- [Actions and action groups](../actions/index.md)
- [Tag sources](../tag-sources/index.md)
- [All recipes](../cookbook/index.md)
- [Scripting](../scripting/index.md)

<!-- Candidate evidence: planned-nav.yml; existing converters/actions/source/export guides; src/action-ui.ts and src/main.ts operation boundaries; regex syntax reference. The two short expressions are explained existing syntax, not newly executed batch examples. -->
