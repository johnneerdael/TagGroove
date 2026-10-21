# About TagGroove and this guide

TagGroove is an independently authored, open-source audio metadata editor.
It is being developed for Windows, macOS, Linux, Android and iOS, with different
capabilities and release qualifications on each platform. Its compatibility
and performance goals do not establish complete Mp3tag parity today.

## Identify the application you are using

Choose **Help → About TagGroove** to see the application's version and GPLv3
license identification. Include that version, your operating system and the
operation used when reporting a problem. A similar-looking screenshot or an
old source checkout does not identify the package installed on your computer.

About also lists the built-in fingerprint libraries, Boost.Regex and locale-data
notices, available without a network connection. Expand **Dependency licenses
and notices** for the component/version index and its referenced license text.
**Android/JVM licenses and notices** contains the Android library inventory,
including notices supplied with bundled parser libraries. Both lists are
available offline.
**Native runtime and installer notices** includes additional audio-codec,
Android C++ runtime, Windows installer and Linux AppImage runtime/launcher
notices, including their retained supplier-library notices. These lists preserve the
supplier's copyright and license wording.
Linux AppImages also carry distribution copyright files under
`usr/share/doc/` and the standard texts they reference under
`usr/share/common-licenses/` inside the image. These package documents supplement
the notices shown in About. They can be read offline from an extracted image.
Copyright and license text keeps its original language when the dialog's
controls and headings are translated. **Sources and releases** leads to the release downloads and the
corresponding-source archive with rebuild and relink instructions. On Android
and iOS, the notice displays the release address to open in your browser.

Use the project's [GitHub releases](https://github.com/johnneerdael/TagGroove/releases)
for packages, source revisions, checksums and release notes. The
[installation chapter](../getting-started/install.md) explains the currently
inspected package set and its signing/preview limits. A successful automated
build is separate from proof that every OS/device installs and behaves the
same way.

## Relationship to Mp3tag

TagGroove is an independent project, not an official Mp3tag edition, update,
support channel or endorsement. The original application remains its author's
property. TagGroove's own source, examples, artwork and guide are separate work.
Do not assume that a Windows-only menu, original configuration archive,
third-party template or parser script transfers unchanged.

Compatibility must be assessed for the particular workflow: tag format,
field mapping, expression context, replacement syntax, action order and output
side effects can matter. For example, Find and format strings/actions share a Boost-compatible regex
engine, but their quoting, case controls and replacement processing differ.
Use the [regex reference](../reference/regular-expressions.md) and the relevant
command's worked cases instead of treating a shared function name as proof
of identical behavior.

Report TagGroove problems on the
[TagGroove issue tracker](https://github.com/johnneerdael/TagGroove/issues).
The [Help menu](../reference/menus/help.md) identifies its actual destinations;
there is no separate donation or original-editor support command in that menu.
An issue report shares reproducible information. It does not promise a
response time or recovery of personal files.

## License and credits

The combined TagGroove application carries the project's
[GPL-3.0-only license](https://github.com/johnneerdael/TagGroove/blob/main/COPYING).
Its native Beatport and Traxsource adapters adapt OneTagger-derived CueRithm
code; their complete license and provenance are available in About.
Existing independently authored components retain their
[MIT notices](https://github.com/johnneerdael/TagGroove/blob/main/LICENSE).
Third-party components retain their own licenses. These notices do not license
the original Mp3tag software.

The application uses Tauri for its native/web interface and Rust for the
metadata engine. The regex bridge includes Boost.Regex under its accompanying
Boost Software License. Source dependencies and bundled component notices are
the authoritative place to inspect third-party terms. This paragraph is an
orientation and does not audit every dependency license.

## How to use this guide

Choose a section for the kind of help needed:

| Need | Section |
| --- | --- |
| A first successful practice session | [Get started](../getting-started/index.md) and [Your first tagging session](../learn/first-tagging-session.md) |
| A worked lesson that explains why each step matters | Tutorials under **Learn** |
| A procedure for an editing task | Guides and the converter/action pages under **Automate** |
| A specific repeated cleanup or organization problem | **Cookbook** recipes |
| Exact syntax, field mappings, menus or settings | **Reference** |
| Diagnosis and a useful issue report | [Troubleshooting](troubleshooting.md) and [FAQ](faq.md) |

The guide describes implemented TagGroove behavior using independently written
explanations and invented practice data. Canonical Mp3tag topics were a
research checklist and do not guarantee runtime equivalence. Source reads,
core/model tests, native UI tests, package checks and actual live publication
are different kinds of evidence; the guide keeps their limits distinct.

Only completed chapters are published and searched. More topics remain in
the writing plan. A large navigation or passing example suite does not prove
that every canonical detail has been covered. Start on copies and confirm a
saved result in the tools used for your own library.

## Improve a page

For a correction, identify the page/section, your TagGroove version, the steps
tried and the actual result. A copied expression and input/output pair help
more than “the script does not work.” If the issue is a missing explanation,
describe the task and the point where the instructions became unclear.

The source guide lives under `docs/user-guide/` in the
[TagGroove repository](https://github.com/johnneerdael/TagGroove). Contributors
follow its contribution and documentation-publication instructions: finish a
page, validate links/examples, obtain the required current-head review and
checks, then verify the published result after merge. Unfinished chapters
remain drafts. Do not copy the original editor's prose, screenshots or real
music metadata into a contribution; use independent explanations and suitable
practice files.

## Related

- [Start using TagGroove](../getting-started/index.md)
- [Frequently asked questions](faq.md)
- [Troubleshooting](troubleshooting.md)
- [Help menu](../reference/menus/help.md)
