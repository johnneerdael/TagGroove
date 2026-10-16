# freedb and gnudb direct-main integration

This branch adds PR #19's complete gnudb lookup and artwork workflow to main
`1fe6a800ac1096494b33d3b461286ad2d91e1a13`. It does not depend on PR #17's
localization work or PR #18's additional providers. The existing main source
writer, English dialogs, MusicBrainz image sizes and local-database reader
remain the baseline.

## Functional contract

- Every request carries the configured email identity or the source-defined
  developer contact and `OpenTagAudioTagEditor` application/version identity.
- The default endpoint uses gnudb HTTPS. Standard CDDB servers keep layout
  query/read support; gnudb-only artist/album and artwork commands require a
  recognized `gnudb.org` host or subdomain.
- Query/search results authorize the chosen remote entry before a read.
  Typed disc IDs are local-database lookups only.
- CDDB and HTTP rate-limit responses stop and explain the retry-later action;
  the client does not submit or retry those requests automatically.
- Entry cover lines and artwork search feed the shared review and covers-only
  workflow. Cover-size settings use the existing MusicBrainz size policy.
- Local one-file and Windows-range snapshots retain selected-layout, typed-ID
  and indexed artist/album lookup, with cancellation and root-identity checks.
- The native `gnudb` fixture suite enforces handshake/query-before-read,
  text/artwork requests, matching/write behavior and rate-limit messages.
  Offline guards route trusted image traffic to local fixtures and reject
  external requests during scripted offline runs.

The owner reports that freedb access is active. Live behavior and local mock
results are recorded separately below; fixture success is not live validation.

## Fresh direct-main verification

The extracted source passes 912 Rust workspace tests (four explicit ignores),
370 frontend tests, all-target workspace Clippy with warnings denied, the
TypeScript/Vite build, formatting, 60 review-gate tests, 25 release tests,
eight publication tests and a strict 166-page guide build. Workflow lint
passes with only the documented unsupported concurrency queue-key exception.
The four ignores remain the system-trash probe, benchmark and two live-service
checks; no ignored check is counted as a normal passing test.

A separate actual native default-HTTPS gnudb query passed and returned
`data/0e026980`, **Sensitize / Falling Through**, using the source-defined
OpenTag handshake. This confirms that query at the time tested; it does not
establish every album/read/artwork outcome or database completeness.

The real macOS native `gnudb` fixture workflow passed all 22 assertions:
Options email validation, developer/user handshake identities, layout and
artist/album queries, listed-entry reads, two-file tag writes, entry/artsearch
artwork choices, covers-only review, rate-limit handling without retries,
and refusal of unlisted remote entries or typed remote disc IDs. The suite
uses a local policy-enforcing server and copied audio; those results are
separate from the live query. It inspects artwork choices, not live artwork
availability or a cover-image write.

The first native runner stopped before UI assertions because its relative
Cargo target path differed from the CLI bundle location. The signed bundle
was copied within this same isolated worktree, verified byte-identical and
replayed without rebuilding; the completed run passed. No other worktree's
artifacts were used. The accidental generated nested target was removed.

No PR #17 localization/typed-diagnostic runtime, PR #18 added providers,
parser changes, Cargo changes or base-version bumps are included. Actual
main's existing core, credentials, provider DTOs and English transport remain
unchanged. A clean exact-head GitHub Codex code review, trusted CI, merge and
release publication remain pending; local/native success is not their proof.
