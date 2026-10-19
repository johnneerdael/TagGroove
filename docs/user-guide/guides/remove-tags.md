# Remove tags or selected tag types

A file can carry several metadata blocks. Removing one block is different
from clearing a displayed field, and neither operation deletes the audio
file. Use copied files while choosing which stored metadata to retain.

## Choose the right kind of removal

| Goal | Route | When it writes |
|---|---|---|
| Clear one field, such as `COMMENT` | Clear it in the tag panel, or remove its row in Extended tags | After **Save tags** |
| Remove configured tag families from selected files | **File → Remove tag…** | When confirmed |
| Remove explicitly chosen ID3v1, ID3v2 or APEv2 blocks | **Tools → Remove tag types…** | When **Remove** is chosen |
| Remove rows from TagGroove's list | **Edit → Remove from list** | No audio/tag deletion |

The **Remove Fields** action is another field-level route; actions write when
run. Use [Remove Fields](../actions/remove-fields.md) when a repeatable list
of field names is the task. A field absent from the tag panel can still exist
in the file; inspect [Extended tags](extended-tags.md).

## Remove a legacy ID3v1 block from an MP3

Suppose the practice file's technical information lists
`ID3v2.4 APEv2 ID3v1`, and the goal is to retain ID3v2 and APEv2:

1. Save or discard pending edits, then select the copied MP3.
2. Choose **Tools → Remove tag types…**.
3. Check **ID3v1** only. Leave **ID3v2** and **APEv2** unchecked.
4. Choose **Remove** and inspect the operation result.
5. Inspect the tag-type information again. It should now show
   `ID3v2.4 APEv2`; the remaining blocks keep their own values.

The dialog shows counts of selected files with each type. An absent type is
disabled. Recognized Lyrics3v2 is an opaque dependent tail, not its own
checkbox or an artificial ID3v1 count. Its choices are for this operation
and do not depend on the **Remove** switches under Options.

Starting afresh from a copied MP3 with all three blocks:

<!-- remove-kinds-examples:start -->
| Only checked type | Remaining tag-type information |
|---|---|
| `ID3v1` | `ID3v2.4 APEv2` |
| `ID3v2` | `APEv2 ID3v1` |
| `APEv2` | `ID3v2.4 ID3v1` |
<!-- remove-kinds-examples:end -->

Removing a block does not copy its unique fields into another block. A title
can appear to change after removal because the reader now displays a different
remaining block. Compare [Read/Write/Remove policies](../reference/options/tags.md)
and the technical tag information when diagnosing that change.

**Remove tag types…** supports MP3/MPEG audio, AAC, TTA and the APE-family
formats Monkey's Audio, Musepack, WavPack, TAK and OptimFROG. It is not a
universal way to strip every format's native metadata. Unsupported files can
report per-file errors in a mixed selection. This tool requires supported
local file access and is unavailable for provider documents.

## Use the configured Remove tag command

**File → Remove tag…** (++cmd+r++ / ++ctrl+r++ with the file list focused)
uses the current policy:

| Format family | Which types are removed |
|---|---|
| MP3/MPEG audio, raw AAC and TTA | Types checked under **Options → Tags → Mpeg → Remove** |
| APE, Musepack, WavPack, TAK and OptimFROG | Types checked under **Options → Tags → Ape, Mpc → Remove** |
| Other supported formats | The format's supported metadata-removal path, rather than those two checkbox groups |

Read the confirmation's type list. For the configured families, no Remove
types selected is an error; an absent selected type needs no change. Other
formats have their own container handling. **Remove tag** is not a guarantee
that every opaque, unrecognized or non-tag container structure is erased.
Use [supported formats](../reference/file-formats.md) for preservation limits.

The command writes immediately when confirmed. It can remove embedded
artwork held in the removed tag. A folder image shown afterward is a display
fallback, not proof that the embedded artwork survived.

## Keep information that matters

APEv2 can contain ReplayGain values and `MP3GAIN_UNDO`. Removing it loses
that information unless another retained copy contains it. Removing ID3v2
can lose artwork, custom fields and other data that ID3v1 cannot represent.
Export or retain copies of metadata needed later before stripping a block.

Changing **Read** only changes what is displayed; it does not delete a tag.
Under the MPEG policy, ordinary Save uses **Write** and preserves unwritten
types; **Remove** selects types for the removal command. The Ape, Mpc policy
has its own save/removal interactions. Review the applicable options before
saving after a cleanup.

Selecting ID3v1 removal removes a recognized dependent Lyrics3v2 tail too,
including an orphan tail without ID3v1. An actual APE-only removal also drops
recognized Lyrics3v2 and regenerates an existing ID3v1 tag from its retained
fields; an ordinary nonempty APE-only Save preserves their raw bytes instead.
Legacy Lyrics3v1 and unrecognized trailing data are retained. This is bounded
tail handling, not a Lyrics3 field editor or universal lyrics cleanup. See
[Mpeg options](../reference/options/tags-mpeg.md#remove) for the distinction.
Removing MP4 metadata can also affect per-chapter tag fields; it is separate
from a dedicated chapter-structure removal command. See [Advanced tag options](../reference/options/tags-advanced.md)
and the format reference when keeping chapter data matters.

## Check errors and undo

A batch can succeed for some files and fail for others. Inspect the reported
file errors before repeating it; repeating against already cleaned files is
not a recovery of removed fields. Read-only files and unsupported source
access can prevent the operation.

**Edit → Undo …** can restore a recorded removal in the current desktop
session; its label identifies the operation. **Redo** reapplies it when
available. Use that history promptly, and retain backups for changes that
must be recoverable after quitting. These steps describe source-backed
behavior, not a guarantee that an arbitrary damaged input can be repaired.

## Related

- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Tag options](../reference/options/tags.md)
- [Mpeg policies](../reference/options/tags-mpeg.md)
- [Ape, Mpc policies](../reference/options/tags-ape-mpc.md)
- [Multi-value fields](../concepts/multi-value-fields.md)

<!-- verified: src/remove-tag.ts:removeTagPrompt, removeTagShortcut; src/utils-ui.ts:removeTagTypesDialog; src/main.ts:remove-tags, remove-tag-types; crates/tag-core/src/tag_types.rs:remove_tag_kinds_with_options; crates/tag-core/src/tag_policy.rs:removed_kinds; crates/tag-core/src/tags.rs:remove_tags_result; src-tauri/src/utilities.rs. Core fixture checks, not native UI verification. -->
