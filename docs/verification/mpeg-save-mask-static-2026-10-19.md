# MPEG Save and Remove: pinned native static evidence

## Finding and scope

The pinned Windows x64 Mp3tag 3.36.1 executable separates ordinary Save from
explicit Remove tag. In the inspected successful MPEG paths, Save uses the
write mask; it does not use the Remove preferences to delete unwritten tag
kinds. The fresh MPEG read seed enables ID3v1 and ID3v2, with APE reading off.
Together these findings require a preservation correction, rather than an
isolated change to a checkbox default.

Artifact SHA-256:
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The analysis uses retained Ghidra 12.1.4 output and raw instructions from an
immutable local copy. No original application, Windows/Wine, native GUI or
provider was executed. This is static control/data-flow evidence, not paired
runtime acceptance. Raw proprietary assembly and decompiler output remain
outside the repository.

## Observed contracts

| Operation | Static conclusion | Important boundary |
| --- | --- | --- |
| MPEG read with mask `3` | ID3v2 replaces enabled ID3v1 fields; APE fields/covers are skipped. | APE physical presence/version/size can still be reported. There is no observed APE-only field fallback. |
| Ordinary nonempty Save with mask `3` | Writes ID3v1/ID3v2 and skips the APE writer. Prefix resizing copies the entire remaining file tail, including existing APE. | Successful ordinary valid-carrier paths; not all malformed inputs or error branches. |
| Explicit Remove with mask `11` | Selects the ID3v1, ID3v2 and APE removal helpers. | Remove preferences are a separate operation policy. |
| Nonempty APE-only Save with mask `8` | Skips the ID3v1 writer, then saves and re-appends the recognized existing ID3v1/Lyrics3 suffix bytes. | Does not regenerate ID3v1 values from the new APE fields or create a missing ID3v1 tag. |
| Nonempty Save with mask `9` | Updates ID3v1 first, then the APE writer preserves that updated suffix. | This does not establish APE-family writer behavior. |
| Empty supplied metadata | Calls concrete Remove using the **write** mask. | The nonempty APE-write raw-byte proof does not establish the empty-path suffix identity. |
| Nonempty metadata containing only fields unavailable in ID3v1 | With ID3v1 writing enabled, still writes a 128-byte TAG projection, appending or replacing V1. | An empty ID3v1 projection is not an empty full metadata model; exact native genre/default serialization is unqualified. |
| ID3v2 only if needed | May skip adding an absent ID3v2 tag when the native fit checks pass; existing ID3v2 is rewritten. | Complete Unicode/locale equivalence of the fit checks is unqualified. |

MPEG bits `1`, `2` and `8` identify ID3v1, ID3v2 and APE. The engine also packs
APE-family settings into bits `0x10`, `0x20` and `0x40`. Those are not extra
MPEG APE switches; the MPEG reader clears the family bits before selection.

An explicitly APE-enabled read can continue displaying the retained old APE
values after a Save that writes ID3 only. Written-kind verification must
inspect the written kind separately; a different displayed priority does not
justify deleting the preserved tag or refusing the save.

### Empty means three metadata collections

The native empty predicate checks text, non-picture binary records, and
pictures. Displayed APE binary items populate the binary collection, so
clearing its text and pictures alone does not make that model empty. Generic
ID3 unknown/PRIV descriptions use a separate technical-property collection;
they do not establish binary-count membership. Special ID3 binary mappings
and complete opaque-only Save behavior remain unqualified.

The port must preserve displayed opaque data when its Fields/Pictures view
cannot establish the complete supplied model. A plain displayed text model
without opaque data is a separate case: clearing it must not leave artwork
in an unread tag selected for writing. A blanket whole-tag deletion based on
empty text/pictures would conflate those cases.

## Consequence for the port

At checkpoint `e8e3e82ba409135dfe8413451eeac07d22b185e8`, MPEG Save still
couples Remove preferences to deletion of unwritten kinds, enables fresh APE
reading, and updates an existing ID3v1 tag when only APE writing is enabled.
Its hidden-APE gain migration compensates for deletion rather than preserving
the native APE data in place. These are concrete source/native differences.
The correction must address them together, preserve explicitly stored read
choices and keep other format-family policies separate.

Required local proof includes independently inspected untouched APE bytes,
conflicting ID3/APE values, nonempty APE-only ID3v1 suffix retention, explicit
Remove, empty-write behavior, audio preservation and actual journal Undo/Redo.
Existing consumer checks must distinguish physical APE preservation from the
values selected for display. No implemented correction is claimed by this
static note alone.

## Retained evidence and review

The read/default phase is recorded in
[MPEG default static seeds](mpeg-default-static-seeds-2026-10-19.md).
The Save/Remove phase retains its complete reader and empty-write addendum,
with frozen-manifest SHA-256
`e28198545c2a05dcf90028847bd5d76dd859e9218e251bd3cfd740c1744c69e0`.
The separate nonempty APE-write/ID3v1 phase has frozen-manifest SHA-256
`6e6819d73df6f5835ce981485ae7ccd1fb91edfa2465d904a0e7054b2aeed127`.
An independent reviewer verified both manifests and followed the retained raw
mask gates, complete-tail copy, and suffix buffer read/write identity without
running the original application or opening a second analysis project.

A follow-up retained-helper review distinguishes truly empty metadata from a
nonempty custom-only model. After the outer full-model empty check, the V1
helper constructs its classic-field projection and unconditionally writes
the same 128-byte buffer. The small addendum has frozen-manifest SHA-256
`a9a243b088b0039e1c6f07d3e2b2efa8e01599a55d2f05e81c1df8a6ca6a1ee9`.
The separate three-collection/technical-property pass has frozen-manifest
SHA-256
`9e12caf3e8e0b95f47a11c70ce0acb70e69c3b6c27a414e19dcd0ec49cbb1cbc`.

Native field-level mirroring, complete fresh-profile startup overrides,
existing encoding reuse, all other format families and original runtime
before/after bytes remain unqualified. Windows work is deferred by the owner.
