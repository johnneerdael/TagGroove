# Build your first action group

Save a three-step title cleanup and run it again from the menu. In about
15 minutes you will record the original title in a comment, replace
underscores with spaces, and normalize capitalization.

- [ ] Two copied audio files, with original copies kept elsewhere.
- [ ] First file: `TITLE` = `paper_boats (LIVE)`, with `COMMENT` absent.
- [ ] Second file: `TITLE` = `north_pier (LIVE)`, with `COMMENT` absent.
- [ ] Pending edits saved or discarded; only the first practice file selected.

The comment is a teaching aid, not a substitute for a file backup. The
first action replaces any existing comment, which is why these practice
files start without one.

## 1. Create a named group

Choose **Actions → Actions…** (++alt+6++; Alt is ⌥ Option on a Mac).
Under **Groups**, choose **New…** and name it `Practice title cleanup`.
Select that group and choose **Edit actions…**.

<!-- figure:actions-dialog -->
<figure markdown>
![The Actions dialog: a list of action groups with check boxes, buttons to create, duplicate, rename, remove and reorder them, and Save groups and Apply checked groups](../assets/screens/actions-dialog-light.png#only-light)
![The Actions dialog: a list of action groups with check boxes, buttons to create, duplicate, rename, remove and reorder them, and Save groups and Apply checked groups](../assets/screens/actions-dialog-dark.png#only-dark)
</figure>
<!-- /figure -->

The group is a saved ordered list of actions. Creating or editing the
list does not itself change the music file. Applying the group will.

## 2. Record the title before changing it

Choose **New…**, select **Format Value** in **Action type**, and enter:

| Setting | Value |
|---|---|
| **Field** | `COMMENT` |
| **Format string** | `%title%` |

Choose **Keep action**. The first action should appear in the list.

When this action runs, `%title%` reads that file's current title and
writes it to `COMMENT`. It creates the comment if missing. Nothing in
this expression edits `TITLE`.

## 3. Replace underscores

Add a second action of type **Replace**:

| Setting | Value |
|---|---|
| **Field** | `TITLE` |
| **Original string** | `_` |
| **Replace with** | One actual space |
| **Only as whole word** | Off |
| **Case-sensitive comparison** | Off |

Choose **Keep action**. This replacement changes every underscore in the
title. Its search and replacement boxes are literal text: do not type
quotes around the space.

## 4. Normalize capitalization

Add a third action of type **Case Conversion**:

| Setting | Value |
|---|---|
| **Field** | `TITLE` |
| **Case** | **Mixed Case** |
| **Words begin after** | `(-_` |

Choose **Keep action**. Mixed Case capitalizes the beginning of words
and lowercases their remaining letters. Here `paper boats (LIVE)` becomes
`Paper Boats (Live)`. It does not know which abbreviations or artist
spellings should remain uppercase.

## 5. Check the order before running

Use **↑ Move up** and **↓ Move down** until the list reads:

| Order | Action | Field | Result after this step |
|---|---|---|---|
| 1 | Format Value | `COMMENT` | `paper_boats (LIVE)` |
| 2 | Replace | `TITLE` | `paper boats (LIVE)` |
| 3 | Case Conversion | `TITLE` | `Paper Boats (Live)` |

Each action reads the results of the actions above it. Moving the comment
copy to the bottom would record the cleaned title instead of the original.
The group name does not determine execution order; the displayed list does.

Choose **Keep group**, then **Save groups**. Saving keeps the reusable
settings; it has not run them on the file.

## 6. Run only this group

In the group list, check only `Practice title cleanup`. A selected row
identifies what to edit; the checkbox identifies what will run.
Choose **Apply checked groups**.

The actions save immediately. Inspect the selected file:

| Field | Before | After |
|---|---|---|
| `TITLE` | `paper_boats (LIVE)` | `Paper Boats (Live)` |
| `COMMENT` | Missing | `paper_boats (LIVE)` |

There is no extra **Save tags** step after applying an action group.
Check the operation result; an error in a later step is not a guarantee
that earlier file changes were rolled back.

## 7. Reuse the group

After the dialog closes, select only the second practice file. Open the
**Actions** menu and choose **Practice title cleanup**. The saved group
runs again without reopening its editor.

The second title becomes `North Pier (Live)`, and its comment becomes
`north_pier (LIVE)`. The expression used the second file's title, not a
fixed title copied from the first file.

**Actions (Quick)…** (++alt+shift+6++) is the one-action route. It is useful
for a single replacement; this three-step sequence belongs in a group.
Quick actions also save when applied, and their previous settings are
remembered.

## 8. Understand repeat and undo

**Edit → Undo** reverses recorded changes in the current desktop session.
Use it immediately after the second practice run and inspect that file's
title and comment again. Another undo can reverse the first run; work
backward through the operation labels instead of assuming one undo always
restores the entire lesson.

The **Edit** menu can offer **Redo** or **Repeat**, depending on the history.
When it offers Repeat for the last action selection, it can run that
selection again. When it offers Redo, ++cmd+y++ / ++ctrl+y++ redoes the
undone operation. Read the menu label before using the shortcut.

The saved group remains available after undo. Undoing a file change does
not remove the reusable group definition.

## What you learned

- A group stores actions; applying it saves file changes.
- The list order determines what each action reads.
- A checkbox controls execution; selecting a row controls editing.
- Format Value calculates a field, Replace changes literal text, and Case
  Conversion follows a capitalization rule.
- A named menu entry reuses the group on the current selection.
- Session undo and a permanent file backup serve different recovery needs.

## Next steps

- [Move guest artists to titles](recipes/feat-artists-to-title.md)
- [Format strings](../scripting/format-strings.md)
- [Multi-value fields](../tagging/concepts/multi-value-fields.md)

For a larger collection of groups, `Cleanup#Titles` creates a **Cleanup**
submenu containing **Titles**. Prefix a mnemonic character with `&`, such
as `&Cleanup#&Titles`, to use **Actions** (++alt+a++), then `C`, then `T`.
Use `&&` when a group's displayed name needs a literal ampersand.

<!-- verified: src/action-ui.ts:groupsDialog, editAction, mnemonic; crates/tag-core/src/actions.rs:execute_groups -->
