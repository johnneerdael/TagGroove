# Updates options

Open **File → Options… → Updates** to control checks against OpenTag releases on GitHub. Checking reports available versions; OpenTag does not automatically download or install them.

!!! tip "New to this?"
    Use **Check now…** for a deliberate check. Review the release notes and choose the package for the platform before replacing an installed build.

## Check for updates on startup

Defaults to off. When enabled, OpenTag checks its version on launch. Network failures do not mean that the installed version is current; they mean the check could not establish that fact.

## Also include development builds (beta)

Defaults to off. Enable to include prereleases in the comparison. This changes which releases are reported, not the app's current features and not an installation channel. Disable it to compare against stable releases only. A beta may be published even when there is no newer stable release.

## Display release notes for new versions

Defaults to on. After a previously used profile starts a different app version, OpenTag opens that version's GitHub release page in the default browser. A fresh profile does not have a previous version to compare. The setting is independent of startup update checking.

## Check now

**Check now…** performs a manual version check. It uses the saved settings: changing the beta or network controls in the open Options dialog does not save them. Press **OK**, then reopen the page if the check must use those new values.

Results distinguish a newer available release, an installed version already at the newest applicable release, and no applicable releases. Follow the release link to obtain a build. These are OpenTag releases; Mp3tag's version numbers and update channel do not apply.

## Related

- [Networking](networking.md)
- [General](general.md)
- [Options overview](index.md)

<!-- verified: src/general-options.ts UPDATE_DEFAULTS, updatesOptionsPage, showReleaseNotes, updateMessage; src/main.ts checkForUpdates; src-tauri/src update-check implementation. -->
