# Tag formats and why readers can disagree

A tag format is the structure used to store metadata. It is distinct from
the audio codec and container. An MP3 stream can carry ID3 and APE blocks;
AAC audio inside MP4 uses MP4 metadata, while a raw `.aac` stream follows a
different supported tag path.

## Choose a format by the file, not by its name alone

Changing an extension does not transcode audio or install another tag format.
OpenTag checks file contents and uses the supported writer for that format.
The [format reference](../reference/file-formats.md) lists exact current
read/write/remove capabilities and exceptional layouts.

| Metadata family | Where it is relevant | Important distinction |
|---|---|---|
| ID3v1 | Legacy MPEG and supported APE-family paths | Small fixed fields; no arbitrary custom fields or embedded artwork |
| ID3v2 | MPEG and supported ID3 paths such as AIFF, WAV, DSF/DFF | Richer frames, revisions and text encodings; limits still vary by field/container |
| APEv2 | Monkey's Audio, Musepack, WavPack and other supported APE paths; can coexist on MPEG | Separate block, including custom fields and supported artwork |
| Vorbis-style comments | FLAC and Ogg-family formats such as Vorbis/Opus/Speex | Native named entries; repeated values and picture storage follow the format |
| MP4 items/atoms | MP4/M4A/M4B and related files carrying AAC/ALAC or other streams | Native items such as title, track pairs and artwork; not an ID3 tag added by renaming |
| ASF attributes | Windows Media / ASF | Native attributes and pictures, with stream-specific limits |
| Matroska tags/attachments | Matroska and WebM | Scoped tags, supported chapters and image attachments; target levels matter |
| RIFF INFO | Supported WAV metadata | Different capacity from ID3v2, which a WAV can also contain |

This table is an orientation. Not every field or image type works
identically in each family. In particular, AIFF's supported editing
path is ID3v2 here; do not infer other editors' native chunk behavior.

## One MP3 can contain conflicting blocks

Suppose one copied practice MP3 physically contains:

<!-- format-stored-blocks:start -->
| Block | TITLE | ALBUM |
|---|---|---|
| ID3v1 | `Old Short Title` | Absent |
| ID3v2 | `Paper Boats` | `Harbour Lights` |
| APEv2 | `Archive Title` | Absent |
<!-- format-stored-blocks:end -->

OpenTag's Mpeg reading policy chooses an enabled block in priority order:
APE, then ID3v2, then ID3v1. It does not merge an album from one block into
a title selected from another. The following Read switches produce these
results for that same physical file after pending edits are resolved and
the file is read again:

<!-- format-read-priority:start -->
| Read ID3v1 | Read ID3v2 | Read APE | TITLE shown | ALBUM shown |
|---|---|---|---|---|
| On | On | On | `Archive Title` | Absent |
| On | On | Off | `Paper Boats` | `Harbour Lights` |
| On | Off | Off | `Old Short Title` | Absent |
| Off | Off | On | `Archive Title` | Absent |
<!-- format-read-priority:end -->

Seeing a blank album with APE enabled does not establish that the ID3v2 album
was erased. Inspect present/read tag information and the Read options before
“repairing” a value that another block already contains. A different player
can choose another block or keep a library cache; investigate the actual file
and reader settings separately.

## Read, Write and Remove are independent decisions

**Read** chooses the displayed metadata. **Write** selects supported tag types
to receive saved data. **Remove** chooses types for **File → Remove tag…**;
saving also removes a selected type when it is not being written.

For example, disabling Write APEv2 while leaving Remove APE enabled can strip
an APE tag during an ordinary save. Saving can synchronize several written
types from the displayed values; conflicting hidden blocks are not guaranteed
to retain independent versions of every field. Keep original files before
changing policies on a mixed collection.

These Mpeg options also serve supported raw AAC and TTA paths. The
**Ape, Mpc** family uses APE ahead of ID3v1 and has its own write/remove
choices. Other containers use their own supported native structures. The
[Tags options](../reference/options/tags.md) and its child pages provide the
actual controls, defaults and restrictions.

## Revision, encoding and logical field names

The MPEG ID3v2 default is **ID3v2.3 UTF-16**. **ID3v2.4 UTF-8** is another
Unicode representation; **ID3v2.3 ISO-8859-1** is a legacy choice that can
lose characters outside its repertoire. Choose compatibility from the actual
consumer's requirements and test a copy, rather than assuming the oldest
or newest spelling is universally best.

A logical field can map to different native identifiers. `TITLE` maps to
ID3v2 `TIT2`, for example. Native field names can also remain visible in
APEv2 or Vorbis Comments; `DATE` is not automatically the same displayed
field as an MP3's `YEAR`. Custom mappings can change that relationship.
Use the [mapping reference](../reference/tag-mapping.md) before rewriting a
field merely because two formats name related information differently.

## Know what a successful write establishes

Writers have field, artwork, numeric and structural limits. An unusual
container can be readable but refused for writing; opaque or unsupported
data can follow preservation rules rather than appearing in the editor.
A successful core operation on one fixture is not a complete player or
native-device compatibility guarantee for every file in that family.

Inspect the operation result, saved read-back and intended consumer. Keep
backups, including original metadata blocks when their differences matter.

## Related

- [What tags are](what-are-tags.md)
- [Supported formats and limits](../reference/file-formats.md)
- [Tags options](../reference/options/tags.md)
- [Removing tags](../guides/remove-tags.md)
- [Multi-value fields](multi-value-fields.md)

<!-- verified: crates/tag-core/src/tag_policy.rs:read_order,displayed_tags,write; crates/tag-core/src/preferences.rs; crates/tag-core/src/mapping.rs; format/container modules. Exact stored-block/read-switch example executes on independently generated MPEG frames with all three tag blocks; no native UI/all-format claim. -->
