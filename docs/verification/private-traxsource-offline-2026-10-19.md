# Private Traxsource offline album parser — 2026-10-19

This PR64 qualification extends the [bundle admission checks](source-update-recovery-2026-10-18.md#current-private-bundle-admission)
to actual execution of one private Traxsource **Release Direct by ID** album
parser. Its earlier HTTP 403 stopped acquisition before parsing; that response
did not prove a parser defect. No retained provider response was available.

## Executed contract

The existing ignored `sources::web::compat::community_script` test ran the real
loader, includes, settings, interpreter and release conversion against manually
invented saved HTML. The album-only route skipped index acquisition. The entire
backend process ran under OS-enforced network denial; a separate localhost
socket control returned `EPERM`. Debug output was disabled.

The invented fixture and hard-coded oracle described a fictional release with
album, album artist, publisher, catalog, date, numeric source ID and canonical
source URL, plus an artwork URL under `fixture.invalid`. Isolated copied settings
explicitly selected four-digit YEAR, ISO RELEASETIME and no generated COMMENT.
This is a controlled setting combination, not a default-settings claim.

| Case | Required and observed result |
| --- | --- |
| Base fictional page | Exactly eight fields, one artwork URL and zero tracks. |
| Independently changed title/date | Exactly ALBUM, YEAR and RELEASETIME change; other fields and artwork stay equal. |
| Deliberately wrong ALBUM oracle | Verifier exits 3 for ALBUM alone while the backend exits 0. |

The verifier requires an actual single test plus SOURCE/RELEASE output, rejects
the HTTP path and compares the complete field/artwork/track inventory. An
intermediate invocation without inputs and the first incomplete markup draft
are excluded from passing evidence. No production change was inferred from
fixture drafting.

The root independently reran all three final cases from a separate private
runner/fixture copy and observed the same two passes and intended oracle failure.
All 16 original runtime files and two retained ZIPs remained byte-identical.
No debug folder was created and no source script or response body is redistributed.

## Provenance and limits

Execution used retained backend test binary SHA-256
`50b5fb6ce6f83c272ada279e6331d4ee0eb14683f038c0da6f816446cf10f05c`,
from the complete 552-pass/14-ignore backend run at rename implementation
`73d092ce05a2977c1621a91f2a2366b9209a1da4`. Nine current command/settings/parser
source files were independently checked byte-for-byte against that commit.
Private receipts bind the executable, scripts/ZIPs, source files, sandbox,
fixture, settings, runner and oracle hashes. The author receipt SHA-256 is
`7823bc010c2f9ab9491f87420772fad89d2b29e03caaf71a2c07d0d141b6b2cc`.
This does not qualify a later application binary merely because it shares a branch.

Invented HTML establishes only this named local execution contract. It is not
a recorded provider response, current provider compatibility, original Mp3tag
comparison, index/search or track parsing, artwork acquisition, native review,
selected apply, audio save/reload or Undo proof. The [joined fictional-source
replay](source-workflow-replay-2026-10-18.md) separately covers its own production
review/journal path. Combining these receipts does not establish a private-source
end-to-end journey. L30/N01/N02/L78/L79 remain open; Windows work is deferred.

Provider-response qualification still needs a legitimately retained response
body with acquisition URL/date and provenance. Original/runtime, native and
live-provider receipts remain separate requirements.

The [SoundCloud scalar-parser check](private-soundcloud-offline-2026-10-19.md)
separately executes that private bundle against invented hydration input.

A later [joined private replay](private-source-joined-replay-2026-10-19.md)
executes the actual parser, production DOM review and internal physical writer
together on invented input. Its provider/public-ingress/native limits remain explicit.
