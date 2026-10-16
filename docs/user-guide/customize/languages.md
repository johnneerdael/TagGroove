# Languages and translations

Choose the language used by OpenTag's controls, menus and standard labels.
OpenTag includes English, Dutch and German. You can add a JSON catalog or
override a bundled translation without changing your music metadata.

## Choose a language

1. Open **File → Options… → Language**.
2. Select **English**, **Nederlands**, **Deutsch**, or an installed user catalog.
   **System default** follows the first available catalog matching a preferred
   system language. For example, `nl-BE` can use Dutch; English is the fallback.
3. Choose **OK**. The interface changes immediately; no restart is needed.

The catalog name in **System default** is shown as its author supplied it.
Field names such as `ARTIST`, format strings such as `%artist%`, and functions
such as `$upper(...)` keep the same syntax. Filenames, tag values and names you
assigned to columns, tools or action groups stay as entered. Error messages
also preserve the field identifiers and export configuration names they report.
An invalid field identifier keeps its entered spaces and characters in the
translated diagnostic so you can identify what needs correcting.

## Add a catalog in the desktop app

The folder-opening action is available only in native desktop builds.
Language selection remains available in mobile builds and browser preview.

1. In **Language**, choose **Open languages folder**. OpenTag creates the
   `languages` folder inside its active configuration folder and writes the
   current `_template.json` there.
2. Copy `_template.json` to a locale-named file, such as `fr.json` or
   `pt-BR.json`. Keep the template as a reference.
3. Edit the copy with a text editor. Set `@meta.name` to the display name and
   translate the text values. Leave the English keys unchanged.

4. Save the file, return to Options and choose **OK** to reload the folder.
5. Reopen **Language** if the new catalog was not yet listed, select its name
   and filename, then choose **OK** to apply it.

For example, these entries in the generated template can become:

```json
{
 "@meta": {"name": "Français"},
 "Save tags": "Enregistrer les tags",
 "Select {filename}": "Sélectionner {filename}"
}
```

This is an excerpt, not a complete catalog. Keep the other generated entries.
The new filename supplies the locale; the generated template has no English
locale override. Keep brace placeholders such as `{filename}` unchanged so
OpenTag can insert the original filename.

For an existing catalog, edit its file and choose Options **OK** to reload it.
OpenTag also reads the folder at startup. A file named `nl.json` or `de.json`
replaces that bundled catalog; removing it restores the bundled version on
the next reload. An `en.json` override also applies to native dialogs and tray
labels.

## Gotchas

- Missing entries or entries with mismatched placeholders fall back to
  English. Check the warning on the Language page when a file is incomplete.
  Malformed JSON files are not loaded.
- If a selected added language disappears, OpenTag falls back to English.
  The Language list identifies the unavailable preference; keeping it selected
  retains it, while choosing **English** and **OK** replaces it. **System
  default** instead follows the available system-language matches.
- A folder scan I/O failure warns and keeps catalogs already loaded in this
  session. A missing folder is an empty successful reload; an invalid individual
  catalog is reported and left out of that reload.
- Catalogs use OpenTag's JSON format; Mp3tag `.lng` files are not supported.
- System-owned controls and some operating-system, library or newly added
  backend messages can remain in their original language or English.
- Installed web sources keep their own names and authored settings prose;
  their own localization supplies that text.

To contribute a translation, keep its keys and placeholders intact and submit
it through the repository's pull-request workflow.

## Related

- [Language options reference](../reference/options/language.md)
- [Format strings](../reference/format-strings.md)

<!-- verified: src/language-options.ts; src/main.ts languagePage/applyLanguage; src/i18n/core.ts and index.ts; src-tauri/src/languages.rs. Catalog/DOM/native-wrapper tests, not final-head desktop qualification. -->
