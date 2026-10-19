# Native-derived semantic vectors

These immutable literal predictions were authored before the implementation repair from the pinned Windows Mp3tag 3.36.1 artifact (SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`). They contain semantic inputs/expected outputs, no proprietary instructions, and no original-application runtime claim.

- `filter-quoting.json`: 12 cases, original SHA-256 `d53bea8a3ae4cdcef666f4663b434bfbbd42af30d2f93d7b017756538a92b5ac`.
- `split-field.json`: eight cases, original SHA-256 `fd7bccb91747dc920d29b822c7e0e1ef3b31fd1766f367c97c6227cd47cbb64c`.

Sources and original failing-product receipts: `docs/verification/filter-quoting-precedence-2026-10-19.md` and `docs/verification/split-field-semantics-2026-10-19.md`. Preserve these expected vectors; new evidence requires a distinct fixture rather than changing an oracle to follow the port.
