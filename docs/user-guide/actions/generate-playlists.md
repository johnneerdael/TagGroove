# Generate Playlists

Use **Generate Playlists** to write playlists for the selected files from
an action group. A partition format decides which tracks belong together;
a filename format names each partition's playlist.

## Create one playlist per album folder

1. Select a few copied files and put them in the desired file-list order.
2. Save pending edits, then open **Actions → Actions (Quick)…**.
3. Choose **Generate Playlists**.
4. Set **Partition format** to `%_folderpath%`.
5. Set **Playlist filename format** to `%_folderpath%playlist.m3u8`.
6. Choose **Apply action** and inspect each folder's playlist.

Tracks with the same folder path enter one playlist, preserving their
relative order in the file list. The `.m3u8` extension requests an M3U8
playlist; choose an extension supported by the playlist writer.

## Understand partitions

TagGroove evaluates **Partition format** for every selected track. Tracks
with exactly equal results form a group. For each group, it evaluates
**Playlist filename format** using that group's first track.

| Goal | Partition format | Playlist filename format |
|---|---|---|
| One playlist in every folder | `%_folderpath%` | `%_folderpath%playlist.m3u8` |
| One per disc in an album folder | `%_folderpath%-%discnumber%` | `%_folderpath%disc-%discnumber%.m3u8` |
| One for the entire selection | `all` | An absolute path chosen for the combined playlist |

The partition value is a grouping key, not an output path. The filename
must distinguish different groups: two partitions that resolve to the
same destination report an error. Adding `%discnumber%` to the partition
alone would not give each disc a distinct output filename.

A relative filename is resolved beside the first track in that partition.
For a combined playlist spanning folders, choose an explicit absolute
destination so the result does not depend on which track comes first.

The filename format uses literal `\` as a folder separator on every platform.
Separators and reserved filename characters in ordinary tag values are
cleaned; native path placeholders such as `%_folderpath%` retain their path
components. The partition format remains a grouping key, not a filename.

## Order and playlist options

This action uses the current playlist settings for entry paths, extended
information, encoding where applicable, and writing behavior. Inspect the
result in the player that will open it, especially when moving the music
and playlists to another machine.

In an action group, playlist generation runs after the group's per-file
actions. A preceding rename therefore appears in the playlist, and even a
playlist action listed before that rename sees the final values. Files with
an earlier action warning still participate. Put it in a separate earlier
group if an intermediate state is required.

Action-group `.mta` exports use native action type 19 with separate filename
and partition expressions. Older TagGroove type 1003 files remain readable.
The action record does not carry the application's global playlist settings;
check those settings in the receiving application before running an exchanged
group. Native schema support does not establish matching output encoding or
player acceptance on every platform.

## Gotchas

- The action includes selected files, not every track in the folder.
- File-list order matters; the action does not automatically sort by
  track number.
- Avoid filename collisions with audio files or other partitions.
- Generated playlists may require separate cleanup. Do not assume session
  undo removes every output file, or that a failed batch rolled back all
  earlier file effects.

<!-- verified: src-tauri/src/action_host.rs:NativeHost::execute_batch; crates/tag-core/src/actions.rs:execute_groups -->

## Related

- [Export](export.md)
- [Format strings](../reference/format-strings.md)
- [Placeholders](../reference/placeholders.md)
- [Actions and action groups](index.md)
