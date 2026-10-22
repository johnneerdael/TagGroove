# Glossary

Look up the words used for tagging, file organization and automation in this
guide. A field name such as `ARTIST` names a kind of information; a value
such as `Juniper Vale` is the information stored there.

!!! tip "New to this?"
    Start with [What tags are](../tagging/concepts/what-are-tags.md), then try
    [Your first tagging session](../getting-started/first-tagging-session.md).
    You do not need to learn the whole glossary before editing an album.

## Actions and calculated text { #automation }

| Term | Meaning |
| --- | --- |
| **Action** | One configured operation, such as replacing text in `TITLE` or exporting embedded covers. An action is not a general-purpose program. |
| **Action group** | A saved, ordered list of actions. Later steps can use values changed by earlier ones. A group is not an all-or-nothing transaction. |
| **Capture group** | A parenthesized part of a regular expression that remembers matched text for later use, for example the digits matched by `([0-9]+)`. The replacement context determines how to refer to it. |
| **Case-sensitive** | Treating uppercase and lowercase as different, so `Tide` and `tide` are not the same text. Matching defaults depend on the command. |
| **Converter** | A focused tool that turns information from one form into another, such as tags into filenames or filenames into fields. It does not mean converting audio encoding. |
| **Expression** | Text written in a tool's syntax to calculate or test something. A format string, filter expression and regex pattern have different jobs and rules. |
| **Format string** | Instructions that build text from literal characters, placeholders and scripting functions. `$num(%track%,2) - %title%` can build `03 - Paper Boats`. |
| **Function / scripting function** | A named calculation inside a format string, such as `$upper(...)` or `$num(...)`. Its arguments are the inputs between parentheses. |
| **Argument** | An input to a function. In `$num(%track%,2)`, `%track%` supplies the text/number and `2` supplies the minimum number of digits. |
| **Literal text** | Characters to use as written rather than interpret as an instruction. Quoting/escaping rules determine how to write special characters literally in each context. |
| **Optional section** | A format-string section in square brackets whose output depends on its evaluated contents. Follow the format-string rules; it is not a regex character class. |
| **Placeholder** | A field/information lookup enclosed in percent signs, such as `%artist%` or `%_filename_ext%`. A placeholder usually reads a value; it does not choose a write destination by itself. |
| **Preview** | A proposed result shown before an operation is applied. It is an inspection aid, not a backup or a reserved output path. A tag-source Preview button instead opens the source page. |
| **Quick action** | A route for configuring and applying one action without first saving it in a reusable action group. Check its settings and write timing. |
| **Regular expression / regex** | A pattern that recognizes a shape of text, such as digits at the start of a title. Matching finds text; replacement decides what to put in its place. |
| **Replacement** | The new text used by a replace operation. In a regex operation it may refer to captured text; exact syntax and quoting depend on the context. |
| **Selection preset** | A saved choice of checked action groups. It chooses groups to run; it does not store the selected audio-file rows or a copy of the music. |

See [Format strings](../scripting/format-strings.md),
[Regular expressions](../scripting/regular-expressions.md) and
[Actions](../actions/index.md) for the full rules.

## Tags, fields and credits { #tags }

| Term | Meaning |
| --- | --- |
| **Album Artist** | The credit used to organize an album as a whole, stored as `ALBUMARTIST`. It can differ from the artist of an individual track. |
| **Artist** | The track's performer credit in `ARTIST`. A field can contain more than one separate artist value when the tag format and workflow support it. |
| **Barcode** | An identifier printed on an edition's packaging. Keep it as text, including leading zeros. It is a search clue rather than proof that local files match the edition. |
| **Catalog number** | A label's identifier for a release, often containing letters, numbers and punctuation. Different editions can require additional details to distinguish them. |
| **Compilation** | An album combining tracks, often by several artists. Use album-level and track-level credits deliberately rather than replacing every artist with the same name. |
| **Custom field** | A metadata field outside the small set shown by default. Whether it can be saved and how another program reads it depend on the file's tag format. |
| **Disc number / disc total** | The disc's position and the number of discs in a set. They are separate ideas from the track's position and track total. |
| **Field** | A named part of metadata, such as `TITLE`, `GENRE` or `MOOD`. It holds zero, one or several values in TagGroove's view of the file. |
| **Genre** | A descriptive category stored in `GENRE`. Suggested values help consistent spelling; they do not guarantee that a category fits a recording. |
| **ISRC** | An identifier for a recording, stored as text. It is different from a release barcode, an album title or a file checksum. |
| **Mapping** | A rule connecting a logical field name to a stored/native field. A mapping changes how values are exposed or written; it does not translate words or calculate a new value. |
| **Metadata** | Information describing a file or recording, such as title, credits, numbering and artwork. Metadata is separate from the audio stream. |
| **Multi-value field** | A field containing several distinct values. Two `ARTIST` values are different from one value containing `Juniper Vale; Mira Solberg`. |
| **ReplayGain** | Loudness-related metadata a compatible player can use to adjust playback. Reading these tags is different from measuring loudness or rewriting audio samples. |
| **Sort field** | An alternate value used for ordering, such as a name arranged without its leading “The”. Setting it need not change the displayed performer credit. |
| **Tag** | The block of descriptive metadata stored with an audio file. Use “field” for an individual named entry and “value” for its contents. |
| **Tag format / tag type** | The storage convention for a tag, such as ID3v2.3, ID3v2.4 or APEv2. One file can contain more than one tag block with conflicting values. |
| **Track number / track total** | A track's position and the album/disc's count. `3/8` is a combined spelling containing both ideas; separate fields and player conventions can differ. |
| **Value** | The content of a field. In `TITLE = Paper Boats`, `TITLE` is the field name and `Paper Boats` is its value. |
| **Year / release date** | The year or date describing a release. A year-only value and a full date carry different detail; check the relevant source/converter before shortening one. |

See [Field names](../reference/field-names.md),
[Multi-value fields](../tagging/concepts/multi-value-fields.md) and
[Track and disc numbers](../tagging/concepts/track-and-disc-numbers.md).

## Files, formats and images { #files }

| Term | Meaning |
| --- | --- |
| **Absolute path** | A destination that supplies its filesystem root/drive rather than depending on a base folder. Compare a full `/Music/...` path with a relative `reports/list.txt`. |
| **Associated file / sidecar** | A separate file kept with audio, such as supported lyrics or karaoke data. It is not automatically embedded in the audio merely because its name matches. |
| **Audio stream** | The encoded sound inside the file, separate from tag fields and the filename. Changing a title or writing a report is not audio conversion. |
| **Bitrate** | The amount of encoded audio data used per second, often shown in kbps. A tag named BITRATE does not change the audio's measured bitrate. |
| **Chapter row / virtual row** | A row representing a part of a container or cue-sheet description rather than an independent ordinary audio file. Supported operations depend on its source. |
| **Codec** | The method used to encode/decode audio, such as FLAC or an MPEG audio encoding. A container and its codec are related but distinct. |
| **Container** | The file structure that holds audio and possibly metadata, pictures or chapters. A filename extension is a clue, not proof of every detail inside. |
| **Cover art / artwork** | Pictures associated with music. They can be embedded in audio or stored as separate files. |
| **Embedded cover** | Picture bytes stored inside the audio file's metadata. Copying only the audio can carry that picture, subject to the receiving player's format support. |
| **Extension** | The suffix after the filename's final dot, such as `.mp3`, `.flac` or `.png`. Renaming the suffix does not convert the file's encoding. |
| **Filename / basename** | The file's name. Some tools use “basename” for the name without its extension; the placeholder reference distinguishes `%_filename%` and `%_filename_ext%`. |
| **Folder image** | A separate image such as `folder.jpg` beside the music. TagGroove can show a folder-image fallback even when the audio has no embedded cover. |
| **Front Cover** | A picture role identifying the intended front of the album artwork. The role and the picture's JPEG/PNG encoding are separate properties. |
| **JPEG / PNG** | Image encodings. Cover export preserves the actual image format unless an operation explicitly converts it. A PNG renamed `.jpg` is still PNG data. |
| **Lossless / lossy** | Lossless compression preserves the represented audio samples; lossy compression discards information to reduce size. Tag editing does not turn one into the other. |
| **Padding** | Reserved tag space that can allow some metadata updates without moving as much surrounding file data. It is unrelated to adding zeros to a track number. |
| **Path** | The location of a file, including its folders and name. A provider document's address is not necessarily an ordinary filesystem path. |
| **Picture type / role** | A label such as Front Cover or Back Cover describing a picture's purpose. Setting a role does not improve its resolution or image quality. |
| **Relative path** | A location interpreted from a base folder. In HTML, `folder.jpg` is relative to the page; an export's relative destination follows its documented export base. |
| **Resolution / image dimensions** | The picture's width and height in pixels. Requesting a larger provider variant does not guarantee that it contains more actual detail. |
| **VBR** | Variable bitrate: the encoded bitrate can vary through the track. A displayed average does not describe every moment of it. |

See [Supported file formats](../reference/file-formats.md),
[Cover art](../tagging/concepts/cover-art.md) and
[Chapters and cue sheets](../tagging/chapters.md).

## Selection, saving and recovery { #saving }

| Term | Meaning |
| --- | --- |
| **Backup** | A separate retained copy you can use for recovery. A preview, cached Library entry or saved configuration is not an audio backup. |
| **Extended tags** | The editor for inspecting more fields and explicit separate values, beyond the commonly shown tag-panel controls. |
| **File list** | The rows of opened files/information in TagGroove. Removing a row from the list differs from deleting its file on disk. |
| **Filter** | A condition limiting which file-list rows are visible. It does not remove the other files from disk. |
| **History / Undo** | Recorded desktop-session operations that can be inspected and replayed when available. History has limits and is not a permanent backup restored at the next start. |
| **Library** | A cached catalog of configured music folders used for browsing/searching. It is separate from the underlying files and does not back them up. |
| **Pending edits / unsaved changes** | Intended changes currently staged in the editor, before their save boundary. Other operations can write immediately; do not apply this timing to every command. |
| **Provider document** | A file made available through a document-access provider, such as the Android document picker. Access, writable state and recovery follow that provider workflow. |
| **Read-only** | Available to inspect but not overwrite through the current route. It can describe a document's access or a calculated column with no editable field target. |
| **Recovery copy** | A retained original baseline or edited working copy used when a provider save is interrupted or uncertain. Saving a new copy does not certify that the original destination was repaired. |
| **Reload tags** | Read the file's currently stored metadata again. Confirm discarding pending edits only when you intend to lose those edits. |
| **Save a copy** | In the provider-document workflow, create another document from an edited copy or original backup at a chosen destination, retaining the source and recovery state. |
| **Selection** | The particular rows a command targets. Visible rows, all loaded rows and selected rows can be different sets. |
| **Tag panel** | The editable area containing common fields and cover controls for the selection. Layout labels can differ from the actual field names. |

The special tag-panel choices **`< keep >`** and **`< blank >`** are not
ordinary metadata values. Keep preserves existing values for that operation;
blank requests removal. An untouched mixed checkbox similarly keeps each
file's value until an actual on/off choice is made. See the
[tag-panel guide](../tagging/tag-panel.md) for context.

## Online sources and outputs { #sources-and-outputs }

| Term | Meaning |
| --- | --- |
| **API key / access token** | A credential used by a provider. Availability and storage differ by source. Keep credentials out of shared configurations, screenshots and issue reports. |
| **Artwork archive** | A source of cover images. MusicBrainz artwork is requested from the Cover Art Archive; a release record does not guarantee a downloadable cover. |
| **Configuration** | Saved settings and reusable definitions, such as layouts, actions or export templates. A whole-app backup bundle has a defined scope and is separate from audio. |
| **CSV** | A text-table format with delimiters and quoting rules. Spreadsheet import settings can change numbers, dates or leading zeros even when the exported text is correct. |
| **Cue sheet** | A text description of tracks/files and timing information. It is not an audio file and does not combine separate recordings into one. |
| **Export** | Create a separate text report from file/tag information. The report is an output snapshot, not a two-way synchronized tag editor. |
| **Export configuration / template** | Reusable instructions describing an export's contents and defaults. An `.mte` stores a template; the generated HTML/CSV/text file stores its rendered result. |
| **Fingerprint** | A computed description of audio used to seek a recording match. AcoustID receives a fingerprint and duration, rather than the full audio file. |
| **HTML entity / escaping** | A way to represent a character safely in HTML text, for example `&amp;` for `&`. HTML text escaping is different from encoding a URL or an attribute. |
| **Loop** | An export-template section repeated for input rows or groups. An inner loop works within the rows supplied by its outer group. |
| **MBID** | A MusicBrainz identifier. Release, release-group, recording, artist and work IDs identify different kinds of records and are not interchangeable. |
| **Partition** | An export choice that divides input rows into separate output documents by a calculated key. Every document needs a destination that avoids collisions. |
| **Playlist** | A list of references to audio files for playback. It does not contain the music bytes and can break when its paths no longer resolve. |
| **Recording** | A particular recorded performance/audio identity. Several releases can include the same recording. |
| **Release / edition** | A particular publication of an album or single, distinguished by details such as country, date, format, label and tracklist. |
| **Release group** | A MusicBrainz grouping of related releases/editions. A release-group address is not a release address for the MusicBrainz tagging lookup. |
| **Tag source** | A provider of proposed metadata, lyrics or artwork. Review its fields and track/file matches before saving. |
| **Web source** | An installed `.src` definition that searches and parses a website. It is different from a built-in provider client, and can need updates when the website changes. |

## Text encodings and downloads { #encodings-and-downloads }

| Term | Meaning |
| --- | --- |
| **BOM / byte-order mark** | Leading bytes that can identify a Unicode text encoding. Whether to write one depends on the output format and receiving program. |
| **Checksum / SHA-256** | A calculated file digest used to compare a download with the release's published value. A matching digest does not substitute for trusting the source or a publisher signature. |
| **Codepage / encoding** | A rule mapping text characters to bytes. Reading bytes using the wrong rule can produce garbled text. |
| **Mojibake** | Garbled text caused by an encoding mix-up. Guessing a conversion repeatedly can damage text that was already correct. |
| **Prerelease / beta** | A release offered for testing before a full release. Including it in an update check does not install it. |
| **Simulator preview** | A development-environment application targeting a simulator. The current iOS archive is not a physical iPhone/iPad installer. |
| **UTF-8 / Unicode** | Unicode defines characters; UTF-8 is a common way to encode them as bytes. Correct encoding and correct font display are separate concerns. |
| **KiB / MiB** | Binary size units: 1 KiB is 1,024 bytes and 1 MiB is 1,048,576 bytes. These are not the same quantities as decimal kB/MB. |

## Related

- [What tags are](../tagging/concepts/what-are-tags.md)
- [Format strings from zero](../scripting/format-strings-from-zero.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Frequently asked questions](faq.md)
- [Reference overview](../reference/index.md)

<!-- Candidate evidence: guide style terms and _snippets/abbreviations.md; source-backed concepts, reference and task pages linked above. This glossary explains terms; it does not claim every format/provider/device supports every described capability. Definitions were reviewed against current guide/source context without executing workflows. -->
