# Local parity closeout execution

Owner objective: finish all work executable on this Mac, retain an OS-specific engineer handover for actual external requirements, complete exact-head GitHub Codex review rounds and trusted CI, and merge PR64. This is an execution checkpoint, not a claim that the whole program is complete.

## Filter quoting (C55 / L11)

The original 12 native-derived vectors were hash-verified and copied unchanged to `crates/tag-core/tests/fixtures/parity-native/filter-quoting.json`. Before repair, five cases failed: a quoted phrase matched words spread across fields, and quoted-prefix/bare-literal mixed operands were refused. The preserved failing executable and source-bound logs are under `taggroove-local-parity-native-red-build-j_exyzis`.

Quoted plain search and quoted HAS now match one contiguous phrase within a value; bare words retain word matching. Leading quoted fragments supply the phrase while trailing bare text is ignored. Bare-first embedded quotes remain literal. Boolean precedence, grouping, regex/format failures, missing operands, limits and existing operator-specific grammar remain guarded. Permanent controls include exact/nonmatching phrases, multivalues, quoted boolean leaves and literal quote characters. A separate double-negation/grouping regression first exposed inconsistent mixed-operand parsing in a boolean leaf; it was repaired through the same literal-operand reader and passed without changing frozen predictions. Its failing executable/log remain under `taggroove-local-parity-boolean-closure-red-y_u8mqh8`.

All 12 frozen vectors now agree with their static predictions. This is local product execution against static original analysis; original Windows runtime, locale and broader grammar acceptance remain qualification under C55/L12/L79.

## Split Field (C56 / L15)

The original eight native-derived vectors were copied byte-for-byte after SHA verification. All eight failed before repair. Publication now compares original value count with raw split-piece count before removing empties. Equal counts retain original values exactly; changed counts trim pieces, remove empties and delete an empty field. Literal separators, order, duplicates, unrelated fields and no-host-effects controls remain intact.

Seven vectors additionally run through the actual desktop action command on independently generated nonzero PCM WAVs with an opaque RIFF chunk and an unselected file. They verify actual saved values/field removal, unchanged PCM/opaque/unselected bytes and exact Undo/Redo. The unchanged-count padded value is an exact no-op with no Undo entry. The explicitly empty-string model vector is not claimed as a physical WAV input: that writer normalizes it to field absence.

## Executed checks

- 39 focused filter/action/guide tests pass, including both frozen-vector cohorts.
- 46 action unit tests pass, retaining old action-codec and failure controls.
- 16 backend history tests pass, including the seven real Split Field action/write/history cases.
- Strict all-target Clippy for tag-core and the backend passes.

Raw source bindings, compiler/test commands, logs and executable hashes are retained in `taggroove-local-parity-core-final-erlkyigl`, `taggroove-local-parity-actions-unit-uqw1k4s3`, `taggroove-local-parity-save-build-ngxf2fe7` and `taggroove-local-parity-clippy-89zl47w1`. These are focused results; final whole-branch review/CI and merge remain required. Production history limits remain 2 GiB for both retained bytes and free-space reserve.

## Remaining execution and handover

The [per-item OS/device handover register](../handover/PARITY-OTHER-MACHINES.md) records all starting intake and qualification identities, with local work still explicitly pending. Installed macOS Mp3tag1.14.1 is separate from the pinned Windows3.36.1 target. No original application was executed for these two repairs. Native action schemas, stronger content baselines, durable history and every remaining local ledger row are still active work.
