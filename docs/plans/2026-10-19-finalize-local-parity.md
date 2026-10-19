# Finalize locally executable parity and merge PR64

Owner objective (2026-10-19): finish every ledger task that can run on this Mac, prepare a concrete OS-specific engineer handover for the rest, mark PR64 ready, complete exact-head GitHub Codex review rounds and required trusted CI, and merge. The owner authorizes all routine work and design decisions without approval questions.


## Owner scope change: finish PR64, then prioritize the next PR

The owner explicitly changed the integration strategy on 2026-10-19: finish the
current L67 history task, freeze PR64 to the work actually implemented, complete
its exact-head GitHub Codex review and trusted CI, and merge it. Do not implement
another ledger feature in PR64. Continue the overall parity objective in a new
PR after this one merges. This supersedes this plan's earlier all-in-one-PR
constraint; unfinished ledger work remains visible rather than becoming a merge
checkbox for PR64.

Before selecting the next batch, publish an exhaustive priority table explaining
what migrating Mp3tag users encounter, missing capabilities versus limitations
or unresolved compatibility versus qualification-only work, user reach/impact,
rough engineering effort/confidence, dependencies and required OS/device.
Prioritize daily correctness, migration and install/use blockers over optional
expansion, catalog-count matching and cosmetic identity. Research unknown
semantics before claiming defects. Include a concrete ordered recommendation
for small coherent follow-up PRs, and link it from the remaining-work handover.

## Scope and invariants

- Authoritative intake: docs/PARITY.md and docs/research/remaining-gaps.md at starting head f2283f5a5ef77ab1935c5706fd8f13bbe32e5143: 58 active IDs, 14 qualification records, 54 represented scopes.
- Keep the current task in existing codex/parity-followups and PR64; after its merge, put further ledger implementation in a new PR. Preserve independent reference oracles and historical receipts. No closure from tests that mirror implementation or from source presence alone.
- Complete portable code on this host even when device/other-OS acceptance must be handed over. Do not classify unimplemented portable code as other-machine work.
- Native Windows captures and Windows-only integration require Windows. Linux desktop integration requires Linux. Physical provider tests require named Android/iOS devices; record host/toolchain needs separately. Mac work remains local, including installed-app/native journeys where feasible.
- Original Windows 3.36.1 and installed macOS Mp3tag are distinct reference artifacts; do not substitute one version/platform for another.
- Preserve production 2 GiB journal byte limit and 2 GiB free-space reserve. Serialize owned Cargo/rustc/media test execution, sanitize test environments, preserve failing executables/logs and scope any cache cleanup to audited disposable compiler products.
- No human approval gates. Final GitHub Codex code-review and current-main immutable validation requirements remain mandatory because the objective explicitly requires review rounds and merge.

## Execution

1. Establish a per-ID local/other-machine disposition and durable handover, including qualifications and exact evidence needed. Leave all unresolved local work visibly pending.
2. Correct confirmed native-derived L11 filter quoting, L15 Split Field publication and L17 action schemas with RED/GREEN tests before implementation. Preserve precedence, malformed controls, legacy action records and opaque data.
3. Prevent L61 concealed-content Undo/Redo overwrites with a content baseline covering every watched file/side-file and replay direction; design/implement durable L67 history retention consistently with the same integrity/admission model.
4. Freeze PR64 after the current L67 task. Evaluate remaining daily workflow, migration, sources, formats, locale and distribution rows in a user-impact/effort table; continue their implementation in new PRs after64 merges. Settle unknown semantics using pinned artifact evidence and reproducible observations. Keep remaining other-OS proof separate from implemented behavior.
5. Finish local qualification required for the current task; keep other local acceptance explicitly pending for the prioritized follow-up. Produce final handover with OS/device, setup, reference artifacts, exact procedure, expected assertions, current limitations and evidence return format for every remaining item.
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

Checkpoint 2026-10-19: L61 is represented by C61 and durable L67 by C62.
Version-2 store/intents, native directory identities, shared retention accounting,
verified-copy recovery export and normal startup history loading are implemented.
The source-bound Rust workspace passes 1,510 tests across 84 executables (21
intentional ignores), including 42 durable controls; strict Clippy passes.
Seven bounded native Mac history/recovery phases pass, followed by a separate
RED/GREEN ordinary-startup probe on the corrected app. Fresh whole-branch local
review found that startup defect; it is repaired and targeted rereview is clean.
All 849 UI tests and pinned strict guide checks pass; L67 implementation is pinned
by C62. Exact-head GitHub Codex review, trusted CI and merge remain integration work. See the durable-history receipt for all
artifact identities, cache-loss caveat and native-input limits. No further
feature belongs in PR64. The exhaustive 49-row priority table and OS register
keep later local and external work explicitly pending.

## Task 3: Prioritized remaining work and handover before PR64 merge

Use the exhaustive per-ID environment register in docs/handover/PARITY-OTHER-MACHINES.md and the importance/effort assessment in docs/handover/PARITY-PRIORITIES.md. Before64 merges, make the current implemented scope and remaining local/other-machine work concrete. After64 merges, continue local components in a new PR in the recommended order. Expected: no hidden or falsely closed local work and no unsupported original/native equivalence claim. The full parity objective continues across those later PRs.

## Task 4: Final review, CI and merge

Execute plan steps6–8 above. Expected: exact-head genuine GitHub Codex CODE review clean; required current-main immutable validation green; resolved threads; PR64 ready and merged; final handover reachable from merged main.
