// Saves one field across a large generated FLAC folder and undoes it. Run
// through scripts/e2e-undo-bulk.sh, which prepends `window.__OPENTAG_BULK__`
// ({files: count, bytes: total size}). The folder's total size exceeds the
// undo journal's 2 GiB cap, so the step is only recorded when clone-backed
// padding writes are counted by the bytes they really keep.
(async () => {
  const config = window.__OPENTAG_BULK__ || {};
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: String(m) });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(100); } throw new Error('timeout waiting for ' + what); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const idle = (ms = 600000) => waitFor(() => !document.querySelector('footer.status.busy'), ms, 'idle');
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push('PASS'); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push('FAIL'); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  let paths = [];
  const albums = async () => {
    const sample = [paths[0], paths[Math.floor(paths.length / 2)], paths[paths.length - 1]];
    const batch = await inv('scan_files', { paths: sample, recursive: false });
    return batch.entries.map(e => (e.fields.ALBUM || e.fields.album || [''])[0]);
  };
  try {
    await waitFor(() => Number(document.querySelector('.location strong')?.textContent.split(' ')[0]) === config.files && !document.querySelector('footer.status.busy'), 300000, `${config.files} files`);
    await report(`loaded files=${config.files} bytes=${config.bytes}`);

    await step('select all and stage ALBUM', async () => {
      document.body.focus(); key('a', { metaKey: true }); await waitFor(() => rows().every(r => r.classList.contains('selected')), 10000, 'selection');
      const f = field('ALBUM'); f.focus(); f.value = 'Bulk Undo Album'; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(300);
      f.blur(); document.body.focus();
      return status();
    });

    await step(`Cmd+S saves ${config.files} files as one undo step`, async () => {
      const started = performance.now();
      key('s', { metaKey: true }); await sleep(300); await idle();
      if (!/Saved tags/.test(status())) throw new Error('status=' + status());
      const state = await inv('history_state');
      if (state.notice) throw new Error('notice=' + state.notice);
      if (state.undo?.label !== 'Save tags' || state.undo.files !== config.files) throw new Error('undo=' + JSON.stringify(state.undo));
      const label = document.querySelector('.menubar [data-op="undo"]')?.textContent.trim();
      return `${Math.round(performance.now() - started)} ms; ${label}`;
    });

    await step('saved values are on disk', async () => {
      const scan = await inv('scan_files', { paths: [rows()[0].dataset.path.replace(/\/[^/]*$/, '')], recursive: true });
      paths = scan.entries.map(e => e.path).sort();
      if (paths.length !== config.files) throw new Error('scanned ' + paths.length);
      const values = await albums(); if (values.some(v => v !== 'Bulk Undo Album')) throw new Error(JSON.stringify(values)); return values.join('|');
    });

    await step('Cmd+Z restores every file', async () => {
      const started = performance.now();
      document.querySelector('tr[data-path]').focus(); key('z', { metaKey: true }); await sleep(300); await idle();
      if (!/Undid Save tags/.test(status())) throw new Error('status=' + status());
      const scan = await inv('scan_files', { paths, recursive: false });
      const left = scan.entries.filter(e => (e.fields.ALBUM || e.fields.album || [''])[0] === 'Bulk Undo Album').length;
      if (left || scan.entries.length !== config.files) throw new Error(`still saved=${left} read=${scan.entries.length}`);
      return `${Math.round(performance.now() - started)} ms; ${status()}`;
    });

    await step('Cmd+Y re-applies the save', async () => {
      document.querySelector('tr[data-path]').focus(); key('y', { metaKey: true }); await sleep(300); await idle();
      if (!/Redid Save tags/.test(status())) throw new Error('status=' + status());
      const values = await albums(); if (values.some(v => v !== 'Bulk Undo Album')) throw new Error(JSON.stringify(values)); return status();
    });
  } catch (e) { await report('FATAL ' + (e && e.message || e)); }
  await report(`SUMMARY ${results.filter(r => r === 'PASS').length}/${results.length} passed`);
  await inv('e2e_exit', { code: 0 });
})();
