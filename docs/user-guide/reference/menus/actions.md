# Actions menu

Use **Actions** to apply a reusable sequence or run one action immediately. An
action can change tags, paths or external files; inspect its settings on copied
practice files before using a large selection.

!!! tip "New to this?"
    [Build your first action group](../../learn/first-action-group.md) teaches
    the difference between saving a workflow and applying it. The
    [action reference](../../actions/index.md) lists all current types.

## Menu entries

| Entry | Shortcut | What it does |
| --- | --- | --- |
| **Actions…** | ++alt+6++ | Open the group library to create, edit, arrange, import, export, save or apply groups. Opening the library does not itself run a group. |
| **Actions (Quick)…** | ++alt+shift+6++ | Configure one action and choose **Apply action** to run it on the current selection, saving immediately. No enclosing group is required. |
| Saved group name | Its configured mnemonic, if any | Run only that saved group on the current selection. Its checkbox state in the library does not prevent this explicit run. |

Alt is ⌥ Option on macOS. On that platform, Option combinations used to type
characters in a text field retain their text-entry behavior; use the menu when
the shortcut is not appropriate to the current focus.

The library can be opened to organize groups even without a selected file.
Applying a group or Quick action requires an eligible selection. Save pending
edits first; provider documents can limit path-dependent actions. Read a
disabled command's explanation instead of assuming it has already run.

## Quick settings are remembered

OpenTag remembers the last Quick action type and that type's last applied
settings. Switching types can recall a different saved configuration. For
example, returning to **Replace** can restore an earlier search/replacement
instead of supplying empty controls. Check every field before applying.

Quick settings are not a named, ordered workflow. To repeat several steps
reliably, put them in a saved action group. The Quick dialog's **Cancel**
closes without applying the current action.

## Checked groups and a direct menu run

In **Actions…**, **Apply checked groups** runs the checked groups in library
order, and each group's actions in their displayed order. Selection highlight
and the execution checkboxes have different jobs: highlighting chooses what
to edit; checking chooses what a multi-group run executes.

A saved group's own menu item runs that group alone. For instance, if
**Clean spacing** is unchecked and **Build filenames** is checked, choosing
**Clean spacing** directly still runs Clean spacing. It does not also run
Build filenames. Use the group library when both steps must run in order.

## Submenus and keyboard mnemonics

A `#` in a group name separates menu levels. An `&` marks the next letter as
its mnemonic; `&&` means a literal ampersand. For example, a saved name
`Cleanup#&Space titles` appears under **Cleanup** as **Space titles**.

Press ++alt+a++ to open the Actions menu, then the marked letters for the
required level and group. Use distinct mnemonic letters within one level;
the current implementation chooses the first matching group when names
reuse the same letter. A submenu can also be opened with the pointer.

The stored full group name remains significant for references such as a
custom toolbar item. A displayed menu label is not a format string and
does not evaluate `%artist%` or scripting functions.

## If the result is unexpected

- Recheck which files are selected and whether another filter hid intended
  files before the action was started.
- Check group/action order and the current Quick settings. A later action
  sees the earlier action's result.
- Inspect the file errors and the available **Edit → Undo** operation.
  Export Cover records its created and replaced images for undo; reports,
  playlists and other external outputs have their own cleanup rules.
- Save group edits with **Save groups** when the workflow must survive
  reopening the app; editing a group is separate from changing audio tags.

## Related

- [Action groups and all action types](../../actions/index.md)
- [Build your first action group](../../learn/first-action-group.md)
- [Keyboard shortcuts](../keyboard-shortcuts.md)
- [Convert menu](convert.md)

<!-- Source baseline main4e4d5fc: src/main.ts menu/enabled/operationReason; src/action-ui.ts menuHTML/runOne/mnemonic/recallQuick/groupsDialog. Canonical menus/actions topics independently explained; OpenTag's remembered Quick state and library-without-selection behavior are explicit differences. No native UI run. -->
