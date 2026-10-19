# ReplayGain and loudness

Why can two tracks have the same peak level but sound different in volume?
Peak amplitude and perceived loudness describe different things. ReplayGain
stores measured playback adjustments as metadata; a supporting player can
use them when playing the audio. Editing those fields is different from
measuring the recording or changing the encoded audio samples.

## The four common fields

| Field | Meaning |
|---|---|
| `REPLAYGAIN_TRACK_GAIN` | Gain adjustment measured for this track. |
| `REPLAYGAIN_ALBUM_GAIN` | Gain adjustment measured for the album as a group. |
| `REPLAYGAIN_TRACK_PEAK` | Peak amplitude recorded for this track. |
| `REPLAYGAIN_ALBUM_PEAK` | Peak amplitude recorded for the album. |

Track mode levels individual songs. Album mode uses a common adjustment to
retain the loudness differences within an album. Negative gain means reduce
level; positive gain means increase it. Peak metadata can help a player
avoid clipping after gain, subject to its settings. See the
[ReplayGain specification](https://wiki.hydrogenaudio.org/index.php?title=ReplayGain_1.0_specification)
for the measurement and playback model.

A stored gain is not a volume percentage, a peak reading, or the track's
bitrate. Changing the number by hand does not run a loudness analysis.
Whether playback uses track gain, album gain, additional preamp gain or no
ReplayGain depends on the player. TagGroove's field display is not a playback
meter.

## Read the stored text before changing it

The following values are invented metadata for a practice copy. They are
not measurements of the guide's example album and should not be copied to
real tracks as a normalization recipe.

<!-- replaygain-concept-fields:start -->
| Field | Practice value |
|---|---|
| `REPLAYGAIN_TRACK_GAIN` | `-4.25 dB` |
| `REPLAYGAIN_ALBUM_GAIN` | `-3.00 dB` |
| `REPLAYGAIN_TRACK_PEAK` | `0.950000` |
| `REPLAYGAIN_ALBUM_PEAK` | `0.990000` |
<!-- replaygain-concept-fields:end -->

TagGroove can inspect these stored fields in **Extended tags**. A custom
column or export can read their placeholders, for example
`%replaygain_track_gain%`. Unknown text fields use the tag family's supported
custom-field path, so storage and player interpretation can differ by format.
ID3v1 has no general custom-field space for this set. Do not assume that an
arbitrary similarly named field is an alias for another loudness scheme.

For a deliberate metadata correction, **Keep changes** stages Extended tags
edits and **Save tags** writes them. Actions and converters have their own
write timing. Reopen/read back the saved file before deciding that a player's
lack of volume change is a failed tag save. It may be ignoring the field or
using a different playback mode. Use an audio-analysis tool that supports
your target workflow to calculate gains from the recording; this guide's
TagGroove workflows edit the stored metadata.

## Preserve the native tag as well as the displayed value

An MP3 can have ReplayGain or MP3Gain recovery data in an APEv2 tag while
TagGroove displays ID3v2. With **Mpeg → Write APEv2** off, an ordinary save
preserves the existing APE tag. **Remove APE** applies to **Remove tag…**;
it does not turn an unrelated Save into APE removal.

New settings leave APE **Read** off and ID3v2 **Read/Write** on. An existing
saved APE reading choice stays in effect. A hidden APE gain is retained in
APE; it is not automatically copied into ID3v2. Conflicting gains therefore
remain separate, and the reading policy determines which one is displayed.

These practice cases use APE **Read/Write** off, ID3v2 **Read/Write** on and
APE **Remove** on. Each starts with ID3v2 `TITLE` = `Paper Boats`. Save
`ARTIST` = `Juniper Vale`, then inspect ID3 and APE separately:

<!-- replaygain-concept-migration:start -->
| APE track gain before | ID3v2 track gain before | Saved ID3v2 track gain | Retained APE track gain |
|---|---|---|---|
| `-4.25 dB` | Absent | Absent | `-4.25 dB` |
| `-4.25 dB` | `-1.50 dB` | `-1.50 dB` | `-4.25 dB` |
| Absent | Absent | Absent | Absent |
<!-- replaygain-concept-migration:end -->

To inspect the retained APE values, enable APE reading and reload the copy.
Switch reading back to ID3v2 when comparing that tag. Reading a different tag
changes the view; it does not merge gains or recalculate loudness.

Explicitly enabling **Write APEv2** updates that tag from the values being
saved. Writing ID3 and APE together can synchronize their displayed fields;
it does not preserve independently conflicting values inside both written
tags. **Remove tag…**, field-removal actions and keep-list actions can erase
loudness or recovery information, so review those operations separately.

`MP3GAIN_UNDO` is another tool's restoration information, not a ReplayGain
track gain or TagGroove's **Edit → Undo** history. Keeping similarly named
text in ID3 does not replace the native APE data a gain tool may need. The
preservation checks inspect native APE fields and file history; they do not
execute MP3Gain itself or prove another player's interpretation.

## What this means in TagGroove

Before cleaning tags, inspect the actual tag families and stored field names,
and retain original files. Keep track and album measurements together where
your workflow needs both. If the audio was replaced, edited or transcoded,
consider whether the old measurements still describe that recording rather
than copying numbers blindly. After an operation, verify the written fields
and the receiving player's settings separately.

Removing gain metadata does not itself reverse audio changes made by a
different program. TagGroove session undo can restore recorded file operations
while available; it does not reconstruct missing analysis or another tool's
lost recovery information.

## Related

- [Tag formats and read/write policy](tag-formats.md)
- [Extended tags](../guides/extended-tags.md)
- [Mpeg tag preferences](../reference/options/tags-mpeg.md)
- [Remove unwanted fields with a keep list](../cookbook/remove-junk-fields.md)

<!-- verified: media_concept_guide.rs round-trips the four written values/deletion and three undisplayed-APE migration cases against real generated MP3 fixtures under LF/CRLF, retaining audio payloads. Source tag_policy.rs and tags.rs; no loudness analysis or player playback test. -->
