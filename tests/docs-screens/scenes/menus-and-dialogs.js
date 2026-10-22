// Docs screenshot scenes: every menu, and the dialogs that open straight from a
// menu command. One table drives them, so adding a screenshot is one line.
//
//   menu:   open the named menu bar menu.
//   op:     run this `data-op` command and wait for its dialog.
//   select: which rows to select first: 'one' (row 0), 'two' (rows 1 and 6), 'all' or 'none'.
//   ready:  a selector that must exist inside the dialog before capture.
//   setup:  optional async function (dialog, helpers) to bring the dialog into the shown state.
//   crop:   'dialog' captures just the dialog; omit for the whole window.
//
// Scenes must not save changes to the files (see README.md in this folder).
(window.OPENTAG_DOCS_SCENE_FILES = window.OPENTAG_DOCS_SCENE_FILES || []).push(h => {
  const select = async how => {
    if (how === 'all') await h.selectAll();
    else if (how === 'two') await h.selectRows([1, 6]);
    else if (how === 'none') { /* keep the list as loaded */ }
    else await h.clickRow(0);
  };
  const spec = [
    // ---- menus (Reference → Menus, Get started, Tagging) ----
    { name: 'menu-file', menu: 'File', select: 'one' },
    { name: 'menu-edit', menu: 'Edit', select: 'one' },
    { name: 'menu-view', menu: 'View', select: 'one' },
    { name: 'menu-actions', menu: 'Actions', select: 'one' },
    { name: 'menu-tag-sources', menu: 'Tag Sources', select: 'one' },
    { name: 'menu-tools', menu: 'Tools', select: 'one' },
    { name: 'menu-help', menu: 'Help', select: 'one' },

    // ---- Tagging ----
    { name: 'remove-tags-dialog', op: 'remove-tags', select: 'one', crop: 'dialog' },
    { name: 'manage-covers', op: 'cover-manage', select: 'all', crop: 'dialog' },
    { name: 'find-dialog', op: 'find', select: 'all', crop: 'dialog' },
    { name: 'copy-files-dialog', op: 'file-copy', select: 'two', crop: 'dialog' },
    { name: 'remove-tag-types-dialog', op: 'remove-tag-types', select: 'all', crop: 'dialog' },
    { name: 'optimize-dialog', op: 'util-optimize', select: 'all', crop: 'dialog' },
    { name: 'unsupported-fields-dialog', op: 'util-unsupported', select: 'all', crop: 'dialog' },

    // ---- Convert (Rename & convert) ----
    { name: 'filename-to-filename', op: 'filename-filename', select: 'all', ready: 'input,textarea', crop: 'dialog' },
    { name: 'text-to-tag', op: 'text-tags', select: 'all', crop: 'dialog' },
    { name: 'auto-numbering', op: 'number', select: 'all', crop: 'dialog' },
    { name: 'tag-to-tag', op: 'tag-tag', select: 'all', ready: '#template, input', crop: 'dialog',
      setup: async d => { const t = d.querySelector('#template') || d.querySelector('input[type="text"]'); if (t) h.input(t, '$upper(%title%)'); await h.sleep(800); } },

    // ---- Actions ----
    { name: 'actions-dialog', op: 'groups', select: 'all', ready: '#apply-groups', crop: 'dialog' },
    { name: 'actions-quick', op: 'action', select: 'all', crop: 'dialog' },

    // ---- Tag sources: first dialog of each provider (no network needed) ----
    { name: 'source-musicbrainz', op: 'source-musicbrainz', select: 'all', crop: 'dialog' },
    { name: 'source-discogs', op: 'source-discogs', select: 'all', crop: 'dialog' },
    { name: 'source-discogs-artist-title', op: 'source-discogs-artist-title', select: 'one', crop: 'dialog' },
    { name: 'source-beatport', op: 'source-beatport', select: 'one', crop: 'dialog' },
    { name: 'source-traxsource', op: 'source-traxsource', select: 'one', crop: 'dialog' },
    { name: 'source-itunes', op: 'source-itunes', select: 'all', crop: 'dialog' },
    { name: 'source-deezer', op: 'source-deezer', select: 'all', crop: 'dialog' },
    { name: 'source-lastfm', op: 'source-lastfm', select: 'all', crop: 'dialog' },
    { name: 'source-acoustid', op: 'source-acoustid', select: 'all', crop: 'dialog' },
    { name: 'source-freedb', op: 'source-freedb', select: 'all', crop: 'dialog' },
    { name: 'source-local-freedb', op: 'source-local-freedb', select: 'all', crop: 'dialog' },
    { name: 'source-lrclib', op: 'source-lrclib', select: 'all', crop: 'dialog' },
    { name: 'source-cover-fanart', op: 'source-cover-fanart', select: 'all', crop: 'dialog' },
    { name: 'source-api-keys', op: 'source-keys', select: 'none', crop: 'dialog' },
    { name: 'source-compare-start', op: 'source-compare', select: 'all', crop: 'dialog' },

    // ---- Export & playlists ----
    { name: 'export-dialog', op: 'export-config', select: 'all', crop: 'dialog' },
    { name: 'export-csv-dialog', op: 'csv', select: 'all', crop: 'dialog' },
    { name: 'playlist-generate-dialog', op: 'playlist-generate', select: 'all', crop: 'dialog' },

    // ---- Customize ----
    { name: 'customize-tag-panel', op: 'panel-options', select: 'one', crop: 'dialog' },
    { name: 'field-values-dialog', op: 'field-values', select: 'one', crop: 'dialog' },
    { name: 'tools-configure', op: 'tools-options', select: 'none', crop: 'dialog' },
    { name: 'favorite-directories', op: 'dir-manage', select: 'none', crop: 'dialog' },
    { name: 'save-configuration', op: 'config-save', select: 'none', crop: 'dialog' },
    { name: 'import-mp3tag-setup', op: 'profile-import', select: 'none', crop: 'dialog' },
  ];

  return spec.map(s => ({
    name: s.name,
    crop: s.crop,
    run: async () => {
      await select(s.select);
      if (s.menu) { h.openMenu(s.menu); return; }
      const d = await h.openDialog(s.op, s.ready);
      if (s.setup) await s.setup(d, h);
      await h.sleep(500);
    },
  }));
});
