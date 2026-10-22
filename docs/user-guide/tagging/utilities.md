# Clean stored metadata and container layout

The built-in utilities remove particular stored information or change a
container's layout. They are not audio repair, loudness normalization or a
general promise that another player will recognize a damaged file. Choose a
specific goal and test one copied ordinary local file first.

## Choose a utility

These commands are available under **Tools** and in the file-list context
menu's **Utils** section.

| Goal | Command | Current scope |
| --- | --- | --- |
| Remove explicitly chosen metadata blocks | **Remove tag types…** | Selected supported ID3v1, ID3v2 or APEv2 blocks |
| Remove ID3v2 data TagGroove does not display | **Remove unsupported fields…** | MP3, AAC, AIFF and WAV ID3v2 tags |
| Set FLAC padding or reorganize MP4 data | **Optimize…** | FLAC and MP4 |
| Remove MP4 chapter structure | **Strip chapters MP4…** | Selected MP4 files with chapters |

Save or discard the selected files' pending edits before starting. These
commands write when applied; they do not stage a change for a later
**Save tags**. Supported local operations can enter the session journal,
but check [Undo's availability and limits](undo-history.md) rather than
assuming every cleanup is recoverable.

## Remove a particular tag type

Use **Tools → Remove tag types…** when the task is “remove the old ID3v1
block but retain ID3v2,” rather than “clear the displayed Title.”

1. Inspect the file's tag-type information. On a practice MP3 it might list
   `ID3v2.4 APEv2 ID3v1`.
2. Open **Remove tag types…**. Each choice shows how many selected files
   contain it; types absent from the selection are disabled.
3. Check only **ID3v1** for this example. Leave **ID3v2** and **APEv2** off.
4. Choose **Remove**, inspect any errors, then check the remaining tag types.
   The intended result is `ID3v2.4 APEv2`.

These explicit choices do not use the **Remove** switches under
**Options → Tags**. The separate **File → Remove tag…** command does use
those configured policies. Read the complete
[tag-removal guide](remove-tags.md) for supported format families, preservation
limits and why the displayed title can change when a different block becomes
visible.

An APEv2 block can contain ReplayGain data and `MP3GAIN_UNDO`. Removing the
block erases that information; it does not reverse earlier gain changes.
Keep the original file when future reversal or a comparison may matter.
Deleting a whole ID3v2 block can also remove covers and custom fields that
another retained block cannot represent.

## Inspect and remove unsupported ID3v2 frames

A file can contain ID3v2 frames with no editable field or cover representation
in TagGroove. **Remove unsupported fields…** targets those frames, rather
than all custom field names or every format's opaque data.

1. Add a temporary [file-list column](../customize/columns.md) with the
   format string below, or use it while inspecting a copied file:

    ```text
    %_id3v2_unknown_frames%
    ```

2. Inspect the identifiers before deleting them. For a deliberately seeded
   practice file, the information can read `GEOB, PRIV, UFID` while a normal
   title and cover are also present.
3. Decide whether another application might need that data. “Unsupported by
   this editor” does not mean meaningless or unwanted everywhere.
4. Choose **Tools → Remove unsupported fields…**, read the applicable-file
   count and choose **Remove unsupported fields**.
5. Recheck the technical column, title and cover. Files with nothing to remove
   are returned without a rewrite; an empty technical value alone is not an
   exhaustive proof that a container contains no opaque information.

Examples of the current removal policy include private frames, general binary
objects, relative-volume/event/ownership frames and some unrecognized unique
identifiers. Recognized identifiers such as MusicBrainz's unique identifier
are retained by their supported mapping. Do not turn this into an “erase every
UFID” rule or a universal sanitization command.

The native utility targets ID3v2 in MP3, AAC, AIFF and WAV. FLAC Vorbis comments,
MP4 atoms and arbitrary binary fields in other containers are outside this
command. On a mixed selection, the UI filters by the applicable format list
before invoking this utility.

<!-- utility-scope-cases:start -->
| Selected formats | Remove unsupported fields targets | Optimize targets |
| --- | --- | --- |
| `MP3, FLAC, MP4` | `1` | `2` |
| `AAC, AIFF, WAV` | `3` | `0` |
| `FLAC, FLAC` | `0` | `2` |
| `MP3` | `1` | `0` |
<!-- utility-scope-cases:end -->

These counts mean **applicable targets**, not a promise that every target
needs rewriting or will succeed. For example, an ordinary MP3 with no unknown
ID3v2 frames can be an applicable no-op.

## Optimize FLAC padding

FLAC reserves padding alongside metadata so that later small tag changes may
fit without relocating the audio frames. **Optimize…** normalizes that padding
to one 4,096-byte block. It can shrink excessive padding or add missing padding;
it does not change the compression level or improve audio quality.

1. Save edits and select the copied FLAC file.
2. Choose **Tools → Optimize…**. Check the count and the explanation before
   choosing **Optimize** in the dialog.
3. Check the result/errors and the retained fields/artwork. The file size can
   increase or decrease; a smaller file is not the sole success criterion.
4. If a player or file manager was the original problem, refresh its view and
   test the actual file there. An TagGroove success message does not qualify
   that other program's metadata limits.

A FLAC file with a leading ID3v2 tag is rejected by this optimization path:
it expects the native FLAC stream marker at the start. Keep a backup and
inspect the tag types before attempting any separate cleanup. Do not remove
all native FLAC metadata or rename an extension merely to bypass that error.
Malformed metadata blocks can also prevent optimization.

The writer verifies the expected metadata region and unchanged encoded audio
bytes before publishing the staged result. That check concerns this operation;
it is not a repair guarantee for an already damaged input. Already normalized
files need no rewrite.

## Optimize MP4 layout

For MP4, Optimize removes unused `free` space and places the movie/metadata
structure before media data. It adjusts the layout rather than re-encoding
or resampling the track. A file already in the optimized layout is returned
without a rewrite.

Use the same copied-file procedure and inspect the result. A layout rewrite
can require more temporary storage than a small metadata edit, and the Undo
snapshot can have a larger cost. Ensure adequate free space and retain a
backup; a layout change does not prove compatibility with every MP4 variant,
audio/video codec or player.

## Strip MP4 chapters deliberately

Chapter navigation and chapter-specific fields may matter for an audiobook
or long recording. Keep the original if you need those divisions later.

1. Resolve pending edits and select the copied MP4. The command is enabled
   when selected MP4 information reports chapters.
2. Choose **Tools → Strip chapters MP4…**. Read the number of files and
   chapters in the normal removal confirmation.
3. Confirm only if losing the navigation is intended. This writes immediately;
   the confirmation preference can affect whether the prompt is displayed.
4. Inspect the returned parent file and its chapter information. Listed chapter
   rows can be replaced by the ordinary parent row when chapters disappear.

The removal turns the targeted chapter boxes into same-sized `free` boxes.
It does not promise a smaller file. Audio/video bytes and unrelated tag regions
are kept by this path's verification; the removed chapter structure is gone.
If removing the resulting free space is a separate goal, **Optimize…** is a
separate operation with its own storage and history cost.

Selecting an MP4 chapter row applies this whole-file utility to its underlying
file, not just that one chapter. Cue-sheet track rows are different: they edit
the sheet and reject this utility. See [Chapters](chapters.md) before applying
whole-file removal to a displayed track list. A file with no applicable MP4
chapter structure does not acquire a new chapter-removal result merely because
its extension resembles an audiobook.

## Gotchas and recovery

Utilities can complete for some files and fail for others. Read per-file errors
and check the current **Undo …** label and notice. With a partial failure, do
not assume that Cancel or closing the dialog restored completed writes. Reduce
the retry selection to files whose state you have actually inspected.

Undo restores a recorded local-file snapshot during the current session;
it is not the original editor's “rewrite only the displayed tag type using
current Options” recovery policy. Snapshot failures or limits can leave the
operation without complete history. Quit only after keeping any state that
must survive the session in an independent backup.

Provider documents and restricted sources have their own mutation/access
rules; this guide does not qualify these utilities for arbitrary provider
save-back, native OS dialogs or player behavior. For a problem caused by a
pending value or reader policy, correct that cause before deleting stored data.

## Related

- [Remove tags or selected types](remove-tags.md)
- [Undo and history](undo-history.md)
- [Tag formats and reading policies](concepts/tag-formats.md)
- [Chapters](chapters.md)
- [Troubleshooting](../help/troubleshooting.md)

<!-- verified: src/utils-ui.ts:UTILS, utilTargets, removeTagTypesDialog, mp4ChapterFiles; src/main.ts:runUtil, strip-mp4-chapters; src-tauri/src/utilities.rs; crates/tag-core/src/optimize.rs, mp4_chapters.rs. Existing focused generated-file checks; no native UI, configured external program or universal repair/player claim. -->
