# Track and disc numbers are metadata, not list positions

`TRACK=3/8` conventionally describes track 3 of 8, while
`DISCNUMBER=2/3` describes disc 2 of 3. Each is one text value in OpenTag's
editing model. It is not a list containing two separate values, a fraction
to calculate, or proof of how many files are currently selected.

## Separate the number from its display

For this practice snapshot, the title is `Paper Boats`. The operation is
currently processing its fifth file:

<!-- number-concept-fields:start -->
| Kind | Name | Values |
|---|---|---|
| Metadata | `TITLE` | `["Paper Boats"]` |
| Metadata | `TRACK` | `["3/8"]` |
| Metadata | `DISCNUMBER` | `["2/3"]` |
| Information | `_COUNTER` | `["5"]` |
<!-- number-concept-fields:end -->

<!-- number-concept-expressions:start -->
| Format string | Calculated text |
|---|---|
| `%track%` | `3/8` |
| `$num(%track%,2)` | `03` |
| `$num(%discnumber%,1)` | `2` |
| `$num(%discnumber%,1)-$num(%track%,2) - %title%` | `2-03 - Paper Boats` |
| `$num(%_counter%,2)` | `05` |
<!-- number-concept-expressions:end -->

`$num` reads a numeric prefix and pads it; it does not preserve `/8` or
save the total somewhere else. The fourth expression calculates a useful
filename stem without changing the stored track/disc fields. Applying a
naming converter is a separate step.

`%_counter%` follows the current operation's file order. It is not the saved
track number: here the counter is 5 while TRACK begins with 3. Using it on
a filtered subset can generate a different sequence from an album's existing
numbers.

## Padding is a representation choice

`3`, `03` and `003` can denote the same current track. Padding is useful
in names sorted as text, but some native formats store numbers as integers
and do not preserve those zeros. Use `$num(%track%,2)` when building padded
filenames even if saved MP4 track metadata reads back as `3/8`.

A two-digit minimum does not truncate track `100`. Conversely, a nonnumeric
value such as `B2` is not converted into a vinyl-side numbering system by
`$num`; its numeric prefix is absent. Preview mixed conventions before
renumbering them.

## Totals and native storage differ by format

In ID3v2, TRACK and DISCNUMBER map to `TRCK` and `TPOS`. MP4 stores a current
number and total in native `trkn` and `disk` pairs. Its mapped total fields
are `TOTALTRACKS` and `TOTALDISCS`; a read can expose both `TRACK=3/8` and
`TOTALTRACKS=8`. Conflicting simultaneously supplied totals are rejected,
and native MP4 components use unsigned 16-bit integers.

`TRACKTOTAL` and `DISCTOTAL` are not universal automatic aliases for those
MP4 fields in this baseline. A similarly named custom field can coexist
without configuring a player's native total. Other formats may expose
native names such as `TRACKNUMBER`; inspect the actual file and mappings
before assuming a blank `%track%` means no stored number exists.

See [read or split track totals](../cookbook/split-track-total.md) before
separating a slash value. A slash-splitting pattern alone does not validate
numbers or check that current track is within the total. A partial selection
also does not establish a whole album's correct total.

## Order files before assigning new numbers

Sorting changes the file list's view; it does not by itself rewrite TRACK.
**Auto-numbering** uses the selected files' current list order, shows a
preview and stages numbers as pending edits. **Save tags** then writes them.
Changing its inputs or selection invalidates the reviewed sequence.

Folder/field counter changes detect transitions between adjacent selected
files. Interleaving discs can create repeated resets rather than one global
group per album. In the per-folder shortcut, a saved total is the final
assigned number in that run; starting at 5 with three files gives total 7.
The regular wizard's entered fixed total is a different mode.

A disc counter and a track counter have separate controls. Enabling a disc
field does not infer disc boundaries from audio, and a directory transition
can restart tracks without writing DISCNUMBER. Review
[Auto-numbering](../converters/auto-numbering.md) before staging a batch.

## Check copies and saved results

Use a small copied selection while learning, then inspect the actual pending
values and save outcome. A number shown in a preview or filename does not
prove the native tag was changed. Actions and applied converters can write
immediately, unlike staged numbering; follow each command's commit step.

Use **Edit → Undo …** for recorded saved changes in the current desktop
session, and retain original files when old ordering/totals must remain
recoverable beyond it.

## Related

- [Auto-numbering](../converters/auto-numbering.md)
- [Number tracks per folder](../cookbook/number-tracks-per-folder.md)
- [Disc numbers from folders](../cookbook/disc-numbers-from-folders.md)
- [Field names and storage](../reference/field-names.md)
- [What tags are](what-are-tags.md)

<!-- verified: crates/tag-core/src/script.rs:num; crates/tag-core/src/numbering.rs; crates/tag-core/src/mapping.rs:patch_mp4,mp4_fields; src/numbering-review.ts; src/main.ts:numbering. Snapshot expressions execute against core; source-linked numbering/native-pair limits, not native/player runtime proof. -->
