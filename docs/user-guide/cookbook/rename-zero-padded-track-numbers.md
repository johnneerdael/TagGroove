# Zero-padded track numbers in filenames

<div class="recipe-meta" markdown>
<span>**Uses** Tag → Filename, `$num`</span>
<span>**Works on** any files with a `TRACK` tag</span>
<span>**Time** 2 minutes</span>
<span>**Undo** ++cmd+z++ / ++ctrl+z++</span>
</div>

## The problem

Your file names start with the track number, but some have one digit and
some two, and a few also have the total. File managers and car stereos sort
them in the wrong order:

| Before | After |
|---|---|
| `1 Low Tide.mp3` | `01 Low Tide.mp3` |
| `10 Bonus Light.mp3` | `10 Bonus Light.mp3` |
| `2 Lanterns on the Water.mp3` | `02 Lanterns on the Water.mp3` |
| `3-12 Paper Boats.mp3` | `03 Paper Boats.mp3` |

Sorted as text, `10` comes before `2`. With two digits everywhere, it doesn't.

## The solution

Try this on a copy of a few files first. Check the results before applying
it to your library; use **Edit → Undo** in the same session to reverse the
recorded changes. Keep the originals as a backup.

1. Select the files (++cmd+a++ / ++ctrl+a++ selects all of them).
2. Choose **Convert → Tag → Filename…** (++alt+1++; Alt is ⌥ Option on a Mac).
3. Enter this format string:

    ```text
    $num(%track%,2) %title%
    ```

4. Click **Preview**, check the new names, then click **Apply**.

## How it works

`%track%` is the track number as stored in the tag. It can be `2`, `02` or
`2/12`, because many programs store the total after a slash.

`$num(x,2)` reads the number at the start of `x` and writes it with at least
two digits:

| `TRACK` tag | `$num(%track%,2)` |
|---|---|
| `2` | `02` |
| `02` | `02` |
| `2/12` | `02` |
| `10` | `10` |
| `112` | `112` (never cut short) |

Everything after the first non-digit is ignored, which is how `/12`
disappears. `%title%` adds the title, and OpenTag keeps the extension.

!!! warning "Check for files without a track number first"
    A file without a `TRACK` tag, or with a vinyl-style track like `B2`, gets
    `00`. To find such files before renaming, type this into the filter
    (++f3++ shows it):

    ```text
    track MATCHES ^[^0-9] OR track MISSING
    ```

    Number them with Auto-numbering or by
    hand, then come back.

## Variations

**Three digits for long compilations or audiobooks**

```text
$num(%track%,3) %title%
```

**Disc number in front, only when there is one**

```text
[$num(%discnumber%,1)-]$num(%track%,2) %title%
```

gives `2-05 Salt & Cedar.mp3` on disc 2, and `05 Salt & Cedar.mp3` on an
album without disc numbers. The [optional section](../reference/format-strings.md#optional-sections)
`[…]` disappears when `DISCNUMBER` is empty.

**A separator after the number**

```text
$num(%track%,2) - %artist% - %title%
```

**Pad the number in the tag too**

To store `02` in the tag itself, use **Convert → Tag → Tag…** (++alt+5++) with
the field `TRACK` and the format string `$num(%track%,2)`. Most players don't
need it, but some car stereos sort by the tag text.

## Related

- [Format strings](../reference/format-strings.md)
- [Placeholders](../reference/placeholders.md)
- [Tags from folder and file names](tags-from-folder-and-filename.md)
