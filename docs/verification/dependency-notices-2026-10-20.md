# Source-bound Rust and JavaScript dependency notices

Unmerged L76 slice. This repairs the confirmed Rust/JavaScript material gaps
and supplies a reproducible component-to-notice inventory. It does not close
L76's native-library policy, platform helper/JVM or final package-delivery work.
P2 reproducible rebuilds and a legal compliance determination are separate.

## Inventory and source boundaries

[`dependency-inventory.json`](../../src/licenses/dependency-inventory.json)
contains 444 crates.io package candidates and four locked NPM components.
[`THIRD_PARTY_DEPENDENCIES.md`](../../THIRD_PARTY_DEPENDENCIES.md) renders their
index and 302 distinct notice texts, deduplicated by SHA-256. Every component
retains its exact version, archive checksum/integrity, declared license and
material paths/hashes. Archive bytes are checked against Cargo.lock or NPM SRI
before any material is read. No downloaded package is executed.

The Rust set follows normal dependency edges from the application root in
actual Cargo metadata captured for all seven release targets, with
`tauri/custom-protocol`. Build/dev edges and traversal through proc-macro crates
are excluded. A shared dependency still belongs to the set if reached through
a normal path. These are conservative source-defined candidates, not a claim
about every linked or dead-stripped function. The complete lockfile includes
602 registry packages; it must not be described as 602 shipped runtime crates.

| Target | Candidate Rust packages |
|---|---:|
| Windows x64 | 294 |
| macOS arm64 | 298 |
| macOS x64 | 299 |
| Linux x64 | 364 |
| Linux arm64 | 363 |
| Android arm64 | 269 |
| iOS arm64 simulator preview | 278 |

The union is 444, not the sum of overlapping target rows. Runtime JavaScript
is `@tauri-apps/api` 2.12.3 and `@tauri-apps/plugin-dialog` 2.8.2. Vite 7.3.7
and esbuild 0.28.2 are explicitly labeled frontend helper providers because
generated support code can reach the frontend; they are not described as
runtime imports, and their entire build-tool dependency graph is not relabeled
as shipped code. Material comes from the exact package-lock tarballs, rather
than the differently versioned installed node_modules.

The collector preserves nested material, including ring's BoringSSL/other-bits,
once_cell and fiat notices, encoding_rs's WHATWG notice and webpki-roots's
license. It also records each applicable crate's actual `UNLICENSE` file;
global deduplication never replaces per-component source membership.

Thirty-three candidate crate archives contain no matching notice filename.
They receive explicit bounded supplements instead of an invented generic
copyright notice: matching published VCS root files, the same-VCS parent
libappindicator archive for its sys member, or selectors' actual MPL source
header plus the full official MPL 2.0 text. The dialog NPM package supplies
SPDX metadata; its supplemental full texts are pinned to the registry-reported
upstream commit. The [supplement map](../../scripts/notices/supplements.json)
and retained source files bind these choices to archive hashes, VCS/source
references and exact bytes. These are source-provenance records, not new license
interpretations. Unknown/missing material refuses collection.

## Regeneration and checks

The [collector](../../scripts/dependency-notices.py) captures metadata itself:

```sh
python3 scripts/dependency-notices.py --collect-metadata /tmp/taggroove-notice-metadata
```

Collection is offline by default; `--allow-network` permits Cargo to fetch
missing registry metadata. It does not compile an application. Each snapshot
binds the current Cargo inputs, target and feature set. Reusing a snapshot
after changing a manifest refuses before publishing anything. Ordinary source
fingerprints normalize JSON/TOML structure and checkout newlines, and ignore
only the application's routine release-version stamps.

Provide the checksum-bound Cargo registry cache and the exact locked NPM
tarballs. NPM filenames use `scope-name-VERSION.tgz` or `name-VERSION.tgz`;
their resolved URLs and integrity values are in package-lock.json.

```sh
python3 scripts/dependency-notices.py --generate --metadata-directory /tmp/taggroove-notice-metadata --cargo-cache /path/to/registry/cache/index.crates.io-DIRECTORY --npm-cache /path/to/locked-npm-tarballs
python3 scripts/dependency-notices.py --check
python3 scripts/dependency-notices.py --check --verify-material --metadata-directory /tmp/taggroove-notice-metadata --cargo-cache /path/to/registry/cache/index.crates.io-DIRECTORY --npm-cache /path/to/locked-npm-tarballs
```

The ordinary check needs no network or package caches: it verifies current
source pins, component/target membership, material hashes, supplemental bytes
and the exact attached outputs. Material replay separately reopens the actual
archives and verifies their checksums and file provenance. New dependencies or
unmapped material require regeneration rather than passing a fixed notice count.
The common notice file contains a delimited generated section; manual notices
outside it remain. Displayed text normalizes line endings; the inventory retains
the original UTF-8 material text and its exact source hash.

All output/backup bytes are staged before any replacement. A late refusal or
handled publication failure preserves/restores original outputs. Rollback only
replaces an unchanged produced file; outside or unobservable state retains an
original recovery copy and reports its path. This is observed ownership, not
an all-files atomic or power-loss transaction. Empty new output directories
can remain after a refusal.

## Bundled surfaces and evidence

About imports the exact generated payload alongside the existing native and
locale-data notices. Upstream text is marked `translate="no"`; controls and
summary labels still translate. A production-DOM regression reproduced a
notice equal to the owned label `Close` becoming `Sluiten`, then verifies the
notice remains literal while the actual button translates. This also avoids
running the translation matcher over the large notice corpus.

The canonical common file and generated escaped RTF contain the same payload.
The RTF path preserves copyright/Unicode, literal control-looking strings and
upstream form-feed page separators. Raw supplement files keep their exact
bytes across checkouts; generated outputs have explicit LF rules.

Focused evidence includes normal/build/dev/macro boundaries, nested archive
material, unsafe/duplicate/link refusal, old-graph refusal, release-stamp and
checkout portability, missing/tampered components/text, detached outputs,
late publication failure, unobservable readback recovery and NPM/helper roles.
Real collection/replay covers all 448 component archives/material bindings.
Final focused checks pass: eleven Python collector cases, thirteen Node
notice/DOM/catalog cases, TypeScript, catalog extraction, ledger consistency
and whitespace checks. A real archive/material replay verifies all 448
components and 302 texts. The strict guide build and independent source,
material and documentation reviews are clear for this slice. No
application/native UI replay or CI run was started here.

The source tree and generated payload are not proof that a new package has
been published. The historical v0.2.56 artifacts remain separate baseline
evidence. Final release binding, native GPL-policy repair, JVM/installer-helper
material and actual postmerge package inspection remain required L76 work.
