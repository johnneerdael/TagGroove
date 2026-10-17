# Windows installer identity

`installer.nsi` is based on the template shipped with the locked Tauri CLI
2.12.2, commit `5b23c82ea5ba6b9aec420dd71f60493edc486aa9`. Source paths and
SHA-256 hashes are recorded in `UPSTREAM.json`. The upstream MIT license is
retained here and included in `THIRD_PARTY_NOTICES.md`.

Tauri's default NSIS template derives persistent registry keys from the visible
product and publisher names. Keep those keys at their original OpenTag values
while displaying TagGroove in the installer and Windows application list.
Keep the original MSI upgrade code in `tauri.conf.json` too.

The local changes are limited to:

- Preserve the original uninstall, publisher and install-location registry keys.
- Recognize both current and legacy MSI display names when switching installer formats.
- Check the former `opentag.exe` process before replacing an existing installation.
- Migrate existing desktop links independently of the new-shortcut checkbox. Rename legacy links only after confirming their executable target. Preserve unrelated destinations; when a name is occupied, keep the owned legacy label and update its target instead.
- Remove retained legacy labels on uninstall only when they still target this installation. Preserve links during updates.

The existing Tauri logic removes the former executable. Creation functions
preserve existing unrelated destination links. The original app identifier
continues to select the same data
directories. These source and packaging checks do not establish a live Windows
upgrade result.

When updating `@tauri-apps/cli`, compare the new upstream template with this
copy, retain these identity rules, update `UPSTREAM.json` and the license if
needed, and run `node --test tests/release.test.mjs`. The release test rejects a
CLI/template version mismatch and evaluates the registry-key definitions with
the actual product configuration. Reviewed Windows CI must compile the installer
before merge.

`tests/nsis-shortcuts.test.mjs` compiles the production migration, creation and
cleanup logic in isolated flat/folder fixtures. It uses unchanged upstream COM
shortcut utilities, recorded in `UPSTREAM.json`, rather than a simulated link
ownership model. A local `makensis` installation permits compilation only.
Compile the fixtures with warnings treated as errors, and retain the upstream
`MAX_PATH` and `SLGP_RAWPATH` definitions required by those helpers. Native
execution is restricted to reviewed Windows CI, which downloads the same NSIS
3.11 toolset used by Tauri and verifies its pinned SHA-256. The silent fixture
process creates real shortcuts exclusively under its temporary directory; it
does not install TagGroove or touch the actual desktop or install registry keys.
Cases cover unchecked desktop creation, occupied destinations, unrelated legacy
links, both owned destination forms, cleared collisions, update/uninstall
cleanup, no-shortcut mode and fresh installs. This fixture is narrower than a
live upgrade of the complete released application.
