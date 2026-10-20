# Reviewed associated-file Copy — 2026-10-20

This unmerged working-branch extension addresses the Copy portion of L22a.
Filename → Filename and action-driven rename routes still need implementation;
inline audio-only behavior, original semantics and native/platform acceptance
remain explicit. This receipt does not close L22a or expand PR54's historical
C06 scope retroactively.

## Behavior

The existing optional `.lrc` / `.cdg` policy remains off by default. When enabled,
Copy and Move each obtain their own mode-bound, single-use preview token before
the existing confirmation. The frozen source/destination paths are displayed;
changes in policy, selection/order, destination, members, content or token mode
observed during final preflight refuse before the first copy. Existing expiration and token limits remain.
An omitted public `preview_transfer.mode` defaults to Move for old callers;
the current frontend supplies Copy or Move explicitly.

Copy includes only exact-basename supported companions, preserving extension
case and file contents. Suffix variants, artwork, other stems and unsupported
extensions are excluded. The full plan checks names and portable letter-case
collisions first; shared companions are rejected. Copy plans additionally hash
full audio bytes and retain the identity of the existing target directory.
The destination is flat and existing, so this route creates no new folders.
Copy hashes audio once to freeze preview, once in full-plan admission before any
mutation, then verifies the digest while streaming the actual copy. Immediate
group/member checks reuse metadata and membership rather than repeatedly reading
the whole audio again. Move retains its existing content-check behavior.

Each group copies companions first and audio last. New leaves are exclusively
created. The independent low-level `tag_core::file_copy` module returns receipts
bound to the created handle's metadata/identity and the bytes actually written.
These are not snapshots first obtained by trusting an arbitrary later target
path. Partial writes update the digest/length only for successful writes; a
source change refuses. Original sources are read, never moved, removed or
re-keyed. Copies receive source timestamps/permissions like ordinary Copy.

On a group failure, receipts reverse only the newly created destination files
still matching recorded content and identity. Observed edits, equal-byte
replacement identities, symbolic-link substitution or unavailable paths are
retained/reported with cleanup details. An ownership failure in the low-level
copy retains the ambiguous destination rather than adopting or deleting it.
Earlier successful audio groups can remain copied, preserving existing transfer
semantics. Preflight conflicts stop the entire selection before starting.

Copy reports only copied audio rows in its existing result shape; companions
are never parsed as audio or re-keyed into the File List. Copy remains without a
transfer Undo step. The last successful Copy destination is remembered only
after at least one audio copy succeeds, separately from Move. Cancellation or
failed preview does not apply a token or remember the uncompleted destination.

Move's grouped rollback/history path and its source capture remain unchanged;
no Copy receipt is used as a Move receipt. Existing inline/other route contracts
are not silently enrolled by changing the low-level rename primitive.

## Focused evidence

RED reproduced Copy bypassing required review, plus the frontend's Move-only
preview. GREEN: four route tests (with internal stale/policy/selection/folder
variants), three low-level receipt tests, and associated-file group tests.
The transfer module passed 17 tests with one real-trash probe ignored; the
association module passed 10; the low-level receipt module passed three. The
frontend file/options/catalog cohort passed 73, followed by the Options-note
regression and its affected module checks. TypeScript, catalog extraction, the
strict guide build, formatting/diff checks and five ledger views passed. Final
read-only review is clear after correcting the stale Options note. The final
streaming-check delta passed the ten association and seventeen transfer tests,
plus library/test-target Clippy for both affected Rust packages. Counts overlap
and are not additive.

Real temporary filesystem cases cover source preservation, independent copies,
exact extensions/suffix exclusion, receipt single use/wrong mode, same-stamp
Unix audio edits, changed/new companions, occupied destinations, changed policy,
selection order, replaced/symlinked destination folders, multi-buffer copies,
late audio failure after a companion copy, equal-byte target replacement,
owned cleanup and refusal to adopt changed targets. Group-level wrong-byte copies
are refused against the frozen digest. Fixtures exercise byte-preserving
transfer behavior; they do not certify an audio parser or metadata writer.

## Limits

No local application/native UI replay, original Mp3tag execution, live provider,
network/cross-volume fault qualification, Windows native acceptance, package or
release execution occurred. Source identity in the existing association/rename
snapshot is inode-bound on Unix; Copy now hashes full audio on every platform,
and destination/rollback receipts use the existing native Windows identity
helpers. Windows handle-close timestamp and readonly behavior is not runtime
certified here. Checks establish observed identity/content changes; the final
check and unlink are not an adversarial filesystem compare-and-swap.

L22b/C06 and L78/L79 retain the original/native/platform qualifications. Review
this slice's final PR head through the required genuine GitHub Codex code review
and immutable Linux CI before merge; local reviews do not satisfy those gates.

## Timestamp-fixture portability

The equal-byte replacement fixture opens its created replacement with write
access before restoring its timestamp. This supplies the write-attributes
permission required by the underlying Windows timestamp API; see Microsoft's
[SetFileTime documentation](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfiletime).
The adjustment affects only the temporary test fixture; Windows runtime
qualification remains separate.
