# iOS and iPadOS

The published iOS package is an unsigned **ARM64 simulator preview**, intended
for a development environment on a Mac. It is not an installer for an iPhone
or iPad. Opening its ZIP in Files does not install TagGroove on your device.

For everyday tagging now, use a [desktop package](../getting-started/install.md)
and copied local music. The iOS preview does not currently provide a supported
route for opening your music documents and saving their tags back.

## Understand the download

Look at the complete filename in the chosen release's **Assets** list.
The iOS naming pattern contains `ios-arm64-simulator-preview-debug.app.zip`.
The words *simulator*, *preview* and *debug* describe the package's purpose;
`arm64` alone does not make it a physical-device application.

| Download or claim | What it means for you |
| --- | --- |
| Simulator `.app.zip` | A packaged application for an ARM64 iOS simulator environment, not a device installation route. |
| Successful simulator build | The source compiled and a simulator-target bundle was produced. It does not prove that a workflow was run successfully. |
| iPhone or iPad layout | A layout can fit a smaller screen without gaining permission to read or write your music. |
| Native libraries built for iOS | Build support does not by itself provide an installed, signed device app or document access. |

The current release path does not provide a signed IPA, TestFlight invitation
or App Store package. Use the release's actual asset list and limitations;
instructions for an unrelated iOS application do not supply those missing
distribution steps.

## Why there is no music-opening procedure here

iOS applications need an explicit document-access route to work with files
chosen outside their own storage. The current TagGroove iOS path has neither
the desktop folder picker nor the Android provider-document integration.
There is therefore no supported sequence here for selecting an album in
Files, editing it and publishing the result back to that document provider.

Do not interpret **Add files**, a desktop screenshot or a successful package
download as evidence that a device has granted access to those files. The
[Android instructions](android.md) describe a separate Android route and
cannot be transferred to iOS by substituting the name of the operating system.

## Choose a practical route for your files

If your goal is to tag an album rather than inspect a preview:

1. Keep a backup of the original music.
2. Make copies available as ordinary local files on a desktop computer using
   your normal file-transfer process.
3. Install the appropriate desktop package and follow
   [Your first tagging session](../learn/first-tagging-session.md).
4. Check the saved files and your player's display before replacing any
   originals or transferring the edited copies back.

TagGroove does not make that transfer automatically. A player's own library
or downloaded streaming cache may not expose ordinary editable audio files;
check what your transfer actually produced before trying to tag it.

## If you are evaluating the preview

Read the release notes before downloading and verify the checksum as described
in [Download and install](../getting-started/install.md). Treat any simulator
setup as a developer task. This guide does not promise simulator launch,
physical-device compatibility, native document-picker behavior or save-back.

For a useful issue report, include the exact package filename, release
version, whether the environment is a simulator or a physical device, and the
operation attempted. Distinguish an unpacking/build problem, an application
launch problem and a file-access problem. Do not attach private music or
credentials.

## Related

- [Download and install](../getting-started/install.md)
- [Android document editing and limits](android.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [What's new](../getting-started/whats-new.md)

<!-- Candidate evidence: src-tauri/src/lib.rs runtime_platform (folder_picker_supported only desktop, document_access_supported only Android); .github/workflows/reviewed-release.yml iOS arm64 simulator job; docs/RELEASING.md iOS signing/distribution notes; docs/verification/mobile-ios-simulator.md. No simulator/device launch, signing, document selection or save-back was performed for this draft. -->
