# Ordinary-save acceptance corpus

This is the first R01 trust-save batch against approved baseline
`4a1a95ff4eddb9da7492aaace68197c4f81347d0`. It adds retained independent
MP3, FLAC and M4A carriers, backend preservation regressions and an executable
paired producer/consumer. It does not change save defaults or certify Mp3tag
parity. R01 and the save/recovery aspects of R17 remain open.

## Evidence boundaries

The reference is the supplied Windows x64 Mp3tag 3.36.1 executable, SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The leanrea workflow reused the existing disposable extraction under
`$TMPDIR/re/mp3tag-3.36.1-roadmap`; its executable hash was checked and `file`
identified a PE32+ x86-64 GUI executable. Its UTF-16 inventory contains
`ID3v2.4 UTF-8` at `0xb65ed8`, `ID3v2.3 UTF-16` at `0xb65f14` and
`ID3v2.3 ISO-8859-1` at `0xb65f50`. These are **seen static labels**, not
captures of selected defaults or original save behavior.

On the inspected macOS host, `wine`, `wine64`, `prlctl` and `VBoxManage` were
not on PATH. No CrossOver, Whisky, Fusion, UTM, Parallels or VirtualBox app
was found in the inspected application locations. QEMU is installed, but no
existing Windows image was found in the standard VM locations checked.
An old `~/.wine` prefix exists; no Wine executable was found in PATH,
Homebrew Caskroom, `/usr/local/bin` or the user Applications folder. No
software was installed, OS image created or reference application executed.
This bounded search does not prove that no other Windows host is available.

**Unknown:** actual original defaults, original save/reload/undo behavior,
native UI input, packaged/released behavior, MP3Gain consumer behavior,
cancellation/interruption and restart recovery. All reference observations
remain pending until operator captures are supplied.

## Retained inputs and assertions

`crates/tag-core/tests/fixtures/save-corpus/manifest.json` records the frozen
carrier hashes, independent sample formula, exact generation commands and
FFmpeg identity. The files contain 0.2 seconds of synthetic mono audio; no
vendor or user media is redistributed. Generation used FFmpeg, but the Rust
regressions and paired harness require no media executable at runtime.

The shared producer adds non-ASCII titles/paths, two ordered artist values,
two custom values, lyrics, a rating field, ReplayGain gain/peak and two
distinct cover images. Native fixtures include an opaque ID3 frame, a FLAC
APPLICATION block and an unknown binary MP4 atom. MP4 does not preserve
cover types/descriptions; expectations start from the format's actual seeded
representation and preserve that representation through the edits.

`crates/tag-core/tests/save_acceptance.rs` exercises six save cases:

| Operation | Expected changes | Expected preservation |
| --- | --- | --- |
| `edit-one` | Set `TITLE` to `Changed café 日本`. | Every other field, cover order/data, native opaque data and encoded audio payload. |
| `edit-many` | Set that title, delete `ALBUM`, set `ARTIST` to `Third`, `Fourth`. | Every other field, cover order/data, native opaque data and encoded audio payload. |

Each case reloads from disk, undoes to exact original bytes, redoes to exact
saved bytes and rejects a NUL-containing field without changing the saved
file or replacing the undo step. Twelve real-file negative controls remove
a custom field, remove a cover, change opaque bytes and corrupt audio bytes
across the three formats. They must produce detectable differences.

Audio identity uses encoded media payload bytes, not whole-file equality:
MPEG excludes leading ID3 (including a validated ID3v2.4 footer) and trailing
ID3v1/APE. FLAC excludes metadata blocks. Ordinary 32-bit MP4 concatenates
top-level `mdat` payloads, including a final box whose zero size means EOF. The
Python receipt computes SHA-256 over those bytes. Separate whole-file hashes
identify paired inputs and retained save/undo/redo stages. Every local case
requires undone bytes to match the original input hash and redone bytes to
match the saved-stage hash, including operator-supplied corpora. These exact
local replay checks are additional to the structural comparisons. They do
not assume that two applications produce identical saved container bytes.
This deliberately small
parser does not qualify arbitrary Lyrics3, extended-size/fragmented MP4,
FLAC with leading ID3, or other specialist layouts. Unsupported layouts
must be qualified separately; a parse failure is not a passing result.

The observer compares decoded fields, ordered covers with image hashes,
opaque ID3 binary frames, native APE binary items, non-comment/picture/padding/STREAMINFO FLAC
metadata blocks and MP4 atoms containing unknown binary data. It also checks
the seeded opaque marker. It is not an exhaustive independent metadata
decoder: both applications' captures are interpreted by the same TagGroove
reader, and hidden metadata that reader does not expose needs further
native-format evidence.

## MPEG defaults and gain-location probe

A separate competing-tag probe reads the APE title in preference to a
different ID3 title. Under current TagGroove defaults, Save writes ID3v1
and ID3v2.3 UTF-16 and removes the unwritten APE tag. `MP3GAIN_UNDO` and
`MP3GAIN_MINMAX` text can then be visible in ID3. That does not demonstrate
compatibility with a gain consumer expecting native APE data.

With the explicit `write_ape=true` option, the probe checks both native APE
gain fields and an opaque binary companion. Both policy variants preserve
the audio payload and can be undone exactly. No defaults were changed to
achieve these checks, and no gain application was executed.

## Run and capture a pair

From the repository root, choose a new disposable capture directory. An external
location such as the example below keeps capture data separate from the checkout.
Repository-local directories such as `./save-pair` also work. Their source-relative
path is recorded as `untracked_capture_workspace`: only untracked files beneath
that capture directory are excluded from source-status identity. Tracked status,
the complete tracked diff, source-file hashes and backend hash remain checked.
Use the capture directory for data. The source root and its ancestors, Git
administration directories and frozen carrier inventory are reserved; invalid
placements are rejected before the CLI builds its backend or creates captures.


```sh
python3 scripts/save-acceptance.py prepare /tmp/taggroove-save-pair
python3 scripts/save-acceptance.py run /tmp/taggroove-save-pair
```

To use captured reference inputs instead, pass `--corpus DIRECTORY` to
`prepare`. Only regular MP3/FLAC/M4A files are copied; the source files remain
untouched. The small parser's layout limits still apply. Check that the
files are suitable for the specified edits before running. Each operation
receives an independent copy of the exact same input bytes for each app.

`contract.json` contains input hashes, actual TagGroove defaults, expected
fields, source/build/platform identities and explicit missing reference
observations. `taggroove-results.json` records expected versus observed
save/undo/redo structure, rejected-write results and MPEG policy observations.
The actual saved/undone/redone files remain beside the receipt. Source head,
dirty diff hash/status, Cargo.lock hash, backend executable hash, compiler
and platform are retained so a dirty development run cannot be mistaken
for a released application.

Preparation, execution and comparison must use the same stable source,
backend executable, dependencies, compiler and platform identities. Capture
timestamps are excluded from equality checks. Identity drift is rejected
before the comparison observer runs, and each phase checks again on
completion. Prepare a fresh pair after changing the repository or build.
The paired receipt retains the original preparation and local save producer
identities separately from the comparison consumer identity.

On a usable Windows/original runtime, follow the generated
`REFERENCE-INSTRUCTIONS.txt`: verify the pinned executable and input hashes,
capture selected Options defaults, execute each operation on its own copies,
then retain files at each save, undo and redo stage. Capture native UI/reload
and runtime evidence separately. Supply the resulting identity JSON:

```json
{
  "application": "Mp3tag",
  "version": "3.36.1",
  "executable_sha256": "484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911",
  "platform": "record the actual Windows build",
  "runtime": "record native Windows or the actual compatibility runtime/version",
  "recorded_at_utc": "record the capture timestamp",
  "defaults": {"record": "the actual selected read/write/remove settings and encoding"},
  "evidence": ["paths to screenshots, logs or capture notes"],
  "input_sha256": {"café 日本.mp3": "the verified pre-edit hash; include every input"}
}
```

```sh
python3 scripts/save-acceptance.py compare /tmp/taggroove-save-pair --identity /tmp/mp3tag-capture.json
```

The consumer requires every reference stage and matching pinned provenance.
It independently rereads retained inputs and local/reference outputs,
compares them against the expected operation and writes structural
differences to `paired-results.json`. Before accepting the local stages, it
checks their recorded whole-file hashes and repeats the exact local
undo/redo-byte checks. Both sides' stage hashes appear in the paired receipt;
original-app stages are compared structurally, without inventing an original
exact-byte undo guarantee. Missing original files, mismatched
producer identity, changed baselines, modified local outputs and observed differences
cannot produce a passing comparison. Provenance is operator supplied; the
tool does not authenticate that a particular application produced a file.
An agreement is limited to these supplied captures and this reader's view.

## Validation and remaining work

```sh
cargo test -p tag-core --test save_acceptance --locked
python3 -m unittest discover -s scripts/tests -p test_save_acceptance.py -v
```

The Python suite executes the producer and consumer with explicitly
synthetic reference copies. It verifies missing-capture/identity rejection,
captured-input copying, baseline/receipt tamper detection and real audio
corruption detection. Those synthetic controls test the harness; they are
not Mp3tag execution evidence.

Additional controls change a disposable backend binary and initialize a
different disposable Git repository. Both must fail producer binding before
comparison. Actual FLAC padding edits and ID3 text-frame reordering leave
the observer's projection unchanged but must fail the local exact-byte
undo/redo checks, both during an imported-corpus run and when retained stages
are changed afterward. The authorized reviewed-build tooling job runs this
Python suite with the stable Rust toolchain; no automatic workload trigger,
review authorization boundary or credential exposure was added.

Remaining R01 work includes captured original defaults and paired results,
an independent metadata decoder/consumer check, real collection expansion,
native UI save/cancel/reload and interrupted/rejected OS writes. Restart
history, non-cloning storage budgets and crash recovery belong to the
separate R17 implementation stream. Specialist formats, platform/device
editing and released artifacts remain separate acceptance programs.


## Review follow-up: audio boundaries

After integrating approved main `84bf037cee0402a5de0de09dd7374cf36dd8ceb1`, four new boundary controls failed against the previous observer and pass with its two parsing fixes. The application save implementation is unchanged.

- An ID3v2.4 header's size omits its optional ten-byte footer. The observer now skips a footer only when the v2.4 flag is set, and validates its signature and matching header fields. Missing, truncated or mismatched footers fail observation. A valid authored title frame/footer wraps the retained MPEG frames; the test reads its title and preserves the same audio before and after an ordinary save. See the [ID3v2.4 structure specification, sections 3.1 and 3.4](https://id3.org/id3v2.4.0-structure).
- A zero-sized top-level MP4 box extends through EOF. The observer now uses the remaining file length and retains bounds checks. The retained M4A is rearranged into a valid terminal-`mdat` layout with its real chunk offset adjusted; metadata and audio are read before an ordinary save, and audio remains identical after it. A final EOF-sized metadata box is skipped rather than hashed as audio. Truncated, oversized, empty-audio and still-unsupported extended-size cases fail. See [Apple's atom size documentation](https://developer.apple.com/documentation/quicktime-file-format/atoms).

The frozen carrier files and six baseline save cases are unchanged. All seven save-acceptance Rust tests pass, including the four new boundary tests. These are local observer/save controls, not captures of original Mp3tag behavior.


A one-off independent decode check with local FFmpeg 9.0.2 confirmed identical decoded PCM SHA-256 for the footer MP3, EOF-sized `mdat` M4A and EOF-sized `moov` M4A against their frozen carrier originals. This checks validity of the authored variants; it is not original-application evidence. The Rust regressions and paired harness still require no media executable at runtime.


Combined-source validation after approved-main integration passed 575 frontend tests, 1,095 Rust tests across 72 result blocks (six existing ignores), all ten Python save-acceptance tests, 107 review-tooling tests and 28 release tests. Production build, strict guide build, strict local Clippy, formatting, actionlint with the documented concurrency-queue exception and diff checks passed. Final-head Codex review and protected current-main platform validation remain required before merge.


## Review follow-up: gain verdict and workspace identity

The policy verdict now checks the seeded displayed values of `MP3GAIN_UNDO`, `MP3GAIN_MINMAX` and `REPLAYGAIN_TRACK_GAIN` for both default and explicit-APE saves, in addition to its native APE, audio and undo checks. `mpeg_policy_passed` records that verdict and `mpeg_policy_gain_errors` identifies differing field paths. Twelve synthetic report faults remove or change each field in each policy; every fault makes the local verdict fail while the ordinary file-capture cases still pass. These controls test the consumer, not original-application behavior.

Repository-local capture data no longer produces a late source-identity failure. The producer records its canonical source-relative data scope and filters only untracked status entries inside it. All tracked entries remain visible, including tracked files inside the capture directory; unrelated untracked files also remain visible. The tracked binary diff, source-file hashes and executable hash are unchanged identity checks. Status parsing uses Git's [NUL-delimited porcelain format](https://git-scm.com/docs/git-status#_porcelain_format_version_1) so whitespace and non-ASCII filenames remain distinct. Source roots, Git administration and the retained-input inventory are rejected before preparation.

All 17 Python acceptance tests pass. Real disposable committed repositories cover unignored, ignored and partially ignored capture directories, symlink aliases, preparation/run/synthetic comparison, and rejection of source edits before the observer executes. The application and Rust sources remain byte-identical to implementation head `03ef0ad24850ca02f44c4866c133686d6e36388f`, where the full 575-frontend/1,095-Rust suite passed; six Rust tests remain intentionally ignored.
