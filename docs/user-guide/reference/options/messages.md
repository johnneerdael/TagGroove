# Messages options

Open **File → Options… → Messages** to choose which completion messages and confirmations appear around editing operations.

<!-- figure:options-messages -->
<figure markdown>
![The messages page of the Options dialog](../../assets/screens/options-messages-light.png#only-light)
![The messages page of the Options dialog](../../assets/screens/options-messages-dark.png#only-dark)
</figure>
<!-- /figure -->

!!! tip "New to this?"
    Keep confirmations for tag removal, file deletion, and unsaved changes enabled while learning. A completion message and a confirmation serve different purposes: one reports what happened, the other asks before proceeding.

## Completion notifications

All three default to off.

| Setting | When it reports |
|---|---|
| **after saving tags** | After an explicit tag save. It is not a dialog for every directly edited file-list cell. |
| **after actions, converters that write files, and Tag → Tag** | After operations that change file information through those paths. Pending auto-numbering edits remain subject to saving. |
| **after removing tags or tag types** | After tag removal. It does not refer to deleting the audio file. |

The file list and status/error area still provide operational feedback when optional completion dialogs are disabled.

## Confirmations and warnings

| Setting | Default | Effect |
|---|---|---|
| **at removing tags** | On | Ask before removing configured tag types. Review [read/write/remove policies](tags-mpeg.md), since tag removal can affect hidden metadata. |
| **at cutting tags** | Off | Ask before cutting tag information. Cutting involves removal, unlike copying. |
| **at pasting tags** | Off | Ask before applying the tag clipboard to the selection. Enable when frequent copying makes an accidental paste costly. |
| **at exporting and creating playlists (offers to open the written file)** | Off | Offer to open a completed output file using its associated application. This controls a post-write offer, not whether the file may be overwritten. |
| **at deleting files** | On | Ask before filesystem deletion. Normal deletion uses Trash/Recycle Bin even with this confirmation off. Permanent deletion remains a distinct operation. |
| **at warnings from Actions (file errors of the last action or action groups)** | On | Show errors from the last action run. With it off, errors are still listed beneath the file list. |
| **at unsaved changes (closing, reloading, refreshing or discarding pending edits)** | On | Ask when an operation would discard pending edits. With it off, those edits can be discarded without that prompt. |
| **at importing covers (when files already have cover art)** | On | Ask whether existing embedded images should be preserved during a tag-source import. |

The unsaved-changes message is not an automatic-save mode. Use [Tags](tags.md) to configure selection-change saving. Avoid disabling the prompt merely to silence a workflow whose save behavior is not yet understood.

## Existing artwork without a question

When **at importing covers** is disabled, **Without asking, existing cover art is** becomes available:

- **Preserved** keeps existing embedded artwork.
- **Replaced or added to, as chosen in the import** follows the cover operation selected in the import. This is the default remembered choice.

The tag-source prompt can remember its answer through **Don't show this message again**; later imports then reuse that choice. Re-enable the message here to review each import again. Preserving existing art and saving a downloaded image alongside an album are separate decisions.

!!! warning "Discarding edits"
    Turning off **at unsaved changes** removes a chance to recover unsaved work. Save tags before closing, reloading, or using a command that discards pending changes.

## Related

- [Tags and automatic saving](tags.md)
- [Tag Sources and cover files](tag-sources.md)
- [Playlist output](playlist.md)

<!-- verified: src/messages.ts MESSAGE_DEFAULTS, messagesOptionsPage, Messages.confirm/notify; src/main.ts message call sites, choosePreserveExistingCovers; src/file-ops.ts confirmation paths. Permanent-delete wording kept separate from normal trash behavior. -->
