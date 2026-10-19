# Unknown scripting functions: pinned static evidence and baseline

The compiler mismatch below describes the preserved pre-repair baseline. Eager unknown-function fallback is now implemented; see [the repair and passing checks](four-parity-repairs-2026-10-19.md#scripting-and-export). Reserved native-known functions, malformed syntax and host failures remain errors.

## Preserved baseline

The Windows x64 Mp3tag 3.36.1 artifact with SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`
contains a parser path that preserves an unknown function name as format text.
The current TagGroove compiler instead rejects an unregistered unknown name.
This is a concrete L10 implementation gap, not a completed correction.

The conclusions come from retained Ghidra 12.1.4 output, raw instructions,
the 72-name native registry and the actual Format action caller. Neither the
original evaluator nor the original application was executed. The following
ASCII examples were independently derived from that control flow:

| Input | Native static prediction |
| --- | --- |
| `pre$zz_unknown_3361(abc)post` | Same text |
| `$zz_unknown_3361()` | Same text |
| `$zz_unknown_3361(a,b)` | Same text |
| `$zz_unknown_3361('a,b')` | `$zz_unknown_3361(a,b)` |
| `$zz_unknown_3361` | Same text |
| `$zz_unknown_3361(%TITLE%)` with TITLE resolving to Rock | `$zz_unknown_3361(Rock)` |

Arguments are evaluated before the function-table lookup. On a table miss,
the parser appends the dollar sign and resumes at the original name, so the
body is scanned again as format text. Eager argument output is discarded,
but field-presence contributions propagate to the parent. Nested fields and
callbacks can therefore run twice, and their first-pass failures can prevent
fallback entirely.

A future correction must preserve that ordering, case/spelling, quoted-text
and placeholder behavior, registered host extensions, presence accounting,
and existing error provenance and resource limits. Returning a raw literal
or swallowing every unsupported/syntax/host error would be incorrect.
Native-known functions that the port has not implemented are a separate gap;
absence from the port's supported list is not proof that a name is unknown
to the original.

The complete static pass has frozen-manifest SHA-256
`3c77488623a124aaa4379e951de46021a680a12d9a1b90f5bc22be30130c0977`.
An independent reviewer checked all 16 retained artifacts, the six predictions,
argument-before-lookup ordering, cursor resumption and output publication.
Raw proprietary analysis remains outside Git. No new implementation, original
runtime/UI validation, Unicode/locale equivalence, division-by-zero behavior,
invalid-character behavior or general error recovery is established here.
