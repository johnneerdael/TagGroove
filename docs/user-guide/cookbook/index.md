# Cookbook

Find a recipe by the result you want. Each recipe starts with a concrete
problem, gives the settings or expression to try, then explains how it works
and how to adapt it.

Use copied files for the first run. Check the recipe's **Works on** and
**Undo** notes, save intended pending edits and start with a few selected
files. A correct-looking expression can still target the wrong field or
selection.

## Rename files and folders

These recipes build names from tags. Check the input tags before treating a
tidy-looking filename as evidence that an album's information is correct.

| Goal | Main tool |
| --- | --- |
| [Put zero-padded track numbers in filenames](rename-zero-padded-track-numbers.md) | Tag → Filename and `$num` |
| [Sort files into Artist/Album folders](rename-into-artist-album-folders.md) | Tag → Filename and folder components |
| [Organize multi-disc albums into CD1/CD2 folders](rename-multi-disc-albums.md) | Tag → Filename with disc numbers |
| [Name compilation albums consistently](rename-compilations.md) | Album Artist and Tag → Filename |
| [Keep filenames portable](rename-safe-characters.md) | Filename rules and deliberate replacements |
| [Make lower-case, dash-separated filenames](rename-lowercase-filenames.md) | Tag → Filename and text functions |

If placeholders such as `%artist%` are new, read
[Format strings from zero](../learn/format-strings-from-zero.md) first.

## Fix and tidy tags

Choose the smallest rule that fits the problem. Keep meaningful credits and
version information unless the recipe's intended change is what you want.

| Goal | Main tool |
| --- | --- |
| [Move “feat.” artists out of the artist field](feat-artists-to-title.md) | Ordered Format Value/regex cleanup |
| [Fix capitalization while keeping exceptions](fix-capitalisation.md) | Case conversion and exception rules |
| [Strip track numbers from titles](strip-track-numbers-from-titles.md) | Anchored regular-expression replacement |
| [Remove double and trailing spaces](trim-spaces.md) | Text/regex cleanup |
| [Split `A; B` into separate artist values](split-multiple-artists.md) | Split Field by Separator |
| [Fill Album Artist from artist information](album-artist-from-artist.md) | Tag → Tag or Format Value |
| [Remove junk fields with a whitelist](remove-junk-fields.md) | Remove Fields Except |
| [Repair garbled characters](fix-mojibake.md) | Diagnosing encodings and codepage conversion |
| [Sort “The …” artist names under the next word](the-artist-sort-names.md) | Sort-field values |
| [Remove chosen bracketed suffixes](remove-text-in-brackets.md) | Carefully scoped regex replacement |

Learn [regular expressions](../learn/regex-from-zero.md) before adapting a
pattern to text that differs from the recipe. To reuse several operations,
follow [Build your first action group](../learn/first-action-group.md).

## Import tags from text and names

The input's structure matters. A positional tracklist needs the correct file
order; a keyed import needs a key that identifies the intended files.

| Goal | Main tool |
| --- | --- |
| [Read tags from folder and file names](tags-from-folder-and-filename.md) | Filename → Tag |
| [Apply a tracklist from a text file](tracklist-from-text-file.md) | Text File → Tag, positional or keyed matching |
| [Split `Artist - Title` stored in the title](split-artist-title.md) | Guess Values |
| [Import lyrics from `.txt` files](lyrics-from-text-files.md) | Import Text File action |
| [Keep only the year from a full date](year-from-date.md) | A calculated field value |

For database imports, use the separate
[online-tagging lesson](../learn/tag-from-online-sources.md) and
[tag-source guides](../tag-sources/index.md). An online edition match is a
different task from splitting known text.

## Track and disc numbers

Check the difference between track, total and disc fields before changing
numbering across several albums. See
[Track and disc numbers](../concepts/track-and-disc-numbers.md).

| Goal | Main tool |
| --- | --- |
| [Number tracks separately in each folder](number-tracks-per-folder.md) | Auto-numbering and folder grouping |
| [Split `3/12` into track 3 and total 12](split-track-total.md) | Parsing and calculated fields |
| [Read disc numbers from CD1/CD2 folders](disc-numbers-from-folders.md) | Folder placeholders and calculated fields |

## Cover art

An image beside a folder and an image embedded inside audio are separate
objects. Inspect which one a command reads and which one it changes.

| Goal | Main tool |
| --- | --- |
| [Embed each album's folder.jpg](embed-folder-jpg.md) | Import Cover from File |
| [Save embedded covers as folder images](export-folder-jpg.md) | Export Cover to File |
| [Shrink oversized embedded covers](resize-covers.md) | Adjust Cover |
| [Keep only the front cover](keep-only-front-cover.md) | Cover inspection and deliberate removal |

Changing a file extension does not convert image bytes. Follow the recipe's
format checks instead of assuming every exported cover is JPEG.

## Find, report and make playlists

Finding files is a useful first step before fixing them. Reports and playlists
are separate outputs; creating one does not rewrite the input tags.

| Goal | Main tool |
| --- | --- |
| [Find files without embedded covers](find-missing-covers.md) | Filter and cover information |
| [Find low-bitrate files](find-low-bitrate.md) | Filter and measured file properties |
| [Find missing album artist or year](find-missing-fields.md) | Field-presence filters |
| [Create one playlist per album folder](playlist-per-album.md) | Playlist generation |
| [Make an HTML album catalogue with a folder image](html-album-catalogue.md) | Export template plus a separate image |
| [Open your library information in a spreadsheet](csv-for-spreadsheets.md) | CSV export and text import settings |

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

- [Learn the tools from the beginning](../learn/index.md)
- [Choose an automation tool](../automate/index.md)
- [Format-string reference](../reference/format-strings.md)
- [Regular-expression reference](../reference/regular-expressions.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)

<!-- Candidate evidence: all 34 recipe paths and their existing content, checked against planned-nav.yml. This is a routing page, not a new execution receipt for those recipes. -->
