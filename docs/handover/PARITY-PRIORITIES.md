# Remaining parity: user impact and priority

Planning snapshot refreshed 2026-10-20, after merged PR68 and the earlier native
source-download repair. L67 is represented by [C62](../research/remaining-gaps.md#c62).
C63 represents L54, and C64 now represents literal inline destinations. This table covers all **48 current intake
IDs**, assigned to later PRs. It is not a feature count, parity percentage,
claim of a defect, or completion certificate. PR64 review, CI and merge are recorded in the [integration evidence](../research/remaining-gaps.md#pr64-integration-evidence).

The owner changed the sequence: **finish L67, freeze and merge PR64 after its
required review/CI, then use a new PR for further ledger implementation**. PR64
has merged; the remaining local implementation now follows this priority table.
C63 represents L54 and C64 represents inline destinations; their original Windows/native qualifications remain open. L47a retains external-rename discovery on Refresh.
The current [intake](../PARITY.md), [scope/evidence companion](../research/remaining-gaps.md)
and [OS handover](PARITY-OTHER-MACHINES.md) remain authoritative. Implemented work
must not be rebuilt because an original/native qualification is still open.

The intake currently contains **11 P1**, **23 P2** and **14 P3** scopes, using
the highest listed priority for mixed rows. P1 and P2 therefore cover **34**
important open scopes. These include compatibility research and audits as well
as implementation work. The 14 qualification identities below are separate.

Unmerged P1 batch work is recorded separately from these main-based counts:
[L47a/L73 implementation and focused checks](../verification/file-tracking-and-extended-save-2026-10-20.md)
and the [L07 native column schema/mapping audit](../verification/mp3tag-column-schema-2026-10-20.md).
Their intake identities remain here until the batch has a covering PR.

[L57b field-value migration](../verification/mp3tag-field-values-migration-2026-10-20.md)
now has a separately verified working-branch implementation for legacy panel
values and `usrfieldvalues.ini`. General global/binary settings remain open;
the full L57b identity has not been closed by this slice. A further working-branch
[binary settings slice](../verification/mp3tag-binary-settings-migration-2026-10-20.md)
imports 18 confirmed boolean options, the Extended Tags save mode and two
playlist text options;
remaining mappings still need investigation.

The 48 intake rows primarily comprise **12 missing-capability scopes**,
**19 limitations in existing behavior**, **15 unresolved compatibility questions**
and **2 measurement/audit scopes**. These are primary planning categories: a
limited feature can also need original-behavior research. The separate 14
qualification identities below are not missing-feature assignments.

## How to read the priorities

- **P1:** daily correctness/save expectations, high-impact migration/source
  compatibility, or desktop release trust. An R item gets investigation first,
  not an invented implementation.
- **P2:** important advanced workflows or bounded compatibility work, sequenced
  by actual affected libraries and dependencies.
- **P3:** optional expansion, specialized formats, or convenience with an existing
  workable route. This includes mobile expansion and new catalogs until desktop
  priorities are satisfied.
- **No P0 is established by this review.** A guarded unsupported-format refusal
  is not a corruption bug. A failed Windows-invalid rename is not proof of data loss.

Reach describes the workflow, not measured user adoption. Importance incorporates
the owner's stated use of tag sources/freedb and preference for substantive desktop
features over translations; source work is therefore not relegated solely because
it is an advanced workflow. Signing and packaging are assessed from recorded
release evidence; inspect the latest artifacts before writing new integration.

Effort is provisional **engineering effort**, including focused code/tests/docs,
not a calendar promise: **S** up to 1 day; **M** 1–3 days; **L** 3–7 days; **XL** more
than 1 week. **R** means the contract/corpus must be established before implementation
can be sized. Confidence reflects the estimate/evidence, not reference equivalence.
Ranges are not additive: rows share infrastructure and some are research only.
The earlier 4–8 engineer-week estimate is a broad planning allowance for the whole
local program, not a commitment or a reason to hold the current PR open.

Local means code, research or native acceptance possible on this Mac. Windows
3.36.1 reference execution stays deferred; Mac Mp3tag 1.14.1 is a separately
versioned comparison, never a substitute. Portable code is not handed off merely
because its eventual device/OS acceptance is external.

A [CSV version of the intake table](PARITY-PRIORITIES.csv) is available for sorting
and filtering.

## Intake assessment

Each ID links to its current source paths, bounded verification and remaining
contract. “Unknown” rows can produce a documented equivalent or deliberate
platform policy instead of new code, but need actual evidence before closure.

| ID | Priority | Work type | Affected users | What a migrating Mp3tag user encounters | Local effort / confidence | Environment and dependency |
| --- | --- | --- | --- | --- | --- | --- |
| [L03](../research/remaining-gaps.md#l03) | P3 | Missing convenience | Daily | No File → Exit item; Mac app-menu Quit already works. | S / high | Mac code and close checks; Windows/Linux acceptance |
| [L04](../research/remaining-gaps.md#l04) | P3 | Unknown compatibility | Windows users | Alt+Enter reveals files; Explorer Properties equivalence has not been observed. | R / high | Windows reference only; deferred |
| [L05](../research/remaining-gaps.md#l05) | P3 | Limited feature | Multi-monitor editors | The floating panel stays inside the app and cannot occupy a separate OS window. | XL / medium | Mac multiwindow code and native checks; other desktops later |
| [L07](../research/remaining-gaps.md#l07) | P1 | Unknown migration mapping | Migrating customized profiles | Undocumented columns.ini keys may import incompletely; known-key import already exists. | R / medium | Mac profile/static analysis; original populated profiles and Windows readback |
| [L13](../research/remaining-gaps.md#l13) | P2 | Limited feature | Unusual artwork inputs | Adjust Cover Original refuses TIFF/unknown formats unless JPEG/PNG is selected. | R / medium | Mac codec/fallback analysis; reference output comparison |
| [L14](../research/remaining-gaps.md#l14) | P2 | Unknown compatibility | Advanced cleanup actions | Remove Fields Except may retain hidden frames; the original deletion policy is unresolved. | R / medium | Mac frame analysis; Windows reference writes |
| [L18](../research/remaining-gaps.md#l18) | P3 | Missing mobile capability | Mobile action users | Provider documents cannot execute action groups; a validator is not an execution engine. | XL / medium | Mac portable host/UI; Android/iOS device save-back |
| [L19](../research/remaining-gaps.md#l19) | P2 | Unknown compatibility | Converter users | A different explicit suffix can produce song.flac.mp3; original suffix policy is unqualified. | R / high | Mac suffix analysis; Windows reference preview/apply |
| [L20](../research/remaining-gaps.md#l20) | P2 | Limited feature; contract unknown | Text-import users | Positional record-count mismatch refuses; keyed skip/unlisted-file support already works. | R / medium | Mac import analysis; Windows reference pairing policy |
| [L22a](../research/remaining-gaps.md#l22a) | P1 | Limited feature; contract unknown | Libraries with lyrics/sidecars | Some copy/rename routes leave companions behind; only Tag → Filename and desktop Move share the existing flow. | R / medium | Mac route analysis/code; reference and cross-volume checks |
| [L24](../research/remaining-gaps.md#l24) | P2 | Limited guarded format | Nero/MP4 chapter libraries | Some indx/cover layouts and rewrites are unsupported; guarded refusals preserve originals. | R → L–XL / low | Mac atom/rewrite work; original and player acceptance |
| [L25](../research/remaining-gaps.md#l25) | P3 | Limited chapter capability | Chapter/audiobook libraries | Chapter URLs/pictures are not exposed and chapter-row playlist commands refuse. | R → L / medium | Mac metadata/UI; original playlist serialization and players |
| [L26](../research/remaining-gaps.md#l26) | P2 | Limited guarded format | Unusual ID3/ADTS libraries | Broader envelope/frame transformations and ADTS probing remain unsupported; ordinary named unsync cases work. | R → L–XL / low | Mac fixtures/parser/writer; original generated carriers |
| [L32](../research/remaining-gaps.md#l32) | P1 | Unknown source compatibility | Migrating tag-source scripts | A declared original version does not guarantee every parser command is supported; grammar compilation already protects installation. | R / medium | Mac command/version mapping; actual source and original results |
| [L33](../research/remaining-gaps.md#l33) | P3 | Missing mobile capability | Mobile Discogs users | Mobile secure token storage is absent; desktop secure storage is a separate route. | L / medium | Mac adapter/contracts; mobile secure-store lifecycle |
| [L34](../research/remaining-gaps.md#l34) | P2 | Unknown field compatibility | Identifier/lyrics editors | UFID edit grammar and USLT grouping may differ; untouched binary/group retention already exists. | R / medium | Mac field/carrier analysis; Windows edit/readback |
| [L35](../research/remaining-gaps.md#l35) | P3 | Limited guarded format | Advanced Matroska users | Info-title/mapping, full SeekHead and ordered/linked chapters remain incomplete; no-room cases refuse safely. | R → XL / low | Mac layout/mapping work; reference and player acceptance |
| [L36](../research/remaining-gaps.md#l36) | P3 | Guarded limit; unmeasured cost | Extreme ASF/WMA files | Headers above 512 MiB refuse; ordinary full-file rewrite cost still needs measurement. | R / medium | Mac bounded fixtures/benchmarks; Windows comparison |
| [L40](../research/remaining-gaps.md#l40) | P3 | Unknown chapter semantics | Chapter automation users | Subsong numbering/name case is unconfirmed; whole-file command deduplication is already implemented. | R / medium | Mac tracing/native commands; Windows reference semantics |
| [L41](../research/remaining-gaps.md#l41) | P2 | Limited guarded utilities | Advanced maintenance users | Optimize refuses some layouts; broader Remove Unsupported requirements are unestablished. | R → L–XL / low | Mac topology/utility work; reference/player results |
| [L43](../research/remaining-gaps.md#l43) | P2 | Missing technical values | Custom-column/export users | Some VBR and container tag-size fields remain empty or incomplete. | M–L / medium | Mac extraction and byte-size oracles; reference display/rounding |
| [L46](../research/remaining-gaps.md#l46) | P2 | Limited Linux workflow | Cover drag-out on Linux | File URIs lack correct encoding, so temporary filenames are restricted to ASCII. | S–M / medium | Portable URI fix on Mac; real Linux drag acceptance |
| [L47a](../research/remaining-gaps.md#l47a) | P1 | Limited daily workflow | Filename/list users | Refresh cannot discover the new path of a file opened individually after an external rename. | L / medium | Mac source-identity implementation; Windows original and cross-platform native acceptance |
| [L47c](../research/remaining-gaps.md#l47c) | P1 | Unknown save policy | Daily editing/selection | Some navigation/selection routes leave drafts pending despite save-on-move expectations; original triggers must be established. | R / medium | Mac draft/native checks; Windows reference trigger policy |
| [L48](../research/remaining-gaps.md#l48) | P2 | Limited playback/directory UX | Selected-file playback users | Play opens files separately instead of one ordered playlist; subfolder choice is outside the picker workflow. | M / medium | Mac playlist lifecycle/UX; each OS player/picker acceptance |
| [L49](../research/remaining-gaps.md#l49) | P2 | Limited workflow; contract unknown | Directory/panel users | Directory dropdown selects rows instead of changing the working directory; format-string behavior is unresolved. | R / medium | Mac state/evaluation analysis; original behavior |
| [L50](../research/remaining-gaps.md#l50) | P2 | Missing migration capability | Customized toolbar users | Vendor toolbar layouts and user symbols must be recreated manually; commands and band placement already exist. | R → implementation TBD / low | Mac schema/import work; populated original layouts |
| [L51](../research/remaining-gaps.md#l51) | P2 | Limited tools; contract unknown | External-tool users | Tag placeholders, Windows shortcuts/batch forms and the 100-launch cap can prevent existing configurations from transferring. | R / medium | Mac placeholder/cap analysis; Windows .lnk/.bat launching |
| [L52](../research/remaining-gaps.md#l52) | P2 | Limited legacy source routes | freedb/CDDB users | Fetched server lists, broader server search/artwork and non-exact local lookup are missing; gnudb and exact local lookup exist. | R → L / low | Mac protocol/local-match work; usable services/reference semantics |
| [L55](../research/remaining-gaps.md#l55) | P2 | Unknown converter/export semantics | Advanced converter users | Relative paths, shared output partitions and slash markers may differ; existing refusals must not be removed speculatively. | R / medium | Mac plan/path analysis; original target/order semantics |
| [L56a](../research/remaining-gaps.md#l56a) | P2 | Limited batch policy; contract unknown | Batch filename conversion | One collision refuses the reviewed plan, including unaffected rows; original partial-clash policy is unqualified. | R / medium | Mac transaction analysis; Windows reference policy |
| [L56b](../research/remaining-gaps.md#l56b) | P2 | Limited legacy encodings/presets | Legacy text/action libraries | Some codepages, ANSI interpretations, quality presets and defaults do not have confirmed equivalents. | R / medium | Mac decoder/preset analysis; original bytes/options comparison |
| [L57b](../research/remaining-gaps.md#l57b) | P1 | Missing/unknown migration mappings | Migrating customized profiles | Global/binary settings and panel custom_values/prefer_tag are not fully carried over; supported import/recovery already exists. | R → bounded mapping PR / medium | Mac schema/mapping work; actual profiles/reference readback |
| [L58](../research/remaining-gaps.md#l58) | P2 | Missing integrated updater | All installed users | The update check opens a release page; downloading and installing are manual. | L–XL / medium | Mac verification/install flow; OS upgrade/restart acceptance; release trust prerequisite |
| [L63](../research/remaining-gaps.md#l63) | P3 | Missing optional catalogs | Users needing another language | Only English/Dutch/German are available; add languages on demonstrated demand rather than matching 43. | L per catalog / medium | Mac authoring/review and native journeys; other OS IME/layout |
| [L64](../research/remaining-gaps.md#l64) | P2 | Limited networking | Restricted-network users | Proxy authentication, SOCKS or PAC gaps may block online tagging/updates for affected networks. | R → L–XL / low | Mac protocol/credential boundaries; native system-proxy acceptance |
| [L65](../research/remaining-gaps.md#l65) | P3 | Unknown original settings | Specialized preference users | Several original setting names have no established meaning; a missing equivalent is not yet a proven missing feature. | R / low | Mac static caller/key study; original toggle/restart when needed |
| [L66](../research/remaining-gaps.md#l66) | P3 | Unknown numbering setting | Numbering users | A possible global leading-zero switch is unresolved; the numbering-wizard option already works. | R / low | Mac config/binary study; original numbering outcomes |
| [L70](../research/remaining-gaps.md#l70) | P2 | Missing packaging subset | Windows/Linux installed users | Association declarations are missing; stable Windows installer identity already exists and manual opening works. | S–M local; native M / medium | Mac-authored config; Windows/Linux installed opening/upgrade/uninstall |
| [L71](../research/remaining-gaps.md#l71) | P1 desktop; P3 iOS | Missing release integration/qualification | New desktop installations | Recorded packages are unsigned/ad-hoc; installation trust and iOS distribution need real signing/artifact evidence. | R → L desktop / medium | Mac certificates/account/artifact work; Windows signing and physical iOS elsewhere |
| [L72](../research/remaining-gaps.md#l72) | P3 | Missing mobile capability | iOS users | The Android provider route does not supply an iOS picker, grants or save-back. | XL / low | Mac bridge/simulator contracts; physical iOS/cloud-provider tests |
| [L73](../research/remaining-gaps.md#l73) | P1 | Unknown save interaction | Extended Tags users | Keep changes stages edits; immediate Save timing on navigation/close may differ from familiar expectations. | R / medium | Mac native edit/navigation/cancel checks; Windows reference timing |
| [L74](../research/remaining-gaps.md#l74) | P3 | Missing mobile capabilities | Provider power users | Provider playlist load/create, CSV/export and Undo/Redo are not enabled by existing read/copy support. | XL / medium | Mac orchestration/UI; Android/iOS provider publication/replay |
| [L76](../research/remaining-gaps.md#l76) | P1 release audit; P2 reproducibility | Audit/qualification | All release users | Complete binary/JS/native notice coverage and reproducible artifacts remain unqualified; no new license defect is claimed. | R → M–L / medium | Mac archive/notice/rebuild checks; other artifacts/native rebuilds on target OS |
| [L77b](../research/remaining-gaps.md#l77b) | P2 | Unknown performance cost | Large/batch libraries | Whole-file rewrites may be slow for remaining containers; impact must be measured before optimizing. | R → M measurement / medium | Mac same-corpus benchmark; other OS/reference comparisons |
| [N03](../research/remaining-gaps.md#n03) | P1 | Unknown source preview semantics | Migrating album-parser sources | Album-preview grammar/precedence is unresolved; the separate full-cover result route already exists. | R / medium | Mac parser/DTO analysis; original outputs/native provider comparison |
| [N04](../research/remaining-gaps.md#n04) | P2 | Unknown automation semantics | Advanced expressions/columns | Column-function and self-reference outcomes may differ; existing evaluator/export repairs already cover named contexts. | R / medium | Mac adapter/recursion tracing; same-vector original execution |
| [N05](../research/remaining-gaps.md#n05) | P1 | Unknown action interchange | Migrating .mta action groups | Remaining recognized/unknown option combinations lack original generated-file and re-import proof; opaque preservation already exists. | R / medium | Mac schema/static work; Windows-generated files/re-import deferred |

## Qualification of already represented behavior

These **14 separate identities** are acceptance/research records, not 14 more
missing features. Twelve have useful local work; L75 and L79 require the actual
other environment. Do not add these numbers to the intake table as a completion
score. Implement a repair only after a demonstrated difference.

| ID | Priority | What the user needs confidence in | Effort / confidence | Where the next evidence can be obtained |
| --- | --- | --- | --- | --- |
| [L10](../research/remaining-gaps.md#l10) | P2 | Scripting errors, arity and Unicode behave as documented; unknown-function repair already exists. | R / medium | Mac entry/error journeys and static tracing; Windows original edge results later. |
| [L12](../research/remaining-gaps.md#l12) | P2 | Broader filter grammar, locale, malformed-query and native input behavior. | R / medium | Mac input/Unicode checks; pinned original comparison later. |
| [L23](../research/remaining-gaps.md#l23) | P3 | Real TAK/OptimFROG audio remains intact; support exists but an independent OptimFROG decoder oracle is missing. | R / medium | Investigate obtainable decoder/provenance locally; external tooling may be needed. |
| [L28](../research/remaining-gaps.md#l28) | P1, reference-dependent | Actual existing Mp3tag/WMP/Nero-written libraries interoperate, beyond authored fixtures. | R → L / medium | Validate supplied captures on Mac; record Mac originals separately; Windows writer corpus requires Windows. |
| [L29](../research/remaining-gaps.md#l29) | P2 | Cover resize/quality/rounding matches the intended contract; controls already exist. | M / medium | Paired Mac-original image outputs; Windows target comparison later. |
| [L30](../research/remaining-gaps.md#l30) | P1 for the owner's sources | Real private source HTTP/TLS/artwork/review/save journeys work, beyond parser and loopback contracts. | R / medium | Mac with authorized owned responses/service availability; not intrinsically another OS. |
| [L31](../research/remaining-gaps.md#l31) | P2 | Configured AcoustID/Last.fm/fanart.tv setup and lookup/artwork work. | R / medium | Mac with legitimately configured keys/service access; never copy secrets into evidence. |
| [L38](../research/remaining-gaps.md#l38) | P3 | Ape/Mpc ID3 display/precedence follows the intended policy; no new display requirement is established. | R / medium | Mac static/carrier study; Windows original read/display later. |
| [L42](../research/remaining-gaps.md#l42) | P2 | Export audio-checksum and absent-checksum behavior is interoperable. | S–M / medium | Paired Mac export and independent media hashes; Windows reference later. |
| [L62](../research/remaining-gaps.md#l62) | P2 | Existing Mac MKV/WebM associations work through Finder/Dock cold and running launches. | S / medium | Isolated installed Mac registration/journey. |
| [L75](../research/remaining-gaps.md#l75) | P3 | Provider permission loss, cloud save-back, original recovery and edited-copy behavior work on real devices. | M–L per provider / low | Named physical Android/iOS devices and providers; desktop models are insufficient. |
| [L77a](../research/remaining-gaps.md#l77a) | P2 | Scan/edit/save/cancel/input performance on a fixed real corpus is understood. | M baseline / medium | Mac native benchmark; other hosts/reference versions remain separate. |
| [L78](../research/remaining-gaps.md#l78) | P1 targeted; P2 broader matrix | Real keyboard, pointer, IME, focus and screen-reader behavior is usable. | M targeted; XL exhaustive / medium | Mac native journeys now; Windows/Linux on their hosts. |
| [L79](../research/remaining-gaps.md#l79) | P1 importance, deferred | Actual pinned Windows-original differential save/Undo/query outcomes exist. | R → L–XL / low | Windows 3.36.1 runtime required; prepared capture tools are not captures. |

The represented implementation records also retain bounded local acceptance:

| Scope | Priority | Concrete local check, not new feature implementation |
| --- | --- | --- |
| [C47](../research/remaining-gaps.md#c47) | P1 | [Historical native replacement/add, relative-path and pending-edit cases](../verification/native-workspace-acceptance-2026-10-19.md) retain their original pins. Replay on current merged loading/restoration paths, queued arrivals during workspace restoration and the separately documented forced-new-process route remain local acceptance. |
| [C57](../research/remaining-gaps.md#c57) | P2 | Native editor/action-file/export/save journeys for the implemented canonical action schemas; Windows original re-import remains separate. |
| [C58](../research/remaining-gaps.md#c58) | P2 | [Historical native toolbar/filter/compact cases](../verification/native-workspace-acceptance-2026-10-19.md) qualify only the pinned older builds. VoiceOver, broader keyboard/IME/accessibility and the retained AX/AppKit menu case remain; no latest-build native toolbar proof is claimed. |
| [C59](../research/remaining-gaps.md#c59) | P1 | [Historical native restarts, exact rows, explicit launches and missing/corrupt recovery](../verification/native-workspace-acceptance-2026-10-19.md) retain their original pins. The [focused merged-source replay](../verification/native-workspace-acceptance-2026-10-19.md#focused-merged-source-replay) qualifies suffixed-directory Refresh and manual/playlist-source-order normal restarts only. Column mode, CUE/chapters, missing/corrupt recovery, quota retry and queued arrivals remain pending. |
| [C60](../research/remaining-gaps.md#c60) | P2 | Actual Mac region/custom-pattern/timezone changes update loaded rows without damaging drafts. |
| [C61](../research/remaining-gaps.md#c61) | P1 | Bounded native Mac concealed-edit replay refusals pass; broader corruption notices and physical input remain qualification. |

## Recommended sequence after PR64

Do not reopen a large umbrella PR. Select one coherent user outcome, define its
acceptance and keep unrelated features in the ledger.

| Order | Bounded next outcome | Candidate IDs | Why this order / exit condition |
| --- | --- | --- | --- |
| 1 | Native desktop confidence and any demonstrated save/restart repair | C47/C59/C61; targeted L78/L73/L47c; L62 where installed context is available | Use postmerge native CI and focused checks for actual failures; do not repeat local UI sweeps to refresh evidence or add speculative implicit saves. C62 records completed bounded history/recovery validation; do not repeat it as missing implementation. |
| 2 | Predictable external-rename Refresh | L47a; C63/C64 represent action protection and known inline destinations | Finish exact source-identity discovery without replacing a missing file with a guessed neighbor or broadening membership to its parent. Preserve the represented inline behavior and its separate Windows/native qualification. |
| 3 | One real migrated tag-source workflow at a time | L30; scoped L32/N03; specific L52 route if needed | Sources are a stated owner priority and freedb is active. Retain actual authorized responses, reproduce a user-visible difference, then repair that route; do not rebuild already delivered parser/install engines or call all sources missing. |
| 4 | Confirmed profile/action migration omissions | Start L07/L57b; N05 as a separate action-file batch | Use actual profiles/static schemas and explicit loss diagnostics. Ship a bounded confirmed mapping rather than all undocumented settings; preserve existing import/recovery and opaque data. |
| 5 | Trusted Mac distribution and bounded release inventory | Desktop L71 with a bounded L76 audit | Broad install/trust benefit. Check current artifacts and actual account/certificate availability first. Windows signing and physical iOS distribution stay separate. This can move earlier if current installation is the adoption blocker. |
| 6 | Small advanced desktop improvements | Playback slice of L48, or L43; each separately | Concrete ordered playback or useful technical values; independent implementations should not be bundled solely to reduce PR count. |
| 7 | Verified updater | L58 | Follow artifact identity/signature and upgrade decisions; the existing release-page fallback works meanwhile. |
| 8 | Corpus-led format/utility and performance work | L26/L34 with L28 inputs; separate L24/L25/L35/L40, L41, L77a/L77b/L36 | Start from an affected real corpus and independent oracle, preserve unchanged refusal, measure before optimization, and split by container/outcome. |
| 9 | Demand-led expansion | L64 when an affected network is known; mobile L18/L33/L72/L74; L63 catalogs | Restricted-network access can be promoted for a real blocked user. Mobile and extra catalogs follow desktop priorities; do not chase an original feature/language count. |

## Traceability and refresh

This assessment reconciles every current intake ID with its anchored evidence,
using the read-only local disposition audit only as a routing aid. That older
audit's L61-WIP state is superseded by C61 and is not copied as current status.
A row's actual source and bounded receipt remain stronger evidence than its
priority label. No new original Windows, service, device or GUI pass was created
by this planning review.

L67 has moved to C62 with bounded implementation/acceptance evidence; it is absent
from the 48-row intake table and CSV. After merge, open the next PR from new
main using this order and the actual user-visible acceptance condition. Keep
unresolved local work local in the handover, even when its final native check
needs another OS. Reassess priority when a concrete failing library, provider,
profile or measured bottleneck changes the impact.
