---
status: draft
---

# Export

!!! abstract "This page is being written"

    It will cover:

    - Export dialog: configurations, template, output name, append, one file per partition, BOM, selection dialog option, ⌘/Ctrl ⇧E per directory

    Until then, the [reference section](../reference/index.md) has the details.

## Verified behavior for the completed guide

Choosing an export configuration prefills its literal `$filename` default:
`$filename(html)` changes `export.txt` to `export.html`, while
`$filename(tracks.cue)` supplies a filename. A name typed afterward is the
explicit output override; the template's encoding still applies.

When the first argument contains placeholders, functions or quoting, the
dialog clears the name field so the renderer can evaluate the template.
For `$filename(%EXPORT_NAME%,utf-8)` with `EXPORT_NAME = report.csv`, leaving
that field empty produces `report.csv`, rather than `out.report.csv`.
Type a name after choosing the configuration if an override is intended.
Check each partition's destination before exporting.
