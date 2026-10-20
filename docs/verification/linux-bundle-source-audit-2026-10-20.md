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

## Approved implementation and actual material controls

The owner approved extending the existing source pipeline to retain verified
distribution source archives/patches and AppImage runtime/launcher source and
notices, bound to both Linux builds, without duplicate application builds or UI
runs. Implementation is in progress; this section does not close L76.

The [source-material collector](../../scripts/linux-source-material.py) authenticates
Release signatures using GnuPG in an isolated temporary directory and a fixed
Ubuntu archive signer. Supplying another public key does not extend that trust
policy. It checks compressed index hashes, selects exact binary/source versions,
and verifies every source archive, patch and `.dsc` byte. The `.dsc` identity and
complete declared file set must agree with the authenticated source index.

Index downloads use the signed SHA-256 `by-hash` address, so a repository update
between requests cannot silently substitute a new index. Binary and source
components are resolved independently: Ubuntu can publish a binary in `universe`
whose exact source is in `main`. Conflicting source selections refuse. Only
source material and repository proofs enter the retained material set; verified
binary packages remain capture inputs for the later packaged-file comparison.

Changed or missing patches, ambiguous versions, duplicate metadata fields,
detached descriptions, incorrect signatures and valid signatures from other
keys refuse. Ten focused tests include real GnuPG authentication and signed
transport fixtures for amd64 and arm64. They run through the existing premerge
headless discovery. Packaged-file mapping and publication integration remain
unfinished; this collector is not a completed new-release source delivery.

The [binary member reader](../../scripts/linux-package-files.py) checks the
authenticated `.deb` hash and its package/version/architecture/source control
identity before reading original file bytes. It streams the control and data
tar archives through `dpkg-deb`, without installation or maintainer-script
execution. Link resolution stays within the archive; missing/escaping/cyclic
aliases and duplicate member paths refuse. Control records are capped before
their bodies are read. Four package regressions join the ten repository cases
in the same focused premerge discovery. A compressed 24 MiB control fixture
reproduced the initial buffering defect and now refuses within the allocation
bound. Independent re-review is clear.

The [structured controls](linux-source-controls-2026-10-20.json) retain exact
input identities and limits:

- Six actual Ubuntu package indexes were verified against signed Release files
  with GnuPG and the official archive key fingerprint
  `F6ECB3762474EDA9D21B7022871920D1991BC93C`, published in
  [Ubuntu's integrity guidance](https://documentation.ubuntu.com/security/software-integrity/image-verification/).
  The selected source index was verified through the same chain.
- GTK `3.24.41-4ubuntu1.3` supplies an original archive, a distribution archive
  with patches/packaging, and a `.dsc`. All three downloaded files pass the new
  production material verifier. The control does not establish that this is
  the only historical binary package with equivalent code.
- An actual signed, digest-addressed capture resolves `libarchive-tools`
  `3.7.2-2ubuntu0.9` from the `universe` binary index to its exact `main` source
  record. The collector verifies all source/patch material and the 72,478-byte
  binary package, retaining 11 source/repository proof files. This exercises
  cross-component selection on supplier data, not just synthetic fixtures; it
  does not establish that this binary is bundled in the app.
- The actual authenticated GTK `.deb` resolves
  `usr/lib/x86_64-linux-gnu/libgtk-3.so.0` to its regular
  `libgtk-3.so.0.2409.32` member. Its 8,156,632 original bytes match the retained
  original input exactly. This verifies package identity and alias/data reading;
  it does not yet verify packaging's ELF transformation or map the whole image.
- The actual packaged GTK has a different full hash from that package's
  original library. Its 6,885 dynamic symbol records, 26 other section records
  and original dynamic strings agree after resolving metadata references; the
  canonical dynamic table adds `$ORIGIN` as RUNPATH. This is one static control,
  not a general proof of ELF transformation safety or all program-header/load
  relationships. The collector should replay the exact bundler tools and
  require full resulting bytes to match, rather than relying on this comparison.
- The image's 944,632-byte runtime prefix matches the published type2-runtime
  binary for commit `fedfad51acc2860b2916a1eef32ba4a500ff31ef`, except its
  16-byte `.digest_md5` field. The runtime's root license lists statically linked
  suppliers; its Makefile also links mimalloc, which needs its own retained
  material. Root MIT wording alone does not finish this runtime audit.
- The top-level `AppRun` is a generated shell wrapper. `AppRun.wrapped` matches
  the publisher's 31,552-byte AppRun mirror exactly; a mirror pointing at a moving
  upstream release does not establish an immutable producer-source revision.

These checks read archive data and metadata. No downloaded application, runtime,
launcher or Linux inspection executable was run, and no local app build or UI
replay was performed.

## Working-branch capture and release integration

The working branch adds `scripts/linux-distribution-capture.py`. On each existing
Linux builder it inventories installed package ownership, selects authenticated
exact binary/source records and reads the selected original members. Every
packaged file needs an explicit distribution, repository, application-build,
generated-template, runtime/launcher or named generated-cache provenance route.
Unknown and ambiguous mappings refuse. Changed ELF bytes require complete replay
with pinned strip/patchelf tools and recipe-derived arguments; the packaged file
cannot choose its own RPATH. The capture retains the selected distribution
sources, patches, packaging instructions and repository proofs.

Both target manifests and digest-addressed objects enter the corresponding-source
archive. Offline verification reauthenticates repository/source selections,
checks every retained object and compares captured configuration, recipes and
repository assets against the archive's proven Git source. Member extraction,
installed-file comparisons and full-byte replay remain trusted builder
observations, rather than an independently repeated offline extraction or build.
The four named generated caches retain complete deployed input sets and their
authenticated generator source provenance; cache regeneration is not claimed.

The release workflow prepares pinned inputs before its existing application
build, captures afterward and passes both captures to source assembly. The final
publisher requires the external sidecars to equal those inside the source
archive, matches each AppImage hash/size and admits only the expected two Linux
sidecars. It keeps the existing 16 public release assets; source material and
sidecars live inside the source archive. No second application build or native
UI test is added by source capture.

Focused integration uses a real Git tree, two synthetic target captures and
actual GnuPG-authenticated fixture metadata. The complete source archive and
release assembler accept the positive fixture and refuse changed sidecars,
tampered summaries, unreferenced rehashed objects, mismatched release/image
identities and unexpected sidecars. The fixture substitutes only its test signer;
production retains the fixed Ubuntu signer. This proves the new data/gate paths,
not successful capture of a new native release. Actual package/source capture,
postmerge native integration and publication remain required. The separate
[runtime source receipt](linux-runtime-material-2026-10-20.md) retains producer
and reproducibility qualifications.
