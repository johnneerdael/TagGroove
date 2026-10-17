# Tag files from folder and file names

<div class="recipe-meta" markdown>
<span>**Uses** Filename → Tag</span>
<span>**Works on** files in `Artist/Album/` folders</span>
<span>**Time** 2 minutes per batch</span>
<span>**Undo** ++cmd+z++ / ++ctrl+z++</span>
</div>

## The problem

Your files have empty or wrong tags, but they are well organised on disk.
The folders name the artist and the album, and the file names hold the track
number and title:

```text
Music/
└── Juniper Vale/
    └── Harbour Lights (2024)/
        ├── 01 - Low Tide.mp3
        ├── 02 - Lanterns on the Water.mp3
        └── 03 - Paper Boats.mp3
```

You want all of that in the tags:

| File | `ARTIST` | `ALBUM` | `YEAR` | `TRACK` | `TITLE` |
|---|---|---|---|---|---|
| `01 - Low Tide.mp3` | `Juniper Vale` | `Harbour Lights` | `2024` | `01` | `Low Tide` |
| `02 - Lanterns on the Water.mp3` | `Juniper Vale` | `Harbour Lights` | `2024` | `02` | `Lanterns on the Water` |

## The solution

Try this on a copy of a few files first. Check the results before applying
it to your library; use **Edit → Undo** in the same session to reverse the
recorded changes. Keep the originals as a backup.

1. Open the `Music` folder, or the artist's folder, and select the files.
   Selecting all of them (++cmd+a++ / ++ctrl+a++) is fine as long as they
   all follow the same layout.
2. Choose **Convert → Filename → Tag…** (++alt+2++; Alt is ⌥ Option on a Mac).
3. Enter this pattern:

    ```text
    %artist%\%album% (%year%)\%track% - %title%
    ```

4. Click **Preview** and check the values for a few files.
5. Click **Apply**. The tags are written to the files straight away; there
   is nothing left to save.

## How it works

A Filename → Tag pattern is a picture of the name, with placeholders where
the values are. OpenTag lines the pattern up against the end of each file's
path:

```text
pattern:  %artist%     \ %album%       (%year%)\ %track% - %title%
path:     Juniper Vale \ Harbour Lights (2024) \ 02      - Lanterns on the Water.mp3
```

- `\` stands for a folder level. The pattern has two, so OpenTag looks at
  the file name and the two folders above it. Everything higher up (`Music`)
  is ignored. Use `\` on macOS and Linux too.
- The text between placeholders must appear exactly, here ` - `, ` (` and `)`.
- The extension is left out automatically.
- The whole name must fit for that file to change. Files that do not
  match are skipped. The **Apply** button shows the number of matching
  files and stays disabled when none match. Review every row; use the
  filter or sort by path to identify files that need a different pattern.

!!! warning "Gotcha: brackets inside album names"
    Placeholders take as little text as possible. For the folder
    `Harbour Lights (Deluxe) (2024)`, `%album%` stops at the first ` (`, so
    `ALBUM` becomes `Harbour Lights` and `YEAR` becomes `Deluxe) (2024`.
    Check the preview for such albums and run them separately with a
    pattern that matches their folder names, such as
    `%artist%\%album% (Deluxe) (%year%)\%track% - %title%`. That pattern
    leaves *(Deluxe)* out of the album name. To keep it, run
    `%artist%\%album%\%track% - %title%` and remove the year from the album
    afterwards with the regular expression `\s*\(\d{4}\)$` (see
    [Regular expressions](../reference/regular-expressions.md)).

## Variations

**No year in the album folder**

```text
%artist%\%album%\%track% - %title%
```

**No track number in the file names**

```text
%artist%\%album%\%title%
```

Then number the tracks with Auto-numbering,
after sorting the file list by file name.

**Disc folders (`CD1`, `CD2`) inside the album**

```text
%artist%\%album%\CD%discnumber%\%track% - %title%
```

**Skip a part you don't need**

`%dummy%` matches text without storing it. For files named
`Juniper Vale - 02 - Lanterns on the Water.mp3`, where the artist is already
in the folder:

```text
%artist%\%album%\%dummy% - %track% - %title%
```

**Two artists in the file name**

The same placeholder twice gives a field two values. For
`Juniper Vale & Mira Solberg - Paper Boats.mp3`:

```text
%artist% & %artist% - %title%
```

## Related

- [Your first tagging session](../learn/first-tagging-session.md)
- [Zero-padded track numbers](rename-zero-padded-track-numbers.md)
- [Regular expressions](../reference/regular-expressions.md)
