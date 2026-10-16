# Customize the toolbars

The main toolbar provides selected built-in commands. The custom toolbar can
also hold named external tools and action groups. Arrange buttons around the
work you repeat, while keeping the menus available for commands you use less
often. Rearranging a toolbar does not itself change audio tags.

## Show a toolbar or recover hidden buttons

Use **View → Toolbars → Main toolbar** and **Custom toolbar** to show or hide
the complete bars. Those switches preserve their configured items.

The custom toolbar starts enabled but empty. It will not appear until it has
a visible command, tool or action-group item. A list containing only separators
and spacers does not make a useful custom toolbar appear.

If buttons disappeared, check the whole-bar switch first, then open its
customization dialog and check the item visibility. **Hide all** inside the
dialog and hiding the complete toolbar are separate settings. Use the View
menu to recover them; loading a whole old configuration changes much more.

## Practice on the main toolbar

1. Choose **View → Toolbars → Customize main toolbar…**.
2. Select **Save tags** in the list. Use **Rename…** or F4 to give this button
   the short label `Save reviewed tags`. Enter keeps the name; Escape cancels
   the current rename. The command still performs Save tags.
3. Select another item and use **↑ Move up** or **↓ Move down**. Checked items
   are visible in this order. Unchecking an item hides it without removing
   the underlying menu command.
4. Add a **Separator** for a visual division, or a **Spacer** to push following
   items toward the far end of the available toolbar space.
5. Choose **OK** to save the arrangement. **Cancel** closes the dialog without
   saving its draft list. Check the result on screen.

A renamed button keeps its command identity. It does not become a new macro,
change the target fields, alter its shortcut or change when the operation
writes. Hover or focus buttons to read their descriptions in the status area.
A disabled button still follows the command's selection, source and busy-state
rules.

**Reset** in the main-toolbar dialog restores its default item arrangement.
It does not change the separate whole-bar show/hide switch. **Show all** and
**Hide all** affect the dialog's item checkboxes; they are not a reset of
options, files or tag values.

### Main commands available

The default visible command buttons are **Open folder**, **Add files**,
**Save tags**, **Reload**, **Filename → Tag**, **Tag → Filename**, **Actions**
and **Numbering**, with separators between task groups.

The customization list also offers the following built-in commands:

| Task family | Other available buttons |
| --- | --- |
| Recover or remove | Undo, Redo, Remove tag |
| Edit and convert | Extended tags, Filename → Filename, Text File → Tag, Tag → Tag, Quick action |
| Output and inspect | Playlist, Export, Find, Refresh |
| Sources and artwork | MusicBrainz, Discogs, Covers |
| File access and workspace | Show in folder, Play, Tag panel, Filter, Options |

These are toolbar labels, which can be shorter than their menu labels.
**Redo** can invoke the available Repeat behavior when there is no recorded
redo step. The [menu reference](../reference/menus/index.md) and
[action guide](../actions/index.md) explain the actual operations and their
write timing. Showing every button is optional; the toolbar complements the
menus rather than replacing them.

## Build a custom toolbar

1. Create and save any external tool or action group you want to reference.
   Give definitions distinct, recognizable names.
2. Choose **View → Toolbars → Customize custom toolbar…**.
3. In **Add to the custom toolbar**, choose an entry under **Commands**,
   **Tools** or **Action groups**, then choose **Add**.
4. Add the other needed entries. Use visibility checkboxes, move controls,
   **Rename…**, separators and spacers as with the main toolbar.
5. Choose **OK**, then enable **Custom toolbar** under View if it was hidden.
   Review selection and pending edits before running a newly added action.

Adding an item already in this custom list selects and shows its existing
entry; it does not create another copy with the same identity. **Remove**
removes the selected custom-toolbar item. **Clear** empties the draft custom
list. Neither command deletes the original tool, action group or built-in
command. Changes are kept only when the dialog is saved with **OK**.

### Keep labels separate from references

For example, suppose you saved an external tool named `Inspect track` and an
action group named `House#Credit cleanup`. A custom button can have a shorter
label without changing which saved definition it references:

<!-- toolbar-label-cases:start -->
| Saved item identity | Custom button label | What remains unchanged |
| --- | --- | --- |
| `tool:Inspect track` | `Inspect` | The tool definition is still named `Inspect track` |
| `action:House#Credit cleanup` | `Credits` | The action group still has its full saved name |
| `save` | `Save reviewed tags` | The built-in Save tags command and write scope |
<!-- toolbar-label-cases:end -->

The identity strings in this table explain the association; choose entries
through the dialog instead of typing those strings into a metadata field.
Action-group choices display their submenu path using a readable separator.

Tool and action references use the saved name. If that definition is removed
or renamed, an old button can become unavailable with a reason such as
**This tool no longer exists. Edit the custom toolbar.** Remove that stale
item and add the intended current definition. Renaming the button's label
alone does not retarget it. Duplicate tool names are especially confusing
because a named custom button resolves the first matching tool; use distinct
names rather than relying on duplicate labels to select a different program.

A named action-group button runs that group directly, even if its checkbox
in the action library is off. It is different from opening the Actions dialog
to apply the currently checked groups. Review the definition and copied test
files before clicking it. Its work can write immediately.

## Use the customization keyboard controls

Focus the item list first. The same controls work in main and custom
customization dialogs. Primary-modifier shortcuts use Command on macOS and
Control on Windows/Linux. Alt is Option on macOS.

<!-- toolbar-key-cases:start -->
| Key | Result in the item list |
| --- | --- |
| `F4` | Rename the selected item |
| `Space` | Toggle the selected item's visibility |
| `Primary+E` | Show all items |
| `Primary+D` | Hide all items |
| `Alt+ArrowUp` | Move the item up |
| `Alt+ArrowDown` | Move the item down |
| `Alt+PageUp` | Move the item to the beginning |
| `Alt+PageDown` | Move the item to the end |
<!-- toolbar-key-cases:end -->

Plain Up/Down, Home and End change the selected item. Inside the rename text
box, Enter keeps and Escape cancels that edit. Closing the full customization
dialog with Cancel discards its unsaved list changes.

Leading, trailing and adjacent visible separators are omitted from the
rendered bar; a separator near hidden items can therefore disappear. Spacers
remain layout items. Keep names short: saved custom labels have surrounding
whitespace removed and a 60 UTF-16-code-unit limit, so some emoji count as two
units. These are button labels; the saved tool/group names are separate data.

## Keep a known layout

A [configuration backup](configuration.md) includes saved toolbar preferences.
Keep one before substantial reorganization. Main-toolbar Reset and
custom-toolbar Clear are narrower tools than restoring the whole application
configuration. On small screens, available space and layout change; use menus
when a full toolbar would be inconvenient. This does not qualify mobile
external-tool execution or desktop-only file commands.

## Related

- [The main window](../getting-started/main-window.md)
- [External tools](tools.md)
- [Actions and action groups](../actions/index.md)
- [Save and restore configuration](configuration.md)
- [Keyboard shortcuts](../reference/keyboard-shortcuts.md)
