# Format Value

Use **Format Value** to create or replace a field with a value calculated
from other tags. It can also build a new filename or folder name. Use it
when the result needs placeholders, conditions or scripting functions.

## Fill a field from other tags

1. Save pending edits and select a few practice files.
2. Open **Actions → Actions (Quick)…** and select **Format Value**.
3. Enter the destination in **Field** and the expression in **Format string**.
4. Choose **Apply action** and inspect the result.

For a reusable workflow, add the same action to an
[action group](index.md#build-a-reusable-group) and choose **Keep action**.
Saving a group alone does not run it.

| Setting | Meaning |
|---|---|
| **Field** | A tag field, `_FILENAME`, or `_DIRECTORY` |
| **Format string** | Text, placeholders and functions evaluated for each selected file |

## Worked examples

| Field | Format string | Result |
|---|---|---|
| `ALBUMARTIST` | `$if2(%albumartist%,%artist%)` | Keep an existing album artist; otherwise use the first artist |
| `YEAR` | `$left(%year%,4)` | `2024-05-01` becomes `2024` |
| `TITLE` | `$replace(%title%,_, )` | `Low_Tide` becomes `Low Tide`; the last argument is one space |
| `COMMENT` | `Tagged with TagGroove` | Set the same comment in every selected file |
| `TRACK` | `$num(%track%,2)` | `2/8` becomes `02`; the total is not retained by this expression |

Try a small selection first. Format Value replaces the destination's whole
value list; it does not append to an existing list.

## Write one or several field values

For an ordinary tag field, two consecutive backslashes (`\\`) in the
**evaluated result** separate values. When that delimiter is present,
surrounding whitespace is trimmed from each component and empty components
are dropped. If none remain, the field is removed. One backslash is ordinary
text. Without the delimiter, a nonempty result stays one value and keeps its
surrounding whitespace.

In this table, `·` marks a space in a stored value; it is not a character to
type. Each separately quoted result is a separate `GENRE` value.

<!-- format-value-output-examples:start -->
| Format string | Values stored in `GENRE` |
|---|---|
| `Rock\\Pop` | `Rock` + `Pop` |
| `Jazz \\ \\ Blues\\` | `Jazz` + `Blues` |
| `$char(32)Live$char(32)` | `·Live·` |
| `AC\DC` | `AC\DC` |
| `\\` | *(field removed)* |
| `$if(%missing%,unused,)` | *(field removed)* |
<!-- format-value-output-examples:end -->

A bare `%artist%` reads only its first value. To write all existing artists
as separate values, use `$meta_sep(artist,\\)`; a comma or ` & ` join instead
produces one combined value. Read the
[multivalue syntax](../reference/format-strings.md#multi-value-fields) before
replacing artist lists. A tag format may limit how separate values are stored.

These output-splitting rules apply to metadata fields. `_FILENAME` and
`_DIRECTORY` use the naming rules below; their backslashes name folder levels,
not several metadata values.

An empty result removes the destination field. A missing `%albumartist%`
alone therefore removes `ALBUMARTIST`; `$if2` is useful when a fallback is
required. `''` is a literal apostrophe, not an empty result.

## Build filenames and folders

For **Field** `_FILENAME`, try:

```text
$num(%track%,2) - %artist% - %title%
```

With track `2/8`, artist `Juniper Vale` and title `Lanterns on the Water`,
the stem becomes `02 - Juniper Vale - Lanterns on the Water`. The original
audio extension is retained. A format result already ending in that
extension does not need a second copy of it.

For `_DIRECTORY`, the result names the containing folder. That can affect
other files in the folder, including files outside the selection. Practice
on a copied folder before using this target.

Literal folder separators in a format string can create folder levels.
Separators originating in tag values use the configured replacement;
see [file and folder rules](../reference/format-strings.md#file-and-folder-names).
An empty filename or directory result is rejected.

## Gotchas

- `_TAG`, `_ALL`, `_FIELDNAME` and information placeholders such as
  `_BITRATE` are not Format Value destinations. Choose a real tag field or
  one of the two naming targets.
- An action writes when it runs. It does not leave the result waiting in
  the tag panel. Keep copies and check **Edit → Undo** for recorded changes.
- Within a group, the expression sees earlier actions' results. Put a copy
  action before any action that clears its source field.
- The preview in **Convert → Tag → Tag…** is useful for experimenting with
  the expression before putting it in a group.

## Related

- [Format strings](../reference/format-strings.md)
- [Placeholders](../reference/placeholders.md)
- [Actions and action groups](index.md)
- [Zero-padded track numbers](../cookbook/rename-zero-padded-track-numbers.md)
