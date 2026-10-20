"""Execution-contract checks for the reviewed validation/release split."""
import json
import re
import unittest
from pathlib import Path

WORKFLOWS = Path(__file__).resolve().parents[2] / "workflows"


def job(source, name):
    match = re.search(rf"^  {name}:\n(.*?)(?=^  [a-z0-9_-]+:|\Z)", source, re.M | re.S)
    if not match:
        raise AssertionError(f"Missing job {name}")
    return match[1]


class LeanWorkflows(unittest.TestCase):
    def test_source_recovery_reuses_original_packages_without_native_builds(self):
        source = (WORKFLOWS / 'source-recovery.yml').read_text()
        self.assertEqual(set(re.findall(r'^  ([a-z0-9_-]+):$', source.split('jobs:', 1)[1], re.M)), {'source', 'publish'})
        for forbidden in ['tauri --', 'cargo build', 'cargo test', 'npm test', 'native-validation', 'secrets.']:
            self.assertNotIn(forbidden, source)
        self.assertIn('group: opentag-release', source)
        recovery, publish = job(source, 'source'), job(source, 'publish')
        self.assertIn('--root "$GITHUB_WORKSPACE/release-source"', recovery)
        self.assertIn('ref: ${{ steps.gate.outputs.source_sha }}', recovery)
        self.assertIn('RELEASE_SOURCE_SHA: ${{ steps.gate.outputs.source_sha }}', recovery)
        for kind in ['native', 'linux', 'macos']:
            step = recovery.split('artifact-ids: ${{ steps.gate.outputs.'+kind+'_ids }}', 1)[0].rsplit('      - uses: actions/download-artifact@v4', 1)[1]
            self.assertIn('run-id: ${{ steps.gate.outputs.run_id }}', step)
        self.assertIn('run-id: ${{ needs.source.outputs.run_id }}', publish)
        self.assertIn('artifact-ids: ${{ steps.gate.outputs.release_ids }}', publish)
        self.assertIn('name: release-corresponding-source', publish)
        self.assertEqual(publish.count('source_recovery.py --run-id'), 2)
        self.assertIn('working-directory: release-source', publish)
        self.assertIn('node ../.release-tools/scripts/release.mjs publish', publish)
        self.assertIn('RELEASE_VERIFIER_REF: ${{ steps.final-gate.outputs.tool_sha }}', publish)

    def test_failed_source_and_publication_retries_use_trusted_main_tools(self):
        source = (WORKFLOWS / 'reviewed-release.yml').read_text()
        recovery, publish = job(source, 'corresponding-source'), job(source, 'publish')
        self.assertIn('ref: main\n          path: .release-tools', recovery)
        self.assertEqual(recovery.count('python .release-tools/scripts/audio-source-bundle.py --root "$GITHUB_WORKSPACE"'), 2)
        self.assertIn('ref: main\n          path: .trusted-gate', publish)
        self.assertIn('node .trusted-gate/scripts/release.mjs publish', publish)
        self.assertIn('RELEASE_VERIFIER_REF: ${{ steps.trusted.outputs.tool_sha }}', publish)

    def test_notary_authentication_precedes_all_platform_builds(self):
        source = (WORKFLOWS / 'reviewed-release.yml').read_text()
        prepare = job(source, 'prepare')
        title = 'Validate Apple notarization credentials before platform builds'
        self.assertIn('needs: authorize', prepare)
        self.assertIn('      - name: '+title+'\n', prepare)
        step = prepare.split('      - name: '+title+'\n', 1)[1].split('      - ', 1)[0]
        self.assertIn("if: steps.plan.outputs.skip != 'true'", step)
        self.assertIn('run: node scripts/notary-preflight.mjs', step)
        for name in ['APPLE_NOTARY_ISSUER_ID', 'APPLE_NOTARY_KEY_ID', 'APPLE_NOTARY_PRIVATE_KEY']:
            self.assertIn(name+': ${{ secrets.'+name+' }}', step)
        self.assertNotIn('APPLE_CERTIFICATE', step)
        self.assertLess(prepare.index('name: Resolve immutable release plan'), prepare.index('name: '+title))
        for name in ['desktop', 'android', 'ios']:
            self.assertIn('needs: prepare', job(source, name))
        self.assertIn('needs: [authorize, prepare]', job(source, 'native-validation'))

    def test_macos_signing_is_scoped_and_verified_before_collection(self):
        source = (WORKFLOWS / 'reviewed-release.yml').read_text()
        desktop = job(source, 'desktop')
        def step(title):
            self.assertIn('      - name: '+title+'\n', desktop)
            return desktop.split('      - name: '+title+'\n', 1)[1].split('      - ', 1)[0]
        prepare = step('Prepare private macOS signing inputs')
        build = step('Build signed and notarized macOS packages')
        verify = step('Verify notarized macOS packages and bind their bytes')
        cleanup = step('Remove private macOS signing inputs')
        for item in [prepare, build, verify]:
            self.assertIn("if: runner.os == 'macOS'", item)
        self.assertIn("if: always() && runner.os == 'macOS'", cleanup)
        self.assertIn('macos-release.py cleanup --directory "$RUNNER_TEMP/taggroove-macos-notary"', cleanup)
        self.assertNotIn('secrets.', cleanup)
        for name in ['APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_NOTARY_ISSUER_ID',
                     'APPLE_NOTARY_KEY_ID', 'APPLE_NOTARY_PRIVATE_KEY']:
            self.assertIn('secrets.'+name, prepare)
        self.assertNotIn('PRIVATE_KEY', build)
        self.assertNotIn('APPLE_CERTIFICATE', verify)
        self.assertNotIn('PRIVATE_KEY', verify)
        self.assertIn('echo "$RUNNER_TEMP/taggroove-macos-notary" >> "$GITHUB_PATH"', prepare)
        self.assertIn('APPLE_API_KEY_PATH: ${{ runner.temp }}/taggroove-macos-notary/AuthKey.p8', build)
        self.assertIn('npm run tauri -- build --target ${{ matrix.target }} --bundles app,dmg --config "$RUNNER_TEMP/taggroove-macos-notary/tauri-release.json" --ci', build)
        self.assertIn('--github-output "$GITHUB_OUTPUT"', verify)
        self.assertIn('--source-commit "$RELEASE_COMMIT"', verify)
        self.assertIn("if: runner.os != 'macOS'", step('Build desktop packages'))
        self.assertEqual(desktop.count('npm run tauri -- build '), 1)
        self.assertLess(desktop.index('name: Prepare private macOS signing inputs'), desktop.index('name: Build signed and notarized macOS packages'))
        self.assertLess(desktop.index('name: Build signed and notarized macOS packages'), desktop.index('name: Verify notarized macOS packages and bind their bytes'))
        self.assertLess(desktop.index('name: Verify notarized macOS packages and bind their bytes'), desktop.index('node scripts/release.mjs collect-desktop'))
        self.assertIn('steps.macos.outputs.artifact_paths', step('Name and hash desktop artifacts'))
        self.assertIn('name: macos-signing-${{ matrix.platform }}', desktop)
        self.assertIn('path: release-assets/macos-signing-${{ matrix.platform }}.json', desktop)
        sources = job(source, 'corresponding-source')
        self.assertIn('pattern: macos-signing-*', sources)
        self.assertIn('path: macos-evidence', sources)
        self.assertEqual(sources.count('--macos-evidence macos-evidence'), 2)
        for name in ['android', 'ios', 'corresponding-source', 'publish']:
            self.assertNotIn('secrets.APPLE_', job(source, name))

    def test_linux_packager_inputs_are_pinned_before_the_single_application_build(self):
        block = job((WORKFLOWS / 'reviewed-release.yml').read_text(), 'desktop')
        title = 'Prepare pinned Linux packaging inputs'
        self.assertIn('name: '+title, block)
        step = block.split('      - name: '+title+'\n', 1)[1].split('      - ', 1)[0]
        self.assertIn("if: runner.os == 'Linux'", step)
        self.assertIn('python scripts/linux-build-tools.py', step)
        self.assertIn('XDG_CACHE_HOME=', step)
        self.assertIn('LDAI_RUNTIME_FILE=', step)
        self.assertLess(block.index('name: Prepare AppImage shared license material and inspector'), block.index('name: '+title))
        self.assertLess(block.index('name: '+title), block.index('name: Build desktop packages'))
        self.assertEqual(block.count('uses: tauri-apps/tauri-action@'), 1)

    def test_linux_capture_follows_build_and_sources_receive_both_builder_artifacts(self):
        source = (WORKFLOWS / 'reviewed-release.yml').read_text()
        desktop = job(source, 'desktop')
        title = 'Capture authenticated Linux package and source bindings'
        self.assertIn('name: '+title, desktop)
        step = desktop.split('      - name: '+title+'\n', 1)[1].split('      - ', 1)[0]
        self.assertIn("if: runner.os == 'Linux'", step)
        self.assertIn('python scripts/linux-distribution-capture.py', step)
        self.assertIn('--source-commit "$RELEASE_COMMIT"', step)
        self.assertIn('--output linux-source-evidence', step)
        self.assertLess(desktop.index('name: Build desktop packages'), desktop.index('name: '+title))
        self.assertLess(desktop.index('name: '+title), desktop.index('node scripts/release.mjs collect-desktop'))
        self.assertIn('name: linux-source-${{ matrix.platform }}', desktop)
        sources = job(source, 'corresponding-source')
        self.assertIn('pattern: linux-source-*', sources)
        self.assertIn('path: linux-evidence', sources)
        self.assertEqual(sources.count('--linux-evidence linux-evidence'), 2)

    def test_appimage_license_material_is_staged_before_build_and_verified_before_collection(self):
        block = job((WORKFLOWS / "reviewed-release.yml").read_text(), "desktop")
        for title, command in [
            ('Prepare AppImage shared license material and inspector', 'python scripts/appimage-licenses.py --stage --prepare-inspector'),
            ('Verify AppImage copyright license references', "python scripts/appimage-licenses.py --verify-built '${{ matrix.target }}'"),
        ]:
            step = block.split('      - name: '+title+'\n', 1)[1].split('      - ', 1)[0]
            self.assertIn("if: runner.os == 'Linux'", step)
            self.assertIn(command, step)
        stage = block.index('name: Prepare AppImage shared license material and inspector')
        build = block.index('name: Build desktop packages')
        verify = block.index('name: Verify AppImage copyright license references')
        collect = block.index('node scripts/release.mjs collect-desktop')
        self.assertLess(stage, build)
        self.assertLess(build, verify)
        self.assertLess(verify, collect)
        config = json.loads((WORKFLOWS.parents[1] / 'src-tauri/tauri.conf.json').read_text())
        self.assertEqual(config['bundle']['linux']['appimage']['files']['/usr/share/common-licenses'],
                         '../target/appimage-common-licenses/')
        tooling = job((WORKFLOWS / '_reviewed-build.yml').read_text(), 'tooling')
        self.assertIn("python3 -m unittest discover -s scripts/tests -p 'test_*notices.py' -v", tooling)
        self.assertIn("python3 -m unittest discover -s scripts/tests -p 'test_appimage_licenses.py' -v", tooling)

    def test_android_notices_match_actual_runtime_archives_before_package_collection(self):
        block = job((WORKFLOWS / "reviewed-release.yml").read_text(), "android")
        build = block.index("npm run tauri -- android build --target aarch64 --apk --ci")
        capture = block.index('python scripts/android-notices.py --capture "$RUNNER_TEMP/taggroove-android-runtime.json"')
        verify = block.index('python scripts/android-notices.py --check --metadata "$RUNNER_TEMP/taggroove-android-runtime.json" --verify-material')
        collect = block.index("node scripts/release.mjs collect-android")
        self.assertLess(build, capture)
        self.assertLess(capture, verify)
        self.assertLess(verify, collect)
        self.assertNotIn('release-assets/android-runtime', block)
        self.assertIn('name: android-runtime-notice-evidence', block)
        runtime = block.index('python scripts/component-notices.py --check --built-platform android --ndk "$NDK_HOME"')
        self.assertLess(verify, runtime)
        self.assertLess(runtime, collect)

    def test_windows_helper_notices_are_checked_after_build_before_collection(self):
        block = job((WORKFLOWS / "reviewed-release.yml").read_text(), "desktop")
        step = block.split('      - name: Verify Windows installer helper notice bindings\n', 1)[1].split('      - ', 1)[0]
        self.assertIn("if: runner.os == 'Windows'", step)
        self.assertIn('python scripts/component-notices.py --check --built-platform windows', step)
        self.assertLess(block.index('name: Build desktop packages'), block.index('name: Verify Windows installer helper notice bindings'))
        self.assertLess(block.index('name: Verify Windows installer helper notice bindings'), block.index('node scripts/release.mjs collect-desktop'))

    def test_premerge_runs_checks_on_one_linux_worker_without_app_builds(self):
        source = (WORKFLOWS / "_reviewed-build.yml").read_text()
        self.assertEqual(set(re.findall(r"^  ([a-z0-9_-]+):$", source, re.M)), {"workflow_call", "authorize", "tooling"})
        tooling = job(source, "tooling")
        self.assertIn("runs-on: ubuntu-24.04", tooling)
        for check in ("npm run test:ui", "cargo test --workspace --locked",
                      "cargo fmt --all -- --check", "cargo clippy --workspace --all-targets --locked -- -D warnings",
                      "npx tsc --noEmit", "python -m mkdocs build --strict", "-p 'test_macos_*.py'"):
            self.assertIn(check, tooling)
        self.assertNotIn("node --test tests/release.test.mjs", source)
        for build in ("tauri build", "android build", "e2e-native.sh", "docs-screenshots.sh", "matrix:", "secrets.", "cache:"):
            self.assertNotIn(build, source)

    def test_native_suites_run_only_in_authorized_postmerge_release(self):
        source = (WORKFLOWS / "_reviewed-native.yml").read_text()
        authorize = job(source, "authorize")
        self.assertIn("reviewed-release.yml@refs/heads/main", authorize)
        self.assertIn("review_gate.py release-check", authorize)
        self.assertNotIn("secrets.", source)
        self.assertNotIn("cache:", source)
        for name in ("desktop", "android"):
            block = job(source, name)
            self.assertIn("needs: authorize", block)
            self.assertIn("if: needs.authorize.outputs.ready == 'true'", block)
            self.assertIn("ref: ${{ inputs.source_sha }}", block)
            for check in ("cargo test", "cargo clippy", "cargo fmt", "npm run test:ui"):
                self.assertNotIn(check, block)
        desktop = job(source, "desktop")
        self.assertIn("fail-fast: true", desktop)
        self.assertNotIn("status=1", desktop)
        for check in ("Build the debug app once", "Run native suites", "--no-build", "OPENTAG_E2E_OFFLINE=1"):
            self.assertIn(check, desktop)
        release = (WORKFLOWS / "reviewed-release.yml").read_text()
        native = job(release, "native-validation")
        self.assertIn("needs: [authorize, prepare]", native)
        self.assertIn("if: needs.prepare.outputs.skip != 'true'", native)
        self.assertIn("uses: ./.github/workflows/_reviewed-native.yml", native)
        self.assertIn("native-validation", job(release, "publish"))
        self.assertNotIn("node --test tests/release.test.mjs", release)

    def test_windows_installer_fixture_still_executes_on_windows(self):
        source = (WORKFLOWS / "_reviewed-native.yml").read_text()
        desktop = job(source, "desktop")
        step = desktop.split("      - name: Check native Windows installer shortcuts\n", 1)[1].split("      - ", 1)[0]
        self.assertIn("if: runner.os == 'Windows'", step)
        self.assertIn("node --test tests/nsis-shortcuts.test.mjs", step)
        self.assertLess(desktop.index("run: npm ci"), desktop.index("node --test tests/nsis-shortcuts.test.mjs"))

    def test_release_packages_validated_source_without_repeating_unit_checks(self):
        source = (WORKFLOWS / "reviewed-release.yml").read_text()
        for name in ("desktop", "android", "ios"):
            block = job(source, name)
            for command in ("npm run test:ui", "cargo test", "cargo clippy", "cargo fmt"):
                self.assertNotIn(command, block, f"{name}: duplicate {command}")
        desktop = job(source, "desktop")
        for required in ("Build desktop packages", "Check macOS document types",
                         "collect-desktop", "--collect-native", "OPENTAG_AUDIO_BUILD_ROOT"):
            self.assertIn(required, desktop)
        self.assertNotIn("choco install ffmpeg", desktop)
        self.assertNotIn("brew install ffmpeg", desktop)
        self.assertEqual(source.count("review_gate.py release-check"), 2)
        self.assertIn("Package and independently verify complete source", source)
        self.assertIn("Verify all platforms, calculate SHA256SUMS", source)


if __name__ == "__main__":
    unittest.main()
