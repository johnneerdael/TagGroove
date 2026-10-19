# Source update snapshot preservation and process recovery — 2026-10-18

Consolidated draft PR64 fixes a reproduced installer refusal that destroyed the
previous recovery snapshot. A complete valid folder/ZIP update could delete the
old backup, then fail while writing the active folder's marker. The actual Unix
permission regression observed the old snapshot disappear despite refusal.
Only disposable destinations were written; user source inputs stayed unchanged.

## One existing recovery path

The [installer](../../src-tauri/src/sources/web/installer.rs) now retains the
validated candidate and old previous snapshot in a permanently marked rotation
container. A bounded intent records prepared/rolled-back/committed states,
tree identities and original cleanup membership. Candidate publication is the
commit point. The existing recovery entry handles rollback/retry; no separate
source restore engine or public archive format is added.

| Process/failure point | Required state |
| --- | --- |
| Staging or marker permission refusal | Original active and old previous bytes remain/restored. |
| Active rename or candidate publication refusal | Exact old active, original marker and previous snapshot restored. |
| Process stop after each rotation move | Fresh recovery restores the old state before commit, or retains the committed new state afterward. |
| Postcommit/partial retired cleanup failure | Active and previous remain untouched; retry deletes only remaining original retired entries. |
| Changed/new data or occupied recovery destination | Refuse without overwriting or deleting the conflicting data. |
| Malformed/unowned/symlink recovery state | Refuse unchanged. |

Native relative paths are hashed without turning them into Unicode source IDs.
The intent is limited to 2 MiB; existing source payload/depth/entry limits stay
intact. The reserved backup marker is excluded only from its internal snapshot
budget, not from content hashing.

## Review-driven interruption controls

Two independent reviews found that generic atomic-write scratch files could
block recovery if a process stopped before persist. Three actual worker kills
inside those writes reproduced the failure. The installer-local publisher now
prepares a complete marked, length/hash-bound package outside hashed trees,
admits it atomically into one reserved slot and renames its data to a fixed
internal target. Recovery validates and discards the owned package before phase
recovery; it does not replay metadata to an arbitrary path. Unknown random temp
files and changed/unmarked/symlink packages still refuse.

The reviews also found a bootstrap mkdir/marker window. Initialization now
prepares and syncs a fully marked sibling directory before admitting its fixed
name, with a fresh collision check. An unowned empty directory appearing at that
boundary remains untouched.

The final focused installer run passed **35 tests, 6 ignored**. The ignores are
three subprocess worker entry points exercised by parent controls, one private
input opt-in and two real non-UTF-8-name cases requiring a Linux filesystem on
this macOS host. The matrix includes six before/after-persist hard exits and
three bootstrap hard exits, fresh-process retry after failed rollback, real
permission failures and partial-cleanup conflict controls. Both independent
re-reviews found their findings resolved. Strict package Clippy, formatting and
whitespace checks passed on the final frozen source.

This establishes named local permission/process-termination behavior, not
power-loss durability, arbitrary concurrent filesystem mutation, native picker,
private-provider or released-platform acceptance. L30/N01/N02/L78/L79 remain
separate. Windows work is deferred. No original program, native GUI, live provider
or device was used; local backend workers ran with network denied.

## Current private bundle admission

After the recovery change, the existing opt-in admission/compiler test was rerun
on both original user-owned folders and their retained flat ZIPs. All four routes
passed: Traxsource loads eight sources/one schema/14 compiled blocks per route;
SoundCloud loads one source/one schema/two compiled blocks. All 16 original
runtime files and both ZIPs remain byte-identical before/after. Only disposable
destinations were used, with network denied and no source text redistributed.
These current-source checks do not supply response parsing, provider requests,
public apply ingress, native review or N02 acceptance.
