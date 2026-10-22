# View menu

**View** changes how loaded information is displayed. Hiding a panel or a
column does not remove the corresponding metadata from audio files.

<!-- figure:menu-view -->
<figure markdown>
![The View menu open in TagGroove, listing its commands](../../assets/screens/menu-view-light.png#only-light)
![The View menu open in TagGroove, listing its commands](../../assets/screens/menu-view-dark.png#only-dark)
<figcaption>The View menu: Extended tags, tag-panel defaults and customization, field values.</figcaption>
</figure>
<!-- /figure -->

## Panels, fields and filtering

| Item | Result |
| --- | --- |
| **Refresh** | Reread loaded playlist/folder/file sources and rebuild membership after confirmation; F5 |
| **Show tag panel** / **Hide tag panel** | Toggle the editor panel; physical Control+Q |
| **Extended tags…** | Inspect and edit fields and embedded artwork beyond the visible tag panel; Alt+T |
| **Show filter** / **Hide filter** | Toggle the filter bar; F3 |
| **Default values in input fields** | Stage configured tag-panel defaults; Save tags writes those pending values |
| **Customize tag panel…** | Configure panel fields and their display/default behavior |
| **Field values…** | Manage the panel's custom-value lists |

Refresh is different from reloading only the selected tags. Resolve pending
edits before a refresh that would replace loaded information. A filtered-out
file remains loaded. Hiding the filter shows all loaded files, while remembering
its text for restoration when the filter is shown again. See [filtering](../../tagging/filter.md)
for manual application, hidden selections and invalid queries.

For example, configure a default genre, then choose **Default values in input
fields**. The value is staged in the panel; it is not an immediate rewrite of
every file. This differs from **Discard pending changes**, which removes
unsaved edits rather than applying defaults.

## Sort and columns

| Item | Result |
| --- | --- |
| **Sort by** | Choose a displayed column as the primary sort; choosing it again reverses direction |
| **Sort by → Playlist order** | Restore remembered source rank for currently listed playlist members, when that session source order is known |
| **Unsorted (load and playlist order)** | Remove active sort criteria and show the maintained load/manual order |
| **Customize columns…** | Edit column visibility, names, values and editable fields |
| **Auto-size columns** | Fit displayed widths to content; Primary+Num + |
| Column visibility choices | Show/hide individual columns without deleting fields |

A column's sort value can differ from its displayed value. Sorting a numeric
track column as text can produce a different order from a padded numeric key.
Headers and the sort indicator show the active criteria. Multi-column header
sorting and the **Sort by** primary choice serve different needs; inspect the
current arrows before using similar-file selection or generating a playlist.

To restore input order after sorting by artist, choose **Unsorted (load and
playlist order)** when the maintained manual/load order is the intended one.
To recover a known playlist's source sequence after shuffling or manual moves,
choose **Sort by → Playlist order**. These choices leave saved regular column
sorts intact. Manual Alt+arrow or Alt+drag ordering starts from the current
displayed order and leaves the sorted view. Neither sorting nor reordering
moves audio files between folders or rewrites the source playlist.

## Toolbars and positions

**Toolbars** contains **Main toolbar**, **Custom toolbar**, **Customize main
toolbar…** and **Customize custom toolbar…**. A hidden toolbar can be restored
from this menu. The custom toolbar can include configured tools and action
groups; an unavailable or removed target still needs attention before it runs.

**Tag panel position** lists Left, Right, Top, Bottom and Floating. The title
bar can also be dragged to dock it. Double-clicking the title bar toggles a
floating/docked position. Size and position are layout settings, not changes
to the tag data.

**Filter position** lists Above the file list, Left of the toolbars and Right
of the toolbars. Compact layouts omit these position menus and keep the filter
with the list. Do not interpret a missing layout menu on a narrow/mobile view
as loss of the underlying fields.

## Keyboard access

Physical Control+Tab cycles through available tag-panel, filter and file-list
panes; Control+Shift+Tab reverses that order. Hidden panes are skipped. On a
Mac these are Control chords, not Command+Tab. See
[keyboard shortcuts](../keyboard-shortcuts.md) for focus-dependent editing,
Alt/Option behavior and dialog keys.
