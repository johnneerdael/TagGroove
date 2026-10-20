# Download and install

Download TagGroove from the project's [GitHub releases](https://github.com/johnneerdael/TagGroove/releases).
Open a release's **Assets** list and choose a package for your operating system
and processor. You do not need Rust, Node.js or the source-code ZIP to run a
published package. Those are part of the developer build workflow.

## Choose the right download

The package formats and platform limits were inspected in **v0.2.12**,
before the rename. The table shows the TagGroove naming pattern for new builds.
Check the chosen release’s own notes and actual asset names before installing.
`arm64` means an ARM processor; `x64` means a 64-bit Intel/AMD processor.
On a Mac, **About This Mac** distinguishes Apple silicon from Intel.

On a narrow screen, scroll this package table sideways to read its limitations.

<div class="native-mapping-table" markdown>
<div markdown style="min-width: 48rem">

| System | Package naming pattern | What to expect |
| --- | --- | --- |
| Windows x64 | `TagGroove-vVERSION-windows-x64-setup.exe` | Desktop installer; no Authenticode publisher signature |
| macOS Apple silicon | `TagGroove-vVERSION-macos-arm64.dmg` or `.app.tar.gz` | Desktop application; check that release's signing notes and manifest |
| macOS Intel | `TagGroove-vVERSION-macos-x64.dmg` or `.app.tar.gz` | Same release-specific signing checks; choose the Intel build |
| Linux x64 or arm64 | Matching `.AppImage`, `.deb` or `.rpm` | Unsigned desktop packages, built on Ubuntu 24.04; older-distribution compatibility is not established |
| Android arm64 | `TagGroove-vVERSION-android-arm64-signed.apk` | Signed preview; document editing and save-back remain in progress |
| iOS ARM64 simulator | `TagGroove-vVERSION-ios-arm64-simulator-preview-debug.app.zip` | Unsigned simulator preview; not an iPhone/iPad installer |

</div>
</div>

There is no Windows portable ZIP or Store package in this release. Copying an
AppImage or a macOS application does not create a separate portable settings
profile: configuration belongs to the current user. Do not apply Mp3tag's
installation modes, Windows requirements or configuration paths to TagGroove.

Packages published before the rename retain their original filenames. Use the
exact asset name from the chosen release when checking its checksum;
`VERSION` below stands for that release’s version.

## Check the download before running it

The same release includes **SHA256SUMS** and **release-manifest.json**. Download
SHA256SUMS, calculate the SHA-256 of the package you downloaded, and compare
all hexadecimal characters with the line for that exact filename. A mismatch
means stop and download the correct asset again. Comparing with the release's
checksum detects a mismatched or damaged download; it is not a replacement
for trusting the download source or a publisher signature.

For example, in the folder containing the downloaded file:

=== "macOS"

    ```sh
    shasum -a 256 'TagGroove-vVERSION-macos-arm64.dmg'
    ```

=== "Windows"

    In PowerShell:

    ```powershell
    Get-FileHash -Algorithm SHA256 -LiteralPath '.\TagGroove-vVERSION-windows-x64-setup.exe'
    ```

    Microsoft's [Get-FileHash reference](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash)
    explains the returned hash and literal-path handling.

=== "Linux"

    ```sh
    sha256sum 'TagGroove-vVERSION-linux-x64.AppImage'
    ```

Change the filename to your actual architecture and package. You only need
to compare the asset you downloaded; checking every line as a batch would
also try to read the other packages you have not downloaded.

## Install the desktop application

=== "macOS"

    1. Open the matching `.dmg`, then copy **TagGroove.app** to your Applications
       folder. With `.app.tar.gz`, extract the archive to obtain **TagGroove.app**
       and copy that application instead.
    2. Open the copied application. The updated release pipeline requires
       Developer ID signatures and Apple notarization for both the app and DMG;
       check the chosen release's own notes and `release-manifest.json`.
       Older releases, including the inspected v0.2.13 and v0.2.56 baselines, have
       no Apple notarization and may be refused under macOS's normal policy.
    3. For an older non-notarized release that you have verified and trust, follow Apple's
       [app-specific opening procedure](https://support.apple.com/en-us/102445):
       after the opening attempt, use **System Settings → Privacy & Security →
       Open Anyway**, when offered, and confirm **Open**.

    A managed Mac may not permit that exception. A damaged-app or malware
    alert is not automatically the same situation; stop and follow Apple's
    guidance. Do not disable Gatekeeper globally or remove security attributes
    as a routine installation step.

=== "Windows"

    1. Run the x64 `-setup.exe` and follow its installation choices.
    2. If Windows displays an unknown-publisher or reputation warning, check
       that the download came from the intended release. v0.2.12 has no
       Authenticode signature; the guide cannot provide a verified publisher
       name to expect. Respect organizational restrictions.
    3. Open **TagGroove** after installation. If the WebView2 runtime is missing,
       the Tauri installer uses its default bootstrapper, which can require
       internet access to obtain it. See Tauri's
       [Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/).

    A runtime download failure is different from an audio-file read error.
    Resolve the installation problem before trying metadata operations.

=== "Linux"

    For `.deb` or `.rpm`, use your distribution's package installer so declared
    dependencies are handled. The v0.2.12 x64 DEB package is named `open-tag`
    and declares GTK 3, WebKitGTK 4.1 and Ayatana app-indicator libraries.
    A package being available does not qualify every distribution/version.

    For an AppImage, make the downloaded file executable using the file
    manager's permissions control, then open it. The command-line equivalent
    for the x64 example is:

    ```sh
    chmod u+x 'TagGroove-vVERSION-linux-x64.AppImage'
    ./TagGroove-vVERSION-linux-x64.AppImage
    ```

    See the official [AppImage quickstart](https://docs.appimage.org/introduction/quickstart.html)
    if the file cannot be launched. Missing runtime/FUSE support or system
    libraries need distribution-appropriate handling; an AppImage is not a
    guarantee of compatibility with every older Linux installation.

## Understand the mobile downloads

The Android APK is signed with the project's persistent application key.
Signing allows that package identity to be checked for updates; it does not
establish desktop feature parity, Google developer-verification status or
permission to install under every device policy. Follow Google's
[Android installation guidance](https://support.google.com/android/answer/9457058?hl=en)
and the device manufacturer's instructions for a downloaded APK. Keep
Play Protect enabled. Installation controls can vary by device and policy.

The iOS archive is a **simulator** application for a development environment.
It cannot be installed on an iPhone by opening the ZIP. Neither mobile asset
is evidence of App Store distribution or validated editing/save-back on your
device. Use copied material and follow the build's source-access messages.

## Update without confusing settings and music

**Help → Check for updates…** checks releases and can open a release page.
It does not install the update automatically. Check the new release's notes,
finish pending edits, close the old application, then use its matching package.
An optional startup check is configured under
[Updates options](../reference/options/updates.md).

Before an update or settings experiment, choose **File → Save configuration…**.
This writes a TagGroove JSON configuration bundle containing the supported
saved options, action groups, export configurations, columns, tag-panel layout,
favorite directories and remembered lists that exist in the configuration
folder. It does not save pending tag edits, back up audio, or include the
Discogs access token. Keep installed source files and other external resources
separately when you need them.

To restore that bundle, finish or discard pending work, then choose **File →
Load configuration…** and review the replacement warning. The application
reloads its window. A configuration bundle is not Mp3tag's ZIP backup format
and does not merge every setting from an arbitrary old installation.

On desktop, **File → Open configuration folder** locates the actual current
folder; use that instead of guessing another editor's path. Do not modify
live configuration files while the application is saving them. Removing the
application is separate from deleting per-user configuration or music: use
the operating system's app/package management and decide deliberately which
backups and settings to retain.

## If the application will not start

Check the release/architecture, verify the checksum, and record the exact
operating-system message. Do not repeatedly reinstall over an unexplained
security alert. When reporting an issue, include the release filename,
version, OS version and whether failure occurs before the window appears or
only after opening files. Do not attach access tokens or personal music.

These steps are based on release assets, package metadata, application source
and official platform instructions. They are not a claim that every native
installation, OS version or mobile device has been tested.

## Related

- [Start using TagGroove](index.md)
- [Opening files](opening-files.md)
- [The main window](main-window.md)
- [Editing, saving and undo](editing-and-saving.md)
