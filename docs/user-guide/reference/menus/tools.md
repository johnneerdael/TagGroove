# Tools menu

The Tools menu contains built-in metadata/container utilities, the tool
configuration command and any external programs you have defined. A built-in
utility writes through TagGroove's supported file path; a named external tool
starts another program with its own behavior and completion rules.

<!-- figure:menu-tools -->
<figure markdown>
![The Tools menu open in TagGroove, listing its commands](../../assets/screens/menu-tools-light.png#only-light)
![The Tools menu open in TagGroove, listing its commands](../../assets/screens/menu-tools-dark.png#only-dark)
<figcaption>The Tools menu: tag-type removal, MP4 chapters, unsupported fields, Optimize and tool settings.</figcaption>
</figure>
<!-- /figure -->

!!! tip "Choose a workflow first"

    Use [built-in utilities](../../tagging/utilities.md) for stored tag/layout
    cleanup, or [external tools](../../customize/tools.md) for executable paths,
    parameters and selection scope. Neither route is an audio repair guarantee.

## Fixed menu items

The current built-in items appear in this order:

| Item | Result | Scope and timing |
| --- | --- | --- |
| **Remove tag types…** | Choose ID3v1, ID3v2 and/or APEv2 blocks to remove | Writes selected supported local files when **Remove** is chosen; explicit types are independent of Options' Remove switches |
| **Strip chapters MP4…** | Remove the applicable MP4 chapter structure | Whole-file immediate operation; normally confirms file/chapter count and leaves freed boxes |
| **Remove unsupported fields…** | Remove ID3v2 frames TagGroove does not display | MP3/AAC/AIFF/WAV applicability; dialog explains applicable/skipped counts |
| **Optimize…** | Normalize FLAC padding or reorganize MP4 layout | FLAC/MP4 applicability; writes only when a change is needed |
| **Configure tools…** | Open the Tools Options editor | Saves definitions/preferences through the Options dialog; opening it does not launch a program |

The four built-in utilities have no dedicated shortcuts displayed in this
menu. The file-list context menu also groups them under **Utils**. Context
placement does not turn them into actions or change their write/recovery scope.

Read [tag removal](../../tagging/remove-tags.md) before erasing a metadata block.
**Remove unsupported fields…** is not a command to delete every custom field,
and **Optimize…** does not re-encode audio or promise every player will read
it. [Chapters](../../tagging/chapters.md) explains why stripping a parent file's
chapters is different from editing one displayed chapter title.

## Configured program entries

Saved tool definitions are appended after the fixed items, separated from
them. Each displays its saved **Name** and launches that definition. With no
definitions, the menu explains that none exist and points to **Configure
tools…**. Removing or reordering a definition changes this generated part of
the menu; it does not install/uninstall the executable.

The first ten definitions display ++cmd+1++ through ++cmd+0++ on macOS or
++ctrl+1++ through ++ctrl+0++ on Windows/Linux. The tenth uses 0. Later tools
remain available by menu selection without a displayed digit shortcut.
Reordering changes which definition owns each numbered slot. These are tools'
shortcuts, not the Alt-digit converter shortcuts.

For example, with a saved tool named `Inspect track`, choose **Tools → Inspect
track** to run it. Its **Path**, **Parameter** and per-file checkbox determine
what is prepared; the menu name is not a shell command. A custom toolbar may
reference the same saved name while displaying a different button label.
Distinct tool names make those references easier to maintain.

The **Default tool (double-click in the file list)** setting is separate from
menu ordering. A chosen definition follows its current-selection/per-file
rules on a row double-click. **No action** leaves that double-click without an
action; use **Edit → Extended tags…** to open the field editor. See
[tool configuration](../../customize/tools.md#shortcuts-default-tool-and-custom-buttons)
for that deliberate distinction.

## Selection and availability

Utilities require an applicable selection and supported mutation access.
Selected pending edits must be resolved first; the unavailable-command reason
can say **Save pending changes first.** Mixed-format utility selections may
skip formats outside that utility's list. An applicable count is not a count
of successfully rewritten files.

Chapter rows are not independent audio files. Supported whole-file utilities
can act once on their underlying parent, while cue-sheet track rows have
narrower field-editing scope. Provider documents and restricted sources follow
additional access/mutation rules. Do not use the presence of a familiar menu
label as proof that every desktop operation is supported there.

Named external tools require the desktop/local launch route and a selection.
Their disabled entries can show a reason. Unlike built-in cleanup, a tool
launch does not save pending tag-panel values first. If the other application
should read those changes, save deliberately before running it. A hidden
selected row can still be a target; review selection before batch launching.

With per-file launching off, the tool runs once for the first selected file.
With it on, it prepares one launch per selected file, asks above ten targets
and rejects a per-file request above 100. The 99-definition limit is different
from that runtime target limit. Program launch and final task success are
also different: **Started** does not report final exit status or prove that
another program opened one window per file.

## Error and recovery scope

Built-in utilities can partially complete and can have limited/unavailable
session Undo. Check their result, errors and current history notice.
An external program's edits and outputs are outside TagGroove's general Undo
contract. Canceling or closing an Options dialog does not reverse a program
that already started.

Use copied files for a new definition and retain independent backups before
removing data. The [Undo guide](../../tagging/undo-history.md) distinguishes
recorded local writes, side images, pending edits, unrecorded file operations
and external outputs.

## Related

- [Built-in utilities](../../tagging/utilities.md)
- [Remove tags](../../tagging/remove-tags.md)
- [External tools](../../customize/tools.md)
- [Toolbars](../../customize/toolbars.md)
- [Keyboard shortcuts](../keyboard-shortcuts.md)

<!-- verified: src/main.ts:menubar Tools assembly, menu, operationReason/runUtil; src/tools.ts:ToolsUI.menuHTML, toolShortcut, run; src/utils-ui.ts; src/chapters.ts; src-tauri/src/utilities.rs. All five fixed current items and dynamic tool entries covered; no native launch or unmerged feature claim. -->
