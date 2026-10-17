# General options

Open **File → Options… → General** to control everyday selection, sorting, and typing behavior. These settings affect the editor rather than the contents of saved tags.

!!! tip "New to this?"
    Leave [automatic saving](tags.md) off until the difference between selecting a file, editing a field, and saving tags is familiar. Automatic selection and automatic saving are separate settings.

## General

| Setting | Default | Effect and when to use it |
|---|---|---|
| **Show current directory in window title** | On | Add the working folder to the title, for example `TagGroove — /Music/Juniper Vale`. Useful with several windows or similar album folders. With no folder loaded, the title remains `TagGroove`. |
| **Automatically select next file when editing in file view** | Off | After committing an edit in a file-list cell, move the selection to the next file. Enable for entering a sequence of titles down a column; disable when several fields on one file need attention. |
| **Auto-completion at selection boxes** | On | Complete typed prefixes using available values in tag-panel fields and file-list cells. The added suffix is selected, so further typing replaces it. Disable if suggestions interfere with free typing. |
| **Use natural sorting (1, 2, 11, …)** | On | Treat digit sequences numerically: `Track 2` precedes `Track 11`. With it off, textual sorting can place `Track 11` before `Track 2`. This changes list order, not track fields or filenames. |
| **Select files automatically when they are added to the list** | Off | Select incoming files after adding them. Useful for adding one album and immediately editing it; check the resulting selection before a batch command. |
| **Keep moved files in the file list** | On | Retain files moved outside a listed folder. With it off, those files leave the list after a move unless they have pending changes. This does not delete the moved files. |

Completion matches prefixes without regard to case and retains the characters already typed. If a candidate is `Juniper Vale` and the typed text is `ju`, the suggested text starts `ju` and completes the rest. Empty input is not filled automatically, and pasting does not trigger this inline completion. The candidates depend on the field's [custom value lists](field-values.md) and current selection.

Even with automatic selection off, loading files into an empty, unselected list selects an initial file for inspection. The setting controls selecting the incoming batch, not a guarantee that the list will have no selected row.

## Notification area

Both controls default to off. **Always show icon in notification area** keeps a tray icon available. **Minimize to notification area** hides the minimized window and its taskbar entry; click the tray icon or choose **Show TagGroove** from its menu to return.

These controls are for Windows and Linux. Linux requires notification-area support, including `libayatana-appindicator`. On macOS the controls are unavailable; minimized windows use the Dock. Do not look for a Windows-style tray setting to recover a minimized Mac window.

## Related

- [Tags and saving](tags.md)
- [Field values and completion choices](field-values.md)
- [Directories](directories.md)

<!-- verified: src/options-ui.ts GENERAL_DEFAULTS, defaultSettings, naturalCollator, windowTitle; src/general-options.ts completion, completeInput, insideFolders; src/main.ts generalPrefs, applyNotificationArea; src/cell-editor.ts. -->
