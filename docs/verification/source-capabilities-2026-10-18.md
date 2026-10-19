# Source installation capabilities and full-cover routing — 2026-10-18

## Installation and include updates

The [installer](../../src-tauri/src/sources/web/installer.rs) now compiles
resolved index and album parser blocks before replacing the active source
folder or its recovery snapshot. The existing compiler supplies include-file,
section, line and command diagnostics. An include-only update checks sources
that depended on the previous working include; a pre-existing incompatible
source does not block an unrelated valid installation.

Minimum-version declarations remain informational. Supported commands can
install despite a higher declared Mp3tag minimum; unsupported commands are
refused despite a lower declaration. No comparison with TagGroove's unrelated
version or complete version-to-feature registry is claimed.

Real-filesystem RED tests showed incoming parser errors and a breaking
include-only update were accepted before the change. The installer suite then
passed 21 cases, with three explicitly ignored platform/private-input cases.
Controls preserve active and previous-folder bytes on refusal, accept a high
minimum with supported commands and preserve an unrelated old incompatible
source.

Four additional runs on disposable copies/flat ZIPs of the supplied private
bundles passed: Traxsource has eight sources, one settings schema and 14 parser
blocks; SoundCloud has one source, one schema and two parser blocks. All
16 original runtime files retained their recorded hashes. Proprietary source
content and responses are not added to this repository.

These runs used OS-enforced network denial. A controlled localhost connection
failed with permission error before the tests; no provider request was made.
Offline Cargo resolution alone would not have prevented interpreter fallback
requests. Recorded index/release response bodies were not available, so no
fresh private-provider response parsing or native tagging journey is claimed.

## Separate full-size cover action

The [web result mapping](../../src-tauri/src/sources/web/mod.rs) retains a
full-size cover URL independently from the thumbnail, index Preview address
and parser/release URL. The optional DTO field preserves older result shapes.
Only HTTP(S) addresses without embedded credentials are accepted, and the
address joins the existing source-session open-page allowlist.

The [query-result controls](../../src/tag-sources.ts) show **Full cover** only
when that address is valid. A user click opens exactly that image address in
the browser. Thumbnail loading, **Preview**, **Next** and tag/artwork apply
retain their separate routes. There is no automatic full-image request or
tag write when the result is displayed.

RED regressions established the dropped result field and absent control.
GREEN checks passed six focused backend cases, two focused control cases and
157 web/source/i18n DOM tests, plus TypeScript, formatting and whitespace checks.
They use independently authored fixtures and actual source/UI functions with
recorded host calls; they do not exercise a native browser or live provider.

The supplied bundles do not declare the separate full-cover/index-preview
fields. They therefore cannot qualify these controls by themselves.
The original album-parser preview output key and precedence remain unproven
and unchanged. N03 is narrowed only for this documented full-cover slice;
L30/L32 and N01–N03 retain their remaining version/capability, provider,
installed/native, album-preview and end-to-end acceptance requirements.
