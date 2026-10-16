# Read or split a track number and total

A value such as `3/12` already carries the current track and its total.
Keep that convention unless a specific report or custom-field workflow needs
two values. This recipe shows a guarded extraction, then an optional split
into `TRACK` and a custom `TRACKTOTAL` field.

## Choose storage before changing fields

`TRACKTOTAL` here is a custom field. It is not OpenTag's mapped MP4 total:
MP4 uses `TOTALTRACKS`, sharing the native `trkn` number/total pair with
`TRACK`. After reading an MP4, OpenTag can show `TRACK=3/12` and
`TOTALTRACKS=12` together. Writing a standalone current number does not mean
that the native total has disappeared.

For normal tag editing or [auto-numbering](../converters/auto-numbering.md),
keeping `TRACK=3/12` is usually sufficient. For a report, a format expression
can read each component without changing the file. Do not create a custom
field merely to make a player show a total it already understands natively.

## Prepare the practice inputs

Use copies with one `TRACK` and at most one `TRACKTOTAL` value per file.
Exclude values containing two consecutive backslashes: Format Value treats
that text as a value separator, even in a fallback that keeps existing text.
The table starts with `TRACKTOTAL=99` so that unmatched input visibly retains
its old custom value. Only a complete pair of positive decimal numbers,
with optional leading zeros, is extracted.

<!-- track-total-cases:start -->
| TRACK before | TRACKTOTAL before | TRACK after | TRACKTOTAL after |
|---|---|---|---|
| `3/12` | `99` | `3` | `12` |
| `05/012` | `99` | `5` | `12` |
| `3` | `99` | `3` | `99` |
| `3/no-total` | `99` | `3/no-total` | `99` |
| `0/12` | `99` | `0/12` | `99` |
| `14/12` | `99` | `14` | `12` |
<!-- track-total-cases:end -->

The last row is numerically shaped but logically suspicious. The expression
does not verify that the current track is at most the total, or that twelve
files exist. Inspect those facts separately.

## Extract the total before changing TRACK

In **Convert → Tag → Tag…**, set the following destination and expression.
Wait for the preview and compare it with the first table. Cancel if the
calculation is only for inspection; **Apply to … files** writes immediately.

<!-- track-total-formulas:start -->
**Field:** `TRACKTOTAL`

**Format string:**

```text
$if($stricmp($regexp(%track%,'\A0*([1-9][0-9]*)/0*([1-9][0-9]*)\z','$2'),%track%),%tracktotal%,$num($regexp(%track%,'\A0*([1-9][0-9]*)/0*([1-9][0-9]*)\z','$2'),1))
```

After this first conversion, the original `TRACK` still contains `3/12`.
If shortening it is also intended, review the second conversion:

**Field:** `TRACK`

**Format string:**

```text
$if($stricmp($regexp(%track%,'\A0*([1-9][0-9]*)/0*([1-9][0-9]*)\z','$1'),%track%),%track%,$num($regexp(%track%,'\A0*([1-9][0-9]*)/0*([1-9][0-9]*)\z','$1'),1))
```
<!-- track-total-formulas:end -->

Run total extraction first. Shortening `TRACK` first removes the slash and
total that the other expression needs; the total expression would then keep
the old `99`. The same two Format Value actions can be saved in a group,
with `TRACKTOTAL` first and `TRACK` second. **Save groups** stores that order;
applying the checked group writes immediately.

## Understand the guard

The regex uses `\A` and `\z` to match the whole text. `0*` consumes leading
zeros; `[1-9][0-9]*` requires a positive number. The two parenthesized groups
capture current track and total, and `$1` or `$2` chooses one.

`$regexp` returns its input unchanged on a nonmatch. `$stricmp` compares that
result with the original `TRACK`. A successful capture contains only digits,
so it cannot equal the original slash-containing pair. Equality therefore
means “keep the old field” for this particular expression. This is a guard
built for this pattern, not a universal regex-match test.

`$num(…,1)` removes padding from the extracted component. By contrast,
`$num(%track%,1)` alone reads the leading number and discards the total;
it does not save that total anywhere. Guess Values with
`%track%/%tracktotal%` can also split a slash, but does not validate numeric
shape. A value such as `3/no-total` would capture text as its “total.”

## Check the saved format

A core field split is not a promise of identical native storage. MP4 uses its
mapped pair fields and 16-bit unsigned components; inconsistent simultaneous
`TRACK` and `TOTALTRACKS` totals are rejected. Other formats may retain
`TRACKTOTAL` only as a custom field, and ID3v1 cannot store arbitrary custom
fields. See [field storage](../reference/field-names.md) and test read-back on
copies before relying on a device's convention.

Unmatched inputs keep the existing custom total; they do not clear stale
`TRACKTOTAL` values automatically. Inspect them instead of treating a
successful batch as validation of every old value.

Keep file backups. Use **Edit → Undo …** for a recorded conversion/action in
the current desktop session. Neither preview nor a custom total is a backup
of metadata removed by a later operation.

## Related

- [Numbers in scripting](../reference/scripting/numbers.md)
- [Regex syntax and captures](../reference/regular-expressions.md)
- [Tag → Tag](../converters/tag-to-tag.md)
- [Auto-numbering](../converters/auto-numbering.md)
- [Number tracks per folder](number-tracks-per-folder.md)

<!-- verified: crates/tag-core/src/actions.rs:FormatValue; crates/tag-core/src/script.rs; crates/tag-core/src/mapping.rs:mp4_fields,patch_mp4. Written two-expression order, reverse-order caution and all cases execute against core under LF/CRLF. No native save-back claim. -->
