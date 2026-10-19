# Filtered whole-value suggestions — 2026-10-19

The owner approved automatic matching lists alongside the existing single-line
inline completion in [the design](../superpowers/specs/2026-10-19-filtered-value-completion-design.md).
PR64 implements that interaction in the tag panel, File List metadata cells and
Extended Tags. Choosing a suggestion never independently saves tags.

## Production behavior

The shared [ValueSuggestions controller](../../src/value-suggestions.ts) captures
the typed prefix before inline completion. It filters fresh host candidates,
preserves source groups and exposes all matches through a window of at most 16
heading/option rows. Actual CSS supplies the 30px desktop/44px mobile row size
used by virtual navigation. Values use literal text and are not translated.

Ordinary lossless single-line values retain suffix completion. Multiline values,
multiple-value arrays, endpoint whitespace and literal value separators remain
available for explicit whole-value choice. A candidate is not initially active:
Arrow keys highlight, Enter chooses, and Tab retains the editor's normal policy.
Escape closes suggestions before the host's later cancellation. Pointer selection
prevents an old-value blur commit. The explicit panel dropdown still offers its
normal commands and unfiltered values, with one popup open at a time.

Dedicated multiline panel/Extended Tags controls keep Enter-to-newline editing
unless a suggestion is explicitly active. File List cells retain Enter/Tab
commit/navigation after a choice; metadata cells use a textarea to avoid losing
line breaks, while filename controls retain single-line behavior. Filename,
directory and technical fields do not acquire metadata suggestions.

The General option, disabled/read-only state and host eligibility gate the
feature and accessible `aria-autocomplete` state. Focus, keys, attribute changes,
field changes and reattachment refresh capability without reading candidates or
editing text. IME composition reads no candidates and neither completes,
navigates nor commits; valid late non-composing input remains supported.
Rerender, removal, blur, modal close, scroll and resize dispose or close popups.

## Exact staging boundary

DOM textarea values normalize line endings, and ordinary typed panel/cell text
uses an existing trim-and-split parser. Preserving only displayed editor text
would therefore corrupt an explicitly chosen value before it reached the draft.
Choices now carry their original value arrays through the existing host
draft/commit APIs to `main.ts::draftField`/`commitCell` and `updateDraft`.

Actual choices preserve leading/trailing spaces, LF, CRLF and CR, plus a literal
`\\` inside one scalar. A genuine array stays several values. Array identity,
not joined display text, controls mixed selections, deduplication and the
unchanged-value guard. The popup quotes separate members of an array; prefix
matching still uses the normal field text. Literal `< keep >`/`< blank >` values
are distinct from the panel commands. Extended Tags uses its existing raw-value
snapshot; a real edit invalidates preserved choice context. Ordinary typed
delimiter parsing and filename policy remain unchanged.

The panel also retains the selection's values from the rendered control,
including pre-existing pending values. Its ordinary input publisher records the
actual parsed values it just staged without rereading candidates. Fresh source
values matching that own-write signature keep the original choices through
`R` → `Ro` typing; a legitimate source change resets them. Selection/reload and
field rerenders clear the control origins. Custom lists remain fresh, and the
manual dropdown uses the same origins. This state belongs to the rendered panel;
there is no new global or persistent candidate store.

## Evidence and remaining qualification

The first real production-panel regression reproduced inline `Ro` → `Rock`
without the missing `Rock`/`Rockabilly` popup. Review-driven RED controls then
exposed missing groups, mobile geometry, LF/CRLF normalization, incorrect disabled
ARIA, endpoint trimming, scalar/array display collisions and command/data
deduplication. Separate-keystroke tests additionally reproduced and repaired
candidate loss caused by the panel's own draft publication. Repairs exercise
actual production controls and the authoritative main staging functions,
intercepting the arguments at `updateDraft`; source inspection verifies that
function preserves the supplied arrays. These controls do not save audio.

The frozen feature suite has **55 passing controls**. The complete frontend
suite has **822 passing tests, zero failures/skips**. Production TypeScript/Vite,
the unchanged 3,392-key catalog check and strict MkDocs pass. Independent scoped
reviews found no remaining confident issue after the lifetime repair.
Raw author logs, RED checkpoints and final source hashes remain outside Git.

These are controlled production-DOM/build checks. Native keyboard, pointer,
screen-reader, visual/mobile-browser layout, large-library performance and
original Mp3tag comparison remain unqualified. No native app/browser or original
runtime was launched. This does not implement L47b cross-session manual order,
change portable-backup contents, supply hosted CI or satisfy final GitHub Codex
approval.
