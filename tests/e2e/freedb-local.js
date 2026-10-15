// Tag Sources → freedb (local) end to end: Options, disc ID search, editing
// values in the review, length sorting, writing and the undo step. Offline:
// scripts/e2e-native.sh copies tests/fixtures/freedb next to the album.
(async () => {
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  const check = async (ok, what) => report(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  const type = (input, value) => { input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); };
  try {
    const rows = await waitFor(() => { const r = [...document.querySelectorAll('tr[data-path]')]; return r.length === 2 && r; }, 15000, 'rows');
    const paths = rows.map(r => r.dataset.path), album = paths[0].replace(/[\\/][^\\/]+$/, ''), freedb = album.replace(/([\\/])[^\\/]+$/, '$1freedb');

    // Options → Tag Sources → Local freedb, through the real dialog.
    document.querySelector('button[data-op="source-options"]').click();
    const options = await waitFor(() => document.querySelector('dialog.options-dialog[open]'), 5000, 'options');
    const tree = [...options.querySelectorAll('[role="treeitem"]')].map(b => b.dataset.page);
    await check(tree.includes('tag-sources-freedb') && tree.includes('tag-sources-local-freedb'), 'options pages ' + tree.filter(p => p.startsWith('tag-sources')).join(','));
    options.querySelector('[data-page="tag-sources-freedb"]').click();
    await check(options.querySelector('[data-option="pages.tag-sources-freedb.address"]').value === 'gnudb.gnudb.org', 'freedb defaults to gnudb');
    options.querySelector('[data-option="pages.tag-sources-freedb.disc_id_field"]').click();
    options.querySelector('[data-page="tag-sources-local-freedb"]').click();
    type(options.querySelector('#local-freedb-directory'), freedb);
    options.querySelector('#options-ok').click();
    await waitFor(() => !options.open, 5000, 'options saved');
    const saved = (await inv('load_settings')).settings.pages;
    await check(saved['tag-sources-local-freedb']?.directory === freedb && saved['tag-sources-freedb']?.disc_id_field === true, 'local folder and DISCID option saved');

    // Select both files and open freedb (local) with Ctrl+L.
    document.querySelector('button[data-op="select-all"]').click();
    await waitFor(() => document.querySelectorAll('tr[data-path].selected,tr[data-path][aria-selected="true"]').length === 2, 5000, 'selection');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'l', ctrlKey: true, bubbles: true }));
    const d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, 'freedb dialog');
    await check(d.getAttribute('aria-label') === 'freedb (local)', 'Ctrl+L opens freedb (local)');
    const toc = d.querySelector('#ts-toc').textContent, search = d.querySelector('.ts-search').getBoundingClientRect(), tocBox = d.querySelector('#ts-toc').getBoundingClientRect();
    await check(tocBox.width > 200 && tocBox.bottom <= search.bottom + 1, `disc ID line sits in the search row (${Math.round(tocBox.width)}px)`);
    await check(/Disc ID [0-9a-f]{8}: 2 tracks/.test(toc), 'disc ID from the selected files: ' + toc);
    d.querySelector('#ts-search').click();
    await waitFor(() => /No disc|found/.test(d.querySelector('.ts-status').textContent), 10000, 'toc search');
    await report('toc search status=' + d.querySelector('.ts-status').textContent);
    const mode = d.querySelector('#ts-mode'); mode.value = 'disc_id'; mode.dispatchEvent(new Event('change', { bubbles: true }));
    d.querySelector('#ts-text').value = '0A000802';
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 10000, 'results');
    const first = d.querySelector('.ts-results tbody tr').textContent;
    await check(first.includes('Fixture Artist') && first.includes('rock') && first.includes('0a000802'), 'result row ' + first);
    await check(!d.querySelector('.ts-preview'), 'local entries have no preview link');
    d.querySelector('#ts-next').click();
    const apply = await waitFor(() => d.querySelector('#ts-apply'), 10000, 'review');
    const fields = [...d.querySelectorAll('.ts-fields tr[data-row]')].map(r => r.dataset.row);
    await check(['TITLE', 'ARTIST', 'ALBUM', 'YEAR', 'TRACK', 'GENRE', 'DISCID'].every(f => fields.includes(f)), 'fields ' + fields.join(','));

    const content = d.querySelector('.dialog-content'), value0 = d.querySelector('.ts-value').getBoundingClientRect(), track0 = d.querySelector('.ts-track-value').getBoundingClientRect();
    await check(content.scrollWidth <= content.clientWidth + 1 && value0.width > 120 && track0.width > 80, `review layout fits (content ${content.scrollWidth}/${content.clientWidth}, value ${Math.round(value0.width)}px, track ${Math.round(track0.width)}px)`);
    // Adjust values before saving: an album field and one track title.
    type(d.querySelector('.ts-value[data-value="ALBUM"]'), 'Edited Album \\\\ Second Value');
    type(d.querySelectorAll('.ts-track-value')[1], 'Renamed Tone');
    await check(d.querySelector('tr[data-row="TITLE"] .ts-summary')?.textContent.includes('2 values'), 'track edit updates the field list');
    d.querySelector('#ts-by-length').click();
    await check(d.querySelectorAll('.ts-matches tbody tr').length === 2 && !d.querySelector('.ts-matches em'), 'sorting by length keeps every file matched');
    d.dispatchEvent(new KeyboardEvent('keydown', { key: 'd', ctrlKey: true, bubbles: true }));
    await check(apply.disabled && ![...d.querySelectorAll('.ts-fields input[type=checkbox]')].some(b => b.checked), 'Ctrl+D disables every field');
    d.querySelector('.ts-fields').dispatchEvent(new KeyboardEvent('keydown', { key: 'e', metaKey: true, bubbles: true }));
    await check(!apply.disabled, 'Cmd/Ctrl+E enables every field');
    await check(document.querySelector('.ts-cover p')?.textContent.includes('No artwork'), 'freedb has no artwork');
    apply.click();
    await waitFor(() => !d.open, 20000, 'apply');
    const disk = await Promise.all(paths.map(async p => (await inv('scan_files', { paths: [p], recursive: false })).entries[0].fields));
    const value = (f, k) => (f[k] || f[k.toLowerCase()] || []).join(' | ');
    await report('disk ' + disk.map(f => ['TITLE', 'ARTIST', 'ALBUM', 'YEAR', 'GENRE', 'DISCID'].map(k => `${k}=${value(f, k)}`).join(' ')).join(' / '));
    await check(disk.every(f => value(f, 'ALBUM') === 'Edited Album | Second Value' && value(f, 'ARTIST') === 'Fixture Artist' && value(f, 'DISCID') === '0a000802'), 'album edit, artist and DISCID written');
    await check(disk.some(f => value(f, 'TITLE') === 'Renamed Tone') && disk.some(f => value(f, 'TITLE') === 'First Tone'), 'track titles written, including the edited one');
    const history = await inv('history_state');
    await check(history.undo?.label === 'Tag source import' && history.undo?.files === 2, 'undo step ' + JSON.stringify(history.undo));
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await report('SUMMARY done');
  await inv('e2e_exit', { code: 0 });
})();
