# Convert Codepage

Use **Convert Codepage** to repair text whose legacy bytes were interpreted
with the wrong character encoding. It reinterprets the existing characters
as bytes, then decodes those bytes with the selected codepage.

This is useful for a known encoding mistake. It is not a language detector
or translation tool. Correct Unicode text can be damaged by applying it
again.

## Repair one known mistake

1. Preserve an untouched copy and select one practice file.
2. Open **Actions → Actions (Quick)…**, then **Convert Codepage**.
3. Set **Field** to the affected field, such as `TITLE`.
4. In **Codepage**, choose the encoding that the original bytes used.
5. Choose **Apply action** and compare with the intended spelling.

For example, the two characters `Ã©` preserve the UTF-8 bytes for `é`.
Choosing **Unicode (UTF-8)** converts them back to `é`. This diagnosis
depends on the original byte mistake; choosing a Windows codepage instead
would not perform the same repair.

## Worked byte-to-text examples

| Existing text | Codepage | Result |
|---|---|---|
| `Ã©` | Unicode (UTF-8), `65001` | `é` |
| `Ïðèâåò` | Cyrillic (Windows-1251), `1251` | `Привет` |
| `Plain ASCII` | Western European (Windows-1252), `1252` | `Plain ASCII` |

The action processes all values of the selected field. It does not
change an audio codec, the tag format, or the encoding of a sidecar text
file. For reading a text file, use **Import Text File** instead.

## Choose the codepage from evidence

The list offers Windows regional encodings, DOS/OEM pages, East Asian
encodings, Mac encodings, ISO-8859 variants, KOI8 and UTF-8:

| Family | Examples in the list |
|---|---|
| Windows | 1250 Central European, 1251 Cyrillic, 1252 Western European, 1253 Greek, 1254 Turkish, 1255 Hebrew, 1256 Arabic, 1257 Baltic, 1258 Vietnamese, 874 Thai |
| DOS/OEM | 437, 720, 737, 775, 850, 852, 855, 857, 858, 860–866, 869 |
| East Asian | 932 Shift-JIS, 936 GBK, 949 Korean, 950 Big5, 54936 GB18030, 51932 EUC-JP |
| Other | 20866 KOI8-R, 21866 KOI8-U, 10000 Mac Roman, 10007 Mac Cyrillic, ISO-8859 variants, 65001 UTF-8 |

An imported action can retain a codepage identifier outside the normal
list; unsupported codepages report an error when run. Do not select an
encoding solely because the output happens to look plausible.

## Targets and limitations

Use a named field first. `_TAG` processes every tag field; `_ALL` also
processes the filename stem. `_FILENAME` keeps the audio extension, and
`_DIRECTORY` changes the containing folder name.

The conversion must recover a byte for every existing character. Values
up to U+00FF map to those byte values; certain Windows-1252 glyphs such as
`€` also have a recoverable mapping. Characters with no legacy-byte
representation produce an error, as do invalid byte sequences for the
chosen codepage. A replacement character `�` or a previous `?` substitution
usually means information was already lost; encoding conversion cannot
reconstruct it.

## Gotchas

- Do not run the action twice without checking the first result.
- Whole-library `_ALL` conversion can damage correct fields and names.
  Select files with the same confirmed problem first.
- Keep the original copy. The action writes when run; inspect operation
  errors and **Edit → Undo** for recorded changes.

<!-- verified: crates/tag-core/src/actions.rs:decode_codepage; src/action-ui.ts:codepageChoices -->

## Related

- [Import Text File](import-text-file.md)
- [Replace](replace.md)
- [Actions and action groups](index.md)
