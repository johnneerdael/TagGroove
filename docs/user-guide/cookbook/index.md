# All recipes

Find a recipe by the result you want. Each recipe starts with a concrete
problem, gives the settings or expression to try, then explains how it works
and how to adapt it.

Every recipe also sits next to the feature it uses, so the
[Tagging](../tagging/index.md), [Rename & convert](../converters/index.md),
[Actions](../actions/index.md) and [Export & playlists](../export/index.md)
menus each list their own. This page lists them all by task.

Use copied files for the first run. Check the recipe's **Works on** and
**Undo** notes, save intended pending edits and start with a few selected
files. A correct-looking expression can still target the wrong field or
selection.

## Rename files and folders

These recipes build names from tags. Check the input tags before treating a
tidy-looking filename as evidence that an album's information is correct.

| Goal | Main tool |
| --- | --- |
| [Put zero-padded track numbers in filenames](../converters/recipes/rename-zero-padded-track-numbers.md) | Tag → Filename and `$num` |
| [Sort files into Artist/Album folders](../converters/recipes/rename-into-artist-album-folders.md) | Tag → Filename and folder components |
| [Organize multi-disc albums into CD1/CD2 folders](../converters/recipes/rename-multi-disc-albums.md) | Tag → Filename with disc numbers |
| [Name compilation albums consistently](../converters/recipes/rename-compilations.md) | Album Artist and Tag → Filename |
| [Keep filenames portable](../converters/recipes/rename-safe-characters.md) | Filename rules and deliberate replacements |
| [Make lower-case, dash-separated filenames](../converters/recipes/rename-lowercase-filenames.md) | Tag → Filename and text functions |

If placeholders such as `%artist%` are new, read
[Format strings from zero](../scripting/format-strings-from-zero.md) first.

## Fix and tidy tags

Choose the smallest rule that fits the problem. Keep meaningful credits and
version information unless the recipe's intended change is what you want.

| Goal | Main tool |
| --- | --- |
| [Move “feat.” artists out of the artist field](../actions/recipes/feat-artists-to-title.md) | Ordered Format Value/regex cleanup |
| [Fix capitalization while keeping exceptions](../actions/recipes/fix-capitalisation.md) | Case conversion and exception rules |
| [Strip track numbers from titles](../actions/recipes/strip-track-numbers-from-titles.md) | Anchored regular-expression replacement |
| [Remove double and trailing spaces](../actions/recipes/trim-spaces.md) | Text/regex cleanup |
| [Split `A; B` into separate artist values](../actions/recipes/split-multiple-artists.md) | Split Field by Separator |
| [Fill Album Artist from artist information](../converters/recipes/album-artist-from-artist.md) | Tag → Tag or Format Value |
| [Remove junk fields with a whitelist](../actions/recipes/remove-junk-fields.md) | Remove Fields Except |
| [Repair garbled characters](../actions/recipes/fix-mojibake.md) | Diagnosing encodings and codepage conversion |
| [Sort “The …” artist names under the next word](../converters/recipes/the-artist-sort-names.md) | Sort-field values |
| [Remove chosen bracketed suffixes](../actions/recipes/remove-text-in-brackets.md) | Carefully scoped regex replacement |

Learn [regular expressions](../scripting/regex-from-zero.md) before adapting a
pattern to text that differs from the recipe. To reuse several operations,
follow [Build your first action group](../actions/first-action-group.md).

## Import tags from text and names

The input's structure matters. A positional tracklist needs the correct file
order; a keyed import needs a key that identifies the intended files.

| Goal | Main tool |
| --- | --- |
| [Read tags from folder and file names](../converters/recipes/tags-from-folder-and-filename.md) | Filename → Tag |
| [Apply a tracklist from a text file](../converters/recipes/tracklist-from-text-file.md) | Text File → Tag, positional or keyed matching |
| [Split `Artist - Title` stored in the title](../actions/recipes/split-artist-title.md) | Guess Values |
| [Import lyrics from `.txt` files](../converters/recipes/lyrics-from-text-files.md) | Import Text File action |
| [Keep only the year from a full date](../actions/recipes/year-from-date.md) | A calculated field value |

For database imports, use the separate
[online-tagging lesson](../tag-sources/tag-from-online-sources.md) and
[tag-source guides](../tag-sources/index.md). An online edition match is a
different task from splitting known text.

## Track and disc numbers

Check the difference between track, total and disc fields before changing
numbering across several albums. See
[Track and disc numbers](../tagging/concepts/track-and-disc-numbers.md).

| Goal | Main tool |
| --- | --- |
| [Number tracks separately in each folder](../converters/recipes/number-tracks-per-folder.md) | Auto-numbering and folder grouping |
| [Split `3/12` into track 3 and total 12](../converters/recipes/split-track-total.md) | Parsing and calculated fields |
| [Read disc numbers from CD1/CD2 folders](../converters/recipes/disc-numbers-from-folders.md) | Folder placeholders and calculated fields |

## Cover art

An image beside a folder and an image embedded inside audio are separate
objects. Inspect which one a command reads and which one it changes.

| Goal | Main tool |
| --- | --- |
| [Embed each album's folder.jpg](../tagging/recipes/embed-folder-jpg.md) | Import Cover from File |
| [Save embedded covers as folder images](../tagging/recipes/export-folder-jpg.md) | Export Cover to File |
| [Shrink oversized embedded covers](../tagging/recipes/resize-covers.md) | Adjust Cover |
| [Keep only the front cover](../tagging/recipes/keep-only-front-cover.md) | Cover inspection and deliberate removal |

Changing a file extension does not convert image bytes. Follow the recipe's
format checks instead of assuming every exported cover is JPEG.

## Find, report and make playlists

Finding files is a useful first step before fixing them. Reports and playlists
are separate outputs; creating one does not rewrite the input tags.

| Goal | Main tool |
| --- | --- |
| [Find files without embedded covers](../tagging/recipes/find-missing-covers.md) | Filter and cover information |
| [Find low-bitrate files](../tagging/recipes/find-low-bitrate.md) | Filter and measured file properties |
| [Find missing album artist or year](../tagging/recipes/find-missing-fields.md) | Field-presence filters |
| [Create one playlist per album folder](../export/recipes/playlist-per-album.md) | Playlist generation |
| [Make an HTML album catalogue with a folder image](../export/recipes/html-album-catalogue.md) | Export template plus a separate image |
| [Open your library information in a spreadsheet](../export/recipes/csv-for-spreadsheets.md) | CSV export and text import settings |

## When a recipe does not quite fit

Compare the example's input with your own before changing the expression.
Differences in field names, multiple values, missing tags, punctuation, file
order or folder layout can change the result. Test a matching and a
nonmatching copied file, then inspect the complete output and any warnings.

For syntax, follow the recipe's reference links. For a sequence you will run
regularly, first make each step work on its own, then build an action group
in the intended order. Repeating a failed whole group can apply earlier
successful steps twice.

## Related

- [All tutorials: learn the tools from the beginning](../learn/index.md)
- [Choose the right tool](../getting-started/choose-a-tool.md)
- [Format-string reference](../scripting/format-strings.md)
- [Regular-expression reference](../scripting/regular-expressions.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)

<!-- Candidate evidence: all 34 recipe paths and their existing content, checked against planned-nav.yml. This is a routing page, not a new execution receipt for those recipes. -->
