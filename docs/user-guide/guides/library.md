# Reuse loaded metadata with the Library

The Library is a persistent cache of metadata TagGroove has read. It can avoid
parsing unchanged files again when you reopen an album. It does not contain
your music, scan an entire disk by itself or turn a remembered path into an
accessible audio file.

## Enable it for one practice folder

1. Open **File → Options… → Library**. **Enable Library** starts off.
2. Enable it. If you want to limit persistent caching, also enable
   **Only include these directories** and use **Add…** to choose your copied
   practice folder.
3. Save the Options with **OK** or **Apply**. Adding a folder to the
   inclusion list does not load or scan it.
4. Open that folder using **Change directory…** or **Add directory…**, with
   the intended subfolder setting. The first load still reads the audio files.
5. Reopen the same unchanged files. Inspect the Library status in Options
   for its stored-file count, database size and last-load cache/read counts.

The status wording **read from disk and stored** counts entries actually
stored in this cache session, not every disk read of an excluded file. With
the cache disabled or unavailable, those counters can describe the preceding
cache-backed load rather than a new direct scan. They are diagnostic hints,
not a complete I/O audit or timing measurement.

Later loads can reuse entries when the path, modification timestamp, file
size and reader/settings fingerprint match. A changed reading policy or
reader revision can require a fresh read even when the physical file did
not change. There is no promised speedup for every collection or storage device.

## Understand what is indexed—and what is queried

The Library is queried internally by the paths being loaded. There is no
separate full-collection Library browser or “open every indexed track” command
in the inspected UI. Open actual files, folders or playlists through the
normal File commands.

[Filter](filter.md) and [Find](find.md) work with the loaded file list. They
do not search every cached path that is currently absent from that list.
A known cache entry also does not grant provider permission, reconnect a
network drive or restore a file removed from disk.

| Item | What it represents | Is it an audio backup? |
| --- | --- | --- |
| Loaded file-list row | Information currently shown for an opened file/row | No |
| Pending edits | Unsaved field/artwork intent in the current workspace | No |
| Library entry | Previously read metadata plus file/reader validity information | No |
| Embedded cover bytes | Image data stored in the audio file and read when needed | Part of that audio file, not the Library cache |
| Independent copied audio file | A separate file you intentionally keep | Only to the extent you have verified and retained that copy |

The database resides below the current configuration folder at
`library/library.db`. It stores metadata and image descriptors; it does not
move embedded image bytes or other binary payloads out of your files into
the database. This differs from the binary-storage behavior described for
the original editor's Library.

## Choose included folders without hiding music

With **Only include these directories** off, the cache can store files loaded
from any ordinary supported location. With it on, only files within the
listed folders are persistently cached. Descendant album folders belong to
the selected parent scope.

Files outside that scope can still be loaded and edited; they are read from
disk rather than stored for later cache reuse. A restricted but empty folder
list caches nothing. The restriction is a cache policy, not a file-list filter,
a ban on opening other folders, or a backup selection.

**Remove** in the included-directory list removes the inclusion entry, not
the music folder. Existing cache rows outside a newly restricted scope can
remain until maintenance removes them, but they are not eligible for scoped
reuse. Recheck the intended scope before **Cleanup Library**: cleanup uses
the directory restriction currently shown in that Options dialog.

## Know when an old value can survive

Consider one file already cached with `TITLE` = `Paper Boats`. These are
examples of cache eligibility, rather than a complete corruption detector:

| Next load | Expected cache behavior |
| --- | --- |
| Same path, size, timestamp and reader settings | The stored entry can be reused |
| The file size or modification timestamp changed | Read the file again |
| Tag-reading options or reader fingerprint changed | Read under the current interpretation |
| **File → Reload tags** is requested | Force a new read instead of reusing the entry |
| An external edit keeps both size and timestamp unchanged | The old entry can still look valid; force a reload |

TagGroove's own supported write/move/delete paths invalidate affected cache
entries so a later load reads them again. That is not an automatic watch of
every external program's changes. A collection manager or timestamp-preserving
editor can leave the stamp unchanged.

When a value looks stale:

1. Inspect whether TagGroove shows a **pending** value rather than a saved one.
   Resolve pending edits deliberately before discarding them.
2. Select the affected files and choose **File → Reload tags**
   (++cmd+t++ / ++ctrl+t++). The normal pending-edit confirmation explains
   that reloading discards selected unsaved intent.
3. Compare the result with the tag-reading policy and the actual saved file.
   Multiple stored tag blocks or a player's own cache can explain disagreement
   even after TagGroove's Library entry is refreshed.
4. If deliberately rebuilding the cache is needed, use **Clear Library…**
   as described below. Clearing it does not repair file contents.

Do not infer a fresh metadata read merely from reopening a file, or infer
saved metadata from what an unsaved field currently displays.

## Clean orphaned entries

Reconnect removable/network storage first if its cached entries should be
kept. **Cleanup Library** checks whether cached paths remain accessible and
within the chosen inclusion scope. A temporarily disconnected path can be
removed from the cache as an orphan.

1. Open the Library Options page and inspect the displayed restriction/folders.
2. Choose **Cleanup Library** and watch its checked/removed counts.
3. Use **Cancel** if necessary. Entries already removed remain removed;
   cancellation stops further checking rather than rolling maintenance back.
4. Inspect the result and any error. Reopening a still-existing file can add
   its metadata again when the enabled inclusion policy allows it.

Cleanup does not delete the referenced audio files, reorganize folders,
refresh every surviving tag value or discard pending edits in the workspace.
It is not a substitute for **Reload tags** after an external timestamp-preserving
edit.

## Clear and rebuild the cache

**Clear Library…** asks whether to remove every cache entry. Choose **Clear**
to proceed or **Keep** to cancel. This maintenance happens immediately;
canceling the outer Options dialog afterward does not restore cleared rows.
The same immediate-maintenance principle applies to a completed cleanup.

Clearing retains the audio files and does not save their pending edits. The
next eligible load reads metadata again and rebuilds entries as needed. An
empty cache is not a durable deletion of sensitive metadata from music files,
backups, exports or other applications.

Use the built-in controls rather than editing a live SQLite database or
removing its write-ahead-log files. The cache is rebuildable; schema changes
or an unusable database can cause it to be replaced. It should not be your
only retained copy of titles, locations or album information.

## Backup, privacy and performance boundaries

A [configuration backup](../customize/configuration.md) includes the saved
Library preferences in settings, but excludes the Library database. Restoring
those preferences on another computer neither restores the cache nor copies
the audio. Verify destination paths and let the cache rebuild from files
that actually exist there.

Cached metadata and paths can still be private even though audio/image bytes
are not stored there. Avoid sharing your configuration folder or database as
an example collection without inspecting it. Clearing the Library is not a
forensic erasure guarantee for storage, journals or backups.

The cache still performs filesystem checks and database I/O. Slow/removable
storage, network latency or endpoint scanning can limit the benefit. Inspect
the hit/read counts and actual storage behavior before attributing all loading
time to tag parsing. The original documentation's Windows Defender exclusion
names a different application's executable; it is not a TagGroove prerequisite
or a recommendation to change your computer's security policy.

These instructions describe the current local-file cache and its controls.
They do not qualify native performance, provider access, audio recovery or a
particular filesystem's alias/consistency behavior.

## Related

- [Library options](../reference/options/library.md)
- [Opening files](../getting-started/opening-files.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Tag formats and reader policies](../concepts/tag-formats.md)
- [Save and restore configuration](../customize/configuration.md)

<!-- verified: src/library.ts; src-tauri/src/library.rs:scan_with, changed, cleanup_library, clear_library; crates/library/src/lib.rs:Stamp, fingerprint, Scope, Session, Library.lookup/cleanup/clear; src/main.ts:reload. Temporary-DB/current-source checks, not personal-library scanning or a native performance benchmark. -->
