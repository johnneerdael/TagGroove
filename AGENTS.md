# TagGroove contributor instructions

Use a feature branch and pull request. Follow [the contribution workflow](docs/CONTRIBUTING.md). Owner-authorized workflow changes and repairs to failed main builds may be pushed directly to main. Protect the inspected main head with a lease, preserve unrelated commits, run the relevant local checks and record the failure/validation evidence. Direct-main commits do not manufacture PR review or release proof.

- Require a clean GitHub Codex **code** review on the exact final PR head before merging. Fix findings, resolve threads and request another review after changes. Local review, resolved old findings, security-only review and elapsed time do not replace this gate.
- Include substantive `## Release notes` in every PR body. Describe resulting user behavior, or write `Internal:` with the actual docs/test/CI change. Keep validation results outside release notes and align the prose with the final diff.
- Update affected `docs/user-guide/` pages when behavior changes. Preserve factual platform, signing, save-back and parity limitations. Require the strict guide build where the guide is available.
- Preserve existing application version, artifact, signature and checksum safeguards. Do not bump base versions in routine PRs; follow [RELEASING.md](docs/RELEASING.md) for a planned release-line change.
- Run unit tests, lint, tooling and documentation checks once on Linux before merge. Run application packaging, desktop native UI and Android integration after merge; publication waits for native integration. Do not repeat pre-merge suites in release jobs. Direct-main release repairs require a `CI-Repair-Run: <failed-run-id>` trailer and the evidence described in `docs/CONTRIBUTING.md`.
- For review/release tooling changes, run `python3 -m unittest discover -s .github/scripts/tests -v`, `node --test tests/release.test.mjs`, and actionlint. Keep write-capable review jobs on trusted `main` code with no PR execution or repository secrets.
