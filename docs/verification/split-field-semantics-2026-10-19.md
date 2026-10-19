# Split Field: empty values and unchanged-count behavior

Eight native-derived literal cases were compared with the current public `apply_action_spec` API. Native results are **static control-flow predictions** from the pinned Mp3tag 3.36.1 x64 executable, SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`. The original application, GUI and file writer were not executed.

## Evidence bindings

Native raw views, instruction slices and `18-evidence.md` are retained in `/tmp/taggroove-native-split-field-pe4v3e3w`. The frozen eight-vector file has SHA-256 `fd7bccb91747dc920d29b822c7e0e1ef3b31fd1766f367c97c6227cd47cbb64c`. Native receipt SHA-256: `de3c23025f7ef6e1292b036916845d9bf0da73e5756dfdacf2cb075ef1cc7f34`; the final 21-artifact manifest SHA-256 is `e241982675f0f7e8a1c739230a825da8f17b23e5f51725bd141520c1f6930719`.

The public-port probe is retained in `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-split-md5audio-audit-wt16xswo/split-public-probe-ax9uwdn_`. Receipt SHA-256: `ab7805b20c014cebeee770fccf602701a76e016c8814c3b40c077e5960cc8fd2`; raw log SHA-256: `96550ffe97b0e5ff9935e1bd7b978e95daf43b709b70b62773f571ecb0a0d6ad`. It reused core library SHA-256 `178906aa466a16b75678b44bf944fa87ebf84990421a3c68f415dc9541dbfcde`, with all 49 producer-source bindings unchanged. The compiler and single eight-case invocation both exited 0; exact current-port assertions passed. All eight comparisons differed from the native predictions.

This exercised the metadata action model only. There were no audio writes, ActionHost side-effect requests, provider calls, Cargo rebuilds, original execution or repository code changes. The 2 GiB reserve remained unchanged. Exit 0 establishes successful diagnosis, not a successful compatibility repair.

## Native publication rule

The existing factory selects Type 16; its vtable reaches `MTActionSplitFields::Run` at `037610`. Literal substring search through `04071c`/`4fab80` splits each original value and flattens the pieces in order. Empty input produces no raw piece; a trailing separator produces a trailing empty piece. No regex compiler is involved.

The action compares the **original value count with the raw split-piece count** before publishing. Equal counts skip every setter, preserving original values even if temporary copies were trimmed. When counts differ, it removes empty pieces, assigns the remaining ordered values or deletes the field if none remain. Constructor/vtable and metadata setter/deletion paths close this as emitted native metadata, rather than only a temporary vector. Duplicate nonempty values survive.

This count rule must be preserved coherently; unconditional trimming, or merely dropping empty pieces after every call, does not reproduce it. Native whitespace/locale classification, other field selectors, invalid options and physical tag serialization remain outside these eight cases.

## Observed public-port contrasts

Every case targets `TITLE`. Unrelated fields and context remained unchanged, and no host side effects were returned.

| Input values | Separator | Native static result | Actual TagGroove values |
| --- | --- | --- | --- |
| `["a;;b"]` | `;` | `["a", "b"]` | `["a", "", "b"]` |
| `[";a"]` | `;` | `["a"]` | `["", "a"]` |
| `["a;"]` | `;` | `["a"]` | `["a", ""]` |
| `[";;"]` | `;` | Field absent | `["", "", ""]` |
| `[""]` | `;` | Field absent | `[""]` |
| `["a;;b", "c;d", "a"]` | `;` | `["a", "b", "c", "d", "a"]` | `["a", "", "b", "c", "d", "a"]` |
| `["a.*b.*"]` | `.*` | `["a", "b"]` | `["a", "b", ""]` |
| `[" a "]` | `;` | `[" a "]` | `["a"]` |

L15 remains an implementation remainder for empty-piece/field-removal handling and the count-based publication rule. Literal separators, multivalue order and duplicate retention are already implemented; no new feature task is assigned for them. No repair is approved or implemented by this evidence note. Original runtime, native interaction, format-specific empty-field storage and Save/Undo acceptance remain unqualified.
