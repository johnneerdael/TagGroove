// Native e2e for TTA/TAK tagging, Matroska chapters in Extended tags and
// Utils → Strip chapters MP4 (with undo). Fixtures come from
// tests/e2e/formats-3-fixtures.sh via OPENTAG_E2E_FIXTURES. The first page
// load turns off the remove confirmation (a native dialog) and reloads into
// the same folder.
(async () => {
  const inv = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const report = m => inv('e2e_report', { message: `${m} [${(performance.now() / 1000).toFixed(1)}s]` });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const waitFor = async (fn, ms = 8000, what = 'condition') => { const t = Date.now(); let last; while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) { last = e; } await sleep(100); } throw new Error('timeout waiting for ' + what + (last ? ' (' + (last.message || last) + ')' : '')); };
  const status = () => document.querySelector('.status-message')?.textContent || '';
  const key = (k, o = {}) => (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...o }));
  const rows = () => [...document.querySelectorAll('tr[data-path]')];
  const row = name => rows().find(r => r.dataset.path.endsWith('/' + name));
  const clickRow = async name => { const r = await waitFor(() => row(name), 8000, 'row ' + name); r.querySelector('td').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await sleep(300); };
  const scan = async name => (await inv('scan_files', { paths: [row(name).dataset.path], recursive: false })).entries[0];
  const idle = () => waitFor(() => !document.querySelector('footer.status.busy'), 10000, 'idle');
  const op = name => document.querySelector(`button[data-op="${name}"]`);
  const ready = name => waitFor(() => op(name) && !op(name).disabled && op(name), 8000, name + ' enabled');
  const field = name => document.querySelector(`.fields [data-field="${name}"]`);
  const value = (fields, name) => Object.entries(fields || {}).find(([k]) => k.toUpperCase() === name)?.[1];
  const results = [];
  const step = async (name, fn) => { try { const detail = await fn(); results.push(['PASS', name]); await report(`PASS ${name}${detail ? ' — ' + detail : ''}`); } catch (e) { results.push(['FAIL', name]); await report(`FAIL ${name} — ${e && e.message || e}`); } };
  const finish = async () => { await report(`SUMMARY ${results.filter(r => r[0] === 'PASS').length}/${results.length} passed`); await inv('e2e_exit', { code: 0 }); };
  const saveTitle = async (name, title) => {
    await clickRow(name);
    const f = await waitFor(() => field('TITLE'), 4000, 'TITLE field');
    f.focus(); f.value = title; f.dispatchEvent(new Event('input', { bubbles: true })); await sleep(200);
    f.blur(); document.body.focus(); key('s', { metaKey: true });
    await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status'); await idle();
    return scan(name);
  };

  try {
    if (!sessionStorage.getItem('formats-3-stage')) {
      await waitFor(() => rows().length >= 6, 15000, 'fixture rows');
      const album = row('stream.tta').dataset.path.replace(/\/[^/]+$/, '');
      const settings = (await inv('load_settings')).settings;
      settings.pages = { ...settings.pages, messages: { ...(settings.pages?.messages || {}), confirm_remove_tags: false }, 'directories-startup': { startup: 'previous' } };
      await inv('save_settings', { settings });
      await inv('remember_directories', { paths: [album] });
      sessionStorage.setItem('formats-3-stage', '2');
      location.reload();
      return;
    }
    await waitFor(() => rows().length >= 6, 15000, 'fixture rows');
    await idle();

    await step('TTA, TAK, M4B and MKA files are scanned with their codecs', async () => {
      const found = [];
      for (const name of ['stream.tta', 'header.tak', 'chapters.m4b', 'chapters.mka']) {
        const entry = await scan(name);
        found.push(`${name}=${entry.format}/${entry.technical._CODEC?.[0]}`);
      }
      const text = found.join(' ');
      for (const expected of ['stream.tta=TTA/True Audio', 'header.tak=TAK/TAK', 'chapters.m4b=MP4/AAC', 'chapters.mka=Matroska/FLAC']) if (!text.includes(expected)) throw new Error(text);
      return text;
    });

    await step('TTA title edit in the tag panel writes ID3v2 like MP3', async () => {
      const entry = await saveTitle('stream.tta', 'TTA edited');
      if (value(entry.fields, 'TITLE')?.[0] !== 'TTA edited' || entry.technical._TAG_READ?.[0] !== 'ID3v2.3') throw new Error(JSON.stringify([entry.fields, entry.technical._TAG]));
      if (Math.abs(entry.duration - 2) > 0.01 || entry.sample_rate !== 44100) throw new Error(`duration ${entry.duration} rate ${entry.sample_rate}`);
      return `${entry.technical._TAG?.[0]}, ${entry.duration.toFixed(2)} s`;
    });

    await step('TAK title edit writes an APEv2 tag', async () => {
      const entry = await saveTitle('header.tak', 'TAK edited');
      if (value(entry.fields, 'TITLE')?.[0] !== 'TAK edited' || entry.technical._TAG_READ?.[0] !== 'APE') throw new Error(JSON.stringify([entry.fields, entry.technical._TAG]));
      return `${entry.technical._TAG?.[0]}, ${entry.channels} ch, ${entry.duration.toFixed(2)} s`;
    });

    await step('Matroska chapters are edited in Extended tags', async () => {
      await clickRow('chapters.mka');
      (await ready('extended')).click();
      const dlg = await waitFor(() => document.querySelector('dialog[open]'), 4000, 'extended dialog');
      const chapterRow = await waitFor(() => [...dlg.querySelectorAll('.extended-row')].find(r => r.querySelector('input').value === 'CHAPTERS'), 4000, 'CHAPTERS row');
      const values = [...chapterRow.querySelectorAll('textarea')].map(t => t.value);
      if (values.join('|') !== '00:00:00.000 Opening|00:00:01.000 Second part') throw new Error(values.join('|'));
      const first = chapterRow.querySelector('textarea');
      first.value = '0:00 Prologue'; first.dispatchEvent(new Event('input', { bubbles: true }));
      dlg.querySelector('#apply-extended').click(); await sleep(200);
      document.body.focus(); key('s', { metaKey: true });
      await waitFor(() => /Saved tags/.test(status()), 8000, 'saved status'); await idle();
      const entry = await scan('chapters.mka');
      if (entry.fields.CHAPTERS?.join('|') !== '00:00:00.000 Prologue|00:00:01.000 Second part') throw new Error(JSON.stringify(entry.fields));
      return entry.fields.CHAPTERS.join(' | ') + `; _SUBSONG_COUNT=${entry.technical._SUBSONG_COUNT}`;
    });

    await step('Strip chapters MP4 is disabled without MP4 chapters', async () => {
      await clickRow('stream.tta');
      if (!op('strip-mp4-chapters')?.disabled) throw new Error('enabled for a TTA file');
      return op('strip-mp4-chapters').title || 'disabled';
    });

    await step('Strip chapters MP4 removes the chapters and Undo restores them', async () => {
      await clickRow('chapters.m4b');
      const before = await scan('chapters.m4b');
      if (before.technical._SUBSONG_COUNT?.[0] !== '2') throw new Error(JSON.stringify(before.technical._CHAPTERS));
      (await ready('strip-mp4-chapters')).click();
      await waitFor(() => /Stripped MP4 chapters/.test(status()), 8000, 'strip status'); await idle();
      const after = await scan('chapters.m4b');
      if (after.technical._SUBSONG_COUNT || after.size !== before.size || value(after.fields, 'TITLE')?.[0] !== 'Book') throw new Error(JSON.stringify([after.technical._SUBSONG_COUNT, after.size, before.size]));
      await ready('undo'); row('chapters.m4b').focus(); key('z', { metaKey: true });
      await waitFor(() => /Undid/.test(status()), 8000, 'undo status'); await idle();
      const restored = await scan('chapters.m4b');
      if (restored.technical._CHAPTERS?.join('|') !== before.technical._CHAPTERS.join('|')) throw new Error(JSON.stringify(restored.technical._CHAPTERS));
      return `${before.technical._CHAPTERS.join(' | ')} → none → ${status()}`;
    });
  } catch (e) {
    await report(`FATAL ${e && e.message || e}`);
  }
  await finish();
})();
