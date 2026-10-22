# Network access, privacy and rate limits

What leaves your computer when you use a tag source, where API keys are
stored, and why a lookup can be slow.

Online searches need a network connection. They send search terms and
identifiers to providers; per-file lyrics lookups send track metadata and
duration. AcoustID sends a locally computed fingerprint and duration, not
the audio file. **Preview** also contacts the source in your browser.
There is no downloaded offline mirror of these online catalogs. A configured
local freedb folder can supply disc metadata without a server lookup.

Personal API keys and the Last.fm sign-in live in `tag-sources.json` in
TagGroove's configuration folder, separately from ordinary Options. They are text with restricted file permissions
where supported, not encrypted by TagGroove. Keep this file out of shared
configuration copies. Discogs also has an optional system keychain location.
Built-in application keys are part of the installed binary; they cannot keep
an individual's secret confidential from someone inspecting that binary.

TagGroove avoids putting credentials in displayed errors. Authentication still
sends the required credential to its provider over HTTPS; Last.fm uses request
URL parameters, and a signed-in request adds your Last.fm session key. Consult
provider terms before redistributing data or artwork.

If a site blocks TagGroove with a Cloudflare check, an optional
[Cloudflare solver](cloudflare-solver.md) that you run can load the page instead.
It receives the addresses of the requests sent through it and nothing that
carries a key, token or signature; TagGroove stops contacting that site directly
for the rest of the session.

Requests are spaced separately by service. iTunes pauses several seconds
between requests; many per-file lookups or album pages take longer. Correct
missing-key errors, and wait before retrying quota or server errors. Starting
a newer source request supersedes older work without resetting provider quotas.
