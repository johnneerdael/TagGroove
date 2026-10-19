# R06: preserve opaque action settings through export

## Consolidated PR64 follow-up: readable own exports

Two further independently authored accepted-input defects were reproduced:
an opaque key starting `[#` with a decoded final `]` exported a line mistaken
for a section header on re-import; repeated Unicode settings expanded into an
oversized native UTF-16 file that the same importer refused.

The draft now escapes that final bracket contextually, including a preceding
literal-backslash run, while preserving the actual opaque key/value/order.
Opaque Unicode stays compact; recognized-action serialization retains its
existing spelling. UTF-8 export and native BOM-inclusive UTF-16 export each
enforce the existing 4 MiB reader budget against their own final wire encoding.
The host checks before atomic publication; refusal preserves an old destination
and does not create an absent one. UTF-16 can admit a BMP-heavy group that does
not fit the independent UTF-8 budget.

The current focused batch passes **nine interop, eight catalog and three native
host tests**. Controls cover section-like keys, literal/control/backslash and
Unicode boundaries, supplementary UTF-16 surrogates, exact/over-limit output,
encoding-specific admission and destination preservation. The new defects first
failed before the fixes. Independent source review found no remaining issue in
this bounded diff. These tests do not establish original-app readback.

The earlier PR58 receipt below is retained as historical evidence for its named
head and decoded-control-character scope.

PR58 branch: `codex/parity-automation-mta-next`. The current change is scoped
against approved main `0bc9d82963cb007374079bf16d10eded3467a4e7`, including
the merged PR57 diagnostics. Earlier validation below predates that integration
and the backslash/control-character correction; it is historical evidence,
not validation of the final PR58 head.

## Demonstrated accepted-input defect

An independently authored file, not a vendor fixture, contains an unknown
record with `T=9999`, `F=TITLE`, `1=line\x0abreak`, `2=left\x0dright` and
`9=\u000a\x0d`. The existing importer accepts it, decodes LF/CR values and
retains the complete property map in an unresolved action with its group
disabled. The exporter then rejects the same record with
`Line breaks cannot be stored in an .mta setting`.

The red test `opaque_mta_values_with_escaped_line_breaks_can_be_exported_and_reimported`
failed at that export before the implementation changed. Its local transcript
is `/tmp/taggroove-mta-preservation-red.log`. This proves a TagGroove
accepted-input preservation defect; it does not establish how the original
app interprets any unknown action, setting or control character.

## Narrow change

Allow the existing MTA value escaper to encode CR/LF only for unresolved
records, using `\x0d`/`\x0a`, which the importer already decodes. Escape a
preceding literal backslash so that re-import can distinguish it from those
control escapes, including runs of backslashes. Preserve the existing
multiline refusal for recognized actions. Leave all action IDs,
option schemas, validators and executable behavior unchanged. An unresolved
action remains unresolved, and re-import leaves its group disabled.

The preservation promise covers decoded opaque property values and action
order. It does not cover original bytes, property order, section numbering,
source encoding or escape spelling. Supported actions retain existing
normalization: Replace flag `0` exports as the equivalent app spelling `0|0`.
The tests compare opaque maps exactly and compare supported actions by their
interpreted action values, without asserting byte-for-byte equivalence.

## Independent regression cases

`crates/tag-core/tests/mta_interop_preservation.rs` covers:

- Unknown action IDs with escaped LF, CR and combined control values.
- A known action ID with an unknown flag: retain the complete opaque map,
  including CRLF, Unicode, equals signs and an unrelated property.
- Actual control characters beside literal `\x0a`/`\u000a` text, trailing
  backslashes and mixed section ordering, so export cannot reinterpret literals.
- One through eight backslashes immediately before CR, LF, CRLF and repeated
  line breaks, followed by trailing backslashes, literal escape text or Unicode.
  Both unknown types and future options retain their maps and reject execution
  validation after re-import.
- Backslash runs before every U+0000–U+00FF scalar, Unicode encoding boundaries,
  supplementary characters, and incomplete or invalid-looking literal escapes.
- Export/re-import through UTF-8, BOM-marked UTF-16LE (the native host's output
  encoding) and BOM-marked UTF-16BE.
- Continued rejection of LF/CR/CRLF in recognized actions, with a control that
  literal escape text still exports, imports and stays executable.

The focused suite passes all five tests after the review correction described
below. No audio files or installed user configuration are used. Native host
serialization is represented by the same UTF-16 encoding boundary; no claim of
native UI execution is made.

## Review correction and current focused validation

Review comment `4305123260` identified that the first change omitted CR/LF
from the literal-backslash lookahead. Importing `\u005c\u000d` accepted a
backslash followed by CR; export then emitted `\\x0d`, which re-import read
as literal `\x0d` text. The control character was lost.

The new `opaque_backslash_runs_before_line_breaks_survive_import_export_reimport`
regression failed before the escaper correction with exactly that difference.
Transcript: `/tmp/taggroove-pr58-slash-controls-red.log` (exit 101). Adding CR/LF
to the existing escape lookahead made the full focused suite pass: five tests,
zero failures. Transcript: `/tmp/taggroove-pr58-slash-controls-green.log`.
The suite includes 192 decoded-value combinations across three encodings for
each of two opaque action kinds, plus the character-boundary regression.
The recognized-action test still refuses actual LF/CR/CRLF and retains literal
escape text in an executable action.

These focused checks apply to the working-tree correction after local merge
`f5602348fe4574cd4b528e550e17e4fdf1de30ff`. Full-suite results, exact-head
remote CODE review, required CI and merge authorization remain separate checks.

## Current integrated local validation

After integrating approved main and correcting the review finding, the local
macOS/Apple Silicon run with Rust/Clippy 1.99 completed:

- `npm test`: 625 frontend and 1,144 Rust tests passed; nine existing Rust tests
  remained ignored. The five preservation regressions are included in that run.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `npm run build`, `cargo fmt --all -- --check`,
  `node scripts/i18n-extract.mjs --check` and `git diff --check`: passed.

The same four private shipped inputs listed below were rechecked with the
current `tag-core` library: import, export, UTF-16LE/BOM encoding and re-import
retained interpreted actions and enabled state, with no import issues. Record
counts remained 19, 1, 1 and 6; all four input sizes and SHA-256 hashes matched
the recorded inventory before and after the probe. No vendor contents entered
the repository. This is a current TagGroove consistency check against shipped
input, not original Mp3tag execution or opaque-control interoperability proof.

## Historical local validation before integration and review correction

The following counts describe the earlier implementation before PR57 was
integrated and before the two new backslash regressions and correction. They
must not be read as results for the final PR58 head.

On macOS/Apple Silicon with Rust/Clippy 1.92, using the established offline
audio archive directory and Apple archive tools:

- `npm test`: 561 frontend and 1,085 Rust tests passed; six existing Rust tests
  remained ignored. Transcript: `/tmp/taggroove-mta-preservation-npm-test.log`.
- `cargo test -p tag-core --locked`: 644 tests passed, including the three new
  cases. Transcript: `/tmp/taggroove-mta-preservation-tag-core.log`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `npm run build`, strict MkDocs, `cargo fmt --all -- --check`,
  `node scripts/i18n-extract.mjs --check` and `git diff --check`: passed.

Cargo printed an existing global-cache auto-clean permission warning; its
commands completed successfully. The unrelated cache was not changed manually.
No remote publication, review request, CI dispatch or merge was performed.

## Historical shipped-input evidence and limits

The following inventory and importer probe were recorded before the current
review correction; they do not establish current-head or original-app runtime
behavior.

Reference executable: Mp3tag 3.36.1 Windows x64, SHA-256 recorded as
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The four supplied shipped files were inventoried directly in that earlier
run; no vendor contents were copied into the repository:

| Shipped action file | SHA-256 | Observed type IDs |
| --- | --- | --- |
| CD-R.mta | `ccf92feca6bf8ea7606ba905ba36ee38627f21a101aa3af10e911e2467d79597` | 2 |
| Case conversion.mta | `9dcf4db6d968ef54c1906e22d28f3a0cfb75352526ef741546199af614684aac` | 1 |
| Schreibweise aendern.mta | `9dcf4db6d968ef54c1906e22d28f3a0cfb75352526ef741546199af614684aac` | 1 |
| Standard.mta | `e6df2a5c8782fa3c433191ef9185529476402a45f4276f71c7979b1edad6b4d7` | 2 |

A temporary local read-only probe loaded each private file with TagGroove's
importer, exported it, encoded the result as UTF-16LE/BOM and imported it again.
It found no import issues and equal interpreted actions for all four files:
19, 1, 1 and 6 records respectively. Its transcript is
`/tmp/taggroove-mta-shipped-reference-probe.log`; its temporary harness was
removed. This is execution of TagGroove against shipped input, not execution
of the original application.

These are static examples of Case Conversion/Replace records and observed
Latin-character `\xNN` escapes. They contain no opaque/control-character
case for this fix, no special-action serialization evidence and no runtime
results. Existing app import/export unit tests for other numeric schemas are
app consistency checks; their names do not independently qualify the original
application's serialization.

R06 still needs original-generated option matrices, unknown-option samples,
original re-import/readback and the three special-action mappings. R05 still
needs original expression results for the retained scripting/context gaps.
This local preservation fix closes none of those reference-proof requirements.
