# Appearance options

Open **File → Options… → Appearance** to change color mode, control size, and separation between file-list rows.

!!! tip "New to this?"
    Enlarge the whole interface when text is hard to read. Enlarge only the toolbar when its buttons are the problem.

## Color mode

**System** is the default and follows the operating system's light/dark preference. **Light** and **Dark** fix the app to the selected mode. The title-bar color-mode control has the same preference; it is remembered on the device.

The setting changes presentation, not artwork or exported files. Mp3tag's documented Windows-version restriction is not an OpenTag requirement; OpenTag uses its own interface theme.

## Symbols

OpenTag does not provide Mp3tag's **Standard** versus **Original** icon-set switch. Its toolbar uses OpenTag's own symbols. Choosing a color mode or changing scale does not load a historical Mp3tag icon set.

## Toolbar scaling

**Toolbar scaling** defaults to **100%**. Available values are **100%**, **110%**, **125%**, **150%**, **175%**, and **200%**. Increase it to enlarge toolbar icons and buttons without enlarging the rest of the interface.

## User interface scaling

**User interface scaling** has the same choices and defaults to **100%**. It zooms the whole interface, including the toolbar. Toolbar scaling remains an additional toolbar-specific factor, so raising both makes toolbar controls grow more than other controls. If the toolbar becomes too large, return its own scale to 100% before adjusting the overall scale.

## Display grid lines in file view

| Choice | Effect |
|---|---|
| **Off** | Default. No added grid or alternating row treatment. |
| **Alternate row colors** | Shade adjacent rows differently to help follow a wide row. |
| **Grid lines** | Draw boundaries between file-list cells. Useful with many columns. |

OpenTag uses explicit choices. Mp3tag's three-state checkbox encodes these appearances differently; a grayed checkbox in that application is not an extra OpenTag mode.

## Related

- [General options](general.md)
- [Options overview](index.md)
- [Tag-panel layout](tag-panel.md)

<!-- verified: src/options-ui.ts APPEARANCE_DEFAULTS, SCALES, appearance page; src/general-options.ts appearanceOptions; src/main.ts applyAppearancePrefs; src/theme.ts. No icon-set switch registered. -->
