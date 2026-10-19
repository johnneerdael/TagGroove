# MPEG preservation and fresh defaults — 2026-10-19

The MPEG correction combines the fresh read default with Save preservation.
Fresh settings read ID3v1 and ID3v2, with APE reading off. Ordinary Save writes
only its enabled kinds and preserves unwritten kinds; explicit Remove uses
its separate Remove selections. An explicitly saved APE-read choice remains
effective. This is a bounded TagGroove implementation and local verification
record, not original-application runtime acceptance.

## Reference authority

The pinned Windows x64 Mp3tag 3.36.1 executable has SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
[MPEG default static seeds](mpeg-default-static-seeds-2026-10-19.md) records
the read/write/remove/encoding initialization evidence.
[MPEG Save and Remove static evidence](mpeg-save-mask-static-2026-10-19.md)
records the separate Save/Remove gates, nonempty APE-only suffix retention,
custom-only ID3v1 projection, and the three-collection empty predicate.
Those five retained phases establish static contracts; no original Windows,
Wine, GUI, user profile or provider was executed.

## Implemented behavior and preservation controls

| Case | Result and local observation |
| --- | --- |
| Ordinary default Save | Writes enabled ID3 kinds without deleting hidden APE. An independently seeded APE header/items/footer region, including conflicting values, gain, opaque binary data and a real embedded PNG, remains byte-identical. |
| Hidden APE-only fields | With APE reading off, its TITLE/ALBUM/gain/artwork do not become displayed or newly written ID3 values. Physical APE presence remains distinct from displayed metadata. |
| Conflicting gain values | Default display reads the ID3 gain of `-7.25 dB`; retained APE still holds its independent `-3.00 dB` gain and MP3GAIN records. Explicit APE read/write instead uses the APE values. There is no hidden-gain migration into ID3. |
| Nonempty APE-only Save | Leaves all 128 bytes of an unwritten existing ID3v1 tag unchanged and does not add absent ID3v1. Enabling both writers updates both kinds. MPEG's APE option no longer promises implicit V1 mirroring; the APE-family policy is unchanged. |
| Different read and write sets | Verification reads the highest effective written kind independently of display priority. An explicitly APE-enabled read can therefore still display old retained APE after an ID3-only Save. APE-only writes also work with APE reading off. |
| Explicit Remove | Removes its selected kinds independently of Read and Write. Physical tag boundaries and encoded audio bytes are checked separately. |

The tests observe complete unwritten APE bytes, complete V1 bytes, ID3 prefixes
and encoded audio ranges independently of the production mapper. In-memory
APE/audio mutations demonstrate observer sensitivity without changing disk or
history. Actual journal controls require current-operation admission and exact
whole-file Undo/Redo; the production 2 GiB reserve is unchanged.

## Empty updates, empty models and opaque sources

An empty `Fields::new()` update on nonempty metadata is a no-op: displayed
values, tag presence and complete file bytes remain unchanged. A same-value
nonempty update separately exercises serialization. Neither is evidence of an
empty supplied metadata model.

A nonempty CUSTOM-only model can have no representable ID3v1 fields. Its
enabled V1 writer still creates or replaces a blank 128-byte projection using
the existing serializer. This differs from removing V1 for a known-empty full
model; exact original genre/default bytes are not qualified.

For a plain displayed model proven empty after its last fields/pictures are
removed, MPEG Save removes the effective **written** kinds. That includes
artwork and opaque binary data in an unread written target. Controls clear a
plain ID3 source into a hidden written APE target containing both PNG and binary
records, and a plain APE source into a hidden written ID3v2 artwork target.
Unwritten kinds and audio remain unchanged. The private written outcome records
the required absences, and readback checks every selected kind physically.

The native predicate counts text, non-picture binary records and pictures.
TagGroove's Fields/Pictures view cannot prove that complete native model for
every source. Its concrete displayed-source guard preserves APE non-cover
binary/locator records and conservatively guards ID3 opaque, special,
transformed and placeholder records. Clearing the last text field therefore
does not blanket-delete those sources. Tests retain APE item flags/key/data and
ID3 unknown/PRIV or chapter records; a written APE envelope may be reserialized,
so its unchanged item bytes are distinct from complete **unwritten** APE identity.

This guard is deliberately conservative, not an emulation of every native
third-collection mapping. Generic native ID3 unknown/PRIV descriptions use a
separate technical channel; special binary mappings and complete opaque-only
Save behavior remain unqualified. Unread target records do not determine the
emptiness of a plain supplied source model.

## Defaults and persistence

Core serde controls distinguish missing/partial settings from explicit
`mpeg.read_ape: true` or `false`, preserve both choices through roundtrip, and
keep APE-family defaults independent. The shared frontend/backend default
fixture and actual Options controls use fresh MPEG APE-read off and APE-family
APE-read on. Options tests retain saved opt-in until an explicit page reset,
without resetting unrelated settings.

Backend regressions cover settings-file load/save and portable settings-only
restore of explicit MPEG opt-in, followed by rollback to the destination's prior
choice. These cases passed in the parent full-workspace run. Three older
test files required explicit APE opt-in and independent raw ID3/APE/audio
retention expectations after the default/policy correction; their targeted
28-test cohort and the repaired full suite both passed.

## Executed evidence and current boundary

The repaired local cohort passed 13 MPEG preservation tests and 82 affected
integration tests in total, including those 13. The seven target counts were
11, 19, 4, 13, 8, 10 and 17. The two hidden-written-artwork cases first failed
semantically with 11 other controls passing, then passed after the source guard
and explicit-absence repair. Earlier construction errors and an overly strict
selected-APE envelope expectation are retained separately and are not counted
as product failures. Core preference tests passed 3 cases and Options tests
passed 12. Scoped core Clippy, rustfmt and diff checks passed at the author freeze.
Two independent reviews reported no remaining finding in that repaired scope.

The immutable repaired receipt has SHA-256
`3065f55286932d32112a3e183061d08d39c9027f6e93b7f6756028b27d0df5e4`;
it binds the passing policy source at
`7f33a03310574824ddff19419bdc0cf497623c477984758809065277ecf3d410`
and policy tests at
`d31a059a608f86e88e378315e3e5ca909b78d95a98f0a5fc27085b2bd270ef1e`.
Raw logs, fixtures and receipts remain outside Git. The later diagnostic-only
change reuses the existing localized message `The tags could not be removed`;
the diagnostic repair was subsequently formatted to one line. Current policy
source SHA-256 is
`ec419ebb3e16b20ad42eb64334a088c396044be297fce24253776ecb34ea3c9f`.
The earlier receipt is unchanged and is not relabeled as execution of that
later source hash.

Parent integration passed **1,369 Rust tests, zero failures and 18 existing
ignores across 82 result groups**, plus **828 frontend tests**. Strict all-target
workspace Clippy, the production TypeScript/Vite build, canonical catalog, eight
documentation tests and strict guide build passed. Formatting passed after the
one-line diagnostic formatting correction; all 13 MPEG policy regressions were
repeated successfully after it. The full Rust run preceded only that formatting
change. Initial obsolete-fixture failures and a separate localhost-denial test
profile error are retained in the external logs, not relabeled as passing runs.
The full paired-consumer Python suite also passed **30 tests**, including the
exact default/explicit-APE policy inventory guard and independent gain/native
byte/history corruption controls. Its raw log SHA-256 is
`841e72625b9dacd585fcc8d2445ff521e688ea9d7207c260e2addb7243e35567`.
A separate fresh retained prepare/run receipt passed all eight local cases;
reference comparison was not performed. These are synthetic/local consumer
checks, not original Mp3tag outcomes.

No final-head Codex code review, merge, CI or release result is claimed.
Raw AAC/TTA inherit the existing MPEG stand-in adapter and have local affected
tests; this does not establish original AAC/TTA policy. Other format-family
production policies, complete field-level mirroring, encoding reuse, actual
loaded-profile defaults and original physical before/after bytes remain
qualified as described in the linked static notes.
