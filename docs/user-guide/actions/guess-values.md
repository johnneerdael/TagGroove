# Guess Values

Use **Guess Values** to split a structured value into several fields.
Supply a source format string to read the text, then a guessing pattern
to say which parts belong in which fields. Despite its name, this action
does not infer a layout: the entire source must match the pattern.

## Split an artist and title

Suppose `TITLE` contains `Juniper Vale / Paper Boats`, and the artist field
is missing.

1. Save pending edits and select a copied practice file.
2. Choose **Actions → Actions (Quick)…**, then **Guess Values**.
3. Enter `%title%` in **Source format string**.
4. Enter `%artist% / %title%` in **Guessing pattern**.
5. Choose **Apply action**, then inspect both fields.

The result is `ARTIST` = `Juniper Vale` and `TITLE` = `Paper Boats`.
The source is evaluated before either destination changes, so reading
from `TITLE` while replacing `TITLE` works.

## Understand the two boxes

| Setting | What it does |
|---|---|
| **Source format string** | Reads tags and evaluates functions to produce one text value |
| **Guessing pattern** | Captures text between exact separators into named fields |

In the pattern, `%artist%` names a destination. It does not read the
existing artist value. The spaces around `/` are literal and must also
occur in the source. Pattern text is case-sensitive.

| Source text | Guessing pattern | Fields written |
|---|---|---|
| `Juniper Vale / Paper Boats` | `%artist% / %title%` | `ARTIST` = `Juniper Vale`; `TITLE` = `Paper Boats` |
| `2024 - Harbour Lights` | `%year% - %album%` | `YEAR` = `2024`; `ALBUM` = `Harbour Lights` |
| `02 - Paper Boats - archive` | `%track% - %title% - %dummy%` | `TRACK` = `02`; `TITLE` = `Paper Boats` |
| `Juniper Vale / Mira Solberg` | `%artist% / %artist%` | Two `ARTIST` values, in that order |

Use `%dummy%` for a part that should be consumed without being stored.
Repeated destination names create multiple values, replacing the field's
previous values.

## Make the source predictable

Functions belong in **Source format string**, for example:

```text
$trim(%comment%)
```

With `COMMENT` = `  2024 - Harbour Lights  `, this produces
`2024 - Harbour Lights`. The pattern `%year% - %album%` can then capture
the two values without surrounding spaces.

Captures themselves are not trimmed. A separator appearing inside a
name can also change where the text splits: captures take the shortest
text that permits the rest of the pattern to match. Choose a separator
that is unambiguous for the selected files.

## Gotchas

- No full match means no field changes for that file. Check both successful
  and unmatched files after a batch.
- Adjacent placeholders such as `%artist%%title%` are rejected because
  there is no separator defining where the first value ends.
- The pattern needs at least one complete `%field%` placeholder. It is
  not a regular expression or a place to call scripting functions.
- `_FILENAME` and `_DIRECTORY` destinations can rename files or folders.
  Captured names are treated as values, with invalid naming characters
  replaced. Use ordinary tag destinations while learning the action.
- This action saves changes immediately. Try copies first and inspect
  **Edit → Undo** for recorded changes.

<!-- verified: crates/tag-core/src/actions.rs:guessing_pattern, try_guess_values, apply_action_spec_with_modes -->

## Related

- [Format strings](../reference/format-strings.md)
- [Format Value](format-value.md)
- [Tags from folder and file names](../cookbook/tags-from-folder-and-filename.md)
- [Actions and action groups](index.md)
