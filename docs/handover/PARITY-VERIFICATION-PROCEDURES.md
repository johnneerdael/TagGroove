# Verification procedures for other machines

Prepared 2026-10-19 for the follow-up after PR64. This is a procedure, not a
record of execution. Keep the [environment register](PARITY-OTHER-MACHINES.md)
and [priority table](PARITY-PRIORITIES.md) beside it. Windows-original execution
remains deferred by the owner. Portable implementation and available Mac work
remain local; an engineer on another OS must not inherit missing portable code
under a verification-only label.

## Start every evidence package with these identities

Record the exact merged source commit, clean/dirty state, lockfile hash, build
command/toolchain, executable/package SHA-256 and actual OS build/architecture.
For mobile, also record device model, OS/API, WebView/provider versions and
installation route. Name the tested filesystem, network share or cloud provider.
A simulator, emulator, CI compile or synthetic DOM run keeps that label.

Use generated or explicitly supplied disposable copies. Retain input and each
result stage separately, with whole-file hashes, independent encoded-audio or
PCM checks where supported, settings, ordered operator actions, logs and native
screenshots. Keep first failing files and diagnostic output. Do not overwrite a
frozen oracle to match a new result. Report pass, demonstrated difference,
unsupported/refused, or unobserved for each named case. A build alone is not a
native acceptance result.

## W1: Windows original baseline and save comparison

Required: an interactive Windows x64 desktop, Python, an existing 7-Zip and the
pinned owner-supplied Mp3tag 3.36.1 installer. The executable must hash to
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`;
the installer must hash to
`a977861003a87b1f00f2e0c91263c61bf12d457c6d457b367ce5f69de28fbea0`.
Mac Mp3tag 1.14.1 (145) is a distinct reference and cannot replace this capture.

Follow [the diagnostic-first capture guide](../verification/mp3tag-reference-capture.md).
From the frozen checkout, substitute new temporary directories and the actual
owned installer/7-Zip paths in these commands:

```text
python scripts/reference-capture/collector.py prepare-runtime NEW_RUNTIME_WORKSPACE --installer OWNED_INSTALLER --seven-zip EXISTING_7ZIP
powershell -NoProfile -File scripts/reference-capture/probe.ps1 -Workspace NEW_RUNTIME_WORKSPACE -RunReference
```

Inspect the probe's PID/image/session and shutdown record. Observe About,
configuration location, portable-mode behavior and selected read/write/remove
switches/encoding in the actual original. A changed candidate mp3tag.cfg alone
does not prove isolation or all selected defaults. Stop the capture batch if
isolation or control mapping remains unresolved; retain that diagnostic outcome.
The probe itself does not edit, Save, Undo or Redo.

Prepare the paired contract on the machine that will run and compare TagGroove;
its source/build/compiler identities must remain stable throughout. Use an
isolated frozen checkout for the whole pair. The producer needs the actual
[workspace/native build prerequisites](../CONTRIBUTING.md#embedded-audio-build-tools),
not Python alone. Use the installed Python3 executable name (`python` on Windows
when `python3` is unavailable):

```text
python3 scripts/save-acceptance.py prepare NEW_PAIRED_WORKSPACE
python3 scripts/save-acceptance.py run NEW_PAIRED_WORKSPACE
```

Transfer the prepared contract and identical hashed input copies to Windows.
Follow generated REFERENCE-INSTRUCTIONS.txt for every edit-one/edit-many case.
Capture the original's actual Save, Reload, Undo and Redo actions and resulting
files. Retain separate saved/undone/redone files and timestamps; do not assume
original Undo restores exact container bytes merely because TagGroove does.

```text
python scripts/reference-capture/collector.py validate CAPTURE_WORKSPACE --contract PREPARED_CONTRACT
python scripts/reference-capture/collector.py export-identity CAPTURE_WORKSPACE --contract PREPARED_CONTRACT --destination NEW_IDENTITY_JSON
```

The collector's evidence/OPERATION/STAGE/FILENAME layout is different from the
paired consumer's reference/OPERATION/STAGE/FILENAME layout. Transfer each stage
explicitly and verify hashes against the unchanged manifest before comparison:

```text
python3 scripts/save-acceptance.py compare NEW_PAIRED_WORKSPACE --identity TRANSFERRED_IDENTITY_JSON
```

Return the validated manifest/identity, original stage bindings, transferred
hashes and paired-results.json with actual differences. Current paired tooling
covers admitted MP3/FLAC/M4A layouts; it cannot certify other containers by
renaming them or substituting synthetic reference output. Keep downloaded vendor
runtime trees out of the evidence return. This procedure addresses L79 and the
named C03/C08/C14/L28 comparison subset, not blanket parity.

## W2–W6: Focused original behavior after W1

Use a separately hashed input copy and one controlled variable per case. Record
original settings and output before comparing TagGroove. The linked current
scope defines the contract; a setting name or static prediction is not an
observed original requirement.

| Procedure / IDs | Actual operator sequence | Expected evidence and useful controls |
| --- | --- | --- |
| W2 — L07/L50/L57b/L65/L66 | Create a populated original profile. Change one column/panel/toolbar/preference option, close normally, retain before/after configuration, reopen and record the visible setting. Import only a disposable copy into isolated TagGroove and record every supported/unsupported report; test restore and reopen. | Exact key/file change, UI meaning, import result and retained opaque data. Include custom_values/prefer_tag and toolbar symbols/layouts where present. No unchanged binary blob is assigned a speculative meaning. Portable missing mappings remain local implementation. |
| W3 — L10/L12/N04, C55/C56 | Enter the immutable filter-quoting and Split Field vectors from crates/tag-core/tests/fixtures/parity-native. Separately exercise malformed/error/Unicode, column and self-reference contexts with the same input values. Capture result order, errors and any saved file output. | Raw expression/input/result and actual original version/locale. Retain matching precedence and malformed controls. The vectors' static provenance is not original runtime proof; do not edit their expected results to conceal differences. |
| W4 — C57/N05/L56b/L14/L29 | Build original action groups for each Type17/18/19 option combination, export, re-import and inspect fields/options. Execute on copies; retain before/after tags, cover bytes/dimensions and playlist output. Separately test unknown options, codepages, empty values and Remove Fields Except with opaque/chapter frames. | Independently original-generated .mta records and original readback, not TagGroove exports fed back as reference. Include disabled-size/sentinel, legacy/opaque and unchanged-refusal controls. Cover pixel/quality evidence stays separate from option-schema compatibility. |
| W5 — L19/L20/L22a/L40/L47a/L47c/L49/L51/L54/L55/L56a/L73 | For each linked intake row, reproduce its named boundary in the original first: mismatched suffix, short/long positional import, companion route, whole-file chapter command, selection-triggered save, directory change, tool parameters/cap, invalid rename character, relative/shared target, one collision or Extended Tags Keep/Save/Cancel. Then repeat the identical action in TagGroove. | Exact input, settings, selected/unselected set, preview, confirmation decisions and disk bytes. Include cancel/no-match/no-op and unaffected-file controls. Do not turn an observed safe refusal into permission to overwrite, or add implicit writes before the reference policy is established. |
| W6 — L24/L25/L26/L28/L34/L35/L36/L38/L41/L42/L43/L77a/L77b | Obtain provenance-recorded original/WMP/Nero or real-encoder carriers for the exact named layout. Read; make one small edit on a copy; Save/Reload; retain format structure, audio and any observed Undo/Redo. For performance, run the same fixed corpus/settings on each named app/build and report scan/edit/save/cancel timings separately. | Actual writer/version/settings, full input/output hashes, native structural/audio observations and the precise supported/refused layout. Keep non-MP3/FLAC/M4A observations outside the paired collector's unsupported schema. Measure whole-file rewrite costs before proposing optimization or removing admission limits. |

### W5a: Inline paths and individually added files (L47a)

Use a generated a.wav beside an unrelated neighbor.wav. In the original, add
only a.wav, rename it outside the app and Refresh. Record whether the renamed
file and neighbor enter the list, the working directory, selection and order.
Repeat with a directory load, a playlist load and exact virtual chapter rows as
separate cases. Stage a disposable tag/artwork draft before a cancelled Refresh
and verify unchanged pending intent and media.

Separately edit both filename-column forms into a new and an existing child
folder. Record separator/absolute/parent-component handling, extension policy,
selection, pending intent, companion behavior and any observed Undo/Redo.
Include cancel/no-op and existing-target controls without overwriting a user
file. Compare those inline cases against the represented
[C64 literal-destination implementation](../verification/inline-filename-paths-2026-10-20.md);
its original Windows and physical native-input outcomes remain qualification.
Only discovering an externally renamed individually loaded file remains L47a
local implementation. Converter path documentation alone does not establish
inline behavior, and rescanning every parent must not silently replace playlist
membership.

### Local C59/C47 replay after final PR64 root classification

The [native workspace receipt](../verification/native-workspace-acceptance-2026-10-19.md)
retains historical observations at `7f29fd7914d6b62fa25bf8fc666d425fbcae08c7`
and `c169e9ee5f40418822eacf7cbf06e7ec3628bf9f`, with their original executable
hashes. Final PR64 head `550a1eed241fa05f711f211dabad65ef3a4913e3` changed loading,
workspace serialization and restoration through root classification. Current
merged acceptance is qualified only by the [focused replay](../verification/native-workspace-acceptance-2026-10-19.md#focused-merged-source-replay)
for suffixed-directory Refresh and manual/playlist-source-order normal restarts;
do not substitute older runs for the remaining cases.

For remaining acceptance, identify an isolated app from the current merged
production source. Exercise column-mode close/no-argument restart, exact
CUE/chapter IDs, unavailable-input retention/recovery, corrupt-snapshot retention
and native folder-picker recovery. Replay direct running-app replace/add and
relative paths containing spaces with pending-edit No/Yes. Include W5b's actual
folder-versus-playlist roots and reopen checks. Record source/executable hashes,
actual native input, argv/cwd, row/root/order/selection snapshots, profile bytes,
process exits and unchanged whole-media hashes. Keep actual quota refusal/retry,
queued arrivals during restoration and the optional forced-new-process route
explicitly pending until separately observed. A focused replay closes only the
named observed cases; C58 controls/geometry remain evidence for their older pins.

### W5b: Folder names that resemble playlists (represented root handling)

This is native qualification of the root-kind review repair, not another missing
playlist engine. Confirm the actual merged source contains the repair before
running this case. Keep Windows-original behavior separately versioned.

Create real folders named `archive.m3u`, `album.cue` and `unicode 名.xspf`, each
containing one disposable audio file and a nested subfolder. Load each folder
and Refresh: record ordinary folder scanning, exact rows, selected rows and
source/manual order. Separately load an actual playlist file with a supported
suffix and record playlist expansion/order. A directory suffix must not cause a
playlist-read error or imply playlist membership.

Configure a harmless local argument recorder as an external tool. For one
loaded folder, inspect `%_filename_rel%` and `%_folderpath_rel%`: their base is
that actual folder. For one actual playlist file, the base is its containing
folder. Preserve root-separator cases `/` and a Windows drive root. With Keep
moved files off, move a disposable file between nested folders inside the loaded
root, then outside it; verify the existing retained/dropped-row and pending-draft
rules. Record media bytes rather than assuming a changed list saved anything.

Close and reopen the saved workspace; repeat with a settings/workspace export
restored to a disposable mapped root. In a separate fixture, replace a playlist
file by a directory at the same suffixed path while the app is closed, and vice
versa. Record the current filesystem classification, Refresh and relative-tool
context; retained metadata must not disguise the changed filesystem kind.
Missing/unreadable roots must remain explicit errors, with no invented neighbors
or overwritten audio. Return source/package/OS identities and exact fixture,
arguments, rows and bytes for each case.

## W7: Windows TagGroove filesystem/history acceptance

Required: the inspected TagGroove build on native Windows, ordinary local storage
and a separately named share/filesystem for any network/unsupported-ID claim.
CI compilation/unit tests remain separate evidence from GUI journeys. Covers
C19/C54/C61/C62 and relevant L78 qualification.

1. On generated audio plus generated replaced/created side files, Save; close
   normally; verify a new process; reopen without file arguments; Undo; reopen
   again; Redo. Compare every audio and side file with independent before/saved
   byte images, and observe the menu and recovery notices on ordinary startup.
2. Before each replay direction, make an outside edit with unchanged length and
   restored modification time. Also test leaf replacement and parent-directory
   replacement. Require refusal without overwriting the outside bytes. Test a
   corrupted retained snapshot separately and retain its failure evidence.
3. Interrupt an owned disposable operation at a recorded phase, reopen and
   distinguish committed history from ambiguous recovery. Require unchanged
   current files, usable fresh history and verified-copy export to a new folder.
   Repeated export must neither overwrite originals nor consume recovery.
4. Check the unchanged aggregate 50-operation/2GiB policy and free-space reserve
   using an adequately sized disposable volume. Keep operation admission,
   clone/copy costs, eviction notices and recovery copies distinct. Cache removal
   is a separate loss boundary; do not call this an independent backup or a
   hardware-power-loss qualification.
5. Record native ID/change-time support, fallback/refusal and last-write timestamps
   after handles close. Repeat on a named network/unsupported-ID filesystem only
   when available. A successful local NTFS journey does not close those cases.

6. Exercise the four C58 toolbar arrangements through actual native controls,
   save/Cancel/reopen, filter docks and compact geometry. For C60, record a real
   region/custom-date-pattern/timezone transition while a disposable draft remains
   staged: loaded displays must update from unchanged RAW values without a Save
   or rescan. Repeat native focus/keyboard/IME/screen-reader cases under L78;
   record only cases actually observed.

Before the installed-app journeys, run the focused native filesystem regression
on the exact source/OS and retain its real fixture/log outcome:

```text
cargo test -p tag-core --test history_durable side_parent_identities_follow_owned_recreation_across_restarts -- --exact --nocapture
```

This checks repeated created-parent Undo/Redo across journal reopening. It does
not substitute for native GUI or network-filesystem acceptance. The
[directory recreation and rollback receipt](../verification/history-directory-identity-2026-10-19.md)
records the Linux-derived failure, controlled reuse vectors and the separate
real-files rollback/later-Redo case. State which controls actually run on the
engineer's OS; Unix-only fixture overrides are not Windows runtime evidence.

## W8: Windows Properties and native shell behavior

L04 requires an actual original and TagGroove Windows desktop. Select a generated
file, invoke the original Properties/Alt+Enter route and record which window,
fields and selected-file scope it opens. Repeat in TagGroove, whose current route
reveals files in the file manager. Check one/multiple selection, cancellation
and paths with spaces. Return the observed difference before choosing a Windows
policy; the already implemented Mac reveal route is not missing code to rebuild.
L51 .lnk/.bat launching also needs Windows: use harmless owned fixtures that write
only argv to a new test log, retain literal spaces/quotes and exit results, and
never infer shell expansion from placeholder substitution alone.

## W9: Source capability and preview reference observations

For L32/N03 and the original portion of L30/L31, use an owned source bundle and
record the exact original application/version, bundle hashes, declared minimum
version, includes/settings and configured options. Capture the actual search,
result selection, album-parser preview, cover routing and Save/Reload output on
owned disposable files. Retain authorized responses with secret values removed;
reproduce the same input/response contract in TagGroove and record its actual
parser/DTO/review result. An unrelated newer app or the Mac reference cannot
establish Windows3.36.1 command support. Existing parser/install/HTTP support is
represented implementation. Private-provider and configured-service journeys
that can run on this Mac remain local work, not Windows-only assignments.

## L1: Linux desktop acceptance

Required: an actual graphical Linux desktop, its named display/session and file
manager, the inspected package and a disposable profile/library. Xvfb synthetic
DOM suites remain headless evidence. Covers Linux portions of C47–C62/L46/L70/L78.

- Reopen saved local rows/order and eligible history through actual window close
  and new processes; use W7's independent replay/refusal/recovery checks on the
  named Linux filesystem. Retain native keyboard/pointer/focus/IME and screen-reader
  observations rather than inferring them from a backend pass.
- For L46, after the portable URI-encoding repair exists, drag cover output with
  spaces, percent signs, hash signs and non-ASCII names into the actual file
  manager/consumer. Verify the resolved file and bytes. The current ASCII
  restriction is known missing implementation, not a failed Linux test to waive.
- For L70, after association declarations exist, install the actual package,
  inspect its MIME/desktop argument declarations, open one and several files,
  then repeat with an already running app, paths with spaces and an upgrade.
  Record file membership/order and identities; manual opening is not installed
  association proof. Keep the portable declaration work local until implemented.
- Repeat the actual toolbar/filter/compact geometry, date-format transitions
  and pending-draft preservation named under C58–C60. Record actual locale,
  timezone and displayed versus RAW values; a formatter-only call is insufficient.

## A1: Physical Android provider qualification

Required: a named physical Android device, matching SDK/adb tooling, the exact
APK and owned disposable provider documents. Start from the bounded emulator
procedure and receipts in [Android document workflows](../verification/android-document-workflows.md).
The fixture project is crates/document-access/verification/provider-fixture.
The existing AOSP/API34 result does not establish physical/cloud-provider behavior.

1. Pick Unicode, writable and read-only documents; cancel selection while a draft
   exists. Observe actual permission/write state and unchanged pending edits.
2. Save/reload supported tags and embedded artwork. Export/read back provider
   bytes independently, compare audio and unselected files, then test edited-copy
   creation/cancel on a read-only source without overwriting it.
3. Use the owned fixture's documented conflict/partial-write controls. Retain
   provider event and byte logs; require observed conflict before a write-open,
   failed readback for a truncated result and intact baseline/edited recovery.
4. Revoke the test grant or make the test provider unavailable; force-stop through
   Settings, verify the old process ended, launch a new process and export retained
   original/edited copies. Repeat with a named real cloud provider using only owned
   test documents. Keep service/network failures distinct from a code difference.
5. Record native keyboard/Back, multiline input, orientation and system-bar theme.
   L18 action execution, L33 mobile secure credentials and L74 provider playlist/
   export/history remain portable implementation tasks until their covering PRs
   exist; read/copy acceptance cannot close them.

## I1: Physical iOS documents and signed IPA

Required: a named iPhone/iPad, macOS with the matching Xcode/device SDK, actual
provisioning/signing credentials and the inspected device-target artifact. The
owner requested a signed device IPA only; TestFlight upload is not part of this
assignment. A simulator bundle or static device library is not a signed IPA or
physical provider test.

L71 release integration and L72 picker/document/save-back implementation remain
local work. First verify their actual merged implementation/artifact; do not
reclassify that missing code as device-only acceptance. After it exists:

1. Inspect IPA/executable/platform/entitlements/signature/provisioning identities;
   install through the intended device route and retain that installation result.
2. Pick owned local and cloud-provider documents, cancel with pending edits, close
   and reopen the app, and verify actual retained access/security-scoped behavior.
3. Repeat supported read/edit/save/readback, read-only edited-copy and conflicting/
   unavailable-provider recovery with independent bytes and unchanged originals.
   Use the Android cases only as behavioral questions, not evidence of an iOS bridge.
4. Return native keyboard/orientation/background/permission-revocation observations
   with device/provider versions. Keep unimplemented actions, exports and provider
   Undo/Redo under their actual portable intake rows until implemented.

## P1: Installed package and release artifacts

L70/L71/L76 require the actual shipped artifact, version/hash/signature/source
bundle, installation and upgrade results on their named OS. Preserve configured
Windows MSI/NSIS/legacy identity; inspect fresh install, old OpenTag upgrade,
shortcuts, defaults and uninstall on disposable installations. Mac Developer ID/
notarization, available archive/license audits and Mac reproducibility remain
local to the available Mac. Signing configuration or a source library build is
not a published/installed signed package. Record actual artifact-linked notices
and corresponding-source/relinking coverage without inventing a license defect
from an incomplete inventory.

## Return one result per named case

Return the ledger/C-record ID, procedure/case ID, prerequisite implementation
commit, source/build/package/OS identities, settings, exact ordered actions,
expected contract, actual result and evidence filenames/hashes. Include refusal,
cancel and unaffected-file controls and the first failing input. Mark every
unrun prerequisite or case unobserved with its actual missing environment/input.
Keep credentials and private configuration values out of the shared package;
retain structural/redacted observations sufficient to reproduce the behavior.
Do not hand over user libraries or vendor runtime trees as test fixtures.
