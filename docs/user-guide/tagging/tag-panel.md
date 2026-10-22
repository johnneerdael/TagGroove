# Edit tags in the tag panel

The tag panel shows editable fields for the selected files. Use it to set
shared album information while keeping each track's own title and number.
Edits are pending until saved, unless an enabled selection-change save
preference writes them when moving on.

![Tag panel with a mixed selection](../assets/screens/tag-panel-keep-light.png#only-light)
![Tag panel with a mixed selection](../assets/screens/tag-panel-keep-dark.png#only-dark)

The screenshot illustrates the keep-existing-values state. Layout and visible
controls depend on the saved panel configuration.

## Set one album for several tracks

1. Select two copied tracks with different titles, `Paper Boats` and
   `North Pier`.
2. Check that the panel's selected-file count is 2. Leave **Title** unchanged.
3. Enter `Harbour Lights` in **Album**.
4. Inspect the list: both rows show the album, while their titles stay distinct.
5. Choose **File → Save tags** (++cmd+s++ / ++ctrl+s++), then check any errors.

Typing one value into a field applies that edit to every selected file.
Leaving a mixed field alone preserves each file's own values; it does not
copy the first track's title into the others.

## Read the mixed-value state

A shared value is displayed normally. When selected files differ, a text
control is empty with the `< keep >` placeholder. That placeholder describes
an instruction to keep existing values; it is not a missing-title diagnosis
for the whole selection.

Use the field's **▾** button to choose a value. A multi-file selection includes
`< keep >`; `< blank >` requests clearing the field. The list also includes
distinct values from the selection, configured custom values and, for Genre,
standard genres unless disabled in Field Values options.

<!-- panel-choice-examples:start -->
| Choice or typed text | Resulting value list |
|---|---|
| Choose `< keep >` | Keep each file's loaded values |
| Choose `< blank >` | Field removed |
| Type `Juniper Vale \\ Mira Solberg` | Two values: `Juniper Vale`, `Mira Solberg` |
| Type `AC\DC` | One value: `AC\DC` |
| Type `%artist%` | One value: `%artist%` |
<!-- panel-choice-examples:end -->

Choose the actual dropdown entry for Keep or Blank; typing its displayed
label is not the same dropdown action. Choosing Keep also abandons a pending
edit to that field and restores each selected file's loaded values. It does
not clear unrelated pending fields.

Two consecutive backslashes separate values in panel input. The pieces are
trimmed; a single backslash is ordinary text. Avoid repeated/trailing
separators: this panel parser can retain empty pieces. It differs from
[Format Value](../actions/format-value.md), whose delimited metadata output
drops empty pieces. Use [Extended tags](extended-tags.md) to inspect and edit
the individual stored values directly.

## Choose and remember suggestions

Suggestions help keep spelling consistent; they do not restrict a field to
the list. Type a new artist, genre or composer normally when needed.
**Options → General → Auto-completion at selection boxes** enables inline
completion and an automatic list of all matching values. Filtering uses the
prefix typed before the inline suffix was added. The automatic list contains
values grouped by Selected files, Custom values and Genres as applicable;
the **▾** button and **Alt+Up/Down** open the full dropdown with
Keep/Blank commands. Opening one list closes the other. Check the completed
spelling before saving, especially when names share a prefix.

A suggestion containing several separate stored values shows each value in
quotes, such as `“Rock” · “live”`. Choosing it keeps those separate values.
A single stored value containing the literal two-backslash display separator
is a separate choice, such as `Rock \\ live`; choosing it keeps one value.
Whole choices retain leading/trailing spaces and line breaks. Typing text
continues to use the trimming and two-backslash parsing rules described above.
Multi-value choices and values whose whitespace or separators would be changed
by that parser require explicit selection; they remain in the matching list.
Selection choices stay available as typing updates pending edits. Selecting
other files or refreshing the panel updates those choices.

Up/Down highlights matching choices; **Enter** chooses a highlighted value.
Nothing starts highlighted in the automatic list, so ordinary Enter in a
multiline field still inserts a newline. Multiline suggestions replace the
whole value only when explicitly chosen, including all its lines. With no
list open, ordinary Up/Down keeps moving the multiline text cursor. Text
controls can still open the full dropdown with an arrow key. In that full
list, **Alt+Home/End** selects its first/last entry.

**Esc** closes a suggestion list before other Escape actions. Tab closes it
while moving focus without choosing a highlighted alternative. Choosing a
suggestion changes the pending field edit and does not save tags. Pasting or
deleting updates the matching choices without inserting a suffix; free text
remains valid. Completion waits for input-method composition to finish.

With the value list closed, **Shift+Enter** adds the current trimmed,
nonempty single-line text to that field's custom values. This saves a
suggestion, not the audio file. To edit, import or export the suggested lists,
use **View → Field values…**; see [Field Values options](../reference/options/field-values.md).
A multiline value is not accepted as one custom-list entry.

## Work with checkboxes, multiline fields and defaults

A configured checkbox can show checked, unchecked or mixed. Changing it writes
`1` or a removal request as a pending edit for the selection. An existing value
such as `false`, `no` or `0` is treated as off. See
[panel customization](../customize/tag-panel.md) before using a checkbox for a
field whose existing spelling matters.

Multiline controls accept line breaks, useful for comments and lyrics.
The same two-backslash value separator still applies. A line break alone
does not turn one text value into several field values.

Most defaults are `< keep >`. A configured visible field with another default
can automatically stage that default when several files are selected, as long
as that field has no pending edit in the selection. Review **Default value**
under **Customize tag panel…** if selecting a batch unexpectedly creates pending
changes. **View → Default values in input fields** explicitly stages configured
visible defaults, including replacements for existing pending values.
Defaults are literal text, not evaluated format strings.

## Show, arrange and navigate the panel

**View → Show tag panel / Hide tag panel** toggles the whole panel. The shortcut
is **Ctrl+Q**, including on macOS where it means Control, not ⌘.
The panel's **⋯** menu or a right-click on empty panel space exposes field
visibility, **Display cover art**, customization and configuration commands.
Hiding a control or the panel does not remove its metadata.

The position menu supports left, right, top, bottom or floating layouts;
resize the dock or floating panel to make the fields usable. Compact layouts
can use pane tabs instead of the wide desktop arrangement. Customize fields
and their order through [Customize tag panel](../customize/tag-panel.md).

An underlined label identifies its configured Alt mnemonic. Alt is ⌥ Option
on macOS; while typing there, use **Control+Option+letter** so an ordinary
Option character remains text. Text fields keep their usual text-edit menu
when right-clicked.

**Directory** is a navigation control, not a tag field or a folder-renaming
command. Choosing a loaded folder selects its visible files. Typing another
folder and pressing Enter, or choosing **Open folder…**, loads a folder and
can replace the list. Resolve pending edits when asked. For renaming from
metadata, use [Tag → Filename](../converters/tag-to-filename.md).

## Inspect the cover separately

The cover area can show embedded artwork, pending pasted artwork or a folder
image fallback. A folder image is not embedded merely because it is visible.
Use [Working with covers](covers.md) to distinguish these states and choose
add, remove, manage or extract operations. Their write timing is separate
from typing a panel value.

## Gotchas

- Check selection count before a shared edit; an active filter does not prove
  that hidden rows are unselected.
- Changing a read/write tag policy can affect saved storage beyond the one
  field changed. Read [Tags options](../reference/options/tags.md).
- A field's visible label can differ from its actual identifier. Use
  **Extended tags…** when checking `ALBUMARTIST` versus `ARTIST`, or custom names.
- Provider sources can allow editing but require **Save a copy…** to retain
  changes. Read the access message and keep copy/artwork limitations in mind.

## Related

- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Multi-value fields](concepts/multi-value-fields.md)
- [Customize the tag panel](../customize/tag-panel.md)
- [Extended tags](extended-tags.md)

<!-- verified: src/tag-panel.ts:selectionState, comboOptions, choiceText, splitValues, comboKey, choose, applyDefaults, install; src/main.ts:TagPanel host, draftField, keepField, applyAutoDefaults; src/panel-dock.ts. No native UI claim. -->
