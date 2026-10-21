# Catalog metadata acquisition

License: GPL-3.0-only. See `COPYING`.

Adapted from selected OneTagger providers vendored by CueRithm:
https://github.com/Marekkon5/onetagger
Base commit: `36523f71f2d9a5947912f3cb930f1a31fcb2e3ee`.
Beatport catalog API adaptation: OneTagger PR526, pinned by CueRithm at
`rosgr100/onetagger@f7adb26a2e0790b0ff751bee04a05c1c20dbd009`.
CueRithm donor revision: `cbc866c3b5bcc395906b5713f64f0dc53b05177c`.
CueRithm donor: `vendor/onetagger/providers/{beatport,traxsource,http}.rs`.

Preserve the upstream GPL notice and license with redistribution. The upstream
authors retain their copyrights.

This crate adapts catalog queries, metadata structures, Traxsource selectors,
challenge detection and the optional FlareSolverr-compatible transport. It adds
complete Traxsource releases, exact track-to-release navigation, application
pagination, bounded async requests and cancellation. It imports no tag writer,
matching engine, UI, audio library, browser cookies or executable hooks.

All fixtures are invented minimal examples. No captured HAR content, credentials
from a browser, or user data is included. The upstream-distributed Beatport public
application credentials are catalog application identifiers, not user credentials.
The host supplies proxy/timeout/TLS/redirect policy and offline endpoint routing.
Solver requests contain only a public catalog URL and timeout; solver use is opt-in.
Challenge state and the Beatport token are retained only in application memory.

Validation uses parsers and loopback HTTP fixtures; it makes no live catalog calls.
