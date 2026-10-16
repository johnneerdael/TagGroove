---
status: draft
---

# Filename → Filename

!!! abstract "This page is being written"

    It will cover:

    - %1–%9 capture/rearrange, examples

    Until then, the [reference section](../reference/index.md) has the details.

## Verified behavior for the completed guide

An old-name pattern that does not match one selected file leaves that file
unchanged. Review the rows marked **No match, not changed**; matching files
can still be renamed together in one undo step. Check the planned paths and
any reported file errors before assuming every rename succeeded.

The new-name format can use `%_workingdir%` and `%_workingpath%` from the app's
working folder. Slashes inside tag values use the replacement configured in
**Options → Files**, rather than an unconditional underscore.
