// Native e2e for Options → Library: the page, loading through the library,
// writes, undo and deletes made by OpenTag updating it, Reload tags, the
// directory restriction, Cleanup Library and Clear Library.
// Fixtures: library-fixtures.sh (one MP3 in a folder next to the album).
// Runs in CI (headless); never on a working desktop.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}, target = document.activeElement || document.body) => target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.split(/[\\/]/).pop() === name);
  const pathOf = name => row(name).dataset.path;
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(250); };
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 20000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const library = () => inv('library_status');
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const mod = /Mac/.test(navigator.platform) ? { metaKey: true } : { ctrlKey: true };
  const openOptions = async page => {
    for (let attempt = 0; attempt < 3; attempt++) {
      document.body.focus(); key('o', mod, document.body);
      const dlg = await waitFor(() => document.querySelector('dialog[open].options-dialog'), 2000, 'options dialog').catch(() => null);
      if (dlg) { dlg.querySelector(`[data-page="${page}"]`).click(); await sleep(150); return dlg; }
      await sleep(300);
    }
    throw new Error('options dialog did not open');
  };
  const toggle = async (dlg, path, value) => {
    const el = dlg.querySelector(`[data-option="${path}"]`); if (!el) throw new Error('no control for ' + path);
    el.checked = value; el.dispatchEvent(new Event('input', { bubbles: true })); el.dispatchEvent(new Event('change', { bubbles: true })); await sleep(100);
  };
  const ok = async dlg => { dlg.querySelector('#options-ok').click(); await waitFor(() => !document.querySelector('dialog[open].options-dialog'), 5000, 'options closed'); await idle(); };
  // View → Refresh; the previous refresh's status text must not end the wait.
  const refresh = async () => {
    await idle(); const b = op('refresh'); if (!b || b.disabled) throw new Error('Refresh is unavailable');
    let started = false;
    const watch = new MutationObserver(() => { if (document.querySelector('footer.status.busy') || /Refreshing/.test(status())) started = true; });
    watch.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true });
    try { b.click(); await waitFor(() => started && !document.querySelector('footer.status.busy') && /Refreshed/.test(status()), 15000, 'refreshed'); } finally { watch.disconnect(); }
  };
  const expect = (actual, wanted, what) => { for (const [k, v] of Object.entries(wanted)) if (actual[k] !== v) throw new Error(`${what}: ${k}=${actual[k]}, expected ${v} (${JSON.stringify(actual)})`); };

  try {
    await waitFor(() => rows().length >= 2, 15000, 'fixture rows');
    const album = pathOf('sine-a.mp3').replace(/[\\/][^\\/]+$/, '');
    const separator = album.includes('\\') ? '\\' : '/';
    const elsewhere = album.replace(/[\\/][^\\/]+$/, '') + separator + 'elsewhere';
    const count = rows().length;

    await step('the Library page sits after Appearance with Mp3tag’s controls', async () => {
      const dlg = await openOptions('library');
      const order = [...dlg.querySelectorAll('[role="treeitem"]')].map(b => b.dataset.page);
      if (order.indexOf('library') !== order.indexOf('appearance') + 1) throw new Error('page order ' + order.join(','));
      const labels = [...dlg.querySelectorAll('.options-page label.action-check')].map(l => l.textContent.trim());
      if (labels.join('|') !== 'Enable Library|Only include these directories') throw new Error('checkboxes ' + labels.join('|'));
      if (!dlg.querySelector('#library-add').disabled || !dlg.querySelector('#library-directories').disabled) throw new Error('directory list enabled without the restriction');
      const text = await waitFor(() => { const t = dlg.querySelector('#library-status').textContent; return /empty/.test(t) && t; }, 5000, 'empty library status');
      await toggle(dlg, 'pages.library.enabled', true);
      await ok(dlg);
      const saved = (await inv('load_settings')).settings.pages.library;
      expect(saved, { enabled: true, restrict: false }, 'saved options');
      return text;
    });

    await step('loading the folder stores every file and the next load reads none', async () => {
      let started = performance.now();
      await refresh();
      const first = performance.now() - started, stored = await library();
      expect(stored, { created: true, files: count, last_hits: 0, last_stored: count }, 'first load');
      if (!/library[\\/]library\.db$/.test(stored.location)) throw new Error('database at ' + stored.location);
      started = performance.now();
      await refresh();
      expect(await library(), { files: count, last_hits: count, last_stored: 0 }, 'second load');
      return `${count} files, ${stored.bytes} bytes; refresh ${first.toFixed(0)} ms then ${(performance.now() - started).toFixed(0)} ms`;
    });

    await step('files outside the included directories are stored without a restriction', async () => {
      const scan = await inv('scan_files', { paths: [elsewhere], recursive: true });
      if (scan.entries.length !== 1) throw new Error('elsewhere entries ' + scan.entries.length);
      expect(await library(), { files: count + 1, last_stored: 1 }, 'elsewhere');
    });

    await step('a tag saved with preserved timestamps is read again on the next load', async () => {
      const dlg = await openOptions('files');
      await toggle(dlg, 'tags.preserve_modified_time', true);
      await ok(dlg);
      await clickRow('sine-a.mp3');
      const f = field('TITLE'); f.focus(); f.value = 'Library Edit'; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
      f.blur(); document.body.focus(); key('s', mod); await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status');
      await refresh();
      expect(await library(), { files: count + 1, last_hits: count - 1, last_stored: 1 }, 'after save');
      await clickRow('sine-a.mp3');
      if (field('TITLE').value !== 'Library Edit') throw new Error('TITLE ' + field('TITLE').value);
    });

    await step('undo is read again on the next load', async () => {
      await waitFor(() => !op('undo')?.disabled, 8000, 'undo available');
      row('sine-a.mp3').focus(); key('z', mod); await waitFor(() => /Undid/.test(status()), 8000, 'undo');
      await refresh();
      expect(await library(), { last_hits: count - 1, last_stored: 1 }, 'after undo');
      await clickRow('sine-a.mp3');
      if (field('TITLE').value !== 'Sine A') throw new Error('TITLE ' + field('TITLE').value);
    });

    await step('Reload tags reads the selected file again and keeps the result', async () => {
      await clickRow('sine-b.mp3');
      row('sine-b.mp3').focus(); key('t', mod); await waitFor(() => /Reloaded/.test(status()), 8000, 'reloaded');
      expect(await library(), { last_hits: 0, last_stored: 1 }, 'reload');
    });

    await step('restricting to the album and Cleanup Library remove the other folder', async () => {
      window.__OPENTAG_E2E_DIALOGS__ = { requests: [], answers: [album] };
      const dlg = await openOptions('library');
      await toggle(dlg, 'pages.library.restrict', true);
      const add = dlg.querySelector('#library-add'); if (add.disabled) throw new Error('Add… stays disabled');
      add.click();
      await waitFor(() => dlg.querySelector('#library-directories option')?.textContent === album, 5000, 'listed album');
      dlg.querySelector('#library-cleanup').click();
      const text = await waitFor(() => { const t = dlg.querySelector('#library-status').textContent; return /^Cleanup checked/.test(t) && t; }, 15000, 'cleanup result');
      if (!/removed 1 orphaned entry/.test(text)) throw new Error(text);
      await ok(dlg);
      expect((await inv('load_settings')).settings.pages.library, { restrict: true }, 'saved restriction');
      expect(await library(), { files: count }, 'after cleanup');
      await inv('scan_files', { paths: [elsewhere], recursive: true });
      expect(await library(), { files: count, last_hits: 0, last_stored: 0 }, 'outside the included directories');
      delete window.__OPENTAG_E2E_DIALOGS__;
      return text;
    });

    await step('files deleted in OpenTag leave the library', async () => {
      const result = await inv('delete_files_permanently', { paths: [pathOf('sine-b.mp3')] });
      if (result.errors?.length) throw new Error(JSON.stringify(result.errors));
      expect(await library(), { files: count - 1 }, 'after delete');
    });

    await step('Clear Library empties the database', async () => {
      const dlg = await openOptions('library');
      const clear = await waitFor(() => { const b = dlg.querySelector('#library-clear'); return !b.disabled && b; }, 5000, 'Clear Library enabled');
      clear.click();
      dlg.querySelector('#library-clear-yes').click();
      const text = await waitFor(() => { const t = dlg.querySelector('#library-status').textContent; return /cleared/.test(t) && t; }, 8000, 'cleared');
      dlg.querySelector('#options-cancel').click();
      expect(await library(), { files: 0 }, 'after clear');
      return text;
    });
  } catch (e) {
    await report(`FATAL ${e && e.stack || e}`);
  }
  await finish();
})();
