// User-installed web sources (.src) end to end: install through the backend
// command the Install menu item uses, the Tag Sources submenu with access
// keys, the generated settings dialog, search → review → import against the
// local fixture site, direct lookups, Ctrl+I and script error reporting with
// debug output. Offline: tests/e2e/web-sources-fixtures.sh serves the site on
// 127.0.0.1 and writes the test sources next to the album.
(async () => {
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  const check = async (ok, what) => report(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  const key = (k, init = {}) => document.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, ...init }));
  const tagSourcesMenu = () => document.querySelector('#tag-sources-menu');
  const openMenu = async () => { const menu = tagSourcesMenu(); menu.querySelector('summary').click(); await waitFor(() => menu.open, 3000, 'Tag Sources menu'); return menu; };
  try {
    await check(window.OPENTAG_E2E_OFFLINE === true, 'offline run (web sources may only reach 127.0.0.1)');
    const rows = await waitFor(() => { const r = [...document.querySelectorAll('tr[data-path]')]; return r.length === 2 && r; }, 15000, 'rows');
    const paths = rows.map(r => r.dataset.path), album = paths[0].replace(/[\\/][^\\/]+$/, ''), folder = album.replace(/([\\/])[^\\/]+$/, '$1web-sources');

    // Install the test sources (Tag Sources → Install tag sources… copies the chosen files).
    const files = ['&Fixture#&Web Test.src', '&Fixture#&Direct lookup.src', '&Fixture#Broken.src', 'Fixture.inc', '&Fixture#Settings.settings'];
    const listing = await inv('web_sources_install', { paths: files.map(f => `${folder}/${f}`) });
    await check(listing.sources.map(s => s.menu).join(',') === '&Fixture#Broken,&Fixture#&Direct lookup,&Fixture#&Web Test' && !listing.problems.length, 'installed sources ' + listing.sources.map(s => s.menu).join(',') + ' problems=' + JSON.stringify(listing.problems));
    await check(/data[/\\]sources$/.test(listing.folder), 'sources folder ' + listing.folder);
    await waitFor(() => document.querySelector('[data-web-source]'), 5000, 'menu rebuilt after install');

    // The menu: one "Fixture" submenu with access keys, the settings entry after the sources.
    let menu = await openMenu();
    const submenu = menu.querySelector('.web-submenu');
    const summary = submenu?.querySelector('summary');
    await check(summary?.innerHTML === '<u>F</u>ixture', 'submenu with access key ' + summary?.innerHTML);
    const items = [...submenu.querySelectorAll('[data-web-source]')].map(b => b.querySelector('span').innerHTML);
    await check(items.join('|') === 'Broken…|<u>D</u>irect lookup…|<u>W</u>eb Test…', 'source items ' + items.join('|'));
    const order = [...menu.querySelectorAll('.menu-list > button, .menu-list > details')].map(e => e.dataset.op || (e.querySelector('[data-web-settings]') ? 'settings' : 'sources'));
    await check(order.indexOf('sources') > order.indexOf('source-cover-discogs') && order.indexOf('settings') > order.indexOf('sources') && order.indexOf('source-install') > order.indexOf('settings'), 'menu order ' + order.join(','));

    // Settings dialog generated from the .settings schema.
    menu.querySelector('[data-web-settings]').click();
    const settings = await waitFor(() => document.querySelector('dialog.web-settings-dialog[open]'), 5000, 'settings dialog');
    await check(settings.querySelector('header').textContent === 'Fixture Settings' && settings.querySelector('.ws-heading')?.textContent === 'Import', 'settings dialog title and heading');
    const comment = settings.querySelector('input[type=checkbox][data-setting]');
    await check(comment && !comment.checked && settings.querySelector('select[data-setting]')?.value === 'en', 'settings show their defaults');
    comment.click();
    settings.querySelector('#ws-save').click();
    await waitFor(() => !settings.open, 5000, 'settings saved');
    const stored = await inv('web_source_settings', { id: '&Fixture#Settings.settings' });
    await check(stored.values.addComment === true && stored.values.language === 'en', 'settings stored ' + JSON.stringify(stored.values));

    // Select the album and open the source with its access keys: F, then W.
    document.querySelector('button[data-op="select-all"]').click();
    await waitFor(() => document.querySelectorAll('tr[data-path].selected,tr[data-path][aria-selected="true"]').length === 2, 5000, 'selection');
    menu = await openMenu();
    key('f');
    await waitFor(() => menu.querySelector('.web-submenu').open, 3000, 'F opens the Fixture submenu');
    key('w');
    let d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 8000, 'web source dialog');
    await check(d.getAttribute('aria-label') === 'Fixture › Web Test', 'access keys open ' + d.getAttribute('aria-label'));
    const artist = d.querySelector('#ts-web-0'), albumField = d.querySelector('#ts-web-1');
    await check(artist?.value === 'Test Artist' && albumField?.value === 'Test Album', `search fields from [SearchBy] formats: ${artist?.value} / ${albumField?.value}`);
    await check(d.querySelector('.ts-web-search').textContent.includes('ARTIST') && d.querySelector('.ts-web-search').textContent.includes('ALBUM'), 'search field labels');
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length === 2, 10000, 'search results');
    const head = [...d.querySelectorAll('.ts-results th')].map(th => th.textContent);
    const first = [...d.querySelectorAll('.ts-results tbody tr')[0].children].map(td => td.textContent);
    await check(head.join(',') === 'Release,Year' && first.join(',') === 'Fixture Artist - Fixture Album,1999', `result columns ${head} / ${first}`);
    await waitFor(() => { const img = d.querySelector('.ts-thumb'); return img && !img.hidden && img.src.startsWith('data:image/png'); }, 8000, 'result cover thumbnail');
    await check(true, 'cover thumbnail from %_coverurl% loaded');
    await check(d.querySelector('.ts-chosen .ts-preview')?.dataset.url === 'http://127.0.0.1:18765/album/7.json', 'preview button for the album page');
    d.querySelector('#ts-next').click();
    const apply = await waitFor(() => d.querySelector('#ts-apply'), 10000, 'review');
    const fields = [...d.querySelectorAll('.ts-fields tr[data-row]')].map(r => r.dataset.row);
    await check(['TITLE', 'ARTIST', 'ALBUM', 'YEAR', 'TRACK', 'GENRE', 'COMMENT'].every(f => fields.includes(f)) && !fields.some(f => f.startsWith('_')), 'review fields ' + fields.join(','));
    await check(d.querySelector('.ts-value[data-value="ALBUM"]').value === 'Fixture Album', 'album value');
    const tracks = [...d.querySelectorAll('.ts-track-value')].map(i => i.value);
    await check(tracks.join('|') === 'Web First|Web Second', 'tracks ' + tracks.join('|'));
    await waitFor(() => { const img = d.querySelector('.ts-cover-preview img'); return img && !img.hidden && img.src.startsWith('data:image/png'); }, 8000, 'cover preview');
    await check(d.querySelector('#ts-cover-write')?.checked, 'cover is written by default');
    apply.click();
    await waitFor(() => !d.open, 20000, 'import');
    const disk = await Promise.all(paths.map(async p => (await inv('scan_files', { paths: [p], recursive: false })).entries[0]));
    const value = (e, k) => (e.fields[k] || e.fields[k.toLowerCase()] || []).join(' | ');
    await report('disk ' + disk.map(e => ['TITLE', 'ALBUM', 'YEAR', 'GENRE', 'COMMENT', 'TRACK'].map(k => `${k}=${value(e, k)}`).join(' ') + ` covers=${e.covers.length}`).join(' / '));
    await check(disk.every(e => value(e, 'ALBUM') === 'Fixture Album' && value(e, 'GENRE') === 'Electronic' && value(e, 'COMMENT') === 'Imported from the fixture site'), 'album fields and the IfVar comment written');
    await check(value(disk[0], 'TITLE') === 'Web First' && value(disk[1], 'TITLE') === 'Web Second', 'track titles written by track number');
    await check(disk.every(e => e.covers.length === 1), 'cover from COVERURL embedded');
    const history = await inv('history_state');
    await check(history.undo?.label === 'Tag source import' && history.undo?.files === 2, 'undo step ' + JSON.stringify(history.undo));

    // Ctrl+I repeats the last tag source, now this web source.
    key('i', { ctrlKey: true });
    d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 8000, 'Ctrl+I');
    await check(d.getAttribute('aria-label') === 'Fixture › Web Test', 'Ctrl+I reopens ' + d.getAttribute('aria-label'));
    d.querySelector('[data-close]').click();
    await waitFor(() => !document.querySelector('dialog.tag-source-dialog'), 5000, 'closed');

    // A source without an index step goes straight to the album page.
    menu = await openMenu();
    [...menu.querySelectorAll('[data-web-source]')].find(b => b.textContent.startsWith('Direct')).click();
    d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 8000, 'direct dialog');
    await check(d.querySelector('#ts-web-0')?.value === '7', 'direct lookup field from a literal format');
    d.querySelector('#ts-search').click();
    await waitFor(() => d.querySelector('#ts-apply'), 10000, 'direct review');
    await check(d.querySelector('header').textContent.includes('Fixture Album'), 'direct lookup opens the album: ' + d.querySelector('header').textContent);
    d.querySelector('[data-close]').click();
    await waitFor(() => !document.querySelector('dialog.tag-source-dialog'), 5000, 'closed');

    // Options → Tag Sources → debug output; a failing script names file, script, line and the debug folder.
    document.querySelector('button[data-op="source-options"]').click();
    const options = await waitFor(() => document.querySelector('dialog.options-dialog[open]'), 5000, 'options');
    options.querySelector('[data-option="pages.tag-sources.web_debug"]').click();
    options.querySelector('#options-ok').click();
    await waitFor(() => !options.open, 5000, 'options saved');
    menu = await openMenu();
    [...menu.querySelectorAll('[data-web-source]')].find(b => b.textContent.startsWith('Broken')).click();
    d = await waitFor(() => document.querySelector('dialog.tag-source-dialog[open]'), 8000, 'broken dialog');
    d.querySelector('#ts-search').click();
    const failure = await waitFor(() => d.querySelector('.dialog-error')?.textContent, 10000, 'script error');
    await report('error ' + failure);
    await check(failure.startsWith('&Fixture#Broken.src [ParserScriptIndex] line 6 (FindLine "this text is not on the page"): "this text is not on the page" was not found') && /Debug output is in .*sources[/\\]debug/.test(failure), 'script error names the line and the debug folder');
    d.querySelector('[data-close]').click();
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await report('SUMMARY done');
  await inv('e2e_exit', { code: 0 });
})();
