# Tags → Ape, Mpc

Open **File → Options… → Tags → Ape, Mpc** for formats whose normal metadata is APE: Monkey's Audio, Musepack, WavPack, TAK, and OptimFROG.

!!! tip "New to this?"
    Keep APEv2 writing enabled for these formats unless a specific player requires another choice. ID3v1 cannot hold the same field set, artwork, or full Unicode text.

## Read

Both switches default to on:

| Switch | Effect |
|---|---|
| **ID3v1** | Allow an existing legacy ID3v1 tag to be displayed. |
| **APE** | Allow an existing APEv1/APEv2 tag to be displayed. |

APE takes priority over ID3v1 when both are enabled and present. The view uses one tag type rather than merging the fields of both. Disable APE reading temporarily to inspect an otherwise hidden ID3v1 value, then restore the intended reading policy. Reading switches do not delete either tag.

## Write

**ID3v1** defaults to off. Enable only when the consuming software requires it, accepting its limited text lengths, genre list, fields, and character repertoire.

**APEv2 (also updates an existing ID3v1 tag)** defaults to on. It writes APEv2 and upgrades APEv1. An existing ID3v1 tag is updated as well, even if the explicit ID3v1 write switch is off. Leaving the explicit switch off therefore avoids requesting a new ID3v1 tag; it does not freeze one already present.

At least one write type must remain enabled. Written types receive values based on the displayed tag. Back up files that intentionally contain differing tag blocks before a save that synchronizes them.

## Remove

| Switch | Default | Effect |
|---|---|---|
| **ID3v1** | On | Remove an existing ID3v1 tag when removal is requested; ordinary saving removes it when it is not being written. |
| **ID3v2** | Off | Remove an existing ID3v2 block. That block is not displayed by this family's reading policy, so it can otherwise remain hidden. |
| **APE** | On | Remove existing APEv1/APEv2 when requested; ordinary saving strips it when APEv2 writing is off. |

**Remove tag…** uses these switches. All Remove switches off produces a no-types-selected error. A checked Remove switch does not override writing the same supported type during Save: a written type is updated rather than removed. In particular, APEv2 writing's update of an existing ID3v1 tag counts as writing that tag.

The Mpeg page has a conditional-ID3v2 control and an ID3v2 encoding choice; this page does not. Choosing an ID3v2 version there does not turn this family into an ID3v2-writing policy.

## Related

- [Tags overview](tags.md)
- [Mpeg policy](tags-mpeg.md)
- [Mappings](tags-mapping.md)

<!-- verified: crates/tag-core/src/preferences.rs ApeTagOptions; crates/tag-core/src/tag_policy.rs Policy::Ape, read_order, write, removed_kinds; crates/tag-core/src/raw_streams.rs Stream.stand_in for TAK/OptimFROG/WavPack; src/options-ui.ts tags-ape controls. -->
