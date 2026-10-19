# Lyrics3v2 removal and hidden APE inventory: preserved diagnosis

The findings below describe the preserved pre-repair baseline. Recognized opaque Lyrics3v2 dependency removal and hidden APE inventory are now repaired; see [the scoped repair and checks](four-parity-repairs-2026-10-19.md#lyrics3v2-and-dependent-tag-removal). Legacy Lyrics3v1 and broader original/native qualifications remain separate.

## Preserved baseline

TagGroove currently removes ID3v1 while leaving its dependent Lyrics3v2 block
behind. A second independently constructed layout shows APE tags hidden from
the initial inventory when Lyrics3 precedes ID3v1. These are unfixed findings;
this note supplies diagnosis and reference evidence, not a product repair.

## Actual TagGroove execution

A disposable 0.25-second mono recording was synthesized with the existing
ffmpeg MP3 encoder, without ID3 or Xing output. An independent frame walk
accounts for all 2,298 encoded bytes in 11 MPEG-1 Layer III frames. Separately
authored Lyrics3v1/v2, ID3v1 and APE blocks extend that owned carrier.

Both public routes were exercised: File Remove through
`remove_tags_with_options`, with only ID3v1 removal selected, and Utils Remove
Tag Types through `remove_tag_kinds_with_options([Id3v1])`. For each lyrics
version, with and without adjacent APE, the operation succeeded and deleted
exactly the final 128 bytes. The complete lyrics block remained.

The initial eight observations were tested against a documentation-derived
joint-removal expectation. Subsequent pinned-native analysis reclassifies them:
**four Lyrics3v2 cases establish the native-backed dependency gap; the four
Lyrics3v1 cases match the traced native retention behavior**. Their original
raw expectations/logs are retained unchanged, not relabeled as new native runs.
Six controls passed: plain ID3v1 removal, and unselected ID3v1 preserving the
complete suffix without a new Undo entry. Four malformed/false-marker cases
preserved pre-ID3v1 bytes; those are local observations, not native acceptance.

Every selected operation required current default journal admission and exact
whole-file Undo/Redo. Encoded audio and the adjacent APE region remained
byte-identical. The first probe did not independently assert the returned
post-operation tag-kind list; physical boundaries and complete bytes establish
its reported removal result.

### Hidden headerless APE

A separate nine-case follow-up compared no Lyrics3, Lyrics3v1 and Lyrics3v2,
each through File Remove and both Utils input orderings `[Id3v1,Ape]` and
`[Ape,Id3v1]`.

Without Lyrics3, all three routes detected ID3v1 and APE and removed both to
the exact original encoded carrier. With either lyrics version, the initial
inventory reported only ID3v1; an explicit APE-only read produced no displayed
APE data. All three routes then removed only ID3v1, leaving the physical APE
and lyrics blocks. Their returned/post-operation tag inventory was empty.
The independent raw observer still found the complete APE bytes.

All nine operations preserved the encoded carrier and passed admitted exact
Undo/Redo. For v2, the missing inventory also prevents the intended APE removal;
changing only a late staging guard would not address that initial selection.
The v1 behavior is a separate legacy boundary. APE-only selection with ID3v1
retained was not executed in this follow-up and must not be inferred from it.

### Public orphan and APE-only follow-up

A new 13-case core-API probe ran after the static public-route review below.
Of twelve single-file cases, six matched their stated expectations and six
did not. One of those mismatches is a direct Utils-orphan API invocation,
explicitly separate from the original orphan-only dialog's eligibility.

File Remove on recognized orphan Lyrics3v2 returned success while preserving
the entire file, inode and modification time, with no new Undo entry. The
direct Utils call did the same. A mixed core-API operation applied the same
ID3v1 selection to an ordinary ID3v1 file and an orphan: only the ordinary file
changed, and the journal correctly recorded one changed file. The native
public-route prediction reaches the stripper for both supported records.

Two actual ID3v1-plus-Lyrics3v2 cases again removed only the 128-byte ID3v1
suffix. Two APE-only removal selections on APE + Lyrics3v2 + ID3v1 returned
success without changing any bytes; the initial inventory exposed only ID3v1.
Ordinary ID3v1 and APE-only-input removal controls succeeded through both APIs.
An unrelated removal mask remained a no-op and an empty Utils selection refused.

All cases retained the complete 2,298-byte encoded carrier and its independent
11-frame accounting. Every mutation required current journal admission and
exact whole-file Undo/Redo; no-op cases retained byte/inode/time identity and
no history entry. This is encoded-audio evidence, not independent decoded PCM,
Tauri batch/checkbox execution or original-runtime acceptance. The retained
library's actual Cargo producer, all 49 source bindings and its unchanged hash
were verified before execution. A missing launcher path stopped an earlier
attempt before compilation; that attempt is retained separately and is not a
semantic result.

## Pinned Mp3tag authority

The inspected Windows x64 Mp3tag 3.36.1 executable and retained copy have
SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Ghidra 12.1.4 static evidence closes actual stream truncation, not merely names
or documentation. No original process, GUI, Windows runtime or paired original
before/after capture was executed.

The traced MPEG removal path runs ID3v1 removal before APE removal. Its bit-1
helper accumulates the recognized ID3v1 and Lyrics3v2 suffix and truncates that
extent. The Lyrics3 detector checks `LYRICS200` and a calculated `LYRICSBEGIN`
position. The complete closed path contains no `LYRICSEND`/v1 scan. Consequently
ordinary legacy Lyrics3v1 remains after the 128-byte ID3v1 removal on that path.

For a well-framed audio + APE + Lyrics3v2 + ID3v1 layout, the static predictions
are: ID3v1-only removal retains APE; ID3v1+APE removal removes the entire tail;
APE-only removal discards APE and Lyrics3v2 and regenerates ID3v1 from fields.
That last path is **not** raw suffix preservation and differs from nonempty
APE writing. Exact regenerated ID3v1 serialization is not qualified here.

The native v2 detector does not validate each size byte as a decimal digit or
walk internal field records. A stricter checked port recognizer would therefore
be a separately stated safety policy, not exact native malformed-input parity.
A subsequent static caller trace closes the ordinary public route for a
recognized orphan v2 block without ID3v1. File Remove's enable predicate checks
the busy state, a live File List and a nonempty selection; it does not require
a cached tag kind. After accepted confirmation, valid supported selected MPEG
records reach the configured bit-1 helper without a per-file tag-kind gate.

Remove Tag Types has a different eligibility rule: its choices use the union
of cached tag kinds across the selection. An orphan-only selection cannot
normally choose ID3v1, but a mixed ID3v1/orphan selection enables that choice.
The accepted bit-1 request then reaches both supported records. Directly calling
TagGroove's Utils API on an orphan alone would therefore be backend evidence,
not proof of the original dialog's ordinary selection behavior.

An independent review verified the 33 public-route artifacts, four reused
inputs and all 17 earlier dependency artifacts, and checked ten literal
predicate/mask oracles. It found no contradictory gate for File Remove/orphan
or mixed-selection ID3v1 reachability. An appended operand binding resolves
the review's remaining APE-availability question: the handler searches for the
three-character UTF-16 string `APE`, ending at its first NUL, rather than the
adjacent `APEv` text. An independent review matched those bytes and their
membership-test-to-checkbox/mask routing against the retained handler. Thus an
actual cached APE kind can enable the choice whose accepted state supplies
mask 8. These are static control-flow facts, not executed original GUI/checkbox
input, malformed-file or Windows acceptance.

[Mp3tag's current MPEG manual](https://docs.mp3tag.de/customization/options/tags/mpeg/)
describes dependent Lyrics3 removal without a version restriction. The pinned
artifact's closed path is narrower. The [legacy specification](https://id3.org/Lyrics3)
and [v2 specification](https://id3.org/Lyrics3v2) explain the independently authored
fixture layouts; neither supplies observed original-editor behavior.

An independent read-only reviewer checked all 15 native layout/mask predictions
against complete detector/removal instructions, verified all 17 manifest entries
and five reused bindings, and confirmed the concrete stream truncate and
Write-versus-Remove distinction. No native proof defect was found in that scope.

## Source cause and repair boundary

Pinned Lofty 0.25.4's empty-ID3v1 writer truncates 128 bytes without lyrics
inspection. Its lyrics finder also compares an eight-byte slice of a 15-byte
buffer with the nine-byte `LYRICS200` marker. Direct runtime inventory contrasts
above establish the observable consequence; no dependency source was edited.

Actual physical removal call sites include both public APIs, known-empty MPEG
Save's written-kind removal, existing APE-family cleanup, and the family's
empty-ID3v1 projection. Empty `Fields::new()` updates remain no-ops and are not
substitutes for a genuinely empty supplied model. The MPEG CUSTOM-only blank
projection and ordinary/unwritten V1 preservation are distinct retained paths.
A coherent repair must preserve existing staging, error provenance, APE
collision/phantom guards and history, and handle inventory before selection.
No repair design or intentional deviation is approved by these findings.

## Retained evidence

| Artifact | SHA-256 |
| --- | --- |
| Initial external diagnosis final receipt | `4ca119327ad36d597c6e0d7f62341b86af6e5082f2af2f69fbb347039c2b3bfc` |
| Initial raw probe log | `254ad876f796484c2da91135c7e26be9587f33229b6599957d1848c9094c1f70` |
| Nine-case inventory follow-up receipt | `aad310ea00603c14f76576f7874d599a820490cdc35d041b3843a26183303ed5` |
| Follow-up raw probe log | `6bac75bc0bf81ebd33f0f8b2852e1efc98b379ebd50732508f925c4f49b9c27f` |
| Native 17-artifact manifest | `36d47ebbbe38d6b025fb88425843246095c29a9c450789e02aeb753396b5698d` |
| Native evidence note | `f0893d3353ed597af00ce236ae3263b5af1fc8ec6314926afc59ba8d43a72a66` |
| Native receipt | `a9a84a447ad232a2f3cf432ae05c093d66ae6cee7b84ea1df6b623d3340602b8` |
| Public-route eligibility evidence | `8fce7c574e32e315ede0bd180ea47355fe1b9b00d4e6a8ec66592d04ef70fd69` |
| Public-route eligibility receipt | `1be97935bae0b3af2edaa03b3e72db8752440a2ff7e9d6c3dc6d8b638631da94` |
| Independent public-route audit receipt | `7d22f27808a1a6f4d081b86a51d3e9b36e3ea7cbda2c53265f5d4e2db0969c63` |
| Appended APE-operand evidence | `b82367331c28bb8b0417773bf4be8bf99144f1962d864e41860b3a36ef69e4f8` |
| Appended APE-operand receipt | `fce2491583362b8afa41c1db9519dc04738243e00fa53c27afc945841d54d20f` |
| Independent APE-operand review receipt | `017387734e9aec5bc018b59c04b25b0cb71d7a652e0000f79573ae2bbb0e7890` |
| Thirteen-case public-core follow-up receipt | `9bacadebf5f9efad34631a64e8fc558aec6b5a6339cab93e6594fb0319b1b9aa` |
| Thirteen-case raw probe log | `126c52606dd0c85b908a8ddee1b072007b89f3d1914aecf044c8aa60e30e2b67` |
| Thirteen-case executed probe | `618b2f20a3d1192147eeb67e0d03aa411f67be6732dfc7cac264a84847d60ef7` |

The local probes ran with credentials omitted and OS network denial. Their 49
source bindings remained unchanged. The production 2 GiB journal reserve was
unchanged; the follow-up waited for adequate free space before any execution.
Raw logs, native instructions, probe binaries and disposable media stay outside
Git. No implementation, native runtime, device/player compatibility, final
CODE review, hosted CI, merge or release result is claimed.
