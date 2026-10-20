# Album-parser browser preview: N03 working branch

This records unmerged work on `codex/refresh-file-identity`. N03 remains in the
intake until a covering PR exists. Final GitHub Codex code review, Linux CI,
merge and original/native/provider acceptance are separate requirements.

## Original contract

Static inspection used the shipped Windows Mp3tag 3.36.1 x64 executable,
SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
It was read and disassembled, never executed. The source audit retained these
addresses in the original executable's address space:

- `[PreviewUrl]` loads through `0x1402751f0` and reaches source-session preview
  storage at `0x1401abeed`–`0x1401abf02`.
- Selected index `preview` loads at `0x1401a924f`–`0x1401a9277`; empty preview
  falls back to the selected URL at `0x1401a927c`–`0x1401a928f`.
- Album execution seeds `CurrentPreview` at `0x1401ad1b1`–`0x1401ad1e9` before
  calling the album parser at `0x1401ad2a1`. Nonempty final output replaces the
  session preview at `0x1401ad2e9`–`0x1401ad2f4`; empty output preserves it.
- Output-to-tag conversion excludes normalized `currentpreview` at
  `0x1401a7ea4`–`0x1401a7eca`. It is not metadata or a track-count input.
- Plain `OutputTo` selects the existing buffer and `Say` appends. `Set` can
  replace or clear it. Native two-argument `OutputTo` is separately tracked
  with the remaining L32 command/arity work.

This establishes the named static contract, not Windows execution parity.

## Working implementation

[The engine](../../src-tauri/src/sources/web/engine.rs) accepts an optional album
preview seed, makes it readable as an output buffer, and charges its bytes to
the existing output limit before execution. Index scripts receive no seed.
[Release conversion and admission](../../src-tauri/src/sources/web/mod.rs)
consume this buffer separately from ordinary fields and track splitting.

The selected/direct preview must have been registered by search and satisfy
the existing HTTP(S), credential and offline policies. The selected parser
URL is the fallback seed. A nonempty accepted final output resolves against
the downloaded page and becomes `preview_url`; empty or unsafe output keeps
the initial preview. The final browser address is session-registered.

[The shared DTO](../../src-tauri/src/sources/model.rs) retains `url` and `id`
and adds optional `preview_url`; other providers omit that property.
[The host](../../src/tag-sources.ts) captures the selected/direct preview in
the release request and uses the returned browser preview in the review.
Rendering does not open a browser or apply tags. C21 full-cover routing stays
separate. The [authoring guide](../user-guide/tag-sources/web-source-reference.md#choosing-the-albums-browser-preview)
explains clearing, appending, replacement and URL fallback.

## Focused evidence

The original domain regression failed because `CurrentPreview` appeared in
track fields. Separate engine regressions reproduced an absent preview seed
and failure to charge an oversized seed. The frontend regression failed
because the chosen preview was absent from the album request.

Focused checks passed: four Rust preview tests (including the 24-case isolated
acquisition matrix), 24 existing source-module tests, four selected frontend
host tests, TypeScript, scoped Clippy, formatting and the strict guide build.
The preview test appears in both Rust selections; these counts overlap.
The local read-only source review reported no findings. This does not replace
the final GitHub review. The isolated acquisition test uses fixed response bodies and
refuses every unexpected request. It exercises direct/index lookup, selected
preview/fallback seeds, append/replacement/empty/unsafe output, registered
browser URLs and unchanged parser identity. It opens no socket or application.
It is opt-in because its transport fixture is process-global:

```sh
cargo test -p opentag --lib album_preview --locked -- --include-ignored --test-threads=1
```

No original application, native UI, live provider, browser or audio writer was
executed for these checks. Their results do not close the separate acceptance
qualifications.
