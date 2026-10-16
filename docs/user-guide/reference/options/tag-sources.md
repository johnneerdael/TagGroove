---
status: draft
---

# Tag sources

!!! abstract "This page is being written"

    It will cover:

    - Default cover file name, MusicBrainz cover size, web source debug output

    Until then, the [reference section](../index.md) has the details.

## Verified behavior for the completed guide

The external cover filename format shares the app's evaluation context,
including `%_workingdir%`, `%_workingpath%` and column functions. This names
the separate image file; it does not rename audio or choose an embedded
picture type. Review the written image path and any file errors.

Cover names using `$columnValues` use the active File List columns. Their
format strings are evaluated on the imported tags when the image is saved,
so a column based on `%ALBUM%` uses the newly imported album value.
