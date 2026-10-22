# Tagging

Tagging is the heart of TagGroove: reading and changing the information stored
inside your audio files, such as titles, artists, albums, track numbers, genres
and cover art. This section covers everything you do by hand with the file
list and the tag panel, plus the background that explains why a player can show
something different from the editor.

If selecting files or saving edits is unfamiliar, complete
[Your first tagging session](../getting-started/first-tagging-session.md)
first. To fill tags from the internet, see [Tag sources](../tag-sources/index.md);
to fill them from names, see [Rename & convert](../converters/index.md); to
repeat a cleanup on many files, see [Actions](../actions/index.md).

## Edit tags

| Your goal | Guide |
| --- | --- |
| Edit common fields across one file or a selection | [The tag panel](tag-panel.md) |
| Inspect less common fields, custom fields or separate values | [Extended tags](extended-tags.md) |
| Transfer metadata between files | [Cut, copy and paste tags](copy-paste-tags.md) |
| Deliberately remove tags or tag types | [Removing tags](remove-tags.md) |
| Understand chapter and cue-sheet rows | [Chapters and cue sheets as rows](chapters.md) |
| Use format-specific cleanup utilities | [Built-in utilities](utilities.md) |

Typing in the tag panel produces pending edits. Nothing is written until you
save, so read [Editing, saving and undo](../getting-started/editing-and-saving.md)
before editing several files at once.

## Work with cover art

| Your goal | Guide |
| --- | --- |
| Add, inspect, replace or remove artwork | [Working with covers](covers.md) |
| Give a whole collection a front cover | [Give every album a cover](give-every-album-a-cover.md) |
| Save embedded covers as `folder.jpg` | [Save embedded covers as folder images](recipes/export-folder-jpg.md) |
| Embed the image already in each folder | [Embed each folder image](recipes/embed-folder-jpg.md) |
| Shrink oversized pictures | [Shrink oversized covers](recipes/resize-covers.md) |
| Keep only each file's front cover | [Keep each file’s front covers](recipes/keep-only-front-cover.md) |
| Understand why another player shows no cover | [Cover art](concepts/cover-art.md) |

## Browse, filter and find

| Your goal | Guide |
| --- | --- |
| Select the right rows, sort them or edit a cell | [The file list](file-list.md) |
| Keep only rows that satisfy a condition in view | [Filtering the file list](filter.md) |
| Find occurrences and navigate results | [Find](find.md) |
| Reuse a cached view of configured music folders | [Library cache](library.md) |
| Find files with missing fields, covers or low bitrate | [Missing fields](recipes/find-missing-fields.md), [missing covers](recipes/find-missing-covers.md), [low bitrates](recipes/find-low-bitrate.md) |

A filter changes which rows you see; it does not delete the other files. Check
the actual selection before editing or applying a batch operation. Hiding a
field or column similarly does not remove its stored metadata. The Library is
a way to browse known files, not an independent music backup.

## Files and history

| Your goal | Guide |
| --- | --- |
| Copy, move, send to trash or permanently delete audio files | [Copy, move and delete files](file-operations.md) |
| Keep associated lyric or karaoke files with supported file operations | [Associated lyrics and karaoke](associated-files.md) |
| Inspect and replay a recorded operation | [Undo and history](undo-history.md) |

A recorded desktop Undo step belongs to the current session; keep normal
backups for recovery beyond it.

## Understand your tags

These pages answer the questions that can make a correct operation look wrong:

| Question | Explanation |
| --- | --- |
| Why do the filename, tag title and player display disagree? | [What tags are](concepts/what-are-tags.md) |
| Why can one file have several kinds of tag? | [Tag formats](concepts/tag-formats.md) |
| Is `A; B` one artist value or two? | [Multi-value fields](concepts/multi-value-fields.md) |
| What is the difference between `3/8` and a total field? | [Track and disc numbers](concepts/track-and-disc-numbers.md) |
| How do album artists and sort names help a library? | [Sort fields and compilations](concepts/sort-fields.md) |
| Why are loudness tags different from changing the audio? | [ReplayGain and loudness](concepts/replaygain.md) |
| Why does text contain garbled characters? | [Text encodings and codepages](concepts/text-encodings.md) |

## Make the workspace fit your routine

Start with [Customize TagGroove](../customize/index.md). It helps choose between
columns, tag-panel fields, suggested values, toolbars, external tools and
language settings. Save the current configuration before a large change so
there is a deliberate return point.

## When the task needs a formula or a batch

A **format string** builds text, a **regular expression** recognizes a text
pattern and an **action** applies a chosen operation. [Scripting](../scripting/index.md)
teaches the first two from the beginning, and [Choose the right tool](../getting-started/choose-a-tool.md)
helps decide between converters, actions, tag sources and export. To write or
load playlists and cue sheets, see [Export & playlists](../export/index.md).

## Related

- [Opening folders and files](../getting-started/opening-files.md)
- [Field names](../reference/field-names.md) and [tag mapping](../reference/tag-mapping.md)
- [Supported file formats](../reference/file-formats.md)
- [Troubleshooting](../help/troubleshooting.md)
