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
| **Open languages folder** | Available only in the native desktop app; hidden in mobile builds and browser preview. Opens the active configuration folder's `languages` directory and refreshes `_template.json`. |
| File warning | Reports files that could not be read or entries that are missing or invalid. Missing translations and entries with invalid placeholders use English. |

The language choice is saved with the application configuration. OpenTag
reads user catalogs at startup and after Options **OK**; reopen the page after
that reload to select a newly added language. If the folder cannot be read,
OpenTag reports the failure and retains catalogs already loaded in this
session. A missing folder is a successful empty reload. File warnings appear
as separate entries, with each filename and count kept together. OpenTag's
warning text follows the selected language; parser and operating-system details
keep their original wording.

Application-owned accessible labels follow the selected language too, including
cover types and states. Type labels in the cover viewer and **Extract cover**
dialog follow that language beside unchanged counters and image properties.
Authored cover descriptions and folder-image names
remain unchanged.

Status messages translate their application text, including copied/cut-tag
summaries and saved-cover notices. Paths and field identifiers within those
messages keep their original spelling. Source locations and names also keep
their spelling, including web-source dialog names announced by assistive technology; unavailable-source and empty-location notices translate. The
window title also translates generated location summaries, including desktop
startup and forwarded launches, when directory display is enabled, while actual paths retain their spelling. Freedb
server names stay as entered during editing; clearing a name shows the translated
**Unnamed server** label. Export duplicate-name errors also retain the name
entered while translating the surrounding message. File-list cell editors announce the same column name shown in the header: default names follow the UI language, while custom names keep their spelling. Selection-preset names also keep their spelling in the removal dialog. Tool-name edits keep authored names in both tool selectors; an empty draft name shows a translated numbered fallback. Toolbar hints keep the tool or action-group name even when its toolbar item has a different display name. Keeping a mixed field reports its technical key unchanged while translating the status. Mapping validation translates its numbered row label while keeping field identifiers unchanged. Favorite and included-directory lists retain saved paths, including relative names.

## Catalog files

| Entry | Meaning |
|---|---|
| `<locale>.json` | Adds a catalog, for example `fr.json` or `pt-BR.json`. The generated template derives its locale from the copied filename. |
| `@meta.name` | The catalog's display name. Its spelling is preserved in both the user option and System default, even if it equals an English UI label. |
| `@meta.locale` | Optional explicit locale override, supported for existing catalogs. The generated template omits it. |
| English text key | Identifies an application string. Translate its value, preserving brace-placeholder names. |
| `@keys` | Optional keyboard-key labels, such as German `Ctrl` → `Strg`. Keep the key names and change their displayed values. |
| `nl.json`, `de.json`, `en.json` | Replaces the corresponding bundled catalog. Removing an override restores the bundled catalog on reload. |
| `_template.json` | The current English source entries. Files beginning with `_` are not offered as languages. |

Files must be JSON objects, no larger than 4 MiB. Missing translations and
invalid placeholder entries use English. Removing a selected added catalog
falls back to English; removing an override restores its bundled language.
An explicit regional choice uses only that locale: removing `pt-BR.json` does
not select `pt.json` or `pt-PT.json`. Regional matching applies to **System
default**. The unavailable saved choice is shown as **Unavailable: pt-BR
(using English)**, for example. Leave it selected to retain that preference, or select
**English** and choose **OK** to replace it.

## What stays unchanged

Language selection does not translate metadata, filenames, paths, format
strings, field identifiers or user-defined names. Installed web sources retain
their authored names, search labels, result column names and settings prose.
OpenTag-owned controls and default search labels still follow
the selected application language.

Catalogs change interface text but keep the current left-to-right layout.
A right-to-left catalog, such as Arabic or Hebrew, does not switch the document
direction or mirror controls, menus and navigation.

Some operating-system controls, library messages and backend text added after
the catalog snapshot remain outside the catalog. A translated UI is not a
claim of identical wording or runtime behavior to another tagging program.

## Related

- [Languages and translations](../../customize/languages.md)
- [Format strings](../format-strings.md)
- [Appearance](appearance.md)
- [Field names and mappings](tags-mapping.md)
- [Options overview](index.md)

<!-- Adapted from the existing options-guide draft at 312b0f0 and reconciled with current source. Verified against language-options.ts, i18n/core.ts, i18n/index.ts, main.ts and languages.rs; no final-head native screen-reader or platform layout claim. -->
