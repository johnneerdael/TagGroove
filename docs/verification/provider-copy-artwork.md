# Provider Save a copy: pending artwork

This change fixes the pending-artwork portion of roadmap R19. It does not
complete the specialist parity area (R14–R20), qualify Android/iOS devices,
or establish run-for-run Mp3tag equivalence.

## Request and write contract

`src/main.ts:saveCopy` snapshots pending fields and the optional pending cover
list into one `document_save_copy` request. The cover entries reuse the tag
clipboard's digest, picture type and description contract.

| Request | Edited copy | Original backup |
| --- | --- | --- |
| No pending cover list | Keep artwork already in the retained edited bytes | Copy the last verified source bytes |
| Nonempty pending cover list | Replace artwork in the supplied order, retaining image bytes, types and descriptions where the container supports them | Ignore the list |
| Explicit empty cover list | Remove all covers from the new copy | Ignore the removal |
| Stale clipboard digest | Fail before opening the destination picker | Ignore unapplied draft artwork |

Clipboard Copy, Paste and Cut use the readable retained working copy even
when the source is read-only or requires recovery. These operations stage
intent only. The source access policy continues to block **Save tags** and
immediate cover mutations, and an unreadable working copy is excluded from
artwork staging.

Recovery startup re-reads every restored entry through
`document_working_entries` before the restore operation finishes. A displayed
verified-backup fallback does not certify that the edited working bytes are
readable. If that read fails, the source and cover panels show the retained
metadata warning, clipboard operations omit artwork changes, and **Original
backup** remains available. Readable recovery copies still permit pending
artwork edits for **Edited copy**.

This reconciliation reads the requested physical working files regardless of
**List chapters as separate files**. It preserves their identities, chapter
summaries and artwork rather than returning virtual chapter rows. The normal
file-list and mutation-result chapter presentation remains unchanged. Repeated
paths are read once; each unsuccessful physical request retains its own path
in the error result, and cue-sheet requests are not expanded into other files.

The native wrapper resolves all requested clipboard image bytes before
`CREATE_DOCUMENT`. The document engine starts from retained edited bytes or
the verified original baseline, stages cover and field changes on the new
copy, then publishes through the existing durable pending/recovery flow.
The `io` command worker already initializes the document store while holding
`FILE_OPERATIONS`; the copy handler uses that initialization path.

The source provider document, source working copy, verified backups and UI
drafts are retained. Cancellation produces no copy record. A publication
failure retains the new copy's edited working bytes and its recovery state;
its provider output may be partial or unverified. No rollback of an unknown
provider output is attempted. A later copy can recover the retained edited
bytes without reading the source provider.

Edited-copy writes snapshot the active tag options once, before destination
creation. Both pending artwork and the subsequent field overlay use that same
write/remove policy and field mappings. For example, an APEv2-only MPEG policy
does not silently become the default ID3 policy when pasted artwork is saved,
and a later pending field does not undo that choice. An original-backup copy
ignores pending fields/artwork and does not apply policy conversions.

Provider import, recovery and published-result projection retain their existing
default read behavior. A custom mapped alias can therefore return under its
native field name even when the new copy was written using the mapping.
Applying user read precedence and field mappings throughout those reads remains
separate acceptance work. MP4 artwork still has no cover type or description.

## Regression evidence

- Before the UI fix, the new `copyRequest` test failed because the request
  omitted `covers` entirely.
- Restoring the original fields-only writer branch made the native regression
  fail: the output contained `CoverFront / Original front` instead of
  `CoverFront / New front` followed by `CoverBack / New back`.
- `src-tauri/src/documents/copy_command_tests.rs` calls the production
  `document_save_copy`, clipboard and cover-read commands. Its only provider
  substitution is the filesystem-backed command fixture. Every output is
  explicitly selected inside a disposable temporary directory and created
  exclusively.
- Native cases cover read-only sources, ordered replacement artwork, image
  byte equality, cover metadata, field deletion, unchanged artwork, explicit
  remove-all, original-backup isolation including stale draft artwork,
  cancellation before record allocation, missing clipboard data before
  destination creation, recovery-source copies, failed publication and retry.
- `src-tauri/src/documents/tests.rs` covers replacement and remove-all through
  partial provider writes, failed fresh reads and mismatched publication
  hashes. Reopening the store retains the edited fields and covers, permits
  a fresh edited copy without source access, and preserves original backups.
- WAV fixtures contain valid PCM data. An independent RIFF chunk walk checks
  unchanged `data` bytes after writes. PNG images are generated from distinct
  RGB values; native cover readback checks their original encoded bytes.
- `tests/document-coordination.test.mjs` executes production `operateImpl`,
  `clipTags`, `stageTagUpdates`, `coverTargets`, `saveCopy`, `enabled` and
  `sourcePolicy` with the real `documentSelection` and tag-clipboard models.
  Copy → Paste and Cut → edited/original copy journeys cover both read-only
  and recovery-required sources. No pending-cover injection or permissive
  policy stub is used. With the old publication gate in `coverTargets`, all
  16 copy-only-source staging/retention cases failed because no artwork intent
  was staged; removing that gate makes them pass. Checks also cover explicit
  remove-all, artwork-only pastes, unavailable working metadata and blocked
  source writes. The controlled transport and dialog boundaries still make
  this model-level UI evidence; it does not exercise OS input or a real picker.
- Four restart cases execute the production `platformReady` callback with
  readable recovery entries or the native restore contract's verified-baseline
  fallback. Real Copy → Paste and Cut are followed by edited/original copy
  requests. Before startup reconciliation, the two unreadable cases staged
  replacement/removal artwork and all four cases omitted the working-byte read;
  with reconciliation they pass, retain the warning where needed, and keep
  original-backup copies isolated. The native transport remains controlled;
  these cases do not claim an actual Android process restart.
- A retained WAV with an embedded two-track cue sheet and artwork exercises
  the production working-entry worker with chapter listing enabled. The
  preference is scoped to that I/O worker thread for the test; no app or global
  settings are installed. The normal `batch` transform is a positive control
  and returns two chapter rows. Before the fix, the working-entry worker did
  the same; it now retains the physical entry and cover summary, including
  ordered deduplication alongside a second retained file and an unmanaged
  `.cue` error. Provider bytes are deleted before the read. The actual async
  command boundary is also checked with default test settings, and unreadable
  working bytes still refuse baseline substitution with either preference.
- Two policy regressions use the frozen owned `tone.mp3`, an APEv2-only MPEG
  write policy, a custom field mapping and actual clipboard image bytes.
  Native APE footer/item parsing checks the absence of new ID3 tags, retained
  fields, mapped native names, deletion intent and the exact PNG bytes; the
  MPEG payload, source document and source working copy remain unchanged.
  Both artwork-only and artwork-plus-fields copies are checked. Restoring
  either the old cover-policy default or the old field-overlay default makes
  its corresponding case fail. Original-backup copies remain byte-identical
  while ignoring stale artwork and pending fields. The nondefault policy is
  scoped to the production synchronous copy body on its I/O worker thread;
  existing default-policy cases also exercise the actual async command.
  Result projection is checked under its current native-name behavior and an
  independent options-aware read verifies the mapped alias. No provider-wide
  read-policy parity or physical Android device is claimed.

Validation commands and their final results are recorded in the PR. These
fixtures exercise real metadata and audio files with a substituted SAF
transport. They do not prove Android binder behavior, cloud-provider
publication or real-device interruption handling. No Mp3tag runtime was
executed and no user music library was operated on.

## Specialist program remaining scope

The roadmap baseline is `4a1a95ff4eddb9da7492aaace68197c4f81347d0`.
The following are outstanding acceptance work, not completed claims. Their
priority follows the approved parity roadmap; a stale register entry must be
reproduced against current source before implementation.

| Initiative | Remaining acceptance work |
| --- | --- |
| R14: specialist metadata/formats | Obtain provenance-recorded real-encoder and reference-writer fixtures for TAK, OptimFROG, Nero, APE, ASF, Matroska, ADTS and uncommon ID3 variants. Independently inspect metadata and unchanged audio after edits. Qualify chapter URLs/artwork, subsong syntax and safe refusals, native field grammar, tag-family interactions, technical hashes and tag sizes. Separate container limits from defects. |
| R15: cover conversion/transfer | Measure resampling, size rounding, quality and Original-format fallback using paired results. Qualify Linux drag-out names and transfer on Linux. Preserve unchanged image bytes where no conversion is requested. |
| R16: networking/legacy sources | Qualify actual keyed-provider setup, needed proxy authentication/SOCKS/PAC cases and gnudb server-list/local lookup behavior. Interpret source minimum-version declarations as required capabilities. Keep live HTTP receipts distinct from parser replay. |
| R17: persistent recovery | Design and validate desktop restart recovery/undo retention and stale-baseline rejection, Windows external-edit detection and snapshot budgets on non-cloning volumes. This PR exercises existing provider restart recovery only; it does not persist desktop Undo history. |
| R18: desktop customization | Observe original File Exit, Windows Properties, floating panels and movable toolbar workflows before selecting platform behavior. Treat binary-only setting names and a possible global leading-zero option as unknown until independently established. |
| R19: mobile documents | Apply read precedence and field mappings to provider read/reload/copy paths; qualify provider undo, exports, playlists and file actions; implement iOS provider selection/save-back and qualify directory/cloud providers. Run permission-loss, interrupted-write and restart recovery cases on real Android/iOS devices. Mobile credentials remain separate work. |
| R20: languages/regions | Add demand-led, independently authored catalogs after high-impact workflows stabilize; qualify complete journeys, Unicode, locale-sensitive formatting, layout and input. Do not reuse Mp3tag proprietary catalogs or infer regional correctness from catalog count. |

Signing and distribution qualification belong to the broader platform/release
work. This PR supplies no new Windows signature, macOS notarization, iOS
physical-device artifact, reproducibility result or dependency-license audit.
