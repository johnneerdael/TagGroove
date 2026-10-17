# Keep filenames portable

<div class="recipe-meta" markdown>
**Uses:** Tag → Filename with `$validate`, `$replace` and trimming ·
**Works on:** copied local files with reviewed track/title fields ·
**Time:** about 10 minutes for a small album · **Undo:** recorded renames in this desktop session; retain originals
</div>

## The problem

A title can contain punctuation that is useful in metadata but unsuitable
for a filename on another system. Choose a naming policy for the copy you
will transfer while preserving the title tag. This recipe replaces a defined
character set and trims trailing dots/spaces; it does not establish
that every generated path is valid on every filesystem or device.

The examples use `TRACK` = `3/8`, and show MP3 destinations. `·` marks a
significant ordinary space in the input column; type actual spaces.

<!-- portable-name-cases:start -->
| TITLE | Expected destination |
|---|---|
| `Paper: Boats?` | `03 - Paper- Boats-.mp3` |
| `Harbour/Live` | `03 - Harbour-Live.mp3` |
| `Harbour\Live` | `03 - Harbour-Live.mp3` |
| `Low Tide...··` | `03 - Low Tide.mp3` |
| `Café at Low Tide` | `03 - Café at Low Tide.mp3` |
<!-- portable-name-cases:end -->

The first row keeps two resulting hyphens: this rule changes characters,
not the title's meaning or punctuation style. Accents are retained.

## The solution

1. Work on copied files and keep the originals elsewhere. Check `TRACK`
   and `TITLE` in **Extended tags…**; use meaningful values before deriving
   names. Save or discard pending edits, then select only the intended copies.
2. Choose **Convert → Tag → Filename…**.
3. Put this in **Format string**, without adding the audio extension:

    <!-- portable-name-format:start -->
    ```text
    $num(%track%,2) - $trimright($replace($validate(%title%,-),'\',-),' .')
    ```
    <!-- portable-name-format:end -->

4. Choose **Preview**. Compare complete destination paths, not only the
   text of the title. Inspect the actual destination folder for existing
   files and compare the selected rows for repeated resulting names.
5. Apply only the reviewed small plan with **Apply to … files**. Read all
   errors and inspect the resulting filenames and unchanged title tags.

The rename applies immediately. A later **Save tags** does not confirm or
finish it. If the selection or expression changes, preview again; a stale
plan or changed source can also prevent applying it. Leave a refused plan
and inspect the cause rather than forcing a different file out of the way.

## Understand the expression in pieces

Read the nested title expression from the inside out:

| Part | What it does |
|---|---|
| `$validate(%title%,-)` | Replace each of `/ ? * " < >` and the vertical-bar and colon characters with `-` |
| `$replace(...,'\',-)` | Also replace literal backslashes with `-`; apostrophes quote that search text |
| `$trimright(...,' .')` | Remove trailing ordinary spaces and periods; the quoted argument is a set of characters |
| `$num(%track%,2)` | Read the leading track number and pad it to at least two digits |
| ` - ` | Put a visible separator between that number and the cleaned title |

`$validate` has a specific replacement set. It does not replace a backslash,
shorten names, reserve a destination, recognize device names, or decide
whether two paths refer to the same file. Its replacement may contain
several characters; each offending character receives the whole replacement.

Filename evaluation has further handling of tag-derived separators,
forbidden characters and control characters. The explicit replacements
above state the desired appearance before those rules are applied.
The [Files options](../reference/options/files.md) govern the default
replacement for separators inside tag values when you do not replace them
explicitly. That default is different from separators you deliberately
write between expression components to create folders.

## Variations

### Use a different visible replacement

Changing only the two replacement arguments from `-` to `_` makes the
policy use underscores. Change both `$validate` and the explicit backslash
replacement if the result should treat those characters consistently.
Choosing an empty replacement removes the characters and can join words:
`A/B` becomes `AB` under `$validate(%title%,)` in ordinary text evaluation.
Choose the intended result before renaming.

### Shorten a title component deliberately

For a known transfer convention, this variation limits the cleaned title
part to at most 80 Unicode scalar values and trims its resulting end:

<!-- portable-short-format:start -->
```text
$num(%track%,2) - $trimright($left($replace($validate(%title%,-),'\',-),80),' .')
```
<!-- portable-short-format:end -->

The track prefix and retained extension add more characters. `80` is an
example policy, not a filesystem limit or a guarantee about the full path.
A long parent folder still contributes to that path. Unicode scalar counts
are not encoded byte counts; combining marks can count separately from
the letters they decorate. Shortening can also erase the part that made
two titles distinct. Inspect every complete destination afterward.

### Preserve folder boundaries you intended

The recipe outputs one filename in each source file's containing folder.
If you add literal folders, keep the component-cleaning expression inside
the filename or folder component it is meant to clean. For example,
`%album%\` written before the recipe's number/title output deliberately
creates an album folder beneath each source folder. It does not gather
separate source folders into one common root automatically.

Do not apply `$validate` to an entire intended path and expect its slashes
to keep creating folders: slashes are in that function's replacement set.
Use explicit path components and a reviewed root. See
[format strings in filenames](../reference/format-strings.md#file-and-folder-names)
before extending a one-folder recipe.

## Limits to check in the real destination

| Question | What this recipe establishes |
|---|---|
| Are the listed punctuation characters replaced? | Yes, according to the expression and actual field text |
| Is every title complete and meaningful? | No; inspect missing/empty/whitespace-only metadata and truncation |
| Are generated names distinct? | No; different source text can collapse to the same cleaned result |
| Is the full path within a destination's limits? | No; folder length, encoding and receiving system rules also matter |
| Will reserved names, trailing characters and case behavior all be accepted? | No; this is a component transformation, not a platform filesystem validator |
| Will the receiving player accept the file? | No; naming is separate from format, codec, artwork and player support |

For example, `A/B` and `A:B` both clean to `A-B`. A distinguishing track
or disc can help, but it does not make every possible input unique. Keep
case sensitivity, existing destinations and actual folder roots in the
review instead of treating character replacement as collision prevention.

Use **Edit → Undo** for recorded renames in the current desktop session;
read its label and keep the originals beyond the session. Verify one copied
album on the actual receiving system before repeating the chosen policy.

## Related

- [Tag → Filename](../converters/tag-to-filename.md)
- [`$validate` reference](../reference/scripting/search-compare.md#validate)
- [Length and trimming functions](../reference/scripting/text.md)
- [Lowercase filename recipe](rename-lowercase-filenames.md)
