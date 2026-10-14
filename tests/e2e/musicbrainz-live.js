(async () => {
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  // Live network test: point /fp: at a folder holding a copy of this file.
  const name = window.OPENTAG_E2E_FILE || '002. Adriatique - Palm Of Your Hands (Extended Mix).flac';
  try {
    const r = await waitFor(() => [...document.querySelectorAll('tr[data-path]')].find(x => x.dataset.path.endsWith('/' + name)), 15000, 'row');
    r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true })); await sleep(400);
    document.querySelector('button[data-op="source-musicbrainz"]').click();
    const d = await waitFor(() => document.querySelector('dialog[open] #ts-artist') && document.querySelector('dialog[open]'), 5000, 'dialog');
    await report('prefill artist=' + d.querySelector('#ts-artist').value + ' album=' + d.querySelector('#ts-album').value);
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 30000, 'results');
    await report('results=' + d.querySelectorAll('.ts-results tbody tr').length + ' first=' + d.querySelector('.ts-results tbody tr').textContent.slice(0, 120));
    await waitFor(() => !d.querySelector('#ts-next').disabled, 5000, 'next'); d.querySelector('#ts-next').click();
    const apply = await waitFor(() => d.querySelector('#ts-apply'), 30000, 'review'); await waitFor(() => !apply.disabled, 30000, 'apply enabled');
    await report('review fields=' + [...d.querySelectorAll('.ts-fields tr')].map(t => t.textContent.trim().split(/\s+/)[0]).slice(0, 14).join(',') + ' matches=' + d.querySelectorAll('.ts-matches tbody tr').length + ' apply=' + apply.textContent);
    apply.click();
    await waitFor(() => !d.open, 60000, 'apply finish');
    await report('status=' + document.querySelector('.status-message').textContent + ' errors=' + [...document.querySelectorAll('.errors p')].map(p => p.textContent).join(' / '));
    const p = [...document.querySelectorAll('tr[data-path]')].find(x => x.dataset.path.endsWith('/' + name)).dataset.path;
    const e = (await inv('scan_files', { paths: [p], recursive: false })).entries[0];
    const f = e.fields; await report('disk ALBUM=' + f.ALBUM + ' ARTIST=' + f.ARTIST + ' MB_ALBUMID=' + f.MUSICBRAINZ_ALBUMID + ' TRACK=' + (f.TRACKNUMBER || f.TRACK) + ' covers=' + e.covers.length);
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await inv('e2e_exit', { code: 0 });
})();
