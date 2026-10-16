# Web-source compatibility on reviewed main

This PR20 candidate integrates with actual main
`21de24598ac0170faa5be7f4e408389c69af30ac`. GNUdb, the additional online
providers and neutral guide publishing are already part of that baseline.
The remaining feature delta is scripting/source compatibility and its guide
examples; PR17 localization and typed diagnostic integration are not included.

## Resulting behavior

- Load source archives from bounded subfolders, resolve include/settings names
  without case sensitivity, and accept consecutive SearchBy triplets.
- Preserve balanced literal parentheses, dollar anchors and numeric captures in
  format-function arguments.
- Read the first JSON value with ordered object members, preserve the text
  cursor when leaving JSON mode, and tolerate unselecting at the root.
- Correct current-line replacement, search and output-buffer behavior, including
  TRACKS/default OUTPUT conventions and plain _LENGTH milliseconds.
- Decode the complete semicolon-terminated named HTML reference set and Unicode
  escapes. Preserve complete direct lookup and preview URLs in their narrowly
  identified complete-address contexts.
- Teach those behaviors in the format/regex reference and three web-source
  guides. Existing file-write and history safeguards remain the baseline.

## Local qualification and final integration

The fresh native qualification at `3aab350a45ce6ff3966857ebf94d3ae15d634758`
rebuilt all five workspace packages in its isolated target. It passed 973 Rust
tests with six explicit ignores, full-workspace/all-target Clippy with warnings
denied, the production TypeScript/Vite build, publication tests and the strict
172-page guide build. The original low-disk Undo failure and successful rerun
are retained separately; no application change was made for that environment
failure. The later fixture/cleanup and provider-exit changes affect the test
harness; the 606 recorded native/build input hashes remain unchanged.

The subsequent `dae5e69ae84fd646f239a15e6b8a597672b6c20f` checkpoint freshly
passed all 384 UI/model tests, including the actual provider-script exit
regression. Its unchanged review-policy/harness inputs retain 84 passing
Python checks. The final merge of actual main adds only the corrected publishing
recovery instructions; this verification note is updated to identify that base.
Production code, dependency files, native/build inputs and all 172 reader pages
remain byte-identical to those qualified checkpoints. Counts above retain their
stated source/run scopes; they are not new full native executions on this head.

The ignores are the system-trash probe, benchmark, three live-service checks
and optional downloaded-script corpus. These local results do not establish
live API availability, native desktop replay, mobile source behavior or
compatibility with every downloaded community script. Exact final-head GitHub
Codex code review and trusted validation against current main are still required
at this local checkpoint. Normal merge, release and live Pages verification are
separate outcomes.

## Review corrections retained in the feature

The initial handwritten character-reference subset left valid names such as
`&Alpha;` untouched. The importer now uses the complete `web_atoms` 0.2.6 table
already present in the lockfile, with no dependency-version changes. An
independent comparison with the primary WHATWG
[entity data](https://html.spec.whatwg.org/entities.json) matched all 2,231
entries, including 2,125 semicolon-terminated names and 93 two-scalar
expansions. Decoding requires the terminating semicolon, is case-sensitive and
performs one pass; unknown references remain literal. Actual regressions cover
Greek, non-BMP, multi-scalar, long-name, numeric, unknown, case and single-pass
values.

The complete-address correction is limited to a direct source with one bare
`%s` field and no album base, and its exact-`%s` preview. It preserves queries,
fragments and existing escapes. Nested preview templates, ordinary search words,
album-ID templates and chained fields retain normal word encoding. The caller
still parses addresses and enforces HTTP(S) without embedded credentials.
Actual regressions cover both preservation and rejection/unchanged contexts.

The source-definition guide tests use the production parser as source.
Tag-core's dev-only serde_json dependency explicitly enables preserve_order,
matching the application and full-workspace feature context. No production
dependency version or lockfile entry was changed for that test feature.
