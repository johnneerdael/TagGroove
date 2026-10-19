# Published source inventory: v0.2.46

This is a static audit of the published [v0.2.46 release](https://github.com/johnneerdael/TagGroove/releases/tag/v0.2.46), whose source commit is `179e705cad91ef42f85107310664409271eb2c4c`. It is not a release or build of draft PR64. No downloaded application was launched, installed or rebuilt.

## Actual artifacts inspected

The release API response, `release-manifest.json`, `SHA256SUMS`, corresponding-source archive and Apple Silicon app archive are retained under `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-release-license-audit-79sjol_p`. The standalone inspector is `/tmp/taggroove-release-license-audit.py`; `receipt.json` binds that inspector and every retained input/output by SHA-256. `final-receipt.json` additionally freezes a local inspector copy, the resolved artifact root and the independent review; its SHA-256 is `700067f78d98eb7a45fbc44853fee2e5c78452cd6ac7cee95bedefc48dd8cb03`.

| Download | SHA-256 |
| --- | --- |
| `release-manifest.json` | `1d669c2e80cabfa34dd1df244324f08d01d9fe90201b99478b7900d15a692bb6` |
| `SHA256SUMS` | `97d558db8cf2e0aa4923faf9f5996a4a25d533640de4437da5448d0d59750042` |
| `TagGroove-v0.2.46-sources.tar.xz` | `23ab1e4dd6bdaf03e67da65cf565ad0bc16865b6fa1aec0beff916486412198f` |
| `TagGroove-v0.2.46-macos-arm64.app.tar.gz` | `e7be6f48296d95927530016c45a5bbfe10358d9d320b1e25a2cff3a16dcb4f22` |

All four downloads match their GitHub asset digests. Both archives also match the downloaded checksum list and release manifest. The source archive was streamed without extracting its contents or executing scripts.

## Observed results

- All **33,491 source-package files**, totaling **1,135,230,770 uncompressed bytes**, match their manifest paths, sizes, modes and hashes. The manifest itself is separately hashed. Duplicate paths and unsafe/nonregular members are rejected.
- Source commit and version agree across release metadata, release manifest and source manifest. The source manifest names the same seven platform targets as the release manifest; this is a metadata binding, not execution on those targets.
- All **601 registry packages** in the shipped Cargo lockfile have matching vendored package names/versions and registry checksums. Each package's complete file/hash set matches its `.cargo-checksum.json`.
- Every one of those package manifests declares a license expression or license-file path. Each explicit license-file path resolves inside the package inventory. These are package declarations, not independent license determinations.
- The report lists **44 packages without a license-like filename** as manual inspection candidates. Their terms may be in README files, source comments or other locations. That filename heuristic does not establish missing notices or a licensing defect.
- The Apple Silicon app archive has its own hashed member inventory. Its `Info.plist` reports version `0.2.46`, executable `taggroove` and identifier `org.opentag.editor`. This does not verify its signature, notarization or launch behavior.

The detailed registry inventory, candidate list and archive inventory are in `license-inventory.json`; `source-file-inventory.json` retains every verified source-file entry.

An independent bounded review recomputed catalog totals, checked five package mappings and all seven target/hash bindings, rehashed the small manifests and macOS archive, and reread its plist. It did not repeat the full source-archive stream. Its receipt is retained under `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-release-license-independent-review-xnkoa2_3`, SHA-256 `ff636a5f471d283094faf193d433de0ba79f6b0b78dd13b65c6f0ff91b66d923`.

## Remaining scope

This closes the named **published Rust-source inventory and integrity check**, not L76 as a whole. The resolved lockfile includes build dependencies and packages for other targets, so it cannot identify every library actually linked into a particular binary. JavaScript dependency notices, nested native-source license content, binary-specific attribution/relinking requirements, other platform packages and reproducible rebuilds remain unaudited here. C36's pending Boost-notice repair is separate from this older release.

No legal compliance approval, complete dependency-license audit, seven-platform package verification or reproducibility claim follows from this inventory.
