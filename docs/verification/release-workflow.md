# First cross-platform publication run

The branded source and release workflow landed on main through PR #1 at
`b15635360ad0f48d6f1b3ceef376b1aa3eaef07a`. Its tree matches the fully passing
three-desktop CI head `7ec08a27e4bc1b6c8252ed53637b0563e3fefb5e`.

The actual [v0.1.0 publication run](https://github.com/johnneerdael/opentag/actions/runs/38511562000)
passed version/tag/source validation and produced the iOS arm64 simulator preview.
Desktop packaging was still running at this checkpoint. The Android setup job
failed before application compilation with `Failed to find package 'tools'`.
No GitHub release existed when checked after that failure.

`android-actions/setup-android@v3` defaults its package input to the legacy tools
package plus platform-tools. The correction explicitly requests platform-tools;
the action still installs command-line tools and performs its normal setup.
JDK, platform/build-tools 37, NDK, signing, source identity and publication gates
are unchanged. The [v3 action definition](https://raw.githubusercontent.com/android-actions/setup-android/v3/action.yml)
and [current upstream explanation](https://github.com/android-actions/setup-android#the-deprecated-tools-package)
support this bounded correction. Workflow lint passed, and a focused read-only
review found no blocker.

The corrected workflow requires a fresh run on its merged commit. Re-running the
old failed job retains the old workflow. Android compilation, all seven matching
artifact groups, digest checks and public release assets remain unverified until
that new run completes. A successful simulator package is not mobile runtime or
document-editing certification.
