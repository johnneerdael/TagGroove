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

## Look up the syntax

- [Format strings](reference/format-strings.md): combine tags, text and
  functions to build a value or filename.
- [Placeholders](reference/placeholders.md): read tag values and file
  properties such as `%artist%`, `%title%` and `%_filename%`.
- [Regular expressions](reference/regular-expressions.md): match, extract
  and replace text, with worked examples.

Use the search box (press ++s++ or ++slash++) to find a topic in this guide.
More topics are being written; only completed pages appear here.

## Get help and follow changes

Download installers and read changes on the
[GitHub releases page](https://github.com/johnneerdael/opentag/releases).
If a step in this guide differs from your build,
[report it on GitHub](https://github.com/johnneerdael/opentag/issues), including
your OpenTag version, operating system and the steps you tried.
