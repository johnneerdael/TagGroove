# Catalog/core foundation

This layer adds the pure translation and catalog-validation library plus the
complete English, German and Dutch catalog data from
`ed9beee4091268d12cecc961d6faadc9746637cb`. Each catalog contains 2,938
translation keys, with its existing metadata and keyboard-key data preserved.
The catalogs retain that checkpoint's exact bytes. The library additionally
includes the two source-confirmed corrections from this foundation's review: string
counts use the translator locale's decimal/grouping separators for plural
selection, and category validation visits nested plural bodies. Display glyphs
remain unchanged; an invalid nested key is excluded and reported once.

The existing application remains at main
`5c49582da4d4e6ba1e63e5fb2f7ce66c1918b0bf`: the language registry, DOM observer,
startup hooks, native catalog commands and Language settings are outside this
foundation. Catalog data is exercised by the pure tests rather than installed
in the application. Existing English application behavior remains the baseline.

Seven existing pure tests cover parsing, placeholders, plural rules, locale
digits, fallback/whitespace behavior and complete German/Dutch key validation.
Two further tests exercise the public history-label helper with authored name
padding, unknown labels, counts, name collisions and once-only nested rendering.
Two review regressions cover locale-sensitive exact/category selection through
explicit and captured values, and nested-category rejection/usable-catalog
filtering with valid nested forms retained. A further producer-bound case covers every catalog identifier and underscored
native limit values through direct/captured rendering and plural parsing. All
nineteen pure cases remain in the
same test module when the complete application test suite is
integrated; the later source-bound extraction and UI tests must also be kept.

Fresh local checks passed 403 UI/model tests with zero failures or skips, the
production build, the strict 172-page guide and eight publication tests. One
existing UI case compiles a temporary pure Rust helper; it is separate from
application-native qualification. All 1,353 existing tracked base files are
byte-identical, including native sources, configuration and literal test inputs.
That main tree equals the earlier `403bf9a` source qualified by 992 Rust tests
with six explicit ignores and all-target Clippy. Those results are reused for
identical native inputs; no new Cargo or application-native run was performed.

This dependency PR requires its own exact-head Codex code approval, trusted
validation and normal merge.
The final integration must preserve every original `ed9beee` test and the two
additional helper tests, both plural corrections and the consistent underscore
placeholder grammar with its producer-bound regression.
The additional source-bound controls cover numeric audio/limit roles and 23
canonical anonymous/count frames. Pattern captures and recursive rendering use
the same numeric-slot classification. The web engine's exact movement frame
accepts its signed `n` distance; other numeric slots retain the unsigned grammar.
Source-owned utility fragments use the two actual label/effect/applies records,
tag-kind conjunctions use their four ordered flag combinations, and the four
picture-label frames accept the 21 known labels, the helper's `Cover` fallback
and canonical `Undefined(0)` through `Undefined(255)` identifiers. Known labels
still translate; technical undefined identifiers remain raw even with a colliding
custom key. Authored filenames retain their unrestricted Select frame.

These constraints follow current producers, rather than inferring a schema from
arbitrary text. The tests call the real utility, tag-prompt, cover-label and
File List helpers. The native picture boundary is its Debug formatter and the
locked Lofty 0.25.4 checksum
`bb21dd79145b0781f2423fda92dd807fcd3911e44b66044d86db3b11ec7accf9`.
The audited upstream `src/picture/mod.rs` SHA-256 is
`01973be38bbb8feea75cbe9ac128e547af92ec7160b393bb82a8a38bc0f57d36`;
UI tests use repository inputs and do not require an installed Cargo registry.

Two further regressions require `other` in every plural block, including nested
and exact-only blocks, and reject zero-padded or number-rounded exact selectors that cannot match
the renderer. Exact selectors must round-trip through the renderer's JavaScript
number representation; representable large integers remain supported. Canonical nonnegative selectors such as `=0`, `=1` and `=10`
remain valid, as do the six plural categories. Invalid entries are removed once
per key, reported in the file warning and rendered using English fallback.
Duplicate selectors retain the existing first-match behavior; this change does
not add a broader ICU grammar.

The complete extractor and source-key idempotence
checks belong to the activation layer, where their actual producers exist.


The four standalone copy-name frames are retained for explicit parts rendering but
are not inferred from arbitrary rendered text: their actual action, column,
helper and export producers create persisted names. The generic failure frame accepts only
the two operations used by the real staging call sites, Cut tags and Paste tags.
A producer-bound regression executes the actual duplication/staging helpers in
German, Dutch and collision-prone custom English and preserves raw names.

A finite adjacent audit enumerated short unconstrained candidates and traced
the bare single-word suffix family. Field `value`/`values` accessible labels
are actual UI scaffolds with arbitrary field identities protected by their
existing data-verbatim marker; the picture suffix already has its bounded
producer guard. Those supported UI frames remain available. Bare `Copy of Family` can still
match the genuine `Copy {name}` field-command frame with `of Family` as its
name; actual persisted names use their existing protected consumers, not that
unmarked lookup. The catalogs,
application sources remain unchanged. The original Copy-of library assertion
now checks explicit parts and raw standalone preservation in its small catalog;
no test or catalog key is removed.
