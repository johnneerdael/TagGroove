# Linux bundle source inventory

L76 remains open. The [AppImage license-text repair](appimage-license-material-2026-10-20.md)
fixes broken references in bundled copyright documents. It does not establish
source delivery for the Linux distribution libraries copied into the image.

## Actual retained inputs

The v0.2.56 x64 AppImage has SHA-256
`81fe77912a172d0c0695820dc91545ac5bd5ba1c8e4382ed68cc47ef79dad880`.
It contains GTK and WebKit shared-library paths and their distribution copyright
documents. The two inspected documents retain these complete byte identities:

| Image member | Size | SHA-256 |
| --- | ---: | --- |
| `usr/share/doc/libgtk-3-0t64/copyright` | 60,525 | `2be2199cc990913cd94874b6f16fc84f1e3c35aaaa4719ad34b4478f1e69c557` |
| `usr/share/doc/libwebkit2gtk-4.1-0/copyright` | 273,584 | `1f0568073d22552e2ef947912b9ed1169e5b7132a50d54d520f227e9408c3890` |

Their headers point to the general GTK and WebKit upstream download locations.
Those URLs alone do not bind a binary package version to its exact distribution
source, including distribution patches and packaging instructions.

The retained v0.2.56 source manifest identifies main
`754462546eb704f14dad300befdb9740885ab523`, contains 33,650 file records, and has
SHA-256 `ae9d9261f31b9957dd2623c80e767fb420f4a0b9dc779a9af81adbbe08a69e7d`.
Its four upstream source archives are:

- `native/archives/bzip2-1.0.8.tar.gz`
- `native/archives/chromaprint-1.6.2.tar.gz`
- `native/archives/ffmpeg-9.0.2.tar.xz`
- `native/archives/zlib-1.3.2.tar.gz`

The manifest has no distribution source archives for the copied GTK/WebKit
libraries. Rust GTK wrapper sources in `app/vendor/` are separate software and
do not supply those C/C++ library sources. This is an inventory result, not a
determination of all licensing obligations or proof of a particular legal defect.

## Remaining P1 source-delivery work

Bind the copied distribution files to the producing builder's installed binary
package, exact version, source package and source version. Verify the copied
library bytes against that selection. Do not infer package versions from a
shared-library filename, copyright directory, or the package currently available
on another host.

Retain and independently verify the selected distribution source material,
including patches and build instructions, alongside the release's corresponding
source. Make the public source manifest cover that material and the exact Linux
package selection. Refuse publication when a copied component cannot be mapped
or its selected source is missing or changed. Keep this collection after the
existing normal package build; it needs no extra application build or UI replay.

Both Linux architectures need their own package/byte mapping. Shared source
archives may be deduplicated by their verified identities; an x64 observation
does not certify the arm64 image. Application relinking and native integration
remain separate checks, while reproducible rebuilds remain P2.
