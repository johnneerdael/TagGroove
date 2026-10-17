# Command line

Launch TagGroove with a folder or file to open it in the desktop file list.
Use `/add` when a launch should append to the existing list. These switches
open the interactive application; they do not provide unattended tag editing,
action execution or export commands.

## Supported arguments

| Argument | What it requests |
| --- | --- |
| `/fp:<folder>` | Load a folder and use it as the working directory when replacing the list |
| `/fn:<file>` | Load a file and use its parent folder as the working directory when replacing the list |
| `/add` | Append the requested files/folders instead of replacing the list; preserve the existing working directory |

Switch names ignore case. A `-` prefix is also accepted, for example
`-fn:/home/you/Music/track.flac`. If the value after `fp:` or `fn:` is empty,
the next argument supplies it. Prefer the complete colon form above, with the
whole argument quoted when it contains spaces.

Bare paths are accepted too: existing folders become folder requests and
other paths become file requests. Relative paths are resolved against the
launching process's working directory, not the shell's script location or the
folder currently displayed in an already-open TagGroove window. Use absolute
paths for shortcuts and scripts that can start from different directories.

Multiple file/folder arguments are allowed. The first requested directory,
or the first file's parent when there are no directories, supplies the working
directory for a replacement request. `/add` alone has nothing to load.
Unrecognized switches can be ignored; they are not an alternate batch API.

## Windows: PowerShell

Replace the executable path below with the location of your installed
`taggroove.exe`. PowerShell's `&` invokes a quoted executable path. Each quoted
argument is passed as one item, including its spaces.

Open a practice album folder:

```powershell
& 'C:\Path\To\taggroove.exe' '/fp:C:\Music\Juniper Vale\Harbour Lights'
```

Open one file instead:

```powershell
& 'C:\Path\To\taggroove.exe' '/fn:C:\Music\Juniper Vale\Harbour Lights\01 - Paper Boats.flac'
```

Append another file without replacing the current list:

```powershell
& 'C:\Path\To\taggroove.exe' '/add' '/fn:C:\Music\Juniper Vale\Harbour Lights\02 - Lanterns on the Water.flac'
```

The single quotes here belong to PowerShell; they are not part of the path
TagGroove receives. In Command Prompt, use double quotes around arguments with
spaces instead. A Windows shortcut can put the switches after its quoted
executable path in **Target**.

## macOS

For an app installed in `/Applications`, invoke the bundle's executable when
passing explicit switches:

```sh
'/Applications/TagGroove.app/Contents/MacOS/taggroove' '/fp:/Users/you/Music/Juniper Vale/Harbour Lights'
```

To append a file:

```sh
'/Applications/TagGroove.app/Contents/MacOS/taggroove' '/add' '/fn:/Users/you/Music/Juniper Vale/Harbour Lights/02 - Lanterns on the Water.flac'
```

Finder **Open With**, dropping files on the Dock icon, and the ordinary
`open -a TagGroove <paths>` document-opening route use macOS open-document events.
They request a replacement list, rather than `/add`. Do not assume that
`open --args` sends new arguments to an already-running application; use the
direct executable route when you need the switches.

## Linux

Use the executable you installed or extracted. For example:

```sh
/path/to/taggroove '/fp:/home/you/Music/Juniper Vale/Harbour Lights'
```

```sh
/path/to/taggroove '/add' '/fn:/home/you/Music/Juniper Vale/Harbour Lights/02 - Lanterns on the Water.flac'
```

The shell quotes preserve spaces as one argument. A bare absolute Unix path
also remains a path; the leading `/` is not automatically treated as an
unknown Windows-style switch.

## Existing windows, ordering and troubleshooting

Desktop launch handling forwards recognized opening requests to an existing
instance and brings its main window forward. Requests wait while another
operation is busy. Replacing the list can require a pending-edit decision;
launching the program does not silently save those edits.

Appended files join the loaded list. An active sort can change their visible
placement. Choose **View → Unsorted (load and playlist order)** when you need
to inspect load order; `/add` is not a command to sort or renumber the files.

If nothing appears, check the executable path, the audio path and shell
quoting. Check TagGroove's file errors for unreadable or unsupported files.
The folder-recursion preference still governs scans of supplied directories.
Opening a second process is not evidence that the requested file loaded:
inspect the resulting file list before continuing.

See [File menu](menus/file.md) for interactive opening commands and
[keyboard shortcuts](keyboard-shortcuts.md) for working with the loaded list.

<!-- Source baseline: main5f4112e. Contracts: file_ops.rs parse_launch, opened_request, install; file-ops.ts FileOpsUI.launch, launchDirectory and scan preferences. Source-derived platform instructions; no new installed-shell/native session was run for this page. -->
