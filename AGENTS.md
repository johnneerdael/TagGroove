# TagGroove contributor instructions

Use a feature branch and pull request. Follow [the contribution workflow](docs/CONTRIBUTING.md).

- Require a clean GitHub Codex **code** review on the exact final PR head before merging. Fix findings, resolve threads and request another review after changes. Local review, resolved old findings, security-only review and elapsed time do not replace this gate.
- Include substantive `## Release notes` in every PR body. Describe resulting user behavior, or write `Internal:` with the actual docs/test/CI change. Keep validation results outside release notes and align the prose with the final diff.
- Update affected `docs/user-guide/` pages when behavior changes. Preserve factual platform, signing, save-back and parity limitations. Require the strict guide build where the guide is available.
- Preserve existing application version, artifact, signature and checksum safeguards. Do not bump base versions in routine PRs; follow [RELEASING.md](docs/RELEASING.md) for a planned release-line change.
- For review/release tooling changes, run `python3 -m unittest discover -s .github/scripts/tests -v`, `node --test tests/release.test.mjs`, and actionlint. Keep write-capable review jobs on trusted `main` code with no PR execution or repository secrets.
