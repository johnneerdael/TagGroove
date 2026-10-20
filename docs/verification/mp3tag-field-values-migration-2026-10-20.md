# Mp3tag field-value migration

Working-branch evidence for the custom-value portion of L57b, based on
`3801fc4`. General global/binary settings in `Mp3tag.cfg` remain an open part
of L57b. This is not complete profile parity, a merged-PR or release receipt.

## Reference evidence

The existing Windows 3.36.1 executable has SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Only bytes and bounded PE disassembly were inspected; no original app was
launched. Repository tests use independently authored data.

| Static location | Observation |
| --- | --- |
| `0x14093a018` | UTF-16 filename `usrfieldvalues.ini`, referenced at `0x140218578` before the field-value writer call at `0x1402185a7`. |
| `0x14021b390–0x14021b49b` | Reader obtains `custom_values` and `prefer_tag`; preference output starts at 1 and is replaced when the key exists. |
| `0x14021b014–0x14021b0f9` | Decoder passes U+000B as the value delimiter. |
| `0x1400489f0–0x140048b50` | Tokenizer advances over delimiter runs and returns nonempty runs. |
| `0x14021b148–0x14021b172` | Writer separates values with character `0x0b` and removes terminal separators. |

Delimiter/default rules are derived from these static paths, not original
runtime output. Pipes and backslashes are literal data. The official
[Tag Panel](https://docs.mp3tag.de/customization/tag-panel/) and
[Field Values](https://docs.mp3tag.de/customization/options/field-values/)
references separately describe per-field suggestions and selection preferences.

## Implementation

[Native decoding](../../src-tauri/src/mp3tag_field_values.rs) covers the
dedicated file and legacy panel entries. It retains literal spaces, order,
Unicode, pipes and backslashes. Empty delimiter runs and exact duplicates do
not add choices; an explicit empty list clears matching saved choices.
Invalid fields, controls, preferences and oversized lists are reported rather
than silently truncated. Original configuration files are not rewritten.

[Settings](../../src-tauri/src/tag_panel.rs) hold an optional per-field
preference map. Old settings retain their global fallback. The
[panel](../../src/tag-panel.ts) honors each override, including false, while an
empty field still offers custom values. File-list and Extended Tags
completion retain their independent custom-candidate route.

Panel import previews layout and legacy values; only OK publishes the patch.
Cancel, Reset and loading another layout do not publish abandoned import data.
Unrelated lists, genre choices and global defaults remain intact.

[Setup import](../../src-tauri/src/profile_migration.rs) exposes
`usrfieldvalues.ini` separately and allows one dedicated file per import.
Dedicated entries take precedence over selected legacy panel entries in either
selection order. The existing journal publishes and restores the native files;
Restore setup checks exact pre-import bytes separately from audio Undo.

The Field Values editor changes/removes overrides and preserves exact nonempty
lines when another line is edited or appended. Incoming plain-text list
normalization does not change existing literal values. Empty export passes `[]`.

## Executed checks

Focused macOS headless/filesystem groups overlap; do not add their counts.

| Scope | Result | Local run |
| --- | --- | --- |
| Native migration/delimiter/budget/refusal/persistence/order regressions | 7 passed | `taggroove-l57b-native-values-regressions-p0cm2ok5` |
| Panel import and settings regressions | 12 passed | `taggroove-l57b-panel-regressions-green-ykfbh968` |
| Setup publication/recovery | 28 passed; one ignored worker entry | `taggroove-l57b-profile-regressions-c372fn84` |
| Existing column import after raw-INI helper extraction | 5 passed | `taggroove-l57b-columns-regressions-k1nhs0xi` |
| Production panel, migration dialog, guide examples and suggestions | 93 passed | `taggroove-l57b-host-guide-cohort-056l9846` |
| TypeScript | Passed | `taggroove-l57b-types-complete-7o_lgjft` |
| Focused locale/catalog/validation checks | 21 passed | `taggroove-l57b-focused-i18n-green-vx52gnhx` |
| Scoped production Clippy | Passed | `taggroove-l57b-values-clippy-ft17ce0l` |
| Strict guide build | Passed | `taggroove-l57b-guide-strict-pb2keiel` |

Initial native and host regressions reproduced the missing import. Review also
identified same-list whitespace loss and empty export; both were reproduced
before fixing. Focused re-review is clear. The runner retains command, source
bindings, output and exit status.

The [native keyboard case](../../tests/e2e/keyboard-ui.js) checks literal entries
and independent true/false preferences through actual settings persistence.
It is syntax-checked only; execution belongs to postmerge CI. Original generated
files, Windows runtime, other-platform acceptance and general global/binary
settings remain unqualified.
