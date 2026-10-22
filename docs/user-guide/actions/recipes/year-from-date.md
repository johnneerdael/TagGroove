# Keep the year from an ISO date

Shorten `2024-05-21` to `2024` when a chosen workflow needs a year-only
`YEAR`. Decide first whether losing the month and day is acceptable. This
recipe changes metadata; it does not repair an invalid calendar date or
restore missing date information.

## Compare a shortcut with a guarded rule

In **Convert → Tag → Tag…**, a single-value field can preview
`$left(%year%,4)`. It takes four characters regardless of their meaning:

<!-- year-left-cases:start -->
| YEAR before | Format string | Preview result |
|---|---|---|
| `2024-05-21` | `$left(%year%,4)` | `2024` |
| `2024` | `$left(%year%,4)` | `2024` |
| `May 2024` | `$left(%year%,4)` | `May ` |
| `unknown` | `$left(%year%,4)` | `unkn` |
<!-- year-left-cases:end -->

The space after `May` in that result is real. `$left` is useful for a
selection already known to begin with a four-digit year; it is not a date
parser. Wait for the preview and Cancel if only inspecting it.

## Change only the chosen date shape

Use the following regex action to affect complete `YYYY-MM-DD` values while
leaving other spellings unchanged:

1. Save or discard pending edits and select one copied file.
2. Open **Actions → Actions (Quick)…**, then **Replace with Regular Expression**.
3. Set **Field** to `YEAR`.
4. Enter Pattern `\A([0-9]{4})-[0-9]{2}-[0-9]{2}\z`.
5. Set **Replace matches with** to `$1` and **Case-sensitive comparison** to Off.
6. Choose **Apply action**, then inspect the saved value and operation result.

<!-- year-regex-settings:start -->
| Setting | Value |
|---|---|
| Field | `YEAR` |
| Pattern | `\A([0-9]{4})-[0-9]{2}-[0-9]{2}\z` |
| Replace matches with | `$1` |
| Case-sensitive comparison | Off |
<!-- year-regex-settings:end -->

<!-- year-regex-cases:start -->
| YEAR before | YEAR after |
|---|---|
| `2024-05-21` | `2024` |
| `1998-12-03` | `1998` |
| `2024` | `2024` |
| `May 2024` | `May 2024` |
| `2024-05-21T10:30:00Z` | `2024-05-21T10:30:00Z` |
| `2024-99-99` | `2024` |
<!-- year-regex-cases:end -->

The last row is intentional: four digits, two digits and two digits describe
shape, not calendar validity. Validate suspect dates separately; this action
still extracts the first four digits from that shaped input. A timestamp
with a time portion does not match this particular whole-value rule.

## Understand the capture and write timing

`\A` and `\z` anchor the entire value rather than each line. `[0-9]{4}`
requires exactly four digits. Its surrounding parentheses capture the year
as group 1; `$1` inserts that capture in the replacement. The hyphens and
remaining two-digit components must be present for a match.

Type the pattern and `$1` directly into their regex-action boxes without
format-string quotes. The action processes each existing value of `YEAR`;
nonmatches keep their text. Unlike the converter's bare placeholder, it does
not first reduce a multi-value field to its first item.

The action writes immediately. To save it for later, add it to a group and
choose **Save groups**; that saves the workflow without running it. Apply
the checked group only to a reviewed selection. There is no full action
preview in this recipe, so test a copied file before a larger batch.

## Keep the original date when it matters

A title's release year, an original release date and a recording date can
be different facts. Choose the right field rather than shortening every date
in the library. `YEAR` and other date names map differently by native format;
see [field names and storage](../../reference/field-names.md). An extra custom
field is not a substitute for a full file backup, and ID3v1 cannot carry all
custom/date information.

Inspect read-back and the intended player's display before applying the
convention throughout a collection. Use **Edit → Undo …** for a recorded
write during the current desktop session, and retain original files for
recovery after quitting.

## Related

- [Text scripting functions](../../scripting/functions/text.md#left)
- [Regex syntax and replacements](../../scripting/regular-expressions.md)
- [Replace with Regular Expression](../replace-regexp.md)
- [Editing, saving and undo](../../getting-started/editing-and-saving.md)

<!-- verified: crates/tag-core/src/actions.rs:RegexReplace; crates/tag-core/src/script.rs:left; crates/tag-core/src/mapping.rs. Actual written expression/settings/cases execute under LF/CRLF; no native UI or universal date-storage claim. -->
