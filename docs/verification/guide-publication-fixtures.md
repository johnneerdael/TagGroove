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
