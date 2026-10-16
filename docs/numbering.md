# Number selected files

Select files and choose **Convert → Auto-numbering**, the **Numbering** toolbar button, or **Ctrl K** (**Cmd K** on macOS). Save existing pending edits first. Numbers follow the current table order.

Set a track start and minimum digit count. Enable an optional total and enter its value. The total starts with the selected file count; it remains the entered value when counters reset. **Ctrl Shift K** (**Cmd Shift K**) numbers each folder from the start again and saves each folder's own total instead. Enable disc numbers to set a separate start, digit count, and total. One minimum digit means no extra leading zeros. Individual tag formats can normalize numeric values, including MP4 removing leading zeros.

Use the counter-change controls to increment discs when adjacent files change folder or a selected tag field. Either trigger advances the counter once per row. Enable track resets to restart at the specified track start for each new disc. A separate field can reset disc numbering, for example ALBUM; this also restarts tracks and takes precedence over an increment on the same row. Field comparison uses the original loaded field values, including multivalues.

Click **Preview**, review the sequence, and click **Stage numbers**. Files remain unchanged until **Save tags**. Changing the selection or options requires another preview. **Cancel** leaves no new drafts. Directory grouping is unavailable when the files have no known local folders.

The controls follow the [documented Mp3tag numbering workflow](https://docs.mp3tag.de/converters/auto-numbering/). Original-runtime edge-case equivalence, saved-operation undo, and remembering wizard settings remain open. See the [verification record](verification/numbering.md).
