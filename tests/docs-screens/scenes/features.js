// Docs screenshot scenes for feature pages: the tag panel with a pending edit, the filter bar,
// the column and toolbar editors and one screenshot per action type (the New action dialog with
// that type chosen). Nothing is saved or applied.
(window.OPENTAG_DOCS_SCENE_FILES = window.OPENTAG_DOCS_SCENE_FILES || []).push(h => {
  const ACTION_TYPES = [
    'adjust_cover', 'case_conversion', 'convert_codepage', 'export', 'export_cover', 'format_value',
    'generate_playlists', 'guess_values', 'import_cover', 'import_text_file', 'merge_duplicate_fields',
    'remove_duplicate_fields', 'remove_fields', 'remove_fields_except', 'replace', 'regex_replace',
    'set_cover_properties', 'split_field',
  ];
  const top = () => [...document.querySelectorAll('dialog[open]')].at(-1);

  // Settings worth showing filled in, by action type: one entry per text box in the dialog (null keeps it).
  // Kind Of Blue titles end in "(feat. …)", which is what the regular-expression example removes.
  const FILL = {
    regex_replace: [null, '\\s*\\(feat\\.[^)]*\\)$', ''],
  };
  const MILES = window.OPENTAG_DOCS_MUSIC ? h.music('albums', /Kind Of Blue/) : undefined;

  const actionType = async value => {
    await h.selectAll();
    const groups = await h.openDialog('groups', '#apply-groups');
    groups.querySelector('[data-group-row] span').click(); await h.sleep(300);
    groups.querySelector('[data-command="edit"]').click(); await h.sleep(1000);
    top().querySelector('[data-command="new"]').click(); await h.sleep(1000);
    const type = await h.waitFor(() => top()?.querySelector('#full-action-type'), 5000, 'action type list');
    h.input(type, value); await h.sleep(900);
    const boxes = [...top().querySelectorAll('input:not([type="checkbox"]):not([type="radio"]), textarea')];
    (FILL[value] || []).forEach((text, i) => { if (text !== null && boxes[i]) h.input(boxes[i], text); });
    await h.sleep(500);
  };

  return [
    {
      name: 'tag-panel-pending', run: async () => {
        await h.clickRow(2);
        const album = await h.waitFor(() => document.querySelector('.fields [data-field="ALBUM"]'), 5000, 'Album field');
        h.input(album, 'Automatic For The People (Demo)');
        await h.waitFor(() => document.querySelector('.fields [data-field="ALBUM"]').value.includes('(Demo)'), 3000, 'edited');
        await h.sleep(600);
      },
    },
    {
      name: 'filter-active', run: async () => {
        const filter = document.querySelector('#filter');
        filter.focus(); h.input(filter, 'TRACK GREATER 8');
        await h.waitFor(() => h.rows().length === 4, 8000, 'filtered rows');
        await h.clickRow(0);
      },
    },
    { name: 'customize-columns', crop: 'dialog', run: async () => { await h.clickRow(0); await h.openDialog('columns', '#column-ok'); } },
    { name: 'toolbar-customize', crop: 'dialog', run: async () => { await h.openDialog('toolbar-customize', '#toolbar-arrangement'); } },
    ...ACTION_TYPES.map(value => ({ name: 'action-' + value.replace(/_/g, '-'), crop: 'dialog', album: MILES, run: () => actionType(value) })),
  ];
});
