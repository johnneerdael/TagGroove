# Content-safe and durable edit history

The owner authorizes implementation and routine design decisions without further
approval gates. This design refines Task 2 of the active local-parity plan. The
current repair addresses L61; L67 remains part of the same task and follows the
content-baseline work.

## Content baseline

Use a private shared file-baseline module for history and staged audio writes.
Retain file identity, length and modification time, and add a streaming SHA-256
digest using the already locked `sha2` dependency. Read with a fixed 64 KiB
buffer, bound reads to the initial length plus an EOF probe, and compare
handle/path metadata before and after reading. Check native change times during
the read where available. Reject substituted nonregular leaf entries. Exclude
change time from the retained equality baseline because hard-link creation and
renaming legitimately change it without changing file content.

Perform cheap history size/free-space admission before reading content. Keep
the existing 2 GiB retained limit and 2 GiB free-space reserve. A refused or
failed history capture retains the existing nonfatal cannot-Undo behavior.
Hashing is linear in file length; the design bounds memory rather than silently
skipping large files or weakening checks above a size threshold.

Verify copied/hard-linked snapshots against the captured digest. Verify staged
restore bytes and the source snapshot before publication. Regular side-file
comparisons use the same streaming baseline rather than reading whole files into
memory; symlink side entries still retain their literal targets.

Track the captured and last successfully produced baseline for watched files.
Refuse a staged save if an outside replacement intervened after capture. Record
the produced staging baseline before publication so an outside edit between
publication and journal commit cannot be misclassified as application work.
Known moves retain the watcher identity; copied moves verify content before
adopting their new file identity. Failed checks invalidate that capture member.

The final check and atomic rename are separate filesystem operations. Do not
claim an atomic compare-and-swap or exclusion of arbitrary concurrent writers.
The regression contract is explicit: observed concealed edits, replacement
identities, corrupt snapshots and substituted leaf symlinks are refused without
overwriting the observed external bytes.

## Durable history after content checks

Add a versioned, bounded index with a cross-process store lock and native path
encoding. Publish synchronized snapshot/index state before collecting old blobs.
Persist capture/replay interruption state and refuse ambiguous interrupted
operations on reopening. Keep completed eligible operations across restarts,
including exact Redo, side-file epochs and partial outcomes. Group completion,
untrusted index paths, preserved budgets and crash-boundary regressions are
required. No automatic repair of an interrupted filesystem operation is implied.

## Evidence

Use real generated PCM WAVs and small side files. Preserve failing executables,
source bindings and independent input/external/output bytes. Cover Undo, Redo,
pure rename, corrupt audio and side snapshots, leaf symlink substitution,
staged-write interference, capture/publication intervals and admission order.
Run the existing history, writer, action and container regressions after the
focused controls pass, then measure the added hashing cost with bounded fixtures.
Native Windows identity/change-time execution remains a Windows handover item.

On Apple ARM64, enable the pinned `sha2` assembly feature through a target-only
dependency. Its runtime CPU dispatch retains the software fallback. Keep other
targets on their existing backend and do not change debug profiles to hide
checking costs. Compare the same history cohort before/after acceleration and
retain a separate bounded warm-cache hashing measurement.

Windows history capture still requires a native file identity. Ordinary staged
Save retains its existing ability to proceed when Windows cannot provide one,
while checking the available metadata and complete contents. Such a Save does
not promise an Undo step. Qualify this unsupported-identity case on Windows;
source inspection and Mac execution do not establish its native behavior.


## Recovery decisions during implementation

Use a version-2 typed operation intent, with captured original paths and snapshot
references published before work, plus an updated replay intent containing reverse
snapshots before any replay mutation. Record side-output preimages in the intent
before the side-output hook returns. A real process-exit test bypasses all destructors;
it does not establish hardware power-loss durability.

On reopening an interrupted operation, preserve current files. Move the ambiguous
active stacks and intent into separate verified recovery records and allow fresh
Undo history. Expose recovery availability independently of transient operation
notices. Recovery and active Undo/Redo share the existing operation and byte caps;
evict the oldest recovery records first and report eviction. Charge interrupted
capture preimages conservatively at full length, because their final write cost was
never committed. Retain known clone-backed charges for already committed operations.

Persist native identity guards for audio ancestors, side-output epochs and folder
moves. Cleanup ownership retains the identities of created parent directories.
Propagate an owned directory recreation to sibling and stacked references to that
same native identity; an outside-created directory does not qualify. Preserve
independent file/side failures and existing ordered folder/side rollback.

The remaining recovery UI must offer explicit export into a new no-clobber folder,
with a manifest that preserves native paths, operation/side roles and literal link
targets. Never restore over current audio or recreate links automatically. Export
must verify snapshot content and leave the bounded recovery store intact. Finish
its fault, native-path, corruption and space controls before claiming L67 complete.
