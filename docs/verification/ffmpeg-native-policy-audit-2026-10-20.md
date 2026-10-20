# FFmpeg AArch64 source-policy audit

L76 finding in progress. This is static source/artifact evidence, not an
execution trace or legal determination. A source patch cannot change the
already published artifact described below.

The hash-verified v0.2.56 release binds source commit
`754462546eb704f14dad300befdb9740885ab523`. All seven native wrapper hashes agree
with the release/source manifests. Their actual FFmpeg compiler dependency
closures have a 1,051-file union; every named file matches the pinned official
FFmpeg 9.0.2 source archive, SHA-256
`8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e`.

Three files in that compiled-source union have explicit GPL-2.0-or-later
headers, despite the recorded `CONFIG_GPL=0` setting:

- `libavcodec/aarch64/ac3dsp_init_aarch64.c`
- `libavcodec/aarch64/ac3dsp_neon.S`
- `libavutil/aarch64/crc.S`

The current AArch64 Makefiles select AC3 DSP objects through AC3DSP/NEON and
the CRC object through ARMV8. Those entries do not additionally require GPL.
The C dispatch code likewise reaches those architecture implementations.
The project explicitly promises a no-GPL native profile, so configuration
flags alone do not supply adequate source-policy evidence.

The downloaded Apple Silicon app asset has SHA-256
`e963b3de8b88a3ce9752f34d96628455fba7ff4dbec338e19103cc896c94ab57`.
Its 43,391,968-byte executable has SHA-256
`2b68e57170c26fef6dbc629a6ad29fd919e486e962bef8cc807412ad0afba7ef`.
Read-only symbol inspection finds all nine routines named by the three files:

| Routine | Defined address in this executable |
|---|---|
| `ff_ac3_exponent_min_neon` | `0x100938310` |
| `ff_ac3_extract_exponents_neon` | `0x100938350` |
| `ff_float_to_fixed24_neon` | `0x100938380` |
| `ff_ac3_sum_square_butterfly_int32_neon` | `0x1009383b0` |
| `ff_ac3_sum_square_butterfly_float_neon` | `0x100938400` |
| `ff_ac3dsp_init_aarch64` | `0x1014e8aa0` |
| `ff_crc32_aarch64` | `0x100a5cfa0` |
| `ff_crc_neon_pmull` | `0x100a5d010` |
| `ff_crc_le_neon_pmull` | `0x100a5d230` |

This establishes defined-symbol presence in the pinned binary; it does not
establish which routines ran on a particular CPU or file.

The required source repair preserves all decoder/demuxer choices and unrelated
SIMD. Gate the two codec Makefile entries and utility CRC object on GPL, and
gate the corresponding generic-C architecture dispatches. CRC's include,
initialization, table-selection and execution branches must change together:
gating execution alone can leave architecture sentinel tables incompatible
with generic C. Existing generic AC3 and CRC fallbacks remain available.

Four hash-pinned source targets suffice: the two AArch64 Makefiles,
`libavcodec/ac3dsp.c` and `libavutil/crc.c`. The three GPL-header implementation
files themselves need no rewriting or relabeling. Patch/input/closure
verification must retain and reconstruct every modified file, including the
Makefiles outside compiler dependency lists. Actual rebuilt native wrappers
and AArch64 artifacts must then demonstrate excluded source/object/routine
families, retained generic behavior and unchanged format coverage. That work
is not certified by this audit or by the prior no-GPL configure flag.
