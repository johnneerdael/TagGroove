# What's new

Read the notes for the TagGroove version you are installing on the project's
[GitHub releases page](https://github.com/johnneerdael/TagGroove/releases).
Each release has its own notes and downloadable packages. The same page is
the archive for older releases, including packages published under the former
OpenTag name.

This guide explains how to use features. Release notes explain what changed
between particular builds. Match the version in **Help → About TagGroove** to
the release's version before following instructions for a newly added feature.

## Find the changes that affect your library

1. Choose **Help → About TagGroove** and note the installed version.
2. Choose **Help → Releases** on desktop, or open the releases link above.
3. Open the proposed release and read its notes. Pay particular attention to
   changes to the formats you edit, saving, file operations, action groups,
   configuration migration and platform limitations.
4. Open **Assets** only after checking which package matches your operating
   system and processor. Follow [Download and install](install.md) to verify
   the package and understand its signing status.

For example, if a release changes how an action handles embedded covers,
read its notes and the [cover-action instructions](../actions/index.md)
before running an existing group on a large selection. A familiar group name
does not tell you which settings the saved group contains.

The release archive belongs to TagGroove. Mp3tag has its own release history;
matching a feature name or version number across the two applications does
not establish identical behavior.

## Check whether an update is available

Choose **Help → Check for updates…**. When an update is offered, **Open
release page** takes you to its notes and packages. **View release notes**
can open the latest known release when no newer applicable version is found.
Checking does not download or install the application for you.

Under **File → Options… → Updates**, choose whether to check on startup and
whether to include development builds. Choose **OK** to save changes before
using a check that needs those new settings.

| Setting | What it changes |
| --- | --- |
| **Check for updates on startup** | Run a version check when TagGroove starts; off by default. |
| **Also include development builds (beta)** | Include prereleases in the comparison; off by default. |
| **Display release notes for new versions** | Open the version's release page when an existing profile starts a different app version; on by default. |

A fresh profile has no previously used version to compare for the last
setting. It is independent of the startup update-check switch.

## Decide whether to try a beta

A prerelease is a build offered for testing before a full release. Enabling
the beta option changes the versions reported by the checker; it does not
switch or install the running application. Read the beta's own notes and
use copied files when trying changed workflows.

Before replacing a build, finish pending edits and save a
[configuration backup](../customize/configuration.md). Keep music backups
separately: a saved configuration does not contain your audio or pending tag
edits. Keep physical export templates and other external resources too when
your workflow depends on them.

## If the check fails or the notes seem unfamiliar

| Situation | Next step |
| --- | --- |
| The check reports a network error | Try later or open the releases page yourself. A failed check does not establish that your version is current. |
| The checker mentions a beta but offers no newer stable release | Decide whether to enable development builds; there may be no newer full release. |
| An old download says OpenTag | Check its exact release, filename and checksum. The project was renamed; old assets retain their published names. |
| A feature appears in the guide but not in your build | Check the installed version and the relevant release notes, then update deliberately if appropriate. |
| A mobile package exists | Read its platform notes. An iOS simulator ZIP is not an iPhone/iPad installer, and a signed Android preview does not establish device or provider qualification. |

## Related

- [Download and install](install.md)
- [Updates options](../reference/options/updates.md)
- [Save and restore configuration](../customize/configuration.md)
- [Troubleshooting](../help/troubleshooting.md)

<!-- Candidate evidence: src/general-options.ts UPDATE_DEFAULTS, showReleaseNotes, updateMessage; src-tauri/src/updates.rs newest, newest_release_respects_the_beta_channel; src/main.ts checkForUpdates; docs/RELEASING.md. No live release-list or update request was made for this draft. -->
