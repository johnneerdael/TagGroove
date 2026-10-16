# Language options

Open **File → Options… → Language** to select the language of OpenTag's
interface. Choose **OK** to apply it immediately.

!!! tip "Add a translation"
    Follow [Languages and translations](../../customize/languages.md) to edit
    the generated catalog and install it in the configuration folder.

## Controls

| Control | Behavior |
|---|---|
| **Language** | Lists **System default**, the bundled English/Dutch/German catalogs, and valid user catalogs. Each catalog uses its authored display name; user entries also show their filename. |
| **System default** | The default choice. Matches preferred system languages to available catalogs by exact locale, then base language, with English fallback. Its parentheses show the resolved catalog name. |
| **Open languages folder** | Available in the desktop app. Opens the active configuration folder's `languages` directory and refreshes `_template.json`. |
| File warning | Reports files that could not be read or entries that are missing or invalid. Missing translations and entries with invalid placeholders use English. |

The language choice is saved with the application configuration. OpenTag
reads user catalogs at startup and after Options **OK**; reopen the page after
that reload to select a newly added language.

Application-owned accessible labels follow the selected language too, including
cover types and states. Authored cover descriptions and folder-image names
remain unchanged.

Status messages translate their application text, including copied/cut-tag
summaries and saved-cover notices. Paths and field identifiers within those
messages keep their original spelling.

## Catalog files

| Entry | Meaning |
|---|---|
| `<locale>.json` | Adds a catalog, for example `fr.json` or `pt-BR.json`. The generated template derives its locale from the copied filename. |
| `@meta.name` | The catalog's display name. Its spelling is preserved even if it equals an English UI word. |
| `@meta.locale` | Optional explicit locale override, supported for existing catalogs. The generated template omits it. |
| English text key | Identifies an application string. Translate its value, preserving brace-placeholder names. |
| `@keys` | Optional keyboard-key labels, such as German `Ctrl` → `Strg`. Keep the key names and change their displayed values. |
| `nl.json`, `de.json`, `en.json` | Replaces the corresponding bundled catalog. Removing an override restores the bundled catalog on reload. |
| `_template.json` | The current English source entries. Files beginning with `_` are not offered as languages. |

Files must be JSON objects, no larger than 4 MiB. Missing translations and
invalid placeholder entries use English. Removing a selected added catalog
falls back to English; removing an override restores its bundled language.

## What stays unchanged

Language selection does not translate metadata, filenames, paths, format
strings, field identifiers or user-defined names. Installed web sources retain
their authored names and settings prose. OpenTag-owned controls still follow
the selected application language.

Some operating-system controls, library messages and backend text added after
the catalog snapshot remain outside the catalog. A translated UI is not a
claim of identical wording or runtime behavior to another tagging program.

## Related

- [Languages and translations](../../customize/languages.md)
- [Format strings](../format-strings.md)

<!-- Adapted from the existing options-guide draft at 312b0f0 and reconciled with current source. Verified against language-options.ts, i18n/core.ts, i18n/index.ts, main.ts and languages.rs; no final-head native screen-reader or platform layout claim. -->
