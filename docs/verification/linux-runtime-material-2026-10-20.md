# Linux runtime source and notice material

This working-branch work advances L76. It does not close the Linux package/source
mapping, publication, native integration or reproducible-build qualifications.

The tracked `scripts/notices/linux-runtime-material.json` selects eight complete
source archives: musl, zstd, zlib, mimalloc, FUSE, squashfuse, type2-runtime and
AppImageKit. It binds 89 retained raw files, including 13 complete notice members,
dependency recipes, patches and producer observations. The runtime recipe binds
FUSE and squashfuse source checksums; the Alpine recipes bind dependency archives
and patches. The AppRun source selection is the standalone C target at the
recorded immutable source commit, rather than an assertion that the entire
AppImageKit checkout builds without its submodules.

`scripts/linux-runtime-material.py` verifies the raw-file and source-archive
identities and the selected archive members without extracting or executing
supplier code. The corresponding-source builder includes all eight original
archives. Its offline verifier requires the exact archive set against the
Git-bound descriptor and raw files; changing an archive and rehashing the outer
manifest does not satisfy that requirement. Local archive input must contain
exactly the eight selected names, with no network fallback for missing files.

The notice generator supplies the complete selected text in `THIRD_PARTY_LINUX.md`,
the combined Markdown/RTF notices and the existing in-app native/runtime notice
card. One guarded publication covers all generated outputs; unknown edits refuse.
No abbreviated identifier is substituted for the selected notice text.

## Focused evidence

- Independent read-only review found no actionable issue in the runtime material,
  notice delivery or source-archive integration.
- A parent replay verified the eight actual pinned archives, totaling 10,975,124
  bytes, against all retained raw material. The generated notice payload contains
  97,014 characters. These are archive/data checks, not app execution.
- Eleven helper regressions cover changed archives/raw files, archive members,
  duplicate/unsafe paths, bounds, offline behavior and recipe checksum bindings.
  The source-bundle suite includes missing runtime material and changed archive
  bytes even after the outer archive manifest is regenerated.
- The focused notice/installer tests and strict guide build passed. No local
  application build, downloaded runtime execution or native UI sweep was used.

## Pinned packaging inputs and replay

`scripts/linux-build-tools.py` prepares exact hash-pinned linuxdeploy, strip,
patchelf, AppRun and type2-runtime inputs under `target/linux-source-tools`.
The CLI version and integrity must match `package-lock.json`. Preparation uses
the pinned host archive inspector; it never runs an extracted packaging tool or
AppImage. It verifies every input before guarded publication and refuses changed
managed files and symlink output parents. A private XDG cache is returned for
the builder to use; preparation does not change the user's global cache.

The separate distribution-file replay runs only the pinned host strip/patchelf
tools against temporary copies and requires complete resulting bytes to equal
the packaged file. The explicitly selected WebKit recipe additionally reproduces
Tauri's literal `/usr` to `././` replacement. It does not discard arbitrary ELF
sections, symbols or metadata to make a comparison pass. Static extraction of
both actual linuxdeploy assets verified the pinned strip/patchelf member bytes;
no Linux executable was run on the local macOS host.

## Remaining limits

Retained producer logs and fresh APK metadata bind selected source recipes; they
do not authenticate historical APK signatures or establish exact historical
producer APK bytes. Runtime source/asset hashes do not prove a reproducible build,
complete compiler/CRT provenance, application relinking, or legal sufficiency.
The per-builder Linux package map and final release integration are still in
progress. Actual new target packages and the published source asset remain
separate evidence requirements.
