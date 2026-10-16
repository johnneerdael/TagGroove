# Keyboard shortcuts

Shortcuts depend on focus. When the cursor is in a text field, clipboard and
undo keys usually edit that text. When the file list has focus, they can act
on selected files or their tags. A modal dialog has its own keys and blocks
most window commands until it closes.

In the tables, **Primary** means **Ctrl** on Windows/Linux and **Command (⌘)**
on macOS. **Control** always means the physical Control key. **Alt** is Option
(⌥) on a Mac, subject to the text-entry rule below. Function keys may require
the keyboard's Fn key.

## Platform and focus rules

- On macOS, Option+letter can type a character. While typing, use
  **Control+Option** for an Alt application shortcut, such as
  Control+Option+T for Extended tags. Outside text fields, Option alone works.
- Windows Ctrl+Alt and Linux AltGraph combinations are kept for text entry;
  they are not converter or action shortcuts.
- macOS reserves shortcuts such as Command+Q, Command+H, Command+M,
  Command+Tab and Command+Space. The panel toggle, add/favorite directory,
  pane cycling and helper menu use the physical Control keys listed below.
- Disabled menu commands remain unavailable through a shortcut. Save pending
  edits or select suitable local files when an operation asks for them.

These concrete cases illustrate the focus rule:

<!-- keyboard-routing-examples:start -->
| Press | Focus and platform | Expected behavior |
| --- | --- | --- |
| `Alt+T` | Windows/Linux text field | Extended tags |
| `Option+T` | macOS text field | Text entry |
| `Control+Option+T` | macOS text field | Extended tags |
| `Alt+5` | Windows/Linux file list | Tag → Tag |
| `Control+Option+5` | macOS text field | Tag → Tag |
| `Command+Shift+K` | macOS file list | Numbering per folder |
| `Command+Shift+K` | macOS text field | Text entry |
| `Control+Tab` | macOS file list | Next pane |
| `Ctrl+Alt+T` | Windows/Linux text field | Text entry |
<!-- keyboard-routing-examples:end -->

## Files, tags and selection

| Shortcut | Command and scope |
| --- | --- |
| Primary+S | Save selected files' pending tag changes |
| Primary+R | Remove tag types chosen in Options; while typing, consumed without removing tags |
| Primary+T | Reload tags from the selected files |
| Primary+B | Discard pending changes; it does not load configured default values |
| Primary+Z | Undo the recorded operation; text fields keep text undo |
| Primary+Y or Primary+Shift+Z | Redo, or repeat the last action when the menu offers Repeat |
| Primary+C / X / V | Copy/cut/paste tags outside text entry; text fields use their text clipboard |
| Primary+A | Select all listed files when not typing |
| Primary+Shift+A | Invert the file selection |
| Primary+U | Unselect all files |
| Primary+N | Select the file after the last selected row; also works from a tag-panel field and keeps focus there |
| Primary+G | Select files matching the active file's sort values |
| Primary+Shift+G | Select the next different group of sort values |
| Primary+F | Open Find for the file list |
| F2 | Edit the first selected file's filename column, or the first editable column when no filename column is available |
| F3 | Show/hide the filter |
| F4 or Primary+Shift+T | Open selected files in the default player; a configured default tool can replace the file-list double-click action |
| F5 | Refresh the loaded folders and file information |

`Primary+N` clamps at the last file; it does not wrap to the first. Similar-file
selection uses the active sort criteria, not an audio fingerprint. Selection
is separate from checking a dialog's enabled fields.

## Remove, delete, copy and move

| Windows/Linux | macOS | Result outside text entry |
| --- | --- | --- |
| Delete | Delete/⌫ | Remove rows from the list; keep the files on disk |
| Ctrl+Delete | Command+Delete/⌫ | Send selected files to the system Recycle Bin/trash |
| Ctrl+Shift+Delete | Command+Shift+Delete/⌫ | Permanently delete selected files without using trash |
| Ctrl+Shift+C | Command+Shift+C | Copy files to another folder |
| Ctrl+Shift+V | Command+Shift+V | Move files to another folder |
| Alt+Enter | Option+Enter | Reveal selected files in the file manager |

The key labelled Delete on many Mac keyboards sends Backspace. OpenTag accepts
that key for these file-list operations. Confirm the command's wording:
removing a row, trashing a file and permanently deleting it have different
effects. File moves/deletions are not ordinary pending tag edits.

## Folders and windows

| Shortcut | Command |
| --- | --- |
| Primary+D | Change directory; where folder picking is unavailable, use Add files |
| Control+H | Add a directory |
| Primary+Shift+D | Load the upper directory |
| Alt+Left / Right | Back/forward through directory history, outside text entry |
| Control+M | Open the first favorite directory |
| Primary+O | Open Options |
| Control+Q | Show/hide the tag panel |
| Control+Tab | Focus the next available pane: tag panel, filter, file list |
| Control+Shift+Tab | Focus the previous available pane |
| Primary+Num + | Auto-size file-list columns to their contents |

Hidden or unavailable panes are skipped during pane cycling. **Control+Q**
does not mean Command+Q on a Mac. The latter belongs to the operating system.
On macOS, Control+H and Control+M are directory commands outside text entry;
while typing they are left to the field's normal editing behavior.

## Converters, actions, export and sources

| Shortcut | Command |
| --- | --- |
| Alt+1 | Tag → Filename |
| Alt+2 | Filename → Tag |
| Alt+3 | Filename → Filename |
| Alt+4 | Text File → Tag |
| Alt+5 | Tag → Tag |
| Primary+K | Auto-numbering |
| Primary+Shift+K | Auto-numbering with a counter reset per folder |
| Alt+6 | Action groups |
| Alt+Shift+6 | Quick actions |
| Primary+E | Export with the saved selection-dialog preference |
| Primary+Shift+E | Export per audio folder |
| Primary+P | Playlist for all loaded files |
| Primary+Shift+P | Playlist per folder for all loaded files |
| Primary+L | Local freedb |
| Primary+I | Reopen the last used tag source |
| Primary+Shift+I | Quick use of the last tag source; inspect its matching behavior first |
| Primary+Alt+M | Manage covers |
| Primary+1 through 9, then 0 | Run configured tools 1–10 |

On macOS, use Control+Option+1 through 6 while typing instead of Option alone.
The cover-manager chord is explicitly Command+Option+M on a Mac. The
playlist-per-folder command is separate from **Generate playlists…**; do not
assume another application's shortcut opens that generator in OpenTag.

## Manual file order

With focus outside text entry, select the rows to move:

| Shortcut | Result |
| --- | --- |
| Alt+Up / Down | Move the selected rows one position |
| Alt+Page Up / Down | Move them to the beginning/end |
| Alt+drag | Move rows to the visible drop line |

Manual ordering leaves the active sorted view and uses the displayed order
as its starting point. Playlists and numbering can then use that list order.
It does not rename or move the audio files on disk.

## Dialog keys

The same F-key can mean something different inside a dialog:

| Area | Keys and meaning |
| --- | --- |
| Action groups | Insert: add; F2: rename; F4: edit; Delete: remove; Space: enable/disable selected groups |
| Action groups | Primary+A: select all; Primary+Shift+A: invert; Primary+E/D: enable/disable all groups; Primary+S: save group order/state |
| Actions and groups | Alt+Up/Down and Alt+Page Up/Down reorder selected items |
| Source review: fields | F2 edits a value; Space toggles its enabled state; Primary+A / Primary+Shift+A select/invert fields; Primary+E/D enable/disable all |
| Source review: tracks | Alt+Up/Down and Alt+Page Up/Down change the file-to-source track order |
| Toolbar customization | F4 edits the selected item; Space shows/hides it; Primary+E/D shows/hides all; Alt+Up/Down or Alt+Page Up/Down reorders |
| Export configurations | F2 renames; F4 or Enter focuses the template; a double-click also edits the template |

Changing a source-review field's selection is not the same as enabling it for
import. Check enabled fields and track matching before applying a source result.
See [actions](../actions/index.md), [export](../export/index.md) and
[Tag Sources menu](menus/tag-sources.md) for the corresponding workflows.

## Helpers and menus

**Control+Space** opens the helper menu beside a supported format-string input.
On macOS, Control+Option+Space also works when the system claims Control+Space
for switching input sources.
Within those helpers, **Shift+Delete** removes a remembered format string.
**Shift+Enter** in a supported tag-panel combo field adds its current text to
the custom-value list; it does not save that value to audio files.

Use arrows to move through an open menu and Escape to close it. Action and
installed web-source entries can have underlined access keys. Alt+A opens the
Actions menu's mnemonic route outside text entry. Focus and a dialog's own
keyboard rules take precedence over a window shortcut.

If a shortcut seems inactive, click the file list and retry a harmless command
such as Find. Check modifier notation, Fn mode, pending edits and modal dialogs.
The [menu reference](menus/index.md) provides the equivalent visible commands.

<!-- Source baseline: main5f4112e; keyboard.ts, edit-history.ts, file-ops.ts, list-commands.ts, list-behaviour.ts, helper-menu.ts, action-ui.ts, toolbar.ts, tag-sources.ts, tag-panel.ts, main.ts. Platform routing is source/unit-test evidence, not a new native keyboard session. -->
