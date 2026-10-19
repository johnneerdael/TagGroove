# Export

Export creates a text file from selected audio files: a spreadsheet, track
list, HTML catalogue or another report. An **export configuration** is the
template that describes its contents. The output file is the finished report;
an `.mte` file stores a reusable template. Export does not convert audio.

Save pending tag edits, select the files, then choose **File → Export…**
(++ctrl+e++; use ++cmd+e++ on a Mac). Export uses the selected files in file-list
order unless the template sorts them. The current desktop workflow requires
local files; provider-document selections can disable the operation.

If **Show selection dialog** is off in
[Options → Export](../reference/options/export.md), the command runs the saved
configuration immediately. Turn that option on again to edit the template or
output choices in the dialog. The steps below assume the dialog is enabled.

## Make the first report

1. Select the **Text** configuration.
2. Set **Export file name** to an absolute path in a practice folder, such as
   `C:\Reports\tracks.txt` on Windows or `/Users/you/Reports/tracks.txt` on a Mac.
   **Browse…** chooses a location without writing the report yet.
3. Leave **Append data** and **Create one output file per** off.
4. Choose **Export**. The configuration and preferences are saved, then the
   report is written. Existing output is replaced unless append is on.

For selected files with `ARTIST = Juniper Vale` and titles `Paper Boats` and
`Lanterns on the Water`, Text writes one artist/title line per file. Open the
output in a text editor to inspect its actual contents. The
[catalogue lesson](../learn/export-a-cd-catalogue.md) builds a report in small
steps; the [template language](template-language.md) explains the commands.

## Manage configurations

Select an item under **Export configurations** to edit it. **Export template**
is a multiline editor. Changes remain when switching configurations within
the dialog, but **Cancel** does not save those edits.

| Control or shortcut | Result |
| --- | --- |
| **New** | Create a configuration and enter its name |
| **Duplicate** | Copy the selected configuration with a distinct name |
| **Rename**, ++f2++, or a second separate click on its selected name | Edit its name; Enter or leaving the name field keeps it, Escape cancels |
| Double-click a configuration, ++enter++, or ++f4++ | Focus its template editor |
| **Remove** | Remove the selected configuration when the library is saved |
| **Import .mte…** | Read a template file into a new configuration |
| **Save configurations** | Save templates and preferences without exporting; keep the dialog open |
| **Export** | Save configurations and preferences, then write the report |

Names must be nonempty and distinct without regard to case. Use **Legacy import
encoding (optional)** when an imported `.mte` contains non-Unicode bytes. This
controls reading the template, not encoding the report. A Unicode BOM
identifies the template's encoding; valid UTF-8 is read as UTF-8. Other legacy
text uses the selected/system ANSI codepage, with Windows-1252 as the fallback
on macOS and Linux.

Saved configurations have `.mte` files in the configuration folder's `export`
subfolder. TagGroove discovers other readable `.mte` files copied there when
opening the dialog. Files added with new names while it is open remain in place when saving;
reopen the dialog to see them. Removing a listed configuration and saving moves
its file to `export/removed/`. Copy a retired file back into `export` to recover
it. If saving reports that a file could not be retired, correct the folder
problem and save again in the same dialog. Reopening can list a file that
could not be retired; remove it again if that is still the intended change.
Keep a backup before editing shared configurations externally. A template
made for another application still needs syntax and output checks; its extension alone
does not establish compatibility.

[Configuration backups](../customize/configuration.md) also carry active
physical templates and their recorded text encoding. Restoring those assets
keeps the original bytes; the matching content uses its recorded decoding
choice, even if the destination's ANSI default differs. An external change to
those bytes invalidates that old decoding choice, so ordinary detection applies
again. Saving an edited template uses the normal saved-template encoding policy.
This concerns reading the template; the report's `$filename` encoding remains
separate.

## Choose a name and encoding

**Export file name** accepts a format string. A relative name uses the first
selected audio file's folder as its base; an absolute name chooses a separate
location. Fields in a name use the first row of the output group, so the name
should identify the group rather than an arbitrary track.

Choosing a configuration prefills a literal `$filename` default.
`$filename(html)` changes `export.txt` to `export.html`;
`$filename(tracks.cue)` supplies that filename in the current output folder.
With `/lists/export.txt` currently shown, `$filename(sub/report.csv)` supplies
`/lists/sub/report.csv`. A rooted name supplies its own destination.

When that argument contains placeholders, functions or quoting, choosing the
configuration clears the name field so the renderer can evaluate it for each
output group. For `$filename(%EXPORT_NAME%,utf-8)` with
`EXPORT_NAME = report.csv`, leaving the field empty produces `report.csv`,
not `out.report.csv`. Type a name after choosing the configuration to override
it deliberately.

Leave this field empty to use `$filename(...)` from the template. A nonempty
dialog name overrides the template's name while leaving its encoding choice
in effect. For example, `$filename(csv,utf-8)` chooses a `.csv` default but does
not replace an explicitly entered `report.txt` name.

`%_workingdir%` and `%_workingpath%` refer to the folder the file list was
last changed to, not necessarily the first audio file’s folder. They are
available in report text and output-name expressions.

Use UTF-8 for new reports. Set **Write BOM** under
[Options → Export](../reference/options/export.md) when the receiving program
needs a byte-order mark. It applies to UTF-8/UTF-16, not ANSI.
**Don't create directories** rejects destinations whose parent folder does
not exist; clear that option to allow folder creation.

## Append or replace

**Append data** adds the entire rendered report to an existing output file.
It does not merge headers, JSON arrays, HTML documents or CSV columns.
Appending a header repeats it. The destination must already use a compatible
encoding; append does not transcode earlier contents. A new BOM is suppressed
when appending to a nonempty file.

Use replace while designing a report. Export writes are not tag edits and are
not restored by tag-edit undo. Keep outputs separate from audio and other
files you need to retain.

## Create one output file per group

Enable **Create one output file per**, then set **Partition format**. TagGroove
evaluates this ordinary format string for every selected file. Equal text
values belong to one report; different values produce different reports.
Grouping is case-sensitive: `Harbour Lights` and `HARBOUR LIGHTS` differ.

For a report per album, `%albumartist%|%album%` works when those fields
reliably identify the album. Fill missing album artists or choose a deliberate
fallback. A separator avoids confusing `AB` + `C` with `A` + `BC`; values
containing that separator still need care.

The output name is evaluated separately for every group. For example,
`%albumartist% - %album%.txt` distinguishes reports in one folder. A fixed
`report.txt` would repeatedly target the same path, so TagGroove rejects the
collision. Each partition starts with fresh template variables and totals.

For a report in each audio folder, partition by `%_folderpath%` and name the
output `%_folderpath%report.txt`. Alternatively, use ++ctrl+shift+e++
(++cmd+shift+e++ on a Mac). Per-directory export temporarily
replaces the saved partition. A plain or relative name is placed under each
audio folder. An absolute output folder keeps that folder and mirrors the
selected folders' relative layout below it. Check destinations when files
span different roots or drives.

## Open results and troubleshoot

[Options → Messages](../reference/options/messages.md) can offer to open a
finished report with its associated program. With several reports, select a
written file; **Show in…** reveals its location. The offer is separate from
whether export succeeded.

| Symptom | Check |
| --- | --- |
| Export disabled | Select local files, save pending edits and wait for the current operation |
| Only one track appears | Put repeated content inside `$loop(...)` and `$loopend()` |
| Blank lines appear | Literal line breaks are output; place control commands deliberately |
| Partition collision | Make the output name differ for every group, including missing-field groups |
| Parent folder missing | Create it or clear **Don't create directories** |
| Incomplete report | Review file errors; several output files are not one transaction |
| Accented characters look wrong | Check output encoding and the receiving program's import setting, separately from `.mte` import encoding |

See [CSV](csv.md) for spreadsheet import and the
[template gallery](template-gallery.md) for ready-to-edit reports.
