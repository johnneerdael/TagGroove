// Tag sources round 3, offline: freedb server list and the local text index
// in Options, Tag Sources → Discogs Release ID, Ctrl+I for any source and the
// Ctrl+Shift+I quick variant, the review's field multi-select (click, Ctrl,
// Shift, Ctrl+A, Ctrl+Shift+A, Space), remembered Suggest sorting, drag and
// drop and Alt+Page Down in the track list, and the Discogs token moving to
// the keychain (an in-memory store in scripted runs). Fixtures come from
// tests/e2e/sources-3-fixtures.sh; live network steps are never taken.
(async () => {
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  const check = async (ok, what) => report(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  const type = (input, value) => { input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); };
  const choose = (select, value) => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); };
  const key = (target, init) => target.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }));
  const click = (target, init = {}) => target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...init }));
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const sourceDialog = () => document.querySelector('dialog.tag-source-dialog[open]:not(.ts-token-dialog)');
  const closeSource = async d => { d.querySelector('[data-close]').click(); await waitFor(() => !sourceDialog(), 5000, 'dialog closed'); };
  try {
    if (!window.OPENTAG_E2E_OFFLINE) await report('note: suite is offline by design; no network steps');
    const rows = await waitFor(() => { const r = [...document.querySelectorAll('tr[data-path]')]; return r.length === 2 && r; }, 15000, 'rows');
    const paths = rows.map(r => r.dataset.path), album = paths[0].replace(/[\\/][^\\/]+$/, ''), freedb = album.replace(/[\\/][^\\/]+$/, '/freedb');
    await check(paths.every(p => p.endsWith('.wav')), 'album holds the two WAV fixtures');

    // Options → Tag Sources → freedb: the named server list.
    op('source-options').click();
    let options = await waitFor(() => document.querySelector('dialog.options-dialog[open]'), 5000, 'options');
    options.querySelector('[data-page="tag-sources-freedb"]').click();
    const names = () => [...options.querySelectorAll('#freedb-server option')].map(o => o.textContent);
    await check(names().join('|') === 'gnudb (HTTPS)|gnudb (HTTP)', 'default server list ' + names().join('|'));
    choose(options.querySelector('#freedb-server'), '1');
    await check(options.querySelector('#freedb-server-protocol').value === 'http' && document.activeElement?.id === 'freedb-server', 'choosing gnudb (HTTP) switches the protocol');
    options.querySelector('#freedb-server-new').click();
    await check(document.activeElement?.id === 'freedb-server-name' && names().length === 3, 'New adds a server and focuses its name');
    type(options.querySelector('#freedb-server-name'), 'Example server');
    type(options.querySelector('#freedb-server-address'), 'cddb.example.org');
    await check(names()[2] === 'Example server' && options.querySelector('[data-option="pages.tag-sources-freedb.address"]').value === 'cddb.example.org', 'name and address edit the selected server');
    options.querySelector('#freedb-server-delete').click();
    await check(names().length === 2 && options.querySelector('#freedb-server').value === '1', 'Delete removes it');
    options.querySelector('#freedb-server-new').click();
    type(options.querySelector('#freedb-server-name'), 'Kept server');
    type(options.querySelector('#freedb-server-address'), 'cddb.example.net');
    options.querySelector('#freedb-server-list').click();
    await check(names().join('|') === 'gnudb (HTTPS)|gnudb (HTTP)' && options.querySelector('#freedb-server-address').value === 'gnudb.gnudb.org', 'Current list of freedb servers restores the list');
    options.querySelector('#freedb-server-new').click();
    type(options.querySelector('#freedb-server-name'), 'Kept server');
    type(options.querySelector('#freedb-server-address'), 'cddb.example.net');

    // Local freedb: folder and text index.
    options.querySelector('[data-page="tag-sources-local-freedb"]').click();
    const indexLine = () => options.querySelector('#local-freedb-index-status').textContent;
    await waitFor(() => /No text index/.test(indexLine()), 5000, 'index status');
    await check(true, 'no text index yet: ' + indexLine());
    type(options.querySelector('#local-freedb-directory'), freedb);
    options.querySelector('#local-freedb-index-build').click();
    await waitFor(() => /Text index: 3 discs from this folder/.test(indexLine()), 20000, 'index built');
    await check(true, 'index built: ' + indexLine());
    options.querySelector('#options-ok').click();
    await waitFor(() => !options.open, 5000, 'options saved');
    const pages = (await inv('load_settings')).settings.pages, server = pages['tag-sources-freedb'];
    await check(server.servers?.length === 3 && server.server === 2 && server.address === 'cddb.example.net' && server.servers[2].name === 'Kept server', 'server list saved with the chosen server in use');
    await check(pages['tag-sources-local-freedb']?.directory === freedb, 'local folder saved');
    const status = await inv('local_freedb_index_status');
    await check(status.discs === 3 && status.directory === freedb, 'index status ' + JSON.stringify(status));

    // Tag Sources → Discogs Release ID asks for the ID only.
    op('select-all').click();
    await waitFor(() => document.querySelectorAll('tr[data-path].selected,tr[data-path][aria-selected="true"]').length === 2, 5000, 'selection');
    op('source-discogs-release').click();
    let d = await waitFor(sourceDialog, 5000, 'Discogs Release ID dialog');
    await check(d.getAttribute('aria-label') === 'Discogs Release ID' && d.querySelector('#ts-mode').closest('label').hidden && !d.querySelector('#ts-text-field').hidden && d.querySelector('#ts-search').textContent === 'Look up' && document.activeElement?.id === 'ts-text', 'Discogs Release ID opens on the ID field');
    await closeSource(d);
    await check(/I/.test(op('source-discogs-release').querySelector('small')?.textContent || ''), 'menu shows Ctrl+I next to the last used source');
    key(document, { key: 'i', metaKey: true });
    d = await waitFor(sourceDialog, 5000, 'Ctrl+I dialog');
    await check(d.getAttribute('aria-label') === 'Discogs Release ID', 'Ctrl+I repeats Discogs Release ID');
    await closeSource(d);

    // freedb (local): text search through the index.
    op('source-local-freedb').click();
    d = await waitFor(sourceDialog, 5000, 'freedb (local)');
    const modes = [...d.querySelectorAll('#ts-mode option')].map(o => o.value);
    await check(modes.includes('text') && modes.includes('toc') && modes.includes('disc_id'), 'local modes ' + modes.join(','));
    choose(d.querySelector('#ts-mode'), 'text');
    d.querySelector('#ts-text').value = 'cafe night';
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 10000, 'text results');
    const found = [...d.querySelectorAll('.ts-results tbody tr')].map(r => r.textContent);
    await check(found.length === 1 && found[0].includes('Café Quartet') && found[0].includes('jazz') && found[0].includes('10000001'), 'text search finds the bundle disc: ' + found.join(' / '));
    d.querySelector('#ts-next').click();
    const apply = await waitFor(() => d.querySelector('#ts-apply'), 10000, 'review');

    // Field multi-select.
    const row = name => d.querySelector(`.ts-fields tr[data-row="${name}"]`);
    const box = name => row(name).querySelector('input[type=checkbox]');
    const selectedNames = () => [...d.querySelectorAll('.ts-fields tr[data-row].selected')].map(r => r.dataset.row);
    const order = [...d.querySelectorAll('.ts-fields tr[data-row]')].map(r => r.dataset.row);
    await report('fields ' + order.join(','));
    click(row('TITLE').querySelector('th'));
    click(row('ARTIST').querySelector('th'), { ctrlKey: true });
    await check(selectedNames().join() === 'TITLE,ARTIST' && row('ARTIST').getAttribute('aria-selected') === 'true', 'click and Ctrl+click select two fields');
    box('ARTIST').click();
    await check(!box('ARTIST').checked && !box('TITLE').checked && box('ALBUM').checked, 'a checkbox click applies to every selected field');
    click(row('YEAR').querySelector('th'), { shiftKey: true });
    const range = order.slice(order.indexOf('ARTIST'), order.indexOf('YEAR') + 1);
    await check(selectedNames().join() === range.join(), 'Shift+click selects the range ' + selectedNames().join());
    let reached = false; const spy = e => { if (e.key === 'a') reached = true; }; document.addEventListener('keydown', spy);
    key(row('YEAR'), { key: 'a', metaKey: true });
    await check(selectedNames().length === order.length && !reached, 'Ctrl+A selects every field and stays in the dialog');
    key(row('YEAR'), { key: 'a', metaKey: true, shiftKey: true });
    document.removeEventListener('keydown', spy);
    await check(selectedNames().length === 0, 'Ctrl+Shift+A inverts the selection');
    click(row('GENRE').querySelector('th'));
    const genre = box('GENRE').checked;
    key(row('GENRE'), { key: ' ' });
    await check(box('GENRE').checked === !genre && document.activeElement === row('GENRE'), 'Space toggles the selected field');
    key(d.querySelector('.ts-fields'), { key: 'e', metaKey: true });
    await check(order.every(name => box(name).checked), 'Ctrl+E enables every field');

    // Suggest sorting is a remembered checkbox; drag and drop and Alt+Page Down reorder.
    d.querySelector('#ts-suggest-length').click();
    await check(localStorage.getItem('opentag.tagSources.suggestSorting') === 'length' && !d.querySelector('#ts-suggest-title').checked, 'Suggest sorting: Length is remembered');
    const fileAt = i => d.querySelectorAll('.ts-matches tbody tr')[i].children[3].textContent;
    const before = [fileAt(0), fileAt(1)];
    const drag = (type, target) => { let event; try { event = new DragEvent(type, { bubbles: true, cancelable: true }); } catch { event = new Event(type, { bubbles: true, cancelable: true }); } target.dispatchEvent(event); };
    const matchRows = () => d.querySelectorAll('.ts-matches tbody tr');
    await check(matchRows()[0].getAttribute('draggable') === 'true', 'matched files are draggable');
    drag('dragstart', matchRows()[0]); drag('dragover', matchRows()[1]); drag('drop', matchRows()[1]); drag('dragend', d.querySelector('.ts-match-area'));
    await check(fileAt(0) === before[1] && fileAt(1) === before[0], `drag and drop moves ${before[0]} to the second track`);
    const down = matchRows()[0].querySelector('button[data-move="1"]'); down.focus();
    key(down, { key: 'PageDown', altKey: true });
    await check(fileAt(0) === before[0] && fileAt(1) === before[1], 'Alt+Page Down moves the first file to the end');
    apply.click();
    await waitFor(() => !d.open, 20000, 'apply');
    const disk = await Promise.all(paths.map(async p => (await inv('scan_files', { paths: [p], recursive: false })).entries[0].fields));
    const value = (f, k) => (f[k] || f[k.toLowerCase()] || []).join(' | ');
    await report('disk ' + disk.map(f => ['TITLE', 'ARTIST', 'ALBUM', 'YEAR', 'GENRE'].map(k => `${k}=${value(f, k)}`).join(' ')).join(' / '));
    await check(disk.every(f => value(f, 'ALBUM') === 'Night Set' && value(f, 'ARTIST') === 'Café Quartet'), 'album and artist written');
    await check(disk.map(f => value(f, 'TITLE')).sort().join() === 'Early Tune,Late Tune', 'both track titles written');
    await check((await inv('history_state')).undo?.label === 'Tag source import', 'one undo step');

    // Ctrl+I now repeats freedb (local); Ctrl+Shift+I searches the files' disc ID at once.
    key(document, { key: 'i', metaKey: true });
    d = await waitFor(sourceDialog, 5000, 'Ctrl+I dialog');
    await check(d.getAttribute('aria-label') === 'freedb (local)' && !d.querySelector('.ts-results'), 'Ctrl+I repeats freedb (local) with the search step');
    await closeSource(d);
    key(document, { key: 'I', metaKey: true, shiftKey: true });
    d = await waitFor(sourceDialog, 5000, 'Ctrl+Shift+I dialog');
    await waitFor(() => d.querySelector('#ts-apply'), 10000, 'quick review').catch(error => { throw new Error(`${error.message}: ${d.querySelector('.ts-status')?.textContent} (${d.querySelector('#ts-toc')?.textContent})`); });
    await check(d.querySelector('header').textContent.includes('Quick Artist') && d.querySelector('#ts-suggest-length').checked, 'Ctrl+Shift+I opens the single disc 07000602 directly; Suggest sorting stays checked');
    await closeSource(d);

    // Discogs token: kept in the keychain on request and moved back.
    const tokenDialog = async () => { op('source-token').click(); return waitFor(() => document.querySelector('dialog.ts-token-dialog[open]'), 5000, 'token dialog'); };
    let t = await tokenDialog();
    const keychain = t.querySelector('#ts-token-keychain');
    await check(!!keychain && !keychain.checked, 'keychain option offered, off by default');
    type(t.querySelector('#ts-token'), 'e2eToken123');
    keychain.click();
    t.querySelector('#ts-save-token').click();
    await waitFor(() => !t.open, 5000, 'token saved');
    let saved = await inv('load_tag_source_settings');
    await check(saved.discogs_token_set && saved.discogs_token_in_keychain, 'token kept in the keychain ' + JSON.stringify(saved));
    t = await tokenDialog();
    await check(/system keychain/.test(t.querySelector('.ts-token-place').textContent) && t.querySelector('#ts-save-token').disabled, 'dialog says where the token is');
    t.querySelector('#ts-token-keychain').click();
    await check(!t.querySelector('#ts-save-token').disabled, 'moving the token enables Save');
    t.querySelector('#ts-save-token').click();
    await waitFor(() => !t.open, 5000, 'token moved');
    saved = await inv('load_tag_source_settings');
    await check(saved.discogs_token_set && !saved.discogs_token_in_keychain, 'token moved back to the private file');
    t = await tokenDialog();
    t.querySelector('#ts-token-clear').click();
    t.querySelector('#ts-save-token').click();
    await waitFor(() => !t.open, 5000, 'token removed');
    await check(!(await inv('load_tag_source_settings')).discogs_token_set, 'token removed');
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await report('SUMMARY done');
  await inv('e2e_exit', { code: 0 });
})();
