# Frequently asked questions

Use these short answers to choose the right next step. The linked chapters
contain the complete procedure and its write, format and platform limits.
Start with copied files when investigating an unfamiliar result.

## Do I need to import music into an OpenTag library?

Open a local folder or add files to the working list. The audio remains in
its existing location; loading does not make a second copy. **Change
directory…** replaces the list, while **Add directory…** and **Add files…**
append. The optional Library feature is a cache of information read from
files, not an audio-file backup. See [Opening files](../getting-started/opening-files.md)
and [Library options](../reference/options/library.md).

## Why did a displayed change disappear?

An ordinary tag-panel or metadata-cell change can still be pending.
**Save tags** writes pending edits for the selected files, not necessarily
every unsaved file in the window. A failed write leaves that file's edit
pending. Replacing the list, reloading or closing can discard unsaved work;
configurable prompts do not save it for you. Check the selected count,
pending markers and individual errors. [Editing, saving and undo](../getting-started/editing-and-saving.md)
walks through a save and re-read.

## Can changing the selection save a file?

Yes, when **Options → Tags → Save tags when using arrow keys or a single
mouse click** is enabled. It is off by default. Extended tags has a separate
navigation save mode. Do not use “click another row” as a no-write test when
automatic saving is enabled. Read [Tags options](../reference/options/tags.md)
before practicing on an original album.

## Does `< keep >` mean the field is empty?

It means preserve each selected file's current value when those values
differ. Leaving it unchanged keeps the differences. Replacing it assigns a
common value, and explicitly clearing a field requests removal. To inspect
one file, select that file alone. See [The tag panel](../guides/tag-panel.md).

## Why are there two artists separated by `\\`?

That notation can display multiple stored values. It does not prove that the
literal separator characters are stored in the file or that a player uses
the same display. Different native formats support value lists differently.
Use [Multi-value fields](../concepts/multi-value-fields.md) before merging or
splitting artists; punctuation inside one person's name is not automatically
a list separator.

## Why does my player show another title, artist or cover?

First save, then reload the file in OpenTag to check its stored result.
If OpenTag reads the intended value, check which file the player opened and
which fields/tag families it uses. A player can retain an older library/cache
entry, choose another tag block, use `ALBUMARTIST` for album grouping, or show
an external image instead of embedded artwork. Do not remove every tag type
to experiment. [Tag formats](../concepts/tag-formats.md),
[Sort fields and compilations](../concepts/sort-fields.md), and
[Cover art](../concepts/cover-art.md) explain the distinctions. The guide does
not certify a particular player's refresh behavior.

## Why did filtering leave more files selected than I can see?

Visibility and selection are separate. A previously selected row can remain
selected when a filter hides it. Clear the filter and deliberately reselect
the intended targets before a batch write. Commands also differ in scope:
for example, the playlist command for all files uses visible rows, while the
selected-files route can include hidden selections. See
[Filtering](../guides/filter.md) and [Playlists](../guides/playlists.md).

## Does Remove tag delete the song?

**File → Remove tag…** removes configured tag families from selected files;
it is not an audio-file deletion command. **Edit → Remove from list** changes
the working list. **Delete files…** and **Delete files permanently…** are
filesystem operations with different recovery limits. Confirm the exact
command before continuing. [Removing tags](../guides/remove-tags.md) explains
policies, immediate writes and artwork consequences.

## Why did a converter or action not wait for Save?

Applying a converter or running an action writes its result immediately.
The ordinary tag-panel pending-edit workflow is not a universal transaction
around every command. A preview lets you inspect a proposed result; it is
not a backup or an output-path reservation. Read the command's page and
[Editing, saving and undo](../getting-started/editing-and-saving.md) for its
commit boundary.

## Will Undo recover everything after restarting?

No. Recorded file history belongs to the current desktop session, even though
it uses temporary on-disk snapshots while running. It is not a saved history
library reopened at the next start. Read **Undo …**'s operation label and
file count. History can be unavailable or incomplete, files changed elsewhere
can be refused, and external reports/playlists have separate cleanup needs.
Retention and free-space limits can remove older steps or prevent a large
operation from being recorded. Read any history notice rather than assuming
a successful write also has a complete Undo snapshot.
Pending edits remain when recorded history is replayed; inspect them before
saving again. Keep ordinary backups for recovery beyond the session. See
[Editing, saving and undo](../getting-started/editing-and-saving.md).

## Why does a format string appear literally in a field?

An ordinary metadata text box does not calculate expressions. To calculate a
field, use [Tag → Tag](../converters/tag-to-tag.md) or a suitable action. To
construct a name, use a filename converter. [Format strings from zero](../learn/format-strings-from-zero.md)
explains placeholders, optional sections, quoting and where evaluation occurs.
A successful calculation can still produce an unwanted empty value, so check
the preview and test input before applying it.

## Can I use the same regex everywhere?

Start by identifying the context. Format strings, actions, filters and
web-source parser commands and **Find** use the Boost-compatible core. Quoting,
replacement processing and case defaults still depend on the context. A pattern working in
Find is not proof it will work unchanged in an action. Follow the
[regex reference](../reference/regular-expressions.md) and the relevant
command's worked examples.

## Why did only part of an action group succeed?

An action failure can produce a per-file warning while later actions/groups
continue. Earlier successful writes remain. Batch exports and playlists can
still include the file with its resulting values. Read every warning and
inspect the step order; repeating the whole group can apply successful
transformations a second time. See [Actions and action groups](../actions/index.md)
and [Troubleshooting](troubleshooting.md).

## Can I paste an old editor's whole configuration into OpenTag?

OpenTag's **Save configuration…** creates its own supported JSON bundle.
**Load configuration…** replaces supported saved configuration and reloads
the window; it is not an importer for another editor's entire settings backup.
Individual export/source formats have their own compatibility limits. Inspect,
adapt and test an imported definition on copies rather than assuming identical
scripting or parser behavior. See [Download and install](../getting-started/install.md),
[Export templates](../export/template-language.md), and
[Web sources](../tag-sources/web-sources.md).

## Are mobile packages equivalent to the desktop application?

No. A compact **Files/Tags** layout describes presentation, not filesystem
access or save-back capability. Provider documents can have read-only,
unknown-access or recovery states, and unsupported commands can be disabled.
The published Android package and iOS simulator preview have explicit release
limits. Check [installation](../getting-started/install.md) and the current
[release notes](https://github.com/johnneerdael/opentag/releases) before using
original material. A simulator archive is not an iPhone installer.

## Can editing tags improve the audio bitrate or quality?

Changing metadata does not turn a compressed recording into a higher-quality
recording. Bitrate, codec and sample rate are technical information, not
ordinary editable title/artist fields. Metadata writes can change container
layout and file size without converting the audio into another codec.
Check [Supported file formats](../reference/file-formats.md) for actual
read/write capabilities; fixture checks are not a guarantee for every file.

## Related

- [Start using OpenTag](../getting-started/index.md)
- [Troubleshooting](troubleshooting.md)
- [About OpenTag and this guide](about.md)
- [Help menu](../reference/menus/help.md)
