# Remove a numbered prefix from titles

<div class="recipe-meta" markdown>

**Uses** Replace with Regular Expression; optional Tag → Tag preview

**Works on** Reviewed `TITLE` values in copied local files

**Time** About 5 minutes

**Undo** Recorded action changes in the current desktop session

</div>

## The problem

A filename import left `03 - Paper Boats` in `TITLE`, although `TRACK` already
holds its track number. Remove a recognized prefix, not every number in
the title. Meaningful numbers elsewhere must survive.

In the table, `·` means one space, `⇥` a tab and `↵` a newline. These symbols
make invisible characters readable; do not type them into a title.

<div class="native-mapping-table" markdown>

<!-- numbered-title-cases:start -->

| `TITLE` before | After this action |
| --- | --- |
| `03 - Paper Boats` | `Paper Boats` |
| `8. Low Tide` | `Low Tide` |
| `2) North Pier` | `North Pier` |
| `·003⇥-⇥Quiet Harbour` | `Quiet Harbour` |
| `Room 101 - Window` | `Room 101 - Window` |
| `12 Lanterns` | `12 Lanterns` |
| `03-Paper Boats` | `03-Paper Boats` |
| `03 - Paper Boats↵04 - Low Tide` | `Paper Boats↵04 - Low Tide` |
| `03 -·` | (empty text) |
| (empty text) | (empty text) |

<!-- numbered-title-cases:end -->

</div>

A title containing only a numbered prefix can become empty. Exclude it from
the write and supply the real title first. An intentionally numbered title
can also match this shape; no regex can decide its musical meaning.

## The solution

1. Save or discard pending edits. Select one copied practice file with an
   understood title. Check all its Title values in Extended tags if there
   are several.
2. To preview the first Title value, use **Convert → Tag → Tag…**, Field
   `TITLE`, and this expression:

    <!-- numbered-title-preview:start -->
    ```text
    $regexp(%title%,'(?-m)^[ \t]*\d{1,3}[ \t]*[-.)][ \t]+',)
    ```
    <!-- numbered-title-preview:end -->

    Leave the third scripting argument empty after the final comma. Two
    apostrophes `''` represent a literal apostrophe, not an empty argument.
    Read the result, then **Cancel** this preview. Applying it would format
    from the first value; it is not the multi-value action below.
3. Choose **Actions → Actions (Quick)…**, select **Replace with Regular
   Expression**, and enter `TITLE` in **Field**. Use this **Regular expression**:

    <!-- numbered-title-pattern:start -->
    ```text
    (?-m)^[ \t]*\d{1,3}[ \t]*[-.)][ \t]+
    ```
    <!-- numbered-title-pattern:end -->

4. Leave **Replace matches with** empty and **Case-sensitive comparison** off.
   An empty replacement removes the matched prefix; a space would leave a
   space behind.
5. Choose **Apply action** for the practice file. The action writes
   immediately and processes each Title value independently. Inspect its
   saved values, the untouched `TRACK`, and any file warning.
6. Select only other reviewed files with the intended shape. Keep hidden
   selections out of the batch. Run the action, then inspect each failure
   before considering a retry.

## How it works

| Pattern part | Purpose |
| --- | --- |
| `(?-m)` | Make `^` refer to the start of the whole value, not each line |
| `^` | Require the prefix at the beginning |
| `[ \t]*` | Allow leading spaces/tabs, without consuming line breaks |
| `\d{1,3}` | Read one to three digits |
| `[ \t]*` | Allow spaces/tabs between number and punctuation |
| `[-.)]` | Require one hyphen, dot or closing parenthesis |
| `[ \t]+` | Require at least one space/tab after that punctuation |

The action replaces the matching prefix with nothing. It does not change
`TRACK`, derive totals or rename the audio file. Without a match, the
original text remains. An empty string remains empty; the core action can
retain an empty Title value, while native storage may normalize it on write.

Each existing Title value is transformed separately. The first-value preview
is useful for a practice input, but does not prove a whole multi-value field
will be safe. The newline example changes only the first line's prefix.
Using `\s` for every separator would also admit line breaks and is broader
than the spacing policy taught here.

## Variations

### Another delimiter or more digits

Change the punctuation class or digit bound only after writing matching and
nonmatching examples for your own files. The main pattern deliberately leaves
`03-Paper Boats` unchanged because there is no separating whitespace after
the punctuation. Do not replace it with an unanchored `\d+`, which can remove
meaningful numbers anywhere in a title.

### Start from filenames instead

If the real problem is that `TRACK` and `TITLE` have not been separated at
all, [Filename → Tag](../../converters/filename-to-tag.md) can capture the number
into its own field. Preserve a copied source before re-importing names.

### Undo a mistaken cleanup

Use the recorded action's **Edit → Undo …** in the current desktop session,
read its count and inspect any refusal. Completed files in a partially failed
batch remain changed until recovered. Do not repeat the group merely because
one file warned; later actions may already have run. Keep originals for
recovery after restarting.

## Related

- [Regular expressions from zero](../../scripting/regex-from-zero.md)
- [Replace with regular expression](../replace-regexp.md)
- [Track and disc numbers](../../tagging/concepts/track-and-disc-numbers.md)
- [Editing, saving and undo](../../getting-started/editing-and-saving.md)
