# Private SoundCloud offline scalar parser — 2026-10-19

This PR64 qualification executes the actual private SoundCloud album/parser route
with invented saved HTML containing hydration JSON. It complements the separate
[Traxsource offline check](private-traxsource-offline-2026-10-19.md). Neither
fixture is a retained provider response.

## Executed contract

The existing ignored compatibility test uses the real loader, copied settings,
interpreter and release conversion. The fixture contains eight empty decoy
objects and a ninth fictional sound record at the location selected by this
script. Its title, username, release date and artwork are independently specified;
a different 1999 creation date and avatar URL serve as decoys. Isolated settings
enable the named fields/artwork and explicitly select four-digit YEAR.

| Case | Required and observed result |
| --- | --- |
| Base fictional page | Exactly TITLE, ARTIST, YEAR and RELEASETIME; one primary artwork URL; empty release album title; zero explicit track rows. |
| Title-only change | Only TITLE changes. |
| Release-date-only change | Only YEAR and RELEASETIME change; the older creation-date decoy remains unused. |
| Deliberately wrong TITLE oracle | Verifier exits 3 for TITLE alone while the backend exits 0. |

The script does not consume the supplied fictional duration of 185,000 ms or
emit track-length rows. This proves neither duration support nor a parsed
`SourceTrack`; the useful outputs in this route are scalar fields. The oracle
also checks the release header and rejects missing/skipped parser execution.

The root independently reran all four cases from a separate private fixture/
runner copy and observed three exact passes plus the intended wrong-oracle
failure. The complete backend process had OS-enforced network denial, checked
separately with a localhost operation returning `EPERM`. No HTTP path ran and
debug stayed disabled. All 16 original runtime files and two ZIPs stayed
byte-identical. Source text, raw fixtures and configured private files are not
redistributed.

## Provenance and limits

The same retained backend and nine-file source binding described in the
[Traxsource provenance](private-traxsource-offline-2026-10-19.md#provenance-and-limits)
were checked before and after execution. The private author receipt SHA-256 is
`220c972000eb7636c11209abf868c6f67901293489ab6e697d9ca7bf64237372`.
Private receipts additionally bind settings, fixture, oracle, runner, sandbox
and logs. No compilation or production source change was needed for this proof.

These are named local invented-input results. They do not establish current
SoundCloud HTML layout, recorded provider responses, authentication or request
behavior, artwork acquisition, search/index routes, original Mp3tag equivalence,
native review, selected apply, physical save/reload or Undo. No claim about a
later application binary follows from sharing a branch. L30/N01/N02/L78/L79
remain open, and Windows work is deferred. Provider-response qualification still
needs a legitimately retained response with acquisition URL/date and provenance.
