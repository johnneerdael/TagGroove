# Directory recreation and retained history — 2026-10-19

The [Linux reviewed validation job](https://github.com/johnneerdael/TagGroove/actions/runs/39300072099/job/118225654486) failed
`side_parent_identities_follow_owned_recreation_across_restarts`: a later Undo
refused an output folder that a preceding Redo had legitimately recreated.
The failing run is for c169e9ee5f40418822eacf7cbf06e7ec3628bf9f; it remains failed.

## Reproduction and repair

Two deterministic Unix controls preserve actual files, directory/type checks,
durable indexes and Journal reopen. A test-only per-thread provider assigns
swapped native IDs to two generated parent paths, reproducing ID reuse without
requiring APFS to choose a particular allocator sequence. Both controls fail
before repair. The mapping control gets inode 11 instead of 22; the two-output
workflow refuses Redo after updating a pending side incorrectly.

Changing only mapping to simultaneous replacements makes the first control
pass, but the actual two-output workflow still fails on cycle 1 Undo. The current
side's newly observed parent guards were being rebased again. This intermediate
failure is retained separately rather than reported as a complete fix.

The repair represents one recreation's replacements as a batch. Every old guard
consumes that batch once; later recreation batches compose in execution order.
The currently replayed side already contains new guards and is excluded from
that batch. File/directory-move guards still receive the changes they need.
The same ownership rule applies to rollback. No retained index schema, limits,
free-space policy or outside-change verification was weakened.

An independent review found another reachable error path: a failed multi-epoch
Undo can successfully roll back its own created folders, while a later retained
Redo still holds their earlier IDs. The new real-files control captures a
conflicting side file at epoch 0, moves X to Y, creates a cover at epoch 1, then
records and undoes another cover replacement. An outside conflict makes the
older Undo refuse and roll back. Pinned native directory handles ensure the
recreated folders receive distinct real IDs; no ID override is used in this
control. The cover and outside bytes are preserved, but later Redo fails before
repair. Forward and successful rollback batches now survive the error result
and update retained history before checkpointing. The rereview found no further
blocking issue.

The final four focused controls pass, including ordered separate generations,
three two-output Undo/Redo/reopen cycles and outside-folder refusal, and the
rollback/later-Redo sequence. The strict user-guide build passed at the updated source snapshot. The complete final-source Rust workspace passed 1,517 tests across 84
executables, with 21 intentional ignores and no compiled-source binding drift.
Its retained root is taggroove-native-daily-workspace-tests-i769l7nj. Strict
workspace/all-target Clippy passed with `-D warnings`; retained root
`taggroove-history-directory-strict-clippy-p20vyopt`.
These controls do not replace the required fresh Linux/Windows/macOS CI run.

## Retained evidence

Roots are under /var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/.

| Observation | Runtime root / executable SHA-256 |
| --- | --- |
| Two initial semantic failures | taggroove-history-directory-reuse-red-runtime-8rjt41gl; 7043004247b5931566afadb0434a658c86b3bd9d4bf50564ecec9edea76363dc |
| Only simultaneous mapping, still failing later Undo | taggroove-history-directory-reuse-single-map-runtime-x9hi6hdj; aedec5f686d35f61e8a0e9f6fc910244e282dc59c88a8e4691bc92f1b5b06f0a |
| Real rollback/later-Redo semantic failure | taggroove-history-directory-rollback-red-runtime-mzobaj05; 2ccbb74a946304bc0ad0344a087f4777344d8a401e8feff91d223fd96ee46bcb |
| Final four controls | taggroove-history-directory-rollback-green-runtime-kt5sng5o; 9375afa0c3a8bf726888aa4f1c4416b84509c427573176301a8a49a654b92264 |

Failing fixtures taggroove-directory-reuse-O49AwN, H4Uztd and aqivtn retain
inputs, independent byte images and the indexes/snapshots present at failure.
The complete CI log is /tmp/taggroove-pr64-linux-118225654486-failed-plain.log.
Commands, source bindings, complete stdout/stderr and retained executables are
kept beside their build/runtime roots. The scoped review did not run tests;
its assessment is separate from runtime results and GitHub CODE approval.

## Windows legacy-identity fixture

The same run's [Windows job](https://github.com/johnneerdael/TagGroove/actions/runs/39300072099/job/118225654610) failed a fixture's replacement of a path
while its original file handle remained open. The test now closes that handle
before replacement and keeps the original object alive through its retained
hard link. It still checks separate-handle/hard-link identity, distinct new-path
identity and unchanged original-object identity through the old hard link.
Production Windows identity calls are unchanged. Static review is clean;
Windows execution of the new fixture remains required, not locally claimed.

## Source bindings

- `crates/tag-core/src/history.rs`: `ef0221850898f9aadde6d1a505a118e629b7b2f384016bd3ef12c9a6eb5c83dc`
- `crates/tag-core/src/history_store.rs`: `143eebfb324424efb29f38faba2493af62df261ae458904f40ae33c8db9658ce`
- `docs/user-guide/guides/undo-history.md`: `a8ac6b980adbaf716b9f414f40ef026b3c07a83748842337ec7bb1d07850aef4`
