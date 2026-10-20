# Linux repository authentication fixtures

`ubuntu-noble-updates-InRelease.xz` is an LZMA-compressed copy of the official
Ubuntu `noble-updates/InRelease` captured during the 2026-10-20 audit.
`ubuntu-archive-keyring.gpg` comes from:

- https://archive.ubuntu.com/ubuntu/dists/noble-updates/InRelease
- https://archive.ubuntu.com/ubuntu/project/ubuntu-archive-keyring.gpg

The production trust policy pins archive fingerprint
`F6ECB3762474EDA9D21B7022871920D1991BC93C`. This old signed fixture tests
signature integrity, not present-day repository freshness or update safety.

`untrusted-*` has a valid cleartext signature from a synthetic key outside that
trust policy. `repository-*` and `example_*` describe a tiny synthetic repository
used for real-signature transport tests under an explicitly scoped test trust
root. The `.deb.fixture` payload tests transport hashing, not Debian archive or
ELF parsing. The original/debian source fixtures are real small tar archives.
No private key is retained or committed. These keys never sign application
packages and do not extend the production trust policy.

Exact retained file identities:

| File | SHA-256 |
| --- | --- |
| `example_1.2-3.debian.tar.xz` | `80951cb31dd0d539a99cd42961ae0bf0a2c03b68813fa7305b5554204546840c` |
| `example_1.2-3.dsc` | `c610fae151200370bfa0f5687257135365da8fd2cf35698f375a1131c187c8c8` |
| `example_1.2.orig.tar.xz` | `7734dda1219c812037471e20b19e843df6f1a4f16cc672ae61ab8bca59aadfc9` |
| `libexample1_1.2-3+b1_amd64.deb.fixture` | `d168a358fb36983c089d30e3032c25fc23bc7ea2367950305a6548867489401f` |
| `repository-InRelease.asc` | `d7480f05d7954be71a617ec3e8bc2f9c1f51bbfc13aea170c4995505d2bc05d0` |
| `repository-Packages-amd64.xz` | `b6b180ed1d1679e8e883d3bc05cef0d1ccd344a46ef2be0e25b6bac185ba4a6f` |
| `repository-Packages-arm64.xz` | `b2f00cd87cfaa2547fbc4eca7a44cae6960b5c3919ef63936d4fab9fbdc55924` |
| `repository-Sources-universe.xz` | `0040f94d11d0039505328a90b2ff48968db873e9e7967307631bf40ef5679275` |
| `repository-Sources.xz` | `714e9f7bf5144fb92260d81d32994020875de57d24b4ef95e253b22283c9eeff` |
| `repository-fixture.json` | `be8115bd9f222f94f35d573c3388bfad552374da37c02017a99190cdac10cddf` |
| `repository-keyring.gpg` | `a278aa1cd5e82155de6e2c801a5026916a4c53e99373fd2ef7fd1f840aca0aa0` |
| `ubuntu-archive-keyring.gpg` | `80a36b0a6de2f69f49d2df75ef473ccde121e9e190b9ea01d20a4f63778d5c31` |
| `ubuntu-noble-updates-InRelease.xz` | `dbe8cc12059f9657a4d3ccbbbba7bfe3de11e1a178b7735000f22535168c1483` |
| `untrusted-InRelease.asc` | `dfacd94434770b59afe063fe0b7cc155bbb3272a79719ee607017f96040c9998` |
| `untrusted-keyring.gpg` | `02e6aadfe2fe956ee4457e5b4de6241d4f9851f37a44d9c6fe14b1480817112e` |
