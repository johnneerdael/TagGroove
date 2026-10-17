# Track and disc numbering parity

The user redirected work to Mp3tag feature gaps and away from Android UI improvements. Continue in the existing isolated worktree on `feat/numbering-parity`; retain the unfinished provider-actions branch independently. The existing action-policy checkpoint remains in this branch's ancestry.

Reference: https://docs.mp3tag.de/converters/auto-numbering/ (read 2026-10-14).

Extend the existing numbering operation rather than introduce a new subsystem. Preserve its table-order selection and pending-edit/Save tags workflow, appearance tokens, and format-specific field mapping. No security tools, binary analysis, or diagnostic simulations are needed.

## Behavior

- Compute a pure Rust plan from ordered paths, current field snapshots, and numbering options. Return logical TRACK/DISCNUMBER patches without reading or writing files.
- Support independent track/disc starts, padding, and editable optional totals. Prefill track total with the selection count. Explicit totals remain constant, including across counter resets.
- Increment the disc counter once when either the directory or selected tag field changes. Compare adjacent rows, not unique directories/values; preserve input order.
- Optionally reset tracks on a new disc. Reset the disc counter on another field change and implicitly reset tracks; this reset takes precedence over simultaneous increment triggers.
- Compare tag names without ASCII case sensitivity and preserve complete multivalues. Validate all options and checked numeric increments before returning any plan.
- Adapt logical patches with the existing per-format field mapping when staging drafts. Display a preview and leave files unchanged until the existing Save tags command runs.
- Reuse the existing dialog style and Light/Dark support. Do not change Android layouts or document access. Directory options must not derive grouping from provider working-copy folders.

## Delivery checklist

- [x] Add failing core tests for ordered counters, totals, triggers, reset precedence, multivalues, and overflow.
- [x] Implement the Rust planner and Tauri command.
- [x] Extend the existing dialog with documented options, preview, error handling, and native field adaptation.
- [x] Run the full existing test suite, build, formatting, and Clippy.
- [x] Exercise the actual desktop workflow on generated audio, including Save/reload and Light/Dark; retain precise evidence boundaries.
- [x] Update the parity ledger, record remaining original-runtime differences, and commit the verified changes.

Documentation establishes the controls and transition rules, but does not specify every padding width, multi-value comparison, or simultaneous-trigger edge case. Tests of TagGroove alone must not be presented as original-runtime equivalence.
