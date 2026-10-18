# Saved configuration and local workspace transfer — 2026-10-18

Scope: the portable-backup implementation in consolidated draft PR64. The owner selected **saved settings and workspace only**: no pending tag/artwork/action-editor values, automatic audio writes or implicit execution of restored definitions. Windows work and original Windows capture remain deferred.

## Resulting contract

New `opentag-configuration` version 2 archives carry supported native JSON, active physical MTE bytes/hash/encoding, allowlisted saved preferences and a versioned local workspace. Workspace v2 includes working directory, manual order, display mode and known source rank; it preserves exact CUE/chapter identities. Legacy window/workspace v1 retains its actual smaller shape and original journal checksum representation.

The public adapter commands are `configuration_transfer_export`, `configuration_transfer_preview` and `configuration_transfer_apply`. They use the **same** PR60 snapshot, journal, expected-content replacement/rollback, startup preference restoration, workspace claim/retry/ack and previous-setup recovery engine. The portable module supplies schemas and plans, not a copied transaction/startup implementation. Old unsafe export/import IPC routes refuse writes and direct callers to these adapters; v1 archives remain supported through the reviewed flow and shared journal.

| Selected component | Outcome |
| --- | --- |
| Native JSON | Replace selected incoming files; preserve absent/unselected destination files. |
| Saved preferences | A selected complete snapshot resets absent known choices, including an empty snapshot. An absent/unselected component preserves destination values. Destination release-announcement bookkeeping remains local. |
| Workspace | Restore every selected identity after exact read validation; do not truncate missing rows. An absent/unselected incoming workspace retains the destination's captured workspace across reload. |
| Explicit settings-only recovery | Deliberately omit the failed captured workspace, rearm the same receipt and disclose zero imported rows/omission. This is distinct from leaving incoming workspace unselected during Load. |
| Physical MTE assets | Preserve raw bytes, including unfinished syntax. A bounded private encoding index binds decoding choices to raw hashes and is journaled with the assets; unrelated target entries survive. |

A changed MTE hash invalidates its old encoding override; ordinary detection then applies. The index is rebuilt from selected asset metadata during transfer rather than copied as an arbitrary portable file. This preserves cross-codepage text and raw bytes without pretending that the destination's system ANSI default was an exported setting. Report-file output encoding is separate.

## Safety and admission

Saved helper/layout publications finish before the transaction before-state is bound. A later refusal publishes no incoming restore state; ordinary saved writes that already completed remain saved. Preview does not discard current drafts. Source bytes/identity, native state, current window, reviewed choices/mappings and resolved workspace carriers are rechecked before Apply. POSIX root mapping is explicit. Unsupported/foreign workspace styles retain bounded read-only source row counts, original roots and path-style metadata while the applied workspace remains unavailable. Review selection must bind the visible settings-only choices before Apply; foreign paths never enter a scanner. Shared shape validation rejects malformed identities, inconsistent ranks/selections, unsupported versions and oversized inventories before any publication.

Native saved-configuration writers share one in-process mutex and refuse writes while publication/recovery has a durable pending receipt. Destination-aware column, panel and action-group exports participate when aimed at managed private state. Pure staging helpers stay unguarded. External filesystem writers are handled by drift/conflict checks; the mutex does not lock unrelated processes.

Backup output protects private configuration/recovery/cache/Undo roots and known input identities, including symlink/hardlink aliases. CUE rows protect both their sheet and the audio resolved through the actual subsong reader. An unreadable CUE refuses before publication instead of guessing. The command does not write audio, invoke a provider or access credential/keychain stores.

Exact workspace reads set `exactRows` on the existing scanner. The shared option helper suppresses automatic chapter expansion while preserving other tag options/mappings and direct virtual-row support. This avoids both losing saved CUE/chapter IDs and expanding a saved physical parent under the destination's chapter-display preference. Ordinary scans retain their default behavior.

## Executed evidence and integration

- [Portable adapter and real-filesystem tests](../../src-tauri/src/configuration_transfer.rs): v1/v2 semantics, actual stored-value groups, raw UTF-16/ANSI MTE data, charset sidecar round trips, component absence/empty snapshots, mappings, source/target drift, exact physical/virtual rows, aliases, credential/audio exclusion and fresh-process acknowledgement/restore.
- [Shared engine tests](../../src-tauri/src/profile_migration.rs): original-layout v1 golden checksum/recovery, snapshot lock, pending-writer refusal/after-ack success, publication failure, interrupted rollback, retained external conflicts and durable start/failed/retry/ack across fresh processes.
- [Export loader tests](../../src-tauri/src/exports.rs): cp1251/cp1252 decoding, unchanged raw-byte save, stale-hash handling, corrupt/conflicting metadata refusal and all retained L44 discovery/removal/retry regressions.
- [Action export worker](../../src-tauri/src/action_host.rs): pending managed-target refusal, shared-lock blocking, post-ack success and ordinary external export availability.
- [Transfer UI tests](../../tests/configuration-transfer.test.mjs), [workspace host](../../tests/configuration-workspace.test.mjs), [common recovery](../../tests/profile-migration.test.mjs) and [production bootstrap](../../tests/profile-startup-bundle.test.mjs): current selection/mapping binding across awaits, draft preservation, storage quota/readback/rollback, pending receipts, exact identities, writer flush/pause/resume, explicit omissions and launch precedence.

The CUE output overwrite and unguarded action-export cases were reproduced on disposable real files before their fixes. Eight deferred selection/mapping regressions exposed stale approvals before revision/fingerprint binding and locked controls; exact-row controls exposed both direction-dependent chapter failures before the scanner mode was added. Native foreign-inventory controls retain original counts/roots and preserve the destination workspace on explicit settings-only apply; malformed/oversized foreign structures refuse without changing source or destination bytes. Existing four integration suites were updated to current production functions/contracts and passed 147 tests; they no longer assert the retired unsafe Load confirmation or automatic fallback after a failed workspace claim.

Final combined local checkpoint: **1,271 Rust tests passed, 11 ignored; 745 frontend tests passed, none failed/skipped**. Production TypeScript/Vite, strict workspace/all-target Clippy, formatting, canonical English catalog, seven documentation-workflow tests and strict MkDocs passed. Source-inventory changes are included. English/Dutch/German cover the new labels; key-specific numeric guards and adversarial translation controls retain count data roles.

The backend focused groups overlap and are not a unique test total. Root combined checks and the final commit/hash receipt supply the current suite totals. Earlier green source snapshots do not prove a later revision. Local tests/reviews are separate from the required exact-final-head GitHub Codex code review, immutable current-main validation, merge and publication.

## Remaining qualification

The saved-only contract is implemented and locally exercised; it is not an original Mp3tag differential or a native installed-app journey. Real picker/keyboard/screen-reader behavior, released desktop restart, real mobile/provider transport/save-back, device behavior and Windows execution remain unqualified. New portable transfer is explicitly desktop-only; no provider grants or referenced executables/source bundles/custom locales are installed by an archive.

Native subprocess/file-backed tests, controlled frontend/Vite tests and original/runtime qualification are distinct evidence layers. File-count and byte bounds are not performance measurements of a 10,000-recording library. Native/browser stores are recoverably coordinated, not one instantaneous operating-system transaction. An interrupted selected restore remains pending until verified preferences, complete workspace reconstruction and durable acknowledgement finish, or the user explicitly chooses a disclosed recovery alternative.
