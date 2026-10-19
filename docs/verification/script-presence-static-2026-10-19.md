# Script presence and predicate publication: pinned static evidence

The pinned Windows x64 Mp3tag 3.36.1 parser keeps visible output and a mutable
found counter separately. Predicates can replace the enclosing counter without
emitting text. This differs from TagGroove's current numeric boolean output and
OR-combined sibling flags. The port is **not fixed** by this investigation.
Native findings are static; the separate local probe below records current
TagGroove mismatches rather than passing conformance.

## Artifact and evidence scope

Artifact SHA-256:
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Retained Ghidra 12.1.4 analysis run:
`bbc2e7d1-45ef-4302-b792-163966a19d5b`.
Analysis profile SHA-256:
`378a660c321a42ed1615fe68445f2948bc93110be7e9858c2809914591120567`.

An independent read-only review checked complete retained parser, argument,
constructor, predicate, selector, Optional and result-publication instructions.
All 33 frozen-manifest entries and four reused-input bindings verified. All
27 literal text/counter predictions agreed with the inspected control flow.
No native target, original evaluator, application, GUI or provider was executed
for this review. Raw proprietary instructions and procedure views remain outside Git.

| Retained artifact | SHA-256 |
| --- | --- |
| `34-frozen-sha256.txt` | `1888a368645df3b9ff1a54f0174b16c35be42bbc60d14de00c2065d53f264ac5` |
| `30-evidence.md` | `e23a6109e6732981b38590f2cdc129bebe99560afae4387f78bf82724b10b2ff` |
| `29-static-oracles.json` | `2810957f91993dad50460f334f2345850cb9f0e14b5ab8e3a54333c5fe69f49c` |
| `33-receipt.json` | `97824f26ea4ba40a9e44aac67f09c3cc7caf2b0c84290be62a0907ab9605f5ee` |

## Sequential found state

The inspected parser stores an output CString at object offset `+0` and a
signed 32-bit found counter at `+0x18`. A nonempty resolved placeholder
increments found before appending its value. Literal or quoted text does not
increment it, regardless of whether that text looks like a number.

Every function argument is evaluated left to right in a fresh zero-counter
child. The argument helper records each child's counter and adds it to the
enclosing parser **before** function lookup and dispatch. Unused selector
branches are evaluated too. `if` and `if2` select using the first argument's
counter, append the chosen already-evaluated string, and leave the enclosing
counter unchanged. The inspected `ifgreater`/`iflonger` callbacks likewise
append their selected string without replacing found.

The inspected predicate family instead replaces the current parser's counter
and appends no text. AND/OR/NOT, equality and comparison predicates therefore
cannot be modeled as isolated numeric strings whose flags are ORed with earlier
siblings. A later false predicate can clear presence established by earlier
text-producing fields. Nonzero predicate results need not all have numeric
value one; the exact CRT mask returned by `isdigit` remains qualified.

With TITLE resolving to `Rock` and MISSING absent, four literal predictions are:

| Input | Native text | Found at return |
| --- | --- | --- |
| `$eql(1,1)` | Empty | 1 |
| `%TITLE%$not(%TITLE%)` | `Rock` | 0 |
| `$if($if(1,%TITLE%,no),T,F)` | `T` | 1 |
| `[%TITLE%$zz_unknown_3361($not(%TITLE%))]` | Empty | 0 |

The nested selector illustrates unused-branch presence: the inner literal
condition has no found value and selects `no`, but its unused TITLE argument
has already contributed presence. The outer selector therefore chooses `T`.
These are static native predictions, not executed original results.

An Optional section evaluates its body in a fresh child. It publishes the
child's text and adds its found counter only when that counter is nonzero.
A suppressed child changes neither earlier outer text nor earlier outer found.
The second parser call visible in that path sees the child's cursor at NUL;
it does not replay the body or reset the child's counter.

## Text publication and unknown rescan

The actual Format action parses the expression and copies the parser's
CString into the normal action-apply path. The complete copy helper reads the
first string pointer and string length/reference metadata, not the found
counter. Neither the dispatcher nor that publication path converts a predicate
result into visible numeric text. This closes the inspected parser/Format
action path, not every application caller or GUI wrapper.

The [earlier unknown-function finding](unknown-functions-static-2026-10-19.md)
establishes eager argument evaluation before a table miss. The fresh counter
review strengthens its rescan boundary: fallback appends the dollar sign and
resumes at the original name in the **same enclosing parser**. A recognized
predicate encountered during that rescan can reset earlier enclosing presence.
Replaying arguments as separate values and OR-folding their flags is therefore
incorrect. Evaluated field or host output must not itself be parsed as code.

The native registry also contains eight names missing from the port's default
implementation: `fmtdatelocal`, `get`, `listtec`, `put`, `puts`, `repl`, `rg2sc`
and `substr`. They are not true unknown names. Explicitly registered host
functions remain a separate dispatch contract; the static pass does not prove
native get/put behavior or authorize swallowing unsupported-name, syntax or
host failures as literal text.

## Actual current-port diagnosis

A separate external Rust executable ran all 27 frozen literal cases through
TagGroove's actual `CompiledTemplate::compile` and
`evaluate_value_with_context`, using the current bound library and a fixed
clock/field context without an extension host. It did not implement a second
evaluator or execute native Mp3tag.

Four complete controls agree, 19 evaluated cases differ, and four true-unknown
cases fail compilation where the native prediction is literal output. Sixteen
of the 19 evaluated mismatches change visible text; three change only the public
presence/truth projection. There are no runtime evaluation errors in this case
set. These are meaningful preimplementation differences, not a passing
conformance result or arbitrary harness failure.

| Input, with TITLE = Rock | Native static text | Actual TagGroove text |
| --- | --- | --- |
| `[X$eql(1,1)]` | `X` | Empty |
| `[%TITLE%$not(%TITLE%)]` | Empty | `Rock0` |
| `$if($if(%TITLE%,yes,no),T,F)` | `T` | `F` |

Literal `1`, quoted `1`, `$if(1,T,F)` and `$if2(ready,fallback)` are the four
full controls. The public API returns boolean `present`/`truth`, not the native
integer counter; comparisons use the independent native counter's nonzero
projection and do not claim exact internal-counter equality. Registered host
side effects and independent host-produced flags are outside this probe.

The immutable local receipt has SHA-256
`951e54ccd29cc3abb23b818ec81c49b25d8e4e810d9fa58ab964646e2a61806d`;
raw result-log SHA-256 is
`0288fc640c2c50c3e390cf47409d0653ad7d4653c599315a660977a0ffab048e`.
It binds the executed probe, exact native-oracle input, current library and 49
unchanged source inputs. No Cargo/library rebuild or repository source edit was
needed; the owned probe ran with credentials omitted and OS network denial.

## Design and resource boundaries

Retained `31-design.md` supersedes the earlier UnknownFunction proposal where
that proposal used isolated argument replay and OR-folded metadata. It proposes
sequential Add/Replace effects and same-frame rescan, but remains design-only.
A separately retained `35-host-contract-addendum.md` resolves the review's
expectation conflict: it proposes migrating unknown-compilation and builtin
predicate-text assertions while adding direct host-produced flag controls.
Its SHA-256 is
`58d46d9d36d33ea75dae458b140ef0fa053b92152a2dce42afcfe9a82279ebcd`.
The original frozen design and native manifest remain unchanged. The combined
bounded design still requires its implementation checkpoint; existing tests
expecting unknown compilation to fail or numeric builtin predicate text cannot
remain unchanged under the new native semantics. Public host-produced `ScriptValue` text/present/truth flags,
registered dispatch, side-effect ordering and error provenance still require
meaningful independent controls. No implicit legacy mode or host adapter has
been approved by this note.

The complete named parser, child constructor, argument helper and delimiter
scanner contain no inspected per-evaluation million-visit guard or explicit
nesting ceiling. Their syntax balance checks, native 64-slot argument collection
and CString length guards are different bounds. This is a bounded negative
finding, not proof that all original callers have unlimited resources.
Nested true-unknown calls can repeat argument evaluation exponentially while
their source and final text remain small. A proposed 1,048,576 AST-visit budget
would be an intentional future port resource policy, not a native threshold;
that choice is not approved here. Existing port input/output/depth safeguards
are not newly certified as native behavior.

Complete numeric comparisons, Unicode/locale and UTF-16 length behavior,
malformed UTF-16, signed found overflow, all native callback effects and full
caller wrappers remain unqualified. Current port numeric equality normalization
must not be inferred native. Export get/put/loop state, columns, actions,
filters, filenames and source-host adapters need coordinated implementation
and regression review if this design is authorized. No product fix, passing conformance result, original runtime acceptance,
final-head review or merge is claimed.
