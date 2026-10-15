# Set Cover Properties

Use **Set Cover Properties** to change the role or description of embedded
pictures. The picture data and dimensions stay the same. Use **Adjust
Cover** when the actual image needs resizing or conversion.

## Label a front cover

1. Keep copies, save pending edits and select practice files whose
   embedded pictures you have inspected.
2. Choose **Actions → Actions (Quick)…**, then **Set Cover Properties**.
3. Choose **Front Cover** in **Cover type**.
4. Keep **Keep existing description** checked.
5. Set **Apply only to covers of type** to the pictures' current role,
   for example **Other**.
6. Choose **Apply action** and inspect the roles afterward.

In this example, pictures previously marked **Other** become **Front
Cover**. Back covers and other roles remain unchanged because the filter
checks each picture's original role.

## Settings

| Setting | Effect |
|---|---|
| **Cover type** | Set a new role, or keep the existing type |
| **Keep existing description** | Checked initially; retain each picture's description |
| **Cover description** | When keeping is off, replace the description with this literal text |
| **Apply only to covers of type** | Restrict changes to an original role, or select **All cover types** |

Type and description are independent. Keeping the type still allows a
description change, and keeping the description still allows a type change.

## Worked settings

| Goal | Cover type | Description setting | Apply only to |
|---|---|---|---|
| Mark unclassified art as front covers | Front Cover | Keep existing | Other |
| Clear descriptions without changing roles | Keep existing | Uncheck keep; leave description empty | All cover types |
| Describe back covers | Keep existing | Uncheck keep; enter `Album back cover` | Back Cover |

Description text is literal: `%album%` is stored as those characters,
not evaluated as a placeholder. An empty replacement clears the
description; leaving **Keep existing description** checked preserves it.

## Gotchas

- **All cover types** can turn a front cover, back cover and booklet into
  the same role. Use the role filter when only one kind should change.
- Files without embedded artwork gain no picture from this action.
- Role and description support varies by tag format and player. Inspect
  the saved file as well as its display in the target player.
- Changes save when the action runs. Use practice copies and inspect
  **Edit → Undo** for recorded file changes.

<!-- verified: crates/tag-core/src/cover_ops.rs:set_cover_properties_with_options; src/action-ui.ts:controls -->

## Related

- [Import Cover from File](import-cover.md)
- [Export Cover to File](export-cover.md)
- [Adjust Cover](adjust-cover.md)
- [Actions and action groups](index.md)
