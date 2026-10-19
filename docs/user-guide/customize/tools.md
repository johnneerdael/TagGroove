# Open selected files in an external tool

A tool starts a program you have installed, using paths from the current
selection. It can open a player, editor or inspection application without
making you browse to each track again. TagGroove does not automatically undo
changes that another application makes.

## Configure one tool on a copied file

1. Choose one copied ordinary local audio file. Save intended pending edits
   first so the other application reads the intended data on disk. A tool
   launch does not save pending tag-panel edits for you.
2. Open **Tools → Configure tools…**, or **File → Options… → Tools**.
   Choose **New** and give the entry a name such as `Inspect track`.
3. Use **Browse…** for **Path** to select the installed program. Enter the
   executable path separately from its arguments. A path containing spaces
   belongs in this field without adding shell quotation marks around it.
4. Leave **Parameter** empty if the program accepts a file path as its only
   argument, or enter:

    <!-- tools-basic-parameter:start -->
    ```text
    "%_path%"
    ```
    <!-- tools-basic-parameter:end -->

5. Start with **for all selected files (otherwise only the first selected
   file)** off. Save the options with **OK** or **Apply**.
6. Run the saved name from the Tools menu or the file-list context menu.
   Check what the external application actually opened before expanding the
   selection or enabling per-file launching.

TagGroove begins with no configured tools and permits up to 99 definitions.
Its editor changes the selected entry's fields directly; there is no separate
Edit-button step. **Remove**, **Move up** and **Move down** manage definitions.
Removing an entry does not uninstall the other program.

These instructions describe the launch contract. This guide has not executed
your chosen program, checked its options, or qualified an installed version.
Use that program's own argument syntax; the examples below explain how
TagGroove parses text, not a promise that a particular program supports a flag.

## Understand the two parsing stages

TagGroove first splits the Parameter text into arguments, then substitutes the
supported path placeholders inside each argument. It passes that argument
list directly to the program, without invoking a command shell.

For the following examples, the selected file is:

<!-- tools-practice-file:start -->
```text
/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac
```
<!-- tools-practice-file:end -->

The table shows the resulting argument list as JSON. The empty-text row means
leave the Parameter box blank; `""` in another row is actual typed double quotes.
JSON quotes, commas and doubled backslashes describe the resulting values;
paste the Parameter text, not the JSON list, into the application. Scroll a
narrow table sideways to read complete argument boundaries.

<div class="native-mapping-table" markdown>

<!-- tools-argument-cases:start -->

| Parameter text | Argument list for this file |
| --- | --- |
| (empty text) | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac"]</code> |
| `"%_path%"` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac"]</code> |
| `%_path%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac"]</code> |
| `--label "%_filename%" "%_path%"` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["--label","03 - Paper Boats","/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac"]</code> |
| `""` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">[""]</code> |
| `"" "%_path%"` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["","/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac"]</code> |
| `'two words'` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["'two","words'"]</code> |
| `"C:\Profiles\Audio Tools\preset.ini" "%_path%"` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["C:\\Profiles\\Audio Tools\\preset.ini","/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac"]</code> |
| `"%artist%" "$upper(%_filename%)"` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["%artist%","$upper(03 - Paper Boats)"]</code> |
| `"%_path%" > report.txt` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">["/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac",">","report.txt"]</code> |
| `"%_path%` | Error: unmatched double quote |

<!-- tools-argument-cases:end -->

</div>

Double quotes group literal whitespace and are removed by the parser. A pair
of double quotes can produce an empty argument. Single quotes are ordinary
characters here, unlike quoting in scripting expressions or many shells.
Backslashes are literal; adding `\` does not create a general quote-escape
language. An unmatched double quote is rejected.

Whitespace-only Parameter text also uses the default file-path argument.
There is no zero-argument tool mode here: an empty box passes a path, while
`""` passes one empty argument. Open a program normally when no file argument
is wanted.

The unquoted `%_path%` still becomes one argument because expansion happens
**after** splitting. Quoting it remains a clear convention, especially when
combining it with literal text. Literal paths with spaces need double quotes
to group them before expansion.

The `%artist%` and `$upper` row illustrates the limited language: ordinary tag
placeholders and scripting functions are not evaluated here. The recognized
`%_filename%` inside the function-looking text expands, but the surrounding
`$upper(…)` remains text. Inserted path text is not recursively scanned again.

Likewise, `>` is passed as an argument; it does not redirect output. Pipes,
shell wildcard expansion and command substitution are not performed by
TagGroove. The selected program decides what its arguments mean. Do not paste
an entire shell command into Path or assume a converter expression is a tool
parameter.

## All seven supported placeholders

Placeholder names are case-insensitive. For the practice file above and one
listed root `/Practice`, these are the illustrative values on a Unix-style
path system. Folder separators follow the actual platform.

<div class="native-mapping-table" markdown>

<!-- tools-placeholder-cases:start -->

| Placeholder | Value |
| --- | --- |
| `%_path%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">/Practice/Juniper Vale/Harbour Lights/03 - Paper Boats.flac</code> |
| `%_filename_ext%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">03 - Paper Boats.flac</code> |
| `%_filename%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">03 - Paper Boats</code> |
| `%_filename_rel%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">Juniper Vale/Harbour Lights/03 - Paper Boats.flac</code> |
| `%_directory%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">Harbour Lights</code> |
| `%_folderpath%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">/Practice/Juniper Vale/Harbour Lights/</code> |
| `%_folderpath_rel%` | <code style="display: block; min-width: 14rem; max-width: 22rem; white-space: pre-wrap; word-break: break-word">Juniper Vale/Harbour Lights/</code> |

<!-- tools-placeholder-cases:end -->

</div>

For a single individually loaded file or playlist, its containing folder supplies
the root context, including `/` or a Windows drive root such as `C:\`.

An actual loaded folder supplies its own path, even when its name ends in a
playlist suffix. For example, loading the folder `/Music/archive.m3u` makes
that folder the relative-path root; loading the playlist file
`/Music/archive.m3u` instead uses `/Music`. The filesystem kind, rather than
the name alone, decides which case applies.

Relative placeholders use the single listed-root context when available. With
multiple roots, no usable root, or a file outside that root, they can return
absolute paths. If the file is directly in the root, its relative folder
path is empty. That empty substitution still occupies its parsed argument;
it is not automatically omitted. Prefer `%_path%` when the other application
requires a complete location.

## Choose one launch or one per file

With **for all selected files** off, TagGroove runs once for the first selected
file in its ordered list. It does not combine every selected path into one
invocation. With the option on, it prepares one invocation per selected file.
Selection can include rows hidden by a filter, so inspect or reselect the
intended targets before launching a batch.

<!-- tools-scope-cases:start -->
| Selected files | Per-file option | Decision | Invocations prepared |
| --- | --- | --- | --- |
| `2` | Off | Run | `1` |
| `2` | On | Run | `2` |
| `11` | On | Decline confirmation | `0` |
| `11` | On | Confirm | `11` |
| `100` | On | Confirm | `100` |
| `101` | On | Confirm | Error: more than 100 per-file targets |
<!-- tools-scope-cases:end -->

The UI asks before starting a per-file tool for more than ten selected files.
The backend caps a per-file request at 100. That is different from the
99-definition configuration limit. Reduce a larger selection rather than
assuming the confirmation removes the limit.

A batch can partly launch before a later launch fails. An error does not
roll back programs already started. Check the external application's state
before retrying a whole selection.

## Paths, platforms and completion

Use a full existing executable path, not a command name that only a shell can
find through its search path. A macOS `.app` bundle is accepted and started
through `/usr/bin/open -a … --args …`; how that application consumes arguments
or opens windows is separate from TagGroove's parsing contract. An app bundle
path on another operating system does not establish support there.

TagGroove sets the spawned process's working directory to each file's existing
containing folder when available. For a macOS bundle, that process is the
`open` helper; the GUI application's own working directory is not guaranteed
here. This is separate from the listed-root context used for relative placeholders. Avoid relying on a relative program path or an
assumed launch directory copied from another tool definition.

**Started** means launch processes were created, not that the external task
finished successfully or opened a separate window for every file. TagGroove
discards their standard output/error and does not report their final exit
status. Check the other application's UI or output. If it edits tags, resolve
any pending TagGroove edits before using **Reload tags** to inspect the result;
a later save of old pending values could overwrite that external edit.

Tools require the desktop application and supported local-file paths.
Provider documents and chapter/CUE row references are not automatically valid
ordinary filesystem inputs for this command. No mobile or external-program
runtime qualification is implied by a saved definition.

## Shortcuts, default tool and custom buttons

The first ten tool definitions use ++cmd+1++ through ++cmd+0++ on macOS or
++ctrl+1++ through ++ctrl+0++ on Windows/Linux. The tenth slot uses 0.
Reordering changes slot assignments. Tool shortcuts are not dispatched while
typing in an ordinary text field, and Alt-digit chords belong to other commands.

**Default tool (double-click in the file list)** starts at **No action**.
Choosing a saved tool makes a row double-click run that definition using the
current selection/scope rules. **No action** leaves a row double-click without
an action. Open **Edit → Extended tags…** explicitly when you want that editor;
saved tools remain available from their menus and shortcuts.

A [custom toolbar](toolbars.md) can reference a tool by its saved name. Give
entries distinct names. Renaming/removing the tool can leave an old custom
button unavailable; changing the button's label does not choose a different
program. Re-add the intended definition through the custom-toolbar dialog.
Keep a [configuration backup](configuration.md) before changing many definitions,
and recheck executable/resource paths after transferring them to another OS.

## Adapt examples from elsewhere

The official Mp3tag community has a [user-defined tools collection](https://community.mp3tag.de/t/user-defined-tools/966).
Treat those command lines as ideas to inspect and adapt: program locations,
quoting, supported placeholders and application options can differ. Test one
copied file with the intended program before saving a batch workflow. A
community example is not a TagGroove compatibility or installation guarantee.

## Related

- [Tools options reference](../reference/options/tools.md)
- [Customize toolbars](toolbars.md)
- [Save and restore configuration](configuration.md)
- [Opening files and selection](../getting-started/opening-files.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
