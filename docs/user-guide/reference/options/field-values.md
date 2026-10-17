# Field Values options

Open **File → Options… → Field Values**, then **Edit field values…**, to maintain reusable choices for any editable tag field. The same editor is available through **View → Field values…**.

!!! tip "New to this?"
    Use a custom list for consistent spellings such as `Indie Folk`, `Instrumental`, and `Live`. The list suggests values; it does not forbid other text or automatically retag existing files.

## Choose a field and enter values

Enter an uppercase field name in **Field**, or select a field under **Fields with custom values**. Put one choice per line in **Custom field values**. Field names are normalized to uppercase; technical information fields beginning with `_` cannot receive editable custom lists.

The editor trims leading/trailing whitespace, drops empty lines and exact duplicates, and preserves the order entered. **Sort A–Z** sorts the current list. A field supports up to 5,000 custom choices. Multiline text belongs in metadata fields, not inside one list entry.

Press the editor's own **OK** to save. **Cancel** abandons its unsaved list edits. Once that **OK** has saved the list, canceling the outer Options dialog does not reverse it.

## Where choices appear

The tag-panel drop-down can include special keep/blank choices, values already present in the selection, the field's custom list, and standard genre choices for `GENRE`. In-place file-list and Extended tags editing also receive completion candidates from field values. [General → Auto-completion](general.md) controls inline completion, independently of storing these choices.

**Prefer values from tags in list at tag panel** defaults to off. Enable it to show only the selection's existing values, alongside applicable keep/blank choices, when the selection already supplies values. If a field is empty, its custom choices remain available. This preference can make a large collection's current spellings easier to review without mixing in the whole custom list.

## Genres

**Show only user-defined genres** defaults to off. Enable it to omit the standard genre list from the panel's choices and use the custom `GENRE` list instead. Existing selected values still appear. In TagGroove the saved list preferences apply without requiring the Mp3tag restart described in the original reference.

A custom genre is still text. ID3v1 supports a fixed indexed genre list and cannot preserve arbitrary new genre names. Use a richer tag type such as ID3v2 or APEv2 when saving user-defined genres; changing this UI list does not extend ID3v1.

## Import, export, and quick addition

**Import…** reads a text list with one value per line and merges it into the current field's list, removing exact duplicates. It does not replace every field's lists. **Export…** writes the current field's list for backup or sharing. Export writes a file immediately, even if the editor is later canceled.

While editing a single-line value in the tag panel, ++shift+enter++ adds that field's current text to its custom list. Empty or multiline values are rejected, and an exact duplicate is not added again. This stores a suggestion; it is not a separate command to save pending audio-file edits.

## Related

- [Tag-panel configuration](tag-panel.md)
- [General completion](general.md)
- [Mpeg and genre storage](tags-mpeg.md)

<!-- verified: src/tag-panel.ts fieldValues, normalizeValues, comboOptions, withCustomValue, addCurrent, settingsError; src/main.ts cellCompletions and ExtendedTags.completions; baseline007a806 has these behaviors. -->
