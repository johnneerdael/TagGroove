# Desktop history startup and retention

This is a preserved pre-repair source audit. C54 supplies the nonfatal startup
warning repair and [C62 now supplies durable desktop history and interruption
recovery](durable-history-2026-10-19.md). The session-only/deletion descriptions
below describe the pinned historical baseline, not the current implementation.
The [C61 content-baseline repair](history-content-baselines-2026-10-19.md) separately
supplies stronger observed-content checks.

## Preserved baseline

This source audit inspected PR64 head `38c0147a0c3ce5a413ae54e17de2351001c291cf`. It did not execute a failing save, repeat the separate [concealed-edit experiment](undo-concealed-external-edit-2026-10-19.md), or change history policy.

The retained audit, source hashes and receipt are in `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-history-open-warning-audit-1fatrkh0`. Receipt SHA-256: `f93c8c14b38073b12ebd8d7419b04ba5e47b7d8b49bef109d74c0426a7e6eef1`.

## Source-confirmed startup warning gap

The [desktop adapter](../../src-tauri/src/history.rs) deliberately allows an operation to complete when Undo capture is unavailable. An initialized [core journal](../../crates/tag-core/src/history.rs) reports budget or snapshot refusal through its existing `cannot be undone` notice. Existing history tests cover that successful-write-plus-notice contract.

Journal initialization has a different path. `journal_work_diagnostic` retains the failed `with_journal(...begin...)` result, executes the work anyway and commits capture only when initialization succeeded. A subsequent `history_state` retries initialization. The frontend's `refreshHistory` catches and ignores that command's error. A persistent open/create failure therefore has no route to the ordinary warning, even if Save succeeds. This is a traced error-propagation gap; no filesystem reproduction was executed in this audit.

A targeted regression can use a scoped, test-only journal-root override and an owned regular sentinel file where a directory is required. Call the real backend Save and history-state functions, verify changed tags with unchanged PCM/opaque chunks/sentinel bytes, and require successful Save with no Undo plus a retrievable warning. Keep successful initialization, ordinary capture refusal, excluded provider operations and later recovery as controls. Do not obstruct the application's real cache directory. This proposed test and correction are not implemented here.

## Why existing snapshots do not supply cross-session Undo

The core `Record`, `Side`, `FileRecord` and capture structures keep paths, fingerprints, operation ordering, directory moves and blob accounting in memory. Disk snapshots retain raw bytes without a restartable operation index. `Journal::open` removes unlocked prior sessions and initializes empty stacks; `Drop` removes the clean-exit session. Retaining anonymous snapshot files alone cannot restore the missing mapping or committed operation state.

The [profile-migration journal](../../src-tauri/src/profile_migration.rs) has durable configuration phases, checksums and rollback receipts, but excludes audio. The [provider-document store](../../src-tauri/src/documents/engine.rs) has separate durable working/backup state and recovery. Desktop Undo excludes those managed document paths. These delivered recovery mechanisms do not implement desktop cross-session history.

L67 therefore retains an actual missing lifecycle: serialize committed history records and owned blob identities, retain/reopen eligible sessions, enforce existing aggregate limits and reject stale baselines. Crash recovery during an uncommitted Save additionally needs durable publication intent. Retention/expiry policy and the stronger L61 content baseline remain separate design decisions. Preserve both the current 2 GiB retained-byte cap and 2 GiB free-space reserve; neither may be lowered to make retention or whole-file rewrites appear cheaper.
