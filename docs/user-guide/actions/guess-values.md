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

## Prepare a bracketed release label with a regex

Suppose each selected file has one `COMMENT` value containing a release label,
such as `[2024] Harbour Lights`. Use a source expression to rearrange that
label before the guessing pattern captures `YEAR` and `ALBUM`. `COMMENT`
itself stays unchanged.

<!-- guess-regex-stages:start -->

Enter this in **Source format string**, exactly as shown:

```text
$regexp($trim(%comment%),'\A\[(\d{4})\][ \t]+(.+)\z','$1 :: $2')
```

Enter this in **Guessing pattern**, without outer quotes:

```text
%year% :: %album%
```

The two stages do different jobs:

1. `$trim` removes the comment's edge whitespace. The quoted regex then
   matches a bracketed four-digit year, a space or tab gap, and an album name.
   `$1 :: $2` builds the intermediate text, for example
   `2024 :: Harbour Lights`. The apostrophes belong to the format-string
   syntax; the regex receives `\[` to match a literal opening bracket. Type
   one backslash in each escape, not `\\`.
2. The guessing pattern reads that intermediate text. `%year%` and `%album%`
   are destinations, and ` :: ` is an exact separator. It does not evaluate
   regex groups, `$1` or scripting functions.

For the following cases, start with `YEAR` = `1999` and `ALBUM` =
`Keep this album`. In the input column, `·` represents one space and `⇥`
represents a tab; substitute those whitespace characters in the actual tag.

| Case | `COMMENT` before | Intermediate source text | `YEAR` after | `ALBUM` after |
|---|---|---|---|---|
| Bracketed label | `··[2024] Harbour Lights··` | `2024 :: Harbour Lights` | `2024` | `Harbour Lights` |
| Tab gap | `[2025]⇥L'Été, Live` | `2025 :: L'Été, Live` | `2025` | `L'Été, Live` |
| No separator | `Live recording` | `Live recording` | `1999` | `Keep this album` |
| Already separated | `2023 :: North Pier` | `2023 :: North Pier` | `2023` | `North Pier` |

The last two rows show a common mistake: treating the source regex as a
validation gate. `$regexp` returns its input unchanged on a nonmatch. Guess
Values still tries its pattern, so an already separated comment can match
even though the regex did not. Select the intended files and inspect the
intermediate text; a regex nonmatch does not skip this action or later ones.

<!-- guess-regex-stages:end -->

### Check the stages before applying

1. Save pending edits and select copied practice files representing the
   different input layouts.
2. Open **Convert → Tag → Tag…**, set **Field** to `COMMENT`, and paste the
   source expression above into **Format string**. Wait for the automatic
   preview and compare it with the intermediate-text column. This previews
   only the source expression, not the guessing pattern or its destinations.
3. Choose **Cancel**, without choosing **Apply**. This leaves the saved
   comments unchanged.
4. Open **Actions → Actions (Quick)… → Guess Values**, enter the two settings
   above, and choose **Apply action** on the copies. It writes immediately.
5. Inspect `YEAR`, `ALBUM` and the unchanged `COMMENT` in **Extended tags…**.
   Check nonmatching files too, then inspect operation errors and the recorded
   **Edit → Undo** entry before running a larger selection.

Keep the quoted regex in the source box. Pasting it into **Guessing pattern**
does not turn that box into a regex parser. See the
[regex quoting reference](../reference/regular-expressions.md#regexp).

<!-- verified: crates/tag-core/tests/action_guide.rs:guess_values_regex_preparation_runs_the_written_stages; src/tag-tag-ui.ts:openTagToTag preview/apply handlers -->

## Gotchas

- No full match leaves this action's destinations unchanged and produces no
  mismatch warning. Later actions and groups still run using the unchanged
  fields: Guess Values does not conditionally skip them. Check both matched
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
