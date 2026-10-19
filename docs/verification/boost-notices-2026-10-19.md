# Boost.Regex notice coverage — 2026-10-19

The compiled regex bridge uses the tracked Boost.Regex 1.89.0 headers and declares
`MIT AND BSL-1.0`. Its vendored license and source provenance were already in the
source tree, but Boost was absent from the common installer notices, offline
About notice list and bundle license expression. The source archive builder
includes tracked files; this omission does not show that a released source
archive lacked the license.

PR64 adds the complete existing license and recorded source URL/archive hash to
[common notices](../../THIRD_PARTY_NOTICES.md). The
[offline list](../../src/audio-licenses.ts) imports that same vendored file
verbatim, avoiding a separately maintained copy. The existing About flow
translates the new heading in English/Dutch/German. Bundle metadata includes
`BSL-1.0`; application/dependency versions and build recipes are unchanged.

The existing [installer notice tests](../../tests/installer-notices.test.mjs)
first reproduced the missing entry, then verified all eight imported notices,
Boost version/provenance, binding to the visible list and bundle metadata. Six
focused notice/catalog tests pass. A production build passes; independent
JavaScript AST observation finds the complete exact vendored license text in
the generated bundle. The root repeats the production build and exact-license
AST observation and runs the complete frontend suite: 761 passed, zero failures
or skips. The canonical English catalog has 3,392 keys. Rust sources are
unchanged from the preceding 1,332-pass/15-ignore checkpoint.

These are source, local test and production-build observations. They do not
constitute a complete dependency-license audit, a legal conclusion, inspection
of every released package or reproducible-build qualification. L76 remains
open. Final exact-head GitHub Codex code review/current-main validation and
publication remain separate requirements.
