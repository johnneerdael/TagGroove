# Network access, privacy and rate limits

What leaves your computer when you use a tag source, where API keys are
stored, and why a lookup can be slow.

Online searches need a network connection. They send search terms and
identifiers to providers; per-file lyrics lookups send track metadata and
duration. AcoustID sends a locally computed fingerprint and duration, not
the audio file. **Preview** also contacts the source in your browser.
There is no downloaded offline mirror of these online catalogs. A configured
local freedb folder can supply disc metadata without a server lookup.

Personal API keys live in `tag-sources.json` in TagGroove's configuration folder,
separately from ordinary Options. They are text with restricted file permissions
where supported, not encrypted by TagGroove. Keep this file out of shared
configuration copies. Discogs also has an optional system keychain location.
Built-in application keys are part of the installed binary; they cannot keep
an individual's secret confidential from someone inspecting that binary.

TagGroove avoids putting credentials in displayed errors. Authentication still
sends the required credential to its provider over HTTPS; Last.fm uses a
request URL parameter. Consult provider terms before redistributing data or
artwork.

Requests are spaced separately by service. iTunes pauses several seconds
between requests; many per-file lookups or album pages take longer. Correct
missing-key errors, and wait before retrying quota or server errors. Starting
a newer source request supersedes older work without resetting provider quotas.
