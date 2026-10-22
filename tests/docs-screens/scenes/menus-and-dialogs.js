// Docs screenshot scenes: every menu, and the dialogs that open straight from a
// menu command. One table drives them, so adding a screenshot is one line.
//
//   menu:   open the named menu bar menu.
//   op:     run this `data-op` command and wait for its dialog.
//   select: which rows to select first: 'one' (row 0), 'two' (rows 1 and 6), 'all' or 'none'.
//   ready:  a selector that must exist inside the dialog before capture.
//   before: optional async function that prepares app state before the command runs.
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
  // Material that fits the provider (only meaningful in a real-music run).
  const music = (kind, re) => (window.OPENTAG_DOCS_MUSIC ? h.music(kind, re) : undefined);
  const EDM = music('songs', /Adriatique/), POP = music('songs', /Taylor Swift/), LYRICS = music('songs', /Lady Gaga/), FINGERPRINT = music('songs', /Olivia Dean/);
  const spec = [
    // ---- menus (Reference → Menus, Get started, Tagging) ----
    { name: 'menu-file', menu: 'File', select: 'one', crop: 'menu' },
    { name: 'menu-edit', menu: 'Edit', select: 'one', crop: 'menu' },
    { name: 'menu-view', menu: 'View', select: 'one', crop: 'menu' },
    { name: 'menu-actions', menu: 'Actions', select: 'one', crop: 'menu' },
    { name: 'menu-tag-sources', menu: 'Tag Sources', select: 'one', crop: 'menu' },
    { name: 'menu-tools', menu: 'Tools', select: 'one', crop: 'menu' },
    { name: 'menu-help', menu: 'Help', select: 'one', crop: 'menu' },

    // ---- Tagging ----
    // (Remove tag…, Copy files to…, Export CSV…, Save configuration… and similar open a native
    // confirmation or file picker that cannot be closed from the page, so they have no scene.
    // The macOS runner stops if a native window appears.)
    { name: 'manage-covers', op: 'cover-manage', select: 'all', crop: 'dialog' },
    { name: 'find-dialog', op: 'find', select: 'all', crop: 'dialog',
      setup: async d => {
        const box = d.querySelector('input[type="text"], input:not([type])'); if (box) h.input(box, 'Breathe');
        [...d.querySelectorAll('button')].find(b => b.textContent.trim() === 'Find')?.click();
        await h.sleep(1500);
      } },
    { name: 'remove-tag-types-dialog', op: 'remove-tag-types', select: 'all', crop: 'dialog' },
    { name: 'optimize-dialog', op: 'util-optimize', album: 'songs/Adriatique - Palm Of Your Hands (Extended Mix).flac', select: 'one', crop: 'dialog' },
    { name: 'unsupported-fields-dialog', op: 'util-unsupported', select: 'all', crop: 'dialog' },

    // ---- Convert (Rename & convert) ----
    { name: 'filename-to-filename', op: 'filename-filename', select: 'two', ready: 'input,textarea', crop: 'dialog',
      setup: async d => {
        [...d.querySelectorAll('button')].find(b => b.textContent.trim() === 'Preview')?.click();
        await h.waitFor(() => d.querySelector('.preview, #preview .preview, table'), 8000, 'preview').catch(() => {});
        await h.sleep(800);
      } },
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
    { name: 'source-beatport', op: 'source-beatport', album: EDM, select: 'one', crop: 'dialog' },
    { name: 'source-traxsource', op: 'source-traxsource', album: EDM, select: 'one', crop: 'dialog' },
    { name: 'source-itunes', op: 'source-itunes', album: POP, select: 'one', crop: 'dialog' },
    { name: 'source-deezer', op: 'source-deezer', album: POP, select: 'one', crop: 'dialog' },
    { name: 'source-acoustid', op: 'source-acoustid', album: FINGERPRINT, select: 'one', crop: 'dialog' },
    { name: 'source-freedb', op: 'source-freedb', select: 'all', crop: 'dialog' },
    { name: 'source-local-freedb', op: 'source-local-freedb', select: 'all', crop: 'dialog' },
    { name: 'source-lrclib', op: 'source-lrclib', album: LYRICS, select: 'one', crop: 'dialog' },
    { name: 'source-cover-fanart', op: 'source-cover-fanart', select: 'all', crop: 'dialog' },
    // What a new person sees: no Last.fm sign-in and no saved fanart.tv key. The owner's real
    // sign-in and key are put back by the runner's state restore when the run ends.
    { name: 'source-api-keys', op: 'source-keys', select: 'none', crop: 'dialog', before: async () => {
      await h.inv('lastfm_sign_out');
      await h.inv('save_tag_source_settings', { update: { fanart_key: '' } });
    } },
    { name: 'source-compare-start', op: 'source-compare', select: 'all', crop: 'dialog' },

    // ---- Export & playlists ----
    { name: 'export-dialog', op: 'export-config', select: 'all', crop: 'dialog' },
    { name: 'playlist-generate-dialog', op: 'playlist-generate', select: 'all', crop: 'dialog' },

    // ---- Customize ----
    { name: 'customize-tag-panel', op: 'panel-options', select: 'one', crop: 'dialog' },
    { name: 'field-values-dialog', op: 'field-values', select: 'one', crop: 'dialog' },
    { name: 'tools-configure', op: 'tools-options', select: 'none', crop: 'dialog' },
    { name: 'favorite-directories', op: 'dir-manage', select: 'none', crop: 'dialog' },
    { name: 'import-mp3tag-setup', op: 'profile-import', select: 'none', crop: 'dialog' },
  ];

  return spec.map(s => ({
    name: s.name,
    crop: s.crop,
    album: s.album,
    run: async () => {
      await select(s.select);
      if (s.before) await s.before();
      if (s.menu) { h.openMenu(s.menu); return; }
      const d = await h.openDialog(s.op, s.ready);
      if (s.setup) await s.setup(d, h);
      await h.sleep(500);
    },
  }));
});
