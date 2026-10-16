# Find files below a bitrate threshold

<div class="recipe-meta" markdown>
**Uses:** Filter, `%_bitrate%`, numeric comparisons and an optional codec check.
**Works on:** Loaded entries with technical information in the installed app.
**Time:** About 5 minutes to inspect a small practice set.
**Undo:** The filter writes nothing; clear it to show all loaded files.
</div>

## The problem

You want to review low-bitrate files without treating an unknown reading as
a quality judgment. Bitrate is a rate of encoded data, not a listening test.
Different codecs, source recordings and variable-rate encoding make a single
cutoff a search aid rather than proof of quality. Use copied practice files
before doing any later conversion or replacement.

These are invented technical snapshots, not measured tracks from the example
album. Zero is included to show how an unavailable/zero reading affects the
numeric query:

<!-- low-bitrate-snapshots:start -->
| ID | `_BITRATE` in kbps | `_CODEC` |
|---|---|---|
| A | `128` | `MPEG-1 Layer III` |
| B | `192` | `MPEG-1 Layer III` |
| C | `320` | `MPEG-1 Layer III` |
| D | `1411` | `PCM` |
| E | `0` | `MPEG-1 Layer III` |
| F | `128` | `AAC` |
<!-- low-bitrate-snapshots:end -->

## The solution

1. Load the files and review any read errors. Technical values must describe
   the loaded file, not a guessed value typed into an ordinary metadata field.
2. Show the Filter with **View → Show filter** or ++f3++.
3. To find a positive measured rate below 192 kbps, enter:

```text
"%_bitrate%" GREATER 0 AND "%_bitrate%" LESS 192
```

4. Wait for automatic application, or press Enter when **Auto-apply filter**
   is disabled. A and F match the practice set. B is exactly 192, so it is
   outside this strictly lower-than threshold; E is zero and is excluded.
5. Inspect a result's codec and technical details before deciding what it
   needs. A value of 128 for AAC is not an assertion of the same audible
   result as 128 for MPEG Layer III.
6. Clear the query, then press Enter in manual mode, to restore the loaded
   list. No audio conversion or metadata save occurred.

## How it works

`%_bitrate%` supplies the loaded bitrate in kbps. Double quotes make the left
operand a complete format-string selector. `GREATER` and `LESS` compare its
leading integer with integer thresholds. `AND` requires both conditions.
Type `192`, not `192 kbps` or a decimal threshold such as `191.5`.

Scroll the table sideways on a narrow screen to read each complete query.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Bitrate filter results">

<!-- low-bitrate-queries:start -->
| Query | Matching IDs | Why |
|---|---|---|
| `"%_bitrate%" LESS 192` | `A,E,F` | Zero is also less than 192. |
| `"%_bitrate%" GREATER 0 AND "%_bitrate%" LESS 192` | `A,F` | Positive readings below the cutoff. |
| `"%_bitrate%" EQUAL 192` | `B` | Exactly the boundary. |
| `"%_bitrate%" GREATER 0 AND "%_bitrate%" LESS 193` | `A,B,F` | Includes integer 192 as well. |
| `"%_codec%" IS "MPEG-1 Layer III" AND "%_bitrate%" GREATER 0 AND "%_bitrate%" LESS 192` | `A` | The positive low-rate MPEG subset. |
<!-- low-bitrate-queries:end -->

</div>

The codec text is an exact example from OpenTag's MPEG-1 Layer III technical
naming; inspect the actual `%_codec%` of other formats/versions instead of
assuming their labels. A filename suffix alone does not prove the encoding.
If a selector supplies no leading number, it does not satisfy a numeric
comparison. An absent value and a displayed fallback zero can therefore
produce different results; investigate the file rather than inventing a rate.

## Variations

- Change the upper cutoff (`192`) when saving another search; keep
  `GREATER 0` to exclude zero readings. “Below 192” excludes 192; for whole-number
  readings, `< 193` includes it. The actual filter keywords remain `LESS` and
  `EQUAL`, not symbolic `<`.
- Use the exact codec constraint when comparing like encodings. For a mixed
  library, inspect each codec family rather than applying one quality label
  to every result.
- Search for `"%_bitrate%" EQUAL 0` separately when investigating zero readings.
  A successful query with no matches is different from a file-read or filter
  syntax error.

Filtering does not select only its visible rows. Before a later write, rename
or removal, **Edit → Unselect all** and deliberately select the desired
results. Preserve pending tag edits intentionally before reloading files.
A tag edit cannot recover information lost during lossy encoding; changing a
metadata value or transcoding to a larger bitrate does not establish restored
audio detail. The browser app preview has simpler substring search and is not
a test of these installed filter expressions.

## Related

- [Filtering the file list](../guides/filter.md)
- [Filter numeric rules](../reference/filter-syntax.md)
- [Information placeholders](../reference/placeholders.md)
- [What tags are](../concepts/what-are-tags.md)

<!-- verified: filter_recipe_guide.rs executes the six technical snapshots/five written queries and main solution under LF/CRLF, checks missing numeric input and invalid decimal threshold, and reads actual MPEG codec/bitrate information from generated frames. No listening/transcoding/native UI quality claim. -->
