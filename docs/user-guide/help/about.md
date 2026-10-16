# About OpenTag and this guide

OpenTag is an independently authored, open-source audio metadata editor.
It is being developed for Windows, macOS, Linux, Android and iOS, with different
capabilities and release qualifications on each platform. Its compatibility
and performance goals do not establish complete Mp3tag parity today.

## Identify the application you are using

Choose **Help → About OpenTag** to see the application's version and MIT
license identification. Include that version, your operating system and the
operation used when reporting a problem. A similar-looking screenshot or an
old source checkout does not identify the package installed on your computer.

Use the project's [GitHub releases](https://github.com/johnneerdael/opentag/releases)
for packages, source revisions, checksums and release notes. The
[installation chapter](../getting-started/install.md) explains the currently
inspected package set and its signing/preview limits. A successful automated
build is separate from proof that every OS/device installs and behaves the
same way.

## Relationship to Mp3tag

OpenTag is an independent project, not an official Mp3tag edition, update,
support channel or endorsement. The original application remains its author's
property. OpenTag's own source, examples, artwork and guide are separate work.
Do not assume that a Windows-only menu, original configuration archive,
third-party template or parser script transfers unchanged.

Compatibility must be assessed for the particular workflow: tag format,
field mapping, expression context, replacement syntax, action order and output
side effects can matter. For example, this guide distinguishes JavaScript
regex in Find from the Boost-compatible core used by format strings/actions.
Use the [regex reference](../reference/regular-expressions.md) and the relevant
command's worked cases instead of treating a shared function name as proof
of identical behavior.

Report OpenTag problems on the
[OpenTag issue tracker](https://github.com/johnneerdael/opentag/issues).
The [Help menu](../reference/menus/help.md) identifies its actual destinations;
there is no separate donation or original-editor support command in that menu.
An issue report is a way to share reproducible information, not a promise of
response time or recovery of personal files.

## License and credits

The independently authored OpenTag application and associated repository
documentation carry the project's
[MIT license](https://github.com/johnneerdael/opentag/blob/main/LICENSE).
Read that file for its permission, notice and warranty terms. Third-party
components retain their own licenses; the project license does not replace
their notices or license the original Mp3tag software.

The application uses Tauri for its native/web interface and Rust for the
metadata engine. The regex bridge includes Boost.Regex under its accompanying
Boost Software License. Source dependencies and bundled component notices are
the authoritative place to inspect third-party terms. This paragraph is an
orientation, not a complete dependency-license audit.

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

The guide describes implemented OpenTag behavior using independently written
explanations and invented practice data. Canonical Mp3tag topics serve as a
research checklist, not a guarantee of runtime equivalence. Source reads,
core/model tests, native UI tests, package checks and actual live publication
are different kinds of evidence; the guide keeps their limits distinct.

Only completed chapters are published and searched. More topics remain in
the writing plan. A large navigation or passing example suite does not prove
that every canonical detail has been covered. Start on copies and confirm a
saved result in the tools used for your own library.

## Improve a page

For a correction, identify the page/section, your OpenTag version, the steps
tried and the actual result. A copied expression and input/output pair help
more than “the script does not work.” If the issue is a missing explanation,
describe the task and the point where the instructions became unclear.

The source guide lives under `docs/user-guide/` in the
[OpenTag repository](https://github.com/johnneerdael/opentag). Contributors
follow its contribution and documentation-publication instructions: finish a
page, validate links/examples, obtain the required current-head review and
checks, then verify the published result after merge. Unfinished chapters
remain drafts. Do not copy the original editor's prose, screenshots or real
music metadata into a contribution; use independent explanations and suitable
practice files.

## Related

- [Start using OpenTag](../getting-started/index.md)
- [Frequently asked questions](faq.md)
- [Troubleshooting](troubleshooting.md)
- [Help menu](../reference/menus/help.md)
