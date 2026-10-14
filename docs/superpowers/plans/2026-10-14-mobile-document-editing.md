# Mobile Document Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Open provider audio documents on Android, perform existing Rust tag and cover edits, verify save-back and retain/export recovery copies.

**Architecture:** Stream provider bytes into private persistent working copies.
Persist pending state before provider publication and verify source/destination
hashes. Route existing basic mutation commands through that engine and expose
access/recovery state in the existing UI.

**Tech Stack:** Tauri 2, Rust, Kotlin SAF, TypeScript, current dependency locks.

**Spec:** `docs/superpowers/specs/2026-10-14-mobile-document-editing.md`

## Global Constraints

- Preserve the full parity objective; provider actions/renames and iOS remain required later work.
- Keep desktop path behavior and existing serializers compatible; no core tag schema change.
- No broad storage permission, URI-to-path guessing, audio base64 transfer or global security changes.
- Keep source and edited recovery bytes across failure/restart; never automatically roll back unknown provider content.
- Restrict all mutation UAT to generated fixtures; Beatport remains read-only.

## Review Focus

- Outside source changes with equal size/time must fail the SHA check before provider truncation.
- Short writes, close errors and verification errors retain baseline and edited files with durable recovery state.
- Revoked/read-only grants must block publication and leave user drafts available.
- Restart, duplicate URI selection and reload must not discard recovery copies.
- Unsafe display names/private symlinks and cancelled save-copy pickers must never escape the private root or report success.

### Task 1: Native streamed document transport

**Files:** existing document-access Rust and Kotlin bridge; new Kotlin stream helper/tests as needed.
**Interfaces:** snapshot/publish/create_document exactly as specified above.

- [x] Add failing byte-stream/policy tests for changed source, truncation, short writes, destination hash verification and invalid private paths.
- [x] Implement bounded streaming, SHA-256 and exclusive private snapshots; validate expected hashes before provider output opens.
- [x] Implement ACTION_CREATE_DOCUMENT cancellation/completion and correct nullable bridge response.
- [x] Run Rust bridge tests and native JVM tests; build Android plugin.
- [x] Freeze source for integration review; root owns commits.

### Task 2: Durable working-copy engine

**Files:** create `src-tauri/src/documents.rs` with tests; root declares module when ready.
**Interfaces:** public functions and DocumentState in the spec; consume Task1 bridge.

- [x] Add failing portable provider tests using real audio and injected failure/change/cancellation states.
- [x] Implement exclusive UUID workspaces, durable manifests/baselines, checked mutation/publication and restart recovery.
- [x] Implement provider technical-context projection, safe image snapshots and original/edited copy export.
- [x] Run focused engine tests, format and strict clippy; freeze source for root integration.

### Task 3: Command and UI integration

**Files:** root owns `src-tauri/src/lib.rs`; interface worker owns `src/main.ts`, `src/style.css` and document-state helper/tests.
**Interfaces:** pick_document_files/document_states/document_save_copy and existing ScanResult unchanged.

- [x] Test native command routing against synthetic managed documents and reject every unadapted write route before cache mutation.
- [x] Route basic tag/cover/filename-to-tag writes and reload through the engine; resolve image content URIs through private snapshots.
- [x] Route Android Add files to the new picker, display access/recovery state, preserve failed-save drafts and add copy recovery dialog.
- [x] Run focused tests, full workspace/52+ frontend tests, TypeScript/frontend build and strict checks.

### Task 4: Native verification and delivery

- [x] Independently review command coverage, grant/path handling and recovery-state durability.
- [x] Build the tracked Android package on the dedicated test AVD; use generated fixtures and native UI controls. Record exact source and APK identity.
- [x] Exercise picker/cancel, Unicode audio load, save/reload, read-only/copy and recovery behavior in both appearances; record packet/PCM/file receipts.
- [x] Update parity/evidence docs without promoting build-only or desktop-only evidence to mobile qualification.
- [ ] Commit logical changes, open PR, run all desktop CI and appropriate Android build/tests; fix actual failures.

Plan self-review: every spec requirement maps to one task. Remaining provider
path/action and iOS work stays explicitly open. No task may claim full parity.

Delivery checkpoint: PR7 is a draft. Source review and all four hosted jobs passed
at `02510c0`; ordinary latest-build macOS UI checks passed. The additional Android
recheck of the final retained-metadata correction remains unexecuted after the
diagnostic device sequence was stopped. Keep this gap explicit; no overall parity
or mobile release qualification is claimed.
