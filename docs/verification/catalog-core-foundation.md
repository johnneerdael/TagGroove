# Catalog/core foundation: local contingency

This layer adds the pure translation and catalog-validation library plus the
complete English, German and Dutch catalog data from
`ed9beee4091268d12cecc961d6faadc9746637cb`. Each catalog contains 2,938
translation keys, with its existing metadata and keyboard-key data preserved.
The library and all three catalog files retain that checkpoint's exact bytes.

The existing application remains at main
`5c49582da4d4e6ba1e63e5fb2f7ce66c1918b0bf`: the language registry, DOM observer,
startup hooks, native catalog commands and Language settings are outside this
foundation. Catalog data is exercised by the pure tests rather than installed
in the application. Existing English application behavior remains the baseline.

Seven existing pure tests cover parsing, placeholders, plural rules, locale
digits, fallback/whitespace behavior and complete German/Dutch key validation.
Two further tests exercise the public history-label helper with authored name
padding, unknown labels, counts, name collisions and once-only nested rendering.
They remain in the same test module when the complete application test suite is
integrated; the later source-bound extraction and UI tests must also be kept.

Fresh local checks passed 393 UI/model tests with zero failures or skips, the
production build, the strict 172-page guide and eight publication tests. One
existing UI case compiles a temporary pure Rust helper; it is separate from
application-native qualification. All 1,353 existing tracked base files are
byte-identical, including native sources, configuration and literal test inputs.
That main tree equals the earlier `403bf9a` source qualified by 992 Rust tests
with six explicit ignores and all-target Clippy. Those results are reused for
identical native inputs; no new Cargo or application-native run was performed.

This is a local contingency checkpoint. Any selected dependency PR still needs
its own exact-head Codex code approval, trusted validation and normal merge.
The final integration must preserve every original `ed9beee` test and the two
additional helper tests. The complete extractor and source-key idempotence
checks belong to the activation layer, where their actual producers exist.
