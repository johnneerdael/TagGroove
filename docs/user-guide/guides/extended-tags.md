# Extended tags

Use **Extended tags…** to see fields that are not on the tag panel, edit
separate values and work with cover art. Open it through **View → Extended
tags…** or ++alt+t++ with files selected. Alt is ⌥ Option on macOS.

For one file, the heading identifies its filename and available tag type.
For a selection, the heading gives the number of files. A field's values
can contain several lines without becoming several separate field values.

## Add a practice field and save it

1. Select one copied practice file and open **Extended tags…**.
2. Choose **＋ Add field**.
3. Enter `COMMENT` as the field name, or choose it through the **►** helper.
   Do not add a second COMMENT row if one already exists; edit that row.
4. Choose **＋ Add value**, then type `Checked for the catalogue` in its
   value box.
5. Choose **Keep changes**. Back in the main window, the change is pending.
6. Choose **File → Save tags** (++cmd+s++ / ++ctrl+s++) to write it.
7. Reopen Extended tags and confirm the saved value.

**Keep changes** stages text-field edits; it does not itself write all tags.
Until saving, **Edit → Discard pending changes** can discard staged field
edits. Keep an original copy and inspect the actual errors after a save.

## Read a selection without overwriting differences

Suppose two selected files have these values:

| Field | Paper Boats | North Pier | Dialog state |
| --- | --- | --- | --- |
| `ALBUM` | `Harbour Lights` | `Harbour Lights` | One shared value is shown |
| `TITLE` | `Paper Boats` | `North Pier` | **Mixed values — keep existing values** |
| `ARTIST` | `Juniper Vale` plus `Mira Solberg` | `Juniper Vale` | Mixed separate-value lists |
| `COMMENT` | `Reviewed` | Absent | Mixed, because presence differs too |

An untouched mixed row preserves every file's own values. It is not a
request to clear the field. Editing its values instead supplies one new
value list to every selected file. For example, entering one `ALBUMARTIST`
value can deliberately standardize the selection, while editing TITLE
would replace each track's distinct title.

A mixed state compares whole value lists, including their order. Two files
with the same names in a different value order can therefore be mixed.
Choose a smaller selection to investigate a difference rather than typing
over it merely to make the indication disappear.

## Separate values and multiline text

Use **＋ Add value** to add another value to a field. Each value has its own
textarea and remove **×** control. Enter within a textarea starts another
line in that value. Two lines in one lyrics/comment value are different
from two ARTIST values.

With **Options → General → Auto-completion at selection boxes** enabled,
typing shows matching whole values from the field's custom/loaded values.
No suffix is inserted in these multiline boxes. Use Up/Down to highlight a
choice, then Enter to replace the whole value; the choice changes only this
dialog's edit, without saving tags or pressing **Keep changes**. Enter with
no highlighted choice, including after a choice was accepted, inserts a
newline as usual. **Alt+Down** opens matching choices explicitly, **Esc**
closes them first, and Tab moves focus without accepting a highlighted
alternative. Suggestions do not restrict free text and pause during
input-method composition.

For a duet credit, keep `Juniper Vale` and `Mira Solberg` as two value boxes
when the format/workflow supports separate artists. The
[multi-value explanation](../concepts/multi-value-fields.md) shows why typing
a semicolon into one box does not necessarily create two stored values.

**Remove field** removes the whole row from the intended result. Removing
all its value boxes also requests field removal. A remaining value box
containing empty text is a different input; the target tag format determines
how an empty text value survives saving. Inspect the saved result.

## Rename a field while retaining each file's values

Edit the **FIELD** name without changing its value boxes. A name-only edit
uses each file's original values, even when the row is mixed. For example,
rename a custom `SESSION_NOTE` field to `SESSIONNOTES`:

<!-- extended-rename-example:start -->
| File | `SESSION_NOTE` before | Rename to | `SESSIONNOTES` after |
| --- | --- | --- | --- |
| `01.flac` | `Early take` | `SESSIONNOTES` | `Early take` |
| `02.flac` | `Final take` | `SESSIONNOTES` | `Final take` |
<!-- extended-rename-example:end -->

The old field is removed from each file, and its own value becomes the new
field's value. If the values are edited as well, the edited list replaces
the originals instead. Do not rename into a field already listed in the
dialog: duplicate field identities are rejected for review.

Field identities use case-insensitive ASCII names, so a second `artist`
row is not another independent field beside ARTIST. Native field spelling
can remain specific to each file. [Field names](../reference/field-names.md)
and [mapping](../reference/tag-mapping.md) explain how names reach tag formats;
a visible custom name is not a promise that every player will display it.

## Copy, cut and paste fields

Right-click a field row outside a value textarea, or focus the row and use
++shift+f10++. **Copy field** retains the field name and its separate values
in TagGroove's field clipboard. **Cut field** additionally marks the original
row for removal. Paste replaces a matching field's values, or adds a row
when the field is absent.

Copy/Cut are unavailable for a mixed row because no single common value
list represents all selected files. Inspect one file or make an intentional
common value first. The field clipboard is separate from copying all tags,
ordinary selected text and image clipboard contents. See
[context menus](../reference/menus/context-menus.md) for the controls.

## Step through individual files

The **<<** and **>>** buttons appear when one file is being edited. They move
to the previous/next row in current file-list order and change the main
selection to that file. A filtered or sorted view changes that sequence;
the buttons are disabled at its ends.

The treatment of changed fields follows
[Options → Tags → Save mode at extended tag dialog](../reference/options/tags.md):

| Mode | When navigating away from a changed file |
| --- | --- |
| **Keep changes pending** | Default. Stage the current field edits and move to the neighboring file. Save them later from the main window. |
| **Ask** | Choose **Save**, **Don't save**, or cancel the question to stay on the current file. |
| **Ignore changes** | Leave the current dialog edits unapplied and move on. |
| **Save changes** | Stage and write the current changed fields before moving; a reported save error keeps the issue visible. |

The separate **Save tags when using arrow-keys/single mouse click** preference
can also write pending changes as selection changes. Check both settings
before relying on navigation as a read-only inspection workflow.

Closing the dialog does not undo edits already kept by an earlier navigation
or written by a save mode. To discard unsaved work, inspect the main window's
pending changes. To reverse a recorded disk operation, inspect its **Undo**
label. Neither is a substitute for an original copy.

## Covers use a different confirmation path

The cover section has **Add cover…**, **Remove cover**, **Extract cover…**
and its **⋯**/right-click menu. It uses the same selection summary as the tag
panel. Cover actions can write immediately; field **Keep changes** is not a
general confirmation or rollback for artwork operations.

[Working with covers](covers.md) explains direct cover commands versus the
**Manage covers…** dialog's **OK** write step. Supported image-file drops
onto the dialog's cover area target that area; dropping elsewhere can have
a different meaning. Extraction creates an external image instead of
removing the embedded one.

## Gotchas

- Confirm the heading and selected-file count before replacing a shared field.
- Leave mixed rows untouched to preserve differences; a blank-looking mixed
  indication is not an empty text value to fill automatically.
- A field rename and a value replacement have distinct effects. Start with
  copied files when reorganizing custom field names.
- A formula typed into a value textarea is ordinary metadata text; this
  dialog is not the Tag → Tag format-string evaluator.
- Unsupported tag fields, provider access or format limits can prevent a
  later write. Check actual saved tags and errors, not only the dialog view.

## Related

- [Multi-value fields](../concepts/multi-value-fields.md)
- [Working with covers](covers.md)
- [Tag options and save modes](../reference/options/tags.md)
- [Field names](../reference/field-names.md)
- [Context and helper menus](../reference/menus/context-menus.md)

<!-- Source baseline main4e4d5fc: extended-tags.ts show/fill/addRow/commit/navigate/askSave; tag-edits.ts selectionTagRows/resolveTagEdits/neighbourFile; main.ts ExtendedTags host; list-behaviour.ts tagsPagePrefs and save_on_move. Canonical extended-tags facts independently taught, with explicit pending-field/immediate-cover distinction and actual textarea controls. No native application launch/save-back claim. -->
