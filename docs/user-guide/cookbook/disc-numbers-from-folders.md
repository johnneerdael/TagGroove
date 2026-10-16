# Read disc numbers from CD1 and CD2 folders

Use a consistent folder convention to fill `DISCNUMBER`, while leaving
unrelated folders' existing disc values alone. This recipe accepts `CD1`,
`CD02`, `CD-03` and similar names with positive decimal numbers. It reads
the containing folder's name; it does not rename that folder or number tracks.

## Inspect the folder and current value

Use copied local files with at most one existing numeric `DISCNUMBER` value.
Exclude text containing two consecutive backslashes: Format Value interprets
those as value separators even when its fallback keeps existing text. The
practice cases start at `DISCNUMBER=7` so that a nonmatch visibly keeps it:

<!-- disc-folder-cases:start -->
| Containing folder name | DISCNUMBER before | DISCNUMBER after |
|---|---|---|
| `CD1` | `7` | `1` |
| `cd002` | `7` | `2` |
| `CD-03` | `7` | `3` |
| `Extras` | `7` | `7` |
| `CD0` | `7` | `7` |
| `CD2 bonus` | `7` | `7` |
<!-- disc-folder-cases:end -->

`%_directory%` is the immediate folder name, not its parent or the whole
path. If tracks are inside `CD1/FLAC`, it reads `FLAC`; this recipe will not
look upward for the disc folder. Inspect the actual layout first.

## Preview a guarded expression

1. Save pending edits and select one copied file from `CD1`.
2. Open **Convert → Tag → Tag…** and set **Field** to `DISCNUMBER`.
3. Enter the expression below and wait for the preview.
4. Compare matched and unmatched practice folders before applying to a batch.
5. Choose **Apply to … files** only when the reviewed values are wanted.

<!-- disc-folder-formula:start -->
```text
$if($stricmp($regexp(%_directory%,'\ACD[ _-]*0*([1-9][0-9]*)\z','$1'),%_directory%),%discnumber%,$num($regexp(%_directory%,'\ACD[ _-]*0*([1-9][0-9]*)\z','$1'),1))
```
<!-- disc-folder-formula:end -->

The same expression can be saved as a **Format Value** action with Field
`DISCNUMBER`. A saved group is only a workflow; applying it writes the
selected files immediately. The converter also writes when applied, rather
than leaving its successful result waiting for Save tags.

## Build the expression in understandable pieces

The extraction inside the expression is:

<!-- disc-extraction-formula:start -->
```text
$regexp(%_directory%,'\ACD[ _-]*0*([1-9][0-9]*)\z','$1')
```
<!-- disc-extraction-formula:end -->

<!-- disc-extraction-cases:start -->
| Folder name | Extraction result before the guard |
|---|---|
| `cd002` | `2` |
| `CD-03` | `3` |
| `Extras` | `Extras` |
| `CD0` | `CD0` |
<!-- disc-extraction-cases:end -->

- `\A` and `\z` restrict the match to the whole folder name.
- `CD` ignores case in this function's default mode.
- `[ _-]*` allows spaces, underscores or hyphens before the number.
- `0*` consumes leading zeros. `([1-9][0-9]*)` captures a positive number;
  an all-zero disc or a suffix such as `bonus` does not fit.
- `$1` returns only the captured digits.

Do not apply that extraction alone to every file: `$regexp` returns its
original input when there is no match, so `Extras` would become the disc
value. The final expression compares extraction with the original folder.
Equality means this particular pattern did not remove the `CD` prefix, so
`$if` keeps the old `%discnumber%`. A successful digits-only capture differs
from the original `CD…` name; `$num(…,1)` then produces an unpadded number.

This guard works because every successful capture necessarily changes this
pattern's input. It is not a general test for an arbitrary regex match.

## Other source layouts

For a strictly reviewed `CD…` selection, Guess Values can use Source
`%_directory%` and Guessing pattern `CD%discnumber%`. That pattern is literal
and case-sensitive and does not validate digits: `CDfoo` captures `foo`.
It also retains captured spacing. Prefer the guarded expression when a
mixed folder selection should leave nonmatches unchanged.

For deeper folder structures, review [Filename → Tag](../converters/filename-to-tag.md)
and its folder-level patterns instead of making this expression assume an
unseen parent layout. A file's name and its containing folder are different
sources.

## Gotchas

- An existing list of disc values is not preserved by a bare `%discnumber%`:
  the fallback reads its first value. Use the stated single-value scope.
- This does not calculate a total disc count. Native formats have their own
  number/total representation; inspect [field storage](../reference/field-names.md).
- A positive numeric shape does not establish the album's actual disc count
  or the intended playing order.
- Use **Edit → Undo …** for a recorded conversion/action in the current
  desktop session. Keep copied files for recovery beyond that session.

## Related

- [Placeholders for folders](../reference/placeholders.md#file-name-and-location)
- [Regex syntax and captures](../reference/regular-expressions.md)
- [Tag → Tag](../converters/tag-to-tag.md)
- [Auto-numbering](../converters/auto-numbering.md)

<!-- verified: crates/tag-core/src/actions.rs:FormatValue; crates/tag-core/src/script.rs:regexp,stricmp,num; src-tauri/src/action_host.rs:path_context. All written extraction/guard examples execute with actual directory context under LF/CRLF. No native UI/save-back claim. -->
