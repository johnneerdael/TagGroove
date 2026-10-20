# AppImage shared-license material

Working-branch L76 repair. Static archive inspection found a missing-document
defect in the published v0.2.56 x64 AppImage. This receipt does not claim a new
package has shipped, license compliance for every component, or reproducibility.

## Confirmed baseline

The release AppImage is 97,294,840 bytes, SHA-256
`81fe77912a172d0c0695820dc91545ac5bd5ba1c8e4382ed68cc47ef79dad880`,
matching its GitHub release asset digest. Read-only 7-Zip inspection identifies
its embedded ZSTD SquashFS at offset 944,632. No application code was run.

It contains 115 regular `usr/share/doc/**/copyright` files but no
`usr/share/common-licenses` members. Its copyright files refer to 15 standard
filenames: Apache-2.0, Artistic, CC0-1.0, GFDL-1.2, GFDL-1.3, GPL, GPL-1,
GPL-2, GPL-3, LGPL, LGPL-2, LGPL-2.1, LGPL-3, MPL-1.1 and MPL-2.0.
Those references therefore cannot be followed within the image.

The locked Tauri CLI 2.12.2 bundler source, commit
`5b23c82ea5ba6b9aec420dd71f60493edc486aa9`, copies configured AppImage files
into the generated Debian data tree before copying its `usr/` tree into the
AppDir. The configuration maps destination `/usr/share/common-licenses` to
the backend-relative `../target/appimage-common-licenses/` directory. It uses
the existing bundler path instead of modifying the generated image afterward.

## Repair and publication guard

[`appimage-licenses.py`](../../scripts/appimage-licenses.py) stages the host's
standard documents as regular byte files. Host aliases must resolve inside
the source directory. Missing Mozilla texts come from retained SPDX
license-list-data v3.29.0 material, immutable commit
`31ba1a50e5397e00a304dbadc76531740e89ee48`. Exact SHA-256 and Git blob identities
are recorded in the [source descriptor](../../src/licenses/appimage-common/SOURCES.json).
All 15 known baseline names must be available before packaging; other host
standard documents are retained too.

The stage manifest binds each file's bytes and origin. Unknown files, outside
edits, unsafe paths and new files appearing between the ownership scan and
publication refuse the operation. The shared publisher stages all outputs
first, guards prior bytes or expected absence, and reverses handled replacement
failures where it still owns the result. This is not a power-loss transaction
or an atomic filesystem snapshot.

The Linux release jobs stage before the existing build and inspect afterward,
before collecting public artifacts. Inspection requires the exact staged
document set and hashes, reads every bundled copyright file, and refuses any
unresolved standard-text reference. It never launches the AppImage. The pinned
7-Zip 26.03 inspectors have independently verified archive and executable
SHA-256 identities for x64 and arm64; neither inspector is shipped with the app.

## Focused evidence and limits

The stage was exercised with the actual standard-license members extracted
without installation from Ubuntu's `base-files_13ubuntu10_amd64.deb`, SHA-256
`cd06729c776397bb74779773f4dece372a1a3f795e5992804022bffab0278ffb`.
The source was the official Ubuntu archive over HTTPS; this control did not
authenticate an APT repository signature. The host material plus two pinned
Mozilla fallbacks produces 17 staged documents. Checking the actual retained
v0.2.56 AppImage then fails with the expected missing-document error.

Focused tests exercise safe alias materialization, pinned fallbacks, repeat
staging, host updates, outside-file preservation, archive-member hashes and
reference closure, inspector integrity and recovery of its executable mode.
Injected initial-scan/publication and download/publication arrivals failed
before the expected-absence repair and now preserve the outside bytes. Read-only
independent review verified that repair and found no further concrete defect.

Local checks pass: six AppImage cases, 28 collector/component cases, 129 GitHub
tooling cases, 28 release cases, 12 affected headless notice cases, eight guide
publication cases, the strict guide build and the locked Tauri configuration
schema. Actionlint passes with only the documented `queue` syntax exception.
Premerge Linux CI now includes these focused Python notice/staging regressions;
archive verification stays after the existing package build.

These controls do not prove that newly built AppImages contain the staged
documents; the post-build release gate must supply that evidence on each Linux
architecture. The check covers reference closure for bundled copyright files,
not complete source offers, relinkability, exact linked dependency membership,
all licensing requirements, runtime behavior or reproducible rebuilds. Native
application UI was not replayed for this slice.
