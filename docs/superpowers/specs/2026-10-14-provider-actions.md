# Provider Action Groups

## Intent and scope

Continue the full Mp3tag parity objective by running existing action groups on
Android-selected audio documents. Reuse the current action language, scripting,
regex implementation and library UI. Desktop execution must remain compatible.

This increment supports 13 action types that edit tags or embedded artwork:
Adjust Cover, Case Conversion, Convert Codepage, Format Value, Guess Values,
Merge Duplicate Fields, Remove Duplicate Fields, Remove Fields, Remove Fields
Except, Replace, Replace with Regular Expression, Set Cover Properties and
Split Field.

External-file Import Cover/Import Text File, Export/Export Cover/Generate
Playlists, filename/directory changes and folder workflows remain subsequent
work toward the same objective. Their unsupported settings must be reported
before any selected document is changed. This increment is not full parity.

## Global constraints

- Preserve the full feature/performance parity objective and existing Light/Dark behavior.
- Preserve desktop action behavior, action-library serialization and core tag schemas.
- Use existing selected-document access only; add no permissions or global configuration changes.
- Keep all mutation tests on generated audio; Beatport remains read-only.
- Use ordinary source tests, builds and normal UI checks; do not resume stopped diagnostic device sequences or binary analysis.
- Keep unexecuted device checks explicit; source tests and builds do not certify device behavior.

## Capability rules

The action type alone does not determine support. Existing transform semantics
include filename changes for `_ALL` in Case Conversion, Convert Codepage,
Replace and Regex Replace. Those selectors, `_FILENAME` and `_DIRECTORY` are
unsupported for those actions in this increment. `_TAG` remains metadata-only;
`_FIELDNAME` retains its existing APEv2/VorbisComments format requirement.

Merge Duplicate Fields, Remove Duplicate Fields and Split Field use `_ALL` for
metadata only, so that valid behavior remains available. Format Value permits
ordinary tag fields; its filename/directory targets remain unsupported.
Guess Values must inspect targets with the same parser used during execution.
It must reject technical/rename targets without approximate pattern matching.
Repeated targets retain their existing multi-value semantics; DUMMY is ignored.

Remove Fields/Remove Fields Except retain embedded-artwork removal semantics.
Do not reinterpret them as tag-only operations. Disabled groups are ignored;
enabled unresolved imported actions and malformed settings produce clear errors.

Validate all enabled groups and selected documents before starting edits.
If a group needs an unavailable operation, report its name and one-based action
number. No earlier group may be applied merely because the unavailable action
was encountered later. Unknown, read-only or recovery-required selected records
prevent execution; the UI explains the selection state.

## Managed execution

Add a focused `src-tauri/src/managed_actions.rs` adapter. Keep the existing
desktop `action_host::execute` path. The managed adapter returns the existing
`ActionResult` shape, with no rename/directory results in this increment.

Preserve first-seen managed identity order and stable selection counters. Each
enabled group runs against eligible files; a runtime error stops later groups
for the affected file while other files can continue. Publish at group boundaries,
consistent with the existing desktop group flush behavior. This is not a promise
of multi-file atomicity. Earlier verified groups remain saved; the existing
original-copy option continues to mean the last verified source.

Use `documents::mutate` for changes. Do not write through an ordinary desktop
path as an alternative. Capture an owned, immutable `DocumentProjection` before
entering the mutation callback. Its pure `apply(&mut AudioEntry)` method reuses
the current source projection: provider URI/name and known dates, with working
folder context omitted. The callback must not reenter the document store.

The managed ActionHost reuses core group evaluation for one file/group. Flush
field changes before an artwork effect and at group end, as the desktop host
does. Refresh fields and measured artwork context after effects, then reapply
the immutable projection. Keep raw tag values, deletion deltas and format rules.
Use existing `EvaluationSettings`; app/locale context and columns remain intact.
Column snapshots apply only while the file's fields still match the request's
initial fields. Keep initial fields across groups rather than resetting them.

Evaluate pure metadata groups once before writing. If fields are unchanged,
return the existing entry without publication. Otherwise apply the prepared
field delta through the document mutation callback without evaluating expressions
again. Groups with possible artwork effects execute once inside that callback.
Do not evaluate random or stateful expressions twice to detect a no-op.

Only fully completed files appear as successful entries. Preserve errors and
retained contents for files that stop part-way through a request. Reconcile those
entries through the existing `reconcileWorkingEntries` UI helper; never describe
retained local contents as a successfully saved source.

## Interfaces

- Core: `guess_value_targets(pattern: &str) -> Result<Vec<String>, String>` shares
  a compiled pattern parser with existing `guess_values`.
- Managed adapter: `IN_PLACE_ACTION_TYPES: &[&str]`,
  `validate_groups(groups: &[ActionGroup]) -> Result<(), String>`, and
  `execute(paths: &[String], groups: &[ActionGroup], settings: EvaluationSettings)
  -> ActionResult`.
- Documents: reexport an internal, cloneable `DocumentProjection` with pure
  `apply(&self, entry: &mut AudioEntry)`; expose
  `projection(path: &Path) -> Result<DocumentProjection, String>` for existing
  managed records. No new serialized document state is required.
- Runtime capability: add `provider_action_types: Vec<&'static str>` to
  `RuntimePlatform`; Android returns the 13 supported type names, other platforms
  return an empty list. Desktop UI execution remains unrestricted by that list.
- Frontend: `providerGroupsReason(groups: readonly ActionGroup[],
  supportedTypes: readonly string[]) -> string` handles known unsupported types
  and explicit transform/Format Value rename targets. Guess Values settings are
  checked authoritatively by the backend before changes.
- ActionUI host: make existing `canExecute` and `executeReason` callbacks accept
  optional `ActionGroup[]` arguments. Existing no-argument hosts remain valid.

## UI behavior

Ready writable Android selections can execute supported Quick actions, selected
groups and presets. Library editing/import remains available independently of
whether execution is possible. A known unavailable type/explicit rename target
disables Run with a useful explanation. Backend validation remains authoritative
for complete settings, including Guess Values targets.

Do not remove the remaining operation explanations for standalone renames,
imports and exports. Do not globally enable all provider operations. Retain
pending-field requirements, source access notices, unavailable-metadata notices,
normal menu behavior and both appearances. Group errors must preserve the original
error while refreshing retained metadata/artwork through the existing helper.

## Acceptance and remaining work

Use the existing ordinary Rust/TypeScript test harness and generated audio to
verify supported settings, exact group ordering, raw/multiple values and field
deletions, artwork, source identity/context, counters/columns, invalid selections,
unavailable actions before changes, no-op behavior and existing error reporting.
Keep the previous reset/Discard and retained-metadata tests passing.

Build Windows/macOS/Linux/Android, review the whole change, and exercise ordinary
Quick/group execution on generated copies through the native UI in Light/Dark.
Check resulting tags and audio independently. Do not resume diagnostic device
sequences. Existing unexecuted Android checks remain explicitly open.

The five external-file action types, all rename/directory branches, folder and
iOS workflows, wider format/provider/device coverage, and measured original-app
performance/semantic comparisons remain required subsequent work. Update
PARITY.md without converting this increment into a full-parity claim.
