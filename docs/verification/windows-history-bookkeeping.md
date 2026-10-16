# Same-size saves and Windows undo bookkeeping

The Windows job of reviewed release run 38840093328 failed
`limits_bound_operations_and_bytes`: the last undo label was `Save 3` instead
of `Save 4`. The earlier release run 38829167747 failed the same-size raw-stream
undo regression with “There is nothing to undo”. Both core tests use private
journals. Their logs do not include before/after file timestamps, so a timestamp
tie in those actual runs remains an inference.

## Reproduced defect

On Windows, the history fingerprint contains length and modification time but
no file identity. Before this repair, commit treated equal fingerprints as no
change and discarded the snapshot before consulting the already captured
staged-writer count. A real same-size save with preserved modification time
could therefore disappear from undo history.

A local full-source probe projected only the fingerprint identity to `None`,
as the production Windows branch does. The production WAV writer changed
TITLE from `3` to `4` with modification-time preservation enabled. Bytes
changed, length and mtime stayed equal, and the old journal retained only
`Save 3`. The unmodified Unix identity control retained `Save 4`. This is a
Windows identity model on macOS, not native Windows execution.

## Repair and contract

A positive captured staged-write count now establishes a content replacement
before the no-op discard. Snapshot verification, operation/byte limits,
external-change checks and retained-byte accounting remain in place.

Actual byte-identical staged replacements already receive undo entries on
Unix because replacing the inode changes its identity. The repair gives
Windows that same replacement behavior. A genuine no-write request, including
an empty update, adds no history entry and preserves redo.

Regressions use actual production writers and private journals:

- Three same-width saves with preserved mtime must each retain their own label,
  undo all changes, redo them, and restore exact original/latest bytes.
- An empty update after undo must add no entry and keep the redo step.
- The existing raw-stream test explicitly preserves/asserts mtime while
  retaining its original byte, audio and cost checks.

The old Windows identity model fails both write regressions. The repaired
model, original Unix control and repaired Unix source each pass all 18 history
and eight raw-stream tests. Focused Clippy, formatting and patch checks pass.
Integrated validation against main `4e4d5fc`: 724 locked Rust workspace tests
and 318 UI tests passed, with four opt-in Rust cases ignored. Production build,
workspace Clippy with warnings denied, formatting, strict guide build and diff
checks passed. The initial cache moved from the completed PR27 worktree
contained absolute Tauri permission paths; only affected generated framework
cache entries were cleared before the successful Rust rerun.

Native Windows CI is required before merge; no CI or release failure was
bypassed or blindly retried.

## Review follow-up: saves after file or folder moves

The first review found that staged-write evidence was keyed only by the original
path. Supported Actions can rename a file or move its folder before saving it.
Three new production regressions reproduce lost tag restoration after a file
rename, a directory move, or both under the Windows identity model.

The tracker now retains the original journal identity and an index of each
watched file's current path. Successful core file/folder moves update that
index; staged writes and commit use the same original identity. Directory
updates use component-prefix ranges, and per-file writes keep indexed lookup.
Copied moves force full snapshot cost rather than claiming block sharing with
an old-volume snapshot. Pure renames still retain no audio snapshot, and
unrelated sibling paths are not rebased.

All three new regressions fail before this follow-up and pass afterward, with
exact original and edited bytes restored by undo/redo. The repaired Windows
identity model passes 133 core unit, 21 history and eight raw-stream tests.
Integrated native-macOS validation passes 728 locked Rust workspace tests
(four opt-in cases ignored); workspace Clippy, formatting and diff checks pass.
UI, production build and strict-guide checks passed earlier in the same PR;
this follow-up changes only core bookkeeping, move notifications and tests.
Actual Windows CI remains required before merge.
