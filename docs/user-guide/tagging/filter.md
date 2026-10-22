# Filtering the file list

Use the Filter to show files that need attention: tracks without an album artist, files above a bitrate, or titles containing a particular phrase. Filtering changes the view; it does not remove files or edit tags.

## Start with a text search

1. Open a folder or add files to the list.
2. Show **View → Show filter** with ++f3++ if the Filter is hidden.
3. Type `Juniper Harbour` into the Filter.

In the installed app, ordinary search requires every space-separated word to occur somewhere in the tags or filename. The words can occur in different fields. `Juniper Harbour` can therefore find `ARTIST=Juniper Vale` with `ALBUM=Harbour Lights`, regardless of case or word order.

The search does not require complete words: `harb` can match `Harbour`. An asterisk is literal text here, not a wildcard. To search only a particular field, switch to an expression.

## Ask a more precise question

| Goal | Enter |
|---|---|
| Find an artist exactly, including one value of a multi-artist field | `ARTIST IS "Juniper Vale"` |
| Find titles containing Live | `TITLE HAS Live` |
| Find missing album artists | `ALBUMARTIST MISSING` |
| Find tracks numbered above 8 | `TRACK GREATER 8` |
| Find Folk tracks except live recordings | `GENRE IS Folk AND NOT TITLE HAS Live` |
| Find a numbered track/total pair | `TRACK MATCHES "^[0-9]+/[0-9]+$"` |

Write keywords such as `IS`, `HAS`, and `AND` in uppercase. Field names and compared text are case-insensitive. Quote a multiword value as one operand; the quotes group the filter text and are not part of the wanted tag value.

Unquoted `HAS` asks about contained words; quoted `HAS` asks for a contiguous phrase. `IS` asks whether a complete stored value equals the text. `MATCHES` runs a regular expression. These are different questions: `TITLE IS "Live*"` looks for the literal asterisk, while `TITLE MATCHES "^Live"` finds a value beginning with Live.

The complete [filter syntax reference](../reference/filter-syntax.md) covers all operators, missing versus empty fields, quoting, numeric rules, and format-string selectors.

## Choose automatic or manual application

Open the helper menu beside the Filter, or focus the field and press ++ctrl+space++. Control is used on every platform; ++ctrl+alt+space++ is also available on macOS when the system reserves Control+Space.

**Auto-apply filter** defaults to enabled. Results update while typing after a short delay. Turn it off to compose an expression without filtering at each partial word, then press Enter to apply it. The text being edited and the currently applied query can differ in manual mode.

When using an input method to choose characters, automatic filtering waits for
that composition to finish. The Enter used to choose a character does not also
apply a manual filter: finish the choice, then press Enter separately.

Enabling Auto-apply while unapplied text exists applies that text. The helper also lists fields, information fields, functions, and history/favorites. Inserted placeholders still need the surrounding filter syntax: a function by itself is not an operator.

## Clear, hide, and restore the filter

Erase the text to show the original loaded list again; in manual mode, press Enter to apply the empty query. **View → Hide filter** or ++f3++ also shows all loaded files. TagGroove remembers the hidden text during the session and restores it when the Filter is shown again. If the hidden text was still being edited in manual mode, showing it restores that text as the applied filter.

A filtered-out file remains loaded. The selection count is separate from the visible count, so a hidden selected file can still be selected. Review the selected count and selection before a write, rename, or removal. Use **Unselect all** and deliberately select the intended visible rows when a batch must be limited to the result set.

## Position the Filter

Use **View → Filter position** to choose **Above the file list**, **Left of the toolbars**, or **Right of the toolbars**. The default is above the list. Compact layouts keep the Filter with the list and omit the position menu.

## If the result looks wrong

**Filtering files…** means the current query is still being evaluated. **No matching files** is a successful query with no matches. **Invalid filter** is an error: read the message, correct the query, or clear it. A syntax error does not mean that the files disappeared from disk.

Start with a simple field query and add one condition at a time. Check these common mistakes:

- Write `MATCHES`, not `MATCH`.
- Use integer thresholds such as `TRACK GREATER 8`; `3.5` and `3:30` are not valid numeric operands.
- Use `%_path%` or another explicit information field for a folder/path search. Plain search does not search every technical field.
- Distinguish a missing field from a present value containing empty text.
- Avoid double quotes inside a quoted literal. Use the regex or `$char(34)` techniques in the reference.

The browser preview has a simpler substring search over its displayed data and paths. The full expression parser described here belongs to the installed app's backend. Do not use browser-preview matches as proof of installed filter behavior.

## Related

- [Filter syntax and tested examples](../reference/filter-syntax.md)
- [Field names](../reference/field-names.md)
- [Regular expressions](../scripting/regular-expressions.md)
- [Information placeholders](../scripting/placeholders.md)

<!-- verified: baseline007a806 src/main.ts toggleFilter, requestFilter, visible, bind Filter handlers, filterAutoApply host, countText; src/panel-dock.ts FILTER_DOCKS; src/helper-menu.ts helperEntries; crates/tag-core/src/filter.rs; executable examples in crates/tag-core/tests/user_guide_filter_reference.rs. No native app session performed. -->
