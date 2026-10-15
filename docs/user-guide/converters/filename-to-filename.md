# Filename → Filename

Use **Filename → Filename** to rearrange parts of existing names without
reading or changing tags. Mark the parts in the old pattern with `%1`
through `%9`, then reuse those parts in the new pattern.

## Swap an artist and title

Start with `Juniper Vale - Paper Boats.mp3` and make
`Paper Boats - Juniper Vale.mp3`:

1. Save pending edits and select copied practice files.
2. Choose **Convert → Filename → Filename…** (++alt+3++; Alt is ⌥ Option on a Mac).
3. Set **Old filename pattern** to `%1 - %2`.
4. Set **New filename pattern** to `%2 - %1`.
5. Choose **Preview** and inspect the source and destination rows.
6. Choose **Rename … files** to apply that reviewed plan.

The old pattern captures `%1` = `Juniper Vale` and `%2` = `Paper Boats`.
The new pattern outputs them in reverse order. The audio extension is
retained; it need not appear in either pattern.

## Keep, drop and transform parts

| Input filename | Old filename pattern | New filename pattern | Result |
|---|---|---|---|
| `003 Juniper Vale - Paper Boats.mp3` | `%1 %2 - %3` | `%2 - %3` | `Juniper Vale - Paper Boats.mp3` |
| `Juniper Vale__Paper Boats.flac` | `%1__%2` | `%1 - %2` | `Juniper Vale - Paper Boats.flac` |
| `Juniper Vale - Paper Boats.mp3` | `%1 - %2` | `$upper(%1) - %2` | `JUNIPER VALE - Paper Boats.mp3` |
| `03 - Paper Boats.mp3` | `%1 - %2` | `$num(%1,3). %2` | `003. Paper Boats.mp3` |

The old pattern is a matching pattern: literal punctuation must match
exactly. It accepts only numbered parts, with no closing percent sign
after the number. `%artist%` is not an old-pattern capture.

The new pattern is an output format. It can use scripting functions and
existing tag placeholders alongside the numbered parts. A part omitted
from the new pattern is discarded from the name. Reusing a part there
is allowed, for example `%2 - %2`.

## Read and create folders

Backslashes in the old pattern mark folder boundaries. For example:

| Setting | Value |
|---|---|
| Input path | `Music/Juniper Vale - Harbour Lights/03 Paper Boats.mp3` |
| **Old filename pattern** | `%1 - %2\%3 %4` |
| **New filename pattern** | `%1 - %2 - %3 - %4` |

The four parts are `Juniper Vale`, `Harbour Lights`, `03` and `Paper Boats`.
The output name is `Juniper Vale - Harbour Lights - 03 - Paper Boats.mp3`.
The path's earlier `Music` component is outside this relative match.

Backslashes in the new pattern create destination folders. A relative
destination is based on the source file's containing folder, not the
earlier root omitted from the matching pattern. Use an absolute destination
when moving several source folders into one new layout. Preview the full
paths before applying.

## Gotchas

- `%10`, an undefined numbered part, or a repeated number in the old
  pattern is rejected. Use each old-pattern number once, from `%1` to `%9`.
- Adjacent captures such as `%1%2` have no separator and are ambiguous.
- The old pattern must match every selected source for a valid plan.
  Separate files with different naming conventions before converting.
- Preview plans expire and are tied to the inputs, selection and source
  files. If the plan is stale or a file changed, run **Preview** again.
- Avoid duplicate destinations and existing files with those names.
  Keep copies and inspect **Edit → Undo** for recorded renames.

<!-- verified: crates/tag-core/src/converters.rs:FilenameConverter, numbered_pattern; src/main.ts:reviewedConverter -->

## Related

- [Filename → Tag](filename-to-tag.md)
- [Tag → Filename](tag-to-filename.md)
- [Format strings](../reference/format-strings.md)
- [Converters](index.md)
