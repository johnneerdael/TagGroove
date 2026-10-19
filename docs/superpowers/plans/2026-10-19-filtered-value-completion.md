# Filtered Metadata-Value Completion Implementation Plan

> **For agentic workers:** Use the subagent-driven-development skill for the assigned frontend task and verification-before-completion before handoff. Execute the checklist in the existing PR64 worktree; no new checkout, branch or PR.

**Goal:** Implement approved choice A across tag-panel values, File List metadata cells and Extended Tags, including explicit whole-multiline-value selection.

**Architecture:** Keep candidate ownership and draft/commit ownership with existing hosts. Add a small shared suggestion-list controller using existing listbox styling, prefix/inline helpers and the repository’s existing virtual-list facilities where suitable. Preserve the manually opened tag-panel list and its Keep/Blank/directory commands; close one list before opening the other.

**Tech Stack:** TypeScript, existing CSS/theme tokens, Happy DOM, Node’s test runner, installed esbuild/Vite. No new dependency or settings schema.

---

## Task 1: Record feature RED against actual controls

**Files:** Create `tests/value-suggestions.test.mjs`; inspect/reuse `tests/ime-editing.test.mjs`, `tests/tag-panel.test.mjs` and existing test loaders.

- [ ] Load actual production TagPanel with installed esbuild (if runtime imports require bundling), and use Happy DOM rather than reconstructing its implementation.
- [ ] With values `Rock`, `Rockabilly`, `Pop`, type `Ro` into a normal metadata field. Assert the existing inline result remains `Rock` with suffix selected, and actual popup options include both `Rock` and `Rockabilly` while excluding `Pop` and Keep/Blank commands. On baseline, the popup assertion must fail because typing closes the list.
- [ ] Use the equivalent direct DOM input sequence, not a test-only application API:

```javascript
input.value = 'Ro';
input.setSelectionRange(2, 2);
input.dispatchEvent(new window.InputEvent('input', {
  bubbles: true, inputType: 'insertText', data: 'o'
}));
assert.equal(input.value, 'Rock');
assert.deepEqual([input.selectionStart, input.selectionEnd], [2, 4]);
assert.deepEqual(optionTexts(), ['Rock', 'Rockabilly']);
```

- [ ] Run `node --test tests/value-suggestions.test.mjs` with raw redirected output under `/tmp/taggroove-value-suggestions/`; confirm a behavioral missing-list failure, not an import/syntax failure.

## Task 2: Implement the shared controller and panel integration

**Files:** Create `src/value-suggestions.ts` if a focused shared controller is appropriate; modify `src/general-options.ts` only where needed to reuse prefix logic, `src/tag-panel.ts`, and `src/style.css` for scoped list positioning/virtual row styles only.

- [ ] Use a host-provided candidate callback and explicit choice callback. Preserve existing candidate order, groups and duplicate policy; values are literal text, never HTML or translated labels.
- [ ] Capture raw input before inline insertion. Single-line completion retains the existing helper semantics; automatic filtering uses the raw prefix. Multiline controls do not insert a suffix.
- [ ] Keep all matches reachable while bounding rendered option DOM. Prefer existing virtual-list arithmetic and fixed list-row geometry over a new framework; keyboard navigation must render the active option so `aria-activedescendant` always exists. Long multiline values can have a single-row display with the full literal value available to assistive text/title.
- [ ] Consume Enter only for an explicitly active suggestion and before the cell/global save handler; no initial automatic active option. Escape closes the suggestions first. Tab retains the existing editor policy. Pointer selection prevents blur from committing the old value.
- [ ] Add source-appropriate combobox/listbox semantics, stable unique option IDs and active state. Use the closest open dialog as popup parent for Extended Tags top-layer visibility; otherwise use the document body.
- [ ] Dispose or close on blur, input removal/rerender, modal close, scroll/resize and context/field change. Global listeners must not retain detached controls.
- [ ] Preserve manual full tag-panel lists, special commands, directory behavior and existing shortcuts; automatic suggestions and manual lists must never coexist.
- [ ] Run the focused feature test until green, then the existing tag-panel and IME tests.

## Task 3: Wire File List and Extended Tags

**Files:** Modify `src/cell-editor.ts`, `src/extended-tags.ts`, `src/main.ts` and their existing host/test adapters; keep interfaces type-only where tests transpile standalone modules.

- [ ] File List metadata controls receive fresh custom/loaded candidates. Filename and technical fields receive none. Choosing a suggestion changes only editor text; first Enter consumes suggestion selection, later Enter follows current commit/advance behavior. Cancel/finish/reattach closes stale popups.
- [ ] Extended Tags completion uses whole value entries, preserving the distinction between newlines and multiple tag values. Explicit choice marks the existing value edited via the current draft pathway; it never calls Save. Normal Enter with no explicit active suggestion inserts a newline.
- [ ] The General option gates all three hosts. Disabled/busy/read-only inputs suppress popups. Existing host draft/changed callbacks remain the only state publication paths.
- [ ] Keep the low-level `completion`/`completeInput` contract tests that still apply, and update real host adapters to test the new path rather than retaining a parallel fake legacy path.

## Task 4: Verify interaction and lifecycle boundaries

**Files:** Extend the new feature suite and relevant existing suites.

- [ ] Assert several/no/exact-prefix matches, retained user casing and selected suffix, Unicode, untrusted HTML-like values and fresh candidate changes.
- [ ] Assert actual panel choices update the draft once; cell choices do not invoke commit; Extended Tags choices do not invoke Save.
- [ ] Assert multiline whole-value selection with other lines preserved until explicit choice, ordinary Enter and later newline behavior.
- [ ] Assert Enter/Tab/Escape and pointer-vs-blur ordering across actual hosts, read-only/disabled/busy and option-off cases, rerender/removal cleanup, modal-parent placement and bounded option DOM with all matches reachable.
- [ ] Assert composition does not read candidates or alter text/selection/navigation, and retain the valid late non-composing input controls. Preserve field/directory and global save/remove shortcuts.
- [ ] Run `node --test tests/value-suggestions.test.mjs tests/tag-panel.test.mjs tests/general-options.test.mjs tests/ime-editing.test.mjs` plus other directly affected host suites identified during implementation, redirecting raw output.

## Task 5: Guide, catalog and handoff

**Files:** Update affected `docs/user-guide/reference/options/general.md`, `docs/user-guide/guides/tag-panel.md`, `docs/user-guide/guides/extended-tags.md` and inline-edit guidance where behavior changes. Add only genuinely new interface text to English/Dutch/German through the existing extraction workflow.

- [ ] Teach how automatic matching differs from the full dropdown, choosing vs committing/saving, multiline behavior and IME/free typing.
- [ ] Run `npm run build`, `node scripts/i18n-extract.mjs --check`, complete `npm run test:ui`, and `git diff --check`, preserving raw logs. Do not run Cargo while the source-workflow author owns it.
- [ ] Run strict MkDocs with an isolated output directory. Do not launch native apps/browsers or fabricate visual proof; record controlled-DOM/build and native/visual limits separately.
- [ ] Freeze owned paths and hashes, provide test/build/catalog logs and a scoped receipt; request independent review from root. No staging/commit/push by the author. Root integrates after both concurrent source/frontend cohorts are ready, then refreshes ledger/body and the final exact-head gates.

## Coordination

The source-workflow author owns `src-tauri/src/sources/http.rs`, `sources/web/mod.rs`, `sources/web/workflow_replay.rs` and `tests/helpers/source-workflow-review.mjs`, plus the Cargo slot. Do not modify those paths. Their runtime review bundles only `src/tag-sources.ts`; approved frontend completion paths do not overlap. Root owns final guide evidence/ledger/release notes and performs final combined verification/review before any final GitHub CODE request or application CI.
