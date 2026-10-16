---
status: draft
---

# Supported file formats

!!! abstract "This page is being written"

    It will cover:

    - Every extension, its tag types (read/write), chapters, covers, technical info

    Until then, the [reference section](index.md) has the details.

## Format preservation notes

These details describe supported edge cases; they are not a complete
format-support matrix.

| File or tag | Behavior and limit |
|---|---|
| MP4 with iTunes and Nero metadata | iTunes fields take precedence; Nero supplies missing values. Editing or deleting a field also updates its existing Nero copy, including mapped field names. OpenTag does not add new fields to the Nero set. |
| Nero artwork in MP4 | When no iTunes artwork exists, Nero artwork is shown. Editing covers writes them as iTunes artwork and removes the Nero artwork copy, including a copy previously hidden by iTunes covers. Ordinary field edits preserve Nero artwork. |
| Unusual MP4 purchase IDs | Nonstandard `plID` items are preserved across unrelated edits. Values of 1–8 bytes can appear as `ITUNESALBUMID`; editing the field writes the standard 8-byte representation. |
| Raw `.aac` streams | Detection accepts up to 64 KiB of padding or junk after leading ID3v2 tags, followed by three consecutive compatible ADTS frames. The search is limited to `.aac` files and stops at an MPEG audio frame. Recognized streams use the applicable APE/ID3v1 options; intervening bytes are retained. |
| Header-less APEv2 tags | Footer-only tags are read without treating the preceding 32 audio bytes as a header. An internal `TAG` sequence is not treated as an ID3v1 tag. APE tags left untouched by the selected write options retain their bytes. |
| DSF / DSDIFF ID3v2 | Unedited compressed or opaque encrypted frames can be preserved byte for byte. Preserving encrypted bytes does not decrypt their contents. Unsynchronization, extended headers, footers and merged duplicate frames still limit preservation. |

For MP4 Nero fields without chapters, a longer edited value that does not fit
the existing Nero space is written to the iTunes tag only. The older Nero
value remains hidden by iTunes precedence. MP4 `udta/indx` data is preserved
but is not exposed for editing.

These behaviors are checked with constructed fixtures. They do not establish
that files written by every Nero or Mp3tag version behave identically. Keep
copies of unusual files and inspect the saved tags in the application that
will use them.
