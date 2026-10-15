# Options reference

Open **File → Options…** with ++cmd+o++ / ++ctrl+o++ to change how OpenTag reads files, presents fields, writes tags, and contacts online services. These pages explain each setting and the consequences of changing it.

!!! tip "New to this?"
    Start with [General](general.md), [Files](files.md), and [Tags](tags.md). Keep automatic saving disabled while learning: selecting another file then leaves a chance to review pending edits before writing them.

## Choose and save a page

Select a page in the tree on the left. The arrow keys move between pages; Home and End select the first and last pages. On a narrow window the tree becomes an **Options page** list. Moving between pages retains the changes being edited.

**OK** validates and saves the settings across pages. If a value is invalid, OpenTag selects the affected page and shows the problem. **Cancel** or Escape closes the dialog without saving its ordinary settings. **Restore page defaults** resets the controls of the selected page in the dialog; press **OK** to save the reset. It does not reset every page at once.

Some buttons open an independent editor or carry out an operation immediately. In particular, Tag Panel and Field Values save through their own **OK** buttons; Library cleanup/clear, building a local freedb index, and exporting a configuration or value list perform work when clicked. Canceling the outer Options dialog does not undo that work.

Changing tag read options can change which metadata is displayed. On **OK**, OpenTag reloads loaded local files that have no pending edits when tag settings change. Files with pending edits retain those edits; save or discard them before reloading. Changing settings alone is not a request to rewrite the music library.

## Pages

| Page | Configure |
|---|---|
| [General](general.md) | Sorting, selection, completion, moved files, and notification area. |
| [Messages](messages.md) | Confirmations, completion messages, and cover preservation. |
| [Files](files.md) | Folder scan extensions, safe names, and modification timestamps. |
| [Directories](directories.md) | Subfolders, favorites, startup, and image-picker location. |
| [Appearance](appearance.md) | Color mode, scaling, and row/grid display. |
| [Library](library.md) | Persistent metadata cache and maintenance. |
| Language | Not available in this guide’s source baseline; the language reference is held until integration. |
| [Tags](tags.md) | Selection-change saving, Extended tags, and folder images. |
| [Tags → Mpeg](tags-mpeg.md) | ID3v1, ID3v2, and APE policies for MPEG-family files. |
| [Tags → Ape, Mpc](tags-ape-mpc.md) | Tag policies for APE-family files. |
| [Tags → Mapping](tags-mapping.md) | Field names shared across tag formats. |
| [Tags → Advanced](tags-advanced.md) | Ratings, chapters, comment language, and storage compatibility. |
| [Tag Panel](tag-panel.md) | Fields, layout, and default editing values. |
| [Field Values](field-values.md) | Custom choices and genres. |
| [Networking](networking.md) | Request timeout and proxy. |
| [Updates](updates.md) | Version checks, beta releases, and release notes. |
| [Tag Sources](tag-sources.md) | Cover output, MusicBrainz sizes, and source diagnostics. |
| [Tag Sources → freedb](tag-sources-freedb.md) | Remote CDDB servers and disc IDs. |
| [Tag Sources → Local freedb](tag-sources-local-freedb.md) | Historical local databases and search indexes. |
| [Export](export.md) | Export dialogs, folders, and byte-order marks. |
| [Playlist](playlist.md) | Paths, naming, encoding, and automatic playlists. |
| [Tools](tools.md) | External programs and double-click behavior. |

## Where settings live

Use **File → Open configuration folder** to locate the active desktop profile. Ordinary options are versioned JSON in the app's configuration directory; other editors have their own configuration files. Color mode is remembered on the device separately. Use the app's configuration save/load commands for supported backups rather than guessing a platform-specific path or editing a running app's files.

The reference describes settings found in the guide’s source baseline. Availability in a downloaded build depends on which features that build includes; a source check does not establish release availability or runtime parity with Mp3tag. A missing page should be checked against the installed build before following instructions for it.

## Related

- [General](general.md)
- [Tags](tags.md)
- [Library](library.md)

<!-- verified: src/options-ui.ts OptionsUI.open, validateOptions, pageTree; src/main.ts optionsApplied; src-tauri/src/settings.rs Settings. Source inspection, not runtime verification. -->
