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
