# Directories options

Open **File → Options… → Directories** for folder scanning and the starting folder of the cover-image picker. Its **Startup** child page controls the fallback folder when there is no remembered local file list or explicit launch request.

<!-- figure:options-directories -->
<figure markdown>
![The directories page of the Options dialog](../../assets/screens/options-directories-light.png#only-light)
![The directories page of the Options dialog](../../assets/screens/options-directories-dark.png#only-dark)
</figure>
<!-- /figure -->

!!! tip "New to this?"
    Start with one album folder. Enabling subfolders for a music-library root can add many more files than expected; the [Files](files.md) extension restriction can narrow the scan.

## Subfolders

**Read subfolders** defaults to on. Folder operations then include nested folders. Turn it off to work only with audio directly inside the selected folder. It controls discovery, not whether an already loaded file can be edited.

## Favorite directory

TagGroove keeps an ordered list of favorites through **File → Favorite directories…** rather than a single favorite-path input on this page. Add the folders used frequently and put the preferred one first. **File → Favorite directory** or ++ctrl+m++ opens the first favorite. Control is used on every platform, including macOS.

Managing favorites is separate from choosing a startup mode. Adding a favorite does not automatically load it every time the app starts.

## Startup directory

Choose **Directories → Startup → Startup directory**:

| Choice | Result |
|---|---|
| **Empty file list** | Default fallback. Do not reopen a folder when there is no remembered list. |
| **Previous directory** | Open the most recent folder from the saved directory history. |
| **First favorite directory** | Open the first folder in the favorites list. |

If there is no matching recent/favorite entry, there is no folder to restore. Files or directories supplied as launch requests take precedence over a remembered list and the normal startup folder. The local desktop app otherwise reopens its remembered file list and order; see [The file list](../../tagging/file-list.md). Startup restoration also depends on folder access being available on the platform; a file-picker-only environment cannot provide the same folder workflow.

## Cover art directory

**Default folder for Add cover** defaults to empty. It sets the starting location of the **Add cover…** picker; it does not automatically import or embed images.

Accepted forms include a full path, a path relative to the selected audio file's folder, and a format string using its fields. For example, `Artwork` points to an `Artwork` subfolder of the selected file's folder; `%artist%` can supply a component of a configured image archive path.

If the configured location is empty or cannot be opened, TagGroove tries the first selected file's folder when that folder contains an image. Otherwise it uses a valid working folder, falling back to the selected file’s folder when no valid working folder is available. Select the intended file before opening the picker, since the initial location derives from the selection.

## Related

- [Files and extension restrictions](files.md)
- [General selection settings](general.md)
- [Tags and folder-image display](tags.md)

<!-- verified: src/options-ui.ts directories controls; src/file-ops.ts startupOptionsPage, startupChoice, FileOpsUI.initialize; src/main.ts CoverUI.startDirectory; src-tauri/src cover_start_directory command. -->
