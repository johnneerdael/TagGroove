# Composition-safe editing — 2026-10-18

Scope: concrete composition-event defects in consolidated draft PR64. This is
production-function/DOM evidence, not a native input-method or original-Mp3tag
session. Native desktop/mobile keyboard and IME qualification remains L78/L63.

## Reproduced failures and resulting behavior

The shared inline completer previously treated `insertText` and
`insertCompositionText` as ordinary completed input even when `isComposing`
was true. It replaced the current text and selection. The file-list cell editor
also committed, canceled or navigated on composing Enter/Escape/Tab. Tag-panel
keys could open/accept a value popup or open a partially composed directory.
Global save/removal shortcuts and manual-filter Enter did not exclude composing
key events. Automatic filtering could rebuild the workspace before composition
finished, including the compact/provider/empty-list paths that use full render.

The completer now leaves composing updates untouched, including an explicitly
named composition update without the flag. Cell/panel/global shortcut handlers
leave composing key events to the input method. The filter retains the edited
text without refreshing while composition is active; composition end applies
automatic mode, while manual mode retains the draft until a separate Enter.
Repeated final-input notification does not refresh an unchanged query again.
An explicit non-composing `insertCompositionText` notification arriving after
composition end likewise retains committed state instead of creating a draft.
Normal non-composing editing, completion and shortcut behavior remain covered.

The standard event meaning is documented in [W3C UI Events](https://w3c.github.io/uievents/#dom-inputevent-iscomposing).
The implementation uses the reported event state; it does not claim that every
native input method/webview reports an identical sequence.

## Executed controls

[`tests/ime-editing.test.mjs`](../../tests/ime-editing.test.mjs) runs nine controls
against actual production modules, actual bound DOM events and the current
renderer handlers extracted through TypeScript's syntax tree:

| Control | Evidence required |
| --- | --- |
| Input and textarea completion | Composing text/selection remain exact; candidates are not read; ordinary input still completes with the suffix selected. |
| Composition-type fallback | An explicit `insertCompositionText` update is not completed when its composing flag is absent. |
| Cell Enter/Escape/Tab | The same editor remains open with no commit/cancel/navigation; a later ordinary Enter commits the actual value. |
| Typed directory | Composing Enter makes no directory request; ordinary Enter uses the typed path. |
| Panel popup | Composing arrow/Enter/Escape/Tab leaves popup and active choice intact; ordinary Enter performs its actual choice. |
| Global Save/Remove | Composing shortcuts make no operation; ordinary Save and row Remove still dispatch, while typed-field Remove keeps its existing reload suppression. |
| Manual filter | Composition end keeps a draft; a separate ordinary Enter applies it. |
| Automatic filter | No refresh during composition; final text applies at composition end, duplicate notification does not refresh twice, later ordinary typing applies. |
| Panel shortcut | Composing Ctrl+Q leaves panel state unchanged; ordinary Ctrl+Q toggles it. |

The first seven controls failed against the prior production handlers; a separate
automatic-filter control failed before the input/composition-end binding was
fixed. The final focused run passed **24 tests** across IME editing, General
options and existing keyboard/UI controls. No original program, native window,
picker, clipboard, provider or device was used, and no audio was opened/written.

Filtered suggestions and multiline completion remain L59. System-locale refresh
remains L60: technical dates originate from the system locale, so changing UI
language alone does not establish that contract or justify rewriting them.
