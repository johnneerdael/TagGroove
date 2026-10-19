# Web-source duration compatibility: pinned static contract and local checks

The reserved web-source `_LENGTH` integer conversion now follows the statically
observed Mp3tag 3.36.1 Windows x64 contract. Ordinary `LENGTH` remains editable
metadata. This note records the implementation checkpoint and its local checks;
it does not establish original-application, provider or physical-save parity.

## Native evidence

The inspected `Mp3tag.exe` has SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
Retained Ghidra 12.1.4 analysis and complete instruction spans close these paths:

- `CMTMainFrame::WebSourceThread` at `0x1401ac628` passes actual parser output
  to the source converter at `0x1401a7c4c`. The converter extracts reserved
  `_length`, removes it from editable fields and populates separate technical
  duration properties when its list matches the source-track count. Ordinary
  `LENGTH` stays in the normal scalar or per-track field collection.
- The review initializer at `0x14022a450` reads technical duration separately.
  `CMTTagSourcesDlg::ApplyTrackData` at `0x1402296f4` copies selected ordinary
  fields through `0x140367784` and `0x140367af4`. The native ID3 writer at
  `0x14033ac7d`–`0x14033ac89` maps ordinary `LENGTH` to `TLEN`; it does not take
  that value from the technical duration property. This closes the static
  source-output → editable-field → ID3 mapping distinction, without executing
  review, Apply or Save in the original application.
- The converter's non-colon branch calls numeric helper `0x14053a4d8`, whose
  complete body calls `0x14053852c` with base 10 and signed conversion enabled.
  It consumes a decimal prefix, accepts one optional ASCII sign and leading
  zeros, and saturates signed overflow at the 32-bit limits. No digits means
  zero. A decimal point, exponent marker or other unrecognized character ends
  the prefix; there is no floating-point fallback.
- At `0x1401a8283`–`0x1401a82ca`, integers above 6000 are integer-divided by
  1000. Only a positive result reaches technical-duration setter
  `0x140366d74`. Positive integers through 6000 are already seconds.

The numeric core explicitly accepts ten decimal digits starting at each of
these 18 BMP code points:

`0030`, `0660`, `06F0`, `0966`, `09E6`, `0A66`, `0AE6`, `0B66`, `0C66`,
`0CE6`, `0D66`, `0E50`, `0ED0`, `0F20`, `1040`, `17E0`, `1810`, `FF10`.

This list includes ASCII and 17 other blocks; it is not every Unicode decimal
block. The pinned whitespace table accepts U+0009–U+000D, U+0020, U+0085 and
U+00A0. Whitespace is skipped before the optional sign, not after it.

The source-converter evidence manifest has SHA-256
`aea959b587318df839aac2ae28e341c1ff54f6909d8105330722362751cc0f3a`;
the numeric-helper manifest has SHA-256
`904e13545c6f952d0da5dd96808434ee04799a479eb9cfa816eaceb8b00e1264`.
Raw proprietary analysis and private scripts remain outside Git.

## Implemented behavior

[The source adapter](../../src-tauri/src/sources/web/mod.rs) retains the public
`SourceTrack.length` representation in seconds and the existing colon branch.
Its non-colon conversion implements the signed prefix, explicit digit ranges,
saturation and integer duration rule above. Literal expectations independently
derived from the native control flow include:

| Reserved `_LENGTH` text | Result in seconds |
| --- | --- |
| `0`, `-6000`, `nonnumeric`, `0x1770`, `+ 6000` | Missing duration |
| `5000` | 5000 |
| `6000`, `006000`, `+6000`, `6000.9`, `６０００`, `٦٠٠٠` | 6000 |
| `6001`, `6001.5` | 6 |
| `61500` | 61 |
| `320357`, `320357.9` | 320 |
| `1e3` | 1 |
| `2147483647`, `2147483648`, a longer positive overflowing prefix | 2147483 |
| `-2147483648`, `-2147483649` | Missing duration |

Ordinary `LENGTH` follows the existing editable-field pipeline and does not
supply `SourceTrack.length`. A scalar `LENGTH` or reserved duration list alone
does not create source tracks. Public types, parser-error provenance and other
provider adapters are unchanged. The [source reference](../user-guide/tag-sources/web-source-reference.md)
documents the duration convention and the ordinary/reserved distinction.

## Local verification checkpoint

Actual Rust producer regressions were run before the production helper changed:
three semantic failures and one passing control, exit 101. The old adapter
returned zero as a duration and interpreted `6000`/`6001` as 6/6.001 seconds.
After the fix, all four regressions passed. They exercise actual engine output
and release conversion, integer spellings and boundaries, digit blocks,
overflow, colon controls and scalar-versus-track cardinality.

[The shared literal fixture](../../tests/fixtures/web-source-length-contract.json)
contains both script output and the complete expected release DTO. Rust checks
actual engine → release serialization against it.
[The Node tests](../../tests/tag-sources.test.mjs) feed that DTO to actual
production matching, duration display, editable-field and apply-item consumers.
These are producer and consumer contract layers, not one joined process or a
physical-save proof for the new duration route. The Node consumer baseline
already passed because it already consumed seconds correctly.

| Check | Recorded result | Retained raw log |
| --- | --- | --- |
| Producer before fix | 3 failed, 1 passed | `05-producer-red.log` |
| Producer after fix | 4 passed | `07-producer-green.log` |
| Web-source subtree, including error-provenance and local-fixture controls | 94 passed, 9 existing ignores | `08-web-subtree-green.log` |
| Node production consumers | 32 passed | `09-node-consumer-green.log` |
| Strict guide build, isolated output | Exit 0 | `10-guide-green.log` |
| Scoped diff check | Exit 0 | `11-diffcheck.log` |
| Cargo formatting check | Exit 0 | `13-fmt-green.log` |
| Canonical catalog check | Exit 0 | `16-canonical-green.log` |

The later parent integration passed **1,369 Rust tests with zero failures and
18 existing ignores across 82 groups**, **828 frontend tests**, strict all-target
workspace Clippy, formatting, the production TypeScript/Vite build, canonical
catalog, eight documentation tests and the strict guide build. All four frozen
implementation hashes below were unchanged. These later results supplement the
author checkpoint without relabeling its original logs or binaries.

The implementation cohort's final receipt has SHA-256
`83a29437ea47504135377454b50946a0cd9244b2fce0b1d7be7a1c57b01d035b`,
and its artifact manifest has SHA-256
`1477f61f108f3d1ae6dfdd68704cda5424b0609fccea1623690e7b869a50c5a0`.
The before/after manifests bind 201 compile and consumer inputs; only
`src-tauri/src/sources/web/mod.rs` differs between producer RED and GREEN.
Their SHA-256 values are respectively
`9c1b4a805fd3ed1641109b6396eb48f8dd3e91de9fabb98c3b6b9325aee46509`
and `5b6d1084d2dcd48cd5acd4d32badf8ff54b9f949e5623529da45bb8c223bea8a`.
Binary receipts bind the executed RED binary
`3d76aaa42cbf690aa01af2c63ac86cfcb19198641aa26a6f2823c32db776786f`
and GREEN binary
`a8a2bc4b9b554f6fc3f8c37b39e04bc386bfcc6fdae3b5cc9b79bd647ab6723d`.
Cargo rebuilt the same executable path: the RED digest is retained, without a
separate RED executable copy.

The four frozen implementation inputs have these SHA-256 values:

| Input | SHA-256 |
| --- | --- |
| [Source and Rust regressions](../../src-tauri/src/sources/web/mod.rs) | `6b7dc9789dcb82c8cad15b72c7b597e19edda1c416a5f1aad4167f98bfa4171c` |
| [Shared fixture](../../tests/fixtures/web-source-length-contract.json) | `32f7180746dca7e4aced3212690474c48cbd032912fa6339a0002a9960bedc2c` |
| [Node consumers](../../tests/tag-sources.test.mjs) | `2856118ed5135b7a02d04b885f3a31afdc0c6a9fb60acecb7448bbfbccdf453b` |
| [User guide](../user-guide/tag-sources/web-source-reference.md) | `ee0f43e6ea6611c2b54d3de1237446b7678a83bc99170330f08446396afc91ca` |

## Remaining limits

Native outcomes above are static predictions from the pinned artifact, not
executed original results. No live provider, original executable, original GUI
or Windows runtime was used. Native whitespace classification at or above
U+0100 delegates to `GetStringTypeW`; complete classification and runtime
locale/table changes remain unobserved. The adapter's pre-existing Rust trim
is retained and is not an exact emulation of that Windows behavior.

Colon-containing native values take a separate branch that stores the original
technical-length string. Its downstream numeric interpretation is not closed
by the integer-helper proof. TagGroove's existing colon parsing remains local
adapter behavior, covered by regression controls without a broader native claim.
Supplementary-plane digit equivalence, malformed UTF-16, CRT errno side effects,
arbitrary native key-case variants, original review defaults and original
physical bytes are outside this checkpoint.
