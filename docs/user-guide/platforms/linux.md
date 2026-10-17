# TagGroove on Linux

Choose a package for your distribution and processor, use the desktop file
manager deliberately, and distinguish application-runtime dependencies from
optional notification-area support. Keep one copied album for the first run.

## Choose an actual published package

The package formats and platform limits below were inspected in
[v0.2.13](https://github.com/johnneerdael/TagGroove/releases/tag/v0.2.13), before
the rename. The table shows the TagGroove naming pattern for new builds;
replace `VERSION` with the chosen release’s version. Older releases retain
their original filenames. Use the actual asset name and checksum from that
release, and read its notes before installing.

Scroll the download table sideways on a narrow screen to read complete filenames.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Linux release downloads">

<!-- linux-package-assets:start -->
| Processor | Package choices |
|---|---|
| x64 / x86-64 | `TagGroove-vVERSION-linux-x64.AppImage`, `TagGroove-vVERSION-linux-x64.deb`, `TagGroove-vVERSION-linux-x64.rpm` |
| arm64 / AArch64 | `TagGroove-vVERSION-linux-arm64.AppImage`, `TagGroove-vVERSION-linux-arm64.deb`, `TagGroove-vVERSION-linux-arm64.rpm` |
<!-- linux-package-assets:end -->

</div>

Choose DEB or RPM when that package format suits your distribution, and let
its package manager resolve the declared dependencies. Choose the matching
AppImage when you want that distribution format. A source archive is not
one of these runnable desktop packages.

The packages are unsigned. The release build uses Ubuntu 24.04 for each Linux
architecture; that build environment does not qualify every older distribution,
container, desktop session or CPU. Read a newer release's own notes before
assuming its requirements and filenames are unchanged.

## Verify and open the package

Compare the downloaded file's hash with **SHA256SUMS** from the same release.
For the x64 AppImage, run this in the folder containing it:

<!-- linux-checksum:start -->
```sh
sha256sum 'TagGroove-vVERSION-linux-x64.AppImage'
```
<!-- linux-checksum:end -->

Change the filename for a DEB/RPM or arm64 download. Compare every hex digit
with the exact asset's entry. This checks integrity against that release,
not a publisher signature or every dependency on your system.

For a DEB/RPM, open the package in your distribution's installer and review
its proposed changes. The actual v0.2.13 x64 DEB package is named `open-tag`,
version `0.2.13`, architecture `amd64`. It declares these runtime packages:

<!-- linux-deb-dependencies:start -->
```text
libayatana-appindicator3-1
libwebkit2gtk-4.1-0
libgtk-3-0
```
<!-- linux-deb-dependencies:end -->

Those are that DEB's declarations, not package names to paste unchanged into
every distribution. A missing WebKitGTK/GTK dependency is an application
startup problem, separate from an unsupported audio file after the window
opens. The package's dependency list alone does not establish older-system
binary compatibility.

For an AppImage, use your file manager's executable-permission control, then
open it. The corresponding terminal steps for the x64 example are:

<!-- linux-appimage-run:start -->
```sh
chmod u+x 'TagGroove-vVERSION-linux-x64.AppImage'
./TagGroove-vVERSION-linux-x64.AppImage
```
<!-- linux-appimage-run:end -->

The official [AppImage quickstart](https://docs.appimage.org/introduction/quickstart.html)
explains the file-manager and terminal routes. If launch reports missing
runtime/FUSE support or system libraries, use distribution-appropriate
help for that exact message. The AppImage being executable is not proof that
all of its runtime needs are met. You do not need Rust/Node just to run a
published package.

## Use the file manager and command line

The inspected v0.2.13 x64 DEB, published before the rename, installs an
**OpenTag** entry with `Exec=opentag`. That historical entry does not declare
file MIME associations or a `%F` file-argument placeholder. New **TagGroove**
builds are configured to use `taggroove`; a new Linux package has not yet been
inspected for this page. Check the installed entry for the version in use.
Do not assume a double-clicked audio file is registered to open in the app.
Use **File → Add files…**, **Change directory…**, or drop copied files/folders
into its window.

A plain window drop replaces the list; holding Ctrl adds to it. **File →
Open in file manager** reveals files/the current folder through desktop
integration. **Play** uses the system's associated application. Availability
and behavior depend on the desktop session and file manager; a file path
can load successfully even when a reveal/default-player action is unavailable.

For a new TagGroove build, the expected executable name is shown below.
The historical v0.2.13 package instead needs `opentag`. This example opens an
existing copied folder; replace the command and location for the installed
version:

<!-- linux-launch:start -->
```sh
taggroove '/fp:/home/you/Music/Practice Album'
```
<!-- linux-launch:end -->

For an AppImage, use its actual filename/path as the executable instead.
The quoted argument preserves spaces and requests a folder load, not
unattended tagging. Recognized requests from another desktop launch are
forwarded to the existing instance. See the [command-line reference](../reference/command-line.md)
for `/add`, absolute paths, pending-edit decisions and load order.

## Shortcuts, focus and the status area

Shared Primary shortcuts use **Ctrl** on Linux. Application shortcuts only
work if the desktop/window manager delivers them to TagGroove. If a chord
is intercepted, choose its visible menu command before changing the task.
Text fields and AltGraph keep their text-entry behavior.

<!-- linux-key-cases:start -->
| Chord | Focus | TagGroove result |
|---|---|---|
| `Ctrl+D` | File list | Change directory |
| `Alt+T` | Text field | Extended tags |
| `AltGraph+T` | Text field | Text entry |
| `Ctrl+Tab` | File list | Next pane |
| `Delete` | File list | Remove rows from list |
| `Ctrl+Delete` | File list | Move files to Recycle Bin or trash |
<!-- linux-key-cases:end -->

A plain Delete removes rows and keeps disk files. Trash and permanent
filesystem deletion are separate operations; read the command label.
Ctrl+S writes selected pending tag edits, while actions/renames have their
own Apply/write boundaries.

Both **Options → General → Notification area** choices begin off.
**Always show icon in notification area** keeps an icon available;
**Minimize to notification area** can hide the minimized window/taskbar entry.
The tray menu's **Show TagGroove**, or clicking the icon, restores it.

The implementation needs an AppIndicator library and a compatible desktop
status area. It probes Ayatana/AppIndicator availability before creating
the icon and reports an error if unavailable. Installing a library is not
proof that your desktop displays indicators; inspect the actual result.
The window is hidden for this option only after icon setup succeeds.
Use the window and normal menus if notification-area support is unavailable.

## Mounted paths, labels and writes

Open audio through its current mounted path. A familiar device label does
not prove that it is mounted at yesterday's path, writable or still online.
Keep the source available until a write finishes and inspect any file error.
For a failed write, try one copied file in a known writable local folder
before assuming the metadata format is at fault.

`%_volume%` is optional information about the path's volume label, not a
mount command or a stored audio tag. Linux lookup consults the mount table
and known device labels; missing/unavailable labels can give empty text.
If you add a display-only column, this format gives a visible fallback:

<!-- linux-volume-format:start -->
```text
$if2(%_volume%,Unlabeled)
```
<!-- linux-volume-format:end -->

A reported `Music Drive` label displays `Music Drive`; an empty/missing label
displays `Unlabeled`. This display choice does not mount the drive, assign a
label, grant access or repair an audio file. The label cache can lag a
mount/label change; inspect the actual path and reload when diagnosing it.

Literal backslashes in filename formats still separate folders on Linux.
Tag-derived slashes have a separate replacement policy. Case and naming
rules depend on the actual filesystem, including a mounted external volume;
[portable filename guidance](../cookbook/rename-safe-characters.md) is a
component policy, not a guarantee for every destination.

## Credentials and per-user configuration

The optional system-keychain choice uses **Secret Service** for the Discogs
token. Its availability in a build does not guarantee an unlocked, working
credential service in every Linux session. Inspect errors and saved-location
status instead of assuming a headless/container session has the same store
as your interactive desktop. The token is excluded from normal configuration
backups and is not displayed as returned secret text.

Use **File → Open configuration folder** to locate this user's actual settings.
An AppImage does not automatically create a portable profile beside itself.
**Save configuration…** saves supported settings, not pending metadata or
music backups. Close the old app after finishing pending work before updating
the executable/package. The update check opens release information; it does
not perform a package-manager or AppImage replacement for you.

## Report a useful Linux problem

Include the release filename, architecture, distribution/version, desktop
session and exact error. State whether failure happens before the window,
only with a file, or in file-manager/tray/credential integration. Package
metadata, source and isolated model checks support these instructions;
they do not claim native launch/write tests on every distribution or session.

## Related

- [Installation checklist](../getting-started/install.md)
- [Opening files](../getting-started/opening-files.md)
- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
- [Library cache and external changes](../guides/library.md)
