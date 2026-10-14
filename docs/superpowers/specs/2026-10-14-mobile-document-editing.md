# Mobile document editing and recovery

The target remains full Mp3tag parity with native mobile interfaces. This is the
first document-editing integration, not a replacement for provider-backed
actions, renames, directory access or the later iOS integration. Desktop paths
and all existing desktop operations retain their current behavior.

## Intended behavior

On Android, Add files uses ACTION_OPEN_DOCUMENT and retained offered grants.
Chosen audio is copied into private persistent application storage and parsed
with the existing Rust metadata reader. URI identity remains opaque. The UI
shows the provider display name and access/recovery state. No broad storage
permission is requested. Only disposable test files are used for mutation UAT;
the Beatport library remains read-only.

Tag panel/Extended tags/numbering, filename-to-tag and embedded cover add/remove
must use the existing Rust mutations followed by checked provider publication.
A successful save is reported only after reading the provider bytes back and
matching their SHA-256. Read-only documents can be edited into Save a copy.
Unsupported provider-backed path/action operations fail before local mutation
and remain explicit future work rather than reporting a private-cache edit as
a saved document.

Each imported document has its own UUID directory, exact safe leaf display
name, working audio, last verified source backup and atomic JSON manifest.
Private paths remain backend handles; display/location and technical path/time
context refer to the provider, never to an invented user filesystem folder.
Duplicate URIs reuse a ready record. Pending/conflicted records must not be
silently replaced by reselection, reload or process restart.

Before mutation, require writable access and a ready record. After the Rust
write, persist a pending manifest before contacting the provider. The provider
rechecks read/write capabilities, hashes current bytes and rejects a mismatch
against the last verified source. It opens a truncating output stream, writes
the edited bytes, closes the stream, then hashes a fresh input stream. Only a
verified result becomes the new baseline and ready state.

SAF has no general compare-and-swap or atomic replace primitive. A change after
the final source hash check can still race a write; interrupted writes can
leave provider contents partial. Do not claim isolation or automatic rollback.
Retain both baseline and edited bytes durably, record recovery state, and expose
Save a copy for either version. Never overwrite an unknown provider state as
an automatic recovery attempt. A pending state discovered at restart becomes
recovery-required. Copy export uses ACTION_CREATE_DOCUMENT, which creates a new
document; it must verify destination bytes too. It does not clear the original
recovery record merely because an export succeeded.

## Interfaces and ownership

`crates/document-access`: extend the existing Rust/Kotlin bridge with:

```rust,ignore
pub struct DocumentSnapshot { pub document: DocumentInfo, pub sha256: String, pub bytes: u64 }
Documents::snapshot(uri: &str, output_path: &Path) -> Result<DocumentSnapshot, String>
Documents::publish(uri: &str, input_path: &Path, expected_sha256: &str) -> Result<DocumentSnapshot, String>
Documents::create_document(name: &str, mime_type: Option<&str>) -> Result<Option<DocumentInfo>, String>
```

The bridge validates paths beneath Android `filesDir` or `cacheDir`, rejects
symlinks/escapes, streams bytes with bounded buffers, creates snapshot files
exclusively, and never transports audio as base64. Each provider call completes
on the existing bounded worker pool; picker callbacks remain asynchronous.
Cancellation returns None. Snapshot validates provider metadata before/after
the streamed read; unknown size/time remain unknown. Publish validates the
expected SHA syntax and private input before opening a provider output stream.

`src-tauri/src/documents.rs`: portable durable working-copy engine plus a
cfg(Android) bridge driver. All calls occur under the existing FILE_OPERATIONS
lock. Public integration functions:

```rust,ignore
pub fn initialize(app: &tauri::AppHandle) -> Result<(),String>;
pub fn pick(app: &tauri::AppHandle) -> Result<ScanResult, String>;
pub fn states() -> Result<BTreeMap<String, DocumentState>, String>;
pub fn is_managed(path: &Path) -> bool;
pub fn mutate(path: &Path, work: impl FnOnce(&Path) -> Result<AudioEntry,String>) -> Result<AudioEntry,String>;
pub fn reload(path: &Path) -> Result<AudioEntry,String>;
pub fn project(entry: &mut AudioEntry);
pub fn snapshot_input(app: &tauri::AppHandle, path: &str) -> Result<PathBuf,String>;
pub fn save_copy(app: &tauri::AppHandle, path: &str, updates: &Fields, version: &str) -> Result<ScanResult,String>;
```

`DocumentState` serializes `uri`, `name`, `writable`, `persisted`, `state`
(`ready`/`recovery_required`) and nullable `error`. Map keys are canonical private
working paths matching AudioEntry.path. Desktop functions are pass-through or
explicit unsupported as appropriate. A provider trait supports desktop tests
against real audio fixtures and injected external changes/partial writes.
AppHandle global registration may be reused; avoid a new background daemon.

Root command integration owns `src-tauri/src/lib.rs`: Add files command
`pick_document_files` returns ScanResult; `document_states` returns the map;
`document_save_copy {path,updates,version}` returns ScanResult (empty on cancel).
`version` is `edited` or `original`. Basic mutations and reload use the engine;
other write routes explicitly reject managed documents until adapted. Image
content URIs are snapshotted privately before passing to the existing writer.
All successful entries pass through `project`. No core tag schema change.

Frontend owns `src/main.ts`/`src/style.css` plus a small pure document-state
helper/test when useful. On Android, Add files invokes pick_document_files;
refresh document_states after operations, preserve drafts on save-back failure,
disable unsupported or unwritable mutation operations with an inline reason,
and expose File > Save a copy… for one selected managed document. The copy
dialog offers edited or original backup when recovery is required. Edited copy
includes current pending draft updates. Empty copy result means cancellation,
not successful save. Copies are added and selected. Menus, compact pane state,
appearance and existing desktop behavior stay intact.

## Acceptance and remaining scope

Portable tests must exercise successful real-WAV tag persistence, duplicate
URI handling, read-only sources, changed-source rejection before truncation,
partial write/read-back failure recovery, restart recovery, private-path
validation, original/edited copy export and cancellation. Verify audio packets
and unselected files independently during native Android UI tests. Exercise
provider picker, Unicode display names, save/reload and recovery UI in Light
and Dark. Compilation alone does not qualify device/provider behavior.

Provider-backed action groups, dynamic filename conversions/renames, output
exports, directory trees, cloud-specific cases and iOS security-scoped editing
remain required subsequent work. Keep them visible in PARITY.md. Do not mark
the overall goal complete from this integration.

Primary references: Android's SAF guide and ContentResolver API, and Tauri 2
mobile-plugin documentation, inspected 2026-10-14. Existing source bridge and
Rust staged writer contracts are authoritative for local interfaces.
