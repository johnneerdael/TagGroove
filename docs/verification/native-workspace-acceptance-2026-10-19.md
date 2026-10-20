# Native local workspace acceptance — 2026-10-19

This historical evidence records named C58 toolbar and C59/C47 workspace and
launch journeys on the exact earlier builds below. Final PR64 head
`550a1eed241fa05f711f211dabad65ef3a4913e3`, merged as
`e4017801dd923b65e1e5e2fc0ece58706374b60d`, subsequently changed loading,
workspace serialization and restoration through root classification. These runs
do not qualify those changed paths. The focused merged-source replay below
qualifies only suffixed-directory load/Refresh and manual/playlist-source-order
normal restarts. Current running-instance CLI, column mode, exact CUE/chapter
identities, missing/corrupt recovery, quota retry and queued arrivals remain
pending. C58 retains only the observed older-build controls and geometry; no latest-build toolbar
result is claimed. C60 live OS transitions and untested keyboard/IME/accessibility
cases remain locally pending.

Authoring host checked on 2026-10-19: macOS 15.8, arm64. Each evidence root
retains its own source and executable identity; earlier runs keep those pins.

The subsequent [path-alias repair](source-root-aliases-2026-10-20.md) changes
loading and restoration again. The replay below remains evidence for its
explicit `e4017801dd923b65e1e5e2fc0ece58706374b60d` production baseline; it is
not native proof of the alias-aware implementation. Local UI replay is not
repeated for that repair; postmerge native CI is the next integration layer.

<a id="focused-merged-source-replay"></a>

## Focused merged-source replay — 2026-10-20

The isolated debug app at documentation head
`9eb0d57517fd4be822588c2f1547952c664a7df1` has production code matching merged
`e4017801dd923b65e1e5e2fc0ece58706374b60d`. Its identifier is
`org.taggroove.editor.e2e.pr65o27rzso3`; executable SHA-256 is
`52bb13824092bdaff41f63e758325ae4b107d7d9c8bfd620facea0cdd095eb6e`.

Native CUA F5 on the loaded real directory `archive.m3u` retains its one row,
reports a successful Refresh and records `root_kinds: ["directory"]`. A separate
process loads the actual playlist in C,A,B source order; native Alt+PageUp moves
B to produce B,C,A, then normal window close exits 0. A no-argument restart
restores manual B,C,A; native View → Sort by → Playlist order shows C,A,B, then normal
close exits 0. The next no-argument restart retains C,A,B in playlist mode and
closes normally. All four processes exit 0; all 13 fixture hashes are unchanged.

Retained root:
`/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-pr65-merged-native-o27rzso3`.
`focused-acceptance.json`, `context.json` and per-phase logs retain the bindings
and observations. This subset establishes no current running-instance CLI,
column-mode, CUE/embedded-chapter, missing/corrupt recovery, quota or queued-arrival
result. Those cases, broader native input and the optional forced-new-process
route remain pending. No C58 toolbar replay was performed.

## Historical corrupt saved-list recovery

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

## Historical ordinary workspace journeys

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

## Historical native C58 toolbar subset

On the same historical `c169e9ee5f40418822eacf7cbf06e7ec3628bf9f` app, add one
Open folder command through the actual custom-toolbar dialog so both bands are visible. Native pointer and
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

## Historical native compact geometry subset

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

## Historical fresh compact launch control

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

## Historical embedded chapter restart

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
