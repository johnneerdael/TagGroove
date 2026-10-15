# Main window layout, Tag Panel docking and Mp3tag tag panel import

Reference pages: `getting-started/main-window`, `customization/tag-panel`,
`customization/options/general`, `menus/view`, and the release notes for the horizontal
Tag Panel, the `_SEPARATOR` field, the customizable toolbars and the Filter docking.

## Implemented

- **Tag Panel position** (`src/panel-dock.ts`, `src/panel-dock.css`, `src/main.ts`). The panel docks
  on the left (default) or right of the File List, above or below it, or floats over the window.
  You can choose the position in three places: View → Tag panel position, the Position items in
  the panel's ⋯ / right-click menu, or by dragging the panel's title bar. While you drag, a preview
  shows where the panel will go. Dropping it near an edge of the workspace docks it there, and
  dropping it anywhere else floats it at that spot. Double-clicking the title bar floats a docked
  panel, or docks a floating panel back on the side it came from. Choosing a position while the
  panel is hidden (Ctrl+Q) shows it again.
- **Horizontal panel** (docked above or below): the fields fill columns as tall as the panel, and the
  cover sits on the right. A `_SEPARATOR` field starts a new column. In a vertical panel it starts a
  new row, as before. Field indexes do not change, so Alt+letter shortcuts, drafts and combo lists
  work in every position.
- **Sizes**: the splitter sets the panel's width (left/right) or height (top/bottom), by dragging or
  with the arrow keys. A floating panel moves by its title bar and resizes with its corner grip. The
  width, height, floating rectangle, last docked side and filter position are saved in the options
  (`pages.layout`), so File → Save/Load configuration carries them. The browser preview keeps a copy
  in local storage, and a width saved by older versions on this device is migrated. A floating
  panel is always kept inside the window, with at least its title bar reachable.
- **Floating** is a pane inside the main window, not a separate OS window, so it never takes focus
  from the main window and never stays on top of other applications. It sits above the File List
  but below menus, drop-down lists and dialogs.
- **Filter position** (View → Filter position): above the File List (default), or left or right of
  the toolbars. F3, the filter count and Auto-apply work the same in every position.
- **Custom Toolbar** (View → Toolbars → Custom toolbar / Customize custom toolbar…): a second
  toolbar next to the main one. It starts empty and takes any toolbar command, any tool from
  Options → Tools and any action group. Tools and action groups are stored by name. If one is
  renamed or deleted, its button is disabled and says so in its tooltip. Buttons show their hint in
  the status bar. Both toolbars can now hold **spacers**, which push the items after them to the far
  end. The main toolbar's command is now labelled Customize main toolbar…, and its id and keys are
  unchanged.
- **Mp3tag tag panel import** (`src-tauri/src/mp3tag_tag_panel.rs`): Customize tag panel → Import
  Mp3tag configuration…, or the panel menu entry of the same name, reads a file the user picks:
  `usrfields.ini` from their Mp3tag configuration folder, or a configuration they saved there.
  OpenTag ships no Mp3tag files. The import reads the file's `[#n]` sections in order and maps
  these keys: `name` to the label (with its `&` shortcut), `value` to the field (`_SEPARATOR` to a
  separator, `_DIRECTORY` to the folder switcher), `enabled` to visibility, `multiline` (0 text, 1
  multiline, 2 checkbox) and `rows` to the control, `default` to the default value, and `size` to
  the 12-column size. With `version=1`, sizes 2/3/4/6/8/9/12 map to Tiny/Smaller/Small/Normal/
  Medium/Large/Full. Older files use 1–4 for Full/Normal/Small/Smaller. The file is decoded as
  UTF-16 with a BOM, UTF-8, or Windows-1252. The imported fields replace the list in the dialog for
  review, and nothing is saved until OK. The dialog lists every entry it left out, with the reason:
  technical fields other than `_DIRECTORY`/`_SEPARATOR`, duplicates, invalid names or text, more
  than 100 fields, and the legacy per-field `custom_values` and `prefer_tag` settings (OpenTag keeps
  those in Field values). The cover setting and panel visibility are not in the file and stay as
  they are.
- **Options → General → Notification area** (`src-tauri/src/tray.rs`): *Always show icon in
  notification area* and *Minimize to notification area*. Both are off by default and only apply
  on Windows and Linux. Minimizing then hides the window and its task bar entry. Clicking the icon,
  or choosing Show OpenTag in its menu, restores the window (Linux status areas only open the
  menu). Quit OpenTag closes the window through the normal close request, so unsaved changes are
  still confirmed. On Linux, the app checks that libayatana-appindicator3 (or libappindicator3) can
  be loaded before it creates the icon. If neither loads, it reports an error and does not hide the
  window. The window is hidden only after the icon exists, so it can always be brought back. On
  macOS the two options are disabled (the Dock keeps minimized windows) and the command reports no
  support.
- **Light and dark** color modes use the existing tokens for every position, the dock preview, the
  floating panel's frame and the toolbar row.

## Tests

- `tests/panel-dock.test.mjs` (9 tests): layout normalization and the legacy width, dock toggling and
  the remembered side, keeping a floating panel reachable, drop targets at the edges and corners,
  splitter growth and limits per side, column filling with separators and tall fields, the tag panel
  rendering columns while keeping field indexes, the panel menu's position and import entries, and
  main/custom toolbar normalization (spacers, tools and action groups by name, hints).
- `src-tauri/src/mp3tag_tag_panel.rs` (3 tests): section order, labels, sizes, types, rows, defaults,
  hidden fields and skipped entries; legacy sizes and the reported custom values; UTF-16 decoding
  and files without fields. `src-tauri/src/tray.rs` (1 test): when the icon exists and when a
  minimize hides the window.
- Native e2e `tests/e2e/panel-dock.js` (fixtures: `tests/e2e/panel-dock-fixtures.sh`, a hand-written
  UTF-16 `usrfields.ini`) runs headless in CI on Linux, macOS and Windows. It covers:
  - the default left dock and the View submenus;
  - right dock geometry, the splitter with the arrow keys, and the saved layout;
  - the top dock's columns, cover placement and editing, and height resizing;
  - the bottom dock;
  - dragging the title bar to float the panel (fixed position, resizable, focus kept in the
    window), moving it, double-click re-docking, and dragging to the left edge;
  - the filter right and left of the toolbars, with filtering and F3;
  - a custom toolbar with a command, a spacer and an Options → Tools entry (hint, saved settings,
    show/hide);
  - the Mp3tag import mapping and its report;
  - the notification area options (off by default, disabled on macOS, icon created and removed on
    Windows/Linux);
  - text contrast in light and dark for the right, top and floating positions;
  - Ctrl+Q with a horizontal dock.

## Decisions

- Floating stays inside the main window. A separate OS window would need the whole tag panel state
  (selection, drafts, combo lists) synchronized between two webviews, and it could take focus or
  stay on top, which the requirements rule out.
- Docking positions apply to the desktop layout (wider than 1000 px). The compact layout keeps its
  Files/Tags switcher, and the filter stays above the list there.
- The Mp3tag import goes through the Customize dialog, so users review the mapped fields before
  anything is saved. Mp3tag's own configuration folder is never searched automatically.
- Minimize to tray is not offered on macOS. A menu-bar extra that hides the Dock icon would go
  against platform conventions for a document editor.

## Known gaps vs Mp3tag

- Floating does not create a separate OS window, so the panel cannot be moved outside the main
  window or onto another screen.
- Mp3tag's toolbars can be rearranged freely as bands. OpenTag shows the main toolbar, then the
  custom toolbar, with the filter docked on either side of them.
- Custom toolbar buttons show text labels. Choosing other symbols, and buttons for individual tag
  sources, are not implemented (the main toolbar's MusicBrainz and Discogs commands can be added).
- The notification area icon cannot be tested for clicks headlessly. CI covers creating and
  removing it, and unit tests cover the minimize logic; hiding on minimize depends on the window
  manager reporting the minimized state.
- `toolbar.ini` / `toolbar_custom.ini` from Mp3tag are not imported.
