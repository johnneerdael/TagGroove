# Native source download refusal and recovery — 2026-10-20

The first lean PR64 validation [39331121637](https://github.com/johnneerdael/TagGroove/actions/runs/39331121637/job/118329463401) authorized head `c1b905e190198c9f5e71738eda7c51fb8a537c3c` against main `036716d3e7e40f21369f602c2a0c52d4ab16fe4e` and immutable merge `a89b6874a5d404f2a8a0f85ff539a2d98619f93e`. TypeScript and formatting passed. Rust test compilation stopped in native source acquisition with `SHA-256 mismatch for tmpk8yz7ebf`; runtime tests did not execute. The run is terminal failure, not a live matrix to wait for.

The failing temporary payload was discarded, and the old message omitted the archive and actual hash. The record therefore cannot establish which source differed or the exact transport/server cause. Fresh HTTPS downloads of all four official source URLs match the unchanged pins and the retained offline originals: Chromaprint 1.6.2, FFmpeg 9.0.2, zlib 1.3.2 and bzip2 1.0.8. The raw log and bounded download results are retained under `/tmp/taggroove-pr64-c1b905e-linux-118329463401.log` and `taggroove-native-source-download-check-5t6j4912`.

## Repair and invariant

The [source builder](../../scripts/audio-deps/build.py) now reports the archive name plus expected and actual SHA-256 values. Only a newly downloaded checksum mismatch receives up to three attempts, with a short bounded backoff. Each attempt must match the same existing pin before atomic cache publication or extraction. All temporary candidates are removed. Persistent wrong bytes fail closed; existing-cache tampering, explicit offline failures and HTTP/network errors do not use this retry path. No source URL, source hash, decoder selection, compiler flag, runtime dependency or application code was changed. The source-builder input digest changes with this tooling edit and remains recorded in each build manifest.

The reproduced HTTP fixtures model a wrong response followed by verified bytes, persistent wrong responses and a tampered existing cache. They do not reconstruct the discarded CI response or prove its precise cause.

## Focused verification

- Baseline: 19 source-builder tests passed and two new recovery/diagnostic tests failed.
- Corrected source-builder tests: 21 passed. The real loopback HTTP cases verify successful recovery, a three-attempt limit, archive/digest diagnostics, exact accepted bytes, extraction and temporary-file cleanup. Tampered cache and offline refusal controls remain enforced.
- CI tooling: 126 passed.
- Release tooling: 28 passed.
- Actionlint and whitespace checks passed. Independent read-only review has no concrete findings.

Logs are retained as `/tmp/taggroove-native-source-download-{red,green,tooling,release}.log`. No local application build, full Rust workspace or native UI sweep was run for this Python build-tool repair. The owner authorizes direct-main workflow/build-tool fixes with an inspected-head lease. The repair itself is not a PR approval or a published release; PR64 must sync the new main, obtain its new exact-head Codex review and pass immutable validation.

## Windows postmerge refusal and declared mirror recovery

PR66's postmerge [run 39348564990](https://github.com/johnneerdael/TagGroove/actions/runs/39348564990)
fails in the Windows native-integration app build, before any Windows UI suite
runs. [Job 118387234377](https://github.com/johnneerdael/TagGroove/actions/runs/39348564990/job/118387234377)
reports zlib 1.3.2 expected SHA-256
`bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16`,
but receives
`cb3b5421ce69faff276320aedcf1bc07103927496d26dd5ba710f09f2928af4e`.
The rejected bytes were discarded, so their precise content and transport cause
remain unknown. The full failure log is retained as
`/tmp/taggroove-pr66-windows-native-118387234377-plain.log`.

Fresh HTTPS requests with the builder's User-Agent to both the existing primary
URL and the official [zlib archive](https://zlib.net/fossils/) return identical
1,502,830-byte payloads matching the unchanged pin. Retained capture:
`taggroove-zlib-source-check-aapt5tah`, including response metadata and hashes.
That local observation does not reconstruct the failed Windows response.

The follow-up repair adds the official fossils URL as zlib's declared mirror.
New downloads send `Cache-Control: no-cache`; checksum mismatches and HTTP
failures can rotate to a declared mirror within the same **three total
attempts**, rather than repeating only one endpoint. Every accepted archive
must still match the original pin before atomic cache publication or extraction.
No-mirror HTTP failures, tampered existing caches and explicit offline failures
retain their previous refusal behavior. Source version, hash, license and
runtime decoder selection are unchanged. The recipe and source manifest bind
the changed source inventory and build script.

The new loopback regressions first fail against the old builder: it repeats
the bad primary, fails on its HTTP error and never contacts the declared
mirror. The repaired 23-test source-builder cohort passes, including recovery
from mismatched bytes and HTTP 503, a three-attempt all-bad mirror bound,
unchanged-pin enforcement, temporary cleanup and existing cache/offline controls.
Independent static review has no findings. These modeled responses are not a
new Windows build result; the repaired main build must supply that evidence.

The owner-authorized repair is pushed directly to main with an inspected-head
lease and `CI-Repair-Run: 39348564990`. The source fix does not wait for the
remaining failed-run matrix. Release authorization still requires the referenced
main run to become a completed failure; publication and native-integration gates
remain enforced.
