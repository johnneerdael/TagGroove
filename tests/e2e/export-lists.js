// Native e2e for export, playlist, Find and Manage List polish: helper lists
// moved from local storage into the configuration (lists.json), bundled export
// configurations, renaming a configuration in place (F2 and a second click),
// the open-after-export offer for one and for several written files,
// per-directory export into an absolute output folder, playlists in manual
// list order with entries relative to the working directory, Find history in
// the configuration with ► removal, Manage List favorites and Duplicate, and
// dragging the cover out of the cover window (the scripted run prepares the
// file; the native drag needs a real pointer).
// Fixtures: export-lists-fixtures.sh (disc2/sine-c.mp3 with a 16x16 cover).
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const errorsText = () => [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' | ');
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const names = () => rows().map(r => r.dataset.path.split(/[\\/]/).pop());
  const clickRow = async (name, o = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...o })); await sleep(250); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 20000, 'idle');
  const lists = async () => (await inv('load_helper_lists')).lists;
  const readText = async path => (await inv('import_export_configuration', { path, legacyEncoding: null })).template;
  const setValue = (input, value) => { input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); input.dispatchEvent(new Event('change', { bubbles: true })); };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); document.querySelectorAll('.helper-menu').forEach(m => m.remove()); document.querySelectorAll('dialog[open]').forEach(d => d.close()); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const exportDialog = async (shift = false) => { document.body.focus(); key('e', { metaKey: true, shiftKey: shift }, document.body); return waitFor(() => document.querySelector('dialog[open].export-dialog'), 5000, 'export dialog'); };
  const item = (dlg, name) => [...dlg.querySelectorAll('#export-list li[data-id]')].find(li => li.textContent === name);
  const outputDialog = () => waitFor(() => document.querySelector('dialog[open].open-output-dialog'), 8000, 'open-after-export dialog (' + status() + ' / ' + errorsText() + ')');
  const closeDialog = async dlg => { (dlg.querySelector('[data-close]') || dlg.querySelector('#export-cancel')).click(); await waitFor(() => !dlg.open, 3000, 'dialog closed'); };
  const playlist = async () => {
    document.body.focus(); key('p', { metaKey: true }, document.body);
    const dlg = await waitFor(() => document.querySelector('dialog[open].playlist-dialog'), 5000, 'confirm filename dialog');
    const target = dlg.querySelector('#playlist-filename').value;
    dlg.querySelector('#playlist-ok').click();
    const offer = await outputDialog();
    if (offer.querySelector('header').textContent !== 'Playlist') throw new Error('offer title ' + offer.querySelector('header').textContent);
    await closeDialog(offer);
    return target;
  };
  const legacy = { 'opentag.history.tag-filename': JSON.stringify([{ value: '%track% - %title%', name: 'Numbered' }, { value: '%title%' }]) };

  try {
    if (!sessionStorage.getItem('export-lists-stage')) {
      // Stage 1: lists as earlier versions stored them, then a restart into the same folder.
      await waitFor(() => rows().length >= 3, 15000, 'fixture rows');
      const album = row('sine-a.mp3').dataset.path.replace(/[\\/][^\\/]+$/, '');
      const settings = (await inv('load_settings')).settings;
      settings.pages = { ...settings.pages, messages: { ...(settings.pages?.messages || {}), confirm_export: true }, 'directories-startup': { startup: 'previous' } };
      await inv('save_settings', { settings });
      await inv('remember_directories', { paths: [album] });
      for (const [k, v] of Object.entries(legacy)) localStorage.setItem(k, v);
      sessionStorage.setItem('export-lists-stage', '2');
      location.reload();
      return;
    }
    await waitFor(() => rows().length >= 3, 15000, 'fixture rows');
    await idle();
    const album = row('sine-a.mp3').dataset.path.replace(/[\\/][^\\/]+$/, '');
    const work = album.replace(/[\\/][^\\/]+$/, '');
    // The app writes and offers paths with the platform's separator.
    const sep = album.includes('\\') ? '\\' : '/';

    await step('helper lists in local storage move into the configuration (lists.json)', async () => {
      const saved = await waitFor(async () => { const l = await inv('load_helper_lists'); return l.exists && l; }, 5000, 'lists.json');
      const moved = saved.lists['opentag.history.tag-filename'];
      const canonical = list => JSON.stringify(list.map(entry => Object.fromEntries(Object.entries(entry).sort())));
      if (!moved || canonical(moved) !== canonical(JSON.parse(legacy['opentag.history.tag-filename']))) throw new Error(JSON.stringify(saved.lists));
      if (localStorage.getItem('opentag.history.tag-filename') !== null) throw new Error('local storage copy left behind');
      return Object.keys(saved.lists).join(', ');
    });

    await step('the Export dialog lists the bundled configurations and renames in place (F2, second click)', async () => {
      const dlg = await exportDialog();
      const listed = [...dlg.querySelectorAll('#export-list li')].map(li => li.textContent);
      for (const name of ['Text', 'Cue sheet', 'CSV (spreadsheet)', 'HTML table', 'Extended M3U playlist', 'JSON', 'Album list']) if (!listed.includes(name)) throw new Error('missing ' + name + ' in ' + listed.join('|'));
      if (dlg.querySelector('#export-name')) throw new Error('separate name field still shown');
      item(dlg, 'HTML table').click(); await sleep(100);
      if (item(dlg, 'HTML table').getAttribute('aria-selected') !== 'true') throw new Error('not selected');
      key('F2', {}, dlg.querySelector('#export-list'));
      let input = await waitFor(() => dlg.querySelector('#export-list .export-rename'), 2000, 'rename field');
      if (input.value !== 'HTML table') throw new Error('rename field value ' + input.value);
      input.value = 'Track table'; key('Enter', {}, input);
      await waitFor(() => item(dlg, 'Track table') && !dlg.querySelector('.export-rename'), 2000, 'renamed item');
      // A second, separate click on the selected name starts renaming; Escape keeps the name.
      item(dlg, 'Track table').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, detail: 1 }));
      input = await waitFor(() => dlg.querySelector('#export-list .export-rename'), 2000, 'rename after a second click');
      input.value = 'Discarded'; key('Escape', {}, input);
      await waitFor(() => item(dlg, 'Track table') && !dlg.querySelector('.export-rename'), 2000, 'cancelled rename');
      if (!dlg.open) throw new Error('Escape closed the dialog');
      // Names stay unique.
      key('F2', {}, dlg.querySelector('#export-list'));
      input = await waitFor(() => dlg.querySelector('#export-list .export-rename'), 2000, 'rename field');
      input.value = 'json'; key('Enter', {}, input);
      await waitFor(() => /already exists/.test(dlg.querySelector('.dialog-error')?.textContent || ''), 2000, 'duplicate name error');
      dlg.querySelector('#export-save').click();
      await waitFor(() => /saved/.test(dlg.querySelector('#export-status').textContent), 5000, 'saved');
      const library = await inv('load_export_library');
      const renamed = library.configurations.find(c => c.id === 'html-table');
      if (renamed?.name !== 'Track table') throw new Error(JSON.stringify(library.configurations.map(c => [c.id, c.name])));
      await closeDialog(dlg);
      return listed.join(' | ');
    });

    await step('the bundled JSON configuration writes valid JSON and offers to open the file', async () => {
      document.body.focus(); key('a', { metaKey: true }, document.body); await sleep(200);
      const dlg = await exportDialog();
      item(dlg, 'JSON').click(); await sleep(100);
      setValue(dlg.querySelector('#export-filename'), `${work}/exports/all.json`);
      dlg.querySelector('#export-run').click();
      const offer = await outputDialog();
      const text = offer.querySelector('.dialog-content').textContent;
      if (!/Open all\.json now\?/.test(text) || offer.querySelector('#open-output-file')) throw new Error('offer: ' + text);
      if (!offer.querySelector('#open-output-reveal')) throw new Error('no Show in file manager button');
      await closeDialog(offer);
      const json = JSON.parse(await readText(`${work}/exports/all.json`));
      const titles = json.map(e => e.title).sort().join(',');
      if (titles !== 'Sine A,Sine B,Sine C' || !json.every(e => typeof e.seconds === 'number' && e.path.startsWith(album))) throw new Error(JSON.stringify(json));
      return titles;
    });

    await step('per-directory export keeps an absolute output folder and offers the written files', async () => {
      const dlg = await exportDialog(true);
      item(dlg, 'Text').click(); await sleep(100);
      setValue(dlg.querySelector('#export-filename'), `${work}/exports/list.txt`);
      if (!/One export file is created in each folder/.test(dlg.querySelector('#export-reason').textContent)) throw new Error('not per directory');
      dlg.querySelector('#export-run').click();
      const offer = await outputDialog();
      const options = [...offer.querySelectorAll('#open-output-file option')].map(o => o.title);
      const expected = [`${work}${sep}exports${sep}list.txt`, `${work}${sep}exports${sep}disc2${sep}list.txt`];
      if (options.length !== 2 || !expected.every(path => options.includes(path))) throw new Error('offered ' + options.join(' | '));
      if (!/Written files \(2\)/.test(offer.textContent)) throw new Error(offer.textContent);
      await closeDialog(offer);
      const top = await readText(expected[0]), disc2 = await readText(expected[1]);
      if (!/Sine A/.test(top) || !/Sine B/.test(top) || /Sine C/.test(top) || !/Test Artist - Sine C/.test(disc2)) throw new Error(JSON.stringify({ top, disc2 }));
      return options.join(' | ');
    });

    await step('Playlist (all files) follows the manual list order; entries are relative to the working directory', async () => {
      await clickRow('sine-a.mp3');
      const before = names();
      key('ArrowDown', { altKey: true }, row('sine-a.mp3'));
      await waitFor(() => names().join() !== before.join(), 3000, 'manual move');
      const order = names();
      const target = await playlist();
      if (target !== `${album}${sep}album.m3u8`) throw new Error('target ' + target);
      const read = await inv('read_playlist', { path: target });
      if (read.files.map(p => p.split(/[\\/]/).pop()).join() !== order.join()) throw new Error(`playlist ${read.files.join()} list ${order.join()}`);
      const text = await readText(target);
      if (!text.includes(`\r\ndisc2${sep}sine-c.mp3\r\n`) || text.includes(album)) throw new Error(JSON.stringify(text));
      return `${before.join(',')} → ${order.join(',')}`;
    });

    await step('a playlist outside the working directory lists absolute paths', async () => {
      document.body.focus(); key('o', { metaKey: true }, document.body);
      const options = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 5000, 'options');
      options.querySelector('[data-page="playlist"]').click(); await sleep(150);
      const relative = options.querySelector('[data-option="pages.playlist.relative"]');
      const label = relative?.closest('label')?.textContent || '';
      if (!/Entries relative to working directory/.test(label) || !relative.checked) throw new Error('label ' + label);
      setValue(options.querySelector('[data-option="pages.playlist.filename"]'), `${work}/outside.m3u8`);
      options.querySelector('#options-ok').click();
      await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
      const target = await playlist();
      if (target !== `${work}${sep}outside.m3u8`) throw new Error('target ' + target);
      const text = await readText(target);
      if (!text.includes(`\r\n${album}${sep}sine-a.mp3\r\n`) || !text.includes(`${album}${sep}disc2${sep}sine-c.mp3`)) throw new Error(JSON.stringify(text));
      return target;
    });

    await step('Find remembers criteria in the configuration; ► removes one', async () => {
      document.body.focus(); key('f', { metaKey: true }, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].find-dialog'), 5000, 'find');
      if (dlg.querySelector('#find-clear-history')) throw new Error('old Clear history button');
      const input = dlg.querySelector('#find-criteria');
      for (const term of ['Sine', 'Test Artist']) { input.focus(); setValue(input, term); key('Enter', {}, input); await waitFor(() => /match/.test(dlg.querySelector('#find-status').textContent), 3000, 'results'); }
      await waitFor(async () => (await lists())['opentag.history.find']?.length === 2, 3000, 'find history saved');
      const button = await waitFor(() => input.parentElement.querySelector('.helper-button'), 3000, '► button');
      button.click();
      const menu = await waitFor(() => document.querySelector('.helper-menu'), 2000, 'helper menu');
      const entries = [...menu.querySelectorAll('button')];
      const remove = entries.find(b => b.textContent === 'Remove from history'), all = entries.find(b => b.textContent === 'Remove all from history…');
      if (!remove || remove.disabled || !all) throw new Error(entries.map(b => b.textContent).join('|'));
      remove.click();
      await waitFor(async () => JSON.stringify((await lists())['opentag.history.find']) === JSON.stringify([{ value: 'Sine' }]), 3000, 'term removed');
      dlg.querySelector('[data-close]').click();
      await waitFor(() => !document.querySelector('dialog[open].find-dialog'), 2000, 'find closed');
      key('f', { metaKey: true }, document.body);
      const again = await waitFor(() => document.querySelector('dialog[open].find-dialog'), 5000, 'find again');
      const suggestions = await waitFor(() => { const id = again.querySelector('#find-criteria').getAttribute('list'); const list = id && document.getElementById(id); return list && [...list.options].map(o => o.value).join(','); }, 3000, 'suggestions');
      again.querySelector('[data-close]').click();
      if (suggestions !== 'Sine') throw new Error('suggestions ' + suggestions);
      return 'remaining: ' + suggestions;
    });

    await step('dragging the cover out writes the displayed image to a file', async () => {
      await clickRow('sine-c.mp3');
      const art = await waitFor(() => document.querySelector('.cover-art img[data-cover-image]')?.src?.startsWith('data:') && document.querySelector('.cover-art'), 8000, 'cover shown');
      const box = art.getBoundingClientRect(), x = box.left + 10, y = box.top + 10;
      const pointer = (type, target, dx) => target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, button: 0, buttons: 1, pointerId: 1, isPrimary: true, clientX: x + dx, clientY: y }));
      pointer('pointerdown', art, 0); pointer('pointermove', window, 2); pointer('pointermove', window, 30);
      await waitFor(() => /Cover file ready to drag: /.test(status()), 5000, 'prepared (' + status() + ' / ' + errorsText() + ')');
      const path = status().replace(/^.*Cover file ready to drag: /, '').trim();
      // Only the Linux drag source needs URI-safe names; elsewhere the album name keeps its space.
      const expected = /Linux/.test(navigator.platform) ? 'Test_Album.png' : 'Test Album.png';
      if (path.split(/[\\/]/).pop() !== expected) throw new Error('file ' + path);
      const staged = await inv('stage_cover_file', { path });
      if (staged.width !== 16 || staged.height !== 16 || staged.mime !== 'image/png') throw new Error(JSON.stringify({ ...staged, data_base64: undefined }));
      window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true }));
      return path;
    });

    await step('Manage List: ticked entries are ► favorites; Duplicate needs a changed copy', async () => {
      // The File List may re-render the filter bar; look the input up each time.
      const filter = () => document.getElementById('filter');
      const helper = () => filter().parentElement.querySelector('.helper-button');
      filter().focus(); setValue(filter(), 'sine'); key('Enter', {}, filter());
      await waitFor(async () => (await lists())['opentag.history.filter']?.[0]?.value === 'sine', 3000, 'filter history');
      helper().click();
      const manage = await waitFor(() => [...document.querySelectorAll('.helper-menu button')].find(b => b.textContent === 'Manage history…'), 2000, 'Manage history…');
      manage.click();
      const dlg = await waitFor(() => document.querySelector('dialog[open].manage-history-dialog'), 2000, 'Manage history dialog');
      dlg.querySelector('li[data-index="0"]').click(); await sleep(50);
      dlg.querySelector('[data-favorite="0"]').click();
      await waitFor(async () => (await lists())['opentag.history.filter'][0].favorite === true, 3000, 'favorite saved');
      dlg.querySelector('[data-duplicate]').click();
      const form = await waitFor(() => dlg.querySelector('.history-form'), 2000, 'duplicate form');
      form.querySelector('[data-keep]').click();
      await waitFor(() => /already listed/.test(form.querySelector('.dialog-error')?.textContent || ''), 2000, 'unchanged copy refused');
      form.querySelector('[data-value]').value = 'sine b';
      form.querySelector('[data-keep]').click();
      await waitFor(() => dlg.querySelectorAll('li[data-index]').length === 2, 2000, 'copy listed');
      const saved = await waitFor(async () => { const l = (await lists())['opentag.history.filter']; return l?.length === 2 && l; }, 3000, 'copy saved');
      if (saved[0].value !== 'sine' || saved[1].value !== 'sine b' || !saved[1].favorite) throw new Error(JSON.stringify(saved));
      dlg.querySelector('[data-close]').click();
      await waitFor(() => !document.querySelector('dialog[open].manage-history-dialog'), 2000, 'closed');
      setValue(filter(), ''); await sleep(300);
      helper().click();
      const menu = await waitFor(() => document.querySelector('.helper-menu'), 2000, 'helper menu');
      const favorites = [...menu.querySelectorAll('button')].map(b => b.textContent);
      if (!favorites.includes('sine') || !favorites.includes('sine b')) throw new Error(favorites.join('|'));
      [...menu.querySelectorAll('button')].find(b => b.textContent === 'sine b').click();
      await waitFor(() => filter().value === 'sine b', 2000, 'favorite applied');
      setValue(filter(), '');
      return JSON.stringify(saved);
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
