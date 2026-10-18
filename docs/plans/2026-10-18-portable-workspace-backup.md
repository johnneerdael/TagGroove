# Portable configuration and workspace backup design

Status: implemented in consolidated draft PR64, with bounded local backend/filesystem and production frontend tests recorded in the [implementation receipt](../verification/portable-configuration-2026-10-18.md). The owner confirmed saved settings/workspace only, excluding unsaved tag/artwork/action-editor edits. Windows work remains deferred. Original-runtime, native installed-app input/restart, device and provider qualification remain separate open acceptance work. The source-inspection and design sections below retain their historical baseline `a80172f314fa5912dac08f752678150a9de834f2`; their line numbers and initial “not executed” statements describe the design pass, not current implementation proof.

## Outcome and explicit policy

Extend File → Save/Load configuration to transfer saved native configuration, remaining application browser preferences and the local file-list workspace. Restore the selected components through one recoverable configuration transaction, including after process restart. Never write audio metadata or invoke an action/tool/source as a side effect of saving/loading configuration.

The owner selected **saved configuration plus local workspace references, without unsaved audio/artwork/action-editor drafts**. Saving configuration may run while audio edits are pending: preserve those edits in the current window, but state that they are absent from the backup. Loading requires an explicit replace decision after a validated preview; block while an action editor has unsaved changes and allow Cancel or explicit discard of pending audio/artwork. Do not offer an implicit Save tags branch inside configuration loading. A user can save tags through the normal command and return.

This is a product choice, not a technical impossibility: transferring unsaved tag values would require a separate versioned draft/recovery contract, baseline fingerprints, artwork payloads, conflict review and stale/provider rejection. PR60 does not currently capture those values. Treat such a future draft recovery feature separately; do not silently advertise this backup as an editing-session snapshot. The confirmed choice defines the configuration/audio boundary for this implementation.

## Source-backed starting point

| Frozen receipt | Actual behavior / implication |
| --- | --- |
| `utilities.rs:315-368` | v1 `opentag-configuration` bundle contains seven allowed JSON files only: settings/actions/exports/columns/tag-panel/directories/lists. `collect` parses JSON but does not call every component validator and does not flush renderer state. |
| `utilities.rs:371-403` | v1 restore validates all input files in staging, then atomically replaces files **one at a time**. A later publication failure can leave earlier files replaced; there is no multi-file rollback journal here. Missing bundle files preserve target files. |
| `utilities.rs:429-447` | Native import caps the input at 32 MiB. A settings reload warning/error can be returned after native files were already published. Do not equate IPC failure with unchanged disk. |
| `main.ts:1957-1964` | Save calls native export without flushing helper-list saves. Load shows a broad confirmation, suppresses helper flush errors, pauses helper writes, imports, clears tag drafts and reloads. It does not capture browser preferences/workspace, nor use the full PR60 audio/artwork/action-editor gate. |
| `profile_migration.rs:14-54,58-119` | Existing allowlisted browser state and typed local workspace; 10,000 file identities/roots, 2 MiB window bound, unique/subset checks, local absolute paths and no URI/provider addresses. Browser keys are allowlisted, but values are only strings/overall-size checked in this layer. |
| `profile_migration.rs:249-285,298-313` | Native snapshot already includes the seven JSON files **and physical export tree**, with bounded regular-file/path checks and hashes; decode refuses unmanaged destinations. |
| `profile_migration.rs:644-729,751-877` | Checksummed private journal, validated before/after/rollback snapshots, expected-content replacement and rollback, symlink/path checks, interrupted-operation recovery and external-conflict retention. Reuse these mechanisms. |
| `profile_migration.rs:896-954,1097-1169` | Durable applying/restoring phases and pending browser/workspace nonce with start/failed/retry/ack, plus previous-journal recovery. This is an existing transaction protocol, not a portable public bundle reader. |
| `profile-migration.ts:25-40,61-86` | Allowed browser keys captured explicitly; quota failure attempts preference rollback; native receipt remains recoverable. Startup recovers preferences before constructing configuration-backed services, then claims/restores/acks workspace. |
| `main.ts:538-560` | Existing setup gate includes pending covers and action editor. Workspace restore scans physical local files before clearing the current list, reconstructs insertion order/selection/roots/filter/anchor/playlist, and refuses missing rows without truncation. |
| `main.ts:112-116,2121-2131`; `file-ops.ts:99-113` | Recovery precedes UI construction; recovered workspace suppresses configured startup-folder loading; explicit launch requests then run in order. Preserve this precedence. |
| `exports.rs:382-432,1022-1055` | Physical `.mte` files are authoritative on load; v1 restore intentionally replaces metadata while retaining target physical templates/discovery. Seven JSON files alone are not a faithful transferable export-template library. Preserve v1 semantics; define v2 asset behavior explicitly. |
| `tag-tag-ui.ts:55-65` | `opentag.tag-tag` remembers field/template and is missing from PR60's preference allowlist. Include it in the portable schema and local rollback snapshots. |
| `helper-menu.ts:92-100` | Field-name lists and helper histories already belong to native `lists.json`; do not add a second portable copy of their old browser-storage mirrors. |

Frozen source hashes: `/tmp/taggroove-portable-workspace-source-receipt.json`. Line references in this document refer to that commit; rebase contracts onto the active playlist/order work before implementation.

## Architecture options

1. **Append browser JSON to the current v1 writer/import loop.** Smallest diff, but old readers could silently ignore fields and sequential native writes/browser storage would still lack one recovery contract. Reject this option.
2. **Versioned portable envelope plus the existing PR60 transaction/recovery engine — recommended.** Add a portable bundle reader/writer/preview adapter; factor narrowly reusable snapshot/validation/journal/replacement/window-state operations out of PR60 or expose them through a crate-private API. Both Mp3tag setup migration and portable Load configuration call that one engine. Native/browser storage are recoverably coordinated; neither a second restore loop nor a second startup protocol is introduced.
3. **General session/draft archive.** Add pending tags/artwork/provider authorization/Undo recovery. This exceeds L57a, adds stale-baseline/secret/provider hazards and conflicts with the intended no-audio-write configuration operation. Exclude it from this delivery.

## Versioned bundle contract

Keep `format: "opentag-configuration"`; default new exports use envelope `version: 2`. New readers continue to accept v1 exactly as legacy partial native-file replacement, including preservation of absent files/target MTE discovery. Old readers already reject version 2 explicitly; do not camouflage v2 as version 1. A legacy-v1 export mode is unnecessary unless an actual older-reader workflow requires one.

A concrete v2 shape:

```json
{
  "format": "opentag-configuration",
  "version": 2,
  "origin": {"path_style": "posix", "platform": "macos"},
  "files": {"settings.json": {}, "actions.json": {}},
  "export_assets": {"Practice.mte": {"encoding": "base64", "bytes": "...", "sha256": "..."}},
  "window": {
    "version": 2,
    "preferences": {"opentag.actions.quick": "..."},
    "workspace": {
      "version": 2,
      "paths": [], "selected": [], "roots": [],
      "working_directory": null,
      "list_order": "columns", "source_order": null,
      "location": "", "location_is_data": false,
      "filter": "", "filter_draft": null,
      "anchor": null, "playlist": null
    }
  }
}
```

This is a schema example, not captured data. Actual normalizer/value types must reuse component contracts. Preserve browser values as strings for exact store readback; validate their contents according to the registry before accepting a component. Do not evaluate templates or execute quick actions while validating.

- Use strict version-specific readers and explicit unknown-field/component policy. Unknown required envelope/window/workspace versions refuse with the original backup unchanged. Unknown preference keys or assets are listed as unsupported at preview; never written or silently counted as restored. Component-level selection can omit unsupported optional components, followed by a new bound plan. Do not salvage structurally corrupt native configuration.
- Retain the existing 32 MiB serialized bundle limit, 4,096 managed entries, and 2 MiB/10,000-row/10,000-root workspace bounds initially. Account for base64 expansion and total serialized bytes before allocating/writing the output. If the exact saved state exceeds a bound, refuse with counts and the playlist route; never truncate. Journal bounds remain separately enforced by the reused engine.
- Absence of an optional v2 window/workspace component means **preserve current**, not clear. Inside an explicitly selected complete preference snapshot, absent known keys mean remove/reset those keys; make this replacement policy visible in preview. An empty workspace explicitly selected means restore an empty list. Absent native files retain legacy preserve semantics unless a future schema explicitly represents removal.
- Include validated active `.mte` assets needed by the exported library, including newly discovered compatible templates; exclude arbitrary files, `export/removed`, hidden metadata and unrelated export-tree contents. Snapshot full current managed/export state locally for rollback through the existing engine, but do not put its entire arbitrary tree in the portable bundle. Validate asset names, case collisions, hashes, text encoding and structural export metadata; preserve unfinished template syntax as raw saved user data without compiling it. Stage v2 export metadata/assets together so target files cannot silently override incoming template text. Preserve unrelated target templates and show same-name replacements explicitly. Do not change v1's tested discovery semantics.
- A full portable browser/preferences backup is not a general source-script/profile archive: source bundles, custom language files, external tools/executables and referenced artwork/audio remain separately installed assets. Report references that are unavailable (e.g. a remembered web source or custom locale), without fetching/installing/executing them.

## Complete preference and workspace inventory

Use one versioned allowlist/validator registry, shared by portable backup and PR60 rollback. Start with existing `PREFERENCE_KEYS` and add the actual Tag → Tag memory key. Prefer semantic names internally while preserving existing storage keys on read/write.

| Stored group | Portable policy |
| --- | --- |
| Language and appearance | Capture `opentag.language`, `opentag.appearance`; validate choices. If a referenced custom catalog is absent, preview the builtin fallback; no implicit download. |
| Main layout/panel width | Preserve existing `opentag.main-layout` and legacy panel-width fallback, with native `settings.pages.layout` remaining authoritative when present, matching current startup. Avoid copying live screen coordinates outside supported layout validation; clamp floating bounds for the current display on restore. |
| Filter behavior | Capture filter-auto-apply/filter-shown. Workspace separately carries the applied filter and unapplied filter draft; the draft is a filter input, not an audio draft. |
| Converter memories | Capture numbering options and **`opentag.tag-tag`** field/template. Validate known fields/types/ranges without running conversion. |
| Quick actions | Capture `opentag.actions.quick` last type plus all saved per-type settings; validate using actual action schemas/catalog, retain disabled/unknown behavior explicitly, never execute. |
| Cover controls | Capture aspect and extract-format choices; no clipboard images/pending cover bytes. |
| Source controls | Capture lastSource, lastBuiltIn, legacy last, saveCover, suggestSorting, and the enumerated per-provider disabled-field selections. A missing installed source remains an unavailable reference; builtin-last remains separately preserved. No API keys/source settings/response cache. |
| Histories/field lists | Native `lists.json` only after awaited flush. Do not serialize `opentag.history.*` or `opentag.field-list` independently. |
| Version announcement bookkeeping | Keep `opentag.last-version` out of the portable user-preference snapshot; preserve the destination value so restoring another installation cannot suppress its current release notes. Retain it in exact local rollback if currently captured. This is an intentional portable/local distinction, not missing user configuration. |
| Unknown localStorage, credentials/provider grants | Exclude categorically; never enumerate arbitrary browser storage into the bundle. Do not read keychain or provider auth containers. |

Capture workspace identity/order, selected identities, roots, current working directory, raw data location vs generated label, applied filter/filter draft, anchor and loaded-playlist identity. Capture both base/manual row order and any distinct playlist order introduced by the active owner; persisted column sort remains native column configuration. Do not assume `entries.keys()` alone represents every new order contract. Extend the existing typed workspace rather than add a parallel main-only storage blob.

The current PR60 workspace misses `fileOps.history.current`: restoring list roots/location does not restore `_workingpath` or directory-navigation current state. Add an explicit validated `working_directory`; restore it through the existing directory-state/working-directory route without opening/scanning a directory as a side effect. Back/Forward navigation stacks can remain session-only if the preview names them as excluded; they are not currently persistent user preferences. Full current local-list workspace does include working directory.

Credential exclusion means known credential/authorization containers are never collected; it is not a claim to detect secrets manually embedded by users in formulas, tool arguments or history strings. Preserve user-authored text without reading credential stores to perform speculative redaction.

No cached tag/technical values are portable workspace state: re-read local files on restore. Preserve exact chapter/cue row identities through the existing physical-file-to-row resolution, refuse missing requested rows, and never collapse several rows to one physical path silently. No Undo/Redo history, application jobs, open dialogs, clipboard, pending audio/artwork/action-editor values, launch queues, scan cache, working-copy grants or provider URI identities travel in this archive.

## Save flow

1. Read recovery status through the shared coordinator; refuse export while an unresolved transaction would make the configuration incoherent. Do not trigger arbitrary profile recovery writes merely by opening a Save picker.
2. Await existing helper-list readiness/flush and pending saved-settings/layout publication. Capture the explicit preference/workspace schema once. Saving must not discard drafts or save audio. Freeze capture against configuration revision drift rather than freezing all unrelated UI indefinitely.
3. Native side validates the capture and every selected native component, resolves/export-validates active template assets, takes a bounded snapshot/hash under the shared configuration mutation lock, and rechecks drift before output publication. The current collect-only JSON parse is insufficient.
4. Write the completed archive atomically to the explicitly chosen output. Reject overlap/aliases with active managed configuration/recovery files and captured audio identities; never overwrite the state being backed up. Cancellation/failure leaves an existing output unchanged. Report component/row counts and exclusions; do not claim pending edits were saved.
5. Release capture pause regardless of success. Do not leave helper histories permanently paused after a failed/cancelled Save.

## Load preview and application

1. Open the chosen backup read-only, enforce size/schema/path bounds and credential exclusions, hash its complete bytes and bind canonical identity before reading recovery state or staging target changes. Load **preview must not discard drafts, touch audio or publish configuration**.
2. Show native components, preference groups, workspace row/root counts, exact unresolved paths/references, template collisions, origin/path compatibility, exclusions and the current rollback snapshot that this operation would supersede. Defaults select supported configuration/preferences; select the complete workspace when all its identities are local/available, otherwise present Restore workspace later / Keep current list. Never silently skip unavailable rows.
3. Reuse the existing local scan/row-resolution path to validate the complete selected workspace read-only before native publication. Permit explicit root-prefix remapping from one POSIX local root to another selected local root; bind the mappings in the preview token. Reject ambiguous/escaping mappings and changed row identities. Do not auto-search disks, follow provider grants, translate arbitrary foreign paths or rewrite action/tool paths heuristically.
4. Apply only after the ordinary busy/action-editor/unsaved-audio gate and explicit user choice. Keep current draft maps untouched until successful publication/reload; confirming discard authorizes their later loss, it need not destroy them before a source-drift or write failure. No audio Save action is invoked.
5. Await flush errors (do not suppress), pause stale configuration/history writers and bind source hash/identity, current native/window hash, selected components, mappings and the resolved workspace result in one native-authoritative plan. Recheck at Apply; drift expires the plan without publication.
6. Stage desired native files using existing component validators/save methods and export preservation rules. Build a desired complete native snapshot by overlaying selected components on current state. Preserve excluded/unselected files and all credential/provider/Undo stores. Persist the existing-style before/after/window rollback journal **before** first replacement; replace through the reused expected-content engine and common mutation lock.
7. Return operation status, not a misleading list-of-files success. Reuse the startup preference-before-construction and workspace claim/ack protocol. All selected components count complete only after native publication, browser readback, full workspace reconstruction and durable ack. A lost IPC response must reconcile durable status rather than reapply/revert a completed result.

## Recoverability and storage coordination

Native filesystem and browser storage cannot form one instantaneous OS transaction. Promise **recoverable application with no admitted mixed-state work**, not physical cross-store atomicity. Reuse PR60 phases, snapshots, content-conflict checks, checksum protection and start/failed/retry/ack, with operation-aware status/report labels.

- Factor the transaction core into one crate-private module (or expose narrow operations); do not copy `replace`, `rollback`, `persist`, `recovered`, hash/path validation or receipt logic into utilities. Keep original-profile parsing/plans in profile_migration. Parameterize source kind/report payload and window before/desired state; do not force portable input through fake Mp3tag candidates.
- Use one configuration-directory mutation lock/coordinator for profile migration and portable load, and make existing native configuration writes participate or trigger authoritative drift rejection. Reject a new mutation while a receipt is pending. Keep old journal v1 readers/checksum compatibility; journal evolution is independent of public archive version. Do not silently move/delete existing `.profile-migration` recovery files. A minimal journal v2 migration must preserve old pending records, then keep explicit version errors recoverable on downgrade.
- Recommend one **latest setup rollback checkpoint** shared by both setup-changing commands. Retain the existing `previous.json` crash-recovery protocol before superseding it and show the replacement in preview. This is not a multi-level Undo stack. If retaining an independent forever-available Mp3tag rollback is desired, that is a real policy expansion; it can use separate named checkpoint slots through the same engine, but should not be silently assumed.
- Before native commit, every refusal publishes no incoming restore state and leaves browser/current list and drafts unchanged. Ordinary saved helper/layout flushes precede the transaction before-state and remain saved even if a later restore check refuses. After commit, block stale frontend writes and report pending until reload/bootstrap resolves the receipt. Settings singleton reload failure is a transaction/recovery error, never reported as untouched disk or complete success.
- Browser storage unavailable/quota/readback mismatch: attempt the existing bounded preference rollback; retain the durable desired/before receipt if rollback cannot finish. Block ordinary startup mutations while acknowledgement is ambiguous. Provide Retry or Restore previous configuration; do not reset browser storage wholesale. No-journal startup still works without browser storage.
- A path disappears after preview or during restart: reconstruct into a temporary in-memory list and do not clear the previous list until complete. Preserve a failed pending receipt for explicit Retry/remap, Restore previous setup, or **Continue with settings only** (explicitly acknowledge omission of workspace). Do not inherit PR60's ordinary-startup fallback as implicit proof of complete portable workspace restoration. A settings-only continuation must suppress unintended startup-folder/playlist loads until its chosen final state is established.
- Process dies during native applying/restoring: existing recovered rollback restores exact known content; external conflicts retain snapshots and require attention. Process dies during browser/workspace claim: durable receipt resumes safely; consumed ack does not replay. Failed workspace receipt never loops or blocks new work forever without an explicit choice.
- Preserve launch precedence: receipt recovery finishes before configuration services; loaded workspace suppresses default startup directory. Explicit subsequent launch requests apply in existing serialized order. Do not replay old launch queues from the backup.

## Missing/moved resources and platform boundaries

- macOS/Linux POSIX paths that remain accessible restore exactly. Missing files/removable volumes report exact counts and retain the complete requested state for retry. Never truncate to the subset that happens to exist.
- An explicit reviewed prefix mapping may relocate a POSIX music root; validate mapped local paths and chapter/playlist identities. Other path-bearing user settings (external tool commands, export destinations, favorites, local freedb directory) are retained as configuration values and reported unavailable rather than executed or rewritten automatically.
- Unknown/foreign path style does not invalidate otherwise portable appearance/columns/actions/metadata configuration. Mark workspace unavailable, default it unselected, retain original path metadata in the report, and permit settings-only restore. Do not add Windows path interpretation, execution or platform acceptance in this work; Windows remains deferred.
- Do not newly desktop-gate the existing v1 Save/Load APIs because profile migration currently has a desktop-only wrapper. Keep the shared core platform-neutral, preserve existing v1 behavior, and expose v2 component capabilities through the runtime platform. Initial complete local-workspace acceptance targets macOS/Linux desktop; mobile preferences/native-config transport and document picker behavior require their own demonstrated capabilities and qualification, without transferring grants.
- Provider documents/grants/caches do not become local portable paths. Do not serialize `content://`, security-scoped bookmarks, permission grants or native working-copy authorization. A provider-backed current list can still export saved preferences/native configuration with an explicit excluded-workspace report; it cannot claim a complete local workspace snapshot. Mobile UI/runtime acceptance remains separate.

## Intended ownership for implementation

No ownership is claimed until root accepts this spec. All future work stays on the same consolidated PR64 branch/worktree.

| Owner slice | Files / responsibility |
| --- | --- |
| Transfer backend | `src-tauri/src/utilities.rs` portable v1/v2 adapters/preview commands; narrow extracted `src-tauri/src/configuration_transfer.rs` shared engine/schema registry if extraction is chosen; `src-tauri/src/profile_migration.rs` delegates to shared engine, retains profile parser. Registration-only `lib.rs` edits coordinated with root. |
| Transfer frontend | Reuse/factor `src/profile-migration.ts` preference capture/restore/startup receipt helpers; new focused `src/configuration-transfer.ts` UI adapter if needed; `src/utils-ui.ts` names/filter only if required. Do not duplicate the main workspace restore callback. |
| Main integration | **Current main.ts/playlist owner or root only**, after their order contract is frozen: Save/Load command routing, snapshot/flush/gate callbacks, working directory + both manual/playlist order state, early common recovery initialization. Coordinate `file-ops.ts` current-directory restoration if needed. |
| Component schemas | Focused exported normalizers/validators in existing action/numbering/tag-tag/layout/source modules only if the shared preference registry cannot use current contracts; never copy their execution logic into backup code. |
| Verification | `utilities.rs` and shared-engine native tests; `tests/profile-migration.test.mjs`, `tests/profile-startup-bundle.test.mjs`; new `tests/configuration-transfer.test.mjs`; actual production-main startup/workspace controls owned with main integration. Existing export tests remain intact. |
| Documentation/catalogs | Root owns guide/PARITY/remaining-gaps/verification receipts and new EN/NL/DE strings. Scope/write no product docs until approved implementation boundaries are final. |

## Exact acceptance / failure matrix

All new implementation cases need independently authored fixtures; tests below are planned, not run in this design task. Preserve existing PR60 controls in addition to the new cases.

| Group | Required cases and observable result |
| --- | --- |
| v1 compatibility | Existing valid v1 restores allowed files; absent files preserve; browser/workspace untouched; target MTE discovery retained exactly as current test. Invalid format/version/key/empty/malformed file rejects before writes. New reader rejects v3; old reader's v2 refusal remains explicit. |
| v2 saved round trip | Every allowed native file, every browser preference including quick action/lastBuiltIn/tag-tag, filter draft, local roots/current working directory/manual+playlist order/selection/anchor/loaded playlist round-trip with exact identities. No pending audio/artwork/action draft payload or Undo state appears. |
| Browser registry | Parameterize each known key with valid default/custom value, absence/reset, wrong type/invalid JSON/unknown key/oversize. Native validator and frontend registry agree; unknown storage keys never captured. Recheck no hidden duplicate helper histories. |
| Save coherence | Debounced helper history and saved-layout writes are flushed; flush rejection preserves existing output and UI; source/native revision drift rejects; no audio save/action/tool/source/native opener command occurs. Pending audio/artwork/action editor remains intact after Save. |
| Template assets | External active `.mte` and edited physical template included; arbitrary export files/removed history excluded. Missing/corrupt/duplicate-case/traversal/oversize asset refuses before mutation. v2 target same-name conflict choices explicit; unrelated late-dropped templates survive; current L44 retirement safeguards remain. |
| Preview immutability | Cancel picker/preview, malformed bundle, unavailable paths, unresolved source/locale and unselected components leave native files, browser prefs, current list, drafts and rollback checkpoint unchanged. Retarget same-byte source and source/target/known-audio aliases refuse. |
| Apply drift | Alter bundle bytes/identity, target native file/export tree, preferences, selection/order/filter/working directory/mapping after preview: no publication, re-preview needed. Parallel configuration save is blocked or detected; no stale helper overwrite. |
| Native write failure | Force failure before first/second/last file and final journal publication; restart from fresh processes at each phase; exact before content restored or known external conflict snapshot retained. Test absent-before file removal. No audio file hash change. |
| Browser failure | Storage getter unavailable, set/remove throws at each key, readback mismatch, rollback itself throws; no complete success/ack, durable receipt survives. No-journal startup tolerates absent storage. Retry/rollback later succeeds without duplicate application. |
| Workspace safety | Zero rows, 10,000 exact rows, 10,001 refusal; duplicates/selection-outside-list/invalid anchor/URI/NUL reject; physical file shared by chapter/cue rows resolves exactly; removed chapter row fails without collapse. Missing one row never replaces list with 9,999 rows. |
| Paths/mapping | Same POSIX root; moved root via reviewed explicit mapping; removable volume absent/reappears; ambiguous map/escape/retarget/replaced file; unknown origin style settings-only restore. No Windows runtime or new Windows-specific behavior. |
| Startup/recovery | Held/rejected status blocks configuration construction; profile/portable operations share one pending coordinator; start crash resumes; ack response lost reconciles consumed record; failed receipt requires explicit retry/remap/settings-only/rollback; startup folder suppressed; queued explicit launches apply afterward in serialized order. |
| Unsaved state | Load cancelled or blocked by busy/action editor keeps drafts; explicit audio/artwork discard consent occurs only after preview; failed Apply preserves in-memory pending values where no reload was committed; successful Load drops them only as disclosed. Never invoke save-tags or provider save-back. |
| Integrity/exclusions | Credential file, keys, source settings, grants, provider recovery, native audio Undo and original source bundle remain byte-identical; no keychain API called; foreign/unknown entries never written. Recovery journal is private/bounded/checksummed; symlinks and tampered paths fail closed. |
| Cross-store scope | Tests assert `pending` vs `complete` accurately; settings-only continuation reports skipped workspace and does not falsely ack full restore. Restoring previous setup returns both native/browser/workspace snapshots, with missing-path outcome explicit. |

Reuse existing test evidence by name: native `populated_folder_preview_import_restart_and_full_content_restore`, `rejected_second_publication_recovers_exact_before_content_from_journal`, `interrupted_restore_and_external_conflict_retain_recoverable_snapshot`, `journal_and_restore_survive_fresh_processes_without_preview_memory`, `claimed_failed_and_rearmed_receipts_survive_fresh_processes`; frontend preference quota/receipt/rearm/lost-ack controls; production bundle held/denied/storage-none/storage-pending startup cases. These were reusable prior source controls at design time. Their subsequent reruns and new portable-v2 controls are named in the implementation receipt; use that evidence rather than this planned matrix as proof.

## Acceptance boundary and unresolved decisions

Implementation can establish deterministic serialization/validation, actual filesystem atomic replacement/rollback, browser storage handling and controlled production-renderer startup/workspace behavior. It cannot establish original Mp3tag equivalence, actual native picker/keyboard behavior, signed/released installation, macOS/Linux OS integration, mobile provider grants/save-back, or current provider behavior. Those retain separate named qualification work. Windows is explicitly deferred.

Three choices are made explicitly in this recommendation: saved state excludes pending audio/artwork/action drafts; portable bookkeeping excludes last-version while local rollback retains it; the latest setup rollback checkpoint is shared and visibly superseded. Root can accept these policies with the design. A separate human answer is needed only if the desired product is a recoverable unsaved-edit session or independent long-lived rollback stacks, because those change the stated scope materially. Path remapping choices are made by the future user in Load preview, not guessed by implementation.


## Integration decisions after the playlist-order change

The user selected saved settings and workspace only on 2026-10-18. Unsaved tag/artwork/action-editor values do not travel in the bundle and Load never saves audio. Keep current drafts intact until a successful, explicitly confirmed Load; cancellation and pre-publication errors preserve them.

The newly implemented session has three separate order concepts: mutable entries insertion/manual order; optional sourceOrder rank; and listOrder mode columns/manual/playlist. Source inputs retain original ordered roots rather than expanded playlist members. Existing row IDs include virtual CUE/chapter identity. Known app-reported moves rebase rows, roots, active playlist/path location and current folder; no source playlist is rewritten.

Use explicit versioned window/workspace readers for the added fields, with old v1 snapshots retaining their actual limited fields. Do not add required fields to an ambiguously named workspace version1. v2 captures the actual current working directory, display mode and source rank for currently captured rows. Prune removed-row rank tombstones while retaining every captured row's relative source rank; Refresh reconstructs a new full source baseline later. Do not require removed historical rank members to exist or re-add removed rows on restoration. A legacy snapshot without rank cannot invent it.

Active .mte assets are saved user data. Preserve their raw bytes, checksum and encoding, including unfinished template syntax which the current load path permits for repair. Validate bounds, safe names, case collisions, decoded text and structurally valid export metadata; do not require that every saved template be executable, and do not rewrite raw assets by passing them through the ordinary save-time template compiler. Nothing is executed during capture, validation or preview. Preserve v1's tested smaller metadata-only behavior.

The quick-action envelope now also contains an optional validated groupCodepage preference. The shared preference registry must preserve it along with Quick last/type settings. The current normalizer, not an earlier schema copy, defines its compatibility/default rules.

Implement backend/schema/transaction reuse first, then renderer integration after the existing main.ts owner has released it. All changes remain in the same consolidated PR64. Root owns documentation, catalogs, ledger, final integrated checks and GitHub actions. Windows-specific work and original Windows capture remain deferred by the user; existing qualification gaps are not marked complete.

Flush boundary clarification: flush ordinary saved helper/layout changes before capturing the transaction before-state. These successful ordinary writes remain saved even if a later flush or preview/drift check refuses the restore. “Unchanged on refusal” means no restore publication relative to that completed-flush boundary; it does not roll back unrelated ordinary saved changes. A failed flush stops backup/restore without publishing output or incoming configuration, retains current drafts/workspace, and reports the failure. Bind preview/apply to the post-flush before-state; test successful flush followed by refusal and partial flush failure explicitly.


## Implementation amendments

The adapter lives in `src-tauri/src/configuration_transfer.rs` and reuses the existing PR60 engine in `profile_migration.rs`. Exact-row scanning suppresses implicit chapter expansion while preserving explicit virtual identities. A hash-bound encoding index preserves raw physical MTE bytes and their decoding choice without exporting the destination system ANSI default. Structural window bounds are shared with read-only unsupported-path inventory; applying a local window still requires local-path validation. Unsupported workspace previews require a settings-only selection review. These amendments implement the saved-only contract without adding foreign-path interpretation or a second recovery engine. See the [implementation receipt](../verification/portable-configuration-2026-10-18.md) for executed controls and remaining qualification.
