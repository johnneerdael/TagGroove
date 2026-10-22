# Replace with Regular Expression

Use this action when the text varies in a predictable way: leading track
numbers, repeated spaces, a dated suffix, or a guest name to extract.
The regular expression describes what to match; the replacement can reuse
captured parts.

<!-- figure:action-regex-replace -->
<figure markdown>
![The Edit action dialog with the action type set to Replace with Regular Expression and its settings](../assets/screens/action-regex-replace-light.png#only-light)
![The Edit action dialog with the action type set to Replace with Regular Expression and its settings](../assets/screens/action-regex-replace-dark.png#only-dark)
<figcaption>The Replace with Regular Expression action's settings in an action group.</figcaption>
</figure>
<!-- /figure -->

## Run the action

1. Save pending edits and select a few copied files.
2. Choose **Actions → Actions (Quick)…**.
3. Select **Replace with Regular Expression**.
4. Enter **Field**, **Regular expression** and **Replace matches with**.
5. Set **Case-sensitive comparison**, then choose **Apply action**.

There is no separate save step. For a preview while designing a pattern,
try the equivalent `$regexp` expression in **Convert → Tag → Tag…** first.

| Setting | Meaning |
|---|---|
| **Field** | One field or a supported target below |
| **Regular expression** | A Boost Perl-style pattern |
| **Replace matches with** | Replacement text and capture references such as `$1`; leave empty to remove matches |
| **Case-sensitive comparison** | Off initially; turn it on when case matters |

The action transforms every value in a multivalue field independently.
It replaces all matches; a value with no match remains unchanged.

## Patterns to try

Enter these patterns and replacements directly, without outer quotes:

| Goal | Field | Regular expression | Replace matches with | Before → After |
|---|---|---|---|---|
| Remove a leading track number | `TITLE` | `^\d+[\s._-]*` | Leave empty | `03_Paper Boats` → `Paper Boats` |
| Collapse repeated spaces | `TITLE` | `\s{2,}` | One space | `North   Pier` → `North Pier` |
| Keep only a year | `YEAR` | `^(\d{4}).*$` | `$1` | `2024-05-01` → `2024` |
| Put "The" after an artist | `ARTIST` | `^The (.+)$` | `$1, The` | `The Lanterns` → `Lanterns, The` |
| Remove a dated remaster suffix | `TITLE` | `\s*\(Remastered \d{4}\)$` | Leave empty | `North Pier (Remastered 2021)` → `North Pier` |

For example, `03_Paper Boats` becomes `Paper Boats`, and
`North Pier (Remastered 2021)` becomes `North Pier`. Test the pattern on
unexpected values too; a successful match on one example is not enough
for an entire library.

## Quotes differ from format strings

The action boxes contain raw pattern and replacement text. Do not wrap a
pattern in single quotes, and type `$1` directly. By contrast, a `$regexp`
function sits inside the format-string language and needs quoting around
special syntax:

```text
$regexp(%title%,'^\d+[\s._-]*',)
```

The final argument above is empty. `''` would insert an apostrophe.
See [the regex reference](../scripting/regular-expressions.md#regexp) for
quoting, groups, lookarounds, case controls and replacement escapes.

## Special targets and extensions

| Target | What the action reads |
|---|---|
| `_TAG` | Every tag field |
| `_ALL` | Every tag field and each filename, including its extension |
| `_FILENAME` | The filename including its extension |
| `_DIRECTORY` | The containing folder name |

For `_FILENAME`, `\s+$` does not remove spaces just before `.mp3`, because
those spaces are not at the end of the input. Use `\s+(?=\.\w+$)` for that
specific case. Protect the extension when rearranging captured groups.
Folder renames can affect unselected files in that folder.

On Windows, a filename result receives the same
[Windows filename cleanup as literal Replace](replace.md#filename-results-on-windows).
The regex operates on the full filename first, so its resulting extension is
retained. For example, a result of `CON.MP3` becomes
`Reserved-Windows-Filename.MP3`. Characters removed during cleanup can make two
results identical; inspect collision errors and the Undo label after the run.
Ordinary tag text and Unix-valid filename characters retain their normal rules.

Bare console names `CONIN$` and `CONOUT$` also become
`Reserved-Windows-Filename`. Their extension-bearing forms, such as
`CONIN$.wav`, are kept; `CON.MP3` still receives the substitution above.

## Gotchas

- Patterns can also match an empty string, producing replacements between
  characters. Preview deliberately before using an empty or optional pattern.
- Invalid syntax reports an error. A valid pattern that finds nothing is
  not an error and leaves the value unchanged.
- `_FIELDNAME` is not a supported selector for this action.
- Keep backups and inspect **Edit → Undo** and operation errors after a run.
  Generated file effects are not a guarantee of whole-batch rollback.

## Related

- [Regular expressions](../scripting/regular-expressions.md)
- [Literal Replace](replace.md)
- [Move guest artists to titles](recipes/feat-artists-to-title.md)
- [Actions and action groups](index.md)
