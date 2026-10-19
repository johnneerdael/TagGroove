# Matroska index boundaries — 2026-10-19

PR64 adds two physical-file controls for existing behavior. It changes no
production Matroska code and reproduces no new save defect. The retained
2,408-byte MKA is a local copy-remux of the independently authored synthetic
FLAC carrier. Its [provenance](../../crates/tag-core/tests/fixtures/save-corpus/tone-matroska.provenance.json)
records input/output/tool hashes, the command and four unchanged encoded FLAC
frames; it is not an original-Mp3tag or reference-writer file.

The [container tests](../../crates/tag-core/tests/containers.rs) create two
independent known-size layouts:

| Layout | Observed result |
| --- | --- |
| SeekHead replaced by equal-extent padding; Tags retained after Cluster/Cues | Read, longer TITLE save and reload succeed. Unedited fields, physical PART_NUMBER/TOTAL_PARTS, Info title, raw Info/Tracks/Cluster/Cues, packet bytes and block/index locations remain unchanged. No SeekHead is introduced. Journal Undo/Redo restore complete original/saved bytes. |
| Exact-capacity 68-byte SeekHead with no adjacent padding or Tags entry | Required metadata-index growth returns the existing no-room diagnostic. The original complete file and logical/media/index observations remain unchanged. Actual committed history has no Undo or Redo entry. |

A [bounded independent EBML observer](../../crates/tag-core/tests/support/matroska_elements.rs)
walks element extents and checks Seek positions and CueClusterPosition/
CueRelativePosition against actual Cluster/SimpleBlock locations. It compares
individual encoded packets with the original FLAC carrier, rather than relying
on the application reader or whole-file hashes alone. In-memory whole-file
mutations of one packet byte and a Cue target are both detected without
altering the physical input or history.

Both new controls pass; the first related run passed ten Matroska tests.
Strict container-target Clippy, formatting and whitespace checks pass. These
local tests qualify existing no-index discovery and safe index-growth refusal.
They do not implement full-head relocation, Segment Info title editing, number
mapping, unknown-size recording support, linked segments or ordered editions.
They do not establish external-player discovery, playback, original output,
native GUI, Windows or device behavior. L35 remains active for those separate
contracts; final GitHub review, hosted validation and publication remain required.


## Shared Undo test admission diagnostic

The root combined run initially failed an existing saved-cover Undo test. A
focused diagnostic reproduced the actual notice that the import could not be
undone because free disk space was below the unchanged 2 GiB reserve. The
operation had completed; the test had then replayed older history without
checking the notice. A later rename test failed on the poisoned test mutex.
This was a test/environment precondition failure, not evidence of a new
Matroska or cover-save preservation defect.

The existing source-import test now asserts successful Undo admission before
replay and empty structured errors for both Undo steps and Redo. After an
audited cleanup of old generated codegen intermediates, the same focused test
passes with its existing exact audio/image assertions. Production reserve,
snapshot policy and session lifetime are unchanged. The user guide already
explains that an older Undo label does not prove that the newest operation
has a recovery copy.
