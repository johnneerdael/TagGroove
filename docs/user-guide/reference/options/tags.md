# Tags options

Open **File → Options… → Tags** to choose when edits are saved and which images the tag panel displays. The child pages configure [Mpeg](tags-mpeg.md), [Ape, Mpc](tags-ape-mpc.md), [Mapping](tags-mapping.md), and [Advanced](tags-advanced.md) behavior.

!!! tip "New to this?"
    An edited field can be pending in TagGroove while the audio file on disk is unchanged. Keep the default pending mode to review a batch before using **Save tags**.

## Save tags when changing selection

**Save tags when using arrow keys or a single mouse click** defaults to off. Enable it to write pending tag-panel edits when selecting another file with the mouse, moving the file-list selection with arrow keys, or choosing **Select next file**. Clicking an empty area of the file list or choosing **Unselect all** also leaves the current selection and can save its pending tags. Moving selection can therefore modify files without a separate Save command. **Select next file** also has ++cmd+n++ / ++ctrl+n++ and can keep focus in the tag panel. This automatic-save path applies to local desktop files; provider-backed documents are excluded.

With it off, selection changes do not provide this automatic write. Review pending changes and choose **Save tags** deliberately. [Messages → at unsaved changes](messages.md) controls prompts when other commands would discard edits; disabling that message is not a way to enable saving.

## Save mode at extended tag dialog

This controls changed fields when **<<** or **>>** moves between files in **Extended tags…**:

| Mode | Result |
|---|---|
| **Keep changes pending** | Default. Keep the edits in TagGroove for a later **Save tags**. |
| **Ask** | Ask how to handle the changed fields before moving. |
| **Ignore changes** | Discard the dialog's changes for the file being left. |
| **Save changes** | Write changes when moving to the other file. A save failure must be resolved rather than treated as a successful move. |

**Save tags** in the editor writes the displayed selection and closes after
success. **Keep changes** stages edits for a later save. The mode above
controls **<<**/**>>** navigation independently of the main file list's
save-on-selection setting. Pending mode does not mean the file has already
been saved. TagGroove's explicit pending mode is an addition to Mp3tag's
documented ask/ignore/save choices.

## Folder images in the tag panel

**Don't display first image from file directory as cover art** defaults to off. When a selected file has no embedded cover, TagGroove may show an image from its folder. This is a display fallback: seeing the picture does not mean it is stored in the audio file.

Enable the option when auditing which files actually contain embedded artwork. It hides the fallback without deleting image files. Import a cover explicitly if the audio file should carry it when copied elsewhere.

## Read, write, and remove policies

The format pages separate three decisions:

- **Read** chooses which existing tag is displayed.
- **Write** chooses which tag types receive saved values.
- **Remove** chooses tag types stripped by **Remove tag**. MPEG ordinary Save preserves unwritten types; the [Ape, Mpc policy](tags-ape-mpc.md) has its own write/remove interaction.

Read selection is not a merge of every stored tag. Two programs can show different values when they prioritize different blocks in the same file. Changes to the policy can also affect more metadata than the single field edited: written tag types are synchronized from the displayed values.

The **Mpeg** policy includes MPEG audio, raw AAC, and TTA. **Ape, Mpc** includes Monkey's Audio, Musepack, WavPack, TAK, and OptimFROG. Other formats follow their native storage rather than these switches; **Remove tag** uses their own supported removal path.

## Related

- [Mpeg tag policies](tags-mpeg.md)
- [Ape, Mpc tag policies](tags-ape-mpc.md)
- [Advanced storage options](tags-advanced.md)
- [Messages](messages.md)

<!-- verified: src/options-ui.ts TAGS_PAGE_DEFAULTS; src/main.ts saveOnMove, ExtendedTags host, folderImageFolder; src/extended-tags.ts navigation/save mode; crates/tag-core/src/tag_policy.rs; crates/tag-core/src/raw_streams.rs Stream.mpeg_policy, stand_in. -->
