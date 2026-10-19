# Shipped MPEG setting seeds — 2026-10-19

The pinned Windows 3.36.1 executable provides stronger default-setting evidence
than option-label strings alone. Its SHA-256 is
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
This is static initialization/control-flow evidence. No original app, Windows
environment, dialog or user profile was executed or observed.

| Published setting payload address | Raw integer | Statically mapped meaning |
| --- | --- | --- |
| `0x140ac32a0` | 3 | MPEG read: ID3v1 and ID3v2; APE bit 8 is absent. |
| `0x140ac3340` | 3 | MPEG write: ID3v1 and ID3v2. |
| `0x140ac32f0` | 11 | MPEG removal: ID3v1, ID3v2 and APE. |
| `0x140ac5870` | 1 | ID3v2.3 UTF-16 mode. |

The property getter/setter access the integer at object offset `+0x40`. Current
`CMTOptID3Dlg::Init` consumes masks 1/2/8; DDX maps the APE-read member to
control `0x614`, and Update reconstructs bit 8 from that same member. The mode-1
branch selects control `0x6df`. Independently decoded PE dialog resource
1066/language 1033 labels that control ID3v2.3 UTF-16. Raw data, instructions,
decompiler output and resource fields agree; a separate reviewer checked them.

The first key cross-references instead reached a guarded legacy Upgrade path:
old registry availability, a version comparison and a nonempty-version guard
control that migration. Its same-looking fallback literals are retained as
historical evidence, not used to infer fresh 3.36.1 defaults. The registry helper
also needs careful interpretation of returned size/type, not an invented
eight-byte persisted setting contract.

The useful source contrast is specific: TagGroove currently enables fresh MPEG
`read_ape`, while the shipped native read seed omits it. Its write/removal/mode
defaults agree with the mapped native seeds. Before changing that read default,
resolve its save consequences: the current writer removes APE on an ordinary
save and derives migrated text from the displayed kind. Reading APE off can hide
APE-only TITLE/ALBUM fields before that removal. It is not established that the
original couples its explicit Remove settings to ordinary Save in the same way.
A checkbox-only correction or advice to opt in cannot substitute for that
preservation comparison. Preserve explicit saved choices and independent
APE-family settings while investigating. Actual loaded-profile state, startup overrides and dialog
member construction remain unqualified; Init conditionally sets members rather
than proving every absent control is cleared. This is not full L39 acceptance.

Fresh retained Ghidra evidence, raw bytes, resource observations and their
23-record export are frozen outside Git. No proprietary native source or
pseudocode is copied here. L39/L79 retain original selected-control and configured
write/default consequences; reuse and MP4 initial settings remain unobserved.
