// Native e2e for File List behaviours: the folder-image fallback of the cover
// pane, column auto-size (divider double-click, Ctrl/⌘ Num +), manual
// reordering (Alt+arrows, Alt+Page keys, Alt+drag) feeding numbering order,
// Copy/Cut/Paste tags with artwork (pending until saved, undoable) and
// Options → Tags "Save tags when using arrow keys / single mouse click".
// Fixtures: list-behaviour-fixtures.sh (folder.png, sine-c.mp3 with a cover).
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
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  const order = () => rows().map(r => r.dataset.path.split('/').pop()).join(',');
  const clickRow = async (name, o = {}) => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...o })); await sleep(250); };
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 20000, 'idle');
  const done = async (pattern, what) => waitFor(() => pattern.test(status()) && !document.querySelector('footer.status.busy'), 15000, `${what} (status "${status()}" errors "${errorsText()}")`);
  const scan = async name => (await inv('scan_files', { paths: [row(name).dataset.path], recursive: false })).entries[0];
  const info = () => document.querySelector('.cover-info')?.textContent || '';
  const titleField = () => document.querySelector('.fields input[data-field="TITLE"]');
  const typeTitle = value => { const input = titleField(); input.focus(); input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); return input; };
  const meta = { metaKey: true };
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const setOptions = async (page, changes) => {
    let dlg;
    for (let attempt = 0; attempt < 3 && !dlg; attempt++) { document.body.focus(); key('o', meta, document.body); dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null); }
    if (!dlg) throw new Error('options dialog did not open');
    dlg.querySelector(`[data-page="${page}"]`).click(); await sleep(150);
    for (const [path, value] of changes) {
      const el = dlg.querySelector(`[data-option="${path}"]`); if (!el) throw new Error('no control for ' + path);
      el.checked = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); await sleep(60);
    }
    dlg.querySelector('#options-ok').click();
    await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed');
    await idle();
  };

  try {
    await waitFor(() => rows().length >= 3, 15000, 'three fixture rows');
    await idle();

    await step('fixtures: three MP3s, only sine-c has embedded art', async () => {
      const covers = await Promise.all(['sine-a.mp3', 'sine-b.mp3', 'sine-c.mp3'].map(async name => (await scan(name)).covers.length));
      if (covers.join() !== '0,0,1') throw new Error('covers ' + covers.join());
      return `order ${order()}`;
    });

    await step('cover pane falls back to the first image of the folder, marked as not embedded', async () => {
      await clickRow('sine-a.mp3');
      const image = await waitFor(() => document.querySelector('.cover-art.folder-image img[data-folder-image]'), 8000, 'folder image');
      await waitFor(() => image.src.startsWith('data:image/png'), 3000, 'folder image data');
      if (!/Folder image/.test(info()) || !/folder\.png/.test(info()) || !/Not embedded/.test(info())) throw new Error('info ' + info());
      if ((await scan('sine-a.mp3')).covers.length) throw new Error('the folder image was embedded');
      await clickRow('sine-c.mp3');
      await waitFor(() => !document.querySelector('[data-folder-image]') && /Front Cover/.test(info()), 5000, 'embedded art wins');
      await clickRow('sine-a.mp3'); await clickRow('sine-c.mp3', { ctrlKey: true, metaKey: true });
      await waitFor(() => !document.querySelector('[data-folder-image]'), 3000, 'no folder image while a selected file has art');
      return info().replace(/\s+/g, ' ').trim();
    });

    await step('"Don\'t display first image from file directory" hides the fallback', async () => {
      await setOptions('tags', [['pages.tags.no_folder_image', true]]);
      await clickRow('sine-b.mp3');
      await waitFor(() => /No artwork/.test(info()), 5000, 'no artwork');
      if (document.querySelector('[data-folder-image]')) throw new Error('folder image still shown');
      await setOptions('tags', [['pages.tags.no_folder_image', false]]);
      await clickRow('sine-a.mp3');
      await waitFor(() => document.querySelector('[data-folder-image]'), 5000, 'folder image back');
      return 'hidden while switched on, shown again when off';
    });

    await step('double-clicking a column divider sizes the column to its contents', async () => {
      const th = () => document.querySelector('th[data-col="title"]'), before = th().offsetWidth;
      th().querySelector('[data-resize]').dispatchEvent(new MouseEvent('dblclick', { bubbles: true, cancelable: true }));
      await waitFor(() => th() && th().offsetWidth < before - 20, 8000, 'narrower title column');
      const saved = await waitFor(async () => { const width = (await inv('load_columns'))?.columns.find(c => c.id === 'title')?.width; return width && width < before ? width : 0; }, 5000, 'saved width');
      return `title ${before}px → ${th().offsetWidth}px (saved ${saved})`;
    });

    await step('Ctrl/⌘ Num + sizes every column', async () => {
      const width = id => document.querySelector(`th[data-col="${id}"]`)?.offsetWidth, before = width('artist');
      document.body.focus(); key('+', { code: 'NumpadAdd', location: 3, ctrlKey: true }, document.body);
      await waitFor(() => width('artist') !== before, 8000, 'artist width');
      const columns = (await inv('load_columns')).columns.filter(c => c.visible).map(c => `${c.id}:${c.width}`).join(' ');
      return `artist ${before}px → ${width('artist')}px; ${columns}`;
    });

    await step('Alt+↑ / Alt+Page keys move the selected file and leave the sorted view', async () => {
      if (order() !== 'sine-a.mp3,sine-b.mp3,sine-c.mp3') throw new Error('start order ' + order());
      await clickRow('sine-c.mp3');
      key('ArrowUp', { altKey: true }, row('sine-c.mp3'));
      await waitFor(() => order() === 'sine-a.mp3,sine-c.mp3,sine-b.mp3', 5000, 'moved up: ' + order());
      if ([...document.querySelectorAll('th[data-col]')].some(th => th.getAttribute('aria-sort') !== 'none')) throw new Error('still sorted');
      await waitFor(async () => (await inv('load_columns')).sort.length === 0, 5000, 'unsorted layout saved');
      key('PageUp', { altKey: true }, row('sine-c.mp3'));
      await waitFor(() => order() === 'sine-c.mp3,sine-a.mp3,sine-b.mp3', 5000, 'moved to the beginning: ' + order());
      key('PageDown', { altKey: true }, row('sine-c.mp3'));
      await waitFor(() => order() === 'sine-a.mp3,sine-b.mp3,sine-c.mp3', 5000, 'moved to the end: ' + order());
      if (document.activeElement !== row('sine-c.mp3')) throw new Error('focus left the moved row');
      return order();
    });

    await step('Alt+drag moves files to the drop line, and numbering follows the new order', async () => {
      await clickRow('sine-a.mp3');
      const wrap = document.querySelector('.table-wrap'), first = row('sine-a.mp3').getBoundingClientRect(), last = row('sine-c.mp3').getBoundingClientRect();
      const pointer = (type, target, y) => target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, altKey: true, button: 0, buttons: 1, pointerId: 1, isPrimary: true, clientX: first.left + 30, clientY: y }));
      pointer('pointerdown', row('sine-a.mp3').querySelector('td'), first.top + 5);
      pointer('pointermove', wrap, first.top + 20);
      pointer('pointermove', wrap, last.bottom - 3);
      if (!document.querySelector('.row-drop-marker')) throw new Error('no drop line while dragging');
      pointer('pointerup', wrap, last.bottom - 3);
      wrap.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, clientX: first.left + 30, clientY: last.bottom - 3 }));
      await waitFor(() => order() === 'sine-b.mp3,sine-c.mp3,sine-a.mp3', 5000, 'dragged to the end: ' + order());
      if (!row('sine-a.mp3').classList.contains('selected')) throw new Error('the dragged file lost its selection');
      document.body.focus(); key('a', meta, document.body); await sleep(200);
      key('k', meta, document.body);
      const dialog = await waitFor(() => document.querySelector('dialog[open].numbering-dialog'), 5000, 'numbering dialog');
      dialog.querySelector('#preview-number').click();
      const files = await waitFor(() => { const cells = [...dialog.querySelectorAll('#number-preview tbody tr')].map(tr => tr.cells[0].textContent); return cells.length === 3 && cells; }, 8000, 'numbering preview');
      dialog.querySelector('[data-close]').click();
      if (files.join() !== 'sine-b.mp3,sine-c.mp3,sine-a.mp3') throw new Error('numbering order ' + files.join());
      return `numbering previews ${files.join(', ')}`;
    });

    await step('Copy tags includes the artwork; pasted covers stay pending until Save, then undo restores', async () => {
      const source = await scan('sine-c.mp3');
      await clickRow('sine-c.mp3'); key('c', meta, row('sine-c.mp3'));
      await done(/Copied tags of 1 file with 1 cover/, 'copied');
      await clickRow('sine-a.mp3'); key('v', meta, row('sine-a.mp3'));
      await done(/Pasted tags to 1 file/, 'pasted');
      await waitFor(() => /Pasted covers are written when you save tags/.test(info()) && document.querySelector('.cover-art img[data-cover-image]'), 5000, 'pending cover shown: ' + info());
      if (!row('sine-a.mp3').querySelector('.rowmarker')) throw new Error('no pending marker');
      await clickRow('sine-b.mp3');
      if ((await scan('sine-a.mp3')).covers.length) throw new Error('written before saving (save on move is off)');
      await clickRow('sine-a.mp3'); key('s', meta, row('sine-a.mp3'));
      await done(/Saved tags: 1 file/, 'saved');
      const saved = await scan('sine-a.mp3');
      if (saved.covers.length !== 1 || saved.covers[0].digest !== source.covers[0].digest || saved.fields.TITLE?.[0] !== 'Sine C') throw new Error(JSON.stringify({ covers: saved.covers, title: saved.fields.TITLE }));
      row('sine-a.mp3').focus(); key('z', meta, row('sine-a.mp3'));
      await done(/Undid/, 'undo');
      const undone = await scan('sine-a.mp3');
      if (undone.covers.length || undone.fields.TITLE?.[0] !== 'Sine A') throw new Error('undo left ' + JSON.stringify({ covers: undone.covers.length, title: undone.fields.TITLE }));
      return `cover ${saved.covers[0].width}×${saved.covers[0].height} written and undone`;
    });

    await step('Cut tags removes the covers on Save', async () => {
      await clickRow('sine-c.mp3'); key('x', meta, row('sine-c.mp3'));
      await done(/Cut tags of 1 file with 1 cover/, 'cut');
      await waitFor(() => /covers are removed when you save tags/.test(info()), 5000, 'pending removal: ' + info());
      if ((await scan('sine-c.mp3')).covers.length !== 1) throw new Error('removed before saving');
      key('s', meta, row('sine-c.mp3')); await done(/Saved tags: 1 file/, 'saved');
      const cut = await scan('sine-c.mp3');
      if (cut.covers.length || cut.fields.TITLE?.length) throw new Error(JSON.stringify({ covers: cut.covers.length, title: cut.fields.TITLE }));
      row('sine-c.mp3').focus(); key('z', meta, row('sine-c.mp3')); await done(/Undid/, 'undo');
      if ((await scan('sine-c.mp3')).covers.length !== 1) throw new Error('undo did not restore the cover');
      return 'removed on save, restored by undo';
    });

    await step('Save tags when moving: a click, an arrow key and Ctrl+N save the file left behind', async () => {
      await setOptions('tags', [['pages.tags.save_on_move', true]]);
      // Order is b, c, a.
      await clickRow('sine-b.mp3'); typeTitle('Saved by click');
      await clickRow('sine-c.mp3'); await idle();
      await waitFor(async () => (await scan('sine-b.mp3')).fields.TITLE?.[0] === 'Saved by click', 8000, 'click saved sine-b');
      typeTitle('Saved by arrow');
      row('sine-c.mp3').focus(); key('ArrowDown', {}, row('sine-c.mp3')); await idle();
      await waitFor(async () => (await scan('sine-c.mp3')).fields.TITLE?.[0] === 'Saved by arrow', 8000, 'arrow saved sine-c');
      await waitFor(() => document.activeElement === row('sine-a.mp3'), 5000, 'focus stays in the file list');
      await clickRow('sine-b.mp3'); await idle();
      const input = typeTitle('Saved by Ctrl+N');
      key('n', meta, input); await idle();
      await waitFor(async () => (await scan('sine-b.mp3')).fields.TITLE?.[0] === 'Saved by Ctrl+N', 8000, 'Ctrl+N saved sine-b');
      if (!row('sine-c.mp3').classList.contains('selected')) throw new Error('Ctrl+N did not select the next file');
      await waitFor(() => document.activeElement?.dataset?.field === 'TITLE', 5000, 'focus stays in the tag panel');
      if (document.querySelector('.dirty-count')) throw new Error('pending edits remain: ' + document.querySelector('.dirty-count').textContent);
      await setOptions('tags', [['pages.tags.save_on_move', false]]);
      return 'three moves, three saves; no pending edits';
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
