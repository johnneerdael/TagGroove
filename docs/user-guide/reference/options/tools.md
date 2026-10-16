# Tools options

Open **File → Options… → Tools** to configure external programs for selected local audio files. Saved tools appear in the Tools menu and file-list context menu. The default tool can also run on a file double-click.

!!! tip "New to this?"
    Start with a player or inspection program and one copied file. A tool is a separate application: it can change files independently of OpenTag, so its changes are not automatically an OpenTag undo step.

## Add, edit, and reorder

**New** adds a tool; select it to edit its fields. **Remove**, **Move up**, and **Move down** manage the list. At most **99 tools** are allowed. The initial list is empty.

The first ten entries have ++cmd+1++ through ++cmd+0++ on macOS, or ++ctrl+1++ through ++ctrl+0++ on Windows/Linux. The tenth entry uses 0. Reordering changes the corresponding shortcut positions. Shortcuts do not run a tool while typing into an ordinary text field.

## Name, Path, and Parameter

| Field | Use |
|---|---|
| **Name** | Nonempty label shown in the menus. |
| **Path** | Full path of the executable. Use **Browse…** to select it. A command name found only through a shell's search path is insufficient. On macOS an application bundle path ending in `.app` is also accepted. |
| **Parameter** | Optional argument text containing supported placeholders. Empty parameters pass the selected file's full path as one argument. |

For a program that accepts a file path, use:

```text
"%_path%"
```

Arguments are separated by whitespace, and double quotes keep an argument together. An unmatched quote is an error. Placeholder values are inserted into the parsed arguments; a file path containing spaces remains one argument when the placeholder occupies one quoted argument.

OpenTag starts the program directly, without a shell. Redirection, pipes, command substitution, and shell wildcard expansion are not performed. Enter the executable in **Path** and its arguments in **Parameter**, rather than a complete shell command in either field.

## Placeholders

For a file `/Music/Juniper Vale/Harbour Lights/03 - Tidepool.flac`, a single listed root `/Music`, and its containing folder:

| Placeholder | Meaning | Illustrative value |
|---|---|---|
| `%_path%` | Complete file path. | `/Music/Juniper Vale/Harbour Lights/03 - Tidepool.flac` |
| `%_filename_ext%` | Filename including extension. | `03 - Tidepool.flac` |
| `%_filename%` | Filename without extension. | `03 - Tidepool` |
| `%_filename_rel%` | File path relative to the list root, where available. | `Juniper Vale/Harbour Lights/03 - Tidepool.flac` |
| `%_directory%` | Containing folder's name. | `Harbour Lights` |
| `%_folderpath%` | Complete containing folder path, with trailing separator. | `/Music/Juniper Vale/Harbour Lights/` |
| `%_folderpath_rel%` | Relative containing folder path, with trailing separator. | `Juniper Vale/Harbour Lights/` |

Relative placeholders depend on the root context. With several roots or a file outside the chosen root, do not assume the same relative result; use `%_path%` when the external program needs an unambiguous file location. Path separators follow the platform.

## For all selected files

A new tool enables **for all selected files (otherwise only the first selected file)**. Enabled means launch one process per selected file. Disabled means run once for the first selected file; it does not collect every selected path into one argument list.

OpenTag asks before launching a per-file tool for more than ten files. The backend limits one per-file invocation request to 100 selected files. Save pending changes first so an external program reads the intended on-disk data. Tool availability is limited to supported local-file contexts; a provider-backed document is not automatically an executable filesystem path.

## Default tool

**Default tool (double-click in the file list)** defaults to **No action**. Select a configured tool to launch it on a file double-click. **No action** disables the external-tool action; it does not remove explicit tools from menus. OpenTag’s file-list double-click falls back to **Extended tags…** when no default tool is selected. This differs from Mp3tag’s documented no-action behavior.

If another program modifies tags, use **Reload tags** to read its changes. Keep the [Library](library.md) timestamp/size cache rules in mind when an external editor preserves those attributes.

## Related

- [General selection behavior](general.md)
- [Library and external changes](library.md)
- [Messages](messages.md)

<!-- verified: baseline007a806 src/tools.ts TOOL_LIMIT, PLACEHOLDERS, toolsOptionsPage, ToolsUI.run; src-tauri/src/utilities.rs split_parameters, tool_placeholders, tool_arguments, run_tool; src/main.ts ToolsUI host and context menu. -->
