# Context menus and helper menus

A context menu lists commands for the place you opened it: a file-list row,
the tag panel, a cover, an Extended tags field or a format-string input.
Check that context and the current selection before choosing a command.
Removing a name from a suggestion list, removing a tag field and removing
a file from the file list have different results.

!!! tip "New to this?"
    Use [your first tagging session](../../getting-started/first-tagging-session.md)
    to practise selection and saving. [Multi-value fields](../../tagging/concepts/multi-value-fields.md)
    explains how separate values affect the field and cover dialogs.

## Open and leave a menu

Right-click the relevant area. In a focused file-list row or cover window,
++shift+f10++ or the keyboard's **Menu** key opens its command menu. The cover
also has a **⋯** button. Arrow keys move between enabled commands; Escape
closes the popup. A text input generally keeps its normal text-editing
context menu instead of opening the surrounding panel's commands.

The small **►** beside a marked input opens a helper menu. With that input
focused, ++ctrl+space++ opens its helper too, using physical Control on
macOS. On a Mac, ++ctrl+alt+space++ is another route when Control+Space is
claimed by input-source switching. The helper's content depends on whether
the input expects a field name, generating expression, matching pattern,
regular expression or history value.

## File-list row menu

Right-clicking a row outside the existing selection selects that row alone.
Right-clicking an already selected row keeps the selection. Thus a menu
opened on one row can act on several selected files. Right-clicking a column
header opens the column menu instead; see [View](view.md).

| Entry | Task and scope |
| --- | --- |
| **Extended tags…** | Edit the selected files' full exposed field set. |
| **Copy as Text…** | Render selected metadata through the Copy as Text format, rather than copying whole audio files. |
| **Remove tag…** | Remove supported tag data under the current removal policy; inspect the confirmation and actual file errors. |
| **Tools** → configured tool name | Launch that external tool under its own selected-file/first-file settings. If no tools exist, **Configure tools…** opens their settings. |
| **Utils → Remove tag types…** | Choose which supported tag families to remove. This is not the helper's suggestion-list command. |
| **Utils → Strip chapters MP4…** | Remove supported MP4 chapter structures from eligible files. |
| **Utils → Remove unsupported fields…** | Run the format-specific cleanup on eligible selected files. |
| **Utils → Optimize…** | Run the supported format optimization after reviewing its prompt. |
| **Remove from list** | Hide the selected rows from the workspace; keep the audio files on disk. |
| **Play** | Open selected audio through the configured/default external playback route. |
| **Open in Finder**, **Open in Explorer**, or the platform file-manager label | Reveal the files in the platform's file manager. |

Disabled commands retain an explanation, for example pending edits or an
unsupported file/provider operation. Utility names are not universal repairs
for every container. The current row menu does not duplicate every command
from the top menus; use [File](file.md), [Edit](edit.md) or [Convert](convert.md)
when the task is not offered here.

## Tag-panel menu

Choose the tag panel's **⋯** or right-click its non-text background.
Right-clicking a text input keeps the system edit menu; the cover window
has the separate commands below.

| Entry | Effect |
| --- | --- |
| **Customize tag panel…** | Edit the field/control layout and defaults. |
| **Field values…** | Edit stored suggestions for tag-panel fields. |
| **Default values in input fields** | Apply the configured defaults to the selected files as pending edits; review and save them separately. |
| A configured field's name | Show or hide that control on the panel. Hiding a control does not delete its tag field. |
| **Display cover art** | Show or hide the cover section. Embedded images remain in files. |
| **Save configuration…** | Export the panel layout to a configuration file. This is not a backup of the selected files' tags. |
| **Load configuration…** | Read and persist a panel layout. Other ordinary panel settings remain separate. |
| **Import Mp3tag configuration…**, where available | Load supported panel settings into the customization dialog for review; **OK** saves the chosen layout. |
| **Position** choices, where available | Move the panel to an offered dock position. |
| **Hide tag panel** | Hide the whole panel. Use **View → Show tag panel** or ++ctrl+q++ to restore it; physical Control is used on macOS too. |

[Tag-panel options](../options/tag-panel.md) describes control types and
default values. An imported layout may contain unsupported items; inspect
the import report instead of assuming identical appearance across apps.

## Cover-window menu

The cover window summarizes embedded artwork across the selection. When all
selected files have the same ordered covers, it shows one current image and
previous/next controls. Different cover sets produce **Covers vary**, rather
than pretending the first file's image belongs to every file.

| Command | Result |
| --- | --- |
| **Add cover…** | Choose an image to embed in the selected files. The chooser uses the resolved cover-art directory when available. Its fallback order is explained below. |
| **Remove cover** | For equal cover sets, remove the displayed occurrence. For varying cover sets, confirm removing all covers from the selection. |
| **Replace cover…** | Replace the displayed embedded image while retaining its position, picture type and description. It requires a common displayed cover. |
| **Extract cover…** | Save the displayed embedded image as an external image file. This does not remove it from the audio file. |
| **Adjust cover…** | Open the cover-adjustment action to change image format, quality or size. |
| **Manage covers…** | Review a whole cover set, including additions, replacements, reordering and removals. Its edits are written when confirmed in that dialog. |
| **Set cover type** | Choose a predefined picture role for the displayed cover, or for all distinct covers in a varying selection. |
| **Set cover description…** | Set the displayed cover's description, or all distinct descriptions in a varying selection. Empty text clears the description. |
| **Cut cover** | Copy the displayed image to the image clipboard, then remove its displayed occurrence from the selected files. |
| **Copy cover** | Copy the displayed embedded image to the image clipboard. |
| **Paste cover** | Embed an image supplied by the supported image clipboard. It is separate from pasting tag fields. |
| **Previous cover** / **Next cover** | Change which common embedded image is displayed; do not reorder the stored cover set. |
| **View larger…** | Open the displayed cover in a larger viewer; Enter or a double-click also opens it. |
| **Correct aspect ratio** | Keep the image's proportions in the display. Turning it off stretches the preview; it does not resize the embedded image. |

For **Add cover…** and **Replace cover…**, TagGroove first evaluates the
[Cover art directory](../options/directories.md) format and uses it if it
resolves to a real folder. Otherwise it uses the selected file’s folder if
that folder has a supported local image. Next it tries the current working
folder, and falls back to the selected file’s folder when no valid working
folder is available. A configured relative folder is resolved beside the
audio file. This changes where the chooser starts, not which image is
automatically embedded.

The menu has 21 predefined picture roles: Other, File icon, Other file
icon, Front Cover, Back Cover, Leaflet, Media, Lead artist, Artist, Conductor,
Band or orchestra, Composer, Lyricist, Recording location, During recording,
During performance, Screen capture, A bright colored fish, Illustration, Band
logo and Publisher logo. Storage support depends on the audio/tag format.
Choosing a role cannot make a format preserve distinctions it does not store.

The following availability examples assume an editable local selection
and an available image clipboard. A common cover is already saved, and is
the first of two images when navigation is relevant.

<!-- cover-menu-cases:start -->
| Cover state | Add | Remove | Replace | Extract | Copy | Next |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | Yes | No | No | No | No | No |
| `same` | Yes | Yes | Yes | Yes | Yes | Yes |
| `varying` | Yes | Yes | No | No | No | No |
<!-- cover-menu-cases:end -->

A folder image labeled **Not embedded in the file** is a preview fallback,
not an embedded cover. Removing an embedded cover does not delete that
separate image file. Import/embed the image explicitly if it must travel
inside the audio file.

Cover clipboard/chooser operations depend on platform support. Save tags
first when a displayed image is pasted but not written: Copy, Cut and
Extract require the saved image. Review results and file errors after a
change; a visible preview alone does not establish successful save-back.

## Extended tags: copy and paste fields

Right-click a field row outside its text value to open **Cut field**,
**Copy field** and **Paste field** (or a named **Paste ARTIST** command).
Copy/Cut need a valid field name and values that are the same in every
selected file. A mixed field cannot be copied as if one value represented
the whole selection.

The copied field retains separate values. Pasting finds an existing field
with the same field identity and replaces its values, or adds a field row
when it is absent. **Cut field** also marks the original row for removal
in the dialog. These are dialog edits: commit/review them through the
normal Extended tags workflow before treating them as written changes.

This field clipboard is distinct from **Edit → Copy tags** and the image
clipboard. Right-clicking inside a value textarea keeps normal text editing;
it does not silently copy the entire field.

## Field-name helper: suggestions, not tag deletion

The **►** next to a field-name input can offer **Choose field**, including
known fields and special targets valid for that action. Choose a field to
replace the field-name input's current text. The following maintenance
commands edit the stored field-name suggestion list:

| Command | What changes |
| --- | --- |
| **Remove field** | Remove the currently entered field name from the stored suggestions, when it belongs to that list. |
| **Remove all fields…** | After confirmation, clear the stored suggestion list. |
| **Reset fields…** | After confirmation, restore the built-in suggestion list. |

None of these deletes metadata from the selected audio files. A removed
name can still appear because it is already present in selected files,
or because the current control offers it as a special target. To remove
metadata, use a real field-removal operation such as the
[Remove Fields action](../../actions/remove-fields.md).

## Format-string and pattern helpers

A generating-expression helper offers the current visible tag-panel fields
first, then **Extended fields**, **Information fields** and **Functions**
where supported. Choosing a field inserts its `%field%` placeholder at the
caret, replacing selected text. Choosing a function inserts its signature;
replace the argument markers with real inputs before using it. The first
argument is selected after insertion so it can be overwritten immediately.

In these examples `▮` marks the caret, not a literal character to type:

<!-- helper-insertion-examples:start -->
| Before | Insert from helper | After |
| --- | --- | --- |
| `Track: ▮` | `%track%` | `Track: %track%` |
| `%artist% - ▮` | `%title%` | `%artist% - %title%` |
| `▮` | `$num(x,y)` | `$num(x,y)` |
<!-- helper-insertion-examples:end -->

A matching-pattern input, such as Filename → Tag, offers field placeholders
and **DUMMY (skip text)**. It does not offer generating functions or technical
properties that cannot be written as captured tags. Regex inputs instead
offer pattern fragments; replacement inputs offer capture references. A
helper inserts syntax for that context; it does not validate every larger
expression you build around it.

Export-aware inputs additionally offer export and playlist information.
Those placeholders still require their real evaluation context. Inserting
`%_total_files%` into an unrelated ordinary tag expression cannot create an
export input set.

## History and favorite expressions

For inputs with remembered values, **Manage history…** opens an editor with
**Add…**, **Duplicate…**, **Edit…**, **Remove**, move-up/down controls and a
**Use selected** button. An entry has the expression, an optional name and
description, and a **Favorite** flag. Favorites appear directly in the
helper menu; choosing one replaces the entire input value, rather than
inserting it at the caret.

For example, name `%albumartist% - %album%` **Album label**, add a description
explaining its intended use, and mark it Favorite. The helper shows Album
label; choosing it restores the stored expression. The name is a label,
not extra text added to the result. A duplicated entry must have different
expression text before it can be kept. Nonempty names must be distinct
within that history list.

Edits to a history entry or its Favorite checkbox are stored as they are
kept; **Close** leaves the list editor, and **Use selected** copies an entry
into the original input. Changing the history does not run its expression
on the audio selection. The list normally retains up to 25 entries by
dropping the oldest nonfavorites first; an all-favorite list can exceed
that soft limit. The histories belong to individual input contexts, not
one global list of every format string.

Where **Manage history…** is unavailable, **Remove from history** removes
the input's current remembered expression, while **Remove all from history…**
clears that input's list after confirmation. ++shift+del++ removes the current
value from an input's drop-down history. Removing history does not clear tags
and does not rewrite an already exported report.

The filter helper also offers **Auto-apply filter**. Turning it on applies
the current edited filter; with it off, use Enter to apply a pending query.
See [filtering](../../tagging/filter.md) for the file-list effect.

## Related

- [Field names](../field-names.md)
- [Format strings](../../scripting/format-strings.md)
- [All action types](../../actions/index.md)
- [Tag-panel options](../options/tag-panel.md)
- [Keyboard shortcuts](../keyboard-shortcuts.md)

<!-- Source baseline main4e4d5fc: row-context-menu.ts/main.ts rowMenu; tag-panel.ts markup/command; cover-model.ts coverMenu/selectionCovers and covers.ts perform/plan; extended-tags.ts fieldMenu; helper-menu.ts helperEntries/FieldList/FormatHistory/insertion/manageHistory. Canonical menus/{cover,fields,format-strings} facts independently explained with TagGroove differences. No native/browser application launch or save-back claim. -->
