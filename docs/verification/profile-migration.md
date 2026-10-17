# R02 guided profile transfer: implementation and proof boundary

This milestone wraps the existing columns, tag-panel, MTA and MTE importers.
It provides a native-authoritative folder/ZIP inventory, selected supported
imports, explicit collision policy, retained partial/unsupported reports, local
configuration rollback/restore and bounded window/workspace recovery. It does
not qualify undocumented Mp3tag global/binary mappings or complete R02 parity.

The independently authored populated profile and its hashes are recorded in
`profile-migration-fixture.json`. No original Mp3tag profile, private source
bundle, credentials or provider response is redistributed. There was no
original Windows application execution or original-profile differential test.

## What the implementation actually binds

- The native preview token retains parsed candidates and hashes the complete
  source inventory, current seven-file native configuration, physical export
  tree and explicitly typed window state. Apply re-reads source/target/window
  state before staging and rejects drift or unseen/duplicate selections.
- Existing parsers and save methods build candidate layouts/groups/templates.
  Source warnings remain visible. Unknown MTA records remain unresolved and
  disabled. Source collections/global/binary files stay outside the import.
- A private checksummed journal records complete before/after managed file
  contents before replacement. Writes recheck bounded regular paths. Failure
  and interrupted-phase recovery restore known content; external conflicting
  edits retain the named snapshot and error rather than overwrite unknown data.
- Restore includes physical MTE files and file absence, not only export JSON
  identities. API credential stores and unsupported native files are untouched.
- Window state uses an explicit preference-key allowlist, local absolute file
  identities, roots, selection/order, location, active/draft filters and playlist
  identity. Helpers/histories come from native lists.json. Limits reject rather
  than truncate: 2 MB window state, 10000 files/roots; 128 MB/4096 profile entries,
  64 folders deep and 4 MB per supported text definition.

Native-file rollback and browser/window recovery are separate phases. Browser
quota/getter errors cannot establish whole-setup completion. Pending receipts
remain and read/ack failures are shown with an explicit retry route.

## Startup and user intent

The earliest native status request runs before configuration-backed frontend
services are constructed. A native cfg guard returns metadata only on mobile,
without resolving configuration paths or performing recovery I/O. Browser demo
startup makes no such request. Missing journals do not create recovery folders.

Desktop bootstrap restores required browser preferences lazily; unavailable
localStorage is harmless without a pending snapshot and blocks with a retry
when restoration needs it. The verified bootstrap result seeds the UI directly,
avoiding a redundant fallible status read before initialization. Existing
runtime-platform failure, language/catalog and queued-launch behavior remains.

A pending workspace restores after saved options/language and before directory
initialization drains explicit launches. Successful recovery suppresses only
favorite/previous startup fallback. A newly queued B replaces or adds after
recovered A. Failed reads retain an attempted receipt but admit explicit launch
or fallback; an ambiguous native claim failure blocks those later actions.
Acknowledgement failure retains the restored list and actionable receipt. Every
manual import, restore or workspace-retry mutation uses the unsaved-audio/
artwork/action-editor gate; cancelling a preview does not discard audio drafts.

## Local evidence

The native suite exercises populated folder and ZIP transfer, partial columns/
panels, unresolved actions, explicit name collisions, legacy MTA/MTE codepage,
source immutability, configured value/template preservation, exact managed
content restoration and removal of files absent before import. Three fresh
processes prove journal readback, restore and consumed-receipt persistence
without the preview's in-memory map.

Negative controls cover source/template/window drift, cancellation, unseen
selections, ZIP traversal/drive/case duplicates, symbolic links, bounded items,
corrupted/path-tampered journals, a controlled second-publication refusal,
interrupted restore and an external recovery conflict that keeps the snapshot.

Frontend tests execute the real UI for choices/cancellation, collision/encoding
arguments, unsaved-edit gates, browser failure and one-shot receipt behavior.
The complete production Vite bundle executes with controlled native boundaries
for held/rejected/no-journal recovery, browser preview, Android/iOS metadata,
and unavailable storage with and without pending recovery. Existing real-main
startup tests now also exercise A→B add/replace ordering, fallback suppression,
failed reads, claim/ack refusal and saved-language/platform-failure contracts.
The actual main workspace callback restores all typed state for 10000 synthetic
rows and preserves the current list when a required row cannot be rescanned.

These are local native functions and bundled-renderer tests with controlled
boundaries, not native OS picker/key input or a released-build certification.
Application commit, exact final review and current-base platform validation are
recorded in the eventual PR, after branding sync/currency checks.

## Remaining R02 and R04 acceptance

- Capture a genuinely populated original profile with permission, compare the
  supported mapping in the original and TagGroove, and record intentional
  differences. No undocumented binary setting is inferred from a filename.
- Qualify actual native desktop folder/ZIP pickers, restart, complete displayed
  profile inspection and restore on the reviewed/current-base builds. Device
  and provider-document permission transfer are outside desktop workspace scope.
- Retain separate scope for global settings, additional preset semantics and
  unsupported components; independently authored fixtures do not certify them.
- R04 remains the private-source native review/import/save/reload/undo journey,
  recorded/direct-ID/settings/matching coverage, and distinct full-cover and
  album-preview semantics. Prior bundle compilation and partial live provider
  parsing do not close those outcomes.
