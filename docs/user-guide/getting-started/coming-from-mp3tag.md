# Bring an Mp3tag setup across

Use **File → Import Mp3tag setup…** on a desktop to inspect a configuration
folder or ZIP and choose the supported parts to import. TagGroove reads the
original without changing it. This imports definitions and preferences; it
does not run an action, export a playlist, contact a tag-source provider or
write audio tags.

## Prepare the profile and your current session

<!-- figure:import-mp3tag-setup -->
<figure markdown>
![The Import Mp3tag setup dialog with buttons to choose an Mp3tag configuration folder or a backup ZIP and an optional legacy codepage](../assets/screens/import-mp3tag-setup-light.png#only-light)
![The Import Mp3tag setup dialog with buttons to choose an Mp3tag configuration folder or a backup ZIP and an optional legacy codepage](../assets/screens/import-mp3tag-setup-dark.png#only-dark)
<figcaption>File → Import Mp3tag setup… reads a configuration folder or backup ZIP and previews what can come across.</figcaption>
</figure>
<!-- /figure -->

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
| `genres.ini` | Import the original custom-genre names as the GENRE suggestion list; choose at most one file. Decode the original escapes and preserve valid names and spaces. An explicitly selected empty list clears the current custom genres. |
| `usrfieldvalues.ini` | Import custom suggestions and per-field preference choices; choose at most one file. Replace matching entries after confirmation while preserving unrelated lists and global defaults. |
| `mp3tag.cfg` | Preview confirmed portable switches, named choices and literal text values; choose at most one file. Import only the listed options and report the number of unmapped records. |
| `.mta` action groups | Reuse the existing MTA importer and retain unresolved actions with their diagnostics. Imported groups remain disabled until reviewed. |
| `.mte` export templates | Decode with the existing export importer and retain the template/name. Importing never executes the template. |
| `.src`, `.inc`, `.settings` source collections | Report them separately. Use [Install tag sources](../tag-sources/web-sources.md) for complete source bundles, then qualify a real lookup. |
| Other files and unmapped binary records | Leave their original bytes untouched. No silent guessed mapping is applied. |

Select the intended items. Selected column/panel layouts replace their active
layouts. For existing action-group/export-template names, choose **Keep current
items** or **Replace current items**. A replacement export retains its existing
TagGroove identity and physical filename. Partial imports keep their warnings;
an unresolved action does not silently become another action type.

Review overlapping genre/field-value sources together. A selected `genres.ini`
replaces the GENRE list supplied by a legacy panel; it preserves lists for
other fields. A selected `usrfieldvalues.ini` then takes precedence for the
matching lists and preference choices it supplies, including over the binary
genre preference. This order is fixed, regardless of checkbox order. The
imported genre-specific **Prefer values from tags** choice affects GENRE only;
the global fallback preference stays as it was.

These lists provide suggestions; importing them does not write audio tags.
Invalid/control-containing names are reported, and a list exceeding 5,000
unique names is refused rather than truncated. A file containing only invalid
names keeps your current list. An empty file is different: selecting it
explicitly clears the custom-genre list, with a warning in the preview.

### Review portable settings

For a supported `mp3tag.cfg`, the **Portable settings** item lists the actual
options found in that file and the values that TagGroove will use: On/Off for
switches, and Ask, Save changes or Ignore changes for Extended Tags navigation.
Selecting the item imports all its listed options together. Unlisted options
keep their current TagGroove values.

The supported options are save tags on arrow-key/single-click selection,
preserve modified-file timestamps, automatically select the next file after
inline editing, selection-box completion, current directory in the title,
natural sorting, selecting newly added files, keeping moved files in the list,
reading subfolders, hiding the folder image as cover art, user-defined genres
only, tag-panel visibility, filter visibility, notification-area icon visibility
and minimizing to that icon. The Extended Tags save mode also transfers;
it controls what happens when you navigate away from an edited file in that
dialog. It is independent of saving on ordinary file-list selection.
Notification-area behavior depends on the desktop
platform. These are preferences, not a transfer of the original window layout.

The appearance grid choice also transfers: **Off**, **Alternate row colors**
or **Grid lines**. Advanced tag preferences include POPULARIMETER display,
listing chapters, Matroska video mapping, single MP4 atoms, reusing unmodified
MP4 atoms/ID3v2 frames and the ID3v2 comment-language code. Reload files to see
changed rating fields or mappings; reload the directory to list/hide chapters.
These settings govern later reading/writing and do not rewrite audio during
setup import.

The incoming-file restriction and folder-separator replacement transfer as
literal text. An empty restriction means all supported formats; it does not
add support for formats TagGroove cannot read. An empty separator replacement
means removing separators from tag values when constructing filenames. The
preview shows an intentional empty text value as **(empty)**. Invalid language
codes, restrictions, separator replacements and unknown grid values are
skipped with a warning, preserving the corresponding current choice.

The **Mpeg** and **Ape, Mpc** Read, Write and Remove switches also transfer,
along with **ID3v2 only if ID3v1 too small** and the supported ID3v2
version/encoding choice. The preview lists these switches individually, even
though Mp3tag stores each group in one binary record. Read controls what is
displayed; Write controls what future saves write. For **Mpeg**, Remove controls
the explicit Remove tag command; ordinary saves leave unwritten tag types
alone. For **Ape, Mpc**, Remove also affects ordinary saves: a selected ID3v2
tag is removed, and selected ID3v1/APE tags are removed when that save does not
write them. Importing these choices does not write audio, but they govern
later saves. Review the Ape, Mpc Remove choices before saving an existing
library; for example, importing its original ID3v2 Remove choice can turn on
removal where your current TagGroove setup had it off.

TagGroove requires at least one Write type for each of those format families.
An incoming group with no supported Write type is therefore skipped with a
warning, keeping your current Write choices. Unknown bits and unrecognized
version/encoding choices are reported rather than guessed. Supported settings
in the same file can still transfer. Review these groups before editing a
mixed-format library, especially if you deliberately disabled a tag type in
your original setup.

The supported **Messages** switches also transfer. Notifications after saving,
changing or removing tags are separate from confirmations for cutting, pasting,
removing tags and deleting files. In particular, **Notify after removing tags**
does not control **Confirm removing tags**. Offers to open exported files or
playlists, Actions warnings, questions about unsaved changes and cover-import
confirmation have their own switches.

Review **Options → Messages** after import. Turning off questions about unsaved
changes allows the relevant close, reload or discard operations to discard
pending edits without asking. Cover-import confirmation and the **Without
asking, existing cover art is** choice are separate: the latter keeps its
current TagGroove value, so check it before importing artwork without a prompt.

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
The original selected-only automatic playlist scope is not transferred.

Export settings also include **Append data**, whether to create one file per
partition, the partition expression and **Write byte order mark (BOM)**.
A disabled partition expression is remembered: enabling **Create one output
file per** later restores it. An explicitly blank source partition expression
uses `%_folderpath%`, matching the original export dialog's default.
An absent expression keeps your current remembered expression; if none exists,
enabling partitioning uses `%_folderpath%`. Missing source switches keep their
current TagGroove choices.

To transfer the original last-used export configuration and output filename,
select both **mp3tag.cfg** and its matching **.mte** template in the preview.
The source template name must match exactly, including letter case. A selected
template retained through **Keep current items** binds to that retained target
configuration; **Replace current items** retains the target identity and
replaces its template. If the matching source template is absent or unselected,
a warning explains that the current active configuration and filename are kept.
The importer does not select an unrelated existing export merely because it
has a similar name. An empty imported export filename is meaningful: it lets
the selected template supply its own output filename.

Malformed partition expressions keep both your current partition switch and
expression. Invalid filename expressions, and Windows drive/UNC/backslash
filenames on macOS/Linux, are skipped with a warning. Edit **File → Export…**
for the new machine rather than relying on an automatic Windows path rewrite.
The imported BOM choice updates the effective **Options → Export** setting.
Importing these choices never runs the template or writes an export file.
If **Show export selection dialog** is off, inspect the active template and
output filename before your first export.

The **Options → Tags → Mapping** list also transfers. The preview shows the
number of rules and each ordered **tag type: source → target** triple. An
explicit source list **replaces the entire current mapping list**; it is not
merged with existing rules. An explicitly empty list clears current custom
mappings. If the source has no mapping-list record, your current list stays.
The supported tag types are `APEv2`, `ID3v2`, `MP4`, `VorbisComment` and `WMA`.
For example, `VorbisComment: DATE → YEAR` retains that exact rule in its
original position. Field names are shown literally, including Unicode.

An unsupported tag type or conflicting source rules keep the entire current
list with a warning. No valid-looking subset replaces it silently. Nonempty
reserved fields and trailing extension bytes are skipped with diagnostics;
the source file is unchanged. Each list is limited to 4096 rows and each raw
field to 4 MB; malformed framing or UTF-8 in the three public fields refuses
that binary settings file. After import, review **Options → Tags → Mapping**
before your next tag edit. Import does not read or rewrite audio with the new
rules.

**Default folder for Add cover** transfers as an explicitly empty value, a
relative folder such as `Scans` or `../Artwork`, or a supported file-derived
expression such as `%_folderpath%Scans`. Its authored text is preserved. Import
does not evaluate the expression, create folders or probe their contents.
The later Add cover chooser uses the expression for the selected file and
retains its existing fallback behavior. Static absolute/drive/URI prefixes,
including surrounding whitespace, and backslash syntax are skipped with a
warning under the portable-only import policy. This also applies on Windows;
host-specific paths are not silently reused on a different machine. Script
branches are not evaluated to discover their eventual paths during import.

The numbering wizard's **Save total track count**, **Disc numbers**, **Save
total disc count**, **Next disc on change of directory** and **Reset track
counter on next disc** preferences also transfer. Each source switch changes
only its matching remembered choice. Existing start values, minimum digits,
entered totals and unrelated preferences stay as they were; missing source
switches do not import the original application's defaults. TagGroove's wizard
always stages track numbers; an original profile's disabled track-number mode
has no equivalent toggle here. Review the wizard's Preview before staging.

The two **change of field** choices transfer when their source enabled switch
and usable field name are both present. Unicode names are preserved. An explicit
disabled switch clears that field choice. A name without its switch, an enabled
switch without a usable name, or names starting with `_` (including `_DIRECTORY`) keep
your current choice and produce a warning. The importer does not reinterpret
`_DIRECTORY` as an ordinary tag or substitute the separate next-directory rule.

These choices appear after setup reload in **Convert → Auto-numbering…**. Inspect
them, run **Preview**, then **Stage numbers**; setup import itself stages no tag
edits and writes no audio. The per-folder numbering shortcut still applies its
own temporary options for that run. It does not overwrite the imported ordinary
wizard memory. **Restore setup** restores the previous remembered numbering
choices along with the captured window state.

If current saved numbering preferences are invalid, importing numbering stops
before publishing any setup changes. With files selected, open Auto-numbering, review its options,
and stage a reviewed sequence to remember valid choices before inspecting the
profile again. Staged tag changes still require **Save tags** to write audio.
An explicit **Leading zeros: Off** for tracks or discs sets only that counter's
**Minimum digits** to `1`. **On** uses dynamic widths in the original application;
it is skipped with a warning and keeps your current fixed minimum digits.
It is not guessed as a fixed width of `2`. The Windows last-codepage chooser
index is also not mapped onto an unrelated codepage; review those choices
manually. A raw chooser index depends on the original Windows
locale list and cannot identify a portable codepage by itself.

### Other preferences in the native settings file

The preview also lists supported appearance, library, playlist and tag-source
preferences. Review them individually before selecting the settings file:

| Preference | What transfers and what to check |
| --- | --- |
| Color mode and language | System/light/dark and the stock English, Dutch or German catalog choice. An automatic source language becomes **System**; it follows this device's available language settings. Custom or unavailable catalogs keep your current language. |
| Interface size | Toolbar size and whole-interface size transfer separately. UI scaling offers every 10% step from 100% to 200%, plus the existing 125% and 175% choices. An invalid original UI percentage is normalized to 100%, as in the original Options dialog, and reported. |
| Startup folder | **Empty** and **Previous directory** transfer. Previous uses TagGroove's current history. A specific folder on the original computer is not silently substituted with a favorite folder here. |
| Library | An explicit unrestricted source policy can transfer with enablement. Restricted source folders are not copied onto this computer. Enabling without an admitted unrestricted policy keeps current enablement and explains why; an explicit disable can still transfer. Current included folders are preserved. |
| Playlists | The UTF-8 BOM choice transfers. Generate Playlists remembers its own filename and partition expression. An explicit empty generator filename uses the source's final admitted ordinary playlist filename; without that source value, your current generator filename stays. An empty partition remains empty. |
| Export folders | **Do not create directories** transfers independently of the playlist folder option. It applies when you later run an export. |
| Tag-source artwork | The cover filename expression and MusicBrainz size transfer. An explicitly blank cover filename stays blank; the existing save-cover workflow uses `folder` as its fallback. With no size record, your current size stays. |
| Cover-import answer | The remembered answer transfers separately from whether the question is shown. **Keep existing covers** means preserve; the other answer means replace. Importing setup changes neither existing artwork nor audio. |
| freedb servers | A nonempty, fully supported server list and its explicit valid selected index transfer together, preserving names and order. Native rows use HTTP; the importer does not silently switch them to HTTPS. Incomplete, invalid or empty datasets keep your entire current list and selection. Your email address stays unchanged. |
| Updates | Startup checking, stable/beta preference and release-note display transfer. They control **TagGroove** releases on GitHub. Original version history, last-check data and product endpoints do not transfer. Turning checks off preserves your current beta preference. |

Host-specific filename paths and invalid expressions stay out with a warning.
Partition expressions are grouping keys, so they are validated as expressions
rather than interpreted as folder paths. Nothing is evaluated during import:
no playlist, export, cover file, directory or network request is created.
After reload, inspect **Options** and **Generate Playlists…** before using the
imported settings. **Restore setup** restores the captured previous settings,
server list, appearance and language.

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
