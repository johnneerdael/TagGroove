# TagGroove on macOS

Choose the build for your Mac, open music through Finder or TagGroove, and
learn which shortcuts use Command versus the physical Control key. This
page describes the desktop application; it does not turn a simulator
preview into an iPhone/iPad app.

## Choose and install the Mac download

The package formats and platform limits below were inspected in
[v0.2.13](https://github.com/johnneerdael/TagGroove/releases/tag/v0.2.13), before
the rename. The table shows the TagGroove naming pattern for new builds;
replace `VERSION` with the chosen release’s version. Older releases retain
their original filenames. Use the actual asset name and checksum from that
release, and read its notes before installing.
In **About This Mac**, distinguish Apple silicon from Intel.

Scroll the download table sideways on a narrow screen to read complete filenames.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Mac release downloads">

<!-- mac-package-assets:start -->
| Mac | Download choices |
|---|---|
| Apple silicon / arm64 | `TagGroove-vVERSION-macos-arm64.dmg`, `TagGroove-vVERSION-macos-arm64.app.tar.gz` |
| Intel / x64 | `TagGroove-vVERSION-macos-x64.dmg`, `TagGroove-vVERSION-macos-x64.app.tar.gz` |
<!-- mac-package-assets:end -->

</div>

These are separate processor builds, not one universal app. A source-code
ZIP is not the desktop download. Use the DMG for a disk-image copy workflow,
or extract the app archive to obtain `TagGroove.app`. Copy the application to
Applications before opening your practice files; do not use the mounted
image as a permanent home for the app.

Compare the downloaded package with the **SHA256SUMS** entry from that same
release. For the Apple-silicon DMG, run this in its download folder:

<!-- mac-checksum:start -->
```sh
shasum -a 256 'TagGroove-vVERSION-macos-arm64.dmg'
```
<!-- mac-checksum:end -->

Change the filename for the package you actually chose. Compare the whole
hash, not just its beginning. A matching checksum detects a mismatched or
damaged download; it does not identify a trusted publisher by itself.

v0.2.13 is **ad-hoc signed and not Apple notarized**. If macOS blocks a
trusted, verified download as an unidentified/non-notarized app, Apple's
[app-specific opening instructions](https://support.apple.com/en-us/102445)
explain the **System Settings → Privacy & Security → Open Anyway** route
after an opening attempt. Managed Macs can restrict that choice. Treat a
malware or damaged-app alert according to Apple's guidance instead of
assuming every alert means the same thing.

The release's packaging and version metadata do not establish a tested
minimum-macOS matrix. Use the build for your processor and include the exact
OS/version message if it cannot start. See [installation](../getting-started/install.md)
for the shared download and recovery checklist.

## Open a copied album

1. Start the copied/installed **TagGroove.app** and choose **File → Change
   directory…** to open a copied album. Keep the original audio elsewhere.
2. Select a track and inspect its tags before editing. Opening files does
   not itself save a new title or rename the file.
3. For later additions, use **File → Add directory…** or **Add files…**.
   An ordinary folder/open-document request replaces the working list;
   finish pending edits before switching albums.

Finder **Open With**, dropping supported audio, playlist or folder items on
TagGroove's Dock icon, and the ordinary `open -a` route feed the app's document
opening queue. The packaged associations offer TagGroove as an alternate;
they do not deliberately make it the default music player.

For an existing copied folder, this Terminal example uses the open-document
route. Replace the example user/folder with your own path:

<!-- mac-open-document:start -->
```sh
open -a TagGroove '/Users/you/Music/Practice Album'
```
<!-- mac-open-document:end -->

That requests a replacement list, not `/add`. To pass explicit TagGroove
switches, use the installed bundle executable as described in the
[command-line reference](../reference/command-line.md). Do not assume
`open --args` delivers new arguments to an already running app. Inspect
TagGroove's resulting file list and any pending-edit decision before proceeding.

Within the window, an ordinary file/folder drop replaces the list; holding
Command while dropping adds to it. Native launch requests are queued for
the current instance rather than being a headless batch-editing interface.

## Use the right modifier key

Most shared Primary shortcuts use **Command** on macOS: Command+S saves
selected pending edits, for example. Some keys keep **Control** because
macOS already uses the Command combination. Option is the Mac key named
Alt in cross-platform instructions.

The table's “text field” means the cursor is typing in a field; “file list”
means focus is outside text entry. Use visible menu commands when a system
shortcut intercepts a key.

<!-- mac-key-cases:start -->
| Chord | Focus | TagGroove result |
|---|---|---|
| `Command+T` | File list | Reload tags |
| `Control+H` | File list | Add directory |
| `Control+M` | File list | First favorite directory |
| `Control+Q` | File list | Toggle tag panel |
| `Option+T` | File list | Extended tags |
| `Option+T` | Text field | Text entry |
| `Control+Option+T` | Text field | Extended tags |
| `Control+Tab` | File list | Next pane |
<!-- mac-key-cases:end -->

Command+Q remains the Mac quit command; it is not the tag-panel toggle.
Command+H/M are not substitutes for Control+H/M in TagGroove. While typing,
Option+letter can produce a character, so Control+Option activates the
corresponding application Alt shortcut instead. Function keys may also
require your keyboard's Fn key. See the complete [shortcut reference](../reference/keyboard-shortcuts.md)
for focus, helper menus and dialog-specific keys.

## Finder, paths and external drives

**File → Open in Finder** reveals selected files or the working folder.
It is different from **Play**, which asks the system's default application
to open the selected audio. Use Finder's information/properties controls
for filesystem details; TagGroove's tag fields are metadata inside the audio.

In filename format strings, a literal backslash still separates folder
components even on a Mac. For reviewed `ARTIST = Juniper Vale`,
`ALBUM = Harbour Lights`, `TRACK = 3/8`, `TITLE = Paper Boats`:

<!-- mac-name-format:start -->
```text
%artist%\%album%\$num(%track%,2) - %title%
```
<!-- mac-name-format:end -->

This describes `Juniper Vale/Harbour Lights/03 - Paper Boats` beneath the
source file's containing folder, with the original audio extension retained.
It is a naming expression, not a Terminal command. Preview full paths before
applying it. Tag-derived slashes are handled differently from literal folder
boundaries; [filename rules](../reference/format-strings.md#file-and-folder-names)
explain that distinction.

Case sensitivity depends on the actual volume. For a spelling-only rename,
try one copy and inspect the result; do not assume another volume behaves
the same way. An external drive must be mounted and writable for writes to
succeed. Reading a tag successfully is not proof that a later write has
permission. Use the actual operation error and keep the source copy.

## Window, credentials and configuration

TagGroove's Windows/Linux notification-area controls are unavailable on macOS.
Minimized Mac windows use the Dock; there is no equivalent tray option to
turn on in this build.

The Discogs token dialog can optionally use the system Keychain. Saving to
Keychain is separate from ordinary options and from a configuration backup;
the UI reports whether a token is set, rather than returning the secret.
If a Keychain operation fails, read the error and inspect its saved-location
status before repeating it. Do not paste the token into an issue report.

Use **File → Open configuration folder** for this user's actual settings
location, and **Save configuration…** for a settings bundle. An app copied
from an archive does not automatically get a portable, beside-the-app
profile. Saving configuration does not save pending tags or back up music.

## If a Mac-specific operation fails

Record the release filename, Mac processor, macOS version and exact error.
Distinguish an app-opening/security problem, a file-access failure, a
shortcut intercepted while typing and a remote-provider error. Try the
visible command on one copied local file before expanding the selection.
These instructions combine inspected release metadata with source/model
checks; they are not a claim of installation or hardware testing on every Mac.

## Related

- [First tagging session](../learn/first-tagging-session.md)
- [Opening files](../getting-started/opening-files.md)
- [Saving and undo](../getting-started/editing-and-saving.md)
- [Configuration backup](../customize/configuration.md)
