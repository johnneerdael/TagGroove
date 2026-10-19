# Export variable presence: pinned contract and baseline

The adapter mismatch below is preserved pre-repair evidence. The approved string-only variable adapter and sequential evaluator repair are implemented; see [the correction and checks](four-parity-repairs-2026-10-19.md#scripting-and-export). Remaining column/self-reference semantics stay qualified.

## Preserved baseline

TagGroove already implements `$get`, `$put` and `$puts` in the public export
renderer. This investigation confirms a presence/argument adapter mismatch under
L09/N04; it does not identify three missing export features. No correction has
been implemented or approved by this evidence work.

## Native contract and scope

The pinned Windows x64 Mp3tag 3.36.1 artifact has SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Retained Ghidra 12.1.4 run `bbc2e7d1-45ef-4302-b792-163966a19d5b` uses profile
`378a660c321a42ed1615fe68445f2948bc93110be7e9858c2809914591120567`.
Native findings are static control-flow evidence, not original runtime results.
Raw proprietary instructions remain outside Git.

The native dispatch table registers `get` at `0x14010dab0`, `put` at
`0x14010f078` and `puts` at `0x14010f1d0`. With active storage and the required
argument count, all three operate on strings. `put` stores and emits its value;
`puts` stores without emitting; `get` emits the stored string or empty text for
a missing key. They do not restore stored presence or change the enclosing found
counter. Every argument has already been evaluated and contributed its child
counter before callback dispatch, as described in the
[unified presence finding](script-presence-static-2026-10-19.md).

The inspected `CMTMainFrame::ExportThread` at `0x140125d30` creates a fresh
variable map for its output render. Renderer `0x1400baa14` shares it through
recursive rendering, and `MTExportParser::FormatEntry` at `0x1400bc724` supplies
it to the script parser. The inspected output exits destroy the map. This closes
that named output construction/reset path, not every export wrapper or action
group. Inspected sort-key parsers and the normal Format action at `0x14002debc`
pass null storage; their variable callbacks emit nothing while eager argument
presence still survives. No general default-function availability claim follows.

## Actual public export diagnosis

A standalone probe linked the retained production `tag_core` library and called
[`render_export`](../../crates/tag-core/src/export.rs) with TITLE=`Rock` and
KEY=`k`. It used the actual export Engine and no replacement evaluator. Nineteen
frozen native oracle expressions were retained; the 23-case port probe adds two
eager/unused-branch predictions and two direct formatter projection controls.

| Cohort | Current-port result | Interpretation |
| --- | --- | --- |
| 15 valid active-storage export expressions | 6 text mismatches; 9 matching controls | Confirmed rendered-text differences against the closed static contract |
| 2 direct formatter controls | Both match | Public literal/field text and flag projection controls |
| 2 export callback-arity controls | Both compile errors | Native callback guards alone do not close public validation behavior |
| 3 native null-storage Format contexts | All rejected by the port compiler | Separate known-name/context compile policy, not missing export support |
| 1 ASCII key-fold expression | Text differs | Conditional on the native default comparator branch; excluded from definitive mismatch count |

The public export API exposes rendered **text**, not Engine presence counters.
The probe therefore records export presence/truth as unavailable. Native found
values below are static predictions; no original counter was observed at runtime.

| Expression | Native predicted text | Native predicted final found | Actual port text |
| --- | --- | --- | --- |
| `$puts(k,%TITLE%)[X$get(k)]` | Empty | 1 | `XRock` |
| `[X$puts(k,%TITLE%)]` | `X` | 1 | Empty |

In the first expression, eager TITLE evaluation contributes to the outer parser.
The Optional section starts a fresh child counter: retrieving text with a literal
key contributes no presence, so that child is suppressed. In the second,
TITLE contributes inside the Optional child even though `puts` emits no text,
so its literal `X` remains visible.

The current [export adapter](../../crates/tag-core/src/export.rs) instead stores
and retrieves full `ScriptValue` objects, including flags. The current
[extension evaluator](../../crates/tag-core/src/script.rs) takes the host-returned
flags after argument evaluation. These two boundaries explain both incorrect
retention of stored presence and loss of eager argument presence. Plain store/get,
missing variables, overwrite and unused-branch storage controls prevent treating
these results as a blanket failure of export variables. The existing
[variable tests](../../crates/tag-core/tests/export.rs) explicitly encode typed
flag retention and will require deliberate migration if a correction is approved.

## Correction boundary

The pending design must combine the L09 sequential engine/argument-found model
with string-only behavior for the built-in export adapter. Changing only stored
text, globally discarding extension flags, or OR-folding every host result with
all arguments would not implement the native counter model. Arbitrary public
`ScriptExtension` implementations retain their independent host-supplied flags
and external error contract; direct host controls in
[script extension tests](../../crates/tag-core/tests/script_extensions.rs) remain
separate from native export behavior. This is an unapproved design boundary,
not a repaired conformance result or a new duplicate feature assignment.

## Retained receipts and qualifications

| Frozen artifact | SHA-256 |
| --- | --- |
| Native `24-evidence.md` | `429c62ce4f708a259e1f64122128930a432dfd2876f5b5fc19769d7ecdd00397` |
| Native `26-receipt.json` | `d9763a59680384f1d4d00bc757a74abdc789804f0b98a15f53a9c2ba76ca5ae8` |
| Native `27-manifest.json` | `7caf42ee55b89491630ef3c1e2afc4b1330eb73a371beaed0a52bf0ad5c1f4d6` |
| Independent native-variable audit receipt | `e7799d2dcd26e4b3839dad5c4264af2fe42ac7ddf83468815fc0ee79dddde1f4` |
| Port `final-receipt.json` | `d9212e3b58660db247d27ceb700a8091240cdc3ba836dc7119f0fb4e7aeea880` |
| Port `final-manifest.json` | `aef78b144ca4dce60493e4347e3599ac178d114c329c43ad9434b314e51e550c` |

Port probe compilation and execution both exited zero; this means the diagnostic
completed, not that the six differences passed conformance. Seven source/test/
guide bindings and the library digest were checked before and after execution.
The linked library SHA-256 is
`178906aa466a16b75678b44bf944fa87ebf84990421a3c68f415dc9541dbfcde`;
the retained producer-relation receipt is
`465f7d6bbd5e9ca1f8a978ad2bb72483c4d4b7af8a5dff2edc607fb09e365396`.
The probe binary SHA-256 is
`1b4d39356c13a0180807cc95ef17fbbba2bcebb8e407fbc95e8953ed0c1ff3fe`.

The port run used an environment allowlist and an OS deny-network sandbox. No
Cargo rebuild, repository change, original application/GUI/Windows execution or
provider request occurred. Locale-dependent key comparison, selected native
runtime state, untraced callers and native public validation remain qualified.
The conditional ASCII key result is not counted as a definitive defect.

An independent reviewer checked all 26 phase files and their manifest, seven
source/guide bindings and seven reused inputs, and all 19 static predictions.
The review found no substantive proof defect within the stated callback,
counter and named-lifetime scope. The retained public-Engine outputs above
were cross-checked without executing the original application.
