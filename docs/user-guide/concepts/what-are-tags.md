# What tags are

Tags are descriptive data stored with an audio file: a title, artist,
album, track number or embedded picture. The audio stream, the file's path
and a player's library records are separate things. Understanding that
separation makes an unexpected display easier to diagnose.

## One recording, several descriptions

Imagine a copied file named `07 - Old Label.mp3` whose stored title is
`Paper Boats`. Those names can disagree without the audio being different.
A player might show the tag title, a filename fallback or its own cached
library information. Check which source it is using before changing every
copy of the file.

OpenTag's tag fields describe the file; changing `TITLE` does not automatically
rename it. A file-list filename edit or a rename converter changes its path
without automatically changing the title. An export creates another output;
it does not turn the report text into saved tags.

## Fields hold values

A field name identifies the kind of information. A value is its content.
`ARTIST` and `ALBUMARTIST` are different fields, not two spellings of the
same credit. A field can have several values, such as two artists, rather
than one string containing punctuation.

This practice snapshot distinguishes stored metadata from information
computed for the current file:

<!-- tag-basics-fields:start -->
| Kind | Name | Values |
|---|---|---|
| Metadata | `TITLE` | `["Paper Boats"]` |
| Metadata | `ARTIST` | `["Juniper Vale","Mira Solberg"]` |
| Metadata | `ALBUM` | `["Harbour Lights"]` |
| Metadata | `TRACK` | `["3/8"]` |
| Information | `_FILENAME_EXT` | `["07 - Old Label.mp3"]` |
<!-- tag-basics-fields:end -->

The brackets illustrate lists, not characters to type into the tag panel.
`TRACK=3/8` here is one value describing track and total. Two ARTIST values
carry separate credits. A semicolon inside one value remains punctuation
unless an operation explicitly splits it.

## Reading a value is not writing it

Format strings read fields and calculate text. For the snapshot above:

<!-- tag-basics-expressions:start -->
| Format string | Calculated text |
|---|---|
| `%title%` | `Paper Boats` |
| `%artist%` | `Juniper Vale` |
| `$meta_sep(artist,' & ')` | `Juniper Vale & Mira Solberg` |
| `%_filename_ext%` | `07 - Old Label.mp3` |
| `$num(%track%,2) - %title%` | `03 - Paper Boats` |
<!-- tag-basics-expressions:end -->

A bare placeholder reads the first value; `$meta_sep` intentionally joins
the artist list for this display. Calculating `03 - Paper Boats` does not
rename the file. A converter or action has to use that result as a naming
target and be applied before a path changes.

Information placeholders such as `%_filename_ext%`, `%_codec%` and
`%_bitrate%` come from the file or operation. Typing a tag field named
`CODEC` does not transcode the audio. Use the documented naming targets
when a path change is intended, rather than treating all displayed
information as writable tag fields.

## The field name and native storage are different

`TITLE` is a logical name. A mapping connects it to native storage: for
example, ID3v2 `TIT2` or an MP4 title item. A filename extension is not a
universal tag format. One MP3 can contain several tag blocks with different
values, and different readers can prioritize them differently.

[Tag formats](tag-formats.md) explains those choices. The
[field reference](../reference/field-names.md) explains meanings and mappings.
A successful text calculation does not prove that every destination format
can store multiple values, long Unicode text, artwork or custom fields.

## Pending edits and saved files

In the tag panel, typing stages edits for the selection. **Save tags** writes
the selected pending changes. **Extended tags…** can show several values
explicitly. Actions and applied converters have their own immediate-write
behavior; they do not all wait for Save tags.

A player's old library display does not establish that a save failed, and an
updated OpenTag display alone does not establish a completed disk write.
Inspect the operation result, read the file back, and check the receiving
player's import/cache behavior when displays disagree.

Work on copies while learning. Native-format and source-access limitations
still apply; retained backups matter beyond the current desktop undo session.

## Related

- [Your first tagging session](../learn/first-tagging-session.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Multi-value fields](multi-value-fields.md)
- [Tag formats](tag-formats.md)
- [Track and disc numbers](track-and-disc-numbers.md)

<!-- verified: crates/tag-core/src/tags.rs:Fields, technical_fields; crates/tag-core/src/script.rs; src-tauri/src/evaluation.rs; src/main.ts:draftField,saveDrafts. Snapshot expressions execute with separate raw/enriched field maps; no player/native UI claim. -->
