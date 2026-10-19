# Customize TagGroove

Arrange TagGroove around the information you use most. A column can make
missing album artists visible, a tag-panel field can make Mood convenient to
edit, and a toolbar can keep repeated commands close at hand.

Start with one change and check its result before changing the whole
workspace. Saved layouts and options affect future work; some controls,
such as tag-panel defaults or external tools, can also affect what happens
to selected files.

## Choose the right customization

| What you want | Change this | Start here |
| --- | --- | --- |
| See another field or calculated value in the file list | Columns: display expression, editable field, sorting, width and visibility | [Columns](columns.md) |
| Edit a field frequently without opening Extended tags | Tag-panel layout: fields, control types, size, order and defaults | [Tag panel layout](tag-panel.md) |
| Keep commands, action groups or tools within reach | Main/custom toolbar: order, labels and visibility | [Toolbars](toolbars.md) |
| Reuse consistent spellings for genres, moods or other fields | Suggested field values | [Field values](field-values.md) |
| Open selected files in another program | External tool definitions and parameters | [External tools](tools.md) |
| Keep a named checkpoint or move supported saved settings | Configuration bundle | [Save and restore configuration](configuration.md) |
| Use another interface language or work on a translation | Language selection and translation resources | [Languages and translations](languages.md) |
| Change scaling, grid appearance, defaults or format policies | Options pages | [Options reference](../reference/options/index.md) |

## Understand display, edit target and stored value

A displayed label is not a field name. A calculated display is not necessarily
a saved value. These distinctions prevent a workspace change from becoming
an unexpected metadata change.

| Example | What it does |
| --- | --- |
| Rename a column header to `Credits` | Change the visible label; review the column's **Value** and **Field** to see what it displays and edits. |
| Show `$upper(%title%)` in a read-only column | Display an uppercase title without rewriting `TITLE`. |
| Add `MOOD` to the tag panel with default `< keep >` | Make Mood available for deliberate editing without using a replacement default. |
| Hide the cover section | Hide that part of the panel; retain embedded pictures. |
| Add a source command to a toolbar | Provide another route to the same source workflow; it does not add an API key or bypass source limits. |

To calculate and store a field, use [Tag → Tag](../converters/tag-to-tag.md)
or [Format Value](../actions/format-value.md). Typing `%artist%` as an ordinary
tag-panel default stores literal text under the default-filling rules; it
does not calculate the artist name.

## A useful first change

Suppose you often find albums without `ALBUMARTIST`:

1. Save the current [configuration](configuration.md).
2. Follow [Columns](columns.md) to add a visible column whose **Value** is
   `%albumartist%`. Give it a recognizable **Name**, such as `Album artist`.
3. Leave **Field (empty = read-only)** empty if this is only an inspection
   column. A writable column needs its own valid field target.
4. Choose **OK**, inspect several copied files and widen the column if needed.
5. Use the [missing-fields recipe](../cookbook/find-missing-fields.md) to find
   the files needing attention. Adding the column has not filled their tags.

This gives you a visible result and one clearly defined change to undo in
the layout if it is not useful.

## Save and share the right thing

A column-layout file or tag-panel-layout file contains that layout. A whole
application configuration bundle includes supported saved components, with
a different restore scope. Neither is a backup of your music.

Finish and save configuration dialogs before taking a checkpoint. Keep
physical export templates, installed web sources and other external
resources separately. Inspect a configuration copy before sharing: paths,
tool parameters and service settings can reveal private information even
when a particular credential is excluded.

## Gotchas

- A non-keep tag-panel default can replace loaded values in a later selection.
  Read the [default-filling rules](tag-panel.md) before using one in a batch.
- Hiding a toolbar and hiding its individual buttons are separate choices.
  The [toolbar guide](toolbars.md) explains how to recover each.
- Loading a whole configuration replaces included saved components and
  reloads the window. Finish pending audio edits and save the current
  configuration first.
- External tools run another program. Review its parameters and intended
  file scope before trying it on originals.

## Related

- [Columns](columns.md)
- [Tag panel layout](tag-panel.md)
- [Toolbars](toolbars.md)
- [Save and restore configuration](configuration.md)
- [Options reference](../reference/options/index.md)

<!-- Candidate evidence: src/columns.ts and src/main.ts column customization; src/tag-panel.ts; existing customize/columns.md, tag-panel.md, toolbars.md, configuration.md. Uppercase display example already exists in the column guide; this candidate was not executed. -->
