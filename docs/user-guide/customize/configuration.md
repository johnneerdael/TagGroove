# Save and restore your configuration

Use a named configuration backup before reorganizing your setup, changing many
options or moving to another computer. It preserves **saved settings and local
workspace references**. Keep a separate backup of your music: the configuration
archive contains neither audio nor unsaved tag/artwork edits.

Restoring a configuration does not save tags, run an action, launch an external
tool or contact a tag source. Finish and save those definitions deliberately;
review what they do before using them on your current files.

For a user-owned Mp3tag folder/ZIP, use the separate
[guided setup import](../getting-started/coming-from-mp3tag.md). Both setup-changing
workflows use the same local **Restore setup** checkpoint, explained below.

## Make a named backup

1. Finish the configuration you want to preserve: choose **OK** or **Apply**
   in Options and **Save groups** for edited action groups. Unsaved dialog
   work is absent from the archive.
2. Choose **File → Save configuration…**. Select a location outside the active
   configuration folder and a recognizable name, such as
   `Harbour-workspace-before-tools.json`.
3. Wait for the success report. Saved history/layout writes must finish before
   capture. Check the reported components, preferences, templates and workspace
   rows, including anything explicitly omitted.
4. Keep this archive separately from the application installation and your
   audio backup. Keep older named archives when you need several checkpoints.

You can save configuration while tag/artwork edits are pending. Those edits
stay in the current window and **are not copied into the backup**. Saving
configuration is not a way to save audio edits or an unfinished action editor.

If a saved-settings flush or capture fails, no new backup is published. Ordinary
saved changes that completed before the failure remain saved. The command
refuses an output that would replace the active private setup/recovery files
or captured audio, including audio referenced by listed CUE tracks; choose a
separate backup destination. If TagGroove cannot determine the audio behind a
CUE row, saving refuses rather than guessing. Make those CUE references
resolvable and try again; the existing output and audio remain unchanged.

## What a new backup includes

New archives use TagGroove's JSON bundle, `opentag-configuration`, version `2`.
The bundle is an allowlisted setup snapshot, rather than a recursive copy of
its configuration folder.

| Component | What is preserved |
| --- | --- |
| `settings.json` | Saved Options, including tools, toolbar definitions and option-page layout |
| `actions.json` | Saved action groups, definitions, order and enabled state |
| `exports.json` and active `.mte` assets | Saved export identities/preferences plus the active physical template bytes and their text encoding |
| `columns.json` | File-list columns and saved column sorting |
| `tag-panel.json` | Tag-panel field configuration, custom suggestion lists and their global/per-field preferences |
| `directories.json` | Favorite and recent directories |
| `lists.json` | Supported helper/history/value lists, names and favorites |
| Saved preferences | Supported appearance/language, layout/filter choices, numbering and Tag → Tag memory, Quick-action settings, confirmed group codepage, cover controls and tag-source choices |
| Local workspace | Exact row identities and manual order, selection, original source roots, working folder, filter/filter input, active playlist and the distinct display mode/source rank when known |

The workspace stores references. It rereads the files when restored; it does
not embed their tags, audio, CUE-sheet contents or artwork. Column sorting
uses those reread saved values; an unsaved title edit is not preserved just
because it changed the visible order. A legacy workspace without source rank
cannot invent an original playlist ordering.

Active export templates retain their saved bytes and matching decoding choice,
including unfinished syntax that you may need to repair. A different default
codepage on the destination does not require rewriting those bytes. Loading
their definition does not execute them.
Removed-template history and arbitrary files in the export folder are excluded.
To transfer the complete export library, select both `exports.json` and
**Export templates** in Load preview. Selecting metadata alone keeps the
destination physical templates; those files remain authoritative. Review
same-name template replacements before applying.

### Keep these separately

- Audio, CUE sheets and their backups, including already embedded artwork.
- Pending tag/artwork edits and unsaved configuration-dialog changes.
- Discogs credentials, personal source keys and provider access grants.
- Installed source scripts/settings, custom language files, external tools,
  referenced images/text files and other separately installed resources.
- Library caches, audio Undo snapshots and provider-document recovery data.

A backup retains references inside definitions without copying those resources
or changing their paths automatically. It can contain private folder names,
labels, formulas and tool arguments. Excluding credential containers does not
anonymize user-authored text; inspect a copy before sharing it publicly.

Release-announcement bookkeeping stays with the destination installation;
loading another computer's preferences does not suppress its release notes.

## Preview a restore before replacing anything

1. Save the **current** setup under another name first, for example
   `Harbour-before-restore.json`. Keep the incoming archive too.
2. Choose **File → Load configuration…** and select the intended TagGroove
   archive. Preview validates and reads it; it does not discard current edits
   or apply the incoming setup.
3. Inspect the component checkboxes, listed export-template replacements,
   workspace counts/unavailable paths and the rollback-checkpoint notice.
4. Select the components you intend to replace. **Unselected components are
   preserved.** Review again after changing selections or root mappings.
5. Choose **Replace selected configuration** only after the review is current.
   Resolve the pending-edit confirmation deliberately. Finish or close an
   unsaved action-group editor before changing setup.
6. Let the window reload and finish recovery. Check the restored options,
   templates, lists and workspace before using an action or external tool.
   Test on one copied recording first.

Confirming discard authorizes loss of the current unsaved audio/artwork edits
when the setup is successfully applied. The Load gate does not save audio and
leaves current unsaved edits in place when you cancel preview or a later
check refuses the operation before applying changes.

### Understand the replacement choices

| Choice | Result |
| --- | --- |
| Select a saved configuration component | Its incoming saved file replaces the current component; action libraries are not automatically merged |
| Leave a component unselected or absent | Keep the destination's current component |
| Select the complete saved-preference component | Replace supported remembered choices; supported choices absent from that snapshot reset to defaults, including an explicitly empty snapshot |
| Leave saved preferences absent/unselected | Keep the destination's preferences |
| Select the complete workspace | Restore all captured rows and their saved workspace state; a selected empty workspace intentionally clears the list |
| Leave incoming workspace absent/unselected | Retain the destination's captured local workspace across reload |

A successful restore validates the saved formats. It does not prove that an
external executable exists or that a source account is available. Installing a
definition never authorizes running it against your files.

## Resolve missing or moved music

Workspace restoration is complete-list restoration: unavailable rows are
reported instead of silently restoring a smaller selection. File and chapter/CUE
row identities must still resolve. Inspect the exact unavailable paths.

- If a removable volume is disconnected, reconnect it and review/retry.
- If a local POSIX music root moved, expand **Move a local music root**, enter
  **Original root** and **Replacement root**, then review the mapping.
  For example, explicitly map `/Volumes/Music` to `/Volumes/Archive/Music`.
- To load settings while keeping your destination list, deselect the incoming
  workspace and choose **Review selection**. This is different from claiming
  its missing music was restored.
- Foreign/unsupported workspace path styles retain their original row count
  and source roots in preview, even though the workspace cannot be selected.
  Choose **Review selection** to bind a settings-only restore before applying.
  TagGroove does not search disks or guess a path translation.

Workspace restoration and root mapping currently support local POSIX paths
(macOS/Linux). Windows-origin workspace references stay unavailable for import;
restore their supported saved settings with the incoming workspace unselected
and choose **Review selection** before applying.

Mappings apply only to the reviewed workspace references. Tool paths, favorite
directories, action inputs/output destinations and other saved strings are not
rewritten heuristically. Recheck them after moving to another computer.

## Finish or recover an interrupted restore

Wait until TagGroove confirms that the selected setup and workspace are restored.
Applying settings and finishing workspace recovery are distinct stages. An
error after applying changes does **not** establish that all saved files remain
unchanged.

Open **Restore setup** when recovery needs attention. Retain its recovery files;
use the offered workspace retry, explicit settings-only continuation or
**Restore previous setup** choice. An explicit settings-only recovery reports
that the captured workspace was omitted. It does not certify that missing
rows were restored.

The latest local rollback checkpoint is shared with **Import Mp3tag setup**.
Preview discloses whether the next operation will supersede it. This checkpoint
is separate from your named JSON backups and from audio Undo; it is not a
multi-level history of every configuration change.

## Load an older version 1 archive

Older `opentag-configuration` version `1` archives remain supported. They contain
only the seven JSON components, without physical `.mte` assets, saved window
preferences or workspace. Loading one preserves the destination components it
omits and retains the existing physical-template discovery behavior. It cannot
supply preferences or file-list state that were never in that archive.

The legacy component cases remain:

<!-- configuration-restore-cases:start -->
| Target before loading | Incoming bundle contains | Expected saved component afterward |
| --- | --- | --- |
| An existing action library | A different valid `actions.json` | The incoming action library replaces that component; the old groups are not automatically merged |
| Existing favorites | No `directories.json` | The existing directory component is kept |
| Existing custom toolbar settings | An incoming `settings.json` | Incoming saved settings, including their toolbar definition, replace that component |
| Existing columns | An invalid included `columns.json` | Validation rejects the bundle before component replacement begins |
<!-- configuration-restore-cases:end -->

All included components are validated before applying the selected setup.
The current Load flow uses the shared recovery checkpoint for old and new
archives; it does not use the former sequential-write shortcut. Unknown bundle
versions, unsupported configuration filenames, invalid components and empty legacy
bundles are refused. Renaming an unrelated archive to `.json` does not convert it.

## Bounds and practical transfer checks

Archives are limited to 32 MiB. A saved window/workspace snapshot is bounded
separately to 2 MiB, at most 10,000 listed rows and 10,000 roots; managed archive
entries are also bounded. A limit error stops the operation instead of truncating
your workspace. For a larger library, first save a playlist, then back up a
smaller workspace and load the playlist separately afterward.

After moving, recheck tools, favorites/startup folders, action/export resource
paths, source installation/credentials and screen layout. Keep named definitions
recognizable while correcting paths. The archive does not carry another
computer's provider permission grants.

## Locate the current configuration folder

On desktop, choose **File → Open configuration folder**. Use the folder reported
by the current application rather than another editor's standard/portable/Store
paths. The stable application identifier is `org.opentag.editor`; copying an
application bundle or AppImage does not create a portable profile beside it.

Desktop configuration transfer does not establish mobile provider access or
save-back support. Follow the inspected build's platform limitations.

Avoid editing live saved files while TagGroove writes them. Direct file changes
bypass the reviewed Load flow. When reporting a restore problem, include the
exact error and TagGroove version without publishing credentials or a personal
music library.

## Related

- [Coming from Mp3tag](../getting-started/coming-from-mp3tag.md)
- [Toolbars](toolbars.md)
- [External tools](tools.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Troubleshooting](../help/troubleshooting.md)
