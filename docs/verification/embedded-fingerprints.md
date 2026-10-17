# Embedded fingerprint producer contract

Implementation in progress on the native AcoustID feature branch; not a released claim.

The requested installed fpcalc1.6.2 artifact (SHA25689d382fd61f67485a329fc73d1b76fc4d7b084c94d26fc2e19c8110a0da08d0b) and linked Chromaprint library (SHA256a9a451daec997fa88381ec15bd5c76df0a9432a62a8fbe2229dcbabccdd93db5) were examined using REA/Ghidra12.1.4. The bounded native trace covers CLI startup/processing/output/SWR, library CAPI/pipeline construction/audio feed/compression, and initialized defaults. Source is the implementation authority; decompiled types and unresolved virtual edges are not original source.

A disposable in-process CABI prototype matched14 accepted generated-media cases (WAV/Unicode,FLAC,MP3,AAC/ALACMP4,Opus,Vorbis,WavPack,TTA,WMAv2,MP2,multichannel,longduration,rounding edge), plus reference rejection for short/malformed/missing files. No personal media or live provider calls were used. Inputs were hashed before/after and unchanged. This establishes the chosen mechanism on macOS; production package/target validation is separate.

## Product contract

- Default fingerprinting is embedded; explicit desktop fpcalc configuration remains an optional override. Lookup/ranking/rate/credentials stay unchanged.
- Upstream Chromaprint1.6.2 TEST2/internalID1;11,025Hz mono signed16bit; SWR engine,filter16,phase8,linear1,cutoff0.8. Direct original-rate CAPI input is not interchangeable with the CLI: the six-channel synthetic fixture demonstrated a real mismatch until compatible conversion was applied.
- Fingerprint only the first120seconds of normalized frames, while reporting full stream/container duration. Check raw item count, not just a nonempty encoded header. Use reference compressed URL-safe unpadded base64.
- Preserve centisecond producer quantization before the existing Rust lookup rounding. Actual unchanged parse_fpcalc/lookup_form from5c49582/ed9beee were compiled independently:49.499s→CLI49.50→request50; raw milliseconds would request49.49.500/49.501both request50. Negative/zero/missing/null/string controls reject.
- Read only an approved local regular file through custom AVIO. Build no FFmpeg protocols/network support; never hand an arbitrary URL/path to FFmpeg. Rust owns file handles and callback lifetime; native contexts/results use RAII and no C++ exception crosses CABI.
- Cancellation/deadline callbacks are cooperative. The UI can return on its deadline while a blocked worker still owns resources; admit at most one native job and keep its permit until actual return. Do not claim hard child-kill equivalence.
- Pinned source/checksums/configuration, actual decoder manifest, patches/notices, and complete corresponding library/application source plus rebuild/relink instructions must accompany releases. Host GPL Homebrew libraries are reference tools only.

## Native source configuration

`scripts/audio-deps/sources.json` pins Chromaprint 1.6.2, FFmpeg 9.0.2,
zlib 1.3.2 and bzip2 1.0.8. The actual built backend registers 218 audio
decoders, zero non-audio decoders and zero protocols. It has no encoders,
runtime programs, network, GPL/nonfree/version3 components or host-library
autodetection. Explicit demuxers cover the existing tag families; registration
does not certify every encoder variant. OptimFROG/DRM decoding is not established
by tag-editing support.

Nine variants are gated upstream by the internal `lgpl_gpl` policy switch:
`adpcm_circus`, `adpcm_ima_escape`, `adpcm_ima_hvqm2`, `adpcm_ima_hvqm4`,
`adpcm_ima_magix`, `adpcm_ima_pda`, `adpcm_n64`, `adpcm_psxc`, and `ahx`.
Their source files, AHX parser and required AHX-to-MP2 bitstream filter carry
LGPL headers. Upstream describes this gate as a developer preference for
LGPL-marked files. A pinned, hash-checked source patch opts into this internal
group while leaving all upstream license checks and public GPL/nonfree/version3
flags disabled. Original source, patch bytes, generated configuration and
actual object/header closure are retained in the build manifest and source
release. This records the reviewed source configuration, not legal certification.

The omission was a real supported-input regression: a generated AIFC `VAPC`
file loads through the actual OpenTag tag reader and fingerprints with the
reference CLI. The original 210-decoder build rejected it. Its actual Rust API
regression now passes with the pinned patch. `hdr`, a video decoder located
inside the upstream audio registration section, is explicitly excluded.

## Validation ledger

Implementation plan: ../superpowers/plans/2026-10-17-embedded-fingerprints.md.
Production API, source-built dependency/cancellation/corpus/package/target results are recorded here as they complete. The prototype proves neither the app integration nor released package independence.

## Actual product qualification

On macOS arm64, the real Rust API built from the pinned original archives and
recorded patches passes all fourteen frozen synthetic reference cases: stereo
WAV with a Unicode name, FLAC, MP3, AAC/ALAC MP4, Opus, Vorbis, WavPack, TTA,
WMA v2, MPEG Layer II, 48 kHz six-channel WAV, a 135.37-second file with a
120-second fingerprint prefix, and the 8.495-second rounding boundary. All
compressed fingerprints and full quantized durations match the independently
hashed installed CLI oracle. Original input hashes remain unchanged.

Additional actual API tests pass for full duration/prefix behavior,
short/malformed rejection, VAPC, pre-cancellation/zero budget, cancellation and
deadline during callbacks, admission before spawn, and retained admission
while a worker is stalled. These tests also run with runtime `PATH` empty.
The app's production await helper is tested against superseded successful
work and a UI deadline while the real admitted worker remains blocked.

The initial source build deliberately failed the frozen Vorbis golden. All
fourteen durations and thirteen other fingerprints matched. Controlled C ABI
hybrids isolated divergence to FFmpeg's decoded float samples before SWR:
using the installed decoder matched the reference, while substituting the
source-built decoder reproduced the difference. Runtime CPU flags set to zero
did not reproduce the source-build behavior. A separate otherwise-identical
build changing `--disable-asm` to `--enable-asm` restored the reference. The
product therefore retains architecture assembly and requires NASM as an x86
build tool. Disabling assembly is not a valid portability shortcut for this
compatibility contract. The complete actual Rust API corpus passes with this
final configuration; no expected fingerprint was changed to conceal the bug.

An independent arm64 preflight built the bundled KissFFT backend instead of
vDSP and also matched all fourteen fingerprints/durations using the same
ASM-enabled decoder. This checks the alternate FFT backend, without claiming
Windows/Linux/Android runtime or package qualification.

The corrected final arm64 native manifest binds 218 audio decoders, zero
non-audio decoders/protocols, 500 nonempty compiler dependency records and the
actual generated build inputs. Every original-source entry is checked against
the pinned FFmpeg archive; generated inputs are retained separately. Cache
validation covers every installed header/library/configuration/notice and
fails on missing or changed files. Archive traversal, tampering, explicit
offline no-network behavior, changed cached headers/inventory and relative
`src/` compiler dependency aliases have regression tests.

The integrated app test executable compiles. Native default/legacy override
migration, existing lookup/ranking/pairing and the production asynchronous
cancellation boundary pass targeted app tests. An isolated real desktop app
with its own application identifier passes offline About/settings smoke tests:
actual linked versions/registration, complete LGPL notice text, source/relink
instructions, license wrapping and an empty optional executable override.
The final ASM desktop relink and offline native UI smoke pass. On frozen native
commit `de887441906b38290a111b57593f283fbed77780`, based on main `7c77a17`,
the complete workspace passes 1,045 tests with six existing ignored tests;
frontend tests pass 384 cases, strict MkDocs and workspace Clippy also pass.
The integrated release/source layer separately passes 86 CI-tooling, 19
source-bundle and 27 release tests; workflow lint remains required on the
combined head.

A host-only source staging control exported that immutable application source,
all 597 Cargo packages, the prebuilt frontend and all four pinned native
archives. It verified 33,279 original staged files. An intentional local
Chromaprint modification adds `-OpenTag-relink-proof` to its version result;
the modified archive has its own filename/hash and an explicit source-pin diff.
A fresh complete application build succeeds with Cargo `--offline --locked`
and local native archives. The running full app, with runtime `PATH` empty,
reports `1.6.2-OpenTag-relink-proof` through its real backend command, alongside
FFmpeg 9.0.2, LGPL 2.1-or-later, 218 audio decoders and no non-audio decoders or
protocols. Its dynamic dependencies are system frameworks/libraries. Every
original staged file remains unchanged except the intentional source-pin JSON.
This demonstrates modification and relinking of the actual app; it is not a
seven-target release source archive or a claim of other-platform runtime parity.
The intentional modification exists only in owned test staging and never in
the shipping component pins.

| Target | FFT | Qualification boundary |
|---|---|---|
| macOS arm64 | vDSP | Native corpus, app UI and complete offline modified-library/app relink qualified locally |
| macOS x64 | vDSP | Native build/corpus and packaging require the Intel CI runner; NASM is a build tool |
| Windows x64 MSVC | KissFFT | Requires MSVC, the verified MSYS2 installation and NASM; runtime/package CI remains separate |
| Linux x64 / arm64 | KissFFT | Host corpus/package CI remains separate; x64 requires NASM |
| Android arm64 | KissFFT | NDK API 24 build/package evidence; actual device runtime comparison remains separate |
| iOS arm64 simulator | vDSP | Unsigned simulator preview build/package evidence; no physical-device qualification |

The source publisher requires all seven real target manifests before producing
the public source asset. Exact-head GitHub code review, trusted platform CI,
merge and release remain separate gates; none is asserted by local results.

## Exact-prefix review correction

The first GitHub code review found that a decoded chunk ending exactly at the
120-second limit did not stop the loop. One more chunk was decoded even though
the fingerprint was complete. The completion comparison now includes equality.

A generated 121-second FLAC fixture uses one-second frames at the normalized
11,025-Hz mono sample rate. Only its final frame is corrupt. The actual Rust API
failed on that trailing frame before the correction and succeeds afterwards,
reporting 120 seconds of processed frames, the full 121-second duration and an
unchanged source file. All fourteen original reference fingerprints and durations
still match; no frozen expected value changed. Fixture construction and hashes
are recorded beside the tests. This is actual producer evidence, not a mocked
chunk counter or an original-runtime equivalence claim.

Fresh post-correction checks passed all 1,046 workspace tests with zero failures
and six existing ignores, all fourteen native producer tests, Rust formatting
and full-workspace/all-target Clippy with warnings denied. The frontend, source
builder and release tooling are unchanged by this correction; their earlier
qualification retains its stated source scope.

## Source configuration and mobile notice follow-up

The Windows CI repair introduces a tracked Cargo linker configuration. The
source exporter previously refused any existing `.cargo/config.toml`. It now
preserves that file as an ordinary Git-bound source blob and adds the separate
`.cargo/opentag-offline.toml` overlay. Rebuild commands explicitly select the
overlay, so vendored crates and original target settings apply together. The
reserved overlay name still cannot replace a tracked project file; verification
requires its exact generated bytes and rejects modified project settings even
when an outer file inventory is rehashed.

An actual Cargo control built and ran one owned, registry-dependent program
offline with an empty Cargo home and the retained read-only vendor directory.
The program confirmed that the original project environment setting survived
alongside the vendor overlay. This checks configuration composition, not a new
full-application or seven-target relink claim. Source-bundle checks pass 21
cases and the release suite passes 27.

The About source button now routes a rejected browser request through the
existing dialog error display. Android/iOS return the releases address in that
request's error; the notice therefore shows the address instead of silently
discarding it. Tests execute the actual About handler, dialog error function
and public-message normalizer with controlled rejected native requests for
both mobile contracts and a successful desktop request. All 389 UI/model tests,
the production build, strict 172-page guide and eight publication tests pass.
These are shared-handler controls rather than mobile-device browser tests.

## DSDIFF review verification

A further review suggested that `.dff` required a separate `dsdiff` demuxer.
The pinned FFmpeg 9.0.2 archive has no such registration: its enabled `iff`
demuxer recognizes DSDIFF `FRM8`/`DSD ` input and implements its metadata and
packet reader in `libavformat/iff.c`. No dependency configuration change was
needed. The archive SHA-256 remains
`8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e`.

A new reproducible, project-authored twelve-second stereo DSD64 carrier is
accepted by the actual tag reader and embedded producer on the original
reviewed configuration. Its fingerprint and full duration match a fresh run
of the independently hashed Chromaprint 1.6.2 CLI; the input remains unchanged.
The fixture directory retains its generator, raw/carrier hashes and reference
capture. All fifteen native crate tests pass, including the unchanged fourteen
original golden media cases, VAPC and exact-prefix regressions. This is a
macOS arm64 API verification; the normal CI target gates remain required.

## Windows source-fixture newline correction

The first trusted Windows desktop run reached the source/release fixture test
and rejected a recipe hash mismatch. Its synthetic Git repository used Python
text writes, which insert CRLF on Windows, while its test manifest hashed the
original LF strings. The fixture now writes its declared UTF-8 bytes directly.
A controlled Windows-text-write regression first reproduces the same
`Native recipe/source input differs` failure, then builds and verifies the
source archive with all seven explicitly synthetic test manifests after the
correction. Production source verification and native dependency hashes are
unchanged; the real native recipe paths already require LF via `.gitattributes`.

The native jobs also exposed a typo in the unexercised empty-download-cache
path: `tempfile.NamedTempFile` is not a Python API. It now uses
`NamedTemporaryFile`, and closes the file before cleanup even on transfer
failure so Windows can remove it. Real loopback HTTP tests cover acquisition,
hash verification, extraction, checksum rejection and HTTP-error cleanup with
empty caches; all ten native source-safety tests pass. A separate fresh
acquisition downloaded all four actual pinned upstream archives and verified
their declared hashes. Existing archive/cache integrity checks remain intact.
The source-bundle suite passes 22 cases, release tests 27 and CI-tooling tests
86; workflow lint passes with the documented `queue` exception.

The changed recipe rebuilt all native libraries on macOS arm64. Its 216
installed files pass manifest/hash verification, retaining 218 audio decoders
and zero protocols. All fifteen native producer tests pass, including the
fourteen unchanged golden media cases, DSDIFF and exact-prefix coverage;
affected all-target Clippy also passes. This is host qualification of the
corrected recipe; trusted platform CI must validate the new final head.

## Native platform validation follow-up

The next trusted run passed Android document integration and the Linux/macOS
native UI suites. Windows reached the native source build and exposed a
version-specific library-name assumption: pinned zlib 1.3.2 declares Windows
static output `zs`, rather than `zlibstatic`. The source builder now stages
`zs.lib` under the `z.lib` name needed by FFmpeg and Cargo. Full Windows
validation remains required on this corrected head.

The following Windows run passed that staging step but failed FFmpeg's C
compiler probe. The builder passed backslash-separated Windows paths into
Bash; upstream configure locates its source directory with Unix `dirname`.
The configure command now uses forward slashes for the script, installation,
include and library paths. A regression exercises the actual Bash directory
operation, and failed native builds now include the compiler/configure log
instead of only its generic failure message. These twelve source-safety tests
pass locally; the corrected MSVC build still requires its trusted CI run.

The captured compiler log then identified `LNK4044`: the initial linker probe
receives extra flags verbatim and rejects Unix `-L` on MSVC. Windows now uses
`-libpath:` while other targets retain `-L`. The pinned FFmpeg MSVC flag filter
also maps `-lz` to `zlib.lib`, whereas Cargo requests `z.lib`; both names are
staged from the same verified `zs.lib` archive. Fourteen source-safety tests
cover these target-specific command and archive contracts. Full MSVC native
and packaging validation remains a required CI gate.

With those flags corrected, the Windows desktop job completed all four
native library builds and reached source-closure collection. Its MSVC
`-showIncludes` records contain headers only: a translation unit such as
`adpcm_data.c`, which includes only a system header, produces an empty record
after upstream excludes SDK paths containing spaces. The collector now binds
each MSVC record to its existing compiled object and unique pinned translation
unit, verifies any `src/` copy against that source, and retains the reported
local headers. Missing objects, missing/ambiguous source units and changed
copies still fail. GNU dependency handling remains strict and unchanged.
Sixteen source-safety tests pass locally; this does not substitute for the
remaining Windows app/API/package gates.

Final review also caught a copied-header provenance gap. Every dependency
record naming `src/` now requires the actual compiler-input alias to exist and
match the pinned file before its original hash is accepted. The regression
rejects altered and missing header aliases for both GNU and MSVC records,
then accepts an identical copy, including NASM's `src//` spelling. All seventeen source-safety tests pass, and
the stricter collector accepts the existing real 530-record x86 object corpus
under both record mappings. This audit does not claim a Windows app build.

The next Windows job passed the native builds and provenance collector, then
MSVC emitted D9007 while compiling the adapter: `/external:I` requires an
explicit `/external:W` level, otherwise the vendor include directory is
ignored. The adapter build now supplies `/external:W0` with that include path,
matching the external-header warning policy already used on other platforms.
Windows app/API/package validation remains required on the corrected head.

After language switching merged into main, the fingerprint setup, notices and
backend diagnostics were registered in all three catalogs. The extractor now
discovers the native crate's `std::error::Error` / `Display` messages, including
`write_str` literals. Thirty-one new keys have Dutch and German translations;
obsolete installation instructions were removed. The optional-override path
uses the existing raw `path` role, and decoder details retain their raw
`value` role. Regressions cover extraction/snapshot agreement, translated setup
and all native error variants, plus literal paths/details containing `$&` and
placeholder-looking text. The fingerprint algorithm and oracle inputs did not
change.

Linux compiled and ran the producer, but its Vorbis result differed from the
ARM golden by thirteen raw bits across 75 items. A local x86 source build
reproduced that exact Linux result. The unmodified upstream fpcalc source was
then compiled independently against the same codec/transform libraries, with
file input enabled only in a separate forensic transport library. All 431
Chromaprint source files were checked against the pinned original archive.
The independent upstream x86 CLI produced exactly the same Vorbis fingerprint
as OpenTag's x86 API; its thirteen other case fingerprints and all fourteen
durations match the original ARM oracle.

The original ARM expectations and all input/carrier hashes remain unchanged.
Tests now use the independently frozen x86 reference only for that one
architecture/input combination, retaining exact comparisons with no numeric
or fingerprint tolerance. Both ARM and x86 actual APIs pass all fifteen native
tests locally. The committed developer capture tool reproduces the original
CLI comparison and records source, library and executable identities; normal
tests execute no media program. No production fingerprint algorithm changed.

The two lowercase-leading zlib/bzip2 license headings are retained through the
explicit extra-key manifest. The catalog regression reads every component
heading from the real notice metadata and requires generated English plus
Dutch/German translations, preventing partial notice localization.

The first combined-head CI run found two frontend harness portability limits.
The history-label source boundary assumed LF, so Windows CRLF read 254 labels
instead of the same 21 production labels. The test now checks both endings
with the same assertions. The macOS DOM suite exhausted Node 22's 2 GiB heap;
that abort was reproduced locally. The frontend runner now caps its heap at
4 GiB and executes one isolated test file at a time to bound peak memory.
All 547 frontend tests pass with that budget. This changes test resources, not
application memory behavior or assertion coverage.

The combined native UI run also found legacy source-dialog selectors left
behind by language switching. Dialogs now derive their accessible name from
the live `aria-labelledby` title. The six affected suites resolve that name
before checking the same expected provider/source, including repeat shortcuts.
The web-source fixture checks its exact authored `Artist`/`Album` labels and
their nontranslation markers. A DOM regression exercises each suite's actual
name resolver with live title changes, a stale `aria-label`, authored case and
a broken binding. Application source-dialog behavior is unchanged.

The subsequent run passed every platform's tests and native UI checks, then
Windows built its release executable but WiX `light.exe` failed at MSI linking.
Tauri 2.12.2 hides the tool output at normal verbosity. Inspection found its
plain-license converter inserts UTF-8 text into Windows-1252 RTF without Unicode
escaping. Replaying that conversion through a Rich Text reader corrupted the
notice's own navigation arrow into `â†’`. That navigation line now uses ASCII
`Help > About OpenTag`; all seven original component notice bodies are intact.
The complete corrected notice round-trips through the same RTF conversion
without text loss. A regression guards the converter's input contract and
full component notices. The Tauri npm entry point enables verbose tool output
for future bundler diagnostics. Actual MSI success still requires Windows CI;
no installer, validation, signing or checksum gate was removed.
