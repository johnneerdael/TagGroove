// Tag sources round 4, offline: LRCLIB lyrics for the selected files, iTunes
// and Deezer imports (fields, high-resolution cover, Ctrl+I), the key-gated
// sources (AcoustID, Last.fm, fanart.tv) disabled until Tag Sources → API
// keys… holds a key, then a Last.fm import and fanart.tv covers, the AcoustID
// fpcalc message, an AcoustID import paired by fingerprint (stand-in fpcalc),
// and the Discogs token text of a build without TagGroove's
// application key. Every service answer comes from
// tests/e2e/sources-4-fixtures.sh on 127.0.0.1:18766.
(async () => {
  const sourceName = dialog => dialog.hasAttribute('aria-labelledby')
    ? dialog.ownerDocument.getElementById(dialog.getAttribute('aria-labelledby'))?.textContent
    : dialog.getAttribute('aria-label');
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a), report = m => inv('e2e_report', { message: String(m) }), sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms, what) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch {} await sleep(150); } throw new Error('timeout: ' + what); };
  const check = async (ok, what) => report(`${ok ? 'PASS' : 'FAIL'} ${what}`);
  const type = (input, value) => { input.value = value; input.dispatchEvent(new Event('input', { bubbles: true })); };
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const sourceDialog = () => document.querySelector('dialog.tag-source-dialog[open]:not(.ts-token-dialog)');
  const closeSource = async d => { d.querySelector('[data-close]').click(); await waitFor(() => !d.isConnected, 5000, 'dialog closed'); };
  const failure = d => d.querySelector('.dialog-error')?.textContent || d.querySelector('.ts-status')?.textContent || '';
  const fields = async paths => Promise.all(paths.map(async p => (await inv('scan_files', { paths: [p], recursive: false })).entries[0]));
  const value = (entry, name) => (entry.fields[name] || []).join(' | ');
  /** Open a source from the menu, search and wait for the review (or the result list). */
  const search = async (name, label) => {
    op(name).click();
    const d = await waitFor(sourceDialog, 8000, `${name} dialog`);
    await check(sourceName(d) === label, `${name} opens "${sourceName(d)}"`);
    d.querySelector('#ts-search').click();
    return d;
  };
  const review = async (d, what) => waitFor(() => d.querySelector('#ts-apply'), 15000, what).catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
  /** Save the review; files that already have artwork get the imported cover in place of it. */
  const apply = async (d, what) => {
    (await review(d, what)).click();
    await waitFor(() => { document.querySelector('dialog.import-covers-dialog[open] #covers-replace')?.click(); return !d.isConnected; }, 20000, `${what} written`).catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
  };
  try {
    await check(window.OPENTAG_E2E_OFFLINE === true, 'offline run: sources answer from 127.0.0.1:18766');
    const rows = await waitFor(() => { const r = [...document.querySelectorAll('tr[data-path]')]; return r.length === 2 && r; }, 15000, 'rows');
    const paths = rows.map(r => r.dataset.path);
    op('select-all').click();
    await waitFor(() => document.querySelectorAll('tr[data-path].selected,tr[data-path][aria-selected="true"]').length === 2, 5000, 'selection');

    // Key-gated sources wait for a key; the others are ready.
    const gated = ['source-acoustid', 'source-lastfm', 'source-cover-fanart'];
    await waitFor(() => gated.every(name => op(name)?.disabled), 8000, 'key-gated sources disabled');
    await check(gated.every(name => /needs an API key/.test(op(name).title)), 'disabled with the reason: ' + op('source-lastfm').title);
    await check(['source-itunes', 'source-deezer', 'source-lrclib', 'source-cover-itunes', 'source-cover-deezer'].every(name => op(name) && !op(name).disabled), 'iTunes, Deezer and LRCLIB need no key');

    // A build without TagGroove's Discogs application key keeps the token guidance.
    op('source-token').click();
    let token = await waitFor(() => document.querySelector('dialog.ts-token-dialog[open]'), 5000, 'token dialog');
    await check(/Create a token on Discogs/.test(token.textContent) && !/works without a token/.test(token.textContent), 'Discogs token text without the application key');
    token.querySelector('[data-close]').click();
    await waitFor(() => !token.isConnected, 5000, 'token dialog closed');

    // LRCLIB: each selected file by its own signature; the review opens at once.
    let d = await search('source-lrclib', 'LRCLIB lyrics');
    await check(d.querySelector('#ts-mode').value === 'files', 'LRCLIB looks up the selected files by default');
    await review(d, 'lyrics review');
    const lyricFields = [...d.querySelectorAll('.ts-fields tr[data-row]')].map(r => r.dataset.row).join(',');
    await check(lyricFields === 'UNSYNCEDLYRICS', 'lyrics review offers UNSYNCEDLYRICS only: ' + lyricFields);
    const lyricTracks = [...d.querySelectorAll('.ts-matches tbody tr')].map(r => r.children[3].textContent);
    await check(lyricTracks.length === 2 && lyricTracks[0].includes('sine-a') && lyricTracks[1].includes('sine-b'), 'one track per file, in file list order: ' + lyricTracks.join(' / '));
    await apply(d, 'lyrics');
    let disk = await fields(paths);
    await check(/Fixture lyric line one\nFixture lyric line two$/.test(value(disk[0], 'UNSYNCEDLYRICS')) && !/\[00:/.test(value(disk[0], 'UNSYNCEDLYRICS')), 'plain lyrics written to the first file: ' + JSON.stringify(value(disk[0], 'UNSYNCEDLYRICS')));
    await check(!value(disk[1], 'UNSYNCEDLYRICS'), 'the file without lyrics is unchanged');
    await check((await inv('history_state')).undo?.label === 'Tag source import', 'lyrics import is one undo step');

    // iTunes: search, review with the songs and high-resolution artwork, import.
    d = await search('source-itunes', 'iTunes');
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length === 1, 15000, 'iTunes results').catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
    const head = [...d.querySelectorAll('.ts-results th')].map(th => th.textContent).join(',');
    await check(head === 'Artist,Title,Year,Country,Type,Tracks', 'iTunes columns ' + head);
    await waitFor(() => { const img = d.querySelector('.ts-thumb'); return img && !img.hidden && img.src.startsWith('data:image/png'); }, 10000, 'iTunes thumbnail');
    await check(d.querySelector('.ts-chosen .ts-preview')?.dataset.url === 'https://music.apple.com/us/album/fixture/900001', 'Preview opens the Apple Music page');
    d.querySelector('#ts-next').click();
    await review(d, 'iTunes review');
    const itunesTracks = [...d.querySelectorAll('.ts-track-value')].map(i => i.value).join('|');
    await check(itunesTracks === 'Apple First|Apple Second', 'iTunes songs ' + itunesTracks);
    await check(d.querySelector('#ts-cover-choice option')?.textContent.includes('1200×1200'), 'cover offered at 1200 pixels: ' + d.querySelector('#ts-cover-choice option')?.textContent);
    await apply(d, 'iTunes');
    disk = await fields(paths);
    await check(value(disk[0], 'ALBUM') === 'Fixture Apple Album' && value(disk[1], 'TITLE') === 'Apple Second', 'iTunes fields written');
    await check(value(disk[0], 'ITUNESALBUMID') === '900001' && value(disk[1], 'ITUNESADVISORY') === '1', 'iTunes IDs and advisory written');
    await check(disk.every(e => e.covers.length === 1), 'iTunes cover written to both files');

    // Ctrl+I repeats iTunes.
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'i', metaKey: true, bubbles: true, cancelable: true }));
    d = await waitFor(sourceDialog, 5000, 'Ctrl+I dialog');
    await check(sourceName(d) === 'iTunes', 'Ctrl+I reopens ' + sourceName(d));
    await closeSource(d);

    // Deezer: ISRC, label and barcode from the album and its track list.
    d = await search('source-deezer', 'Deezer');
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length === 1, 10000, 'Deezer results').catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
    d.querySelector('#ts-next').click();
    await apply(d, 'Deezer');
    disk = await fields(paths);
    await check(value(disk[0], 'ISRC') === 'NLA001500001' && value(disk[1], 'ISRC') === 'NLA001500002', 'Deezer ISRC per track');
    await check(value(disk[0], 'PUBLISHER') === 'Fixture Records' && value(disk[0], 'BARCODE') === '0123456789012' && value(disk[0], 'GENRE') === 'Ambient', 'Deezer label, barcode and genre');

    // API keys: saved privately, then the gated sources open.
    op('source-keys').click();
    const keys = await waitFor(() => document.querySelector('dialog.ts-keys-dialog[open]'), 5000, 'API keys dialog');
    await check(keys.querySelectorAll('fieldset.ts-key').length === 3 && /acoustid\.org\/new-application/.test(keys.textContent), 'API keys dialog lists AcoustID, Last.fm and fanart.tv with their registration pages');
    type(keys.querySelector('#ts-key-lastfm'), 'e2e-lastfm-key');
    type(keys.querySelector('#ts-key-fanart'), 'e2e-fanart-key');
    type(keys.querySelector('#ts-key-acoustid'), 'e2e-acoustid-key');
    keys.querySelector('#ts-save-keys').click();
    await waitFor(() => !keys.isConnected, 5000, 'keys saved');
    await waitFor(() => gated.every(name => !op(name).disabled), 5000, 'gated sources enabled');
    const view = await inv('load_tag_source_settings');
    await check(view.lastfm_key_set && view.fanart_key_set && view.acoustid_key_set && !JSON.stringify(view).includes('e2e-'), 'keys saved and never returned to the window');

    // Last.fm: the album's top tags become GENRE.
    d = await search('source-lastfm', 'Last.fm');
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length === 1, 10000, 'Last.fm result').catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
    d.querySelector('#ts-next').click();
    await review(d, 'Last.fm review');
    await check(d.querySelector('.ts-value[data-value="GENRE"]')?.value === 'Electronic', 'top genre tag, "seen live" skipped: ' + d.querySelector('.ts-value[data-value="GENRE"]')?.value);
    await apply(d, 'Last.fm');
    disk = await fields(paths);
    await check(value(disk[0], 'GENRE') === 'Electronic' && value(disk[1], 'TITLE') === 'Last Second', 'Last.fm genre and titles written');

    // fanart.tv covers: release groups from MusicBrainz, artwork by popularity.
    d = await search('source-cover-fanart', 'fanart.tv cover art');
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length === 1, 10000, 'fanart.tv release groups').catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
    d.querySelector('#ts-next').click();
    await review(d, 'fanart.tv review');
    const artwork = [...d.querySelectorAll('#ts-cover-choice option')].map(o => o.textContent);
    await check(artwork.length === 2 && artwork[0].includes('5 likes'), 'fanart.tv covers by likes: ' + artwork.join(' / '));
    await check(!d.querySelector('.ts-fields'), 'covers only: no fields');
    await apply(d, 'fanart.tv cover');
    disk = await fields(paths);
    await check(disk.every(e => e.covers.length >= 1) && value(disk[0], 'GENRE') === 'Electronic', 'fanart.tv cover written, fields unchanged');

    // AcoustID optional producer override: a missing explicit program is explained.
    op('source-options').click();
    const options = await waitFor(() => document.querySelector('dialog.options-dialog[open]'), 5000, 'options');
    options.querySelector('[data-page="tag-sources-online"]').click();
    const fpcalc = await waitFor(() => options.querySelector('[data-option="pages.tag-sources-online.fpcalc"]'), 5000, 'online services page');
    await check(!!options.querySelector('[data-option-button="tag-source-keys"]') && options.querySelector('[data-option="pages.tag-sources-online.itunes_cover_size"]')?.value === '1200', 'Online services page: API keys button and iTunes cover size');
    type(fpcalc, '/no/such/fpcalc');
    options.querySelector('#options-ok').click();
    await waitFor(() => !options.open, 5000, 'options saved');
    d = await search('source-acoustid', 'AcoustID');
    await waitFor(() => /optional fpcalc override was not found at \/no\/such\/fpcalc/.test(failure(d)), 10000, 'fpcalc message');
    await check(true, 'AcoustID explains the invalid optional override: ' + failure(d));
    await closeSource(d);

    // AcoustID with a stand-in fpcalc: sine-a is identified as the release's
    // second track and sine-b as its first, against their tags and order.
    const windows = navigator.userAgent.includes('Windows'), sep = paths[0].includes('\\') ? '\\' : '/';
    const answers = paths[0].replace(/[\\/][^\\/]+$/, '').replace(/[\\/][^\\/]+$/, '') + sep + 'source-answers' + sep + (windows ? 'fpcalc.cmd' : 'fpcalc');
    op('source-options').click();
    const again = await waitFor(() => document.querySelector('dialog.options-dialog[open]'), 5000, 'options');
    again.querySelector('[data-page="tag-sources-online"]').click();
    type(await waitFor(() => again.querySelector('[data-option="pages.tag-sources-online.fpcalc"]'), 5000, 'fpcalc field'), answers);
    again.querySelector('#options-ok').click();
    await waitFor(() => !again.open, 5000, 'options saved');
    d = await search('source-acoustid', 'AcoustID');
    await waitFor(() => d.querySelectorAll('.ts-results tbody tr').length === 1, 20000, 'AcoustID results').catch(error => { throw new Error(`${error.message}: ${failure(d)}`); });
    const found = [...d.querySelectorAll('.ts-results tbody tr td')].map(td => td.textContent);
    await check(found.includes('Fixture MB Album') && found.includes('2 of 2 files'), 'AcoustID lists the release holding both files: ' + found.join(' | '));
    d.querySelector('#ts-next').click();
    await review(d, 'AcoustID review');
    const paired = [...d.querySelectorAll('.ts-matches tbody tr')].map(r => r.children[3].textContent);
    await check(paired[0]?.includes('sine-b') && paired[1]?.includes('sine-a'), 'files paired by fingerprint, not by order or track tags: ' + paired.join(' / '));
    await apply(d, 'AcoustID');
    disk = await fields(paths);
    await check(value(disk[0], 'TITLE') === 'MB Second' && value(disk[1], 'TITLE') === 'MB First', 'MusicBrainz titles written to the identified files');
    await check(value(disk[0], 'ACOUSTID_ID') === 'acoustid-a' && value(disk[1], 'ACOUSTID_ID') === 'acoustid-b', 'ACOUSTID_ID per file: ' + value(disk[0], 'ACOUSTID_ID') + ', ' + value(disk[1], 'ACOUSTID_ID'));
  } catch (e) { await report('FAIL ' + (e.message || e)); }
  await report('SUMMARY done');
  await inv('e2e_exit', { code: 0 });
})();
