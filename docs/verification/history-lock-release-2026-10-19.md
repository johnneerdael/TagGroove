# History-store lock release — 2026-10-19

## Failure and attribution

[Linux job 118289998685](https://github.com/johnneerdael/TagGroove/actions/runs/39318735158/job/118289998685)
on `beb6e78d673be69d80d2d610d07f49ebbb54f0d7` passed 41 durable-history
cases but failed `a_second_writer_cannot_open_the_same_durable_store` at its
immediate reopen after dropping the first journal. Its initial concurrent-writer
refusal passed. That assertion discarded the reopen error, so the natural CI
log does not prove lock contention. The assertion now reports the actual error.

The test binary also spawns a child in another case. On Unix, a duplicated or
inherited descriptor can retain the same open-file-description lock after the
original descriptor closes. The [Linux flock specification](https://man7.org/linux/man-pages/man2/flock.2.html)
describes shared ownership through `dup`/`fork` and explicit unlock. This provides
a plausible CI interleaving, not an observed trace of that particular failure.

## Deterministic native regression

The new Unix unit test uses a fresh real store and a real duplicated lock handle;
it does not override directory identities or simulate the locking API:

1. Open the first journal and duplicate its lock descriptor.
2. Require a second independently opened journal to refuse concurrent ownership.
3. Drop the first journal while keeping the duplicate alive.
4. Require a new journal to open immediately, without retries or sleeps.
5. Close the stale duplicate and require the new journal to keep excluding writers.
6. Drop the new journal and require another open to succeed.

Before repair, step 4 fails with `Undo history is already open` and an actual
would-block error. The failing executable, logs and temporary store are retained.
After repair, the complete sequence passes. This demonstrates the retained-handle
mechanism on the authoring Mac; it does not establish the exact Linux CI schedule.

## Repair and ownership boundary

`StoreLock` wraps the file immediately after successful lock acquisition, before
the fallible metadata-stamp read. Failed acquisition never constructs this owner
and cannot unlock another journal. Its destructor explicitly unlocks, then lets
the contained file close normally; teardown does not panic on an unlock error.

The lock remains the last declared field of `Journal`. Watcher cleanup and other
journal resources therefore finish dropping before ownership is released.
Explicit unlock is not performed early in `Journal::drop`. Index version,
snapshot accounting, operation limits and free-space reserve are unchanged.
The user guide explains ownership release and retains the warning that operations
performed without snapshots do not acquire Undo afterward.

An independent read-only reviewer checked the failure mechanism, acquisition
errors, field-drop ordering, concurrency and Windows API compatibility. Final
rereview found no issues. This is separate from GitHub Codex code approval.

## Focused verification and retained artifacts

All roots below are under the authoring Mac temporary directory:
`/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/`.

| Check | Retained root | Result |
| --- | --- | --- |
| Controlled RED build/runtime | `taggroove-store-lock-release-red-build-zke23rm3`, `taggroove-store-lock-release-red-runtime-fp8ya5bb` | One regression fails with would-block; fixture `taggroove-store-lock-release-TMCSh3` retained. |
| History unit cohort | `taggroove-store-lock-release-green-build-q2jqrdwy`, `taggroove-store-lock-release-green-history-runtime-4w77plk8` | 15 passed, 0 failed, 201 filtered out, including the new regression. |
| Durable-history integration | `taggroove-store-lock-release-durable-build-928moo2y`, `taggroove-store-lock-release-durable-runtime-rn5lzjqy` | 42 passed, 0 failed, including the original writer-release case. |
| Scoped strict Clippy | `taggroove-store-lock-release-focused-clippy-_g2qj5rb` | Library plus durable-history test target, `-D warnings`, exit 0. |
| Strict user guide | `taggroove-store-lock-release-strict-guide-yryoq0ao` | Exit 0 after the guide clarification. |

Retained executable SHA-256 values:

- RED unit: `7b39591a57148868cc4e03fd18521c277d7966db17940c93e2477aa21f6d8dfb`.
- GREEN unit: `d9a3860f342f07313676dff40a5c0a44be38a0b187b6b3c43254fd43261afdb4`.
- GREEN integration: `e3ef53270d7a84f12254989a71bd338c6e33cb7a2f18b44e7f1b1fdc2afcc505`.

Each runtime's 141 core source/fixture bindings still match the inspected files.
The guide was edited and strictly built separately. Commands, compiler artifacts,
stdout/stderr, input states and outcomes are retained. No local native UI or full
workspace sweep was run for this repair. Native Windows/Linux CI and the exact
final-head GitHub Codex review remain required before merge; local tests do not
qualify installed, network-filesystem or original-application behavior.
