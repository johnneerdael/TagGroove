// Docs screenshot scenes for the converter pages (workstream W4). A model for
// other scene files: register a factory that receives the driver's helpers
// and returns { name, run } scenes. See README.md in this folder.
(window.OPENTAG_DOCS_SCENE_FILES = window.OPENTAG_DOCS_SCENE_FILES || []).push(h => [
  {
    // Two files whose names follow "NN - Title": 02 - Lanterns On The Water
    // and 08 - Harbour Lights (rows 1 and 6 when sorted by file name).
    name: 'filename-to-tag', run: async () => {
      await h.clickRow(1);
      h.rows()[6].querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ctrlKey: true }));
      await h.waitFor(() => h.rows().filter(r => r.classList.contains('selected')).length === 2, 3000, 'two files selected');
      h.op('filename-tags');
      const d = await h.waitFor(() => h.dialog()?.querySelector('#template') && h.dialog(), 5000, 'Filename → Tag');
      h.input(d.querySelector('#template'), '%track% - %title%');
      d.querySelector('#preview-button').click();
      await h.waitFor(() => d.querySelector('#preview .preview'), 8000, 'preview');
    }
  },
]);
