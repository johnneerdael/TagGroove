# Finalize locally executable parity and merge PR64

Owner objective (2026-10-19): finish every ledger task that can run on this Mac, prepare a concrete OS-specific engineer handover for the rest, mark PR64 ready, complete exact-head GitHub Codex review rounds and required trusted CI, and merge. The owner authorizes all routine work and design decisions without approval questions.

## Scope and invariants

- Authoritative intake: docs/PARITY.md and docs/research/remaining-gaps.md at starting head f2283f5a5ef77ab1935c5706fd8f13bbe32e5143: 58 active IDs, 14 qualification records, 54 represented scopes.
- Keep all work in existing codex/parity-followups and PR64. Preserve independent reference oracles and historical receipts. No closure from tests that mirror implementation or from source presence alone.
- Complete portable code on this host even when device/other-OS acceptance must be handed over. Do not classify unimplemented portable code as other-machine work.
- Native Windows captures and Windows-only integration require Windows. Linux desktop integration requires Linux. Physical provider tests require named Android/iOS devices; record host/toolchain needs separately. Mac work remains local, including installed-app/native journeys where feasible.
- Original Windows 3.36.1 and installed macOS Mp3tag are distinct reference artifacts; do not substitute one version/platform for another.
- Preserve production 2 GiB journal byte limit and 2 GiB free-space reserve. Serialize owned Cargo/rustc/media test execution, sanitize test environments, preserve failing executables/logs and scope any cache cleanup to audited disposable compiler products.
- No human approval gates. Final GitHub Codex code-review and current-main immutable validation requirements remain mandatory because the objective explicitly requires review rounds and merge.

## Execution

1. Establish a per-ID local/other-machine disposition and durable handover, including qualifications and exact evidence needed. Leave all unresolved local work visibly pending.
2. Correct confirmed native-derived L11 filter quoting, L15 Split Field publication and L17 action schemas with RED/GREEN tests before implementation. Preserve precedence, malformed controls, legacy action records and opaque data.
3. Prevent L61 concealed-content Undo/Redo overwrites with a content baseline covering every watched file/side-file and replay direction; design/implement durable L67 history retention consistently with the same integrity/admission model.
4. Finish local daily workflow, migration, sources, format/container, locale, distribution and documentation rows in dependency order; settle unknown semantics using pinned artifact evidence and reproducible local observations. Keep remaining other-OS proof separate from implemented behavior.
5. Perform all available local qualification journeys. Produce final handover with OS/device, setup, reference artifacts, exact procedure, expected assertions, current limitations and evidence return format for every remaining item.
6. Run complete relevant local checks, fresh whole-branch independent review, resolve findings, commit/push. Reconcile docs, checklist and handover against actual final source.
7. Mark PR64 ready. Request genuine GitHub Codex CODE review on exact final head, resolve/review again until clean. Run/reconcile trusted current-main immutable CI, fix failures, repeat review after code changes, verify live readiness, then merge without bypassing protection.
8. Verify merged source and handover link, final repository/PR state and remaining machine-specific work before claiming goal completion.

## Decisions

The current user goal supersedes earlier per-batch approval checkpoints. Ordinary implementation tradeoffs are made on the owner's behalf and documented with evidence. A task is handed over only for its actual OS/device dependency, never because local work is slow or complex.

## Task 1: Confirmed local semantic repairs

Spec: existing pinned native oracles and the owner goal; no new approval stage.

Interfaces: L11 changes only filter operand interpretation; L15 and L17 share actions.rs but separate handlers/codec. L61/L67 consume the core history API, so they follow this task without changing these oracles.

Steps:
1. Copy/hash-verify original frozen 12-filter/eight-Split-Field semantic vectors into permanent regression fixtures, without changing expected results.
2. Compile then execute `native_parity_remainders`; Expected: five filter differences and all eight Split Field differences fail semantically; malformed/precedence controls remain represented.
3. Implement quoted-prefix phrase versus bare-literal operand policy coherently; preserve parser limits, known malformed syntax and operator precedence.
4. Implement Split Field raw-piece-count publication, empty removal/deletion, duplicate/order preservation and unchanged-count no-op.
5. Add schema-aware native17/18/19 import/export with legacy1001–1003 compatibility, disabled resize and sentinel handling; use independently authored decoded records, not a renumber-only change.
6. Run focused action/filter/codec and affected guide tests. Expected: all vectors and existing unaffected controls pass; new oracle hashes stay identical.
7. Refresh guide/ledger evidence and commit this real completed slice. Keep native-runtime qualifications in handover.

## Task 2: Content-safe and durable history

Inspect current watcher/replay paths and concealed-edit evidence; write failing same-inode/size/mtime and crash/reopen tests before implementation. Keep exact Undo/Redo, aggregate limits, refusal behavior and complete portable history state. Expected: concealed external bytes are never overwritten and committed eligible sessions reopen with verified baselines.

## Task 3: Remaining local ledger and handover

Use the exhaustive per-ID register in docs/handover/PARITY-OTHER-MACHINES.md. Complete each local component with real validation; split OS/device-only acceptance with reproducible handover steps. Expected: no unresolved local rows, no unsupported claim of original/native equivalence.

## Task 4: Final review, CI and merge

Execute plan steps6–8 above. Expected: exact-head genuine GitHub Codex CODE review clean; required current-main immutable validation green; resolved threads; PR64 ready and merged; final handover reachable from merged main.
