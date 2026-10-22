# Case Conversion

Use **Case Conversion** to make capitalization consistent in a field or
filename. It applies a mechanical rule; it does not recognize artist names,
abbreviations or words that should stay lowercase.

<!-- figure:action-case-conversion -->
<figure markdown>
![The Edit action dialog with the action type set to Case Conversion and its settings](../assets/screens/action-case-conversion-light.png#only-light)
![The Edit action dialog with the action type set to Case Conversion and its settings](../assets/screens/action-case-conversion-dark.png#only-dark)
<figcaption>The Case Conversion action's settings in an action group.</figcaption>
</figure>
<!-- /figure -->

## Change a field's capitalization

1. Select a few copied files and save pending edits.
2. Choose **Actions → Actions (Quick)…**, then **Case Conversion**.
3. Set **Field** to `TITLE` and **Case** to **Mixed Case**.
4. Keep **Words begin after** as `(-_` for this example.
5. Choose **Apply action** and inspect the result.

`NORTH PIER (LIVE)` becomes `North Pier (Live)`. Save the same settings in
an [action group](index.md#build-a-reusable-group) for repeated use.

## Choose a mode

| Case | Input | Result |
|---|---|---|
| **lower case** | `NORTH PIER` | `north pier` |
| **Mixed Case** | `NORTH PIER` | `North Pier` |
| **Sentence** | `NORTH PIER. LOW TIDE` | `North pier. low tide` |
| **UPPER CASE** | `North Pier` | `NORTH PIER` |

**Sentence** uppercases the first character of the whole value and
lowercases everything after it. It does not start again after a period.
If the value begins with a space or quote, that character occupies the
first position; trim or remove the prefix first when appropriate.

## Teach Mixed Case where words begin

Whitespace always begins a new word. **Words begin after** adds individual
characters that should also begin a word. The initial setting `(-_` means
an opening parenthesis, hyphen or underscore starts the next word.

| Input | Words begin after | Result in Mixed Case |
|---|---|---|
| `LOW-TIDE` | `(-_` | `Low-Tide` |
| `LOW-TIDE` | Leave empty | `Low-tide` |
| `NORTH.PIER` | `.` | `North.Pier` |
| `DJ NIMBUS` | `(-_` | `Dj Nimbus` |

The setting is a set of characters, not a regex or a list of words. It
does not preserve `DJ`, `EP`, `III` or an artist's chosen spelling. Restore
those with a later **Replace** action when the rule is unambiguous.

## Special targets

| Field | What changes |
|---|---|
| A field such as `TITLE` | Every value of that field |
| `_TAG` | All tag fields |
| `_ALL` | All tag fields and the filename stem |
| `_FILENAME` | The filename stem; the audio extension is retained |
| `_DIRECTORY` | The containing folder name |
| `_FIELDNAME` | Field-name capitalization on APEv2 and Vorbis Comment tags |

Changing a folder can affect files outside the selection. For naming
targets, inspect the actual path after the action, especially for a change
that alters only capitalization on a case-insensitive volume.

## Gotchas

- Apply a named field before trying `_TAG`: comments, URLs and identifiers
  may require exact capitalization.
- Case conversion processes each multivalue item separately and preserves
  their order.
- The action saves when it runs. Use practice copies and inspect
  **Edit → Undo** for the recorded change.

<!-- verified: crates/tag-core/src/actions.rs:case_value, transform; src/action-ui.ts:controls -->

## Related

- [Replace](replace.md)
- [Format Value](format-value.md)
- [Actions and action groups](index.md)
