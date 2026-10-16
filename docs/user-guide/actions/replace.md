# Replace

Use **Replace** for literal text changes: underscores to spaces, an unwanted
word, or a consistent spelling. The search text is not a regular expression;
`.` means a dot and `*` means an asterisk.

## Make a replacement

1. Save pending edits and select a few copied files.
2. Choose **Actions → Actions (Quick)…**, then **Replace**.
3. Choose a **Field**, enter **Original string** and **Replace with**, then
   set the two comparison options.
4. Choose **Apply action** and check the changed fields.

Add the same settings to a group when the change will be repeated.

| Setting | Meaning | Initial value |
|---|---|---|
| **Field** | A field name or supported target below | `TITLE` |
| **Original string** | The literal text to find; cannot be empty | Empty |
| **Replace with** | The replacement text; leave empty to delete matches | Empty |
| **Only as whole word** | Restrict the match to whole words as defined below | Off |
| **Case-sensitive comparison** | Distinguish upper- and lowercase search text | Off |

All occurrences are replaced in each value of the chosen field.
Unmatched values stay unchanged. The replacement is literal too: `$1` is
text here, not a captured group.

## Worked examples

| Field | Original string | Replace with | Before → After |
|---|---|---|---|
| `TITLE` | `_` | One space | `Low_Tide` → `Low Tide` |
| `ALBUM` | `(Deluxe)` | Leave empty | `Harbour Lights (Deluxe)` → `Harbour Lights `, with a trailing space |
| `TITLE` | `Live` | `Concert` | `LIVE at the Pier` → `Concert at the Pier` when case sensitivity is off |
| `ARTIST` | ` & ` | ` and ` | `Kofi & the Tidewater Band` → `Kofi and the Tidewater Band` |

Enter the actual spaces around ` & ` and ` and `; do not type quotes.
For deleting a suffix *and* its preceding spaces, use the regex action
with a pattern that includes those spaces.

## Whole-word matching

**Only as whole word** accepts a match at the start or end of a value,
or next to whitespace or one of these punctuation characters:

```text
. , ; : ! ?
```

A hyphen, underscore, slash or parenthesis does not count as that boundary.
For example, a whole-word search for `Live` can change `Live!` but leaves
`Live-Version` unchanged. Use regular expressions when a different word
boundary rule is needed.

## Special targets

- `_TAG` changes every tag field.
- `_ALL` changes every tag field and the filename stem.
- `_FILENAME` changes the filename stem, keeping the audio extension.
- `_DIRECTORY` changes the containing folder name.

The replacement can therefore have wider consequences than the field list
suggests. Keep a folder backup when using a naming target, and avoid changing
a directory merely to fix tag text.

## Gotchas

- Quick-action settings are remembered. Check them before each run.
- Literal replacement does not trim the result. A deleted word can leave
  double or trailing spaces.
- Changes are saved when the action runs. Check operation errors and the
  **Edit → Undo** label rather than assuming the whole batch is atomic.

## Related

- [Replace with Regular Expression](replace-regexp.md)
- [Format Value](format-value.md)
- [Actions and action groups](index.md)
