---
status: draft
---

# Text File → Tag

!!! abstract "This page is being written"

    It will cover:

    - Text file or pasted text, import mask, per-file records, matching by %_filename_ext%/%_path%, encodings

    Until then, the [reference section](../reference/index.md) has the details.

## Verified behavior for the completed guide

A keyed import containing `%_filename_ext%` or `%_path%` can describe more
files than the current selection. Lines for unselected files are skipped;
selected files with no matching line keep their tags. For example, import
three selected tracks' rows from a whole-album table without deleting its
other rows first.

This flexibility belongs to keyed matching. A positional import still needs
the intended row-to-file ordering. If no text row matches a selected file,
OpenTag reports that no changes are planned. Inspect matched and unmatched
files before applying a partial plan.
