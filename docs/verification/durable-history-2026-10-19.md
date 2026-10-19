# Durable desktop history: implementation and bounded evidence

Status: L67 is represented by C62 in frozen PR64. Implementation commit:
`7f29fd7914d6b62fa25bf8fc666d425fbcae08c7`. The full-branch local review finding is
repaired and targeted rereview is clean. Final exact-head GitHub Codex review,
trusted current-main CI and merge remain pending. The owner froze further feature
work in PR64; later work is prioritized in the [handover](../handover/PARITY-PRIORITIES.md).

## User behavior and boundaries

Eligible completed Undo/Redo steps reopen across processes. A bounded native-path
index and exclusive native-identity lock protect one store. Captures publish typed
original-path/preimage intents, including side outputs before their hook returns.
Replay publishes its eligible reverse snapshots before filesystem mutation.
Publication precedes collection of no-longer-referenced blobs.

Interrupted reopening leaves current files unchanged and retires ambiguous active
history into separate recovery records. Fresh history remains usable. Recovery,
Undo and Redo share the unchanged 50-operation/2 GiB retained limit; Unix admission
keeps the existing 2 GiB reserve. Unknown interrupted write costs are conservatively
charged at full preimage length. Already committed clone-backed costs stay known.
Oldest recovery records are evicted first, with a notice, when history needs room.

Native directory identity guards cover audio ancestors, folder moves and side-file
epochs. Cleanup retains ownership identities for created parents. Owned recreation
updates matching sibling and stacked references; an outside-created empty folder
is refused. Leaf/content integrity and unchanged refusal remain intact. Completed
process-local group identifiers reset on reopen, and partial replay retains only
successful members for the opposite stack.

Export creates a new no-clobber folder, verifies full snapshot contents, and writes
a versioned manifest last. Native path units, before/after roles, side-output move
epochs and literal link targets are retained for completed/replay records.
Interrupted capture records instead use each captured absolute path with epoch 0
and no reconstructed move list, because completed moves are supplied only at
commit. This is capture-relative provenance, not a claim that no folder moved.
Links are exported as data, not
recreated; originals and retained history are not overwritten or consumed. The
persistent recovery notice remains independently of ordinary save notices.

The store remains in the application cache. Cache removal/purging can lose history;
this is a bounded recovery aid, not an independent backup. The final file check
and replacement are separate operations; arbitrary simultaneous writers are not
universally excluded. Native Windows identity/publication and hardware power-loss
qualification remain separate from Mac process/filesystem evidence.

## RED/GREEN and local validation

The [source-bound receipt](durable-history-2026-10-19.json) identifies retained
executables, source hashes, fixtures, logs and scopes. Earlier prototype failures
and their input/oracle bytes remain retained on the authoring Mac.

- Backend overlap/replay ordering and stale startup-notice clearing were reproduced
  before repair. Capture/journal-drop watcher leaks and failed replay preparation
  were separately reproduced and repaired.
- Four directory-substitution failures reproduced unsafe acceptance/removal/moves;
  the repaired matrix includes ordinary owned recreation, multi-epoch reused paths,
  stacked Redo references and non-symlink directory replacements.
- A real child process exits 91 without Capture/Journal destructors after changing
  audio and a cover. Reopening preserves those results and original recovery copies,
  allows new Save/Undo/Redo, and enforces smaller byte/operation limits.
- Recovery export initially refused because it was absent; positive audio/cover and
  native dangling-link targets now export with unchanged originals, repeat export,
  full-content corruption refusal and destination-reserve checks. A separate
  failing manifest control established the missing folder-move epoch before repair.
- The focused core/history cohort passes 290 tests, with one manual benchmark ignored.
  The whole backend library passes 571 tests, with 18 intentional ignores. The
  complete Rust workspace passes 1,510 tests across 84 executables, with 21
  intentional ignores and no Rust source drift against its compiled bindings.
  All 849 UI tests, TypeScript/Vite, strict core+backend all-target Clippy,
  formatting and the pinned strict guide build pass for the recorded snapshots.
  All 1,521 local links across 153 changed documents resolve. Exact-head GitHub
  review/CI remain separate gates.

## Native Mac application journey

A separately identified, ad-hoc debug application was built from this snapshot;
its executable SHA-256 is recorded in the receipt. Generated PCM WAV and cover
fixtures are independent of any real library. Seven phases passed:

1. Save through the actual WebKit UI and Rust backend.
2. Restart in a new native process and Undo, with exact original file bytes.
3. Restart again and Redo, with exact saved file bytes.
4. Open an interrupted capture, display recovery independently of Undo/Redo,
   export through the actual native folder picker, and perform fresh Save/Undo
   while recovery remains available.
5. Refuse a concealed same-file/same-size/restored-mtime PCM edit on native Redo.
6. Save a new step for the other direction.
7. Refuse the corresponding concealed external PCM edit on native Undo.

Save/Undo/Redo were driven by synthetic UI events against the real native app.
The Export button, Go to Folder and Open selection used actual CUA native input.
The parent independently compares WAV bytes and exported preimages. The
[observed recovery window](assets/durable-history-native-recovery-2026-10-19.png)
shows the actual rendered notice and export control, not a fabricated browser state.
This does not certify the broader physical keyboard/IME/accessibility matrix.

Two fixture/environment limitations are retained rather than hidden. The first
native attempt hit real low free space: Save succeeded while history correctly
reported unavailable Undo. After the successful three-process Save/Undo/Redo run,
the disposable cache was absent before interruption setup. The recovery phase was
therefore seeded from a fresh real production-journal capture, not claimed as old
history surviving cache deletion. The abrupt-exit helper and native app use the
same store/index code; its closed disposable store was moved without changing
snapshot/lock identities. A symlink-root setup was correctly refused and replaced
with a regular-directory setup. This is not a power-loss or native UI process-kill
qualification.

## Review correction: ordinary no-argument startup

The whole-branch local review found that loading history only after file work
hid retained Undo/Redo and recovery on an ordinary no-argument launch. Two tests
executing the actual platform-ready initializer failed before repair. Desktop
startup now refreshes history independently of a restored or empty workspace;
all seven history-UI controls pass. A native no-argument probe reproduced the
hidden recovery banner on the preceding app, then passed on the rebuilt app
with the retained recovery banner and restored row visible. Both executable
identities and RED/GREEN receipts remain distinct in the JSON; the seven phases
above are not relabeled as runs of the newer executable. The same review's
capture-epoch ambiguity is addressed by the explicit provenance limits above.
Targeted local rereview reports no remaining concrete issue; it is not the
required GitHub Codex CODE review.

## Still to finalize for PR64

Obtain genuine exact-final-head GitHub Codex CODE review and trusted
current-main CI. Mark the PR ready and merge only when those gates are satisfied.
Do not start another parity feature in PR64. Windows/Linux/device/original acceptance
and other explicitly pending local work remain visible for the prioritized next PR.
