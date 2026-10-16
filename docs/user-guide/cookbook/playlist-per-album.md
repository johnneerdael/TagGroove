# Write one playlist in each album folder

<div class="recipe-meta" markdown>
**Uses:** Generate playlists, folder-path partitioning and M3U8 output.

**Works on:** Copied ordinary local audio files already organized into album folders.

**Time:** About 10 minutes for two practice albums.

**Undo:** Playlists are external outputs written on OK. Keep existing lists and clean up practice outputs deliberately; tag undo is not a playlist backup.
</div>

## The problem

Your visible list spans two album folders. You want a separate playlist in
each folder, with tracks following their current list order. Equal album
titles alone are not a reliable partition: different releases or locations
can share a title. This recipe groups by containing folder.

Keep originals, save intended tag edits, and use fresh practice output names.
Return to whole audio-file rows before starting; chapter/CUE-track rows are
not independent playlist files. Clear a filter if hidden tracks should be
included, and arrange the file-list order deliberately.

## The solution

1. Load two copied album folders. For this example, the visible rows have this
   order. Paths below are relative to a common practice root:

    <!-- album-playlist-members:start -->

    | Order | Folder | File | Disc |
    |---|---|---|---|
    | `1` | `Harbour Lights` | `01 - Paper Boats.wav` | `1` |
    | `2` | `Harbour Sessions` | `01 - North Pier.wav` | `1` |
    | `3` | `Harbour Lights` | `02 - Low Tide.wav` | `2` |

    <!-- album-playlist-members:end -->

2. Choose **Edit → Unselect all** if the generator should use the whole visible
   list. Otherwise, select exactly the subset wanted: a nonempty selection
   takes precedence over all-visible scope.
3. Choose **File → Generate playlists…**. Set **Partition format** to:

    ```text
    %_folderpath%
    ```

4. Set **Playlist file name** to:

    ```text
    %_folderpath%listening.m3u8
    ```

5. Inspect the live preview. It should show two playlists; the full output
   paths end in the following relative names:

    <!-- album-playlist-outputs:start -->

    | Playlist | Members in their playlist order |
    |---|---|
    | `Harbour Lights/listening.m3u8` | `01 - Paper Boats.wav`, `02 - Low Tide.wav` |
    | `Harbour Sessions/listening.m3u8` | `01 - North Pier.wav` |

    <!-- album-playlist-outputs:end -->

6. Check that the outputs are new practice names in the intended folders,
   rather than audio files or existing valuable playlists. Review playlist
   options for extended labels, relative entries and BOM.
7. Choose **OK** to write. Read errors, inspect each output and load a playlist
   back. No later **Save tags** step creates these external files.

## How it works

`%_folderpath%` supplies a path including its trailing separator. Identical
partition results group files together even when another folder's row appears
between them in the list. The generator keeps each group's relative list
order; it does not sort the filenames or track tags independently.

The filename format is evaluated against the group's first readable file.
Here its folder path plus the literal `listening.m3u8` gives a distinct output
inside each folder. The grouping key and filename are separate decisions:
changing the partition alone does not automatically make output names unique.
The `.m3u8` suffix chooses UTF-8 M3U output, not a copy of the audio.

The preview displays the total and at most its first 50 output paths. Changing
inputs updates that planning display; **OK** performs the current write
request. It is not a reservation of filesystem names. A later error is not
proof that no earlier output was written. Keep the original lists and inspect
actual results without assuming universal alias/overwrite protection.

## Variations

**File → Playlist per folder (all files)** is a direct alternative for the
current visible whole-file list. It uses the configured **Filename of
playlist** from Playlist options without individual filename-confirmation
dialogs. A relative configured name is resolved inside each group's folder;
an absolute name overrides that base and can cause different groups to share
one output name. Check the configuration before using this direct command.
Its shortcut is ++cmd+shift+p++ / ++ctrl+shift+p++;
this shortcut invokes the per-folder command in OpenTag, not the generator
dialog. Try the previewed recipe first when destinations are unfamiliar.

For one list per disc within each folder, use these two coordinated formats:

```text
%_folderpath%-%discnumber%
```

```text
%_folderpath%disc-$num(%discnumber%,2).m3u8
```

For the three practice rows, expect:

<!-- album-playlist-disc-outputs:start -->

| Source row | Relative disc playlist |
|---|---|
| `1` | `Harbour Lights/disc-01.m3u8` |
| `2` | `Harbour Sessions/disc-01.m3u8` |
| `3` | `Harbour Lights/disc-02.m3u8` |

<!-- album-playlist-disc-outputs:end -->

Check disc tags first: a missing/nonnumeric disc can create an unintended
`disc-00` name. Two distinct partition values can
still converge on one filename if their numeric formatting discards differences.

For an action-group workflow, use [Generate Playlists](../actions/generate-playlists.md).
It includes selected files and executes after that group's per-file actions,
so preceding renames/field changes can affect the playlist. A recipe that
needs an earlier intermediate state should use a separate group.

## If a playlist is incomplete or in the wrong place

- Check selected versus visible scope, the active filter and ordinary-file
  versus chapter rows. The generator does not scan every unlisted file in a
  folder just because one listed track came from it.
- Check actual list order and readable-file errors. A count of selected paths
  does not prove every file was read successfully.
- Relative output names without `%_folderpath%` use the working-directory
  context. This recipe makes each target explicit with its folder placeholder.
- Relative entries are calculated from each output's folder, subject to the
  configured working root and filesystem roots. Test after moving an album
  with its playlist as a unit; moving the audio independently can break it.
- **Don't create directories** requires existing parent folders; turning it
  off is not permission to overwrite valuable files.

## Related

- [Playlists and cue sheets](../guides/playlists.md)
- [Playlist options](../reference/options/playlist.md)
- [Generate Playlists action](../actions/generate-playlists.md)
- [The file list](../guides/file-list.md)
- [Chapters and CUE rows](../guides/chapters.md)

<!-- verified: playlist_chapter_guide.rs executes the written partition/name expressions per member; guide-playlist-chapters.test.mjs executes actual UI scope and per-folder request models. Grouping/write ordering source-checked in playlists.rs; no native filesystem alias/overwrite or player guarantee. -->
