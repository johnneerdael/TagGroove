# Mp3tag numbering memory migration — 2026-10-20

This is unmerged L57b work. Nine native records now transfer supported numbering
choices to the existing `opentag.numbering` browser memory and durable setup
restore receipt. **L57b remains open pending the remaining-choice disposition
review.** This receipt does not establish complete profile coverage from an
allowlist count, original Windows equivalence, native acceptance or delivery.

## Pinned source and exact mappings

Windows Mp3tag 3.36.1 x64 SHA-256:
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Static PE inspection only; original not executed. These are raw 16-byte IDs.

| Native record identifier | Target numbering memory |
| --- | --- |
| `f4a314340ea7c044a6de766e7a2adc74` | `track_total` |
| `aaeda90a7158644ab6371c40786599a8` | `next_disc_on_directory` |
| `e242331383e5a2448bec645e3eabdd87` | `reset_track_on_next_disc` |
| `c90c8b8c433f5f4e8c86ba63e902fb8b` | `disc_enabled` |
| `5a98eb5f2200764a94daf6d4d9300c0b` | `disc_total` |
| `2da5f759503fa947932b7e5310a032fa` | Next-field enabled state |
| `36d2f91d9589434ea5669328ab6b9486` | `next_disc_on_field` text |
| `b9f3835eafb14c4ba2ca0a1e319f33a3` | Reset-field enabled state |
| `89d6949ac805e64b9eab9c7fa533a0db` | `reset_disc_on_field` text |

Seven byte booleans use first-byte nonzero semantics, skipping record extension
bytes. Two text records use the existing strict UTF-8 / first-NUL / 4 MB bound.
The native default for reset-track is true, but absent source records do not
import native defaults. Existing target memory is validated by the same
configuration-transfer schema before overlay; unmentioned values and unrelated
browser preferences remain unchanged. Malformed existing memory refuses before
publication rather than silently replacing it with defaults.

Wizard constructor `0x14024106c` selects RT_DIALOG `0x443`; DDX `0x1402412dc`,
OnInit `0x1402414fc` and OnOK `0x14024241c` bind the resource labels, members,
globals and save bytes. `TrackWizardResetFolders` means next disc on directory
change, separately from resetting the track counter on a new disc. The pinned
1800-byte resource has SHA-256
`b5b8a71b450b1a3916526fce48a81efa6298f98bdf8cbd2a60bc15b7d814dc6d`.

A follow-up completed the text codec evidence: OnOK's wide dialog text calls
`0x140026a10` at `0x1402426fd` / `0x140242774`, then writes the resulting byte
strings through globals `0x140ae88f0` / `0x140ae8960`. The helper calls
`0x14028ac00` for byte count and `0x14028a930` for encoding. Surrogate-pair
combination and UTF-8 one/two/three/four-byte emission are visible in the bounded
`0x14028a930..0x14028aad0` window (SHA-256
`a45c4778f16ad1002c597dafeab0f43628311b5cdada1a0f7c1b6a6a2cecf399`).
This is not a Unicode case-folding or malformed-original-text runtime claim.
OnOK also uses member `0x62c` in the second combo collection branch, while saving
the reset enabled byte from `0xa40`; this receipt does not infer a universal
original checkbox interaction truth table from the happy path.

## Pairing and unsupported choices

Explicit false clears the target field-change string. Explicit true requires
its own explicit, nonblank, non-control field name that does not start with `_`.
Rejecting the underscore prefix is an explicit conservative admission policy,
not proof that every possible underscore name is a native path expression. Text alone does
not invent an enabled state. Incomplete or unsupported pairs keep the current
paired choice and receive fixed diagnostics; invalid source field text is not
echoed. Unicode ordinary names transfer literally. The existing target planner
uses case-insensitive ASCII matching against tag fields; this work does not
create a new path-expression or Unicode normalization engine.

`_DIRECTORY` and other underscore pseudo fields are not ordinary tag fields and
are not substituted with the separate next-directory option. The later [mapping/cover/padding slice](mp3tag-mapping-cover-settings-migration-2026-10-20.md)
imports explicit leading-zero Off as one minimum digit; dynamic On remains L66,
since fixed digit counts are not equivalent for larger sequences. Track-disabled behavior has no target toggle. LastCodepage
(`719e4c7cd5ad814cba6915241b1d50d1`) is the Windows chooser row index, not LCID or
codepage. These choices remain skipped; their final migration dispositions still
need review. The original chooser order is host/locale dependent. Broader
unsupported codecs remain L56b. Original/native import/restart/restore remains
C12/C27/L78/L79 and is not replaced by the tests below.

At this slice the combined adapters recognized 68 portable native records, producing at
most 79 preview options (48 byte booleans, 17 packed switches, three choices,
ten text values and one u32-derived BOM switch). Disabled or invalid pairs can
produce fewer rows. This arithmetic is not a completeness proof.

## Verification

RED reproduced unsupported direct numbering records, then paired field values
remaining unchanged. GREEN: four focused profile tests, including eight
five-switch matrices (40 mapped-value checks), two partial/no-memory cases,
five invalid-existing-memory cases and seven paired-field scenarios. Checks
cover independent switches, extension bytes, Unicode, missing pairs, explicit
disable, invalid/pseudo fields, preserved unrelated counters and browser state,
unchanged source bytes and exact durable managed/window restoration.

Final profile cohort: 41 passed and one restart-worker entry ignored. Binary
settings cohort: 13 passed; frontend numbering/migration/configuration/catalog
cohort: 84 passed. These overlap the focused cases and are not additive.
Library/test-target Clippy, format/diff checks, catalog extraction, strict guide
build and all five ledger views (48 rows, seven complete CSV fields each) passed.
Read-only final source/tests/docs/locale review found no findings. No
application/native UI replay, original execution, provider request or GitHub
review/CI/merge result is claimed here. Import runs no numbering planner or
audio writer; its resulting choices are reviewed in the existing wizard before
numbers can be staged and then saved.
