# TagGroove branding integration

The supplied `TagGroove-Brand-Guide.html` and `TagGroove-Brand-Kit.zip` are preserved byte-for-byte under `originals/`. The extracted kit remains under `TagGroove-Brand-Kit/`, including its own guidance, tokens and 249-file manifest. The vector system is the canonical artwork source; lettering is outlined.

`PROVENANCE.json` records the supplied input hashes and every application/guide resource copy. Run `python3 scripts/verify-brand.py` to check the kit, integrated hashes, Android resource references, stable application identity and registered gnudb client name. Android resource identifiers keep their existing `opentag_` spelling; the web manifest's scope/start URL are adapted to the app root. These transformations are recorded explicitly.

| Surface | Resource placement |
|---|---|
| Application header, About, empty artwork and glyphs | Direct imports from the supplied kit in `src/main.ts` |
| Windows | Supplied ICO at `src-tauri/icons/icon.ico` |
| macOS | Supplied ICNS at `src-tauri/icons/icon.icns` |
| Linux | Supplied PNG/SVG hicolor tree under `src-tauri/icons/linux/hicolor/`, named for the `taggroove` executable |
| Android | Supplied legacy densities and v26/v33 adaptive/themed resources under both `src-tauri/icons/android/` and generated `gen/android/app/src/main/res/` |
| iOS | Supplied opaque 1024px image and catalog under generated `gen/apple/Assets.xcassets/AppIcon.appiconset/` |
| Web | Supplied favicon, Apple touch, pinned-tab and PWA files at `public/brand/` |
| Guide | Supplied light/dark logos, mark and favicons at `docs/user-guide/assets/brand/` |

All 412 images present before the rename were visually inspected. Wordmarks, banners and splash artwork were replaced with supplied TagGroove files. Fifty-six unused icons from previous generated branding were removed. Historical verification screenshots remain unchanged as evidence of the builds they describe. Public guide captures are refreshed from the reviewed Linux application. See `docs/verification/taggroove-image-audit.json` and `docs/verification/taggroove-rename.md`.

The rename changes visible branding, executable and release asset names. It preserves `org.opentag.editor`, existing preference keys, configuration/tag-panel format identifiers, credential and signing settings, internal crates/ABI, and the exact gnudb client name `OpenTagAudioTagEditor`. Existing backup formats and application data locations are preserved. Product-derived installer/package names change; cross-platform install/upgrade behavior is not yet device-tested. No base-version bump is included.
