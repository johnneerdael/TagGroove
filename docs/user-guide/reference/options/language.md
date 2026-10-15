---
status: draft
---

# Language options

This completed reference is held until the language implementation is integrated. The guide baseline does not yet register a Language options page.

Open **File → Options… → Language** to choose the language used by OpenTag's interface. The language changes after **OK**; the inspected implementation does not require restarting the app.

!!! tip "New to this?"
    Field names, placeholders, and functions keep their spelling in every language. Continue to type `%artist%` and `$upper(...)` when using a translated interface.

## Choose a language

**System default** is the default setting. It selects the language matching the system from the available catalogs, with English fallback. The inspected source ships English, Dutch, and German. Each entry uses its own language name, and an additional user catalog is identified by its filename.

Changing language translates the app's own controls and standard labels. Names deliberately entered for columns, tag-panel fields, tools, or action groups remain as entered. It does not translate the audio metadata, rename files, or change scripting syntax.

## Add or update a translation

Choose **Open languages folder** in the desktop app. The folder is inside the active configuration directory. Copy `_template.json` to a locale-named JSON file such as `fr.json` or `pt-BR.json`, set the catalog's language name, and translate the text values. Preserve the catalog keys and brace placeholders: the app needs those to locate text and insert values.

A catalog called `nl.json` or `de.json` replaces the shipped translation of that locale. OpenTag reads the folder at startup and again after Options **OK**. After adding or changing a file, choose the language and press **OK** to reload it. Invalid or incomplete catalogs produce warnings; missing text falls back to English.

These JSON catalogs are OpenTag's format. Mp3tag language resources and its translation-submission instructions are not an import format for this folder.

## Related

- [Appearance](appearance.md)
- [Field names and mappings](tags-mapping.md)
- [Options overview](index.md)

<!-- verified: src/language-options.ts LANGUAGE_DEFAULTS, languageOptionsPage, languageChoices; src/main.ts applyLanguage; src/i18n catalogs and fallback. Source inspection, not runtime language QA. -->
