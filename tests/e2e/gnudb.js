// Tag Sources → freedb against a gnudb-like CDDB server, end to end: the
// email option and its validation, the handshake (developer contact or the
// user's address), lookups by track layout and by artist and album, reading
// only listed entries, cover art from entry lines and artsearch, Covers:
// freedb, 429 without retries, and no lookups by a typed disc ID. Offline:
// tests/e2e/gnudb-fixtures.sh runs the mock server on 127.0.0.1:18767, which
// refuses requests that break gnudb's policy.
(async () => {
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  const check = async (ok, what) => report(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  const type = (input, value) => { input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); };
  const choose = (select, value) => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); };
  const developer = /^hello: john neerdael\.nl OpenTagAudioTagEditor v\d+\.\d+\.\d+/;

  const openOptions = async () => {
    document.querySelector('button[data-op="source-options"]').click();
    const options = await waitFor(() => document.querySelector('dialog.options-dialog[open]'), 5000, 'options');
    options.querySelector('[data-page="tag-sources-freedb"]').click();
    await waitFor(() => options.querySelector('[data-option="pages.tag-sources-freedb.email"]'), 5000, 'freedb page');
    return options;
  };
  const saveOptions = async options => { options.querySelector('#options-ok').click(); await waitFor(() => !options.open, 5000, 'options saved'); };
  const openSource = async (op, label) => {
    document.querySelector(`button[data-op="${op}"]`).click();
    const d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 5000, label);
    await check(d.getAttribute('aria-label') === label, `${op} opens ${label}`);
    return d;
  };
  // Searching sets the status at once; the answer replaces it (rows of an earlier search may still be shown).
  const search = async d => {
    d.querySelector('#ts-search').click();
    await waitFor(() => !/Searching/.test(d.querySelector('.ts-status')?.textContent || ''), 10000, 'search answer');
    return [...d.querySelectorAll('.ts-results tbody tr')].map(r => r.textContent);
  };
  const review = async d => { d.querySelector('#ts-next').click(); return waitFor(() => d.querySelector('#ts-apply') || d.querySelector('.dialog-error'), 10000, 'review'); };
  // The dialog's close event (queued after close()) ends the session and removes it.
  const close = async d => { d.querySelector('[data-close]').click(); await waitFor(() => !d.isConnected, 5000, 'dialog closed'); };
  const coverCount = d => d.querySelectorAll('#ts-cover-choice option').length;

  try {
    await check(window.OPENTAG_E2E_OFFLINE === true, 'offline run (freedb may only reach 127.0.0.1)');
    const rows = await waitFor(() => { const r = [...document.querySelectorAll('tr[data-path]')]; return r.length === 2 && r; }, 15000, 'rows');
    const paths = rows.map(r => r.dataset.path);

    // Options → Tag Sources → freedb: the email field, its validation and the mock server.
    let options = await openOptions();
    const email = options.querySelector('[data-option="pages.tag-sources-freedb.email"]');
    const label = email.closest('label').querySelector('span').textContent;
    await check(/Your email address \(sent to the freedb server\)/i.test(label), 'email field label: ' + label);
    const page = options.querySelector('.options-page').textContent;
    await check(page.includes('john@neerdael.nl') && page.includes('OpenTagAudioTagEditor') && page.includes('never submits'), 'the page explains the developer contact, the program name and that nothing is submitted');
    type(email, 'me@localhost');
    options.querySelector('#options-ok').click();
    const invalid = await waitFor(() => options.querySelector('.dialog-error')?.textContent, 5000, 'email validation');
    await check(options.open && /email address/.test(invalid), 'an invalid email address is refused: ' + invalid);
    // The refused page is shown again: its inputs are new elements.
    type(options.querySelector('[data-option="pages.tag-sources-freedb.email"]'), '');
    options.querySelector('#freedb-server-new').click();
    type(options.querySelector('#freedb-server-name'), 'Mock gnudb');
    choose(options.querySelector('#freedb-server-protocol'), 'http');
    type(options.querySelector('#freedb-server-address'), '127.0.0.1');
    type(options.querySelector('#freedb-server-port'), '18767');
    type(options.querySelector('#freedb-server-path'), '/~cddb/cddb.cgi');
    await saveOptions(options);
    let saved = (await inv('load_settings')).settings.pages['tag-sources-freedb'];
    await check(saved.address === '127.0.0.1' && saved.port === '18767' && saved.protocol === 'http' && !saved.email, 'mock server selected, no email saved');

    // Track layout lookup: cddb query, then cddb read of the listed entry.
    document.querySelector('button[data-op="select-all"]').click();
    await waitFor(() => document.querySelectorAll('tr[data-path].selected,tr[data-path][aria-selected="true"]').length === 2, 5000, 'selection');
    let d = await openSource('source-freedb', 'freedb');
    const modes = [...d.querySelectorAll('#ts-mode option')].map(o => o.value);
    await check(modes.join(',') === 'toc,artist_album', 'search types are the track layout and artist and album, no typed disc ID: ' + modes);
    await check(/local freedb folder/.test(d.querySelector('.ts-status').textContent), 'the dialog explains that a typed disc ID needs a local folder');
    let found = await search(d);
    await check(found.length === 1 && found[0].includes('Layout Album') && found[0].includes('1a2b3c4d') && d.querySelector('.ts-results thead').textContent.includes('Entry ID'), 'query lists the gnudb entry: ' + found.join(' | '));
    await review(d);
    const comment = d.querySelector('.ts-value[data-value="COMMENT"]')?.value || '';
    await check(developer.test(comment), 'handshake without an email sends the developer contact: ' + comment);
    await waitFor(() => coverCount(d) === 5, 10000, 'artsearch covers');
    await check(coverCount(d) === 5 && d.querySelector('#ts-cover-choice').textContent.includes('release 16efdbf1'), 'entry without cover lines offers artsearch covers: ' + [...d.querySelectorAll('#ts-cover-choice option')].map(o => o.textContent).join(' | '));
    d.querySelector('#ts-back').click();

    // Artist and album search, then read and write the chosen result.
    choose(await waitFor(() => d.querySelector('#ts-mode'), 5000, 'search again'), 'artist_album');
    type(d.querySelector('#ts-artist'), 'Fixture Artist'); type(d.querySelector('#ts-album'), 'Text Album');
    found = await search(d);
    await check(found.length === 2 && found[0].includes('Text Album') && found[0].includes('rock') && found[0].includes('0f00aa02'), 'text search with the selection\'s track count: ' + found.join(' | '));
    const apply = await review(d);
    await check(coverCount(d) === 2 && d.querySelector('#ts-cover-choice option').textContent.includes('release 8763f781'), 'cover lines of the entry become the artwork choices');
    const write = d.querySelector('#ts-cover-write');
    if (write) { write.checked = false; write.dispatchEvent(new Event('change', { bubbles: true })); }
    apply.click();
    await waitFor(() => !d.open, 20000, 'apply');
    const disk = await Promise.all(paths.map(async p => (await inv('scan_files', { paths: [p], recursive: false })).entries[0].fields));
    const value = (f, k) => (f[k] || f[k.toLowerCase()] || []).join(' | ');
    await check(disk.every(f => value(f, 'ALBUM') === 'Text Album' && value(f, 'ARTIST') === 'Fixture Artist'), 'text search result written: ' + disk.map(f => value(f, 'ALBUM')).join(','));

    // The user's own email address goes into the handshake.
    options = await openOptions();
    type(options.querySelector('[data-option="pages.tag-sources-freedb.email"]'), 'tester@example.org');
    await saveOptions(options);
    d = await openSource('source-freedb', 'freedb');
    await search(d);
    await review(d);
    const own = d.querySelector('.ts-value[data-value="COMMENT"]')?.value || '';
    await check(/^hello: tester example\.org OpenTagAudioTagEditor v\d/.test(own), 'handshake with the user\'s email address: ' + own);
    d.querySelector('#ts-back').click();

    // 429 stops the lookup and is not retried (the mock would answer a retry).
    choose(await waitFor(() => d.querySelector('#ts-mode'), 5000, 'search again'), 'artist_album');
    type(d.querySelector('#ts-artist'), 'Busy'); type(d.querySelector('#ts-album'), 'Anything');
    d.querySelector('#ts-search').click();
    const busy = await waitFor(() => d.querySelector('.dialog-error')?.textContent, 10000, '429 error');
    await sleep(2000);
    const retried = [...d.querySelectorAll('.ts-results tbody tr')].some(r => r.textContent.includes('Retried'));
    await check(/429/.test(busy) && /try again later/.test(busy) && !retried, '429 Too Many Requests is reported and not retried: ' + busy);
    await close(d);

    // Covers: freedb asks gnudb's cover art search without reading an entry.
    d = await openSource('source-cover-freedb', 'freedb cover art');
    await check([...d.querySelectorAll('#ts-mode option')].map(o => o.value).join(',') === 'toc,artist_album', 'covers-only search types');
    choose(d.querySelector('#ts-mode'), 'artist_album');
    type(d.querySelector('#ts-artist'), 'Katie Melua'); type(d.querySelector('#ts-album'), 'Piece by Piece');
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelector('#ts-apply'), 10000, 'covers review');
    await check(coverCount(d) === 5 && !d.querySelector('.ts-fields') && /artwork/.test(d.querySelector('.ts-status').textContent), 'covers-only review lists the artsearch images');
    await close(d);

    // The backend refuses reads of entries no query listed and typed disc IDs.
    const unlisted = await inv('tag_source_release', { provider: 'freedb', input: 'rock/deadbeef', ticket: 1e9 }).then(() => 'read', e => String(e));
    await check(/Look the disc up first/.test(unlisted), 'an entry no query listed is never read: ' + unlisted);
    const typed = await inv('tag_source_search', { provider: 'freedb', query: { mode: 'disc_id', text: '0a000802', lengths: [] }, ticket: 1e9 + 1 }).then(() => 'searched', e => String(e));
    await check(/local freedb folder/.test(typed), 'a typed disc ID is not sent to the server: ' + typed);
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await report('SUMMARY done');
  await inv('e2e_exit', { code: 0 });
})();
