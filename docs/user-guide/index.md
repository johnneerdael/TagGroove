---
hide:
  - navigation
  - toc
---

# OpenTag user guide { .md-visually-hidden }

<div class="ot-hero" markdown>

<div markdown>

![OpenTag](assets/brand/logo-on-light.svg#only-light){ .ot-hero__logo }
![OpenTag](assets/brand/logo-on-dark.svg#only-dark){ .ot-hero__logo }

## Tag, rename and organize your music

OpenTag edits the metadata inside your audio files: titles, artists,
albums, track numbers and cover art. Start with a copy of an album, learn
to edit and save its tags, then give the files consistent names.

[Start your first session](learn/first-tagging-session.md){ .md-button .md-button--primary }
[Download OpenTag](https://github.com/johnneerdael/opentag/releases/latest){ .md-button }

</div>

<div markdown>

![The OpenTag main window with an album open](assets/screens/main-window-light.png#only-light){ .screen }
![The OpenTag main window with an album open](assets/screens/main-window-dark.png#only-dark){ .screen }

</div>

</div>

!!! note "An evolving editor and guide"
    OpenTag is in development. This guide covers desktop workflows; mobile
    builds and simulator previews have different capabilities. Check the
    [release notes](https://github.com/johnneerdael/opentag/releases) for
    available packages and limitations. OpenTag is an independent project;
    complete Mp3tag compatibility remains a goal.

## Learn by doing

<div class="grid cards" markdown>

-   :material-school-outline:{ .lg .middle } **Your first tagging session**

    ---

    Open an album, change tags on one file and a selection, rename the
    files, and undo a change.

    [:octicons-arrow-right-24: Start the tutorial](learn/first-tagging-session.md)

-   :material-numeric:{ .lg .middle } **Give track numbers two digits**

    ---

    Rename `2 Low Tide.mp3` to `02 Low Tide.mp3` so files sort in order.

    [:octicons-arrow-right-24: Zero-padded track numbers](cookbook/rename-zero-padded-track-numbers.md)

-   :material-folder-music-outline:{ .lg .middle } **Get tags from names**

    ---

    Use your existing artist and album folders to fill empty tags.

    [:octicons-arrow-right-24: Tags from folder and file names](cookbook/tags-from-folder-and-filename.md)

-   :material-account-music-outline:{ .lg .middle } **Keep guest artists in titles**

    ---

    Build an action group that moves “feat.” names from the artist field
    into the title.

    [:octicons-arrow-right-24: Move guest artists to titles](cookbook/feat-artists-to-title.md)

</div>

## Build on the first session

| What you want to learn | Start here |
| --- | --- |
| Install, open files and understand the workspace | [Start using OpenTag](getting-started/index.md), [installation](getting-started/install.md) and [the main window](getting-started/main-window.md) |
| Edit shared fields and decide when to save | [The tag panel](guides/tag-panel.md) and [Editing, saving and undo](getting-started/editing-and-saving.md) |
| Transfer or remove stored tags | [Cut, copy and paste tags](guides/copy-paste-tags.md) and [Removing tags](guides/remove-tags.md) |
| Inspect the list, unfamiliar fields and artwork | [The file list](guides/file-list.md), [Extended tags](guides/extended-tags.md) and [Working with covers](guides/covers.md) |
| Understand metadata and native storage | [What tags are](concepts/what-are-tags.md), [tag formats](concepts/tag-formats.md) and [track/disc numbers](concepts/track-and-disc-numbers.md) |
| Set up your workspace and locate text | [Configure columns](customize/columns.md), [customize the tag panel](customize/tag-panel.md) and [Find](guides/find.md) |
| Clean text and artist credits in a safe order | [Trim spaces](cookbook/trim-spaces.md), [trailing annotations](cookbook/remove-text-in-brackets.md), [artist lists](cookbook/split-multiple-artists.md) and [Artist - Title](cookbook/split-artist-title.md) |
| Correct text encoding or keep only reviewed fields | [Capitalization exceptions](cookbook/fix-capitalisation.md), [mojibake repair](cookbook/fix-mojibake.md) and [a field keep list](cookbook/remove-junk-fields.md) |
| Work through an album's numbering | [Split track totals](cookbook/split-track-total.md), [number per folder](cookbook/number-tracks-per-folder.md), [disc numbers](cookbook/disc-numbers-from-folders.md) and [year text](cookbook/year-from-date.md) |
| Understand and manage embedded artwork | [Cover-art concepts](concepts/cover-art.md), [embed folder images](cookbook/embed-folder-jpg.md), [resize covers](cookbook/resize-covers.md) and [keep front covers](cookbook/keep-only-front-cover.md) |
| Inspect incomplete or unusual files | [Missing fields](cookbook/find-missing-fields.md), [missing embedded covers](cookbook/find-missing-covers.md) and [bitrate thresholds](cookbook/find-low-bitrate.md) |
| Keep display, sort order and loudness metadata distinct | [Sort fields and compilations](concepts/sort-fields.md) and [ReplayGain](concepts/replaygain.md) |
| Combine fields, functions and fallback values | [Format strings from zero](learn/format-strings-from-zero.md), then the [64-function reference](reference/scripting/index.md) |
| Match, capture and clean patterned text | [Regular expressions from zero](learn/regex-from-zero.md), then [regex syntax and examples](reference/regular-expressions.md) |
| Repeat several changes in order | [Build your first action group](learn/first-action-group.md), then [all action types](actions/index.md) |
| Build folder paths for albums, discs and compilations | [Artist/album folders](cookbook/rename-into-artist-album-folders.md), [multi-disc albums](cookbook/rename-multi-disc-albums.md) and [compilations](cookbook/rename-compilations.md) |
| Load or create playlists and understand chapter rows | [Playlists](guides/playlists.md), [one playlist per album folder](cookbook/playlist-per-album.md) and [chapters/CUE rows](guides/chapters.md) |
| Rename files, import text or number albums | [Converters](converters/index.md) and [auto-numbering](converters/auto-numbering.md) |
| Create a catalogue or spreadsheet | [Export a CD catalogue](learn/export-a-cd-catalogue.md), then [CSV](export/csv.md) and the [template language](export/template-language.md) |
| Parse a web source you control | [Write a source from local JSON](tag-sources/writing-web-sources.md), then the [command reference](tag-sources/web-source-reference.md) |

## Look up the syntax

- [Format strings](reference/format-strings.md): combine tags, text and
  functions to build a value or filename.
- [Placeholders](reference/placeholders.md): read tag values and file
  properties such as `%artist%`, `%title%` and `%_filename%`.
- [Regular expressions](reference/regular-expressions.md): match, extract
  and replace text, with worked examples.

For everyday lookup, see [Options](reference/options/index.md),
[keyboard shortcuts](reference/keyboard-shortcuts.md), [filter syntax](reference/filter-syntax.md)
and [field mappings](reference/tag-mapping.md).
[Context and helper menus](reference/menus/context-menus.md) explain cover
commands, field suggestions and named history entries; the
[menu reference](reference/menus/index.md) helps locate each control.

Use the search box (press ++s++ or ++slash++) to find a topic in this guide.
More topics are being written; only completed pages appear here.

## Get help and follow changes

Download installers and read changes on the
[GitHub releases page](https://github.com/johnneerdael/opentag/releases).
If a step in this guide differs from your build,
[report it on GitHub](https://github.com/johnneerdael/opentag/issues), including
your OpenTag version, operating system and the steps you tried.
