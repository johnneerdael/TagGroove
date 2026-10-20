# Mp3tag 3.36.1: uncovered parity work

For the next work sequence, use the [user-impact, effort and environment table](handover/PARITY-PRIORITIES.md). PR64 has merged; remaining implementation belongs in small new PRs, selected by importance.

This is the single actionable **uncovered** list: missing behavior and unresolved semantic research for which the inspected open or merged PRs provide no covering implementation. Outstanding verification of implemented behavior belongs in the [qualification register](research/remaining-gaps.md#qualification-of-implemented-scopes), outside this intake. Represented work is excluded even while its PR is open; exclusion is not a completion claim. The [coverage companion](research/remaining-gaps.md) records delivered work, inspected PR scopes, historical row IDs, evidence and intentional limits. The [feature inventory](FEATURE-INVENTORY.md) retains the full acceptance contract.

Reference: Windows x64 Mp3tag 3.36.1, executable SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`. Inspected merged baseline: main `69cd9f063c66290adbb6b3db3a70b2d5a1ca4592`, after merged PRs49–67 and the owner-authorized native source-download repair. Intake refreshed **2026-10-20**. PR64, PR66 and PR67 review/CI/merge identities are recorded in the [coverage companion](research/remaining-gaps.md); packaging, publication and native acceptance remain separate claims.

The [implementation checkpoint table](research/remaining-gaps.md#pr64-implementation-checkpoints) identifies the exact commits and bounded evidence behind represented work. Planned checklists, local checks and an open PR do not establish delivery or original/native acceptance. See the [coverage and refresh rules](research/remaining-gaps.md#refresh-and-evidence-rules).

L01–L79 preserve the external roadmap register identities; suffixes split partial rows and N01–N06 add uncovered acceptance from existing verification notes. These are work-item identifiers, not feature counts or a parity percentage. Each evidence link records the source/verification basis and why no inspected PR covers the stated remainder. Unknown original behavior calls for observation before selecting an implementation.

Implemented features leave this intake even when their native/original acceptance is still outstanding. Those limits stay in the qualification register and under the covering C records in the [coverage companion](research/remaining-gaps.md#represented-and-delivered-scopes); they are not duplicate implementation tasks.

The follow-up [C25/C59 input-alias repair](verification/source-root-aliases-2026-10-20.md)
preserves folder membership and Refresh roots across known in-app renames.
Its focused headless evidence does not close L47a or requalify older native
workspace receipts against the changed loading/restoration code. That checkpoint
left 49 active IDs; the C63 follow-up below now represents L54.

C51–C54 represent the scoped scripting/export, Lyrics3v2, playlist-filename and
history-startup repairs in PR64. Historical L09/L45a anchors now point to their
covering records; L10 error behavior and L38 Ape/Mpc display remain separate
qualification records. C62 now represents cross-session history and N04 retains
column/self-reference semantic research. The [repair evidence ledger](verification/four-parity-repairs-2026-10-19.md)
records focused results and their limits; these are not full parity or release proof.

C55/C56 now cover the native-derived filter-quoting and Split Field publication
repairs. Their original-runtime acceptance remains with those records; these
implemented behaviors have left active intake. See the [local closeout evidence](verification/local-parity-closeout-2026-10-19.md)
and [OS/device handover register](handover/PARITY-OTHER-MACHINES.md).

C58–C60 represent accessible toolbar-band placement, ordinary saved-only list
restart and loaded-row regional-date refresh. The [native workspace receipt](verification/native-workspace-acceptance-2026-10-19.md)
retains historical Mac toolbar/filter, bounded compact, exact-row/restart,
missing/corrupt-input recovery and explicit running-app command-line cases at
their original source/executable pins. Final PR64 changed loading and workspace
serialization/restoration through root classification after those runs. The [focused merged-source replay](verification/native-workspace-acceptance-2026-10-19.md#focused-merged-source-replay) records suffixed-directory Refresh and manual/playlist-source-order
normal restarts only. Current running-instance CLI, column mode, CUE/chapter
identities, missing/corrupt recovery, quota retry and queued arrivals remain pending.
C58 evidence is limited to the older builds; broader native input and live C60
OS transitions remain local work. Windows/Linux and original
acceptance remain separate; use the [OS procedures](handover/PARITY-VERIFICATION-PROCEDURES.md).
The [implementation receipt](verification/daily-workflows-2026-10-19.md) retains
its original source-bound controls and limits.

C57 covers the canonical native action-file schemas and bounded source-preserving
export compatibility. Original generated-file/readback and native acceptance remain
with [its implementation record](research/remaining-gaps.md#c57).

C47 also covers the implemented and documented direct-executable route for
arguments sent to a running macOS app. Historical L69 now retains only actual
macOS runtime qualification under that record; it is not missing launch code.
The native receipt above retains historical explicit replace/add, relative paths
with spaces and pending-edit decisions. Replay against the current merged
loading/restoration paths, queued arrival during restoration and the separately
documented forced-new-process route remain unqualified.

C61 covers full-content history and staged-write checks, including concealed external edits and corrupted snapshots. A bounded native Mac Save/Undo/Redo journey passes; broader native/Windows qualifications remain under [the represented scope](research/remaining-gaps.md#c61). C62 now covers persistent history, interruption recovery and verified-copy export; see [the durable-history receipt](verification/durable-history-2026-10-19.md).

Windows-specific work and original Windows captures are **deferred by the owner**. Retain those requirements as unqualified; continue the cross-platform work without treating deferral as completion.

C63 now represents [edited Windows action-filename protection](verification/action-filename-protection-2026-10-20.md), including source-backed cleanup and portable regressions. L54 leaves this intake; exact original Windows/runtime qualification remains with C63 and L79. C64 additionally represents [literal inline filename/folder destinations](verification/inline-filename-paths-2026-10-20.md); L47a retains only external-rename discovery on Refresh. There are **48 active IDs** and **14 separate qualification identities**.

## Uncovered work

| ID | Area | Actionable remainder | Evidence |
| --- | --- | --- | --- |
| L03 | Menus | Settle the desktop File → Exit workflow; there is no File-menu item, while macOS has app-menu Quit. | [Scope and evidence](research/remaining-gaps.md#l03) |
| L04 | Menus | Qualify Windows Properties versus the current Alt+Enter file-manager reveal route and choose the platform behavior from observed reference evidence. | [Scope and evidence](research/remaining-gaps.md#l04) |
| L05 | Tag panel | Qualify a separate OS-window floating panel; the existing undocked panel is an in-window overlay. | [Scope and evidence](research/remaining-gaps.md#l05) |
| L07 | Migration | Determine unsupported/undocumented columns.ini keys from populated original profiles; guided import does not extend the existing known-key parser. | [Scope and evidence](research/remaining-gaps.md#l07) |
| L13 | Covers | Establish Original-format fallback for TIFF/unknown input; current Adjust Cover requires JPEG or PNG for these inputs. | [Scope and evidence](research/remaining-gaps.md#l13) |
| L14 | Actions | Observe original Remove Fields Except behavior for unmapped PRIV/GEOB/chapter frames before changing their retention. | [Scope and evidence](research/remaining-gaps.md#l14) |
| L18 | Mobile actions | Implement provider execution for in-place tag/embedded-artwork action groups, plus file, cover-file and rename actions. Only in-place validation exists; the managed command still rejects all action groups. Qualify actual device save-back separately. | [Scope and evidence](research/remaining-gaps.md#l18) |
| L19 | Converters | Compare a nonmatching explicit Filename → Filename suffix: song.mp3 plus %1.flac currently becomes song.flac.mp3. Matching suffixes are already deduplicated ignoring case. | [Scope and evidence](research/remaining-gaps.md#l19) |
| L20 | Converters | Compare positional Text File → Tag count-mismatch policy with the original; positional import still requires one record per selected file. Keyed lists already skip unselected records and leave unlisted files unchanged, so do not implement that support again. | [Scope and evidence](research/remaining-gaps.md#l20) |
| L22a | Associated files | Determine original companion behavior for Copy, Filename → Filename, inline edits and action-driven renames, then extend only confirmed gaps; the merged opt-in flow covers Tag → Filename and desktop Move. | [Scope and evidence](research/remaining-gaps.md#l22a) |
| L24 | Formats | Qualify/edit Nero udta/indx and cover placement; extend required chapterless rewrites beyond currently guarded layouts and qualify original/native behavior. Supported mirror growth/shrink and atomic refusal are represented by C34. | [Scope and evidence](research/remaining-gaps.md#l24) |
| L25 | Chapters | Expose/qualify QuickTime href URLs and CHAP artwork, and establish original subsong playlist paths; chapter-row playlist operations currently refuse. | [Scope and evidence](research/remaining-gaps.md#l25) |
| L26 | Formats | Qualify broader ADTS probing, combined global-unsync/optional-header transforms, update/restriction declarations and arbitrary compressed/encrypted/grouped retention. Merged support also preserves the named ordinary v2.4 frame-unsynchronisation cases; ADTS junk scan remains .aac-only, bounded to 64 KiB and three frames. | [Scope and evidence](research/remaining-gaps.md#l26) |
| L32 | Source capabilities | Qualify capability coverage beyond compiled parser commands and map declared original-version features without comparing unrelated app versions. Merged installation/include updates already compile supported grammar before replacement; high minimum declarations remain informational. | [Scope and evidence](research/remaining-gaps.md#l32) |
| L33 | Mobile credentials | Provide and qualify mobile secure credential storage for the Discogs token. | [Scope and evidence](research/remaining-gaps.md#l33) |
| L34 | Native fields | Qualify original UFID owner/identifier grammar (64-byte identifier, one per owner), binary retention, and USLT language/description grouping; observation of retained groups does not certify original edit syntax. | [Scope and evidence](research/remaining-gaps.md#l34) |
| L35 | Matroska | Settle TRACK 3/12 versus TOTAL_PARTS, Segment Info title editing, full-SeekHead rewrite support, cross-segment chapters and ordered editions; qualify original/native behavior and broader layouts. C37 covers local unindexed round trips and unchanged no-room refusal. | [Scope and evidence](research/remaining-gaps.md#l35) |
| L36 | ASF/WMA | Qualify headers above the current 512 MiB refusal and whole-file write costs. C43 covers admitted raw text/name/Content-slot preservation; that repair does not remove the size or rewrite-cost limits. | [Scope and evidence](research/remaining-gaps.md#l36) |
| L40 | Chapters | Confirm %_subsong% base, Nero field-name case and original/native whole-file command behavior. Existing Remove tag and Strip chapters MP4 target the containing file once for embedded chapter rows; CUE rows remain refused. | [Scope and evidence](research/remaining-gaps.md#l40) |
| L41 | Utils | Qualify fragmented MP4, stco→co64 growth and FLAC with leading ID3v2 before extending Optimize; assess Remove Unsupported Fields beyond ID3v2 on MP3/AAC/AIFF/WAV. | [Scope and evidence](research/remaining-gaps.md#l41) |
| L43 | Technical fields | Complete/qualify Musepack/WavPack %_vbr% and tag sizes for WAV, AIFF, MP4, Ogg, Matroska, ASF and DSD containers. | [Scope and evidence](research/remaining-gaps.md#l43) |
| L46 | Covers | Qualify Linux drag-out and percent-encode file URIs before relaxing the current ASCII temporary-name restriction. | [Scope and evidence](research/remaining-gaps.md#l46) |
| L47a | File list | Discover an externally renamed individually loaded file during Refresh without guessing a neighbor or loading unrelated files. | [Scope and evidence](research/remaining-gaps.md#l47a) |
| L47c | Saving | Compare save-on-move policy for editor Enter/Tab, Find, Select all and Select directory; these routes do not currently trigger it. | [Scope and evidence](research/remaining-gaps.md#l47c) |
| L48 | File operations | Qualify temporary-playlist playback and a native folder-picker subfolder choice; Play currently opens each file separately and the native picker has no subfolders checkbox. | [Scope and evidence](research/remaining-gaps.md#l48) |
| L49 | Tag panel | Qualify the dropdown changing the working directory and format-string fields; the dropdown currently selects rows. Typed-directory Enter and automatic multi-selection defaults are already implemented. | [Scope and evidence](research/remaining-gaps.md#l49) |
| L50 | Toolbar layouts | Implement/qualify toolbar.ini/toolbar_custom.ini layout import and user symbol choice, using a real migration profile. Individual built-in source commands are already available. | [Scope and evidence](research/remaining-gaps.md#l50) |
| L51 | Tools | Settle original non-path placeholder support, .lnk/.bat launch behavior and the 100-file per-file launch cap. | [Scope and evidence](research/remaining-gaps.md#l51) |
| L52 | Legacy sources | Qualify a fetched freedb server list, non-gnudb artist/album/artwork search and non-exact local lookup; current refresh restores two built-ins, network search is gnudb-only and local lookup is exact. | [Scope and evidence](research/remaining-gaps.md#l52) |
| L55 | Converters | Compare relative-target resolution, two partitions sharing one output file (currently refused), and slash folder markers in the old Filename → Filename pattern. | [Scope and evidence](research/remaining-gaps.md#l55) |
| L56a | Converters | Compare all-or-nothing Filename → Filename reviewed-plan clash handling; one clash currently refuses the complete plan. | [Scope and evidence](research/remaining-gaps.md#l56a) |
| L56b | Actions | Qualify EBCDIC/ISO-2022 and explicit Import Text File codepages, UTF-8-valid ANSI interpretation, quality preset numbers and default field lists; current supported choices are TagGroove policy. | [Scope and evidence](research/remaining-gaps.md#l56b) |
| L57b | Migration | Establish semantics and implement confirmed mappings for unsupported original profile settings, including global/binary data and omitted tag-panel custom_values/prefer_tag options. Supported import/restart/restore acceptance remains under C12/C27. | [Scope and evidence](research/remaining-gaps.md#l57b) |
| L58 | Updates | Provide/qualify downloading and installation of updates; the current check only offers the GitHub release. | [Scope and evidence](research/remaining-gaps.md#l58) |
| L63 | Languages | Add independently authored, demand-led catalogs beyond English/Dutch/German and qualify complete localized journeys, Unicode/input and layout; original bundled count is 43, not an acceptance score. | [Scope and evidence](research/remaining-gaps.md#l63) |
| L64 | Networking | Implement/qualify needed proxy authentication, SOCKS and PAC behavior while keeping credentials out of ordinary configuration backups. | [Scope and evidence](research/remaining-gaps.md#l64) |
| L65 | Settings research | Establish meaning/equivalents for ConvFTFShowDlg, ConvFTTShowDlg, ConvTFFShowDlg, ConvTTFShowDlg, ConvReplaceShowDlg, ToolOverwriteDefault, CoverPopup and HideID3Bar; binary strings alone do not specify behavior. | [Scope and evidence](research/remaining-gaps.md#l65) |
| L66 | Settings research | Determine whether ID3LeadingZeros denotes a global switch beyond the implemented numbering-wizard option. | [Scope and evidence](research/remaining-gaps.md#l66) |
| L70 | File opening | Add Windows/Linux association declarations and qualify installed file opening and upgrade continuity; stable Windows installer identity is already configured. | [Scope and evidence](research/remaining-gaps.md#l70) |
| L71 | Distribution | Qualify Windows signing, macOS signing/notarization and physical iOS distribution; current release packages are unsigned on Windows, ad-hoc on macOS and simulator-only on iOS. Physical iOS native-library compilation is implemented; it supplies no signed IPA, notarization or device-runtime evidence. | [Scope and evidence](research/remaining-gaps.md#l71) |
| L72 | iOS documents | Implement/qualify provider picker, document access and save-back; runtime document access currently enables Android only. | [Scope and evidence](research/remaining-gaps.md#l72) |
| L73 | Extended Tags | Settle staged Keep changes versus disk Save with native input/navigation/close/cancel; immediate-save currently applies only when moving between files. | [Scope and evidence](research/remaining-gaps.md#l73) |
| L74 | Provider documents | Implement/qualify provider playlist loading/creation, CSV/export and Undo/Redo. Read-policy implementation and pending-artwork copy are represented separately; their PRs do not enable these operations. | [Scope and evidence](research/remaining-gaps.md#l74) |
| L76 | Distribution | Complete binary-specific, JavaScript and native-library license/notice audits, other released-package checks and reproducible builds. The v0.2.46 Rust-source inventory is hash-verified; C36 separately covers the Boost notice repair. | [Scope and evidence](research/remaining-gaps.md#l76) |
| L77b | Performance | Measure and improve whole-file rewrite costs for APE, RIFF, Ogg and other remaining containers. | [Scope and evidence](research/remaining-gaps.md#l77b) |
| N03 | Source previews | Establish original album-parser preview output grammar/precedence before implementing any confirmed missing route. C21 covers the separate result full-cover action and retains its native/provider acceptance. | [Scope and evidence](research/remaining-gaps.md#n03) |
| N04 | Automation contexts | Establish remaining column-function and self-reference semantics before implementing any confirmed missing behavior. C22 covers mutation/displayed/list contexts; C51 covers sequential evaluator flags and string-only export variables. | [Scope and evidence](research/remaining-gaps.md#n04) |
| N05 | Action interchange | Capture original-generated recognized-action option matrices and unknown-option samples, then original re-import/readback; Merged PR58 preserves decoded opaque values and order, including backslash/control-character boundaries; it supplies no original readback. | [Scope and evidence](research/remaining-gaps.md#n05) |
