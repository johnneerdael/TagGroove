---
status: draft
---

# Chapters and cue sheets as rows

!!! abstract "This page is being written"

    It will cover:

    - List chapters as separate files, editing chapter titles, cue sheet tracks, Strip chapters MP4

    Until then, the [reference section](../reference/index.md) has the details.

## MP4 Nero metadata and artwork

Chapter-row field changes use MP4 field mappings and also update or remove
matching fields already present in the file's Nero global metadata. iTunes
metadata takes precedence, with Nero filling missing values. This prevents
a deleted value from reappearing from its Nero copy after saving.

Nero artwork is shown only when iTunes artwork is absent. A cover edit writes
the resulting covers to the iTunes artwork box and removes the Nero artwork
copy; a normal text-field edit preserves that copy. Unrelated MP4 index data
is retained. These rules do not add a separate Nero artwork editing mode.
