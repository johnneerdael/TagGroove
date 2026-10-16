---
status: draft
---

# Undo and history

!!! abstract "This page is being written"

    It will cover:

    - What can be undone (saved writes, renames, actions), redo/repeat, limits

    Until then, the [reference section](../reference/index.md) has the details.

## Same-size saves and preserved dates

Saving a different title with the same number of characters still creates an
undo step when the save keeps the file size and modification date unchanged.
Undo restores the previous tags; Redo reapplies the save, subject to the
normal history limits. A request that writes nothing adds no step and keeps
an existing redo step.
