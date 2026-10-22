# Split an Artist - Title credit stored in TITLE

Extract an artist and track title when `TITLE` contains a predictable credit
such as `Juniper Vale - Paper Boats`. Use Guess Values for the extraction,
then trim the two captured fields. This changes metadata; it does not rename
the audio file.

## Prepare a small practice selection

Use copied files with one `TITLE` value per file and at most one existing
`ARTIST` value. Inspect the existing artist: extraction will replace it when the source matches. The separator
in this recipe is one space, a hyphen, and one space: ` - `.

For the expected results below, `·` marks an ordinary space inside quoted
values. It is a display marker, not text to type.

<!-- credit-cases:start -->
| TITLE before | ARTIST before | ARTIST after | TITLE after |
|---|---|---|---|
| `Juniper Vale - Paper Boats` | `Old credit` | `Juniper Vale` | `Paper Boats` |
| `··AC/DC - North Pier··` | `Old credit` | `AC/DC` | `North Pier` |
| `Paper Boats` | `Juniper Vale` | `Juniper Vale` | `Paper Boats` |
| `··Paper Boats··` | `··Juniper Vale··` | `Juniper Vale` | `Paper Boats` |
| `Kofi - Tide - Quiet Harbour` | `Old credit` | `Kofi` | `Tide - Quiet Harbour` |
<!-- credit-cases:end -->

The final row shows the ambiguity: the parser uses the first delimiter that
permits a full match, not knowledge of artist names. If `Kofi - Tide` is the
intended artist, this pattern is wrong for that file. Choose another source,
edit that file manually, or use a separately tested pattern for that layout.

## Run extraction on one copy

1. Save or discard pending edits and select the first practice file.
2. Open **Actions → Actions (Quick)…** and choose **Guess Values**.
3. Enter `%title%` in **Source format string**.
4. Enter `%artist% - %title%` in **Guessing pattern**.
5. Choose **Apply action** and inspect both fields in **Extended tags…**.

The source expression reads the old title before either destination changes.
The `%artist%` and `%title%` tokens in the guessing pattern name destinations;
they do not fetch existing values. The expected first result is
`ARTIST=Juniper Vale`, `TITLE=Paper Boats`.

This applies immediately. There is no full Guess-Values dry-run preview in
this recipe. Test a copied file and compare results before expanding the
selection. Use **Edit → Undo …** for a recorded write when needed.

## Add trimming in a reusable group

Once the source layout is confirmed, create an **Actions…** group named
`Practice split title credit`. Add the following rows in **Edit actions…**:

<!-- credit-actions:start -->
| Step | Action | Field / source | Pattern / format / separator | Replacement / option |
|---|---|---|---|---|
| 1 | Guess Values | `%title%` | `%artist% - %title%` | Not used |
| 2 | Format Value | `ARTIST` | `$trim(%artist%)` | Not used |
| 3 | Format Value | `TITLE` | `$trim(%title%)` | Not used |
<!-- credit-actions:end -->

For row 1, the third column is **Source format string** and the fourth is
**Guessing pattern**. For the Format Value rows they are **Field** and
**Format string**. Keep the actions and group, **Save groups**, then apply
only the checked practice group to the intended copies.

Captures preserve their surrounding spaces, so trimming happens after the
split. Trimming first would affect only the outside of the combined source;
it would not necessarily clean each captured artist/title independently.
A source using a different separator, such as `Artist – Title` with an en
dash, does not match this literal hyphen pattern.

The third and fourth example rows are important: Guess Values makes no field
changes when the source does not fully match, but the later Format Value
rows still run. Thus the complete group can trim fields on a nonmatching
file. Restrict the selection to the intended layout if those extra trims
are unwanted. A warning from another action also does not stop later actions
in the group; inspect every reported file result.

## Preview a trim without writing it

For a single stored title or artist, **Convert → Tag → Tag…** can preview
`$trim(%title%)` or `$trim(%artist%)` with the corresponding destination.
Wait for the result, then Cancel if this is only an experiment. This previews
the individual expression on the current tags, not the whole ordered group
or the tags as they would look after a future Guess Values row.

A bare placeholder reads only the first value. Do not run this group on
files with multiple title or artist values when the other values must be
preserved, even if the Guess Values row would not match.
After extraction, use [Tag → Filename](../../converters/tag-to-filename.md) as a
separate reviewed rename if filenames should also change.

## Common mistakes

- Do not paste a regex such as `(.+) - (.+)` into Guessing pattern. Guess
  Values uses named placeholders and literal separators, not capture groups.
- Omitting the separator between `%artist%` and `%title%` is ambiguous and
  rejected. The pattern needs text that defines where one capture ends.
- The first action overwrites a matching file's artist. Keep a file backup
  when the previous credit matters; an extra metadata field is not a complete
  backup of the original file.
- Actions write immediately; **Save groups** only stores the recipe. Check
  the selected files and undo label rather than pressing Save tags to “apply”
  the group afterward.

## Related

- [Guess Values](../guess-values.md)
- [Format Value](../format-value.md)
- [Remove extra spaces](trim-spaces.md)
- [Build your first action group](../first-action-group.md)

<!-- verified: crates/tag-core/src/actions.rs:GuessValues, try_guess_values, FormatValue; crates/tag-core/src/script.rs:trim. Exact written sequence executes in order on matches, nonmatches and ambiguous delimiter cases under LF/CRLF. No native UI claim. -->
