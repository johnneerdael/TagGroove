# Filtered metadata-value completion for PR64

Status: approved by the owner on 2026-10-19, choice A: automatic matching list
with existing single-line inline completion. The written proposal was supplied
before the owner selected that exact interaction. No second approval is pending.
Inspected baseline: `309e5bce6e7bbac4e0cd3b26c33c4308bb26823b`.

## Result

Offer all relevant matching saved/loaded metadata values while editing a tag,
rather than offering only the first inline match. Support the tag panel,
File List metadata cells and Extended Tags, including whole multiline values.
A suggestion changes an editor value/draft; it never independently saves audio.
Filename, directory and read-only technical-field controls retain their own
behavior. The General auto-completion option still disables the feature.

## Existing behavior and reuse

- `src/general-options.ts::completeInput` currently appends/selects the first
  longer case-insensitive prefix match and suppresses composition/paste/deletion.
- `src/tag-panel.ts` has an accessible grouped listbox with Keep/Blank commands,
  manual Arrow/Alt+Arrow opening, explicit Enter selection and blur/Escape cleanup.
  Typing closes it and only single-line text fields call completion.
- File List cells call completion over custom values plus loaded-file values;
  their Enter/Tab handlers own commit/navigation.
- Extended Tags already calls completion for its value textareas. It does not
  have a filtered suggestion list. Do not describe all multiline completion as
  wholly absent or rebuild existing value/field plumbing.
- Existing IME controls (C28) remain a required regression boundary.

## Interaction choices

| Choice | Single-line values | Multiline values |
| --- | --- | --- |
| A: automatic matching list (recommended) | Keep existing inline suffix selection and additionally show matching values while typing. | Show matching whole values; never insert a suggestion until explicitly chosen. |
| B: open suggestions manually | Keep existing inline completion; filter the list after the user opens it. | Open/filter through the dropdown or Alt+Down; choose a whole value explicitly. |
| C: choose every suggestion explicitly | Show matching values while typing, without appending any inline suffix. | Same explicit whole-value selection as A. |

A preserves the existing fast single-line entry habit while making alternative
matches discoverable. B limits popup movement during typing. C gives uniform
explicit selection but changes the current single-line inline-entry habit.
All choices use whole field values, not replacement of an arbitrary current
line inside lyrics/comments. Current-line completion would need a separate
product decision because candidates are stored metadata values, not line snippets.

## Approved detailed behavior (A)

1. Capture the user's typed prefix before applying any inline suffix; filter
   candidates using that prefix, not the generated completion suffix.
2. Keep existing candidate sources, ordering, grouping and deduplication rules.
   Automatic matching lists contain actual values only, not Keep/Blank commands.
   The explicit tag-panel dropdown still browses its full normal list/commands.
3. In multiline editors, a candidate is a complete value. Typing and Enter
   without an explicitly active suggestion keep ordinary multiline editing.
   Arrow navigation activates a candidate; Enter then chooses it. A subsequent
   Enter returns to ordinary newline entry. Alt+Down explicitly opens the list.
4. Enter that chooses a suggestion is consumed before a File List commit or
   global Save handler. A later Enter uses that editor's existing commit policy.
   Tab keeps the editor's existing navigation/commit behavior and closes the
   list; merely highlighting a different candidate is not acceptance.
5. Escape closes suggestions before falling through to the editor's existing
   cancellation behavior on a later Escape. Free text, including no matches,
   remains valid. Mouse selection must not cause blur to commit the old value.
6. Composition input does not read candidates, change text/selection, navigate
   the popup or trigger a save. Close stale suggestions at composition start.
   Preserve the existing valid late non-composing input behavior.
7. Pasting/deleting may update matching choices but never silently insert a
   suffix. Disabled/busy/read-only inputs do not open suggestions. Dispose popup
   state on rerender, removal, blur, selection/field change and modal close.
8. Use the existing design language and listbox/combobox semantics, with literal
   untrusted values escaped and never translated as interface labels. Keep long
   matching lists usable without constructing an unbounded popup DOM.

## Implementation boundary

Reuse the existing tag-panel list behavior and a small shared value-suggestion
controller where cell/Extended Tags need the same semantics. Do not introduce a
second candidate store, persistence format, save pipeline or unrelated component
framework. Hosts retain draft/commit ownership and provide fresh candidates.
Implementation details must follow the selected interaction choice.

## Verification

Exercise actual production DOM handlers: several matches/none/exact match;
custom/selection/genre precedence; Unicode and untrusted literal values;
whole multiline values and caret/selection preservation; Enter/Tab/Escape and
mouse-vs-blur ordering; composition and late commit; disabled option and busy
controls; rerender/removal cleanup; no save RPC from suggestion acceptance.
Keep existing IME, inline-entry, panel and cell-navigation controls. Run the
production build and affected controlled-DOM suites, then the consolidated
checks. Native keyboard/screen-reader and original Mp3tag qualification remain
separate evidence. No native GUI/browser/device activity is authorized here.

## Relationship to the other pending choice

This does not answer or implement L47b cross-session manual-order persistence.
The user's saved-settings/workspace-only portable backup decision remains
separate. No unsaved tag/artwork edits are added to a portable backup.


## Verification and visual scope

Use the current compact TagGroove list styling, system typography, theme tokens
and focus/active treatment. Matching values and optional source groups are the
content; opening, highlighting, accepting and closing are the interactions.
No decorative motion or new theme is needed.

The current session permits production DOM/build tests, not launching a native
app or browser. Preserve this constraint. Record the visual/native verification
limit instead of producing a synthetic screenshot as if it were runtime proof.
