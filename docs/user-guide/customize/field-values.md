# Build reusable field-value choices

Custom field values are saved suggestions for fields such as `GENRE`, `MOOD`
or `COMPOSER`. They help keep spellings consistent while leaving you free to
type another value. Saving a suggestion does not retag existing files; choosing
it for selected tracks and saving those edits is a separate step.

## Make a small MOOD list

1. Choose **View → Field values…**, or **File → Options… → Field Values →
   Edit field values…**.
2. Enter `MOOD` in **Field** and press Enter. Field identifiers are trimmed
   and converted to uppercase. Use the actual metadata field name, rather
   than a display label you gave a tag-panel control.
3. Put one choice on each line in **Custom field values**:

    <!-- field-values-practice:start -->
    ```text
    Reflective
    Uplifting
    Spacious
    ```
    <!-- field-values-practice:end -->

4. Choose the editor's own **OK** and check that the values were saved.
5. If MOOD is not in your tag panel, add an editable MOOD control through
   [tag-panel customization](tag-panel.md). Defining a list does not add a
   new visible field automatically.
6. Select one copied practice track. Open the MOOD dropdown, choose
   `Reflective`, then inspect the pending change. Use **Save tags** only when
   that is the value you intend to write.

The list persists in saved tag-panel configuration. It is not a restriction
on legal metadata values, a rule that automatically repairs older spellings,
or an action applied to the whole collection.

## Understand normalization and order

The editor trims the edges of each line, removes empty lines and exact
repetitions, and keeps the first occurrence order. For example, paste this
practice list, including its blank line:

<!-- field-values-normalization:start -->
```text
  Reflective
Uplifting

Reflective
reflective
```
<!-- field-values-normalization:end -->

The saved choices are:

<!-- field-values-normalized:start -->
```text
Reflective
Uplifting
reflective
```
<!-- field-values-normalized:end -->

`Reflective` and `reflective` remain distinct. Decide on the desired spelling
rather than expecting a case-insensitive cleanup. **Sort A–Z** sorts the
current field's list using the application's text collator; keep your entered
order if you prefer a hand-arranged shortlist.

A field supports up to 5,000 choices. Entries must be nonempty single lines;
invalid control characters or an oversized list can prevent saving. Technical
information fields starting with `_`, such as `_PATH`, cannot receive editable
custom lists. Do not rename a technical field to make it appear writable.

Switching **Field** captures the current field's edits in this dialog's draft.
**Fields with custom values** lets you revisit another configured list.
Clear a field's list and save if you want to remove its custom suggestions;
that does not remove the field or existing values from audio files.

## Selection values, custom values and genres

The tag-panel dropdown can combine special keep/blank choices, values already
in the selection, the custom list and standard `GENRE` choices. A selection
value already present in the custom list is shown once. Special choices are
instructions: `< keep >` retains each file's own value, while `< blank >`
stages clearing the field.

**Prefer values from tags in list at tag panel** changes what that panel
shows. With it on and at least one nonempty selection value, the dropdown
uses the selection's values rather than adding the full custom/standard list.
If the field is empty across the selection, custom suggestions remain
available. For the practice MOOD list above:

<!-- field-values-preference:start -->
| Current selected MOOD values | Prefer tag values | Ordinary value choices, excluding keep/blank |
| --- | --- | --- |
| `Reflective` | Off | `Reflective, Uplifting, Spacious` |
| `Reflective` | On | `Reflective` |
| Both empty | On | `Reflective, Uplifting, Spacious` |
<!-- field-values-preference:end -->

The preference concerns the tag-panel list. File-list and Extended tags
completion use custom candidates together with values from loaded files;
do not assume the panel's shortlist is a global validation rule.
**Options → General → Auto-completion at selection boxes** independently
controls inline completion.

For GENRE, **Show only user-defined genres** omits the standard genre list.
Existing values from selected files can still appear, even if they are not
in your custom list. Saving the preference takes effect in TagGroove without
the restart described for the original editor.

A custom genre remains text, not an extension of ID3v1's fixed indexed genre
vocabulary. Use an appropriate richer format such as ID3v2 for names outside
that vocabulary and inspect the actual saved file. Changing a dropdown does
not extend a container's storage capabilities.

## Import and export one field's list

**Import…** reads one value per text line and merges it with the current
field's dialog list. Exact repetitions are removed. It does not import audio
tags, replace every field's list or assign the imported entries to tracks.
Inspect the merged list, then choose the editor's **OK** to persist it.

The current reader handles BOM-marked UTF-8/UTF-16 and tries UTF-8 before its
Windows-1252 fallback for unmarked text. An older list in another code page
can display incorrectly; keep the original and convert a copy to a known
Unicode encoding before importing again.

**Export…** writes the current field's captured list as UTF-8 with a BOM and
CRLF line endings. The suggested name follows the field, such as `mood.txt`.
Choose a separate new text-file destination; do not select an audio file or
the only copy of an older list backup.
This file write happens immediately. Canceling the editor afterward does
not delete the exported text file. An export is a useful named checkpoint
before a large list change; it is not an audio-file backup.

## Add a value while editing

In a tag-panel field, type the intended single-line text and press
++shift+enter++. TagGroove adds that field's text to its saved custom list.
An exact duplicate is not added again; empty or multiline text is rejected.
If saving the list fails, read the error rather than assuming it persisted.

This shortcut stores a suggestion. It does not separately save pending audio
edits. A field containing a multi-value separator has the normal tag-panel
interpretation when applied to tracks; custom list management does not
change [multi-value syntax](../concepts/multi-value-fields.md).

## Keep the two save boundaries clear

**Cancel** in the field-values editor abandons its unsaved list draft.
Once its own **OK** has saved the list, canceling an outer Options dialog
does not undo that saved change. An immediate list export also survives a
later Cancel. Neither action commits pending track metadata.

The saved `tag-panel.json` component, included in a
[configuration backup](configuration.md), carries custom lists and their
preferences. For sharing just one list, export that field instead of sharing
your entire personal configuration. Session audio Undo is not list-version
history: retain a named export or configuration checkpoint before a major edit.

## Related

- [Tag-panel customization](tag-panel.md)
- [Tag-panel editing](../guides/tag-panel.md)
- [Field Values options](../reference/options/field-values.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Save and restore configuration](configuration.md)

<!-- verified: src/tag-panel.ts:parseValueList, withLists, withCustomValue, comboOptions, fieldValues; src/main.ts:cellCompletions and importList/exportList host; src-tauri/src/tag_panel.rs:read_value_list, write_value_list. Actual written LF/CRLF model examples; no native dialog or audio save-back claim. -->
