# Repair a confirmed UTF-8 decoding mistake

Repair text whose UTF-8 bytes were once read as legacy characters, such as
`MarÃ© Serena` instead of `Maré Serena`. Choose the repair from evidence about
the mistake, then test it on one copied file. The action cannot infer a
language or recreate characters already replaced with `?`.

## Recognize the byte path

The intended `é` has UTF-8 bytes `C3 A9`. Reading those bytes as Western
legacy characters can produce the two visible characters `Ã©`. The repair
recovers those byte values and decodes them as UTF-8 again.

That is why this case uses **Unicode (UTF-8)**, codepage `65001`: it names
what the original bytes should have meant. It does not mean “save this tag
using an arbitrary UTF-8 switch,” and it does not change the audio codec.
A different original byte encoding needs its own diagnosis and codepage.

## Try one affected field

1. Keep an untouched original and select one copied practice file.
2. Inspect **Extended tags…** and identify the affected field and values.
3. Choose **Actions → Actions (Quick)…**, then **Convert Codepage**.
4. Use these settings and choose **Apply action**.
5. Compare the resulting title with a known correct spelling and read any
   operation error before selecting more files.

<!-- mojibake-settings:start -->
| Setting | Value |
|---|---|
| Field | `TITLE` |
| Codepage | Unicode (UTF-8), `65001` |
<!-- mojibake-settings:end -->

<!-- mojibake-cases:start -->
| TITLE before | Result using codepage 65001 |
|---|---|
| `MarÃ© Serena` | `Maré Serena` |
| `Mira â€“ live` | `Mira – live` |
| `Harbour â™¥` | `Harbour ♥` |
| `North Pier` | `North Pier` |
| `Maré Serena` | Conversion error |
| `Mar� Serena` | Conversion error |
| `Mar? Serena` | `Mar? Serena` |
<!-- mojibake-cases:end -->

The correctly accented title is intentionally an error case for this
particular byte repair. Its characters do not encode a valid UTF-8 byte
sequence through this reconstruction. An error is a reason to stop and
inspect, not to try every encoding until one result looks plausible.

The last row remains readable ASCII but does not regain the lost accent.
A previous replacement with `?` or `�` can mean that the original character
information is already gone. Compare an original backup or another reliable
source instead of accepting a guessed name as an encoding conversion.

## Keep the repair narrowly targeted

The action processes every value of the chosen field. A field containing
already-correct and incorrectly decoded text can refuse conversion; it is
not a per-character language detector. For this action, unrepresentable
characters or invalid byte sequences report an error rather than silently
returning replacement glyphs.

Use `TITLE` first. `_TAG` includes all tag fields; `_ALL` also includes
filenames, and `_DIRECTORY` targets a folder name. Those scopes can affect
correct text and paths. They are unsuitable for an unconfirmed whole-library
experiment.

The byte recovery accepts characters up to U+00FF directly and supported
Windows-1252 glyphs such as the punctuation in the practice cases. It is not
a universal inverse of every historical mis-decoding. More complicated
multi-stage damage needs a separately tested diagnosis.

## Separate text repair from storage settings

Correcting a value does not choose ID3v2.3 versus ID3v2.4, change a sidecar
file's encoding, or prove that a target player supports its eventual native
tag. Review [tag write options](../../reference/options/tags-mpeg.md) and
[text encodings](../../tagging/concepts/text-encodings.md) for those separate choices.

`$ansi` evaluates with the application's ANSI context; it is not a replacement
for selecting the known original byte encoding in this action. When importing
new text, choose the correct source encoding instead of deliberately loading
bad text and repairing it afterward.

## Check timing and avoid a second repair pass

The action writes immediately. Saving it in a group stores a workflow without
running it; applying that group writes the selected files. Do not run the
same repair again just because the spelling was corrected once. The corrected
accented text can now fail this same conversion.

A warning from one action does not stop later actions/groups. Avoid appending
case conversion or deletion until this repair has been inspected; those can
make diagnosis harder or change other data after a repair warning.
Use **Edit → Undo …** for a recorded write in the current desktop session,
and retain the original file for recovery beyond it.

## Related

- [Convert Codepage](../convert-codepage.md)
- [Text encodings and codepages](../../tagging/concepts/text-encodings.md)
- [Capitalization with exceptions](fix-capitalisation.md)
- [Editing, saving and undo](../../getting-started/editing-and-saving.md)

<!-- verified: crates/tag-core/src/actions.rs:decode_codepage,ConvertCodepage,transform. Exact written UTF-8 repair/error cases execute against core under LF/CRLF; no native UI or universal recovery claim. -->
