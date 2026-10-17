// Native e2e for playlists, cue sheets and export options: Options → Playlist
// and Options → Export, File → Playlist (all files) with the confirm filename
// dialog, Generate Playlists with preview, File → Export without the selection
// dialog and per directory (Shift), and loading playlists and cue sheets
// through a real second command-line launch (`/fn:<playlist>`), which the
// single-instance handler forwards to this window.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const names = () => rows().map(r => r.dataset.path.split(/[\\/]/).pop());
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); document.querySelectorAll('dialog[open]').forEach(d => d.close()); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const options = async (page, edit) => {
    document.body.focus(); key('o', { metaKey: true });
    const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options dialog');
    dlg.querySelector(`[data-page="${page}"]`).click(); await sleep(150);
    await edit(dlg);
    dlg.querySelector('#options-ok').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
  };
  const setText = (dlg, path, value) => { const el = dlg.querySelector(`[data-option="${path}"]`); el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); };
  const setCheck = (dlg, path, value) => { const el = dlg.querySelector(`[data-option="${path}"]`); if (el.checked !== value) el.click(); };
  // A second launch of this binary with Mp3tag's /fn: switch, started as a Tool.
  const relaunch = async (file, any) => {
    const binary = window.OPENTAG_E2E_BINARY;
    if (typeof binary !== 'string' || !binary) throw new Error('Native runner did not supply its executable');
    await inv('run_tool', { tool: { name: 'Relaunch', path: binary, parameters: `"/fn:${file}"`, for_each_file: false }, paths: [any], root: null });
  };

  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    const album = rows()[0].dataset.path.replace(/[\\/][^\\/]+$/, '');
    const anyFile = rows()[0].dataset.path;

    await step('Options → Playlist and Options → Export pages save their values', async () => {
      await options('playlist', async dlg => {
        setText(dlg, 'pages.playlist.filename', 'list.m3u8');
        setText(dlg, 'pages.playlist.extinf_format', '%title% by %artist%');
        setCheck(dlg, 'pages.playlist.confirm_filename', true);
      });
      await options('export', async dlg => setCheck(dlg, 'pages.export.show_dialog', false));
      const pages = (await inv('load_settings')).settings.pages;
      if (pages.playlist?.filename !== 'list.m3u8' || pages.export?.show_dialog !== false) throw new Error(JSON.stringify(pages));
      return JSON.stringify({ playlist: pages.playlist.filename, export: pages.export });
    });

    await step('Playlist (all files) asks for the evaluated name and writes list order with extended info', async () => {
      document.querySelector('[data-sort="filename"]').click(); await sleep(300); // descending
      await waitFor(() => names()[0] === 'sine-b.mp3', 3000, 'descending order');
      document.body.focus(); key('p', { metaKey: true });
      const dlg = await waitFor(() => document.querySelector('dialog[open].playlist-dialog'), 5000, 'confirm filename dialog');
      const input = dlg.querySelector('#playlist-filename');
      if (!/[\\/]album[\\/]list\.m3u8$/.test(input.value)) throw new Error('proposed ' + input.value);
      dlg.querySelector('#playlist-ok').click();
      await waitFor(() => /Created playlist list\.m3u8 with 2 files/.test(status()), 8000, 'created status (' + status() + ')');
      const read = await inv('read_playlist', { path: album + '/list.m3u8' });
      if (read.files.map(p => p.split(/[\\/]/).pop()).join() !== 'sine-b.mp3,sine-a.mp3') throw new Error(JSON.stringify(read));
      const text = await inv('import_export_configuration', { path: album + '/list.m3u8', legacyEncoding: null });
      if (!/#EXTINF:3,Sine B by Test Artist\r\nsine-b\.mp3\r\n/.test(text.template)) throw new Error(JSON.stringify(text.template));
      return text.template.replace(/\r\n/g, ' | ');
    });

    await step('a command-line launch with a playlist loads it in playlist order and clears the sort', async () => {
      document.querySelector('[data-sort="filename"]').click(); await sleep(300); // ascending again
      await waitFor(() => names()[0] === 'sine-a.mp3', 3000, 'ascending order');
      await relaunch(album + '/list.m3u8', anyFile);
      await waitFor(() => /list\.m3u8$/.test(document.querySelector('.location .path')?.textContent || ''), 15000, 'playlist location');
      await idle();
      if (names().join() !== 'sine-b.mp3,sine-a.mp3') throw new Error('order ' + names().join());
      if (/[↑↓]/.test(document.querySelector('thead').textContent)) throw new Error('sort indicator still shown');
      return names().join(' → ');
    });

    await step('View → Unsorted returns to playlist order after a column sort', async () => {
      document.querySelector('[data-sort="filename"]').click(); await sleep(300);
      await waitFor(() => names()[0] === 'sine-a.mp3', 3000, 'sorted');
      const unsorted = document.querySelector('button[data-op="list-unsorted"]');
      if (!unsorted || unsorted.disabled) throw new Error('Unsorted unavailable');
      unsorted.click();
      await waitFor(() => names().join() === 'sine-b.mp3,sine-a.mp3', 3000, 'playlist order');
      if (!document.querySelector('button[data-op="list-unsorted"]').disabled) throw new Error('still enabled');
      return names().join(' → ');
    });

    await step('Generate Playlists previews and writes one playlist per partition', async () => {
      document.body.focus(); key('a', { metaKey: true }); await sleep(200);
      // ⌘/Ctrl+Shift+P is Playlist per folder (keyboard reference); the generator is opened from the File menu.
      document.querySelector('button[data-op="playlist-generate"]').click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].generate-playlists-dialog'), 5000, 'generator dialog');
      const set = (id, value) => { const el = dlg.querySelector(id); el.value = value; el.dispatchEvent(new Event('input', { bubbles: true })); };
      set('#generate-filename', '%_folderpath%gen-%title%.pls'); set('#generate-partition', '%title%');
      await waitFor(() => /2 playlists/.test(dlg.querySelector('#generate-preview').textContent), 5000, 'preview');
      const preview = dlg.querySelector('#generate-preview').textContent;
      dlg.querySelector('#generate-ok').click();
      await waitFor(() => /Created 2 playlists/.test(status()), 8000, 'generated status (' + status() + ')');
      const read = await inv('read_playlist', { path: album + '/gen-Sine A.pls' }).catch(e => { throw new Error(`${e}; preview: ${preview}; status: ${status()}`); });
      if (read.files.length !== 1 || !read.files[0].endsWith('sine-a.mp3')) throw new Error(JSON.stringify(read));
      return preview.replace(/\s+/g, ' ').slice(0, 120);
    });

    await step('File → Export without the selection dialog exports at once', async () => {
      await idle(); document.body.focus(); key('e', { metaKey: true });
      await waitFor(() => /Exported metadata for 2 files/.test(status()), 8000, 'export status (' + status() + ')');
      if (document.querySelector('dialog[open].export-dialog')) throw new Error('selection dialog shown');
      const text = (await inv('import_export_configuration', { path: album + '/export.txt', legacyEncoding: null })).template;
      if (!/Test Artist - Sine B\r\nTest Artist - Sine A/.test(text)) throw new Error(JSON.stringify(text));
      return JSON.stringify(text);
    });

    await step('Shift exports a cue sheet per directory that loads back by command line', async () => {
      const library = await inv('load_export_library');
      library.preferences.configuration = 'cue-sheet';
      await inv('save_export_library', { library });
      document.body.focus(); key('e', { metaKey: true, shiftKey: true });
      const cue = await waitFor(() => inv('read_playlist', { path: album + '/export.cue' }), 8000, 'cue sheet (' + status() + ')');
      // As in Mp3tag, a cue sheet lists its tracks (rows of the sheet), each playing its FILE.
      if (cue.files.map(p => p.split(/[\\/]/).pop()).join() !== 'export.cue\u00011,export.cue\u00012' || cue.errors.length) throw new Error(JSON.stringify(cue));
      document.querySelector('[data-sort="filename"]').click(); await sleep(300);
      await relaunch(album + '/export.cue', anyFile);
      await waitFor(() => /export\.cue$/.test(document.querySelector('.location .path')?.textContent || ''), 15000, 'cue location');
      await idle();
      // The track rows show their files' names, in the sheet's order.
      const shown = rows().map(r => r.querySelector('td[data-col="filename"] > span[translate="no"]')?.getAttribute('title'));
      if (shown.join() !== 'sine-b.mp3,sine-a.mp3' || names().join() !== 'export.cue\u00011,export.cue\u00012') throw new Error('order ' + shown.join() + ' / ' + names().join());
      return shown.join(' → ');
    });

    await step('missing playlist entries are reported while the others load', async () => {
      const library = await inv('load_export_library');
      library.configurations.push({ id: 'broken', name: 'Broken playlist', template: '$filename(m3u8,utf-8)#EXTM3U$char(13)$char(10)gone.mp3$char(13)$char(10)$loop(1)%_filename_rel%$char(13)$char(10)$loopend()' });
      library.preferences.configuration = 'broken';
      await inv('save_export_library', { library });
      document.body.focus(); key('a', { metaKey: true }); await sleep(200);
      key('e', { metaKey: true });
      const state = () => `${status()}; busy ${!!document.querySelector('footer.status.busy')}; selected ${document.querySelectorAll('tr.selected').length}; focus ${document.activeElement?.tagName}; dialog ${document.querySelector('dialog[open]')?.className || 'none'}; errors ${document.querySelector('section.errors')?.textContent?.slice(0, 300) || 'none'}`;
      const early = state();
      await waitFor(() => inv('read_playlist', { path: album + '/export.m3u8' }), 8000, 'export').catch(e => { throw new Error(`${e.message}; after the keys: ${early}; now: ${state()}`); });
      await relaunch(album + '/export.m3u8', anyFile);
      await waitFor(() => /export\.m3u8$/.test(document.querySelector('.location .path')?.textContent || ''), 15000, 'playlist location');
      await idle();
      const errors = document.querySelector('section.errors')?.textContent || '';
      if (rows().length !== 2 || !/Line 2: gone\.mp3: file not found/.test(errors)) throw new Error(`${rows().length} rows; errors: ${errors}; status: ${status()}`);
      return `${status()} / ${errors.replace(/\s+/g, ' ').slice(0, 100)}`;
    });

    await step('Create playlist automatically writes the playlist after saving tags', async () => {
      await options('playlist', async dlg => {
        setCheck(dlg, 'pages.playlist.auto_create', true);
        setText(dlg, 'pages.playlist.filename', 'auto-%_directory%.m3u8');
      });
      const first = rows()[0];
      first.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(250);
      const f = document.querySelector('.fields [data-field="TITLE"]');
      f.focus(); f.value = 'Sine B Edited'; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
      f.blur(); document.body.focus(); key('s', { metaKey: true });
      const read = await waitFor(() => inv('read_playlist', { path: album + '/auto-album.m3u8' }), 10000, 'automatic playlist (' + status() + ')');
      if (read.files.length !== 2) throw new Error(JSON.stringify(read));
      return status();
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
