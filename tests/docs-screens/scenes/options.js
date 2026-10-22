// Docs screenshot scenes for the Options reference pages: one scene per page
// of the Options dialog (File → Options…), cropped to the dialog. The ids are
// the page ids in src/options-ui.ts; an id the build does not have fails its
// own scene and the rest continue.
(window.OPENTAG_DOCS_SCENE_FILES = window.OPENTAG_DOCS_SCENE_FILES || []).push(h => {
  const pages = [
    'general', 'messages', 'files', 'directories', 'appearance', 'library', 'language',
    'tags', 'tags-mpeg', 'tags-ape', 'tags-mapping', 'tags-advanced', 'tag-panel',
    'field-values', 'networking', 'updates', 'tag-sources', 'tag-sources-catalog', 'tag-sources-freedb', 'tag-sources-local-freedb',
    'export', 'playlist', 'tools',
  ];
  return pages.map(id => ({
    name: 'options-' + id,
    crop: 'dialog',
    run: async () => {
      await h.clickRow(0);
      const d = await h.openDialog('options', '[data-page]');
      await h.optionsPage(d, id);
    },
  }));
});
