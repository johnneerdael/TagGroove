# TagGroove on Windows

Install the published desktop package, distinguish WebView2 startup problems
from audio-file errors, and choose explicit paths and encodings for a small
practice set. Windows uses Ctrl for the shared Primary shortcuts.

## Choose and verify the installer

The package formats and platform limits below were inspected in
[v0.2.13](https://github.com/johnneerdael/TagGroove/releases/tag/v0.2.13), before
the rename. The table shows the TagGroove naming pattern for new builds;
replace `VERSION` with the chosen release’s version. Older releases retain
their original filenames. Use the actual asset name and checksum from that
release, and read its notes before installing.

Scroll the download table sideways on a narrow screen to read complete filenames.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Windows release download">

<!-- windows-package-assets:start -->
| Processor build | Asset |
|---|---|
| x64 / 64-bit Intel or AMD | `TagGroove-vVERSION-windows-x64-setup.exe` |
<!-- windows-package-assets:end -->

</div>

This release has no separate Windows ARM64, 32-bit, portable ZIP or Store
package. Check a newer release's own assets before choosing it. A framework's
ability to build another target is not evidence that TagGroove published or
qualified that target.

Download **SHA256SUMS** from the same release and compare the full hash of
the installer. In PowerShell, from the download folder:

<!-- windows-checksum:start -->
```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '.\TagGroove-vVERSION-windows-x64-setup.exe'
```
<!-- windows-checksum:end -->

`-LiteralPath` means the filename is used as written rather than as a
wildcard. A matching hash is an integrity check against that release;
it does not substitute for trusting the source. The v0.2.13 installer
has **no Authenticode publisher signature**, so the guide cannot give a
verified publisher identity to expect in an unknown-publisher warning.
Review the download and your organization's policy before running it.

## Install and start with WebView2 available

1. Run the verified `-setup.exe` and follow its installation choices.
2. Open **TagGroove** from the installed shortcut.
3. If startup fails before the editor appears, record the exact message
   and check its runtime requirements before trying any audio operations.

TagGroove's Windows interface uses **Microsoft Edge WebView2**. The current
installer configuration leaves Tauri's default runtime bootstrapper mode
in place: if WebView2 is missing, obtaining it can require internet access.
An offline machine or restricted download policy can therefore need runtime
provisioning through its administrator. See the official
[Tauri Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/#webview2-installation-options).
That is separate from Rust/Node build tooling, which a user does not need
to install merely to run this desktop download.

The published package and runtime setting do not establish support for
every Windows version or policy configuration. Keep the OS version and
failure stage in a report rather than treating every startup error as a
bad MP3. Use the shared [installation checklist](../getting-started/install.md)
for download and update decisions.

## Open files from Explorer or the File menu

Use **File → Change directory…** for a copied album, or **Add files…** and
**Add directory…** for additions. You can also drop copied files/folders
into the app. A plain drop replaces the current list; holding Ctrl adds to
it. Finish pending edits before replacing a working list.

**File → Open in Explorer** reveals selected files or the working folder.
**Play** opens audio through the system's default association. The installer
configuration does not automatically register TagGroove as the default audio
handler or add a custom Explorer context-menu command. Use the File
menu or window drop if a file-manager launch is not configured.

A shortcut or script can pass file/folder arguments to the installed
executable. In PowerShell, `&` invokes a quoted executable path. This is
an example, with both paths to replace with your own locations:

<!-- windows-launch:start -->
```powershell
& 'C:\Path\To\taggroove.exe' '/fp:C:\Music\Practice Album'
```
<!-- windows-launch:end -->

The `/fp:` argument opens that folder; it does not run a tagging action.
A second recognized launch is forwarded to the existing instance. For
append requests and quoting, use the [command-line reference](../reference/command-line.md).
Do not infer that a requested folder loaded merely because a process started;
inspect the actual file list and any pending-edit decision.

## Shortcut focus and the notification area

Click the file list for selection commands. Text fields retain their text
editing behavior, and Ctrl+Alt/AltGr combinations can be character entry.
Use menu commands if a desktop utility intercepts a chord.

<!-- windows-key-cases:start -->
| Chord | Focus | TagGroove result |
|---|---|---|
| `Ctrl+D` | File list | Change directory |
| `Alt+T` | Text field | Extended tags |
| `Ctrl+Alt+T` | Text field | Text entry |
| `Ctrl+Tab` | File list | Next pane |
| `Delete` | File list | Remove rows from list |
| `Ctrl+Delete` | File list | Move files to Recycle Bin or trash |
<!-- windows-key-cases:end -->

Removing a row keeps its disk file. A trash command changes the filesystem;
permanent deletion is a separate command. Read the command label rather than
using Delete as a general “clear this” key. Ctrl+S saves selected pending tag
changes; actions and renames have their own immediate Apply/write steps.

Under **Options → General → Notification area**, **Always show icon in
notification area** and **Minimize to notification area** both start off.
The first keeps an icon available; the second can hide a minimized window
and its taskbar entry. Click its tray icon or choose **Show TagGroove** to
restore it. The tray menu's **Quit TagGroove** follows the window-close route,
including the pending-work decision. Minimizing is not saving pending tags.

## Paths and older text encodings

Quote paths with spaces in scripts. Literal folder separators typed into
a TagGroove filename format create folders; slashes inside a tag value are
data and use the configured replacement. Preview the entire destination
and retain enough track/disc information to keep names distinct.

For a long-path or destination error, try a copied album in a shorter,
clearly writable location and inspect that actual result. A character-cleaning
expression does not establish full-path validity or support by another
program. See [portable names](../converters/recipes/rename-safe-characters.md) for the
scope of `$validate`, Unicode counts and explicit truncation policies.

A Windows ANSI codepage is not synonymous with UTF-8 or Windows-1252.
TagGroove uses the system ANSI codepage for applicable legacy operations.
New text should use Unicode where possible. In **Text File → Tag**, the
legacy fallback is used only after BOM/valid-UTF-8 detection.

These practice bytes illustrate choosing a **known explicit** fallback.
They do not guess the codepage of every Windows computer:

<!-- windows-encoding-cases:start -->
| Source bytes in hex | Legacy fallback | Decoded text |
|---|---|---|
| `43 61 66 C3 A9` | `windows-1252` | `Café` |
| `43 61 66 E9` | `windows-1252` | `Café` |
<!-- windows-encoding-cases:end -->

The first row is valid UTF-8 and does not use the fallback. The second is
legacy text under the stated codepage. Inspect accents before applying an
import. Changing a filename extension or changing Windows' display language
does not repair already garbled field text; follow the
[encoding guide](../tagging/concepts/text-encodings.md) for the relevant operation.

## Credentials, settings and updates

The optional system-keychain choice uses Windows Credential Manager for
the Discogs token. If the credential operation fails, check its status and
error before repeating it. The secret is not included in a normal configuration
backup or returned as visible token text.

Use **File → Open configuration folder** to find this user's saved settings;
do not substitute another editor's configuration path. **Save configuration…**
saves supported configuration, not pending tags or audio backups. Finish
pending work and close the old app before updating with a newer verified
package. An update check can open a release page; it is not an automatic
in-place updater.

## If Windows-specific behavior differs

Include the exact release asset, Windows version, architecture and message,
and state whether the problem occurs during installer/runtime startup,
opening a file, writing a tag, or using a tray/shortcut action. Start with
one copied local file. Read-only package inspection and source/model tests
do not prove installation, WebView2 download or every Windows filesystem
behavior on your machine.

## Related

- [Opening files](../getting-started/opening-files.md)
- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
- [Files options](../reference/options/files.md)
- [Configuration backup](../customize/configuration.md)
