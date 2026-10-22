# Tags → Mpeg

Open **File → Options… → Tags → Mpeg** to control ID3v1, ID3v2, and APE tags in MPEG-family audio. This policy also serves raw AAC and TTA through TagGroove's format adapters. An AAC stream inside an MP4/M4A container instead uses MP4 tags.

<!-- figure:options-tags-mpeg -->
<figure markdown>
![The tags mpeg page of the Options dialog](../../assets/screens/options-tags-mpeg-light.png#only-light)
![The tags mpeg page of the Options dialog](../../assets/screens/options-tags-mpeg-dark.png#only-dark)
</figure>
<!-- /figure -->

!!! tip "New to this?"
    A file can hold several independent tag blocks with conflicting values. Check what is read before choosing what to write or remove. Try a copied file before changing a library-wide policy.

## Read

New settings enable **ID3v1** and **ID3v2** reading and leave **APE** reading off. Existing saved choices stay in effect. TagGroove displays one enabled, present tag type, in this priority order:

| Priority | Switch | Reads |
|---|---|---|
| 1 | **APE** | APEv1 or APEv2. |
| 2 | **ID3v2** | ID3v2 metadata. |
| 3 | **ID3v1** | The older fixed-size tag. |

If a higher-priority enabled type is absent, the next present type is used. Contents are not assembled field by field from all three. For example, a file with an APE title and a different ID3v2 title shows the APE title while APE reading is enabled. Disabling APE reading reveals the ID3v2 view; it does not remove APE from the file.

The file's technical tag information identifies present/read tag types. When diagnosing another player's different result, inspect that information rather than assuming the visible value is the only stored value.

## Write

| Setting | Default | Effect |
|---|---|---|
| **ID3v1** | On | Write the limited legacy tag. Long text, extra fields, Unicode outside its character repertoire, multiple values, and artwork cannot all be represented. |
| **ID3v2** | On | Write the richer ID3v2 tag using the selected revision/encoding. |
| **ID3v2 only if ID3v1 too small** | Off | When both ID3v1 and ID3v2 writing are enabled, create ID3v2 only if needed. An existing ID3v2 tag is still updated. |
| **APEv2** | Off | Write APEv2, upgrading an existing APEv1 tag. ID3v1 writing has its own switch; APEv2 writing alone leaves an existing ID3v1 tag unchanged. |

At least one write type must remain enabled. Saving synchronizes written types from the displayed field set, so enabling a second type is not a promise to preserve a conflicting hidden tag independently. Types whose Write switches are off remain in the file. This also preserves an unread APE tag's native gain and recovery data without copying its hidden fields into ID3.

If APE reading is enabled while only ID3 writing is enabled, a successful save can update ID3 while the displayed title still comes from the unchanged APE tag. Turn APE reading off to inspect ID3, or deliberately choose which types to write. Changing Read selects a view; it does not synchronize or delete the other tags.

This also applies to embedded artwork. Manage covers verifies the types selected
for writing; a retained APE cover can still be the displayed image while APE
reading is enabled. Turn APE reading off to inspect the written ID3 cover.

### When ID3v1 is too small

TagGroove checks more than text length. Artwork, custom fields, multiple values, a user-defined genre, and characters outside Latin-1 require ID3v2. In the inspected implementation, title/artist/album fit only within 30 characters; comment space is 28 characters when a track is present, otherwise 30; year must be four digits; a track must fit its legacy positive byte value. A `TRACK` value containing a total, such as `3/8`, needs ID3v2 to preserve that text.

| Metadata in a file without ID3v2 | With conditional ID3v2 writing |
|---|---|
| Short title, artist, album, four-digit year, standard genre | ID3v1 may suffice. |
| Same fields plus embedded front cover | ID3v2 is needed. |
| Same fields plus `MOOD=Reflective` | ID3v2 is needed. |
| Existing ID3v2, even if values would fit ID3v1 | Existing ID3v2 is updated. |

### ID3v2 version and encoding

| Choice | Use and tradeoff |
|---|---|
| **ID3v2.4 UTF-8** | Unicode in ID3v2.4. Choose when all consuming players support this revision. |
| **ID3v2.3 UTF-16** | Default. Unicode in ID3v2.3; a practical compatibility choice for older consumers that support Unicode but not v2.4. |
| **ID3v2.3 ISO-8859-1** | Legacy non-Unicode output for a device that specifically requires it. Characters outside Latin-1 are written as `?`, so names in other scripts can lose information. |

Changing this setting affects future writes; it does not convert every loaded file immediately. Keep Unicode unless a known consuming device requires another format, then test its actual display on a copy.

## Remove

**ID3v1**, **ID3v2**, and **APE** removal all default to on. **File → Remove tag…** removes the selected types that exist. These switches do not request deletion during ordinary Save. For example, leaving **Write APEv2** off preserves an existing APE tag even while **Remove APE** is checked.

If every Remove switch is off, the remove command reports that no types are selected. Disabling Read alone never requests deletion.

When the complete metadata being saved is empty, Save removes the types selected under **Write**. This can remove artwork or fields in a written type that was not selected for reading. Unwritten types remain untouched. For example, clearing a plain ID3 title while writing only APE can remove the old APE tag, including its hidden cover; it leaves the unwritten ID3 tag in place. Its unchanged title can therefore appear again when the file is reloaded with ID3 reading enabled.

Clearing visible text does not establish that every record is empty. Artwork, binary records, chapters or other retained native data can remain. Use **Remove tag…** when the intention is to remove selected tag types completely, and check those Remove switches first.

Recognized Lyrics3v2 tails are preserved as opaque data rather than editable
lyrics fields. An APE tag before such a tail can still be detected and read.
Selecting **ID3v1** removal also removes the recognized dependent Lyrics3v2
tail, including one with no remaining ID3v1 block. Lyrics3v2 does not create
its own tag-type checkbox or a synthetic ID3v1 inventory entry.

An actual **APE-only removal** also drops the recognized Lyrics3v2 tail and
regenerates an existing ID3v1 tag from its retained fields. This differs from
an ordinary nonempty **APE-only Save**, which preserves the raw Lyrics3v2 and
ID3v1 bytes. Legacy Lyrics3v1 and unrecognized trailing bytes are retained;
these commands are not a general lyrics cleanup or Lyrics3 field editor.
Exact original-app padding and decoding behavior remains unqualified.

## Related

- [Tags overview](tags.md)
- [Ape, Mpc policy](tags-ape-mpc.md)
- [Advanced settings](tags-advanced.md)
- [Field mappings](tags-mapping.md)

<!-- verified: crates/tag-core/src/preferences.rs MpegTagOptions; crates/tag-core/src/tag_policy.rs read_order, id3v1_sufficient, write, removed_kinds; crates/tag-core/src/raw_streams.rs mpeg_policy; crates/tag-core/src/mapping.rs latin1_lossy; crates/tag-core/tests/tag_policy.rs. Recognized Lyrics3v2 tail handling is covered by core physical-file fixtures; original runtime padding/decoding remains unqualified. -->
