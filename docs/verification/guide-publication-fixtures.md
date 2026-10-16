# Native fixture isolation for guide-publishing validation

PR #34's reviewed head `e5439480a24839ffafafd17749742f7c71e7d866`
failed the Linux native job in trusted run `38946158621`, job `117125904733`.
The saved `sources-4.txt` contains only `The fixture answer server did not start`;
the provider app suite did not launch. The other 25 native suites completed.
Artifact `12066397182` contains those suite logs. The macOS job
`117125904955` failed with the same source-4 fixture-start message, with no
other native-suite failure in artifact `12067651447`. There, GNUdb started
at 18:22:26 UTC and sources-4 started at 18:24:41 UTC, also before the
150-second GNUdb lifetime ended.

GNUdb started its detached 150-second server at 18:22:04 UTC. The provider
fixture started at 18:23:14 UTC while that server was still alive. Both bound
`127.0.0.1:18766`. The original fixture launcher discarded the background
server's stderr, so the CI log does not contain a bind exception. A separate
fixture-only reproduction using the actual Python handler bodies reproduced
`Address already in use` and the GNUdb server's 404 for the provider readiness
route. That reproduction remapped only the shared port to an owned ephemeral
port and opened no application window.

The fix assigns the GNUdb test server and its UI test client port 18767.
The built-in provider fixture and application routing remain on port 18766.
No application behavior changes.

Validation executed both actual fixture shell scripts together on ports
18767 and 18766 in a temporary directory. An owned Python launcher recorded
child PIDs and retained their stderr. Both scripts returned success with empty
server stderr. While both servers were alive, GNUdb accepted its policy-valid
query/read sequence, and the provider answered iTunes, Deezer and LRCLIB
requests. Each rejected the other server's readiness route. Both owned child
processes were terminated and both ports stopped listening.

`test_fixture_servers.py` executes the actual Python handler bodies concurrently
without FFmpeg or application windows. It fails on the former shared-port setup
with the reproduced bind exception and passes with isolated ports, including
real CDDB query/read and iTunes HTTP responses. It does not compare hardcoded
port literals. JavaScript and shell syntax checks also pass.

These local checks establish fixture isolation and protocol responses. They do
not establish native provider UI success on Linux, macOS or Windows. The new
final head still requires a clean Codex review and a fresh trusted validation
run that executes those native suites before merge.

## Windows profile cleanup follow-up

The isolated-port head `33188e5ff089446bee1b78336ef48bb938dd2146`
executed GNUdb's 22 assertions and the provider suite's 40 assertions successfully
on Linux, macOS and Windows in run `38948648968`, test merge
`0fe00c089369cefee5dfc0f92e0ff2438f8f4bd2`. Each saved suite log ends with
`SUMMARY done` and has no failed assertion. This is offline fixture evidence,
not live-service validation.

Windows job `117134332084` still failed before launching four later suites:
`keyboard-ui`, `launch-forwarding`, `export-lists` and `chapters`. Artifact
`12068491046` shows only `rm: cannot remove ... Device or resource busy`
for the disposable test identifier's `EBWebView` profile files in those four
logs; they contain no application assertion or summary. The other 22 suites
completed. No retry or application change was used to conceal that failure.

The existing pending translation branch at
`bf1114a73149d4e24866506eb631c1bc4c6eeeaa` already has a cleanup fix. This
follow-up brings over its fifteen cleanup attempts, two seconds apart, and
stops leftover WebView2 helpers only when both `GITHUB_ACTIONS=true` and
`RUNNER_ENVIRONMENT=github-hosted`. The existing test-identifier restriction
also applies. A GitHub-hosted job owns its disposable VM; local and self-hosted
runs never issue that process-termination command. Their file cleanup receives
the longer retry window without terminating other applications.

A portable regression executes the actual script's cleanup block with fake
`taskkill`, `rm`, `sleep` and path commands. It verifies termination precedes
cleanup only for the hosted case, rejects generic-CI and self-hosted termination,
leaves a production identifier untouched, and tolerates seven simulated busy
attempts before success. It fails against both the old five-attempt block and
the translation branch's broader generic-CI termination condition. No real
Windows application or process was launched, terminated or removed locally.

The 186 successful provider/GNUdb assertions above belong to the historical
`33188e5` head. The cleanup follow-up requires its own final-head Codex review
and fresh trusted cross-platform CI before merge.

## Provider script completion follow-up

The next head `d53f93067899444f646cfe3bae85185effccd98b` again completed
all 62 GNUdb/provider assertions on each desktop platform in run `38952366774`,
test merge `93c23118f6c854d8f7a30a8e9803fd9cff3b9691`. Linux and macOS
completed all 26 native suites; Windows completed 25, with only `keyboard-ui`
blocked by a profile lock before app launch, after its thirty-second cleanup
window. The existing suppressed `taskkill` output does not establish which
process held the files or whether that command matched it.

A source inventory then found the concrete omission: `sources-4.js` was the
only one of the 26 suite entry scripts to report `SUMMARY` without invoking
`e2e_exit`. That native hook calls `app.exit(code)` when scripted testing is
enabled. On Windows, the source-4 group occupied about 193 seconds, consistent
with fixture preparation and the harness's 180-second timeout. The harness's
existing pipeline suppresses exit status; the earlier assertions and summaries
are evidence of those checks, not evidence of orderly application shutdown.

The fix adds the same awaited exit hook as the other suites, immediately after
the summary. It preserves every provider assertion and the existing cleanup
bounds. A behavior-level Node VM control executes the actual script, advances
its existing wait loop into a guarded failure without a DOM or app, and checks
that failure reporting is followed by `SUMMARY done` and then `e2e_exit(0)`.
It fails against the old script and passes with the exit call. The native
harness still fails a suite that reports a failed assertion.

Per-invocation WebView user-data-folder isolation was investigated against
Microsoft and Wry documentation, but was not implemented. The confirmed missing
exit is addressed first. No application Rust, feature behavior, signing code or
fixture port changes are included in this follow-up. Its final head still
requires a fresh Codex review and full trusted CI; previous native assertions
are not relabeled as results for this head.
