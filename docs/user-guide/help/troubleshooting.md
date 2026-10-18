# Troubleshooting

Start by identifying the failed step: opening the application, reading a
file, calculating a preview, applying a write, or seeing the result in another
program. A successful step before it does not prove the later one succeeded.
Use copied practice files and keep the exact message before dismissing it.

## Protect work before diagnosing

If there are pending edits, decide whether to save or discard them before
replacing the list, reloading, closing or loading a configuration. Save only
the intended selection and read each file error. With selection autosave
enabled, moving to another row can itself write pending changes.

After a batch, distinguish three outcomes: saved files, failed files whose
changes remain pending, and files not attempted after cancellation. Cancel
stops remaining work; it does not reverse completed writes. Use the current
**Undo …** operation only after reading its label, count and any notice.
[Editing, saving and undo](../getting-started/editing-and-saving.md) explains
why pending edits can survive an Undo and overwrite its restored result if
saved without inspection.

## The application does not open

1. Confirm the exact release filename, operating system and architecture.
2. Compare the downloaded package's SHA-256 with that release's checksum.
3. Record the OS message and whether the application window ever appeared.
4. Follow the package-specific [installation guidance](../getting-started/install.md).
   A security-policy block, missing runtime, corrupted download and failed
   audio read are different problems.

Do not globally disable security controls or repeatedly replace an unexplained
package. No guide instruction can grant installation permission on a managed
computer or establish support for an untested OS version.

## Files are missing from the list

Check these in order so a view problem does not become an unnecessary write:

1. Check the loaded count. Did **Change directory…** replace an earlier list,
   or did **Add directory…** append to it?
2. Clear the filter. A loaded file can be hidden by a valid query; an invalid
   query has its own error state.
3. Check **Options → Directories → Read subfolders**. Reopen the folder after
   changing it.
4. Check **Options → Files → Restrict incoming files to**. This limits files
   discovered in folders, not every explicitly added file.
5. Read the file-error area. A supported extension does not make a damaged,
   inaccessible or unsupported variant readable.

A playlist references other paths and can contain missing or rejected remote
entries. A CUE sheet can produce track rows rather than one ordinary audio
row. Consult [Opening files](../getting-started/opening-files.md),
[Playlists](../guides/playlists.md), and [Chapters](../guides/chapters.md).
Do not treat an empty result as proof the original files were deleted.

## A tag write failed or the old value came back

1. Select one copied file and make a recognizable change to an ordinary
   field, such as its album.
2. Choose **File → Save tags**, then read that file's result. A success count
   for other files is not its success.
3. If the save failed, retain the pending edit while checking the reported
   access/format problem. Confirm the file is writable in its actual source.
   Another program using the file or a restricted location may require action
   outside TagGroove; do not change broad permissions as a first experiment.
4. After a successful save, use **File → Reload tags** and compare the value.
   Reload can discard pending edits, so resolve them before accepting it.
5. If the saved value is still unexpected, inspect the native storage and
   read/write policy with [Tag formats](../concepts/tag-formats.md) and
   [Tags options](../reference/options/tags.md).

A provider document can require source-access refresh or recovery rather
than an ordinary local save. Follow its actual **Source read-only**,
**Source access unknown** or **Recovery required** state. **Save a copy…**,
when available, has its own fields/cover limits; it is not a universal desktop
Save As or proof that the original provider document was updated.

## The player still shows old or different information

First establish the saved result with a reload in TagGroove. Then check that
the player opened the same path, including any rename. If it did, inspect its
metadata/library refresh behavior and the fields/tag families it chooses.
Do not assume a different displayed album artist or image proves a failed
write. The player may use a grouping field, another tag block, or an external
folder image. Test one copied file after an intentional player refresh.

See [Sort fields and compilations](../concepts/sort-fields.md),
[Tag formats](../concepts/tag-formats.md), and [Cover art](../concepts/cover-art.md).
Do not remove every tag family or embedded image to diagnose a display cache.
This procedure cannot certify an individual player's behavior.

## A calculation or regex preview is wrong

Reduce the example before applying anything:

1. Copy the exact input field value, preserving significant spaces, line
   breaks and value-list boundaries.
2. Record the exact converter/action, field target and formula. A plain text
   field, filename converter and filter do not share one evaluation context.
3. Test one small expression in the intended preview. Check spelling and the
   available placeholders before adding more functions.
4. Check commas, apostrophes, literal parentheses, empty arguments and
   quoting with [Format strings](../reference/format-strings.md).
5. For regex, record pattern, replacement and case setting separately.
   **Find** and the shared core use Boost-compatible syntax, but their quoting
   and case controls differ. Filter and web-source defaults also need their own context.
6. Compare both a matching and a nonmatching practice value. “No change” can
   mean no match, rather than a reported parser error.

Do not copy a successful Find pattern into a write action without checking
that action's dialect and preview. Follow [Regular expressions](../reference/regular-expressions.md)
and [Format strings from zero](../learn/format-strings-from-zero.md) for worked
examples. A correct expression still needs appropriate metadata and source
context; unavailable context is not repaired by adding arbitrary quotes.

## A rename or generated output fails

Inspect the complete proposed path, not only its last filename. Check missing
artist/album fields, unexpected leading separators, current working-directory
context, extension handling and the destination that already exists.
A second run can build on the first run's changed path. Use the
[folder-layout recipe](../cookbook/rename-into-artist-album-folders.md) to
practice relative roots and optional components before changing a library.

Do not treat preview as a reservation or assume every filesystem case/alias
collision is protected identically. If an operation partly succeeded, inspect
the reported final paths before retrying. Reports, playlists and saved images
are external files with command-specific overwrite/history behavior. Keep
backups of outputs worth preserving; session Undo is not a general output
rollback service. See [Export](../export/index.md) and
[Playlists](../guides/playlists.md).

## An action group produces warnings or repeats a bad result

Open the group definition and read the order of checked groups and actions.
A failed action can warn for one file while later actions continue. Export
and playlist actions are deferred group outputs and see the group's final
values. Turning off a warning dialog under **Options → Messages** does not
turn that warning into a successful transformation.

Inspect a practice file after each necessary stage, or duplicate the group
and reduce it to the failing part. Do not blindly rerun a group whose earlier
steps already changed fields or paths. Keep the original definition when
reporting a problem. [Actions and action groups](../actions/index.md) explains
continuation, output timing and recorded versus external side effects.

## A tag source or update check fails

Identify the stage before changing settings: fetching a response, parsing it,
matching tracks, downloading artwork, or applying reviewed metadata. A network
response alone does not prove the parser or tag write succeeded. Record the
source/provider name, request stage and exact message. Do not expose access
tokens, authorization headers or personal account data in a report.

For a custom web source, use its [installation guide](../tag-sources/web-sources.md)
and [parser reference](../tag-sources/web-source-reference.md). A script for
another engine may need adaptation. Isolate parsing with a non-sensitive
saved/minimal response rather than repeatedly calling a live service while
editing a pattern. Respect the service's own access requirements.

A failed **Check for updates…** only establishes that the release list could
not be read for that check. It does not prove the installed version is current
or that every tag-source service is unavailable. See
[Updates options](../reference/options/updates.md).

## Report a reproducible problem

**Help → Report an issue** opens the project's GitHub issue page on desktop.
On the current mobile Help route, a message provides the URL instead of
using the desktop opener. A useful report contains:

- TagGroove version from **Help → About TagGroove**, OS version, architecture and
  package filename when installation is involved.
- The exact menu/dialog/action, selected and visible file counts, filter state,
  and whether unsaved edits were present.
- File/container type and whether the row is ordinary audio, a chapter/CUE row
  or a provider-backed document.
- Numbered steps, expected result, actual result and the exact displayed error.
- For formulas, copyable input, expression, replacement and case options. For
  layout issues, a screenshot showing the relevant controls and window size.
- A generated or minimal copied fixture only when needed and safe to share.

The current Help menu has no **Open log folder** command or promised automatic
support archive. Start with the visible operation/file errors and dialog
messages. If diagnostic output is requested, review it before sharing:
paths, script contents, configuration and service responses can contain private
data. **Dismiss** clears visible errors; it neither fixes the cause nor retries
the operation. Preserve the message first.

Configuration backups contain supported saved configuration, not a forensic
record of every operation or a backup of pending edits/audio. Do not upload
a whole configuration or library merely because one field failed. The
[Help menu](../reference/menus/help.md) and
[installation page](../getting-started/install.md) describe the actual support
and configuration commands.

## Related

- [Frequently asked questions](faq.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Opening files](../getting-started/opening-files.md)
- [Messages options](../reference/options/messages.md)
- [About TagGroove and this guide](about.md)
