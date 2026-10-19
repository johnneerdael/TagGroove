# Native local workspace acceptance — 2026-10-19

Prepared for the small follow-up after PR64 merges. This evidence covers the
named C58 toolbar and C59 workspace journeys. C60 live OS transitions, quota
retry, queued launches and untested keyboard/IME/accessibility cases remain
locally pending; the named results do not close whole feature areas.

Authoring host checked on 2026-10-19: macOS 15.8, arm64. Each evidence root
retains its own source and executable identity; earlier runs keep those pins.

## Corrupt saved-list recovery

The isolated native macOS debug app uses identifier
`org.taggroove.editor.e2e.cirepairbe506642`, executable SHA-256
`9a8a79d0adaf137d40d63c9b1426cc130c2b9fd43e939f3993d174816c9823b3`.
Its production source hashes match PR64 head
`c169e9ee5f40418822eacf7cbf06e7ec3628bf9f`. The build context records the prior
published head plus uncommitted Rust repair at build time. It is an ad-hoc-signed
debug app, not an installed release or a Windows-original comparison.

1. On two generated MP3 copies, wait for an ordinary valid saved workspace.
   Inject exactly `{"version":` into this disposable profile's workspace key
   through the existing debug hook. Native CUA clicks close the window normally.
2. Start a new process without file arguments. Observe zero rows and the visible
   corrupt-list error. The malformed storage text remains exact. Dismiss the
   error through native input and close normally; the dismissal render leaves
   the malformed text unchanged.
3. Start another process without arguments. Observe the same error and exact
   malformed text. Use the real Open folder button and macOS folder picker to
   choose only the owned generated directory. Observe two rows, a valid exact
   workspace snapshot and no remaining error.
4. Close normally; start another process with no file arguments. Observe the
   exact recovered two paths and a successful restored-workspace status.

All four runner-controlled processes exit 0 and the two MP3 hashes remain
unchanged in every phase. The app is closed; its disposable profiles are retained
alongside the full logs and fixtures. One AX observation after the seed close
implicitly reopened an additional process; it displayed the corrupt error and
was closed before the explicit runner-controlled restarts. This incidental
observation is not substituted for their PID/log/result records.

Retained root: `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-native-corrupt-workspace-ln3yrvu9`.
`context.json`, `acceptance.json`, each phase's driver/command/input hashes,
stdout/stderr/results and `profiles-after/` supply the evidence. The hook mutates
storage only to seed the malformed fixture; native buttons/picker/close actions
perform recovery. Later observations are read-only.

## Earlier ordinary workspace journeys

The earlier native app uses source commit
`7f29fd7914d6b62fa25bf8fc666d425fbcae08c7` and executable SHA-256
`9f6c5626ede3430553c1f4365b91f981eb8cc79804ee26aee340cbafb34068ea`.
Its retained root is
`/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-native-workspace-acceptance-ebi8twho`.
Actual native pointer/keyboard and process restarts cover:

| Receipt | Named result |
| --- | --- |
| `acceptance.json` | Alt+PageUp changes manual order; two normal closes and subsequent no-argument processes retain the changed order. The first relaunch was triggered by CUA bundle observation; the later supervised restart records exact no-argument argv. Running-app `/fn:` replaces and `/add` appends. |
| `order-modes.json` | Separate source, manual and column order survive mode changes and ordinary restarts without overwriting their independent ranks. |
| `cue-restart.json` | Two exact CUE virtual IDs survive a normal close and no-argument restart. |
| `missing-cue-recovery.json` | Temporarily unavailable CUE input retains its saved snapshot through native renders and close; restoring the exact original bytes restores the same virtual IDs. |
| `relative-cli-pending-edits.json` | Relative paths containing spaces resolve against the second process's working directory. Native No keeps the current draft; `/add` retains it and adds rows; native Yes discards the draft and replaces the list. |

Generated media are unchanged; the missing CUE is restored byte-for-byte.
All test apps are closed. These older runs keep their own source/executable
pins; the newer build is not substituted for them. The initial acceptance
JSON's pending list is its original checkpoint: the later named JSONs record
subsequently executed cases without altering that historical record.

## Native C58 toolbar subset

On the same source-bound newer app, add one Open folder command through the
actual custom-toolbar dialog so both bands are visible. Native pointer and
keyboard controls select and save all four arrangements; screenshots show
main/custom order on shared and stacked rows. Reopen verifies the saved choice;
select a different choice and Cancel, then reopen verifies the saved choice is
unchanged. Saving Custom above main survives normal close and a new process.

Use native View controls for list/right/left filter docking. Type sine-b and
press Enter: only sine-b remains visible. Native F3 hides the filter and shows
both rows; F3 again retains the query. Move the active filter between docks,
then return to list docking and clear it: both rows remain available. Both
runner processes exit 0 and all generated audio bytes remain unchanged.

`toolbar-acceptance.json`, `toolbar-native/`, `toolbar-restart/` and
`profiles-toolbar-after/` retain the observations and identities. The native
CUA conversation contains the inspected screenshots. Normal-sized geometry was
observed. The compact sections below record separate geometry/input controls;
VoiceOver and other OS acceptance remain pending.
Shared-row bands divide the available width and main commands wrap within their
band at this window size; the observation does not certify optimal space use.

## Native compact geometry subset

The macOS native Top Left sizing command produced an 853×533 compact viewport;
Return to Previous Size restored 1240×772. `compact-acceptance.json` records the
observed transitions and unchanged media. The screenshot shows stacked custom/
main bands, Files/Tags controls, three list columns and list-docked filter. This
is geometry evidence only: Tags-tab AX input did not visibly switch panes.

Subsequent native close/menu observations stalled. A retained process sample
shows AppKit accessibility NSMenu tracking; no frontend-loop defect is established.
Escape and normal Quit were requested; the process subsequently exited 0. No
SIGTERM was sent: the final pre-signal check found it already terminal. The next section records an independently launched compact control. Preserve
this earlier observation as an unclosed accessibility/menu case; it does not
establish an application resize defect.

## Fresh compact launch control

An independently built 900×800 test configuration uses the same clean production
source at `c169e9ee5f40418822eacf7cbf06e7ec3628bf9f` and fresh identifier
`org.taggroove.editor.e2e.compact1aaedd3d`. Its executable SHA-256 is
`ba52e121119bb598c26f30ffbf39cbb2707a67d16de593325bd98fcbfb5b6041`.
The actual WebView width remains 900px. Native pointer input activates Tags and
shows Sine A, returns to Files, selects the second row and shows Sine B. Native
window close exits 0. A new process with no file arguments restores both paths
and the second-row selection; another native close exits 0. Generated MP3 hashes
remain unchanged. The retained context binds every production source hash.

Root: `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-native-compact-direct-mxgdp130`;
`acceptance.json`, per-process driver/identity/logs/results and copied test
profiles retain the observations. Runtime logs prove pane changes and saved
selection; native CUA AX states contain editor input values, which the driver's
body.innerText measurement excludes. This passing fresh-window control does
not erase the earlier AX-driven AppKit menu-tracking observation or establish
VoiceOver/original-application equivalence.

## Embedded chapter restart

On the retained 9a8a79d0 source-bound app, enable List chapters only in the owned
profile before launch. Open generated M4B, Matroska and MP3 copies: two virtual
rows per chapter carrier and one physical MP3 yield five exact IDs. Native
pointer input selects Matroska chapter 2; normal window close exits 0. Disable
automatic List chapters in the same owned profile while closed, then launch a
new process without file arguments. All five exact IDs, source order and selected
Matroska chapter 2/anchor restore with no error. A second native close exits 0;
all three carrier hashes remain unchanged. `chapter-acceptance.json`,
`chapters-first/`, `chapters-restart/`, settings and fixture identity records are
retained under taggroove-native-corrupt-workspace-ln3yrvu9. This qualifies the
named ordinary-restart case, not arbitrary chapter save/playlist compatibility.
