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
Final ASM desktop relink, complete workspace/Clippy checks, modified-library
source-package relink proof and other platform CI remain separate pending
qualification. No release or merge is asserted by these local results.
