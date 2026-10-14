# Provider Action Groups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development task-by-task. Track steps with checkboxes.

**Goal:** Execute existing in-place tag/artwork action groups through Android document publication while retaining desktop behavior.

**Architecture:** Reuse core action parsing/evaluation with a small managed host. Validate group capabilities and document selection before edits, then use the existing per-file document mutation workflow and UI retained-entry reconciliation.

**Tech Stack:** Existing Rust/Tauri 2, TypeScript and locked dependencies.

**Spec:** `docs/superpowers/specs/2026-10-14-provider-actions.md`

**Execution:** Native sequential implementation in the existing isolated checkout, followed by one independent whole-branch review. Branch `feat/provider-actions` starts at `528dcf7`; PR7 remains unchanged on its own branch.

## Global Constraints

- Preserve the full feature/performance parity objective and existing Light/Dark behavior.
- Preserve desktop action behavior, action-library serialization and core tag schemas.
- Use existing selected-document access only; add no permissions or global configuration changes.
- Keep all mutation tests on generated audio; Beatport remains read-only.
- Use ordinary source tests, builds and normal UI checks; do not resume stopped diagnostic device sequences or binary analysis.
- Keep unexecuted device checks explicit; source tests and builds do not certify device behavior.

## Review Focus

- `_ALL` can mean filename plus tags for transforms, but metadata only for merge/split/duplicate actions; preserve that distinction.
- A later unavailable action must not leave earlier groups applied.
- Artwork operations can change fields/context before the next expression; keep projected source identity and current artwork measurements coherent without reentering store locks.
- Failed later groups must preserve earlier verified groups and truthful partial results, pending-field choices and metadata notices.
- Pure metadata no-ops must avoid publication and expressions must execute only once per file/group.

### Task 1: Shared guessing parser and managed capability validation

**Files:** modify `crates/tag-core/src/actions.rs`, create `src-tauri/src/managed_actions.rs`, declare the module under Android/test in `src-tauri/src/lib.rs`.

**Interfaces:** produce `guess_value_targets(pattern: &str) -> Result<Vec<String>, String>`, `IN_PLACE_ACTION_TYPES: &[&str]` and `validate_groups(groups: &[ActionGroup]) -> Result<(), String>`.

- [ ] Add failing parser tests for repeated ARTIST targets, DUMMY, Unicode/newline values and the existing malformed/adjacent-placeholder rules.
- [ ] Extract the existing compiled Guess Values pattern parser; have target inspection and execution consume the same parser without changing current output semantics.
- [ ] Add capability tests using all 13 valid action DTOs; reject the five external-file types, unresolved enabled imports, rename selectors and malformed Guess Values settings with group/action position. Ignore disabled unsupported groups.
- [ ] Pin Replace `_ALL` as unavailable while Merge Duplicate Fields `_ALL` remains supported; cover `_TAG`, `_FIELDNAME` and Format Value/Guess Values technical targets.
- [ ] Implement the focused capability module, run the covering tests and commit.

### Task 2: Managed host, projection and native command route

**Files:** modify `src-tauri/src/managed_actions.rs`, `src-tauri/src/documents.rs`, `src-tauri/src/documents/engine.rs` and `src-tauri/src/lib.rs`. Add focused Rust tests using the existing generated-audio document command harness.

**Interfaces:** consume Task1 validation; produce the spec's `DocumentProjection`, `documents::projection`, managed `execute` and runtime `provider_action_types` field. Keep `execute_action_groups` arguments and `ActionResult` serialization unchanged.

- [ ] Write failing ordinary action tests with generated WAV/FLAC where supported: uppercase TITLE then replace text; repeated values/deletions; `_FIELDNAME` format eligibility; embedded-artwork removal, adjustment and properties; expression using provider filename/URI, counters and columns after earlier field changes.
- [ ] Assert an unavailable action in a later group changes no selected file; a mixed invalid/read-only selection starts no edits; repeated managed identities are processed once.
- [ ] Expose the owned immutable projection from existing records and use its pure apply method inside managed callbacks. Preserve existing `documents::project` callers.
- [ ] Implement the managed host and group-boundary loop. Use prepared fields once for pure metadata groups, skip unchanged groups, and use existing tag/artwork writers inside `documents::mutate` for changes.
- [ ] Assert ordinary per-file errors preserve meaningful results and do not continue that file's later groups; existing document retention/reporting contracts stay intact. Do not add device diagnostic sequences.
- [ ] Route managed/Android action requests to the adapter; keep the existing desktop route. Add Android capability type names to runtime metadata, with an empty list elsewhere.
- [ ] Run focused native tests, workspace tests, formatting and strict Clippy; commit.

### Task 3: Action UI capability and retained-result integration

**Files:** create `src/provider-action-policy.ts` and `tests/provider-action-policy.test.mjs`; modify `src/main.ts`, `src/action-ui.ts`, `src/document-state.ts` and relevant existing coordination tests.

**Interfaces:** consume `provider_action_types`; produce `providerGroupsReason(groups: readonly ActionGroup[], supportedTypes: readonly string[]) -> string`. Extend `ActionUIHost.canExecute` and `executeReason` with optional `ActionGroup[]` arguments.

- [ ] Write failing policy tests for a supported metadata group, supported artwork group, disabled external-file group, enabled external-file group, explicit rename selectors, missing capability metadata and unchanged desktop behavior.
- [ ] Explain in-place provider action support; keep unavailable standalone file operations disabled. Use the selected groups/settings when deciding Run availability and reason, without duplicating the Guess Values parser.
- [ ] Update Quick/group/preset callers to pass their current settings into the optional capability callbacks. Preserve library editing and no-argument host compatibility.
- [ ] Integrate `reconcileWorkingEntries` into the immediate group result/error path, preserving operation errors and existing pending-field/reset choices.
- [ ] Run frontend/coordination tests and production build; commit.

### Task 4: Ordinary UI checks, review and delivery evidence

**Files:** update `docs/PARITY.md`, add source-attributed verification notes and normal UI receipts; add no diagnostic tooling.

- [ ] Run full source checks and hosted platform builds; inspect actual results.
- [ ] Review the whole change independently; address confirmed findings and re-review the fixes.
- [ ] Exercise normal Quick/group execution on generated audio through native UI in Light/Dark; verify resulting tags, unchanged audio and unselected files. Mark every unexecuted device case explicitly incomplete.
- [ ] Record exact source/build identity, ordinary screenshots and remaining external-file/rename/folder/iOS/performance work. Deliver a separate reviewable PR based on the document integration branch where appropriate.

## Self-review

Every spec section maps to Task1 parser/capabilities, Task2 execution/context,
Task3 UI/results or Task4 ordinary verification/delivery. No action dialect,
core schema, permission or dependency expansion is planned. The 13-type increment
does not remove the remaining five action types or filename/directory branches
from the overall goal. Existing unfinished Android rechecks remain unfinished.
