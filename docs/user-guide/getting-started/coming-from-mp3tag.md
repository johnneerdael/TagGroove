# Bring an Mp3tag setup across

Use **File → Import Mp3tag setup…** on a desktop to inspect a configuration
folder or ZIP and choose the supported parts to import. TagGroove reads the
original without changing it. This imports definitions and preferences; it
does not run an action, export a playlist, contact a tag-source provider or
write audio tags.

## Prepare the profile and your current session

1. Locate the configuration folder or backup ZIP from your own Mp3tag setup.
   Keep a separate original copy outside the TagGroove configuration folder.
   The selected folder or ZIP must neither contain nor be inside that folder;
   overlapping locations are rejected before recovery or import can write.
   For `mp3tag.cfg`, only the confirmed options listed below are imported;
   other binary settings are reported without guessing their meaning.
2. Save or close edited action-group dialogs. Save pending audio/artwork edits,
   or explicitly discard them when prompted. Configuration restoration cannot
   recover discarded in-memory audio edits.
3. Choose **File → Import Mp3tag setup…**, then **Folder…** or **ZIP…**.
   For older action/export files, enter their original legacy codepage when
   known, such as `windows-1252`. Leave it empty to use the existing individual
   importers' defaults. The chosen codepage is retained in the report.
4. Read the inventory. Unsupported and invalid files stay visible, with their
   reasons; seeing a file in the inventory does not mean it can be transferred.
   Select only one incoming action group or export template for each destination
   name. Choosing two source files with the same name, including case variants
   in different subfolders, is rejected under both keep and replace policies.
   Those policies decide collisions with the current setup only.

Inputs are bounded to 128 MB and 4096 entries, with paths at most 64 folders
deep. Supported text definitions are limited to 4 MB each. ZIP paths must be
relative and cannot contain traversal, drive prefixes, symbolic links or
case-duplicate names. Linked files/folders are not followed. Split or simplify
an oversized profile rather than renaming unsupported files to another suffix.
Window recovery is bounded to 2 MB, 10000 listed files and 10000 roots. For a
larger workspace, save the file list as a playlist and clear the list before
inspecting the setup; the importer never silently truncates it.

## Choose what comes across

| Profile component | What the guided importer does |
| --- | --- |
| `columns.ini` or INI presets under a `columns` directory | Preview the existing column importer result; choose at most one layout to replace the active layout. Unsupported expressions/columns are reported. |
| `usrfields.ini` or INI presets under `panels` | Preview supported tag-panel controls and legacy custom-value settings; choose at most one layout. Preserve unrelated TagGroove lists and report unsupported entries. |
| `usrfieldvalues.ini` | Import custom suggestions and per-field preference choices; choose at most one file. Replace matching entries after confirmation while preserving unrelated lists and global defaults. |
| `mp3tag.cfg` | Preview supported portable settings as On/Off choices; choose at most one file. Import only the listed options and report the number of unmapped records. |
| `.mta` action groups | Reuse the existing MTA importer and retain unresolved actions with their diagnostics. Imported groups remain disabled until reviewed. |
| `.mte` export templates | Decode with the existing export importer and retain the template/name. Importing never executes the template. |
| `.src`, `.inc`, `.settings` source collections | Report them separately. Use [Install tag sources](../tag-sources/web-sources.md) for complete source bundles, then qualify a real lookup. |
| Other files and unmapped binary records | Leave their original bytes untouched. No silent guessed mapping is applied. |

Select the intended items. Selected column/panel layouts replace their active
layouts. For existing action-group/export-template names, choose **Keep current
items** or **Replace current items**. A replacement export retains its existing
TagGroove identity and physical filename. Partial imports keep their warnings;
an unresolved action does not silently become another action type.

When both a legacy panel and `usrfieldvalues.ini` provide a field-value entry,
the dedicated file takes precedence. Review those selections together. Field
values are suggestions and do not write audio tags during setup import.

### Review portable settings

For a supported `mp3tag.cfg`, the **Portable settings** item lists the actual
options found in that file and the values that TagGroove will use: On/Off for
switches, and Ask, Save changes or Ignore changes for Extended Tags navigation.
Selecting the item imports all its listed options together. Unlisted options
keep their current TagGroove values.

The supported options are save tags on arrow-key/single-click selection,
preserve modified-file timestamps, automatically select the next file after
inline editing, selection-box completion, current directory in the title,
reading subfolders, hiding the folder image as cover art, user-defined genres
only, tag-panel visibility, filter visibility, notification-area icon visibility
and minimizing to that icon. The Extended Tags save mode also transfers;
it controls what happens when you navigate away from an edited file in that
dialog. It is independent of saving on ordinary file-list selection.
Notification-area behavior depends on the desktop
platform. These are preferences, not a transfer of the original window layout.

Playlist preferences also include automatic creation after saving/renaming,
extended information, relative entries, confirming the filename and whether
directories may be created. A supported playlist filename and extended-info
format also transfer, preserving their text, Unicode and spaces. These values
are shown literally in the preview; import does not evaluate their expressions
or create a playlist. Each imported text value is limited to 4 MB. Malformed
UTF-8 refuses that binary file; the source stays unchanged.

The export selection-dialog preference transfers separately. If you import
automatic playlist creation, review **Options → Playlist** before saving tags:
an absent or unsupported filename/format keeps its current TagGroove value.
An empty playlist filename is skipped. On macOS/Linux, filenames containing
backslashes or a Windows drive prefix are skipped with a warning; edit the
filename in **Options → Playlist** for the new machine. The importer does not
guess how to rewrite a Windows path inside an expression.
The original selected-only automatic
playlist scope is not transferred. For exports, an imported **Show export
selection dialog: Off** uses the active TagGroove export configuration; it does
not import or select an original last-used export configuration by itself.

For example, if the preview says **Show filter: Off**, importing it hides the
filter after setup reload. Check that value before proceeding if you use the
filter every day. The previous value remains available through **Restore setup**.

Unmapped records can include other preferences and private account data. Their
payloads are not copied into TagGroove, and the source file is never rewritten.
A file with no supported settings, or an incomplete record, cannot be selected.
If the existing saved TagGroove settings are invalid, importing these options
refuses rather than replacing the rest of the setup with defaults.

For example, a profile might contain two valid columns and a third with a
malformed expression, a panel with Title and Comment plus an unsupported technical
control, an action group and a text export. Selecting those components imports
the two supported columns, supported panel fields, the disabled action group
and export. The skipped column/control and any unknown global-settings file
remain in the report. A syntactically valid unknown function can instead
remain as literal wrapper text in a column; review its rendered output rather
than treating successful import as proof that the function was implemented.
This is a partial profile transfer, not full editor parity.

## Bring an action group safely

Keep a copy of the original `.mta` file, then choose **Actions → Actions… →
Import .mta…**. Inspect the imported actions before enabling or running the
group. An unknown action type or setting becomes an **Unresolved imported
action**, and the imported group starts disabled.

Use **Export .mta…** to retain those unresolved settings for later inspection
or recovery. Encoded line breaks and literal backslash text survive export
and re-import, including when they appear next to each other. The action
remains unresolved; this does not prove that its settings work in TagGroove
or that the original app accepts the exported file. Export preserves decoded
values rather than the original bytes, section
numbers or escape spellings. Recognized actions with actual multiline
settings still report an export error.

See [Import and export an action group](../actions/index.md#import-and-export-an-action-group)
for encoding choices and unresolved-action handling. Importing or exporting
a group does not run its actions against audio files.

## Import, restart and verify

Choose **Import selected setup** after reviewing the replacements. The native
backend binds the preview to the exact source inventory, current managed
configuration, physical export templates and captured window state. If any of
those change, it rejects Apply and requires a fresh inspection.

TagGroove stages the supported outputs with the existing parsers/validators,
then records a local recovery journal before replacing managed files. The
window reloads to read the saved configuration. It rescans the captured local
workspace and restores file order/selection, location, filter and the explicitly
supported window preferences, including the last built-in quick tag source. A workspace receipt is consumed only after that
restore succeeds; later starts do not replay a consumed workspace. Recovery
runs before newly queued file-opening requests, so an explicit new launch is
then honored as a replace or add operation. Successful recovery suppresses
only the usual favorite/previous-folder fallback, not explicit launches.

Check the columns, panel, disabled action groups and export definitions after
reload. Review action behavior and external paths before using a definition
on copied audio. A successful definition import is not proof of equivalent
Mp3tag behavior, valid external commands or working provider credentials.

If workspace files are unavailable, their recovery stays pending in **File →
Restore setup…**. Make those files accessible and choose **Retry workspace**.
A failed file-read attempt is not automatically replayed over later work on
every start. A claim interrupted by a renderer/application exit resumes on the
next start, before ordinary startup or queued launches. Only a durably recorded
completed failure requires manual retry. If the claim or failure record cannot
be written, startup stays in a blocking retry dialog before admitting new
launches. An acknowledgement failure retains the restored workspace and a
pending, actionable receipt.

**Retry workspace** first records the native retry request, then restores browser
preferences and reloads. A rejected native request leaves current preferences
and the workspace unchanged. A browser-storage failure after that request keeps
the retry pending for the next startup; it does not mark recovery complete.
Audio contents are reread from disk; they are not restored from the setup snapshot.

## Restore the configuration from before import

Open **File → Restore setup…** to read the saved full report and current state.
Choose **Restore previous setup** to replace later managed configuration changes
with the pre-import snapshot. Save/discard pending audio edits deliberately
before proceeding. This is separate from the audio-file **Undo** command.

The local restore point contains the seven managed configuration files
(settings, actions, exports, columns, tag panel, directories and remembered
lists), the physical `export/` tree, and an explicit allowlist of application
window preferences plus the typed local workspace. Files created by the import
are removed when they were absent before it; prior managed file bytes are
restored. Field/history lists therefore retain their saved names/favorites,
and export definitions retain their actual template files.

API credential stores, installed source files, the library cache, audio-file
history and document-provider permissions are outside this restore point and
remain untouched. Browser globals and arbitrary storage keys are not captured.
The snapshot is local under `.profile-migration` in the current configuration
folder; it can contain personal paths and definitions and is not a public share
format. A later completed setup import or configuration Load replaces the offered
restore point; its preview discloses that replacement. Keep named backups when
you need more than this one local checkpoint.

Native file recovery and browser/window restoration are separate phases. A
failed file replacement attempts rollback; an interrupted applying/restoring
phase recovers before configuration-backed frontend services initialize.
Unexpected external edits, a damaged journal or a failed rollback retain the
snapshot and show a recovery error rather than overwrite unknown content.
Browser-storage failures keep their workspace receipt pending. Read the error
and use the offered reload/retry; do not delete the recovery journal to hide it.

New **Save configuration… / Load configuration…** archives include saved
settings, active physical export templates, supported remembered choices and
local workspace references. They use the same setup recovery system and latest
rollback checkpoint. Older version 1 seven-file archives remain supported but
cannot supply absent templates/preferences/workspace. See
[Save and restore configuration](../customize/configuration.md) for selections,
explicit path mapping and saved-only exclusions. Keep audio and separately
installed resources backed up independently.

## What remains to qualify

The populated automated fixture is independently authored. It exercises the
supported formats and negative/recovery cases; it is not a captured original
Mp3tag profile or a Windows differential comparison. Remaining global/binary settings,
unsupported mappings, native desktop picker/restart interaction, and any real
profile-specific differences still need separate evidence. No complete R02 or
full-parity claim follows from this import workflow.
