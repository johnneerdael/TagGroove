# Clean up a messy library

Practice a repeatable cleanup on one copied album before deciding what
belongs in a larger collection. In about 20 minutes you will survey the
files, save a focused action group, check its output, stage track numbers,
and rename from the reviewed tags. Each stage has its own write boundary.

- [ ] Three copied, writable whole-track files, ordered as `01_copy.mp3`,
      `02_copy.mp3` and `03_copy.mp3`; keep the originals elsewhere.
- [ ] Each has `ARTIST` = `Juniper Vale`, `ALBUM` = `Harbour Lights`, one
      `TITLE` value from the table below, and no `ALBUMARTIST`.
- [ ] Add `PRACTICE_NOTE` = `Delete after this lesson` to these practice
      copies in Extended tags, save, and keep all other useful fields.

In the table, `·` represents an ordinary space. Type real spaces in the
practice titles, not the dot characters.

<!-- learn-cleanup-cases:start -->
| Practice file | TITLE before | TITLE after cleanup |
|---|---|---|
| `01_copy.mp3` | `··paper···BOATS··` | `Paper Boats` |
| `02_copy.mp3` | `·lanterns··ON·the·water·` | `Lanterns On The Water` |
| `03_copy.mp3` | `north··PIER` | `North Pier` |
<!-- learn-cleanup-cases:end -->

This is a particular editorial policy for invented practice text. It does
not mean every title, abbreviation, credit or custom field in your real
collection needs the same treatment.

## 1. Survey one album before changing it

Open the copied folder and inspect Filename, Title, Artist, Album and Track
in the file list. Use **View → Customize columns…** if you need to inspect or adjust
the displayed columns; [custom columns](../customize/columns.md) explains
adding fields such as album artist. A display column does not create or
repair a stored field.

Select one file and inspect **Extended tags…**. Confirm the starting values,
including the practice-only note. Close without further edits. Resolve
file-read errors and save pending edits before running actions.

## 2. Ask one useful filter question

Show the Filter with **View → Show filter** and enter:

<!-- learn-cleanup-filter:start -->
```text
ALBUMARTIST MISSING
```
<!-- learn-cleanup-filter:end -->

Wait for automatic application, or press Enter in manual mode. All three
practice files should remain visible. `MISSING` asks whether that field is
absent; an existing empty value is a different condition. The filter changes
the view and writes no tags.

Clear the filter before the next stage. Choose **Edit → Unselect all**, then
select only the three practice copies. Filtering is not a promise that
previously selected hidden rows are unselected.

## 3. Build a narrow reusable action group

Open **Actions → Actions…**, choose **New…**, name it `Practice album cleanup`,
then choose **Edit actions…**. Add these six actions in the displayed order.
Choose **Keep action** after each one.

Scroll this table sideways on a narrow screen to keep each pattern intact.

<div class="native-mapping-table" markdown="1" tabindex="0" role="region" aria-label="Practice cleanup action settings">

<!-- learn-cleanup-actions:start -->
| Step | Action | Field | Pattern / format / choice | Replacement / additional setting |
|---|---|---|---|---|
| 1 | Replace with Regular Expression | `TITLE` | `\A[ \t]+` | Leave empty; case-sensitive comparison Off |
| 2 | Replace with Regular Expression | `TITLE` | `[ \t]+\z` | Leave empty; case-sensitive comparison Off |
| 3 | Replace with Regular Expression | `TITLE` | `[ \t]+` | One actual space; case-sensitive comparison Off |
| 4 | Case Conversion | `TITLE` | Mixed Case | Words begin after `(-_` |
| 5 | Format Value | `ALBUMARTIST` | `%artist%` | No replacement box |
| 6 | Remove Fields | `PRACTICE_NOTE` | No format string | Remove this field only |
<!-- learn-cleanup-actions:end -->

</div>

The first two regexes remove spaces/tabs at the very start and end of
each title value. `\A` and `\z` mean the whole value's edges. The third
compresses remaining runs of spaces/tabs into **one real space**. Do not
type quotation marks around that replacement or leave it empty: an empty
replacement would join the words.

Mixed Case capitalizes word beginnings and lowercases other letters. It
produces the practice results above, including `On The`; it does not know
your preferred short-word style, acronyms or intentional artist spelling.
The [capitalization recipe](../cookbook/fix-capitalisation.md) teaches a
separate exceptions policy.

`%artist%` reads the first artist value and sets one album-artist value.
That is suitable for this single-artist exercise. For compilations, set the
intended shared album artist deliberately; for several artist values, choose
a [list-aware policy](../cookbook/album-artist-from-artist.md). Do not assume
one expression establishes the correct album credit in every collection.

The last row removes the named practice field. It does not remove every
unknown field. Before deleting real identifiers, lyrics or comments, inspect
what they contain and which other programs need them.

## 4. Save the group, then run it deliberately

Check the action order, choose **Keep group**, then **Save groups**. The
group is now reusable, but saving its definition has not processed the audio.

Check only `Practice album cleanup` and choose **Apply checked groups**.
These actions write immediately. Selecting a group row chooses what to
edit; its checkbox chooses what runs. Read the result and every per-file
error. An error does not promise that earlier actions were rolled back, and
ordinary action warnings can allow later actions to continue.

## 5. Verify a result and the fields you kept

Inspect each file's title against the starting table. Each `ALBUMARTIST`
should now be `Juniper Vale`, and `PRACTICE_NOTE` should be absent. Verify
that `ARTIST`, `ALBUM` and useful unrelated metadata remain as intended.

Run `ALBUMARTIST MISSING` again: these successfully cleaned copies should
no longer match. Clear the filter and deliberately reselect the practice
files before continuing. A clean result for this one query does not prove
that every field in the album is correct.

## 6. Number only a known sequence

Sort by Filename so `01_copy.mp3`, `02_copy.mp3`, `03_copy.mp3` are in that
order. Confirm that this is the intended listening order; numbering cannot
discover it from the music.

Open **Convert → Auto-numbering…**. Set **Begin at track** to `1` and the
track **Minimum digits** to `2`. Leave total count, disc numbers and all
counter-change/reset options off for this one-album run.

Choose **Preview**. Expect `01`, `02`, `03`. Choose **Stage numbers**, inspect
the pending values, then **File → Save tags** to write them. Staging alone
has not saved the numbers. Some tag formats store numeric values without
leading zeros; the filename expression below adds its own padding.

## 7. Rename only after tags are correct

Save remaining pending edits, select the same three copies and choose
**Convert → Tag → Filename…**. Use:

<!-- learn-cleanup-rename:start -->
```text
$num(%track%,2) - %title%
```
<!-- learn-cleanup-rename:end -->

Preview the destinations against the staged/saved track values:

<!-- learn-cleanup-final:start -->
| Practice file | Numbering result | Rename destination |
|---|---|---|
| `01_copy.mp3` | `01` | `01 - Paper Boats.mp3` |
| `02_copy.mp3` | `02` | `02 - Lanterns On The Water.mp3` |
| `03_copy.mp3` | `03` | `03 - North Pier.mp3` |
<!-- learn-cleanup-final:end -->

Check the actual destination folders and existing files too. If a rename
is refused or the preview is wrong, inspect the cause; do not delete an
existing destination to force this lesson through.

Apply the reviewed rename. This changes the filenames immediately and
keeps their audio extensions; it is not another pending text edit.

## 8. Stop at a verified album, then reuse selectively

Compare the result with the kept originals. Check titles, credits, track
order, filenames and any player behavior you care about. Use **Edit → Undo**
by its operation label to reverse recorded changes in the current desktop
session; the cleanup, saved numbering and rename are distinct stages.
Keep originals for recovery after quitting.

For another album, survey first. Reuse the saved group only when its title,
album-credit and field-removal rules suit that album. A title already clean
may be unchanged by the spacing rules, but a newly added exception or a
different credit structure still needs a fresh trial. Avoid applying the
group blindly to a whole mixed collection.

## What you learned

- Columns and filters help ask precise questions before editing.
- A saved action group is an ordered policy, not an automatic judgement of quality.
- Actions write immediately; auto-numbering stages edits; saving writes those edits.
- Field removal and album credits need explicit, collection-specific decisions.
- Review tags before deriving filenames, and inspect each stage's errors and undo label.

## Next steps

- [Build your first action group](first-action-group.md): explore ordering and repeat behavior in smaller steps.
- [Find missing fields](../cookbook/find-missing-fields.md): distinguish absent fields, empty values and useful queries.
- [Number tracks per folder](../cookbook/number-tracks-per-folder.md): handle more than one album after reviewing boundaries.
- [Remove reviewed junk fields](../cookbook/remove-junk-fields.md): design a deliberate retention policy.
