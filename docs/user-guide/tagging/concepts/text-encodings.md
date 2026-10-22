# Text encodings and codepages

Why can the same title appear as `Café` in one file and `CafÃ©` in another?
The intended characters and the bytes used to store them are different
layers. An encoding tells a reader how to turn bytes back into characters.

## Characters are not bytes

Unicode identifies characters independently of a file format. For example,
`é` is U+00E9. UTF-8 and UTF-16 are different ways to encode that character:

| Encoding example | Bytes for `é` | What must be known when reading |
|---|---|---|
| UTF-8 | `C3 A9` | Interpret the bytes as UTF-8 |
| UTF-16 little-endian | `E9 00` | Interpret 16-bit units in little-endian order |
| ISO-8859-1 | `E9` | Use that restricted single-byte character set |

UTF-8 and UTF-16 can represent the Unicode character repertoire. A legacy
codepage represents a smaller repertoire and assigns particular meanings
to its byte values. Windows-1252 and ISO-8859-1 overlap for many characters
but are not identical encodings.

A byte-order mark (BOM) can identify a Unicode encoding at the start of a
text file: `EF BB BF` for UTF-8, `FF FE` for UTF-16 little-endian, and
`FE FF` for UTF-16 big-endian. It is an encoding marker, not an extra title
character the reader should display.

## How garbled text is created

Consider the UTF-8 bytes for the final character in `Café`:

```text
Intended character:       é
UTF-8 bytes:              C3 A9
Wrong Windows-1252 read:  Ã©
```

The wrong decoding produces real Unicode characters `Ã` and `©`. Saving
those characters correctly as UTF-8 preserves the wrong text; it does not
recover `é`. This kind of garbling is often called *mojibake*.

The direction of the mistake matters. Different legacy encodings can
interpret the same bytes differently, so selecting an encoding by the
language name alone can produce another plausible but wrong result.

## Tag format and text encoding are separate choices

An MP3 file can contain ID3 tags. The ID3 revision and selected text
encoding determine how its text frames are written. TagGroove's MPEG options
offer these combinations:

| ID3v2 write choice | Text encoding |
|---|---|
| ID3v2.3 UTF-16 | Unicode UTF-16 |
| ID3v2.3 ISO-8859-1 | Restricted Latin-1 |
| ID3v2.4 UTF-8 | Unicode UTF-8 |

The initial default is ID3v2.3 UTF-16. The Latin-1 choice cannot preserve
characters outside U+0000–U+00FF; TagGroove's conversion replaces those with
`?`. Keep originals before choosing a restricted encoding for compatibility.

This is metadata encoding. It does not change MP3 audio encoding,
bitrate or sound quality. Other tag formats have their own text-storage
rules; an ID3 option does not change how every audio format stores text.

## Reading external text files

Text import uses a decoding rule for the source file, separate from the
tag encoding used when the imported values are saved. For TagGroove's
structured text import, Import Text File action and export-template reader:

1. A supported Unicode BOM selects that decoding.
2. Without a BOM, valid UTF-8 is read as UTF-8.
3. Otherwise a supplied ANSI/legacy fallback is used; without one, the
   reader falls back to Windows-1252.

A fallback is not a command to reinterpret a file already accepted as
valid UTF-8. Save new text files as UTF-8. For UTF-16 text input, include
its BOM so the reader can identify the byte order. Invalid sequences can
report an error rather than being silently replaced.

## Repair and character-loss operations differ

The **Convert Codepage** action can recover some known mistakes by turning
the existing characters back into legacy bytes and decoding those bytes
with the chosen encoding. For example, choosing UTF-8 for `CafÃ©` recovers
`Café` when those characters still preserve the original bytes.

By contrast, `$ansi` limits text to characters available in the current
ANSI codepage. With Windows-1252 context:

| Expression | Result |
|---|---|
| `$ansi(Café)` | `Café` |
| `$ansi(猫)` | `?` |
| `$upper($ansi(Café))` | `CAFÉ` |

On a UTF-8 system context, `$ansi` preserves Unicode text. It is not an
automatic mojibake repair. A `?` or replacement character introduced by
an earlier lossy conversion usually means the original information is
already missing; another codepage cannot infer the lost name.

## What this means in TagGroove

Start by checking whether the text is wrong in the saved tag or only in
one player's display. If the tag contains correct Unicode, a destructive
repair action can make it worse. If the stored text is garbled, preserve
the original and test one diagnosed conversion on a copy.

Choose a Unicode tag encoding when the target player supports it. For
older players, verify the specific revision and encoding they need using
a practice file. Check accented characters, non-Latin names and artwork
after saving rather than treating an encoding label as a compatibility test.

<!-- verified: crates/tag-core/src/script.rs:ansi; crates/tag-core/src/actions.rs:decode_codepage; crates/tag-core/src/export.rs:decode_export_template; crates/tag-core/src/mapping.rs:Id3Target::encoding, latin1_lossy; crates/tag-core/src/preferences.rs:MpegTagOptions::default -->

## Related

- [Multi-value fields](multi-value-fields.md)
- [Format strings](../../scripting/format-strings.md)
- [Your first tagging session](../../getting-started/first-tagging-session.md)
