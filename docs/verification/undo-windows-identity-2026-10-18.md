# Windows Undo replacement identity — 2026-10-18

## Implemented scope

The [history fingerprint](../../crates/tag-core/src/history.rs) now obtains
Windows size, modification time and filesystem identity from the same
attributes-only file handle. It prefers `GetFileInformationByHandleEx` with
`FileIdInfo` (volume and 128-bit ID), falling back to
`GetFileInformationByHandle` (volume and legacy 64-bit file index).
Failure of both queries is an error, never a silent size/time-only fallback.

An initial Windows fingerprint error abandons that capture through the
existing **cannot be undone** notice. Replay treats an unavailable fingerprint
as changed and refuses replacement. The Unix metadata/device/inode path,
staged-write tracking and 2 GiB limits are unchanged. Other non-Unix,
non-Windows targets retain their prior fallback. No content hashing,
ChangeTime check, unstable Rust API or new package version is introduced.

The Windows-only dependency uses the already locked windows-sys version.
Primary API contracts are [FILE_ID_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info),
[the extended query](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex)
and [the legacy query](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle).
The [legacy structure](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-by_handle_file_information)
documents filesystem-specific identity limits; prefer the full-width query
rather than assuming a 64-bit index is universally unique.

## Executed local evidence

The new real-filesystem integration regression replaces a file with another
of the same length and restores its exact modification time. Both Undo and
Redo refuse the replacement and preserve its bytes. On the macOS host,
26 history integration and nine history unit tests passed, including staged
same-size/timestamp-preserving saves, no-op Redo retention, moves and limits.
Targeted strict Clippy, formatting and whitespace checks passed.

A disposable macOS source copy projecting only the old `identity = None`
algorithm failed the replacement regression. This is an algorithm control,
not Windows execution, ABI proof or original-Mp3tag comparison. Independent
read-only review checked current local Windows bindings, handle ownership,
access flags and error propagation; it ran no Windows code.

## Required qualification and limits

Actual Windows compilation and runtime tests remain required on the final
reviewed head. Windows-only controls exercise the legacy query across reopen,
hard links and replacement, and the capture-failure notice. Hosted CI and
installed Windows/network-filesystem qualification are separate evidence.

This detects replaced filesystem objects, not every in-place edit. A writer
restoring the same object's size and timestamp can still escape detection.
Native IDs may be reused or behave differently across filesystems/network
servers. Falling back from a full-width identity to a legacy representation
is conservatively treated as a change. There is no ReFS-wide uniqueness,
provider-wide, original-runtime or universal external-change guarantee.
L61 therefore retains qualification and remaining detection limits.
