---
status: draft
---

# Auto-numbering

!!! abstract "This page is being written"

    It will cover:

    - Track and disc numbering, minimum digits, totals, per-folder restarts, change-of-field triggers

    Until then, the [reference section](../reference/index.md) has the details.

## Verified behavior for the completed guide

With **Ctrl+Shift+K** (**Command+Shift+K** on macOS), numbering starts again
when the folder changes. Keep each folder's files together in the processing
order. When saving a total too, each numbering run uses its final assigned
track number. With the usual starting value 1, a 12-track folder and an
8-track folder use totals 12 and 8, rather than both using 20.

A different starting value changes that final number. Select all intended
tracks and check the preview when the total should describe a complete
disc; a subset or an additional configured reset can create a shorter run.
