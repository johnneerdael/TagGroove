# Help menu

Use **Help** to find documentation, inspect keyboard shortcuts, report a
problem, check released versions or identify the installed TagGroove version.
These commands do not require selected audio files.

!!! tip "New to this?"
    Start with [your first tagging session](../../learn/first-tagging-session.md).
    Keep the app version from **About TagGroove** with any troubleshooting notes.

## Every current entry

| Entry | Result |
| --- | --- |
| **Documentation** | On desktop, open the [published TagGroove user guide](https://taggroove.neerdael.nl/) in the default browser. Its source remains in the TagGroove repository. |
| **Keyboard shortcuts** | Show the app's shortcut overview, assembled from current menu commands plus additional editing/navigation keys. |
| **Report an issue** | On desktop, open TagGroove's GitHub issue page in the default browser. |
| **Check for updates…** | Read the release list and show whether a newer applicable version is available. It does not download or install an update. |
| **Releases** | On desktop, open TagGroove's GitHub releases page for packages and release notes. |
| **About TagGroove** | Show TagGroove's version and MIT license identification. Close the dialog to return to the workspace. |

On the inspected mobile path, external Help destinations report the URL to
open in a browser instead of launching the desktop browser opener. Follow
the message on that build. This is separate from whether the in-app shortcut
or About dialog can be shown.

## Read the shortcut overview

Shortcuts use Command on macOS where the command is a primary-modifier
shortcut, and Control on Windows/Linux. Some controls deliberately retain
physical Control on every platform. The overview also contains commands
that do not have a top-menu entry, such as moving within an action list or
opening a format-string helper.

Find the command by task, then check its focus context. Copy while editing
text copies text; **Copy tags** outside a text editor has a different purpose.
The [keyboard reference](../keyboard-shortcuts.md) explains the platform and
focus distinctions with examples.

## Check and obtain an update

1. Choose **Help → Check for updates…**.
2. Read the reported current/latest versions and whether the offered release
   is stable or a prerelease.
3. Choose **Open release page** for an offered update, or **View release notes**
   when inspecting the latest known release.
4. On that page, choose the package appropriate to the actual platform and
   review its installation/signing limitations before installing it yourself.

[Options → Updates](../options/updates.md) controls prerelease inclusion and
automatic startup checks. A manual check still uses the prerelease preference.
A failed check reports that the release list could not be read; it is not
evidence that the installed version is current. Conversely, a successful
release-list request does not test every tag source or network service.

There is no separate **First Steps**, donation, Mp3tag website or Mp3tag
community command in this TagGroove menu. Use this guide's first-session link
for the introductory task and TagGroove's own issue/release pages for support.
A similarly named item in another application is not a TagGroove support route.

## Report a useful problem

Record the TagGroove version from **About TagGroove**, the operating system,
the command used and a short sequence that reproduces the result. Include
the actual displayed error and the file/container type when relevant.
Use a copied or minimal example where possible. A screenshot can explain a
layout issue, while a copyable format string is more useful for a formula
problem. Do not treat the issue form as a promise of an immediate response.

## Related

- [Your first tagging session](../../learn/first-tagging-session.md)
- [Keyboard shortcuts](../keyboard-shortcuts.md)
- [Updates options](../options/updates.md)
- [Menus overview](index.md)

<!-- Source baseline main4e4d5fc: src/main.ts Help menu, operateImpl and checkForUpdates; src/utils-ui.ts menuShortcuts/EXTRA_SHORTCUTS/shortcutsDialog; src-tauri/src/utilities.rs HELP_PAGES/open_help_page; updates.rs. Canonical Help tasks mapped to actual TagGroove entries or explicit differences. No new browser/network/native update check. -->
