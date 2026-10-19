# Opening files

Loading files puts their current metadata into the file list. Start with a
copied album, then decide whether the next location should **replace** the
list or **add** to it. Loading is not a tag save, an audio conversion or a
backup of the originals.

## Open one practice folder

1. Choose **File → Change directory…** (++cmd+d++ / ++ctrl+d++ on desktop).
   From an empty window, **Open folder…** offers the same starting task.
2. Select the copied folder. Resolve any pending edits before accepting a
   replacement/discard prompt.
3. Wait for scanning to finish. Check the loaded count and any file errors.
4. Select one row and compare its filename with **Title** and **Album** in
   the tag panel. They need not match: names and stored fields are independent.

**Change directory…** replaces the list and sets the working directory.
The working directory supplies context for commands and placeholders; it is
not necessarily every loaded file's parent after other folders are added.
**Options → Directories → Read subfolders** controls folder recursion and is
on by default. Changing the setting does not rebuild the existing list;
open the folder again to apply it.

## Replace or add deliberately

<!-- first-use-load-scope:start -->
| Command or gesture | List behavior | Working directory |
| --- | --- | --- |
| **Change directory…** | Replace | The chosen folder |
| **Add directory…** | Append | Keep the existing working directory |
| **Add files…** | Append | Keep the existing working directory |
| Plain file/folder drop onto the file list | Replace | Does not establish a new working directory by itself |
| Cmd-drag on macOS / Ctrl-drag on Windows or Linux | Append | Keep the existing working directory |
<!-- first-use-load-scope:end -->

**Add directory…** uses ++ctrl+h++ (Control on every platform, including
macOS). An append is useful for editing two albums together, but check the
selection before assigning shared values. Already loaded files are not
intended to become duplicate rows merely because the same file is added
again; this does not establish a guarantee about every filesystem alias.

Drop onto the **file list**, away from the cover area: dropping an image on
artwork controls is a different task. Hold the add modifier during the drop.
If the result's scope is uncertain, stop and inspect the loaded/selected
counts before running a write operation.

Replacing the list can discard pending edits. Confirmation is controlled by
**Options → Messages** and is not an automatic save. A scan can also report
individual unreadable files; do not assume that a completed load means every
requested file was accepted or that an empty result preserves the old list.

## Control which files a folder scan finds

Under **Options → Files → Restrict incoming files to**, enter extensions such
as `*.mp3;*.flac`. The restriction applies to files discovered inside folders.
An explicitly added file can still be read even when its extension is outside
that restriction. An empty restriction allows the supported audio extensions.
Filename patterns such as `album*.mp3` are not extension entries.

For a concrete practice folder, suppose these supported, readable files exist:

<!-- first-use-scan-files:start -->
```text
Practice/01 - Paper Boats.mp3
Practice/bonus.wav
Practice/Disc 2/01 - Intro.mp3
```
<!-- first-use-scan-files:end -->

The expected results are:

<!-- first-use-scan-cases:start -->
| Input | Read subfolders | Restriction | Loaded paths relative to Practice |
| --- | --- | --- | --- |
| Folder | Yes | `mp3` | `01 - Paper Boats.mp3`; `Disc 2/01 - Intro.mp3` |
| Folder | No | `mp3` | `01 - Paper Boats.mp3` |
| Folder | Yes | (empty) | `01 - Paper Boats.mp3`; `bonus.wav`; `Disc 2/01 - Intro.mp3` |
| Explicit file `bonus.wav` | No | `mp3` | `bonus.wav` |
<!-- first-use-scan-cases:end -->

Folder restrictions do not remove files already in the list. If an album is
missing, check the folder chosen, recursion, extension restriction, any active
filter, and the file errors. A recognized extension does not make a damaged
file readable. Review [supported formats](../reference/file-formats.md) for
the difference between reading a container and writing its metadata.

## Return to a directory

**File → Favorite directories…** manages a saved ordered list. Add the current
folder or browse to another, use **Make first** for the default favorite, then
**Save**. Removing a favorite removes the shortcut, not the folder or music.
**Favorite directory** (++ctrl+m++, Control on every platform) opens the first
favorite and replaces the list. The File menu also offers individual favorites
and recent directories when present.

**Previous directory** and **Next directory** use the working-directory
history; they are not track navigation and are not the same as the recent
list. **Upper directory** opens its parent when one exists. These operations
can replace the list, so resolve pending edits first. Alt is Option on macOS;
++alt+left++ / ++alt+right++ navigate directory history when not editing text.

Under **Options → Directories → Startup**, choose an empty list, the previous
directory, or the first favorite. An unavailable previous/favorite entry is
not a saved snapshot of your old selection. A queued launch request takes
priority over startup restoration. Keep **Empty file list** while learning if
automatic opening makes it hard to see which folder was requested.

## Open from outside the window

The desktop executable accepts file/folder launch requests, including requests
to an already running instance; the [command-line reference](../reference/command-line.md)
explains append versus replacement flags. macOS file-open requests are handled
when the system delivers them. This does not mean the v0.2.12 installer
registers TagGroove for every audio extension or adds an Explorer context menu.
Use **Add files…** if the file manager's **Open With** route is unavailable.

**File → Load playlist/cue sheet…** loads supported local playlist entries or
CUE track rows and replaces the list after the pending-edit decision. A
playlist is a list of references, not embedded audio. Missing files and remote
URLs can produce errors. CUE/chapter rows have different write and command
limits; read [Playlists](../guides/playlists.md) and
[Chapters](../guides/chapters.md) before treating them as ordinary files.

A folder can have a name such as `archive.m3u` or `album.cue`. TagGroove checks
whether the input is a file or a directory: an existing folder with one of
these suffixes is scanned as a folder, rather than read as a playlist. Refresh
keeps that distinction. A missing or unreadable path can still report an error;
its suffix alone does not prove that it is a readable playlist.

## Understand platform access

Desktop folder loading describes local filesystem paths. In a document-provider
build, **Add files…** selects documents and folder commands can be unavailable.
The menu can say **Folder access is not available here. Use Add files.**
Access, writability and recovery are separate from whether a row is visible.
A mobile preview or browser preview is not evidence of desktop save-back
capability. Read the source/access message before editing important material.

## Related

- [The main window](main-window.md)
- [Editing, saving and undo](editing-and-saving.md)
- [Directories options](../reference/options/directories.md)
- [Files options](../reference/options/files.md)
- [The file list](../guides/file-list.md)
