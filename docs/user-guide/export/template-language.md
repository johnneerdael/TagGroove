---
status: draft
---

# Export template language

!!! abstract "This page is being written"

    It will cover:

    - Template commands ($filename, $loop/$loopend, headers, $puts etc. as implemented), export-only placeholders, partitions

    Until then, the [reference section](../reference/index.md) has the details.

## Verified behavior for the completed guide

`%_workingdir%` and `%_workingpath%` use the app's working folder in export
text and output-name expressions. They are distinct from each audio file's
own folder placeholders.

Without an encoding argument, an export uses ANSI: the system codepage on
Windows, and Windows-1252 on macOS/Linux. Choose explicit `utf-8` or `utf-16`
when all metadata characters must survive. Bundled configurations already
name a Unicode encoding. Template import and report output encodings remain
separate choices.
