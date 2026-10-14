// Live network check (MusicBrainz, Cover Art Archive): result paging and the
// covers-only source with Save image to file. Not part of the offline suite.
(async () => {
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  const check = async (ok, what) => report(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  try {
    const rows = await waitFor(() => { const r = [...document.querySelectorAll('tr[data-path]')]; return r.length === 2 && r; }, 15000, 'rows');
    const paths = rows.map(r => r.dataset.path);
    document.querySelector('button[data-op="select-all"]').click();
    await sleep(300);

    // Paging: a broad search has far more than 50 releases.
    document.querySelector('button[data-op="source-musicbrainz"]').click();
    let d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, 'dialog');
    d.querySelector('#ts-artist').value = 'Daft Punk'; d.querySelector('#ts-album').value = '';
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 30000, 'results');
    const firstPage = d.querySelectorAll('.ts-results tbody tr').length;
    await check(firstPage === 50 && d.querySelector('#ts-more'), `first page ${firstPage} rows, ${d.querySelector('.ts-more span')?.textContent}`);
    d.querySelector('#ts-more').click();
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length > firstPage, 30000, 'second page');
    await check(d.querySelectorAll('.ts-results tbody tr').length > 50, `after More: ${d.querySelectorAll('.ts-results tbody tr').length} rows; ${d.querySelector('.ts-status').textContent}`);
    await check(d.querySelector('.ts-chosen .ts-preview')?.dataset.url.startsWith('https://musicbrainz.org/release/'), 'preview link on the chosen result');
    d.querySelector('[data-close]').click();
    await waitFor(() => !document.querySelector('dialog.tag-source-dialog'), 5000, 'closed');

    // Covers: MusicBrainz… writes only artwork and saves the image next to the files.
    document.querySelector('button[data-op="source-cover-musicbrainz"]').click();
    d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, 'covers dialog');
    await check(d.getAttribute('aria-label') === 'MusicBrainz cover art', 'covers-only dialog');
    const mode = d.querySelector('#ts-mode'); mode.value = 'release_id'; mode.dispatchEvent(new Event('change', { bubbles: true }));
    d.querySelector('#ts-text').value = 'd073287b-d1bd-4f11-a933-a4386f8cf701';
    d.querySelector('#ts-search').click();
    const apply = await waitFor(() => d.querySelector('#ts-apply'), 30000, 'review');
    await check(!d.querySelector('.ts-fields') && !d.querySelector('.ts-matches'), 'no fields or matching in covers-only mode');
    const save = await waitFor(() => d.querySelector('#ts-cover-save'), 10000, 'cover section');
    if (!save.checked) save.click();
    await check(/Save cover to 2 files/.test(apply.textContent), 'apply label ' + apply.textContent);
    apply.click();
    await waitFor(() => !d.open, 60000, 'apply');
    const entries = (await inv('scan_files', { paths, recursive: false })).entries;
    await check(entries.every(e => e.covers.length === 1) && entries.every(e => !e.fields.ALBUM?.includes('Discovery')), 'cover written, fields unchanged: ' + entries.map(e => e.covers.length + '/' + e.fields.ALBUM).join(' '));
    // A failed image save is reported per folder and keeps the dialog open, so closing means it was saved.
    const history = await inv('history_state');
    await check(history.undo?.label === 'Tag source import', 'undo step ' + JSON.stringify(history.undo));
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await report('SUMMARY done');
  await inv('e2e_exit', { code: 0 });
})();
