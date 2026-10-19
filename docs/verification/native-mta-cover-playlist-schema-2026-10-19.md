# Native action-file schemas: cover and playlist interchange gap

> Implementation update: [C57 execution](native-mta-implementation-2026-10-19.md)
> implements the canonical schemas below. Preserve this original static diagnosis;
> its earlier unresolved-importer observation is historical, not current source status.


Adjust Cover, Set Cover Properties and Generate Playlists already exist in
TagGroove. Their current `.mta` exchange records are incompatible with the
pinned Mp3tag 3.36.1 records. This narrows L17 to a known discriminator/schema
repair; it does not assign three new action implementations or close N05's
original-application readback requirements.

## Closed static record path

The Windows x64 artifact SHA-256 is
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The existing Ghidra 12.1.4 run and profile were retained. The trace closes the
factory, numeric `T` record loader, editor map copy, `[#n]` array codec and
literal `.mta` LoadActions/SaveActions callers. These are decoded record
contracts, not merely class-name or action-count inferences.

| Action | Native `T` | Current TagGroove `T` | Native option layout |
| --- | ---: | ---: | --- |
| Adjust Cover | 17 | 1001 | `1` format; `2` packed resize; `3` numeric quality |
| Set Cover Properties | 18 | 1002 | `1` new type; `2` description; `F` original-type filter |
| Generate Playlists | 19 | 1003 | `F` filename expression; `1` partition expression |

The native factory returns no action object for 1001–1003. Its accepted editor
maps reach the actual record writer. The playlist per-file virtual entry is an
empty `RET imm16(0)` stub; those bytes do not establish a zero return value.
Separate deferred dispatch reads the same action's type, compares it to 19,
and reaches its Generate method. The empty stub therefore does not establish
missing playlist generation.

Native Adjust Cover's packed resize is
`<pixels>|<shape>|<size policy>|<enabled state>`, with shape tokens
`scaleProportionally`, `scaleToSquare` or `cropToSquare`, size `max` or `min`,
and `enabled` or `disabled`. Enabled size targets outside 10–10000 skip the
image loop; disabled size does not necessarily disable square/crop transforms.
The Run consumer accepts quality 20–100 and otherwise uses 80. These bounds
describe effective Run behavior, not an original GUI default or every accepted
user input. TagGroove 1001 instead uses `2` for quality and separate `3`–`5`
resize options, so changing only its type number is unsafe.

Native Properties uses canonical `-1` sentinels for keeping a type/description
and selecting all original types. Empty description clears it. Its constructor
supplies those keep/all defaults for missing keys. TagGroove 1002 uses optional
values, presence-sensitive descriptions and `3` for the filter. A literal
description `-1` is not equivalent to the native keep sentinel. Converting an
export needs an explicit representation policy, not a blind number change.

Numeric-prefix/invalid-type grammar, original GUI validation and output byte
encoding/BOM remain unqualified. The native record codec and caller trace
do not supply original-generated files or original import/edit/re-export runs.

## Actual current public-codec baseline

A standalone probe called the actual public `import_mta`, UTF-8/UTF-16 exporters
and action validation, without executing any action or touching audio. Its
literal vector contains 13 native-ID records, including canonical examples,
missing/default/sentinel cases and effective-Run edge cases, plus three minimal
app-ID factory contrasts. These were independently authored decoded records,
not claimed native-produced file bytes.

All 13 native-ID records remained unresolved and disabled. Three additional
complete legacy 1001–1003 controls were executable and retained their expected
options through both encodings, confirming that the actions themselves exist.
Among the minimal app-ID contrasts, 1002 alone is a valid keep/keep/all no-op;
1001 and 1003 lack required fields. They are not three additional native records.

The probe checked caller-supplied names, numeric section ordering, complete
decoded property maps, issues, disabled state and UTF-8/UTF-16 re-import. Opaque
property and order corruption controls were detected. This confirms existing
safe unresolved-record preservation rather than introducing that functionality.

Compilation and the baseline invocation exited zero. A second invocation
repeated those observations and exited one at the explicit diagnostic comparison
`expected_canonical=3 actual_canonical=0`. This is a conformance predicate,
not a production parser crash or an independent second set of feature cases.
All 80 generated output files, source, executable and logs were retained.

## Evidence and implementation boundary

| Artifact | SHA-256 |
| --- | --- |
| Native schema evidence | `c903171259be533d0feb29254cb88db5a2df6ebb1295103671e529ddbb23c611` |
| Native receipt | `6856f383a83cf00a52b3db4c277068d78532ae7ba3d4e0a49ee3422dcacc2d43` |
| Native 44-entry manifest | `f1f6e00ef59b449c1025b28e4440b0e1aecb7785c0dad26cbb240d6d3f7d0325` |
| Independent native review receipt | `9492cb471fa3c5b5a58a92ac4af5336947f6df526f25088a66cff426ad3cfff3` |
| Actual public-codec execution receipt | `34e0d474ce42c8d30b455574b6926425e7f9d7cb1c9a7433e4656f1cdf5e90ab` |

The independent reviewer verified the 44 manifest entries and all 16 static
predictions. Current-port execution used the producer-verified retained library,
unchanged 49 compiled-source bindings, an environment allowlist and OS network
denial. Historical source/guide audit bindings retain their time-of-audit scope.
No Cargo rebuild, repository edit, original/native application execution,
provider request, image transformation or playlist output was performed.

A repair must handle native schemas and existing app records coherently,
including disabled resize and sentinel distinctions. Existing opaque retention,
group disabling, order, warnings and bounded atomic export are represented
features. No serializer/parser correction or policy approval follows from this
diagnosis. See [L17](../research/remaining-gaps.md#l17),
[N05](../research/remaining-gaps.md#n05) and the
[action exchange guide](../user-guide/actions/index.md#import-and-export-an-action-group).
