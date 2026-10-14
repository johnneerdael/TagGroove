# OpenTag branding integration

The user supplied `OpenTag-Brand-Guide.html` and `OpenTag-Brand-Kit.zip` on 2026-10-14. The original bytes are preserved under `originals/`; the extracted kit, including its README, integration guidance and inventory, remains under `OpenTag-Brand-Kit/`.

`PROVENANCE.json` records the input hashes, 249 verified manifest entries, and byte-for-byte copies into application resources. No artwork was regenerated or edited. The web manifest's deployment paths are the only integration-specific edit to copied kit files.

The interface imports the supplied horizontal light-surface logo, outlined UI glyphs, palette tokens, fallback artwork and About artwork. Orange buttons use the supplied ink for text; small accent text and focus outlines use a darker orange. The fallback image is decorative and is labeled separately from actual embedded artwork.

| Surface | Resource placement |
|---|---|
| Windows | Supplied ICO at `src-tauri/icons/icon.ico`, explicitly listed in Tauri bundle settings |
| macOS | Supplied ICNS at `src-tauri/icons/icon.icns`, explicitly listed in Tauri bundle settings |
| Linux | Supplied PNG/SVG hicolor tree at `src-tauri/icons/linux/hicolor/`, installed via Debian/RPM/AppImage file mappings; desktop executable/icon name is `opentag` |
| Android | Supplied legacy densities and v26/v33 adaptive/themed resources copied into both `src-tauri/icons/android/` and generated `gen/android/app/src/main/res/`; existing application manifest references `@mipmap/ic_launcher` |
| iOS | Supplied opaque 1024px image and `Contents.json` copied into generated `gen/apple/Assets.xcassets/AppIcon.appiconset/`; existing Xcode app-icon setting is `AppIcon` |
| Web | Supplied favicon, Apple touch, pinned-tab and PWA files at `public/brand/`; main document references them; manifest scope/start URL point to the app root |

The generated mobile projects currently contain the provided resources. Reapply the kit's Android `res/` and iOS `AppIcon.appiconset/` after regenerating those projects; do not replace the supplied platform files with newly rasterized desktop artwork.

Linux file mappings follow Tauri's [Debian](https://v2.tauri.app/distribute/debian/#custom-files), [RPM](https://v2.tauri.app/distribute/rpm/), and [AppImage](https://v2.tauri.app/distribute/appimage/#custom-files) configuration. Frozen-source Android aarch64 debug APK and unsigned arm64 iOS simulator app builds passed; both include the provided icon resources. The iOS asset catalog compiles without unassigned-icon warnings after unused template PNGs were moved to a hashed backup. See `../../docs/verification/mobile-final-artifacts.json`. Windows/Linux packaging and actual installation/device icon behavior remain unverified.

`VERIFICATION.json` records input/copy hashes, glyph/XML checks, iOS opacity/dimensions and measured contrast ratios. The branded macOS arm64 debug app built with Rust 1.95.0 and passed strict ad-hoc signature verification. Native CUA screenshots now verify the real header/logo, supplied glyphs, empty state, fallback artwork, About artwork, tagline, version 0.1.0/MIT, and Finder-rendered application icon. Both disposable fixture files loaded and retained their original hashes; no library files were edited. See [the visual proof](../../docs/verification/branding-app.md) and its source/artifact receipt. These captures identify the build-time snapshot, before the later native Windows output-path fix; branding sources remained unchanged. This local app is ad-hoc signed and is not notarized.

`mobile-preview.html` embeds the actual app at 390×800 without sample files. Its production-browser layout was captured, but the current Chrome profile client-blocks/collapses the header logo. Development SVG module requests also report `net::ERR_BLOCKED_BY_CLIENT`. This is a preview-environment limitation; browser extensions/security settings and supplied assets were left unchanged. The native macOS captures are the authoritative rendered in-app branding proof. Native mobile runtime/device icon behavior remains unverified.
