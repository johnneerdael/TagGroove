# Placeholders

<p class="lead">A placeholder is a name between percent signs. OpenTag replaces it with a value for each file: <code>%title%</code> with the title, <code>%_bitrate%</code> with the bitrate, <code>%_folderpath%</code> with the folder the file is in.</p>

This page lists every placeholder OpenTag understands, grouped by what it
tells you. For the rules of the language around them (optional sections,
quoting, conditions), see [Format strings](format-strings.md).

!!! tip "Finding them while you type"
    You don't have to remember these names. Press the **►** button next to
    any format string input, or ++ctrl+space++ (Control on every platform), and
    pick a placeholder from the menu.

## Two kinds of placeholders

**Tag fields** have plain names: `%artist%`, `%album%`, `%mood%`. They read
the file's tag, and any field name works. A missing field gives empty text.
See [Placeholders: `%field%`](format-strings.md#placeholders) for the
details. Common field names include `ARTIST`, `TITLE`, `ALBUM` and `TRACK`.

**Information placeholders** start with an underscore: `%_length%`,
`%_covers%`. OpenTag works them out from the file itself, its location, its
audio stream or the current operation. You can read them but not write them.

The rest of this page lists the information placeholders. All of them:

- are **case-insensitive**: `%_BITRATE%` and `%_bitrate%` are the same,
- give **empty text when the value can't be measured** for a file (for
  example `%_bitspersample%` on an MP3), so you can test them with `[…]` and
  `$if`,
- are **read-only**: you can't set them as a field in Tag → Tag or the tag
  panel. To rename a file, use the `_FILENAME` and `_DIRECTORY`
  targets instead.

The example values are for track 2 of the guide's example album, stored at
`/Users/you/Music/Juniper Vale/Harbour Lights/02 - Lanterns on the Water.mp3`
on a Mac. On Windows the paths use `\` and a drive letter.

## File name and location

| Placeholder | Gives | Example |
|---|---|---|
| `%_path%` | The full path of the file | `/Users/you/Music/Juniper Vale/Harbour Lights/02 - Lanterns on the Water.mp3` |
| `%_filename%` | The file name without extension | `02 - Lanterns on the Water` |
| `%_filename_ext%` | The file name with extension | `02 - Lanterns on the Water.mp3` |
| `%_extension%` | The extension, without the dot | `mp3` |
| `%_folderpath%` | The folder, **ending with a separator** | `/Users/you/Music/Juniper Vale/Harbour Lights/` |
| `%_directory%` | The name of the folder the file is in | `Harbour Lights` |
| `%_parent_directory%` | The name of the folder above that | `Juniper Vale` |
| `%_volume%` | The name (label) of the disk the file is on | `Music Drive` |
| `%_workingdir%` | The name of the folder you opened in OpenTag | `Music` |
| `%_workingpath%` | The full path of that folder, ending with a separator | `/Users/you/Music/` |

Because `%_folderpath%` already ends with a separator, append a file name
directly: `%_folderpath%folder.jpg`.

`%_workingdir%` and `%_workingpath%` are the folder you chose with
**File → Change directory…** (or the first folder you opened). They are
handy for paths relative to your library's root, and empty when you opened
single files.

## Audio and technical information

| Placeholder | Gives | Example |
|---|---|---|
| `%_length%` | Playing time, `m:ss`, or `h:mm:ss` from one hour | `4:04`, `1:15:03` |
| `%_length_seconds%` | Playing time in whole seconds | `244` |
| `%_bitrate%` | Average bitrate in kbit/s | `192` |
| `%_samplerate%` | Sample rate in Hz | `44100` |
| `%_bitspersample%` | Bit depth (lossless and PCM formats only) | `16`, `24` |
| `%_mode%` | Channels: the MPEG channel mode for MP3, otherwise `Mono`, `Stereo` or `n channels` | `Joint Stereo` |
| `%_codec%` | The audio codec | `MPEG-1 Layer III`, `FLAC`, `AAC`, `ALAC`, `Vorbis`, `Opus`, `PCM`, `Monkey's Audio`, `Windows Media Audio` |
| `%_vbr%` | MP3: `VBR`, `CBR` or `ABR`. Monkey's Audio: the compression level | `CBR`, `Extra High` |
| `%_tool%` | The encoder that made the file (LAME version, Vorbis vendor) | `LAME3.100` |
| `%_filetype%` | The container format | `WAV` |

### Video (MP4 and Matroska files)

| Placeholder | Gives | Example |
|---|---|---|
| `%_video_codec%` | Video codec | `MPEG-4 Visual` |
| `%_video_width%`, `%_video_height%` | Frame size in pixels | `1920`, `1080` |
| `%_video_bitrate%` | Video bitrate in kbit/s | `4500` |
| `%_segment_title%` | Matroska only: the segment title | `Live in Bergen` |

### Chapters

When chapters are listed as separate rows, each
chapter row has its own values:

| Placeholder | Gives | Example |
|---|---|---|
| `%_subsong%` | The chapter number, counting from 0 | `0` for a file without chapters |
| `%_subsong_count%` | The number of chapters in the file | `1` for a file without chapters |

### MP4 dates

| Placeholder | Gives | Example |
|---|---|---|
| `%_mp4_creation_time%`, `%_mp4_modification_time%` | The dates stored inside an MP4/M4A file, as local date and time | `1/2/24 3:04:05 AM` |
| `%_mp4_creation_time_raw%`, `%_mp4_modification_time_raw%` | The same, as seconds since 1 January 1970 (for `$fmtdate`) | `1704164645` |

## Tags inside the file

| Placeholder | Gives | Example |
|---|---|---|
| `%_tag%` | Every tag type in the file, separated by spaces | `ID3v2.3 ID3v1` |
| `%_tag_read%` | The tag OpenTag is showing for this file | `ID3v2.3` |
| `%_tag_size%` | Size of the tags in bytes | `4096` |
| `%_tag_size_prepended%`, `%_tag_size_appended%` | Tag bytes before and after the audio | `4096`, `128` |
| `%_id3v2_character_encoding%` | Text encodings used in the ID3v2 tag | `UTF-16` |
| `%_id3v2_unknown_frames%` | ID3v2 frames OpenTag keeps but doesn't interpret | `XABC` |

`%_tag_size%` and its variants exist for MP3, APE, FLAC, AAC, Musepack and
WavPack files. The other two only exist for files with an ID3v2 tag.

## Cover art

These describe the **first** picture in the file.

| Placeholder | Gives | Example |
|---|---|---|
| `%_covers%` | How many pictures the file has | `1`; `0` without covers |
| `%_cover_type%` | The picture type | `Front Cover` |
| `%_cover_mimetype%` | The image format | `image/jpeg` |
| `%_cover_description%` | The picture's description | `Album cover` |
| `%_cover_size%` | Size in bytes | `5843` |
| `%_cover_width%`, `%_cover_height%` | Size in pixels (JPEG and PNG only) | `600`, `600` |

`%_covers%` is the one to filter on. `"%_covers%" EQUAL 0` shows every file
without artwork.

## File properties

| Placeholder | Gives | Example |
|---|---|---|
| `%_file_size%` | Size, rounded, with a unit (B, KB or MB) | `5.6 MB` |
| `%_file_size_bytes%` | Size in bytes | `5878272` |
| `%_file_size_kb%` | Size in whole KiB | `5740` |
| `%_file_size_mb%` | Size in MiB with two decimals | `5.61` |
| `%_file_mod_date%` | Last modified, as a short local date | `10/15/26` |
| `%_file_mod_datetime%` | Last modified, local date and time | `10/15/26 4:12:09 PM` |
| `%_file_mod_datetime_raw%` | Last modified, seconds since 1 January 1970 | `1792073529` |
| `%_file_create_date%`, `%_file_create_datetime%`, `%_file_create_datetime_raw%` | The same for the creation time | |

Dates use your system's date format. For a fixed format, pass the `_raw`
value to `$fmtdate`:
`$fmtDate('yyyy-MM-dd',%_file_mod_datetime_raw%)`.

The creation-time placeholders are empty on file systems that don't record
when a file was created (many Linux file systems).

## Information about the operation

| Placeholder | Gives | Example |
|---|---|---|
| `%_counter%` | The file's position in the current operation: 1 for the first selected file, 2 for the next | `2` |
| `%_date%` | Today's date, short local format | `10/15/26` |
| `%_datetime%` | Now, local date and time | `10/15/26 4:12:09 PM` |
| `%_app%` | The program name and version | `OpenTag v0.1.0` |

`%_counter%` follows the order of the files in the file list, so sort the list
first when you use it to number files. For track numbers, the
Auto-numbering wizard is usually the better
tool.

## Export only

These exist only inside export templates,
because they describe the export as a whole or need extra work per file.

| Placeholder | Gives | Example |
|---|---|---|
| `%_counter%` | The number of the current file in the loop | `2` |
| `%_max_counter%` | The highest counter value of the loop | `8` |
| `%_total_files%` | Number of exported files | `8` |
| `%_total_size%` | Their total size, with a unit and your decimal mark | `39.80 MB` |
| `%_total_size_raw%` | Their total size in bytes | `41733120` |
| `%_total_time%` | Their total playing time, `hh:mm:ss`, with a day count from one day (`1d 02:03:04`) | `00:28:58` |
| `%_total_time2%` | The same, but hours keep counting past 24 | `00:28:58` |
| `%_total_time_raw%` | Their total playing time in seconds | `1738` |
| `%_filename_rel%` | The file's path relative to the export file | `Harbour Lights/02 - Lanterns on the Water.mp3` |
| `%_folderpath_rel%` | Its folder relative to the export file | `Harbour Lights/` |
| `%_crc%` | CRC32 checksum of the file, 8 hexadecimal digits | `4F2A91C0` |
| `%_md5%` | MD5 checksum of the file, 32 hexadecimal digits | `9e107d9d372bb6826bd81d3542a419d6` |
| `%_md5audio%` | The audio checksum stored in FLAC and WavPack files, otherwise an MD5 of the audio without tags. Empty when unavailable | |

Checksums take time to compute on large selections, because OpenTag has to
read every file completely.

## Playlists only

In the extended-info format string and file name of
playlists:

| Placeholder | Gives | Example |
|---|---|---|
| `%_playlist_filename%` | The playlist's file name without extension | `Harbour Lights` |
| `%_playlist_filename_ext%` | With extension | `Harbour Lights.m3u8` |
| `%_playlist_folderpath%` | The playlist's folder, ending with a separator | `/Users/you/Music/Juniper Vale/Harbour Lights/` |

## Web source scripts only

`%_url%`, `%_coverurl%`, `%_preview%` and `%_current%` exist only inside
web source scripts, where they
describe the page being parsed.

## Placeholders compared with Mp3tag

!!! info "Coming from Mp3tag"
    OpenTag reads the same placeholder names as Mp3tag, so your format
    strings, columns and export templates keep working. OpenTag adds two:
    `%_filetype%` (the container format) and `%_segment_title%` (the
    Matroska segment title).

## Related

- [Format strings](format-strings.md)
- [Zero-padded track numbers](../cookbook/rename-zero-padded-track-numbers.md)
