# Tags → Advanced

Open **File → Options… → Tags → Advanced** for format-specific interoperability choices. Change one setting at a time when investigating a particular player's behavior.

!!! tip "New to this?"
    These are storage and interpretation choices. Keep a copy of a representative file and check it in the program that motivated the change; TagGroove source checks do not establish that program's compatibility.

## ID3v2 comment language code

**ID3v2 comment language code (ISO-639-2)** defaults to `eng`. Enter a three-letter alphabetic code, such as `deu` for German. TagGroove normalizes the stored language code to lower case when writing new comment frames. A two-letter interface locale such as `de` is not valid here.

This is an identifier in an ID3v2 comment frame, not an automatic translation of the comment text or the app interface. Some readers choose comments by language identifier. The original Mp3tag compatibility guidance says localized Windows Explorer or Windows Media Player may need the Windows language’s three-letter code, such as `deu`, while iTunes requires `eng` and can ignore comments with another code. Those are the documented historical reader distinctions; this guide does not verify those programs’ current behavior. Keep `eng` unless there is a tested reason to change it, then confirm the target reader displays the comment.

## Always display POPULARIMETER values for ID3v2 ratings

Defaults to off. Normally supported player-specific rating frames can appear through convenience fields such as `RATING MM` or `RATING WMP`. Enable this to expose ID3v2 POPM values as `POPULARIMETER` instead.

The raw field includes an owner identifier, rating byte, and play counter in `owner|rating|counter` form. It is not the same text representation as a star rating. For example, changing display mode does not mean a raw value of `128` is a 128-star score. Reload tags to inspect the new field view before editing or filtering ratings.

## List chapters as separate files

Defaults to off. Enable and reload the directory to list supported chapters as individual rows. Supported source paths include MP4 chapters, Matroska chapters, ID3v2 `CHAP`, and embedded cue sheets. This is useful for editing chapter titles; disable it when working with the file's global tags as one row.

A chapter row refers to part of its parent media file. It is not a newly split audio file. `%_subsong_count%` identifies the count of subsongs/chapters and `%_subsong%` their zero-based index, useful when adding diagnostic file-list columns. Reload the directory after changing the option so row expansion can be rebuilt.

## Use recommended mapping for Matroska video

Defaults to on. Matroska tags can target different semantic levels. The same native name can mean different things at different levels: for example, `PART_NUMBER` can represent a track at target type 30 or a disc at target type 50.

For files containing video, this option uses the recommended video mapping, including title at target type 50, album at 60, and episode/season semantics. Disable only when a particular consumer expects an older mapping such as title at target type 30. Reload tags after changing the choice. Changing the view alone does not rewrite all existing Matroska tags.

## Use single MP4 atom for multiple values

Defaults to on. Store a field's multiple values as several data entries inside one field atom. With it off, use a separate field atom per value. The conceptual field still has several values; this changes their container arrangement.

For example, two artist values can be represented under one artist atom with two data entries, or two artist atoms. Different players may interpret these arrangements differently. Test with the actual consumer rather than joining distinct artists into one string merely to work around a display issue.

## Reuse unmodified MP4 atoms

Defaults to off. With reuse enabled, an unchanged existing field can stay in its original storage rather than being remapped on a save. This matters for user-defined atoms whose field name has a documented standard atom: the existing atom remains until the value changes. A changed value is written through the normal mapping.

Leave it off when normalizing such fields to the documented representation is desired. Enable when an established collection or legacy reader relies on the existing arrangement. This is a field-storage preservation policy, not a guarantee that the complete MP4 file remains byte-for-byte unchanged after editing another field.

## Reuse unmodified ID3v2 frames

Defaults to off. The equivalent ID3v2 policy can preserve an unchanged user-defined `TXXX` frame when that field also has a documented native frame mapping. Without reuse, saving can normalize it to the mapped frame even though its text was not edited. With reuse, changing the field's value triggers writing through its normal mapping.

The option does not change the meanings of [user-defined field aliases](tags-mapping.md). It determines whether an unchanged native representation should be reused while a tag is being written.

## Related

- [Mpeg version and encoding](tags-mpeg.md)
- [Field mappings](tags-mapping.md)
- [Tags and folder images](tags.md)
- [Files and timestamps](files.md)

<!-- verified: crates/tag-core/src/preferences.rs AdvancedTagOptions; crates/tag-core/src/mapping.rs ID3 POPM mapping and comment language; crates/tag-core/src/tags.rs Id3Write, save_preserving_native; crates/tag-core/src/matroska.rs; crates/tag-core/src/subsongs.rs; crates/tag-core/tests/tag_policy.rs and formats_edge.rs. No current iTunes/Explorer/WMP runtime assertion. -->
