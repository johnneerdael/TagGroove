# ASF raw text preservation — 2026-10-19

An unrelated metadata edit could rewrite admitted noncanonical UTF-16LE text
in the same ASF metadata object. The reader exposed a lossy text view; rebuilding
the object then encoded that view, losing an unpaired surrogate, an odd trailing
byte or redundant terminating NULs. Attribute names and Content Description
slots shared this decode/encode path, including slots whose decoded text was
empty. This is a preservation defect for input the current reader admits, not
evidence that those byte sequences are valid Unicode or accepted by another editor.

The repair is confined to the [header codec](../../crates/tag-core/src/asf/header.rs),
[ASF field adapter](../../crates/tag-core/src/asf/mod.rs) and
[filesystem integration tests](../../crates/tag-core/tests/asf.rs).
Source-backed text retains its original bytes when decoding and re-encoding
would differ. Raw names belong to individual attributes, so occurrences with
identical displayed names or values do not borrow one another's bytes.
Nonzero Content slots that decode to empty remain hidden internal occurrences;
zero-length absent slots remain absent. They introduce neither visible empty
fields nor a false `_TAG` indication.

An explicit field edit, including an equal-display value or a user-mapped edit,
constructs newly authored Unicode and name data. Removal deletes the occurrence
and its provenance; a later unrelated save cannot restore it. Legacy TRACK
updates create a fresh value, while unchanged picture attributes retain their
existing name provenance. Stream/language fields, value types, existing location
selection, large-value ordering and format/size guards remain unchanged. No
public API, dependency, diagnostic or separate layout engine is added.

## Reproduction and corrected controls

The disposable carrier is locally synthesized audio: a 523 Hz sine, 0.25 seconds,
mono, encoded with the installed ffmpeg WMAv2 encoder at 64 kbit/s. Independently
authored descriptors and Content slots are inserted into its bounded header;
the complete Data/Index tail remains identical to the encoder output. No user
recording, proprietary script or provider response is included.

The initial test used TITLE as an intended other-object control. The encoder
also stored TITLE in Extended Content Description, so that test did not isolate
another object. It is retained as an investigation result, not counted as an
independent other-object control. Replacing it with Content-only COMMENT gave
two passing controls and one semantic failure:

- COMMENT preserved the untouched Extended descriptors and completed exact Undo/Redo.
- Explicit replacement of both noncanonical values produced ordinary Unicode
  serialization and completed exact Undo/Redo.
- An ALBUM sibling edit successfully wrote, received current-operation Undo
  admission and preserved the media tail, then failed independent raw descriptor
  equality. That failing test did not reach its later Undo/Redo assertions.

Before production edits, the expanded suite produced four passing controls and
five semantic preservation failures: the original same-Extended value case,
raw name/value cases in Extended, Metadata and Metadata Library, and a Content
sibling edit. These failures followed successful production writes. The expanded
helpers completed media and exact Undo/Redo checks before their preservation
assertions. The repair first passed all nine cases; subsequent test strengthening
added the tenth new control and isolated observer mutations.

## Independent oracles and passing scope

The observers walk bounded Header Object, child-object, descriptor and Content
slot extents. They retain raw name/value bytes, type, stream and language instead
of asking the production parser to validate its own output. Independent in-memory
mutations of names, values, stream/language and an empty slot's discarded byte
must change the observation; those controls do not touch disk or history.

| Control | Qualified result |
| --- | --- |
| Extended sibling and other-object edits | Untouched surrogate and odd-byte values remain byte-identical; COMMENT supplies the corrected other-object control. |
| Extended, Metadata and Metadata Library occurrences | Raw names and values survive a same-location edit, including distinct raw occurrences with equal decoded names/values and retained stream/language identity. |
| Equal-display, mapped and genuine replacement | Edited records use canonical newly authored serialization rather than recovering source bytes by display equality. |
| Removal followed by another save | Removed raw names/values and hidden Content slots do not reappear. |
| Content sibling edits | Nonempty surrogate text, redundant terminal NULs, decoded-empty odd-byte slots, canonical empty slots and zero-length slots retain their untouched bytes. |
| Empty Content visibility | Empty slots stay out of displayed fields and do not alone create `_TAG`; explicit removal remains effective after another save. |
| Media and history | Each new operation requires default journal admission, the current file receipt, complete Data/Index tail equality, and actual whole-file byte equality after Undo and Redo. |

The final ASF integration target passes **19 tests: 10 new and 9 existing**.
The ASF unit filter passes **11 tests**. Targeted strict Clippy, formatting and
scoped diff checks pass. The initial Clippy run found one constant-size chunk
iteration lint in the test observer; after the test-only correction, Clippy and
all 19 integration tests were rerun successfully. The unit run preceded only
that observer lint correction; production hashes were unchanged.

Every Cargo/test process ran under OS network denial with the established
locked/offline build environment. Each command bound 50 source/build inputs
before and after execution without drift. Journal free-space admission retained
the existing 2 GiB reserve. The later parent integration passed 1,369 Rust tests with zero failures and
18 existing ignores across 82 groups, 828 frontend tests and strict all-target
workspace Clippy. Production ASF and its 19-test target hashes above were
unchanged. Eight documentation tests and the strict guide build also passed.

## Evidence identities

The immutable external receipt `final-receipt.json` has SHA-256
`436c391ca745580fdcc3cadf69ebb31c6380ad2ed1f8db6f2418ef9877f62487`.
It binds the raw logs, source manifests, retained executable, patch and local
carrier preparation receipt. The external files are evidence identifiers, not
redistributed media or repository fixtures.

| Artifact | SHA-256 |
| --- | --- |
| Header codec | `e1a51a93b46e20fac7aafc323506de62580c91beff1aa06741e021ff04b8e145` |
| ASF adapter | `7ae080a4dff4c7e5ea011337163fb9cc3d204e5e78df83f5fe34a4f4a58235df` |
| Integration tests | `17ee33e32d7ed4d8a08ad68b35ee2d5ab7c5134980d5145deab12e5575642962` |
| Corrected original RED log | `212b09efbb186d4f7bdca0394f1525d6cbefb3ff7fde6ed7d586885a4f6a85fe` |
| Expanded RED log | `6b3b5af58cdec1b3f09ee157614023b66a18b3cbe374968c833e062a4d95bbde` |
| Final integration log | `ff95e39e875e2d90a4845463166989644799b8cd99c3ab82c110db6bff7efc39` |
| Unit log | `e16ffd557c2f7b87514b36814ae3f3dca8aa3b70695cd5fea55ad8b1e4d23a80` |
| Final targeted Clippy log | `51328bf2f6d3357d370f7f041f9495c016f6334d554511198b75af49d0e4465c` |
| Retained final integration executable | `e22286acef86a7026293e97e9ffdf5dca3cd0de1700a8ac83067c88bbd57ae50` |
| Installed ffmpeg executable | `2be8e0caabc6ce733a628d1523f43235702d714826df3408e04e09b2a079b7a3` |
| Retained encoder-output carrier | `c0e5706281587d5a9f294b947d4f3447b38fd131a4755f9dd8dddcc6ec011f3c` |
| Initial injected carrier | `383f32e99004b4f4f0e6285336c3a75b70e45bec6e729ba4cab279c2dd70ac24` |

This qualifies the named admitted text paths and disposable local carriers.
It does not establish original-editor equivalence, valid-Unicode compatibility,
Windows/native/player acceptance, decoded playback, every ASF object topology,
unparsed trailers, universal preservation or headers beyond the existing
512 MiB limit. Neither the focused checks nor the later integration checks
supply a release, final-head CODE review or merge result.
